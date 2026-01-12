//! Timer system for Horizon Lattice.
//!
//! Provides one-shot and repeating timers that integrate with the event loop.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::error::{Result, TimerError};
use crate::event::LatticeEvent;

new_key_type! {
    /// A unique identifier for a timer.
    pub struct TimerId;
}

/// The type of timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerKind {
    /// Fires once after the specified duration.
    OneShot,
    /// Fires repeatedly at the specified interval.
    Repeating,
}

/// Internal timer data.
#[derive(Debug)]
struct TimerData {
    /// When this timer should next fire.
    next_fire: Instant,
    /// The interval for repeating timers.
    interval: Duration,
    /// The kind of timer.
    kind: TimerKind,
    /// Whether this timer is active.
    active: bool,
}

/// An entry in the timer queue (min-heap by fire time).
#[derive(Debug, Clone, Copy)]
struct TimerQueueEntry {
    id: TimerId,
    fire_time: Instant,
}

impl PartialEq for TimerQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.fire_time == other.fire_time
    }
}

impl Eq for TimerQueueEntry {}

impl PartialOrd for TimerQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimerQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (BinaryHeap is max-heap by default).
        other.fire_time.cmp(&self.fire_time)
    }
}

/// Manages all timers for the application.
#[allow(dead_code)]
pub struct TimerManager {
    /// All registered timers.
    timers: SlotMap<TimerId, TimerData>,
    /// Priority queue of pending timer fires (min-heap by fire time).
    queue: BinaryHeap<TimerQueueEntry>,
}

#[allow(dead_code)]
impl TimerManager {
    /// Create a new timer manager.
    pub fn new() -> Self {
        Self {
            timers: SlotMap::with_key(),
            queue: BinaryHeap::new(),
        }
    }

    /// Start a one-shot timer that fires after the specified duration.
    ///
    /// Returns the timer ID that can be used to cancel the timer.
    pub fn start_one_shot(&mut self, duration: Duration) -> TimerId {
        let now = Instant::now();
        let next_fire = now + duration;

        let data = TimerData {
            next_fire,
            interval: duration,
            kind: TimerKind::OneShot,
            active: true,
        };

        let id = self.timers.insert(data);
        self.queue.push(TimerQueueEntry {
            id,
            fire_time: next_fire,
        });

        id
    }

    /// Start a repeating timer that fires at the specified interval.
    ///
    /// The first fire occurs after `interval` duration.
    /// Returns the timer ID that can be used to cancel the timer.
    pub fn start_repeating(&mut self, interval: Duration) -> TimerId {
        let now = Instant::now();
        let next_fire = now + interval;

        let data = TimerData {
            next_fire,
            interval,
            kind: TimerKind::Repeating,
            active: true,
        };

        let id = self.timers.insert(data);
        self.queue.push(TimerQueueEntry {
            id,
            fire_time: next_fire,
        });

        id
    }

    /// Stop and remove a timer.
    ///
    /// Returns `Ok(())` if the timer was found and removed, or an error if not found.
    pub fn stop(&mut self, id: TimerId) -> Result<()> {
        if let Some(timer) = self.timers.get_mut(id) {
            timer.active = false;
            self.timers.remove(id);
            Ok(())
        } else {
            Err(TimerError::InvalidTimerId.into())
        }
    }

    /// Check if a timer is currently active.
    pub fn is_active(&self, id: TimerId) -> bool {
        self.timers.get(id).is_some_and(|t| t.active)
    }

    /// Get the duration until the next timer fires, if any.
    ///
    /// Returns `None` if there are no active timers.
    pub fn time_until_next(&mut self) -> Option<Duration> {
        // Clean up any inactive timers from the front of the queue.
        while let Some(entry) = self.queue.peek() {
            if !self.timers.get(entry.id).is_some_and(|t| t.active) {
                self.queue.pop();
            } else {
                break;
            }
        }

        self.queue.peek().map(|entry| {
            let now = Instant::now();
            if entry.fire_time > now {
                entry.fire_time - now
            } else {
                Duration::ZERO
            }
        })
    }

    /// Process all timers that should fire now.
    ///
    /// Returns a list of timer events to dispatch.
    #[tracing::instrument(skip(self), target = "horizon_lattice_core::timer", level = "trace")]
    pub fn process_expired(&mut self) -> Vec<LatticeEvent> {
        let now = Instant::now();
        let mut events = Vec::new();

        while let Some(entry) = self.queue.peek() {
            // Check if this timer should fire.
            if entry.fire_time > now {
                break;
            }

            let entry = self.queue.pop().unwrap();
            let id = entry.id;

            // Check if timer is still active.
            let Some(timer) = self.timers.get_mut(id) else {
                continue;
            };

            if !timer.active {
                continue;
            }

            // Timer has fired.
            tracing::trace!(target: "horizon_lattice_core::timer", ?id, "timer fired");
            events.push(LatticeEvent::Timer { id });

            match timer.kind {
                TimerKind::OneShot => {
                    // One-shot timers are removed after firing.
                    timer.active = false;
                    self.timers.remove(id);
                }
                TimerKind::Repeating => {
                    // Schedule the next fire.
                    timer.next_fire = now + timer.interval;
                    self.queue.push(TimerQueueEntry {
                        id,
                        fire_time: timer.next_fire,
                    });
                }
            }
        }

        events
    }

    /// Get the number of active timers.
    pub fn active_count(&self) -> usize {
        self.timers.iter().filter(|(_, t)| t.active).count()
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe wrapper around `TimerManager` for use from the application.
pub(crate) struct SharedTimerManager {
    inner: Mutex<TimerManager>,
}

#[allow(dead_code)]
impl SharedTimerManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TimerManager::new()),
        }
    }

    pub fn start_one_shot(&self, duration: Duration) -> TimerId {
        self.inner.lock().start_one_shot(duration)
    }

    pub fn start_repeating(&self, interval: Duration) -> TimerId {
        self.inner.lock().start_repeating(interval)
    }

    pub fn stop(&self, id: TimerId) -> Result<()> {
        self.inner.lock().stop(id)
    }

    pub fn is_active(&self, id: TimerId) -> bool {
        self.inner.lock().is_active(id)
    }

    pub fn time_until_next(&self) -> Option<Duration> {
        self.inner.lock().time_until_next()
    }

    pub fn process_expired(&self) -> Vec<LatticeEvent> {
        self.inner.lock().process_expired()
    }

    pub fn active_count(&self) -> usize {
        self.inner.lock().active_count()
    }
}

impl Default for SharedTimerManager {
    fn default() -> Self {
        Self::new()
    }
}
