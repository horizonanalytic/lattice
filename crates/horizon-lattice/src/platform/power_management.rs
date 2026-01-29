//! Power management services.
//!
//! This module provides cross-platform power management functionality including
//! battery status detection, sleep prevention, and power event notifications.
//!
//! # Battery Status
//!
//! ```ignore
//! use horizon_lattice::platform::{PowerState, PowerSource};
//!
//! // Check power source
//! if PowerState::power_source() == PowerSource::Battery {
//!     println!("Running on battery");
//! }
//!
//! // Get battery level
//! if let Some(level) = PowerState::battery_level() {
//!     println!("Battery: {:.0}%", level);
//! }
//!
//! // Get detailed battery info
//! for battery in PowerState::batteries()? {
//!     println!("Battery: {:?}", battery);
//! }
//! ```
//!
//! # Sleep Prevention
//!
//! ```ignore
//! use horizon_lattice::platform::SleepInhibitor;
//!
//! // Prevent system from sleeping during long operation
//! let _guard = SleepInhibitor::new()
//!     .reason("Encoding video")
//!     .prevent_display_sleep(true)
//!     .prevent_system_sleep(true)
//!     .start()?;
//!
//! // Sleep is prevented until `_guard` is dropped
//! do_long_operation();
//! // Guard drops here, normal sleep behavior resumes
//! ```
//!
//! # Power Events
//!
//! ```ignore
//! use horizon_lattice::platform::PowerEventWatcher;
//!
//! let watcher = PowerEventWatcher::new()?;
//!
//! watcher.sleep_imminent().connect(|_| {
//!     println!("System is about to sleep!");
//!     // Save state, close connections, etc.
//! });
//!
//! watcher.wake().connect(|_| {
//!     println!("System woke up!");
//!     // Restore state, reconnect, etc.
//! });
//!
//! watcher.start()?;
//! ```
//!
//! # Platform Notes
//!
//! ## Battery Status
//! - **Windows**: Uses Windows Management APIs via `starship-battery`
//! - **macOS**: Uses IOKit via `starship-battery`
//! - **Linux**: Uses sysfs via `starship-battery`
//!
//! ## Sleep Prevention
//! - **Windows**: Uses `SetThreadExecutionState`
//! - **macOS**: Uses `IOPMAssertionCreateWithName`
//! - **Linux**: Uses `systemd-logind` D-Bus inhibitor interface
//!
//! ## Power Events
//! - **Windows**: Uses `WM_POWERBROADCAST` messages
//! - **macOS**: Not yet implemented (requires IOKit power notifications)
//! - **Linux**: Uses `systemd-logind` D-Bus `PrepareForSleep` signal

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_lattice_core::Signal;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for power management operations.
#[derive(Debug)]
pub struct PowerManagementError {
    kind: PowerManagementErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Some variants only used on certain platforms
enum PowerManagementErrorKind {
    /// Failed to query battery status.
    BatteryQuery,
    /// Failed to inhibit sleep.
    SleepInhibit,
    /// Failed to set up power event watcher.
    PowerEvents,
    /// Operation not supported on this platform.
    UnsupportedPlatform,
    /// I/O or system error.
    Io,
}

impl PowerManagementError {
    fn battery_query(message: impl Into<String>) -> Self {
        Self {
            kind: PowerManagementErrorKind::BatteryQuery,
            message: message.into(),
        }
    }

    fn sleep_inhibit(message: impl Into<String>) -> Self {
        Self {
            kind: PowerManagementErrorKind::SleepInhibit,
            message: message.into(),
        }
    }

    #[allow(dead_code)] // Only used on Windows/Linux
    fn power_events(message: impl Into<String>) -> Self {
        Self {
            kind: PowerManagementErrorKind::PowerEvents,
            message: message.into(),
        }
    }

    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: PowerManagementErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn io(message: impl Into<String>) -> Self {
        Self {
            kind: PowerManagementErrorKind::Io,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the operation is not supported.
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == PowerManagementErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for PowerManagementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PowerManagementErrorKind::BatteryQuery => {
                write!(f, "battery query error: {}", self.message)
            }
            PowerManagementErrorKind::SleepInhibit => {
                write!(f, "sleep inhibit error: {}", self.message)
            }
            PowerManagementErrorKind::PowerEvents => {
                write!(f, "power events error: {}", self.message)
            }
            PowerManagementErrorKind::UnsupportedPlatform => {
                write!(f, "unsupported platform: {}", self.message)
            }
            PowerManagementErrorKind::Io => {
                write!(f, "I/O error: {}", self.message)
            }
        }
    }
}

impl std::error::Error for PowerManagementError {}

// ============================================================================
// Power Source
// ============================================================================

/// The current power source for the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PowerSource {
    /// Running on AC power (plugged in).
    Ac,
    /// Running on battery power.
    Battery,
    /// Power source could not be determined.
    #[default]
    Unknown,
}

// ============================================================================
// Battery State
// ============================================================================

/// The charging state of a battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BatteryState {
    /// Battery is charging.
    Charging,
    /// Battery is discharging (on battery power).
    Discharging,
    /// Battery is fully charged.
    Full,
    /// Battery is empty.
    Empty,
    /// Battery state is unknown.
    #[default]
    Unknown,
}

// ============================================================================
// Battery Info
// ============================================================================

/// Information about a system battery.
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    /// Current charge level as a percentage (0.0 - 100.0).
    pub level: f32,
    /// Current battery state.
    pub state: BatteryState,
    /// Estimated time until the battery is empty (in seconds), if available.
    pub time_to_empty: Option<u64>,
    /// Estimated time until the battery is fully charged (in seconds), if available.
    pub time_to_full: Option<u64>,
    /// Current voltage in volts, if available.
    pub voltage: Option<f32>,
    /// Current energy rate in watts (positive = charging, negative = discharging).
    pub energy_rate: Option<f32>,
    /// Battery temperature in Celsius, if available.
    pub temperature: Option<f32>,
    /// Number of charge cycles, if available.
    pub cycle_count: Option<u32>,
    /// Battery health as a percentage (0.0 - 100.0), if available.
    pub health: Option<f32>,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            level: 0.0,
            state: BatteryState::Unknown,
            time_to_empty: None,
            time_to_full: None,
            voltage: None,
            energy_rate: None,
            temperature: None,
            cycle_count: None,
            health: None,
        }
    }
}

// ============================================================================
// Power State
// ============================================================================

/// Query the current power state of the system.
///
/// This provides static methods to check battery levels, power source,
/// and other power-related information.
pub struct PowerState;

#[cfg(feature = "power-management")]
impl PowerState {
    /// Get the current power source (AC or battery).
    ///
    /// Returns `PowerSource::Unknown` if the power source cannot be determined
    /// or if no batteries are present in the system.
    pub fn power_source() -> PowerSource {
        use starship_battery::Manager;

        let Ok(manager) = Manager::new() else {
            return PowerSource::Unknown;
        };

        let Ok(batteries) = manager.batteries() else {
            return PowerSource::Unknown;
        };

        for battery in batteries.flatten() {
            match battery.state() {
                starship_battery::State::Charging | starship_battery::State::Full => {
                    return PowerSource::Ac;
                }
                starship_battery::State::Discharging => {
                    return PowerSource::Battery;
                }
                _ => {}
            }
        }

        PowerSource::Unknown
    }

    /// Check if the system is running on battery power.
    pub fn is_on_battery() -> bool {
        Self::power_source() == PowerSource::Battery
    }

    /// Check if the system is plugged in (on AC power).
    pub fn is_plugged_in() -> bool {
        Self::power_source() == PowerSource::Ac
    }

    /// Get the current battery level as a percentage (0.0 - 100.0).
    ///
    /// Returns `None` if no battery is present or if the level cannot be determined.
    /// If multiple batteries are present, returns the average level.
    pub fn battery_level() -> Option<f32> {
        use starship_battery::Manager;
        use starship_battery::units::ratio::percent;

        let manager = Manager::new().ok()?;
        let batteries = manager.batteries().ok()?;

        let mut total = 0.0;
        let mut count = 0;

        for battery in batteries.flatten() {
            total += battery.state_of_charge().get::<percent>();
            count += 1;
        }

        if count > 0 {
            Some(total / count as f32)
        } else {
            None
        }
    }

    /// Get detailed information about all batteries in the system.
    pub fn batteries() -> Result<Vec<BatteryInfo>, PowerManagementError> {
        use starship_battery::Manager;
        use starship_battery::units::{
            electric_potential::volt, power::watt, ratio::percent,
            thermodynamic_temperature::degree_celsius, time::second,
        };

        let manager =
            Manager::new().map_err(|e| PowerManagementError::battery_query(e.to_string()))?;

        let batteries = manager
            .batteries()
            .map_err(|e| PowerManagementError::battery_query(e.to_string()))?;

        let mut result = Vec::new();

        for battery in batteries.flatten() {
            let state = match battery.state() {
                starship_battery::State::Charging => BatteryState::Charging,
                starship_battery::State::Discharging => BatteryState::Discharging,
                starship_battery::State::Full => BatteryState::Full,
                starship_battery::State::Empty => BatteryState::Empty,
                _ => BatteryState::Unknown,
            };

            let info = BatteryInfo {
                level: battery.state_of_charge().get::<percent>(),
                state,
                time_to_empty: battery.time_to_empty().map(|t| t.get::<second>() as u64),
                time_to_full: battery.time_to_full().map(|t| t.get::<second>() as u64),
                voltage: Some(battery.voltage().get::<volt>()),
                energy_rate: Some(battery.energy_rate().get::<watt>()),
                temperature: battery.temperature().map(|t| t.get::<degree_celsius>()),
                cycle_count: battery.cycle_count(),
                health: Some(battery.state_of_health().get::<percent>()),
            };

            result.push(info);
        }

        Ok(result)
    }

    /// Check if any battery is present in the system.
    pub fn has_battery() -> bool {
        use starship_battery::Manager;

        Manager::new()
            .and_then(|m| m.batteries())
            .map(|mut b| b.next().is_some())
            .unwrap_or(false)
    }
}

#[cfg(not(feature = "power-management"))]
impl PowerState {
    /// Get the current power source (AC or battery).
    pub fn power_source() -> PowerSource {
        PowerSource::Unknown
    }

    /// Check if the system is running on battery power.
    pub fn is_on_battery() -> bool {
        false
    }

    /// Check if the system is plugged in (on AC power).
    pub fn is_plugged_in() -> bool {
        false
    }

    /// Get the current battery level as a percentage (0.0 - 100.0).
    pub fn battery_level() -> Option<f32> {
        None
    }

    /// Get detailed information about all batteries in the system.
    pub fn batteries() -> Result<Vec<BatteryInfo>, PowerManagementError> {
        Err(PowerManagementError::unsupported_platform(
            "power-management feature is not enabled",
        ))
    }

    /// Check if any battery is present in the system.
    pub fn has_battery() -> bool {
        false
    }
}

// ============================================================================
// Sleep Inhibitor
// ============================================================================

/// Options for what types of sleep to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SleepInhibitOptions {
    /// Prevent the display from sleeping/dimming.
    pub display: bool,
    /// Prevent the system from entering sleep mode.
    pub system: bool,
}

/// Builder for creating a sleep inhibitor.
///
/// Use this to prevent the system from sleeping during long-running operations
/// like file transfers, video encoding, or presentations.
#[derive(Debug, Clone)]
pub struct SleepInhibitorBuilder {
    reason: String,
    app_name: String,
    options: SleepInhibitOptions,
}

impl Default for SleepInhibitorBuilder {
    fn default() -> Self {
        Self {
            reason: "Application requested".to_string(),
            app_name: "horizon-lattice".to_string(),
            options: SleepInhibitOptions {
                display: false,
                system: true,
            },
        }
    }
}

impl SleepInhibitorBuilder {
    /// Create a new sleep inhibitor builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the reason for preventing sleep (shown in system UI on some platforms).
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Set the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Set whether to prevent display sleep.
    pub fn prevent_display_sleep(mut self, prevent: bool) -> Self {
        self.options.display = prevent;
        self
    }

    /// Set whether to prevent system sleep.
    pub fn prevent_system_sleep(mut self, prevent: bool) -> Self {
        self.options.system = prevent;
        self
    }

    /// Start preventing sleep. Returns a guard that restores normal sleep
    /// behavior when dropped.
    #[cfg(feature = "power-management")]
    pub fn start(self) -> Result<SleepInhibitorGuard, PowerManagementError> {
        SleepInhibitorGuard::new(self.reason, self.app_name, self.options)
    }

    #[cfg(not(feature = "power-management"))]
    pub fn start(self) -> Result<SleepInhibitorGuard, PowerManagementError> {
        Err(PowerManagementError::unsupported_platform(
            "power-management feature is not enabled",
        ))
    }
}

/// A guard that prevents sleep while it exists.
///
/// When this guard is dropped, normal sleep behavior is restored.
/// This uses RAII to ensure sleep prevention is always properly cleaned up.
pub struct SleepInhibitorGuard {
    #[cfg(feature = "power-management")]
    _inner: keepawake::KeepAwake,
    #[cfg(not(feature = "power-management"))]
    _marker: std::marker::PhantomData<()>,
}

#[cfg(feature = "power-management")]
impl SleepInhibitorGuard {
    fn new(
        reason: String,
        app_name: String,
        options: SleepInhibitOptions,
    ) -> Result<Self, PowerManagementError> {
        let awake = keepawake::Builder::default()
            .display(options.display)
            .idle(options.system)
            .sleep(options.system)
            .reason(&reason)
            .app_name(&app_name)
            .create()
            .map_err(|e| PowerManagementError::sleep_inhibit(e.to_string()))?;

        Ok(Self { _inner: awake })
    }
}

#[cfg(not(feature = "power-management"))]
impl SleepInhibitorGuard {
    fn new(
        _reason: String,
        _app_name: String,
        _options: SleepInhibitOptions,
    ) -> Result<Self, PowerManagementError> {
        Err(PowerManagementError::unsupported_platform(
            "power-management feature is not enabled",
        ))
    }
}

impl fmt::Debug for SleepInhibitorGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SleepInhibitorGuard").finish()
    }
}

/// Convenience type alias for the sleep inhibitor builder.
pub type SleepInhibitor = SleepInhibitorBuilder;

// ============================================================================
// Power Events
// ============================================================================

/// Reason for a power state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerEventReason {
    /// System is about to sleep (user initiated or idle timeout).
    SleepImminent,
    /// System has woken from sleep.
    Wake,
    /// Power source changed (AC to battery or vice versa).
    PowerSourceChanged,
}

/// Internal state for the power event watcher.
struct PowerEventWatcherInner {
    sleep_imminent: Signal<()>,
    wake: Signal<()>,
    power_source_changed: Signal<PowerSource>,
    running: AtomicBool,
}

/// Watches for power-related system events.
///
/// This allows applications to be notified when the system is about to sleep
/// or has woken up, enabling them to save state or reconnect to services.
pub struct PowerEventWatcher {
    inner: Arc<PowerEventWatcherInner>,
}

impl PowerEventWatcher {
    /// Create a new power event watcher.
    pub fn new() -> Result<Self, PowerManagementError> {
        Ok(Self {
            inner: Arc::new(PowerEventWatcherInner {
                sleep_imminent: Signal::new(),
                wake: Signal::new(),
                power_source_changed: Signal::new(),
                running: AtomicBool::new(false),
            }),
        })
    }

    /// Signal emitted when the system is about to sleep.
    ///
    /// Connect to this signal to save state, close connections, or perform
    /// other cleanup before the system enters sleep mode.
    pub fn sleep_imminent(&self) -> &Signal<()> {
        &self.inner.sleep_imminent
    }

    /// Signal emitted when the system wakes from sleep.
    ///
    /// Connect to this signal to restore state, reconnect to services, or
    /// refresh data after the system wakes up.
    pub fn wake(&self) -> &Signal<()> {
        &self.inner.wake
    }

    /// Signal emitted when the power source changes.
    ///
    /// The signal provides the new power source (AC or Battery).
    pub fn power_source_changed(&self) -> &Signal<PowerSource> {
        &self.inner.power_source_changed
    }

    /// Start watching for power events.
    ///
    /// This spawns a background thread or registers event handlers to monitor
    /// for power state changes. Events will be delivered to connected signals.
    #[cfg(target_os = "linux")]
    pub fn start(&self) -> Result<(), PowerManagementError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            let result = pollster::block_on(async { linux_power_event_loop(&inner).await });

            if let Err(e) = result {
                eprintln!("Power event watcher error: {}", e);
            }
        });

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn start(&self) -> Result<(), PowerManagementError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            if let Err(e) = windows_power_event_loop(&inner) {
                eprintln!("Power event watcher error: {}", e);
            }
        });

        Ok(())
    }

    /// Start listening for power events (macOS - limited support).
    #[cfg(target_os = "macos")]
    pub fn start(&self) -> Result<(), PowerManagementError> {
        // macOS power events require IOKit's IORegisterForSystemPower which
        // is not currently exposed by io-kit-sys crate. Battery status and
        // sleep prevention still work via starship-battery and keepawake.
        Err(PowerManagementError::unsupported_platform(
            "power events require IOKit bindings not yet available; \
             battery status and sleep prevention work normally",
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub fn start(&self) -> Result<(), PowerManagementError> {
        Err(PowerManagementError::unsupported_platform(
            "power events not supported on this platform",
        ))
    }

    /// Stop watching for power events.
    pub fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }
}

impl Default for PowerEventWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create PowerEventWatcher")
    }
}

// ============================================================================
// Platform-specific implementations - Linux
// ============================================================================

#[cfg(target_os = "linux")]
async fn linux_power_event_loop(
    inner: &PowerEventWatcherInner,
) -> Result<(), PowerManagementError> {
    use zbus::Connection;
    use zbus_systemd::login1::ManagerProxy;

    let connection = Connection::system()
        .await
        .map_err(|e| PowerManagementError::power_events(e.to_string()))?;

    let manager = ManagerProxy::new(&connection)
        .await
        .map_err(|e| PowerManagementError::power_events(e.to_string()))?;

    // Subscribe to PrepareForSleep signal
    let mut stream = manager
        .receive_prepare_for_sleep()
        .await
        .map_err(|e| PowerManagementError::power_events(e.to_string()))?;

    use futures_util::Stream;

    while inner.running.load(Ordering::SeqCst) {
        // Poll with a timeout so we can check the running flag
        match futures_util::future::poll_fn(|cx| {
            use std::task::Poll;
            match std::pin::Pin::new(&mut stream).poll_next(cx) {
                Poll::Ready(item) => Poll::Ready(Some(item)),
                Poll::Pending => {
                    // Wake up periodically to check running flag
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        })
        .await
        {
            Some(Some(sig)) => {
                if let Ok(args) = sig.args() {
                    if args.start {
                        inner.sleep_imminent.emit(());
                    } else {
                        inner.wake.emit(());
                    }
                }
            }
            Some(None) | None => break,
        }
    }

    Ok(())
}

// ============================================================================
// Platform-specific implementations - Windows
// ============================================================================

#[cfg(target_os = "windows")]
fn windows_power_event_loop(inner: &PowerEventWatcherInner) -> Result<(), PowerManagementError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
        DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, RegisterClassW,
        TranslateMessage, WM_DESTROY, WM_POWERBROADCAST, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    };
    use windows::core::PCWSTR;

    // Power broadcast constants
    const PBT_APMSUSPEND: u32 = 0x0004;
    const PBT_APMRESUMESUSPEND: u32 = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    // Store a raw pointer to inner in thread-local for the window proc callback
    // This is safe because we ensure the inner outlives the window message loop
    thread_local! {
        static INNER_PTR: std::cell::Cell<*const PowerEventWatcherInner> =
            const { std::cell::Cell::new(std::ptr::null()) };
    }

    // Store pointer to inner for the callback
    INNER_PTR.with(|cell| {
        cell.set(inner as *const _);
    });

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_POWERBROADCAST => {
                INNER_PTR.with(|cell| {
                    let ptr = cell.get();
                    if !ptr.is_null() {
                        let inner = unsafe { &*ptr };
                        match wparam.0 as u32 {
                            PBT_APMSUSPEND => {
                                inner.sleep_imminent.emit(());
                            }
                            PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC => {
                                inner.wake.emit(());
                            }
                            _ => {}
                        }
                    }
                });
                LRESULT(1) // TRUE
            }
            WM_DESTROY => LRESULT(0),
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    unsafe {
        let class_name = to_wide("HorizonLatticePowerWatcher");
        let hinstance = GetModuleHandleW(None)
            .map_err(|e| PowerManagementError::power_events(e.to_string()))?;

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            HINSTANCE(hinstance.0),
            None,
        )
        .map_err(|e| PowerManagementError::power_events(e.to_string()))?;

        let mut msg = MSG::default();
        while inner.running.load(Ordering::SeqCst) {
            // Use PeekMessage to avoid blocking indefinitely
            if PeekMessageW(&mut msg, hwnd, 0, 0, PM_NOREMOVE).as_bool() {
                if !GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                // No message, sleep briefly and check running flag
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let _ = DestroyWindow(hwnd);
    }

    // Clear the pointer
    INNER_PTR.with(|cell| {
        cell.set(std::ptr::null());
    });

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = PowerManagementError::battery_query("test error");
        assert!(error.to_string().contains("battery query"));
        assert!(error.to_string().contains("test error"));

        let error = PowerManagementError::unsupported_platform("test");
        assert!(error.is_unsupported_platform());
    }

    #[test]
    fn test_power_source_default() {
        assert_eq!(PowerSource::default(), PowerSource::Unknown);
    }

    #[test]
    fn test_battery_state_default() {
        assert_eq!(BatteryState::default(), BatteryState::Unknown);
    }

    #[test]
    fn test_battery_info_default() {
        let info = BatteryInfo::default();
        assert_eq!(info.level, 0.0);
        assert_eq!(info.state, BatteryState::Unknown);
        assert!(info.time_to_empty.is_none());
        assert!(info.time_to_full.is_none());
    }

    #[test]
    fn test_sleep_inhibitor_builder() {
        let builder = SleepInhibitor::new()
            .reason("Video playback")
            .app_name("TestApp")
            .prevent_display_sleep(true)
            .prevent_system_sleep(true);

        assert_eq!(builder.reason, "Video playback");
        assert_eq!(builder.app_name, "TestApp");
        assert!(builder.options.display);
        assert!(builder.options.system);
    }

    #[test]
    fn test_sleep_inhibit_options_default() {
        let options = SleepInhibitOptions::default();
        assert!(!options.display);
        assert!(!options.system);
    }

    #[test]
    fn test_power_event_watcher_new() {
        let watcher = PowerEventWatcher::new();
        assert!(watcher.is_ok());
        let watcher = watcher.unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_power_event_reason_variants() {
        let _sleep = PowerEventReason::SleepImminent;
        let _wake = PowerEventReason::Wake;
        let _changed = PowerEventReason::PowerSourceChanged;
    }

    #[cfg(feature = "power-management")]
    #[test]
    fn test_power_state_queries() {
        // These should not panic even on systems without batteries
        let _source = PowerState::power_source();
        let _on_battery = PowerState::is_on_battery();
        let _plugged_in = PowerState::is_plugged_in();
        let _level = PowerState::battery_level();
        let _has_battery = PowerState::has_battery();
    }

    #[cfg(feature = "power-management")]
    #[test]
    fn test_batteries_query() {
        // Should not error even on systems without batteries
        let result = PowerState::batteries();
        assert!(result.is_ok());
    }
}
