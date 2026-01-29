//! Tests for TCP client and server functionality.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use horizon_lattice_net::tcp::{
    TcpClient, TcpClientConfig, TcpConnectionState, TcpServer, TcpServerConfig, TcpServerState,
    TcpSocketConfig,
};
use horizon_lattice_net::websocket::ReconnectConfig;

#[test]
fn test_socket_config_builder() {
    let config = TcpSocketConfig::new()
        .no_delay(true)
        .keep_alive(Duration::from_secs(60))
        .read_buffer_size(16384)
        .write_buffer_size(16384)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .write_timeout(Duration::from_secs(30));

    assert!(config.no_delay);
    assert_eq!(config.keep_alive, Some(Duration::from_secs(60)));
    assert_eq!(config.read_buffer_size, 16384);
    assert_eq!(config.write_buffer_size, 16384);
    assert_eq!(config.connect_timeout, Some(Duration::from_secs(10)));
    assert_eq!(config.read_timeout, Some(Duration::from_secs(30)));
    assert_eq!(config.write_timeout, Some(Duration::from_secs(30)));
}

#[test]
fn test_client_config_builder() {
    let config = TcpClientConfig::new("localhost", 8080)
        .no_delay(true)
        .keep_alive(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .auto_reconnect();

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 8080);
    assert_eq!(config.address(), "localhost:8080");
    assert!(config.socket.no_delay);
    assert_eq!(config.socket.keep_alive, Some(Duration::from_secs(30)));
    assert!(config.reconnect.is_some());
}

#[test]
fn test_server_config_builder() {
    let config = TcpServerConfig::new("0.0.0.0", 9000)
        .no_delay(true)
        .backlog(256);

    assert_eq!(config.bind_address, "0.0.0.0");
    assert_eq!(config.port, 9000);
    assert_eq!(config.bind_addr(), "0.0.0.0:9000");
    assert!(config.socket.no_delay);
    assert_eq!(config.backlog, 256);
}

#[test]
fn test_client_initial_state() {
    let config = TcpClientConfig::new("127.0.0.1", 8080);
    let client = TcpClient::new(config);

    assert_eq!(client.state(), TcpConnectionState::Disconnected);
    assert!(!client.is_connected());
    assert_eq!(client.host(), "127.0.0.1");
    assert_eq!(client.port(), 8080);
    assert_eq!(client.address(), "127.0.0.1:8080");
}

#[test]
fn test_server_initial_state() {
    let config = TcpServerConfig::new("127.0.0.1", 0);
    let server = TcpServer::new(config);

    assert_eq!(server.state(), TcpServerState::Stopped);
    assert!(!server.is_listening());
    assert_eq!(server.connection_count(), 0);
    assert!(server.connections().is_empty());
}

#[test]
fn test_send_before_connect_fails() {
    let config = TcpClientConfig::new("127.0.0.1", 8080);
    let client = TcpClient::new(config);

    let result = client.send(b"test data");
    assert!(result.is_err());
}

#[test]
fn test_reconnect_config_builder() {
    let config = ReconnectConfig::new()
        .max_attempts(5)
        .initial_delay(Duration::from_millis(500))
        .max_delay(Duration::from_secs(30))
        .backoff_multiplier(1.5);

    assert_eq!(config.max_attempts, Some(5));
    assert_eq!(config.initial_delay, Duration::from_millis(500));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.backoff_multiplier, 1.5);
}

#[tokio::test]
async fn test_client_server_echo() {
    // Start server
    let server_config = TcpServerConfig::new("127.0.0.1", 0);
    let server = TcpServer::new(server_config);

    let server_started = Arc::new(AtomicBool::new(false));
    let server_started_clone = server_started.clone();

    server.started.connect(move |()| {
        server_started_clone.store(true, Ordering::SeqCst);
    });

    // Echo back any received data
    server.new_connection.connect(|conn| {
        let conn_clone = conn.clone();
        conn.data_received.connect(move |data| {
            let _ = conn_clone.send(data.clone());
        });
    });

    server.start();

    // Wait for server to start
    for _ in 0..100 {
        if server.is_listening() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(server.is_listening());

    // Get the port the server bound to
    let local_addr = server
        .local_addr()
        .expect("Server should have local address");
    let port = local_addr.port();

    // Create client
    let client_config = TcpClientConfig::new("127.0.0.1", port).no_delay(true);
    let client = TcpClient::new(client_config);

    let client_connected = Arc::new(AtomicBool::new(false));
    let client_connected_clone = client_connected.clone();

    let received_data: Arc<parking_lot::Mutex<Vec<u8>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    client.connected.connect(move |()| {
        client_connected_clone.store(true, Ordering::SeqCst);
    });

    client.data_received.connect(move |data| {
        received_data_clone.lock().extend(data);
    });

    client.connect();

    // Wait for client to connect
    for _ in 0..100 {
        if client.is_connected() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(client.is_connected());

    // Send test data
    let test_data = b"Hello, TCP Server!";
    client.send(test_data).unwrap();

    // Wait for echo response
    for _ in 0..100 {
        if received_data.lock().len() >= test_data.len() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let received = received_data.lock().clone();
    assert_eq!(received, test_data);

    // Cleanup
    client.disconnect();
    server.stop();
}

#[tokio::test]
async fn test_multiple_clients() {
    // Start server
    let server_config = TcpServerConfig::new("127.0.0.1", 0).no_delay(true);
    let server = TcpServer::new(server_config);

    let connection_count = Arc::new(AtomicUsize::new(0));
    let connection_count_clone = connection_count.clone();

    server.new_connection.connect(move |_conn| {
        connection_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    server.start();

    // Wait for server to start
    for _ in 0..100 {
        if server.is_listening() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let local_addr = server
        .local_addr()
        .expect("Server should have local address");
    let port = local_addr.port();

    // Create multiple clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client_config = TcpClientConfig::new("127.0.0.1", port);
        let client = TcpClient::new(client_config);
        client.connect();
        clients.push(client);
    }

    // Wait for all clients to connect
    for _ in 0..100 {
        if clients.iter().all(|c| c.is_connected()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Verify all connected
    assert!(clients.iter().all(|c| c.is_connected()));
    assert_eq!(connection_count.load(Ordering::SeqCst), 3);
    assert_eq!(server.connection_count(), 3);

    // Cleanup
    for client in &clients {
        client.disconnect();
    }
    server.stop();
}

#[tokio::test]
async fn test_broadcast() {
    let server_config = TcpServerConfig::new("127.0.0.1", 0);
    let server = TcpServer::new(server_config);

    server.start();

    for _ in 0..100 {
        if server.is_listening() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let local_addr = server
        .local_addr()
        .expect("Server should have local address");
    let port = local_addr.port();

    // Create two clients with their own receive buffers
    let received1: Arc<parking_lot::Mutex<Vec<u8>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let received2: Arc<parking_lot::Mutex<Vec<u8>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let client1_config = TcpClientConfig::new("127.0.0.1", port);
    let client1 = TcpClient::new(client1_config);
    let received1_clone = received1.clone();
    client1.data_received.connect(move |data| {
        received1_clone.lock().extend(data);
    });
    client1.connect();

    let client2_config = TcpClientConfig::new("127.0.0.1", port);
    let client2 = TcpClient::new(client2_config);
    let received2_clone = received2.clone();
    client2.data_received.connect(move |data| {
        received2_clone.lock().extend(data);
    });
    client2.connect();

    // Wait for both to connect
    for _ in 0..100 {
        if client1.is_connected() && client2.is_connected() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Broadcast message
    let broadcast_msg = b"Broadcast message!";
    server.broadcast(broadcast_msg);

    // Wait for both to receive
    for _ in 0..100 {
        let r1_len = received1.lock().len();
        let r2_len = received2.lock().len();
        if r1_len >= broadcast_msg.len() && r2_len >= broadcast_msg.len() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(&*received1.lock(), broadcast_msg);
    assert_eq!(&*received2.lock(), broadcast_msg);

    // Cleanup
    client1.disconnect();
    client2.disconnect();
    server.stop();
}

#[tokio::test]
async fn test_graceful_disconnect() {
    let server_config = TcpServerConfig::new("127.0.0.1", 0);
    let server = TcpServer::new(server_config);

    let connection_closed = Arc::new(AtomicBool::new(false));
    let connection_closed_clone = connection_closed.clone();

    server.connection_closed.connect(move |_id| {
        connection_closed_clone.store(true, Ordering::SeqCst);
    });

    server.start();

    for _ in 0..100 {
        if server.is_listening() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let local_addr = server
        .local_addr()
        .expect("Server should have local address");
    let port = local_addr.port();

    let client_config = TcpClientConfig::new("127.0.0.1", port);
    let client = TcpClient::new(client_config);

    let client_disconnected = Arc::new(AtomicBool::new(false));
    let client_disconnected_clone = client_disconnected.clone();

    client.disconnected.connect(move |()| {
        client_disconnected_clone.store(true, Ordering::SeqCst);
    });

    client.connect();

    for _ in 0..100 {
        if client.is_connected() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(client.is_connected());
    assert_eq!(server.connection_count(), 1);

    // Disconnect client
    client.disconnect();

    // Wait for disconnect signals
    for _ in 0..100 {
        if client_disconnected.load(Ordering::SeqCst) && connection_closed.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(client_disconnected.load(Ordering::SeqCst));
    assert!(connection_closed.load(Ordering::SeqCst));
    assert!(!client.is_connected());

    server.stop();
}

#[test]
fn test_connection_state_display() {
    assert_eq!(TcpConnectionState::Disconnected.to_string(), "Disconnected");
    assert_eq!(TcpConnectionState::Connecting.to_string(), "Connecting");
    assert_eq!(TcpConnectionState::Connected.to_string(), "Connected");
    assert_eq!(TcpConnectionState::Reconnecting.to_string(), "Reconnecting");
    assert_eq!(TcpConnectionState::Closing.to_string(), "Closing");
}

#[test]
fn test_server_state_display() {
    assert_eq!(TcpServerState::Stopped.to_string(), "Stopped");
    assert_eq!(TcpServerState::Starting.to_string(), "Starting");
    assert_eq!(TcpServerState::Listening.to_string(), "Listening");
    assert_eq!(TcpServerState::Stopping.to_string(), "Stopping");
}
