//! Network change monitoring.

use std::net::IpAddr;
use std::sync::Arc;

use horizon_lattice_core::signal::Signal;
use parking_lot::Mutex;

use super::interface::NetworkInterface;
use crate::error::{NetworkError, Result};

/// Event describing a change in network interfaces.
#[derive(Debug, Clone)]
pub struct InterfaceEvent {
    /// Interfaces that were added.
    pub added: Vec<InterfaceChange>,
    /// Interfaces that were removed.
    pub removed: Vec<InterfaceChange>,
    /// Current list of all interfaces after the change.
    pub current_interfaces: Vec<NetworkInterface>,
}

/// Information about an interface change.
#[derive(Debug, Clone)]
pub struct InterfaceChange {
    /// Index of the interface.
    pub interface_index: u32,
    /// Name of the interface (if available).
    pub interface_name: Option<String>,
    /// IP addresses associated with the change.
    pub addresses: Vec<IpAddr>,
}

/// Monitors network changes and emits signals.
///
/// The `NetworkMonitor` watches for network interface changes using
/// platform-specific APIs and emits signals when changes occur.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::network_info::NetworkMonitor;
///
/// let monitor = NetworkMonitor::new()?;
///
/// // Watch for online/offline changes
/// monitor.online_state_changed.connect(|is_online| {
///     if is_online {
///         println!("Network is now online");
///     } else {
///         println!("Network is now offline");
///     }
/// });
///
/// // Watch for interface changes
/// monitor.interface_changed.connect(|event| {
///     for added in &event.added {
///         println!("Interface added: {:?}", added.interface_name);
///     }
///     for removed in &event.removed {
///         println!("Interface removed: {:?}", removed.interface_name);
///     }
/// });
///
/// // Start monitoring (runs in background)
/// monitor.start()?;
/// ```
pub struct NetworkMonitor {
    /// Signal emitted when online/offline state changes.
    pub online_state_changed: Arc<Signal<bool>>,

    /// Signal emitted when network interfaces change.
    pub interface_changed: Arc<Signal<InterfaceEvent>>,

    /// Internal state.
    inner: Arc<Mutex<MonitorInner>>,
}

struct MonitorInner {
    /// Whether monitoring is active.
    is_running: bool,
    /// Current online state.
    is_online: bool,
    /// Handle to stop the watcher (drop to stop).
    _watcher_handle: Option<netwatcher::WatchHandle>,
}

impl NetworkMonitor {
    /// Create a new network monitor.
    pub fn new() -> Result<Self> {
        // Check initial online state by looking for non-loopback interfaces with addresses
        let is_online = check_online_state();

        Ok(Self {
            online_state_changed: Arc::new(Signal::new()),
            interface_changed: Arc::new(Signal::new()),
            inner: Arc::new(Mutex::new(MonitorInner {
                is_running: false,
                is_online,
                _watcher_handle: None,
            })),
        })
    }

    /// Check if the network is currently online.
    ///
    /// This returns `true` if there's at least one non-loopback interface
    /// with an IP address assigned.
    pub fn is_online(&self) -> bool {
        self.inner.lock().is_online
    }

    /// Start monitoring for network changes.
    ///
    /// This starts a background watcher that will emit signals when
    /// network interfaces change. The watcher uses platform-native APIs
    /// for efficient change detection.
    pub fn start(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        if inner.is_running {
            return Ok(());
        }

        let online_signal = Arc::clone(&self.online_state_changed);
        let interface_signal = Arc::clone(&self.interface_changed);
        let inner_clone = Arc::clone(&self.inner);

        // Start the netwatcher
        let handle = netwatcher::watch_interfaces(move |update| {
            // Build the interface change event from the diff
            // The diff contains interface indices (u32) for added/removed interfaces
            let added: Vec<InterfaceChange> = update
                .diff
                .added
                .iter()
                .map(|&ifindex| {
                    // Try to find the interface in the current snapshot
                    let iface = update.interfaces.get(&ifindex);
                    let name = iface.map(|i| i.name.clone());
                    // netwatcher's ips field is Vec<IpAddr>
                    let addresses: Vec<IpAddr> = iface.map(|i| i.ips.clone()).unwrap_or_default();

                    InterfaceChange {
                        interface_index: ifindex,
                        interface_name: name,
                        addresses,
                    }
                })
                .collect();

            let removed: Vec<InterfaceChange> = update
                .diff
                .removed
                .iter()
                .map(|&ifindex| {
                    // Removed interfaces won't be in the current snapshot
                    InterfaceChange {
                        interface_index: ifindex,
                        interface_name: None,
                        addresses: Vec::new(),
                    }
                })
                .collect();

            // Get current interfaces using our wrapper
            let current_interfaces = NetworkInterface::list();

            // Check online state
            let new_online_state = check_online_state();

            // Update internal state and emit signals
            {
                let mut guard = inner_clone.lock();
                let old_online_state = guard.is_online;
                guard.is_online = new_online_state;

                // Emit online state change if changed
                if old_online_state != new_online_state {
                    drop(guard); // Release lock before emitting
                    online_signal.emit(new_online_state);
                }
            }

            // Emit interface change event if there were any changes
            if !added.is_empty() || !removed.is_empty() {
                interface_signal.emit(InterfaceEvent {
                    added,
                    removed,
                    current_interfaces,
                });
            }
        })
        .map_err(|e| NetworkError::Io(e.to_string()))?;

        inner._watcher_handle = Some(handle);
        inner.is_running = true;

        Ok(())
    }

    /// Stop monitoring for network changes.
    pub fn stop(&self) {
        let mut inner = self.inner.lock();
        inner._watcher_handle = None;
        inner.is_running = false;
    }

    /// Check if the monitor is currently running.
    pub fn is_running(&self) -> bool {
        self.inner.lock().is_running
    }

    /// Get the current list of network interfaces.
    ///
    /// This is a convenience method that returns the same result as
    /// `NetworkInterface::list()`.
    pub fn interfaces(&self) -> Vec<NetworkInterface> {
        NetworkInterface::list()
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create network monitor")
    }
}

impl Drop for NetworkMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Check if the system appears to be online.
///
/// Returns `true` if there's at least one non-loopback interface
/// that is up and has at least one IP address assigned.
fn check_online_state() -> bool {
    NetworkInterface::list()
        .iter()
        .any(|iface| iface.is_up && !iface.is_loopback() && iface.has_addresses())
}

/// Perform a network connectivity check.
///
/// This attempts to establish a connection to well-known endpoints
/// to verify actual internet connectivity.
///
/// # Arguments
///
/// * `timeout_secs` - Optional timeout in seconds (default: 5 seconds)
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::network_info::check_connectivity;
///
/// let is_connected = check_connectivity(Some(3)).await;
/// if is_connected {
///     println!("Internet connection is available");
/// }
/// ```
pub async fn check_connectivity(timeout_secs: Option<u64>) -> bool {
    use std::time::Duration;
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(5));

    // Try to connect to well-known endpoints
    let endpoints = [
        ("1.1.1.1", 80),        // Cloudflare
        ("8.8.8.8", 53),        // Google DNS
        ("208.67.222.222", 53), // OpenDNS
    ];

    for (host, port) in endpoints {
        let addr = format!("{}:{}", host, port);
        if let Ok(Ok(_)) = timeout(timeout_duration, TcpStream::connect(&addr)).await {
            return true;
        }
    }

    false
}
