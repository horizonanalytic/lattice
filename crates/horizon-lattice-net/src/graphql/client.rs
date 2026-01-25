//! GraphQL client implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;

use super::request::{GraphQLRequest, INTROSPECTION_QUERY};
use super::response::GraphQLResponse;
use super::subscription::{SubscriptionConfig, SubscriptionConnection, SubscriptionStream};
use crate::error::{NetworkError, Result};
use crate::http::{HttpClient, HttpClientBuilder};

/// Builder for creating a GraphQL client.
pub struct GraphQLClientBuilder {
    http_url: String,
    websocket_url: Option<String>,
    http_client: Option<HttpClient>,
    http_client_builder: Option<HttpClientBuilder>,
    default_headers: HashMap<String, String>,
    auth_token: Option<String>,
    connection_init_payload: Option<Value>,
    request_timeout: Option<Duration>,
    connection_timeout: Duration,
    keep_alive_interval: Option<Duration>,
}

impl GraphQLClientBuilder {
    /// Create a new builder with the specified GraphQL endpoint URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            http_url: url.into(),
            websocket_url: None,
            http_client: None,
            http_client_builder: None,
            default_headers: HashMap::new(),
            auth_token: None,
            connection_init_payload: None,
            request_timeout: None,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Some(Duration::from_secs(30)),
        }
    }

    /// Set a separate WebSocket URL for subscriptions.
    ///
    /// If not set, the HTTP URL will be converted to WebSocket protocol
    /// (http:// -> ws://, https:// -> wss://).
    pub fn websocket_url(mut self, url: impl Into<String>) -> Self {
        self.websocket_url = Some(url.into());
        self
    }

    /// Use an existing HTTP client.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Use a custom HTTP client builder.
    pub fn http_client_builder(mut self, builder: HttpClientBuilder) -> Self {
        self.http_client_builder = Some(builder);
        self
    }

    /// Add a default header to all requests.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    /// Add multiple headers.
    pub fn headers(mut self, headers: impl IntoIterator<Item = (String, String)>) -> Self {
        self.default_headers.extend(headers);
        self
    }

    /// Set bearer token authentication.
    ///
    /// This adds the Authorization header and includes the token
    /// in the WebSocket connection init payload.
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        let token = token.into();
        self.auth_token = Some(token.clone());
        self.default_headers
            .insert("Authorization".into(), format!("Bearer {}", token));
        self
    }

    /// Set the connection init payload for WebSocket subscriptions.
    ///
    /// This is sent when establishing the WebSocket connection and can
    /// include authentication tokens or other initialization data.
    pub fn connection_init_payload(mut self, payload: impl Serialize) -> Self {
        self.connection_init_payload = serde_json::to_value(payload).ok();
        self
    }

    /// Set the request timeout for HTTP operations.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Set the connection timeout for WebSocket connections.
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the keep-alive interval for WebSocket connections.
    ///
    /// Set to `None` to disable keep-alive pings.
    pub fn keep_alive_interval(mut self, interval: Option<Duration>) -> Self {
        self.keep_alive_interval = interval;
        self
    }

    /// Build the GraphQL client.
    pub fn build(self) -> Result<GraphQLClient> {
        // Build or get HTTP client
        let http_client = if let Some(client) = self.http_client {
            client
        } else if let Some(builder) = self.http_client_builder {
            builder.build()?
        } else {
            let mut builder = HttpClient::builder();
            if let Some(timeout) = self.request_timeout {
                builder = builder.timeout(timeout);
            }
            builder.build()?
        };

        // Derive WebSocket URL if not provided
        let websocket_url = self.websocket_url.unwrap_or_else(|| {
            Self::http_to_ws_url(&self.http_url)
        });

        // Build connection init payload
        let init_payload = if self.connection_init_payload.is_some() {
            self.connection_init_payload
        } else if let Some(ref token) = self.auth_token {
            // Default: include auth token in connection init
            Some(serde_json::json!({
                "Authorization": format!("Bearer {}", token)
            }))
        } else {
            None
        };

        Ok(GraphQLClient {
            inner: Arc::new(GraphQLClientInner {
                http_client,
                http_url: self.http_url,
                websocket_url,
                default_headers: self.default_headers,
                connection_timeout: self.connection_timeout,
                keep_alive_interval: self.keep_alive_interval,
                init_payload,
                subscription_connection: Mutex::new(None),
            }),
        })
    }

    fn http_to_ws_url(url: &str) -> String {
        if url.starts_with("https://") {
            format!("wss://{}", &url[8..])
        } else if url.starts_with("http://") {
            format!("ws://{}", &url[7..])
        } else {
            url.to_string()
        }
    }
}

struct GraphQLClientInner {
    http_client: HttpClient,
    http_url: String,
    websocket_url: String,
    default_headers: HashMap<String, String>,
    connection_timeout: Duration,
    keep_alive_interval: Option<Duration>,
    init_payload: Option<Value>,
    subscription_connection: Mutex<Option<SubscriptionConnection>>,
}

/// A GraphQL client for queries, mutations, and subscriptions.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::graphql::{GraphQLClient, GraphQLRequest};
///
/// let client = GraphQLClient::new("https://api.example.com/graphql")
///     .bearer_auth("my-token");
///
/// // Execute a query
/// let request = GraphQLRequest::query("{ users { id name } }");
/// let response = client.execute(request).await?;
///
/// // Subscribe to events
/// let subscription = GraphQLRequest::subscription("subscription { events { id } }");
/// let mut stream = client.subscribe(subscription).await?;
/// ```
#[derive(Clone)]
pub struct GraphQLClient {
    inner: Arc<GraphQLClientInner>,
}

impl GraphQLClient {
    /// Create a new GraphQL client with the specified endpoint URL.
    pub fn new(url: impl Into<String>) -> GraphQLClientBuilder {
        GraphQLClientBuilder::new(url)
    }

    /// Create a new builder for configuring a GraphQL client.
    pub fn builder(url: impl Into<String>) -> GraphQLClientBuilder {
        GraphQLClientBuilder::new(url)
    }

    /// Get the HTTP endpoint URL.
    pub fn url(&self) -> &str {
        &self.inner.http_url
    }

    /// Get the WebSocket URL for subscriptions.
    pub fn websocket_url(&self) -> &str {
        &self.inner.websocket_url
    }

    /// Execute a GraphQL operation (query or mutation).
    ///
    /// For subscriptions, use `subscribe()` instead.
    pub async fn execute(&self, request: GraphQLRequest) -> Result<GraphQLResponse> {
        if request.is_subscription() {
            return Err(NetworkError::Request(
                "Use subscribe() for subscription operations".into(),
            ));
        }

        // Build the HTTP request
        let mut req = self
            .inner
            .http_client
            .post(&self.inner.http_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        // Add default headers
        for (name, value) in &self.inner.default_headers {
            req = req.header(name.as_str(), value.as_str());
        }

        // Serialize the GraphQL request
        let body = serde_json::to_string(&request)
            .map_err(|e| NetworkError::Json(e.to_string()))?;

        req = req.text(body);

        // Execute the request
        let response = req.send().await?;

        // Check for HTTP errors
        if !response.is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetworkError::HttpStatus {
                status,
                message: Some(body),
            });
        }

        // Parse the GraphQL response
        let graphql_response: GraphQLResponse = response.json().await?;
        Ok(graphql_response)
    }

    /// Execute a query and parse the result.
    ///
    /// Convenience method that calls `execute()` and parses the data.
    pub async fn query<T: serde::de::DeserializeOwned>(
        &self,
        query: impl Into<String>,
    ) -> Result<T> {
        let request = GraphQLRequest::query(query);
        let response = self.execute(request).await?;
        response.data()
    }

    /// Execute a query with variables and parse the result.
    pub async fn query_with_variables<T: serde::de::DeserializeOwned>(
        &self,
        query: impl Into<String>,
        variables: impl Serialize,
    ) -> Result<T> {
        let request = GraphQLRequest::query(query).variables(variables);
        let response = self.execute(request).await?;
        response.data()
    }

    /// Execute a mutation and parse the result.
    pub async fn mutate<T: serde::de::DeserializeOwned>(
        &self,
        mutation: impl Into<String>,
    ) -> Result<T> {
        let request = GraphQLRequest::mutation(mutation);
        let response = self.execute(request).await?;
        response.data()
    }

    /// Execute a mutation with variables and parse the result.
    pub async fn mutate_with_variables<T: serde::de::DeserializeOwned>(
        &self,
        mutation: impl Into<String>,
        variables: impl Serialize,
    ) -> Result<T> {
        let request = GraphQLRequest::mutation(mutation).variables(variables);
        let response = self.execute(request).await?;
        response.data()
    }

    /// Subscribe to a GraphQL subscription.
    ///
    /// This establishes a WebSocket connection (if not already connected)
    /// and returns a stream of subscription messages.
    pub async fn subscribe(&self, request: GraphQLRequest) -> Result<SubscriptionStream> {
        if !request.is_subscription() {
            return Err(NetworkError::Request(
                "Expected a subscription operation".into(),
            ));
        }

        // Get or create subscription connection
        let connection = self.get_or_create_subscription_connection().await?;

        // Subscribe
        connection.subscribe(request).await
    }

    /// Fetch the schema using introspection.
    ///
    /// Returns the raw introspection result as JSON.
    pub async fn introspect(&self) -> Result<GraphQLResponse> {
        let request = GraphQLRequest::query(INTROSPECTION_QUERY);
        self.execute(request).await
    }

    /// Get or create the subscription connection.
    async fn get_or_create_subscription_connection(&self) -> Result<&SubscriptionConnection> {
        // Check if we already have a connection
        {
            let guard = self.inner.subscription_connection.lock();
            if guard.is_some() {
                drop(guard);
                // Return a reference - this is safe because the connection is stored in Arc
                let guard = self.inner.subscription_connection.lock();
                return Ok(unsafe {
                    // SAFETY: The connection is stored in the Arc and won't be dropped
                    // while we hold a reference to the client
                    &*(guard.as_ref().unwrap() as *const SubscriptionConnection)
                });
            }
        }

        // Create a new connection
        let config = SubscriptionConfig {
            url: self.inner.websocket_url.clone(),
            init_payload: self.inner.init_payload.clone(),
            connection_timeout: self.inner.connection_timeout,
            keep_alive_interval: self.inner.keep_alive_interval,
            headers: self.inner.default_headers.clone(),
        };

        let mut connection = SubscriptionConnection::new(config);
        connection.connect().await?;

        // Store the connection
        let mut guard = self.inner.subscription_connection.lock();
        *guard = Some(connection);

        // Return a reference
        Ok(unsafe {
            &*(guard.as_ref().unwrap() as *const SubscriptionConnection)
        })
    }
}

impl std::fmt::Debug for GraphQLClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphQLClient")
            .field("http_url", &self.inner.http_url)
            .field("websocket_url", &self.inner.websocket_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_to_ws_url() {
        assert_eq!(
            GraphQLClientBuilder::http_to_ws_url("https://example.com/graphql"),
            "wss://example.com/graphql"
        );
        assert_eq!(
            GraphQLClientBuilder::http_to_ws_url("http://example.com/graphql"),
            "ws://example.com/graphql"
        );
    }

    #[test]
    fn test_builder_defaults() {
        let client = GraphQLClient::new("https://api.example.com/graphql")
            .build()
            .unwrap();

        assert_eq!(client.url(), "https://api.example.com/graphql");
        assert_eq!(client.websocket_url(), "wss://api.example.com/graphql");
    }

    #[test]
    fn test_builder_custom_ws_url() {
        let client = GraphQLClient::new("https://api.example.com/graphql")
            .websocket_url("wss://ws.example.com/graphql")
            .build()
            .unwrap();

        assert_eq!(client.websocket_url(), "wss://ws.example.com/graphql");
    }

    #[test]
    fn test_builder_auth() {
        let client = GraphQLClient::new("https://api.example.com/graphql")
            .bearer_auth("my-token")
            .build()
            .unwrap();

        // Auth token is stored internally
        assert!(client.inner.init_payload.is_some());
    }
}
