//! gRPC channel management.

use std::time::Duration;

use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};

use crate::error::{NetworkError, Result};
use crate::tls::TlsConfig;

/// Builder for creating gRPC channels.
pub struct GrpcChannelBuilder {
    endpoint: String,
    tls_config: Option<TlsConfig>,
    connect_timeout: Option<Duration>,
    timeout: Option<Duration>,
    keep_alive_interval: Option<Duration>,
    keep_alive_timeout: Option<Duration>,
    keep_alive_while_idle: bool,
    http2_adaptive_window: bool,
    initial_stream_window_size: Option<u32>,
    initial_connection_window_size: Option<u32>,
    tcp_nodelay: bool,
    origin: Option<Uri>,
    user_agent: Option<String>,
}

impl GrpcChannelBuilder {
    /// Create a new builder with the specified endpoint.
    ///
    /// The endpoint should include the scheme (http:// or https://).
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tls_config: None,
            connect_timeout: None,
            timeout: None,
            keep_alive_interval: None,
            keep_alive_timeout: None,
            keep_alive_while_idle: false,
            http2_adaptive_window: false,
            initial_stream_window_size: None,
            initial_connection_window_size: None,
            tcp_nodelay: true,
            origin: None,
            user_agent: None,
        }
    }

    /// Configure TLS for secure connections.
    ///
    /// For https:// endpoints, TLS is automatically enabled.
    /// Use this method to provide custom certificates.
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    /// Set the connection timeout.
    ///
    /// This is the maximum time to wait for establishing a connection.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Set the request timeout.
    ///
    /// This is the maximum time to wait for a response.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the HTTP/2 keep-alive interval.
    ///
    /// Sends PING frames at this interval to keep the connection alive.
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(interval);
        self
    }

    /// Set the HTTP/2 keep-alive timeout.
    ///
    /// If no response to a keep-alive PING is received within this time,
    /// the connection is considered dead.
    pub fn keep_alive_timeout(mut self, timeout: Duration) -> Self {
        self.keep_alive_timeout = Some(timeout);
        self
    }

    /// Enable keep-alive while the connection is idle.
    pub fn keep_alive_while_idle(mut self, enable: bool) -> Self {
        self.keep_alive_while_idle = enable;
        self
    }

    /// Enable HTTP/2 adaptive window size.
    ///
    /// This automatically adjusts the window size based on bandwidth-delay product.
    pub fn http2_adaptive_window(mut self, enable: bool) -> Self {
        self.http2_adaptive_window = enable;
        self
    }

    /// Set the initial stream-level window size.
    pub fn initial_stream_window_size(mut self, size: u32) -> Self {
        self.initial_stream_window_size = Some(size);
        self
    }

    /// Set the initial connection-level window size.
    pub fn initial_connection_window_size(mut self, size: u32) -> Self {
        self.initial_connection_window_size = Some(size);
        self
    }

    /// Enable or disable TCP_NODELAY.
    ///
    /// When enabled (default), disables Nagle's algorithm for lower latency.
    pub fn tcp_nodelay(mut self, enable: bool) -> Self {
        self.tcp_nodelay = enable;
        self
    }

    /// Set the origin header for the connection.
    pub fn origin(mut self, origin: Uri) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set the user agent string.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Build and connect the channel.
    pub async fn connect(self) -> Result<GrpcChannel> {
        let uri: Uri = self
            .endpoint
            .parse()
            .map_err(|e| NetworkError::InvalidUrl(format!("Invalid gRPC endpoint: {}", e)))?;

        let mut endpoint = Endpoint::from(uri);

        // Configure TLS
        if let Some(tls_config) = self.tls_config {
            let client_tls_config = Self::build_tls_config(tls_config)?;
            endpoint = endpoint.tls_config(client_tls_config)
                .map_err(|e| NetworkError::Tls(e.to_string()))?;
        } else if self.endpoint.starts_with("https://") {
            // Auto-enable TLS for https endpoints
            endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| NetworkError::Tls(e.to_string()))?;
        }

        // Apply timeouts
        if let Some(timeout) = self.connect_timeout {
            endpoint = endpoint.connect_timeout(timeout);
        }
        if let Some(timeout) = self.timeout {
            endpoint = endpoint.timeout(timeout);
        }

        // Apply keep-alive settings
        if let Some(interval) = self.keep_alive_interval {
            endpoint = endpoint.keep_alive_timeout(interval);
        }
        if let Some(timeout) = self.keep_alive_timeout {
            endpoint = endpoint.keep_alive_timeout(timeout);
        }
        if self.keep_alive_while_idle {
            endpoint = endpoint.keep_alive_while_idle(true);
        }

        // Apply HTTP/2 settings
        if self.http2_adaptive_window {
            endpoint = endpoint.http2_adaptive_window(true);
        }
        if let Some(size) = self.initial_stream_window_size {
            endpoint = endpoint.initial_stream_window_size(size);
        }
        if let Some(size) = self.initial_connection_window_size {
            endpoint = endpoint.initial_connection_window_size(size);
        }

        // Apply TCP settings
        endpoint = endpoint.tcp_nodelay(self.tcp_nodelay);

        // Apply origin
        if let Some(origin) = self.origin {
            endpoint = endpoint.origin(origin);
        }

        // Apply user agent
        if let Some(user_agent) = self.user_agent {
            endpoint = endpoint.user_agent(user_agent)
                .map_err(|e| NetworkError::InvalidHeader(e.to_string()))?;
        }

        // Connect
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| NetworkError::Connection(format!("gRPC connection failed: {}", e)))?;

        Ok(GrpcChannel {
            inner: channel,
            endpoint: self.endpoint,
        })
    }

    /// Build a lazy channel that connects on first use.
    pub fn connect_lazy(self) -> Result<GrpcChannel> {
        let uri: Uri = self
            .endpoint
            .parse()
            .map_err(|e| NetworkError::InvalidUrl(format!("Invalid gRPC endpoint: {}", e)))?;

        let mut endpoint = Endpoint::from(uri);

        // Configure TLS
        if let Some(tls_config) = self.tls_config {
            let client_tls_config = Self::build_tls_config(tls_config)?;
            endpoint = endpoint.tls_config(client_tls_config)
                .map_err(|e| NetworkError::Tls(e.to_string()))?;
        } else if self.endpoint.starts_with("https://") {
            endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| NetworkError::Tls(e.to_string()))?;
        }

        // Apply other settings (same as connect)
        if let Some(timeout) = self.connect_timeout {
            endpoint = endpoint.connect_timeout(timeout);
        }
        if let Some(timeout) = self.timeout {
            endpoint = endpoint.timeout(timeout);
        }
        endpoint = endpoint.tcp_nodelay(self.tcp_nodelay);

        // Create lazy channel
        let channel = endpoint.connect_lazy();

        Ok(GrpcChannel {
            inner: channel,
            endpoint: self.endpoint,
        })
    }

    fn build_tls_config(config: TlsConfig) -> Result<ClientTlsConfig> {
        let mut client_config = ClientTlsConfig::new().with_native_roots();

        // Add custom CA certificates
        for cert in &config.root_certificates {
            for der_cert in cert.der_certs() {
                // Convert DER to PEM format for tonic
                let pem = Self::der_to_pem(der_cert.as_ref(), "CERTIFICATE");
                client_config = client_config.ca_certificate(tonic::transport::Certificate::from_pem(pem));
            }
        }

        // Add client identity if present (for mTLS)
        if let Some(ref identity) = config.identity {
            // Convert identity to PEM format
            let cert_pem = Self::certs_to_pem(identity.cert_chain());
            let key_pem = Self::private_key_to_pem(identity.private_key())?;
            client_config = client_config.identity(
                tonic::transport::Identity::from_pem(cert_pem, key_pem)
            );
        }

        Ok(client_config)
    }

    fn der_to_pem(der: &[u8], label: &str) -> String {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(der);
        format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----\n",
            label,
            b64.chars()
                .collect::<Vec<_>>()
                .chunks(64)
                .map(|c| c.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n"),
            label
        )
    }

    fn certs_to_pem(certs: &[rustls::pki_types::CertificateDer<'static>]) -> String {
        certs
            .iter()
            .map(|c| Self::der_to_pem(c.as_ref(), "CERTIFICATE"))
            .collect()
    }

    fn private_key_to_pem(key: &rustls::pki_types::PrivateKeyDer<'static>) -> Result<String> {
        use rustls::pki_types::PrivateKeyDer;
        let (label, der) = match key {
            PrivateKeyDer::Pkcs1(k) => ("RSA PRIVATE KEY", k.secret_pkcs1_der()),
            PrivateKeyDer::Pkcs8(k) => ("PRIVATE KEY", k.secret_pkcs8_der()),
            PrivateKeyDer::Sec1(k) => ("EC PRIVATE KEY", k.secret_sec1_der()),
            _ => return Err(NetworkError::Tls("Unknown private key format".into())),
        };
        Ok(Self::der_to_pem(der, label))
    }
}

/// A gRPC channel for making RPC calls.
///
/// This wraps a tonic `Channel` and provides integration with the
/// Horizon Lattice networking module.
#[derive(Clone)]
pub struct GrpcChannel {
    inner: Channel,
    endpoint: String,
}

impl GrpcChannel {
    /// Create a new builder for configuring a gRPC channel.
    pub fn builder(endpoint: impl Into<String>) -> GrpcChannelBuilder {
        GrpcChannelBuilder::new(endpoint)
    }

    /// Get the endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the underlying tonic channel.
    ///
    /// Use this to create gRPC clients from generated code.
    pub fn into_inner(self) -> Channel {
        self.inner
    }

    /// Get a reference to the underlying tonic channel.
    pub fn inner(&self) -> &Channel {
        &self.inner
    }
}

impl From<GrpcChannel> for Channel {
    fn from(channel: GrpcChannel) -> Self {
        channel.inner
    }
}

impl std::fmt::Debug for GrpcChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcChannel")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = GrpcChannelBuilder::new("http://localhost:50051");
        assert_eq!(builder.endpoint, "http://localhost:50051");
    }

    #[tokio::test]
    async fn test_builder_lazy_connect() {
        let channel = GrpcChannel::builder("http://localhost:50051")
            .connect_lazy()
            .unwrap();
        assert_eq!(channel.endpoint(), "http://localhost:50051");
    }

    #[test]
    fn test_builder_with_settings() {
        let _builder = GrpcChannel::builder("http://localhost:50051")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .keep_alive_interval(Duration::from_secs(60))
            .tcp_nodelay(true)
            .user_agent("MyApp/1.0");
    }
}
