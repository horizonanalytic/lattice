//! Network information tests.

use horizon_lattice_net::network_info::{
    InterfaceType, MacAddress, NetworkInterface, NetworkMonitor,
};

#[test]
fn test_list_interfaces() {
    let interfaces = NetworkInterface::list();
    // Should have at least loopback interface on any system
    assert!(!interfaces.is_empty(), "Should have at least one network interface");

    // Should have a loopback interface
    let has_loopback = interfaces.iter().any(|iface| iface.is_loopback());
    assert!(has_loopback, "Should have a loopback interface");
}

#[test]
fn test_loopback_has_addresses() {
    let interfaces = NetworkInterface::list();
    let loopback = interfaces.iter().find(|iface| iface.is_loopback());

    if let Some(lo) = loopback {
        assert!(lo.has_addresses(), "Loopback should have addresses");
        assert!(lo.is_up, "Loopback should be up");
        assert_eq!(lo.interface_type, InterfaceType::Loopback);
    }
}

#[test]
fn test_default_interface() {
    // This might fail in isolated environments without network
    let default = NetworkInterface::default_interface();
    // Just test that the function doesn't panic
    if let Some(iface) = default {
        assert!(!iface.name.is_empty());
    }
}

#[test]
fn test_mac_address_display() {
    let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    assert_eq!(mac.to_string(), "AA:BB:CC:DD:EE:FF");

    let mac2 = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    assert_eq!(mac2.to_string(), "00:11:22:33:44:55");
}

#[test]
fn test_interface_all_addresses() {
    let interfaces = NetworkInterface::list();
    for iface in &interfaces {
        let all_addrs = iface.all_addresses();
        let v4_count = iface.ipv4_addresses.len();
        let v6_count = iface.ipv6_addresses.len();
        assert_eq!(all_addrs.len(), v4_count + v6_count);
    }
}

#[test]
fn test_network_monitor_creation() {
    let monitor = NetworkMonitor::new();
    assert!(monitor.is_ok(), "Should be able to create network monitor");

    let monitor = monitor.unwrap();
    assert!(!monitor.is_running(), "Monitor should not be running initially");
}

#[test]
fn test_network_monitor_online_state() {
    let monitor = NetworkMonitor::new().expect("Failed to create monitor");
    // Just verify it doesn't panic - actual state depends on system
    let _is_online = monitor.is_online();
}

#[test]
fn test_network_monitor_start_stop() {
    let monitor = NetworkMonitor::new().expect("Failed to create monitor");

    // Start monitoring
    let result = monitor.start();
    assert!(result.is_ok(), "Should be able to start monitoring");
    assert!(monitor.is_running(), "Monitor should be running after start");

    // Starting again should be a no-op
    let result = monitor.start();
    assert!(result.is_ok(), "Starting again should succeed");

    // Stop monitoring
    monitor.stop();
    assert!(!monitor.is_running(), "Monitor should not be running after stop");
}

#[test]
fn test_network_monitor_interfaces() {
    let monitor = NetworkMonitor::new().expect("Failed to create monitor");
    let interfaces = monitor.interfaces();

    // Should match NetworkInterface::list()
    let direct_list = NetworkInterface::list();
    assert_eq!(interfaces.len(), direct_list.len());
}

#[tokio::test]
async fn test_check_connectivity() {
    use horizon_lattice_net::network_info::check_connectivity;

    // This test may fail in isolated environments without network access
    // We just verify the function doesn't panic and returns a bool
    let _is_connected = check_connectivity(Some(2)).await;
}
