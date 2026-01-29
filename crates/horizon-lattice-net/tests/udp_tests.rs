//! Tests for UDP socket functionality.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use horizon_lattice_net::udp::{
    Datagram, MulticastConfig, UdpSocket, UdpSocketConfig, UdpSocketState,
};

#[test]
fn test_config_builder() {
    let config = UdpSocketConfig::new("0.0.0.0", 8080)
        .broadcast(true)
        .recv_buffer_size(32768);

    assert_eq!(config.bind_address, "0.0.0.0");
    assert_eq!(config.port, 8080);
    assert_eq!(config.bind_addr(), "0.0.0.0:8080");
    assert!(config.broadcast);
    assert_eq!(config.recv_buffer_size, 32768);
}

#[test]
fn test_any_address_config() {
    let config = UdpSocketConfig::any_address(5000);
    assert_eq!(config.bind_address, "0.0.0.0");
    assert_eq!(config.port, 5000);
}

#[test]
fn test_multicast_config() {
    let multicast_addr: Ipv4Addr = "239.255.0.1".parse().unwrap();
    let interface: Ipv4Addr = "192.168.1.1".parse().unwrap();

    let config = MulticastConfig::new()
        .join_group(multicast_addr)
        .join_group_on("239.255.0.2".parse().unwrap(), interface)
        .loopback(true)
        .ttl(5);

    assert_eq!(config.groups.len(), 2);
    assert_eq!(config.groups[0], (multicast_addr, None));
    assert_eq!(
        config.groups[1],
        ("239.255.0.2".parse().unwrap(), Some(interface))
    );
    assert!(config.loopback);
    assert_eq!(config.ttl, 5);
}

#[test]
fn test_socket_initial_state() {
    let config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket = UdpSocket::new(config);

    assert_eq!(socket.state(), UdpSocketState::Unbound);
    assert!(!socket.is_bound());
    assert!(socket.local_addr().is_none());
}

#[test]
fn test_send_before_bind_fails() {
    let config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket = UdpSocket::new(config);

    let result = socket.send_to(b"test data", "127.0.0.1:9999".parse().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_socket_state_display() {
    assert_eq!(UdpSocketState::Unbound.to_string(), "Unbound");
    assert_eq!(UdpSocketState::Binding.to_string(), "Binding");
    assert_eq!(UdpSocketState::Bound.to_string(), "Bound");
    assert_eq!(UdpSocketState::Closing.to_string(), "Closing");
    assert_eq!(UdpSocketState::Closed.to_string(), "Closed");
}

#[test]
fn test_datagram_creation() {
    let data = vec![1, 2, 3, 4];
    let source: SocketAddr = "192.168.1.100:5000".parse().unwrap();
    let datagram = Datagram::new(data.clone(), source);

    assert_eq!(datagram.data, data);
    assert_eq!(datagram.source, source);
}

#[tokio::test]
async fn test_socket_bind() {
    let config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket = UdpSocket::new(config);

    let bound = Arc::new(AtomicBool::new(false));
    let bound_clone = bound.clone();

    let bound_addr: Arc<parking_lot::Mutex<Option<SocketAddr>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let bound_addr_clone = bound_addr.clone();

    socket.bound.connect(move |addr| {
        bound_clone.store(true, Ordering::SeqCst);
        *bound_addr_clone.lock() = Some(*addr);
    });

    socket.bind();

    // Wait for bind
    for _ in 0..100 {
        if socket.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(socket.is_bound());
    assert!(bound.load(Ordering::SeqCst));
    assert!(socket.local_addr().is_some());

    let addr = bound_addr.lock().unwrap();
    assert_eq!(socket.local_addr(), Some(addr));

    socket.close();
}

#[tokio::test]
async fn test_send_receive() {
    // Create sender socket
    let sender_config = UdpSocketConfig::new("127.0.0.1", 0);
    let sender = UdpSocket::new(sender_config);
    sender.bind();

    // Create receiver socket
    let receiver_config = UdpSocketConfig::new("127.0.0.1", 0);
    let receiver = UdpSocket::new(receiver_config);

    let received_data: Arc<parking_lot::Mutex<Vec<u8>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    let received_from: Arc<parking_lot::Mutex<Option<SocketAddr>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let received_from_clone = received_from.clone();

    receiver.datagram_received.connect(move |datagram| {
        *received_data_clone.lock() = datagram.data.clone();
        *received_from_clone.lock() = Some(datagram.source);
    });

    receiver.bind();

    // Wait for both to bind
    for _ in 0..100 {
        if sender.is_bound() && receiver.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(sender.is_bound());
    assert!(receiver.is_bound());

    let receiver_addr = receiver.local_addr().unwrap();
    let sender_addr = sender.local_addr().unwrap();

    // Send data
    let test_data = b"Hello, UDP!";
    sender.send_to(test_data, receiver_addr).unwrap();

    // Wait for receive
    for _ in 0..100 {
        if !received_data.lock().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let data = received_data.lock().clone();
    assert_eq!(data, test_data);

    let from = received_from.lock().unwrap();
    assert_eq!(from, sender_addr);

    // Cleanup
    sender.close();
    receiver.close();
}

#[tokio::test]
async fn test_bidirectional_communication() {
    // Create two sockets
    let socket1_config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket1 = UdpSocket::new(socket1_config);

    let socket2_config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket2 = UdpSocket::new(socket2_config);

    let received1: Arc<parking_lot::Mutex<Vec<u8>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let received1_clone = received1.clone();

    let received2: Arc<parking_lot::Mutex<Vec<u8>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let received2_clone = received2.clone();

    socket1.datagram_received.connect(move |datagram| {
        received1_clone.lock().extend(&datagram.data);
    });

    socket2.datagram_received.connect(move |datagram| {
        received2_clone.lock().extend(&datagram.data);
    });

    socket1.bind();
    socket2.bind();

    // Wait for bind
    for _ in 0..100 {
        if socket1.is_bound() && socket2.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let addr1 = socket1.local_addr().unwrap();
    let addr2 = socket2.local_addr().unwrap();

    // Send from socket1 to socket2
    socket1.send_to(b"From socket 1", addr2).unwrap();

    // Send from socket2 to socket1
    socket2.send_to(b"From socket 2", addr1).unwrap();

    // Wait for both to receive
    for _ in 0..100 {
        let r1 = !received1.lock().is_empty();
        let r2 = !received2.lock().is_empty();
        if r1 && r2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(&*received1.lock(), b"From socket 2");
    assert_eq!(&*received2.lock(), b"From socket 1");

    socket1.close();
    socket2.close();
}

#[tokio::test]
async fn test_datagram_sent_signal() {
    let config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket = UdpSocket::new(config);

    let bytes_sent = Arc::new(AtomicUsize::new(0));
    let bytes_sent_clone = bytes_sent.clone();

    socket.datagram_sent.connect(move |count| {
        bytes_sent_clone.fetch_add(*count, Ordering::SeqCst);
    });

    socket.bind();

    for _ in 0..100 {
        if socket.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Send data (to a random port, doesn't matter if it's received)
    let test_data = b"Test message";
    socket
        .send_to(test_data, "127.0.0.1:9999".parse().unwrap())
        .unwrap();

    // Wait for sent signal
    for _ in 0..100 {
        if bytes_sent.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(bytes_sent.load(Ordering::SeqCst), test_data.len());

    socket.close();
}

#[tokio::test]
async fn test_close_signal() {
    let config = UdpSocketConfig::new("127.0.0.1", 0);
    let socket = UdpSocket::new(config);

    let closed = Arc::new(AtomicBool::new(false));
    let closed_clone = closed.clone();

    socket.closed.connect(move |()| {
        closed_clone.store(true, Ordering::SeqCst);
    });

    socket.bind();

    for _ in 0..100 {
        if socket.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    socket.close();

    for _ in 0..100 {
        if closed.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(closed.load(Ordering::SeqCst));
    assert_eq!(socket.state(), UdpSocketState::Closed);
}

#[tokio::test]
async fn test_broadcast_send() {
    let config = UdpSocketConfig::new("127.0.0.1", 0).broadcast(true);

    let socket = UdpSocket::new(config);
    socket.bind();

    for _ in 0..100 {
        if socket.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(socket.is_bound());

    // Just verify we can call set_broadcast without error
    socket.set_broadcast(true).unwrap();

    socket.close();
}

#[tokio::test]
async fn test_multiple_datagrams() {
    let sender_config = UdpSocketConfig::new("127.0.0.1", 0);
    let sender = UdpSocket::new(sender_config);

    let receiver_config = UdpSocketConfig::new("127.0.0.1", 0);
    let receiver = UdpSocket::new(receiver_config);

    let message_count = Arc::new(AtomicUsize::new(0));
    let message_count_clone = message_count.clone();

    receiver.datagram_received.connect(move |_datagram| {
        message_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    sender.bind();
    receiver.bind();

    for _ in 0..100 {
        if sender.is_bound() && receiver.is_bound() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let receiver_addr = receiver.local_addr().unwrap();

    // Send multiple datagrams
    for i in 0..5 {
        sender
            .send_to(format!("Message {}", i).as_bytes(), receiver_addr)
            .unwrap();
    }

    // Wait for all to be received
    for _ in 0..100 {
        if message_count.load(Ordering::SeqCst) >= 5 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(message_count.load(Ordering::SeqCst), 5);

    sender.close();
    receiver.close();
}
