//! GraphQL subscription support over WebSocket.
//!
//! Implements the graphql-transport-ws protocol (graphql-ws).
//! See: https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::request::GraphQLRequest;
use super::response::GraphQLResponse;
use crate::error::{NetworkError, Result};

/// WebSocket message types for graphql-transport-ws protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMessage {
    /// Client -> Server: Initialize connection
    ConnectionInit {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    /// Server -> Client: Connection acknowledged
    ConnectionAck {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    /// Client -> Server: Ping
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    /// Server -> Client: Pong
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    /// Client -> Server: Subscribe to operation
    Subscribe {
        id: String,
        payload: SubscribePayload,
    },
    /// Server -> Client: Operation result
    Next {
        id: String,
        payload: GraphQLResponse,
    },
    /// Server -> Client: Operation error
    Error {
        id: String,
        payload: Vec<ErrorPayload>,
    },
    /// Server -> Client: Operation complete
    Complete { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubscribePayload {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<Value>,
    #[serde(rename = "operationName", skip_serializing_if = "Option::is_none")]
    operation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorPayload {
    message: String,
    #[serde(default)]
    locations: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<Value>,
}

/// A message received from a subscription.
#[derive(Debug, Clone)]
pub enum SubscriptionMessage {
    /// Data received from the subscription.
    Data(GraphQLResponse),
    /// The subscription completed normally.
    Complete,
    /// An error occurred.
    Error(String),
}

/// A stream of subscription messages.
pub struct SubscriptionStream {
    receiver: mpsc::Receiver<SubscriptionMessage>,
    subscription_id: String,
    complete_sender: Option<mpsc::Sender<String>>,
}

impl SubscriptionStream {
    /// Get the next message from the subscription.
    pub async fn next(&mut self) -> Option<SubscriptionMessage> {
        self.receiver.recv().await
    }

    /// Stop the subscription.
    pub async fn stop(&mut self) {
        if let Some(sender) = self.complete_sender.take() {
            let _ = sender.send(self.subscription_id.clone()).await;
        }
    }

    /// Get the subscription ID.
    pub fn id(&self) -> &str {
        &self.subscription_id
    }
}

impl Drop for SubscriptionStream {
    fn drop(&mut self) {
        if let Some(sender) = self.complete_sender.take() {
            let id = self.subscription_id.clone();
            tokio::spawn(async move {
                let _ = sender.send(id).await;
            });
        }
    }
}

/// Internal state for the subscription connection.
struct SubscriptionState {
    subscriptions: HashMap<String, mpsc::Sender<SubscriptionMessage>>,
    next_id: AtomicU64,
}

/// Configuration for the subscription connection.
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// WebSocket URL for subscriptions.
    pub url: String,
    /// Connection initialization payload (e.g., auth tokens).
    pub init_payload: Option<Value>,
    /// Connection timeout.
    pub connection_timeout: Duration,
    /// Keep-alive interval.
    pub keep_alive_interval: Option<Duration>,
    /// Additional headers for the WebSocket connection.
    pub headers: HashMap<String, String>,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            init_payload: None,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Some(Duration::from_secs(30)),
            headers: HashMap::new(),
        }
    }
}

/// A GraphQL subscription connection manager.
pub(crate) struct SubscriptionConnection {
    config: SubscriptionConfig,
    state: Arc<Mutex<SubscriptionState>>,
    write_tx: Option<mpsc::Sender<WsMessage>>,
    complete_tx: mpsc::Sender<String>,
    complete_rx: Option<mpsc::Receiver<String>>,
}

impl SubscriptionConnection {
    /// Create a new subscription connection.
    pub fn new(config: SubscriptionConfig) -> Self {
        let (complete_tx, complete_rx) = mpsc::channel(32);
        Self {
            config,
            state: Arc::new(Mutex::new(SubscriptionState {
                subscriptions: HashMap::new(),
                next_id: AtomicU64::new(1),
            })),
            write_tx: None,
            complete_tx,
            complete_rx: Some(complete_rx),
        }
    }

    /// Connect to the WebSocket server.
    pub async fn connect(&mut self) -> Result<()> {
        let url = &self.config.url;

        // Build WebSocket request with custom headers
        let mut request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(url)
            .header("Sec-WebSocket-Protocol", "graphql-transport-ws");

        for (key, value) in &self.config.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let request = request
            .body(())
            .map_err(|e| NetworkError::WebSocket(e.to_string()))?;

        // Connect with timeout
        let connect_future = tokio_tungstenite::connect_async(request);
        let (ws_stream, _) = tokio::time::timeout(self.config.connection_timeout, connect_future)
            .await
            .map_err(|_| NetworkError::Timeout)?
            .map_err(|e| NetworkError::WebSocket(e.to_string()))?;

        let (write, read) = ws_stream.split();

        // Create channels for communication
        let (write_tx, write_rx) = mpsc::channel::<WsMessage>(32);
        self.write_tx = Some(write_tx.clone());

        // Take ownership of complete_rx
        let complete_rx = self.complete_rx.take().unwrap();

        // Spawn write task
        let state = self.state.clone();
        tokio::spawn(Self::write_task(
            write,
            write_rx,
            complete_rx,
            state.clone(),
        ));

        // Spawn read task
        let state = self.state.clone();
        tokio::spawn(Self::read_task(read, state, write_tx.clone()));

        // Send connection init
        let init_msg = WsMessage::ConnectionInit {
            payload: self.config.init_payload.clone(),
        };
        self.send_message(init_msg).await?;

        // Start keep-alive if configured
        if let Some(interval) = self.config.keep_alive_interval {
            let write_tx = write_tx.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(interval);
                loop {
                    interval.tick().await;
                    let ping = WsMessage::Ping { payload: None };
                    if write_tx.send(ping).await.is_err() {
                        break;
                    }
                }
            });
        }

        Ok(())
    }

    /// Subscribe to a GraphQL operation.
    pub async fn subscribe(&self, request: GraphQLRequest) -> Result<SubscriptionStream> {
        let write_tx = self
            .write_tx
            .as_ref()
            .ok_or_else(|| NetworkError::WebSocket("Not connected".into()))?;

        let id = {
            let state = self.state.lock();
            state.next_id.fetch_add(1, Ordering::Relaxed).to_string()
        };

        let (tx, rx) = mpsc::channel(32);

        {
            let mut state = self.state.lock();
            state.subscriptions.insert(id.clone(), tx);
        }

        let subscribe_msg = WsMessage::Subscribe {
            id: id.clone(),
            payload: SubscribePayload {
                query: request.query,
                variables: request.variables,
                operation_name: request.operation_name,
                extensions: request.extensions,
            },
        };

        write_tx
            .send(subscribe_msg)
            .await
            .map_err(|e| NetworkError::WebSocket(e.to_string()))?;

        Ok(SubscriptionStream {
            receiver: rx,
            subscription_id: id,
            complete_sender: Some(self.complete_tx.clone()),
        })
    }

    async fn send_message(&self, msg: WsMessage) -> Result<()> {
        let write_tx = self
            .write_tx
            .as_ref()
            .ok_or_else(|| NetworkError::WebSocket("Not connected".into()))?;

        write_tx
            .send(msg)
            .await
            .map_err(|e| NetworkError::WebSocket(e.to_string()))
    }

    async fn write_task(
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        mut write_rx: mpsc::Receiver<WsMessage>,
        mut complete_rx: mpsc::Receiver<String>,
        state: Arc<Mutex<SubscriptionState>>,
    ) {
        loop {
            tokio::select! {
                msg = write_rx.recv() => {
                    match msg {
                        Some(ws_msg) => {
                            if let Ok(json) = serde_json::to_string(&ws_msg)
                                && write.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                        }
                        None => break,
                    }
                }
                id = complete_rx.recv() => {
                    if let Some(id) = id {
                        // Remove subscription and send complete message
                        {
                            let mut state = state.lock();
                            state.subscriptions.remove(&id);
                        }
                        let complete_msg = WsMessage::Complete { id };
                        if let Ok(json) = serde_json::to_string(&complete_msg) {
                            let _ = write.send(Message::Text(json.into())).await;
                        }
                    }
                }
            }
        }
    }

    async fn read_task(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        state: Arc<Mutex<SubscriptionState>>,
        _write_tx: mpsc::Sender<WsMessage>,
    ) {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        Self::handle_message(ws_msg, &state).await;
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Connection closed, notify all subscriptions
        let mut state = state.lock();
        for (_, tx) in state.subscriptions.drain() {
            let _ = tx.try_send(SubscriptionMessage::Error("Connection closed".into()));
        }
    }

    async fn handle_message(msg: WsMessage, state: &Arc<Mutex<SubscriptionState>>) {
        match msg {
            WsMessage::Next { id, payload } => {
                let state = state.lock();
                if let Some(tx) = state.subscriptions.get(&id) {
                    let _ = tx.try_send(SubscriptionMessage::Data(payload));
                }
            }
            WsMessage::Error { id, payload } => {
                let state = state.lock();
                if let Some(tx) = state.subscriptions.get(&id) {
                    let message = payload
                        .first()
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| "Unknown error".into());
                    let _ = tx.try_send(SubscriptionMessage::Error(message));
                }
            }
            WsMessage::Complete { id } => {
                let mut state = state.lock();
                if let Some(tx) = state.subscriptions.remove(&id) {
                    let _ = tx.try_send(SubscriptionMessage::Complete);
                }
            }
            WsMessage::ConnectionAck { .. } => {
                // Connection acknowledged
                tracing::debug!(target: "horizon_lattice_net::graphql", "Connection acknowledged");
            }
            WsMessage::Pong { .. } => {
                // Keep-alive response
            }
            _ => {}
        }
    }
}
