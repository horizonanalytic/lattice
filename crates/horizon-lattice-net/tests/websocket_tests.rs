//! WebSocket client tests.
//!
//! Note: These tests require network access and a running WebSocket echo server.

use horizon_lattice_net::websocket::{CloseCode, CloseReason, ReconnectConfig, WebSocketConfig};
use std::time::Duration;

#[test]
fn test_websocket_config_builder() {
    let config = WebSocketConfig::new("wss://echo.websocket.org")
        .header("Authorization", "Bearer token")
        .header("X-Custom", "value")
        .auto_reconnect();

    assert_eq!(config.url, "wss://echo.websocket.org");
    assert_eq!(config.headers.len(), 2);
    assert_eq!(
        config.headers.get("Authorization"),
        Some(&"Bearer token".to_string())
    );
    assert!(config.reconnect.is_some());
}

#[test]
fn test_reconnect_config() {
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

#[test]
fn test_close_code_conversion() {
    assert_eq!(CloseCode::Normal.as_u16(), 1000);
    assert_eq!(CloseCode::Away.as_u16(), 1001);
    assert_eq!(CloseCode::Protocol.as_u16(), 1002);
    assert_eq!(CloseCode::Error.as_u16(), 1011);
    assert_eq!(CloseCode::Custom(4000).as_u16(), 4000);

    assert_eq!(CloseCode::from_u16(1000), CloseCode::Normal);
    assert_eq!(CloseCode::from_u16(1001), CloseCode::Away);
    assert_eq!(CloseCode::from_u16(4001), CloseCode::Custom(4001));
}

#[test]
fn test_close_reason() {
    let reason = CloseReason::normal();
    assert_eq!(reason.code, CloseCode::Normal);
    assert!(reason.reason.is_none());

    let reason = CloseReason::with_reason(CloseCode::Away, "Server shutting down");
    assert_eq!(reason.code, CloseCode::Away);
    assert_eq!(reason.reason, Some("Server shutting down".to_string()));
}

#[test]
fn test_websocket_client_creation() {
    use horizon_lattice_net::websocket::{WebSocketClient, WebSocketState};

    let config = WebSocketConfig::new("ws://localhost:8080");
    let client = WebSocketClient::new(config);

    assert_eq!(client.state(), WebSocketState::Disconnected);
    assert!(!client.is_connected());
    assert_eq!(client.url(), "ws://localhost:8080");
}

#[test]
fn test_send_before_connect_fails() {
    use horizon_lattice_net::websocket::WebSocketClient;

    let config = WebSocketConfig::new("ws://localhost:8080");
    let client = WebSocketClient::new(config);

    // Sending before connect should fail
    let result = client.send_text("Hello");
    assert!(result.is_err());

    let result = client.send_binary(vec![1, 2, 3]);
    assert!(result.is_err());
}
