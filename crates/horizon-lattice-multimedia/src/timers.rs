//! High-precision timer module for Horizon Lattice.
//!
//! This module provides sub-millisecond accurate timing by combining native
//! sleep for the bulk of the wait period with spin-waiting for the final
//! portion, achieving consistent timing without excessive CPU usage.
//!
//! # Use Cases
//!
//! - Audio/video synchronization
//! - Game loop timing
//! - Animation frame timing (when vsync is unavailable)
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_multimedia::timers::{HighPrecisionTimer, TimerConfig};
//! use std::time::Duration;
//!
//! // Create a timer with default configuration
//! let timer = HighPrecisionTimer::new(Duration::from_millis(16))?;
//!
//! // Connect to tick events
//! timer.on_tick(|event| {
//!     println!("Tick at {:?}, drift: {:?}", event.elapsed, event.drift);
//! });
//!
//! // Start the timer
//! timer.start()?;
//!
//! // ... later
//! timer.stop()?;
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use spin_sleep::{SpinSleeper, SpinStrategy};

use horizon_lattice_core::signal::{ConnectionId, Signal};

use crate::error::{MultimediaError, Result};

/// Default native sleep accuracy in nanoseconds (1ms).
/// The timer will use native sleep until this threshold, then spin.
const DEFAULT_NATIVE_ACCURACY_NS: u32 = 1_000_000;

/// Configuration for high-precision timers.
#[derive(Debug, Clone)]
pub struct TimerConfig {
    /// The accuracy threshold in nanoseconds below which spin-waiting is used.
    /// Lower values mean more spinning (more CPU usage, higher accuracy).
    /// Default: 1ms (1,000,000 ns)
    /// Note: Maximum value is ~4.29 seconds (u32::MAX nanoseconds).
    pub spin_threshold_ns: u32,

    /// The spin strategy to use while spin-waiting.
    /// Default: `SpinStrategy::YieldThread` (yields to other threads while spinning)
    pub spin_strategy: SpinStrategyConfig,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            spin_threshold_ns: DEFAULT_NATIVE_ACCURACY_NS,
            spin_strategy: SpinStrategyConfig::YieldThread,
        }
    }
}

/// Spin strategy configuration (mirrors `spin_sleep::SpinStrategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinStrategyConfig {
    /// Yield the thread while spinning (lower CPU, slightly less accurate).
    /// This is the default on non-Windows platforms.
    #[default]
    YieldThread,
    /// Use a spin loop hint instruction.
    /// This is the default on Windows.
    SpinLoopHint,
}

impl From<SpinStrategyConfig> for SpinStrategy {
    fn from(config: SpinStrategyConfig) -> Self {
        match config {
            SpinStrategyConfig::YieldThread => SpinStrategy::YieldThread,
            SpinStrategyConfig::SpinLoopHint => SpinStrategy::SpinLoopHint,
        }
    }
}

/// Event data passed to tick callbacks.
#[derive(Debug, Clone)]
pub struct TimerEvent {
    /// Time elapsed since the timer started.
    pub elapsed: Duration,
    /// The tick number (starts at 1).
    pub tick_count: u64,
    /// The drift from the expected tick time.
    /// Positive values mean the tick was late.
    pub drift: Duration,
    /// Whether the drift was positive (late) or negative (early).
    pub drift_positive: bool,
}

/// Wrapper around `spin_sleep` for one-shot precise sleeping.
///
/// Use this when you need to sleep for a precise duration without
/// creating a repeating timer.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_multimedia::timers::PreciseSleeper;
/// use std::time::Duration;
///
/// let sleeper = PreciseSleeper::new();
/// sleeper.sleep(Duration::from_micros(500)); // Sleep for 500 microseconds
/// ```
pub struct PreciseSleeper {
    sleeper: SpinSleeper,
}

impl PreciseSleeper {
    /// Create a new precise sleeper with default configuration.
    pub fn new() -> Self {
        Self {
            sleeper: SpinSleeper::new(DEFAULT_NATIVE_ACCURACY_NS)
                .with_spin_strategy(SpinStrategy::YieldThread),
        }
    }

    /// Create a precise sleeper with custom configuration.
    pub fn with_config(config: &TimerConfig) -> Self {
        Self {
            sleeper: SpinSleeper::new(config.spin_threshold_ns)
                .with_spin_strategy(config.spin_strategy.into()),
        }
    }

    /// Sleep for the specified duration with high precision.
    pub fn sleep(&self, duration: Duration) {
        self.sleeper.sleep(duration);
    }

    /// Sleep until the specified deadline with high precision.
    pub fn sleep_until(&self, deadline: Instant) {
        let now = Instant::now();
        if deadline > now {
            self.sleeper.sleep(deadline - now);
        }
    }

    /// Sleep for the specified number of seconds.
    pub fn sleep_s(&self, seconds: f64) {
        self.sleeper.sleep_s(seconds);
    }

    /// Sleep for the specified number of nanoseconds.
    pub fn sleep_ns(&self, nanoseconds: u64) {
        self.sleeper.sleep_ns(nanoseconds);
    }
}

impl Default for PreciseSleeper {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared signals for the high-precision timer.
struct TimerSignals {
    /// Emitted on each timer tick.
    tick: Signal<TimerEvent>,
    /// Emitted when the timer is stopped.
    stopped: Signal<()>,
    /// Emitted when an error occurs.
    error: Signal<String>,
}

/// Internal state of the timer.
struct TimerState {
    /// Whether the timer is currently running.
    running: bool,
    /// The configured interval.
    interval: Duration,
    /// Start time of the timer.
    start_time: Option<Instant>,
    /// Current tick count.
    tick_count: u64,
}

/// A high-precision interval timer with signal-based notifications.
///
/// This timer uses native OS sleep for the bulk of the wait period and
/// spin-waiting for the final millisecond to achieve sub-millisecond
/// accuracy without excessive CPU usage.
///
/// # Signals
///
/// - `tick`: Emitted on each timer interval with timing information.
/// - `stopped`: Emitted when the timer is stopped.
/// - `error`: Emitted if an error occurs in the timer thread.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_multimedia::timers::HighPrecisionTimer;
/// use std::time::Duration;
///
/// let timer = HighPrecisionTimer::new(Duration::from_millis(16))?; // ~60 FPS
///
/// timer.on_tick(|event| {
///     // Called every ~16ms with sub-ms accuracy
///     println!("Frame {}, drift: {:?}", event.tick_count, event.drift);
/// });
///
/// timer.start()?;
/// ```
pub struct HighPrecisionTimer {
    /// Configuration for the timer.
    config: TimerConfig,
    /// Internal state.
    state: Arc<Mutex<TimerState>>,
    /// Flag to signal the timer thread to stop.
    stop_flag: Arc<AtomicBool>,
    /// Handle to the timer thread.
    thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// Shared signals.
    signals: Arc<TimerSignals>,
}

impl HighPrecisionTimer {
    /// Create a new high-precision timer with the specified interval.
    ///
    /// The timer is created in a stopped state. Call `start()` to begin.
    ///
    /// # Arguments
    ///
    /// * `interval` - The duration between timer ticks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let timer = HighPrecisionTimer::new(Duration::from_millis(16))?;
    /// ```
    pub fn new(interval: Duration) -> Result<Self> {
        Self::with_config(interval, TimerConfig::default())
    }

    /// Create a new high-precision timer with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `interval` - The duration between timer ticks.
    /// * `config` - Timer configuration including spin threshold and strategy.
    pub fn with_config(interval: Duration, config: TimerConfig) -> Result<Self> {
        if interval.is_zero() {
            return Err(MultimediaError::Timer("interval cannot be zero".into()));
        }

        let state = Arc::new(Mutex::new(TimerState {
            running: false,
            interval,
            start_time: None,
            tick_count: 0,
        }));

        let signals = Arc::new(TimerSignals {
            tick: Signal::new(),
            stopped: Signal::new(),
            error: Signal::new(),
        });

        Ok(Self {
            config,
            state,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: Arc::new(Mutex::new(None)),
            signals,
        })
    }

    /// Start the timer.
    ///
    /// This spawns a background thread that emits `tick` signals at the
    /// configured interval.
    ///
    /// Returns an error if the timer is already running.
    pub fn start(&self) -> Result<()> {
        let mut state = self.state.lock();
        if state.running {
            return Err(MultimediaError::TimerAlreadyRunning);
        }

        state.running = true;
        state.start_time = Some(Instant::now());
        state.tick_count = 0;
        self.stop_flag.store(false, Ordering::SeqCst);

        let interval = state.interval;
        drop(state);

        let sleeper = SpinSleeper::new(self.config.spin_threshold_ns)
            .with_spin_strategy(self.config.spin_strategy.into());

        let state = Arc::clone(&self.state);
        let signals = Arc::clone(&self.signals);
        let stop_flag = Arc::clone(&self.stop_flag);

        let handle = std::thread::Builder::new()
            .name("high-precision-timer".into())
            .spawn(move || {
                let start = Instant::now();
                let mut next_tick = start + interval;
                let mut tick_count: u64 = 0;

                while !stop_flag.load(Ordering::SeqCst) {
                    // Sleep until the next tick
                    let now = Instant::now();
                    if next_tick > now {
                        sleeper.sleep(next_tick - now);
                    }

                    // Check if we should stop after sleeping
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }

                    tick_count += 1;
                    let actual_time = Instant::now();
                    let elapsed = actual_time - start;

                    // Calculate drift
                    let expected = start + interval * tick_count as u32;
                    let (drift, drift_positive) = if actual_time >= expected {
                        (actual_time - expected, true)
                    } else {
                        (expected - actual_time, false)
                    };

                    // Update state
                    {
                        let mut state = state.lock();
                        state.tick_count = tick_count;
                    }

                    // Emit tick signal
                    let event = TimerEvent {
                        elapsed,
                        tick_count,
                        drift,
                        drift_positive,
                    };
                    signals.tick.emit(event);

                    // Schedule next tick
                    next_tick += interval;

                    // If we're significantly behind, catch up
                    let now = Instant::now();
                    if next_tick < now {
                        // Skip missed ticks and realign
                        let missed = (now - next_tick).as_nanos() / interval.as_nanos();
                        next_tick += interval * (missed as u32 + 1);
                    }
                }

                // Emit stopped signal
                signals.stopped.emit(());
            })
            .map_err(|e| MultimediaError::Timer(e.to_string()))?;

        *self.thread_handle.lock() = Some(handle);
        Ok(())
    }

    /// Stop the timer.
    ///
    /// This signals the timer thread to stop and waits for it to finish.
    /// The `stopped` signal is emitted when the timer fully stops.
    ///
    /// Returns an error if the timer is not running.
    pub fn stop(&self) -> Result<()> {
        {
            let state = self.state.lock();
            if !state.running {
                return Err(MultimediaError::TimerNotRunning);
            }
        }

        // Signal the thread to stop
        self.stop_flag.store(true, Ordering::SeqCst);

        // Wait for the thread to finish
        let handle = self.thread_handle.lock().take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }

        // Update state
        {
            let mut state = self.state.lock();
            state.running = false;
            state.start_time = None;
        }

        Ok(())
    }

    /// Check if the timer is currently running.
    pub fn is_running(&self) -> bool {
        self.state.lock().running
    }

    /// Get the configured interval.
    pub fn interval(&self) -> Duration {
        self.state.lock().interval
    }

    /// Set a new interval.
    ///
    /// The new interval takes effect on the next tick if the timer is running.
    pub fn set_interval(&self, interval: Duration) -> Result<()> {
        if interval.is_zero() {
            return Err(MultimediaError::Timer("interval cannot be zero".into()));
        }
        self.state.lock().interval = interval;
        Ok(())
    }

    /// Get the current tick count.
    pub fn tick_count(&self) -> u64 {
        self.state.lock().tick_count
    }

    /// Get the elapsed time since the timer started.
    ///
    /// Returns `None` if the timer has not been started.
    pub fn elapsed(&self) -> Option<Duration> {
        self.state.lock().start_time.map(|t| t.elapsed())
    }

    /// Connect a callback to the tick signal.
    ///
    /// The callback is invoked on each timer tick with a `TimerEvent`
    /// containing timing information.
    pub fn on_tick<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&TimerEvent) + Send + Sync + 'static,
    {
        self.signals.tick.connect(callback)
    }

    /// Disconnect a tick callback.
    pub fn disconnect_tick(&self, id: ConnectionId) {
        self.signals.tick.disconnect(id);
    }

    /// Connect a callback to the stopped signal.
    ///
    /// The callback is invoked when the timer stops.
    pub fn on_stopped<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&()) + Send + Sync + 'static,
    {
        self.signals.stopped.connect(callback)
    }

    /// Disconnect a stopped callback.
    pub fn disconnect_stopped(&self, id: ConnectionId) {
        self.signals.stopped.disconnect(id);
    }

    /// Connect a callback to the error signal.
    ///
    /// The callback is invoked if an error occurs in the timer thread.
    pub fn on_error<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&String) + Send + Sync + 'static,
    {
        self.signals.error.connect(callback)
    }

    /// Disconnect an error callback.
    pub fn disconnect_error(&self, id: ConnectionId) {
        self.signals.error.disconnect(id);
    }
}

impl Drop for HighPrecisionTimer {
    fn drop(&mut self) {
        // Ensure the timer thread is stopped when the timer is dropped
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }
    }
}

/// Perform a high-precision one-shot sleep with default configuration.
///
/// This is a convenience function for simple use cases.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_multimedia::timers::precise_sleep;
/// use std::time::Duration;
///
/// precise_sleep(Duration::from_micros(500));
/// ```
pub fn precise_sleep(duration: Duration) {
    spin_sleep::sleep(duration);
}

/// Perform a high-precision sleep for the specified number of seconds.
pub fn precise_sleep_s(seconds: f64) {
    SpinSleeper::default().sleep_s(seconds);
}

/// Perform a high-precision sleep for the specified number of nanoseconds.
pub fn precise_sleep_ns(nanoseconds: u64) {
    SpinSleeper::default().sleep_ns(nanoseconds);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    #[test]
    fn test_precise_sleeper_creation() {
        let sleeper = PreciseSleeper::new();
        let start = Instant::now();
        sleeper.sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();
        // Should be at least 10ms but not excessively more
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(20));
    }

    #[test]
    fn test_precise_sleeper_with_config() {
        let config = TimerConfig {
            spin_threshold_ns: 500_000, // 0.5ms
            spin_strategy: SpinStrategyConfig::YieldThread,
        };
        let sleeper = PreciseSleeper::with_config(&config);
        let start = Instant::now();
        sleeper.sleep(Duration::from_millis(5));
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(5));
        assert!(elapsed < Duration::from_millis(15));
    }

    #[test]
    fn test_timer_creation() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(100)).unwrap();
        assert!(!timer.is_running());
        assert_eq!(timer.interval(), Duration::from_millis(100));
    }

    #[test]
    fn test_timer_zero_interval_rejected() {
        let result = HighPrecisionTimer::new(Duration::ZERO);
        assert!(result.is_err());
    }

    #[test]
    fn test_timer_start_stop() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(50)).unwrap();

        let tick_count = Arc::new(AtomicU64::new(0));
        let tick_count_clone = Arc::clone(&tick_count);

        timer.on_tick(move |_event| {
            tick_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        timer.start().unwrap();
        assert!(timer.is_running());

        // Wait for a few ticks
        std::thread::sleep(Duration::from_millis(175));

        timer.stop().unwrap();
        assert!(!timer.is_running());

        // Should have approximately 3 ticks (175ms / 50ms = 3.5)
        let ticks = tick_count.load(Ordering::SeqCst);
        assert!((2..=5).contains(&ticks), "Expected 2-5 ticks, got {ticks}");
    }

    #[test]
    fn test_timer_double_start_rejected() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(100)).unwrap();
        timer.start().unwrap();
        let result = timer.start();
        assert!(result.is_err());
        timer.stop().unwrap();
    }

    #[test]
    fn test_timer_stop_when_not_running_rejected() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(100)).unwrap();
        let result = timer.stop();
        assert!(result.is_err());
    }

    #[test]
    fn test_timer_tick_count() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(20)).unwrap();
        timer.start().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let count = timer.tick_count();
        timer.stop().unwrap();
        assert!((3..=7).contains(&count), "Expected 3-7 ticks, got {count}");
    }

    #[test]
    fn test_timer_dropped_while_running() {
        let timer = HighPrecisionTimer::new(Duration::from_millis(50)).unwrap();
        timer.start().unwrap();
        // Timer should cleanly stop when dropped
        drop(timer);
    }

    #[test]
    fn test_precise_sleep_functions() {
        let start = Instant::now();
        precise_sleep(Duration::from_millis(10));
        assert!(start.elapsed() >= Duration::from_millis(10));

        let start = Instant::now();
        precise_sleep_s(0.01);
        assert!(start.elapsed() >= Duration::from_millis(10));

        let start = Instant::now();
        precise_sleep_ns(10_000_000);
        assert!(start.elapsed() >= Duration::from_millis(10));
    }
}
