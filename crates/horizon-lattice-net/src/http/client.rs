//! HTTP client implementation.

use std::sync::Arc;
use std::time::Duration;

use reqwest::redirect::Policy;

use super::request::{HttpMethod, HttpRequestBuilder};
use crate::error::{NetworkError, Result};
use crate::tls::{Certificate, Identity, TlsConfig, TlsVersion};

/// Configuration for the HTTP client.
#[derive(Clone, Debug)]
pub struct HttpClientConfig {
    /// Request timeout.
    pub timeout: Option<Duration>,
    /// Connect timeout.
    pub connect_timeout: Option<Duration>,
    /// Whether to follow redirects.
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow.
    pub max_redirects: usize,
    /// Whether to enable cookie storage.
    pub cookies_enabled: bool,
    /// Default user agent.
    pub user_agent: Option<String>,
    /// Proxy URL.
    pub proxy: Option<String>,
    /// TLS configuration.
    pub tls: TlsConfig,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            connect_timeout: Some(Duration::from_secs(10)),
            follow_redirects: true,
            max_redirects: 10,
            cookies_enabled: true,
            user_agent: Some(format!(
                "HorizonLattice/{} (Rust)",
                env!("CARGO_PKG_VERSION")
            )),
            proxy: None,
            tls: TlsConfig::default(),
        }
    }
}

/// Builder for creating an HTTP client with custom configuration.
pub struct HttpClientBuilder {
    config: HttpClientConfig,
    default_headers: http::HeaderMap,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClientBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
            default_headers: http::HeaderMap::new(),
        }
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Disable request timeout.
    pub fn no_timeout(mut self) -> Self {
        self.config.timeout = None;
        self
    }

    /// Set the connect timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = Some(timeout);
        self
    }

    /// Disable redirect following.
    pub fn no_redirects(mut self) -> Self {
        self.config.follow_redirects = false;
        self
    }

    /// Set the maximum number of redirects to follow.
    pub fn max_redirects(mut self, max: usize) -> Self {
        self.config.max_redirects = max;
        self
    }

    /// Disable cookie storage.
    pub fn no_cookies(mut self) -> Self {
        self.config.cookies_enabled = false;
        self
    }

    /// Set the user agent string.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = Some(user_agent.into());
        self
    }

    /// Set a proxy URL.
    pub fn proxy(mut self, proxy_url: impl Into<String>) -> Self {
        self.config.proxy = Some(proxy_url.into());
        self
    }

    /// Add a custom root certificate to trust.
    ///
    /// This can be used to connect to servers with self-signed or custom CA certificates.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice_net::{HttpClient, Certificate};
    ///
    /// let ca_cert = Certificate::from_pem_file("/path/to/ca.crt")?;
    /// let client = HttpClient::builder()
    ///     .add_root_certificate(ca_cert)
    ///     .build()?;
    /// ```
    pub fn add_root_certificate(mut self, cert: Certificate) -> Self {
        self.config.tls.root_certificates.push(cert);
        self
    }

    /// Use only the provided root certificates, ignoring system certificates.
    ///
    /// This must be combined with `add_root_certificate()` to provide at least one CA.
    pub fn tls_certs_only(mut self) -> Self {
        self.config.tls.use_only_custom_roots = true;
        self
    }

    /// Set the client identity for mutual TLS (mTLS) authentication.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice_net::{HttpClient, Identity};
    ///
    /// let identity = Identity::from_pem_files(
    ///     "/path/to/client.crt",
    ///     "/path/to/client.key",
    /// )?;
    /// let client = HttpClient::builder()
    ///     .identity(identity)
    ///     .build()?;
    /// ```
    pub fn identity(mut self, identity: Identity) -> Self {
        self.config.tls.identity = Some(identity);
        self
    }

    /// Set the minimum TLS version.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice_net::{HttpClient, TlsVersion};
    ///
    /// let client = HttpClient::builder()
    ///     .min_tls_version(TlsVersion::Tls1_3)
    ///     .build()?;
    /// ```
    pub fn min_tls_version(mut self, version: TlsVersion) -> Self {
        self.config.tls.min_version = version;
        self
    }

    /// Set the complete TLS configuration.
    ///
    /// This replaces any previously configured TLS settings.
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.config.tls = config;
        self
    }

    /// Accept invalid TLS certificates.
    ///
    /// # Warning
    ///
    /// This is insecure and should only be used for testing.
    pub fn danger_accept_invalid_certs(mut self) -> Self {
        self.config.tls.danger_accept_invalid_certs = true;
        self
    }

    /// Add a default header that will be sent with every request.
    pub fn default_header(
        mut self,
        name: impl TryInto<http::HeaderName>,
        value: impl TryInto<http::HeaderValue>,
    ) -> Result<Self> {
        let name = name
            .try_into()
            .map_err(|_| NetworkError::InvalidHeader("Invalid header name".to_string()))?;
        let value = value
            .try_into()
            .map_err(|_| NetworkError::InvalidHeader("Invalid header value".to_string()))?;
        self.default_headers.insert(name, value);
        Ok(self)
    }

    /// Build the HTTP client.
    pub fn build(self) -> Result<HttpClient> {
        let mut builder = reqwest::Client::builder();

        // Timeout configuration
        if let Some(timeout) = self.config.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(connect_timeout) = self.config.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }

        // Redirect policy
        if self.config.follow_redirects {
            builder = builder.redirect(Policy::limited(self.config.max_redirects));
        } else {
            builder = builder.redirect(Policy::none());
        }

        // Cookie storage
        if self.config.cookies_enabled {
            builder = builder.cookie_store(true);
        }

        // User agent
        if let Some(ref ua) = self.config.user_agent {
            builder = builder.user_agent(ua);
        }

        // Proxy
        if let Some(ref proxy_url) = self.config.proxy {
            let proxy =
                reqwest::Proxy::all(proxy_url).map_err(|e| NetworkError::Proxy(e.to_string()))?;
            builder = builder.proxy(proxy);
        }

        // TLS configuration
        let tls = &self.config.tls;

        // Add custom root certificates
        for cert in &tls.root_certificates {
            for reqwest_cert in cert.to_reqwest_certificates() {
                builder = builder.add_root_certificate(reqwest_cert);
            }
        }

        // Disable built-in root certificates if using only custom roots
        if tls.use_only_custom_roots {
            builder = builder.tls_built_in_root_certs(false);
        }

        // Set client identity for mTLS
        if let Some(ref identity) = tls.identity {
            let reqwest_identity = identity.to_reqwest_identity()?;
            builder = builder.identity(reqwest_identity);
        }

        // Set minimum TLS version
        builder = builder.min_tls_version(tls.min_version.to_reqwest_version());

        // Accept invalid certificates (dangerous - testing only)
        if tls.danger_accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // Default headers
        builder = builder.default_headers(self.default_headers.clone());

        let client = builder.build()?;

        Ok(HttpClient {
            inner: Arc::new(HttpClientInner {
                client,
                config: self.config,
                default_headers: self.default_headers,
            }),
        })
    }
}

/// Internal state for the HTTP client.
struct HttpClientInner {
    client: reqwest::Client,
    config: HttpClientConfig,
    #[allow(dead_code)] // Stored for potential future use
    default_headers: http::HeaderMap,
}

/// A high-level HTTP client for making requests.
///
/// The client is cheaply cloneable and thread-safe. Clones share the same
/// underlying connection pool and configuration.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::http::HttpClient;
///
/// let client = HttpClient::new();
///
/// // Simple GET request
/// let response = client.get("https://httpbin.org/get").send().await?;
/// println!("Status: {}", response.status());
///
/// // POST with JSON
/// let response = client
///     .post("https://httpbin.org/post")
///     .json(&serde_json::json!({"key": "value"}))
///     .send()
///     .await?;
/// ```
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<HttpClientInner>,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// Create a new HTTP client with default configuration.
    pub fn new() -> Self {
        HttpClientBuilder::new()
            .build()
            .expect("Failed to create HTTP client with default configuration")
    }

    /// Create a builder for configuring a new HTTP client.
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    /// Get the client's configuration.
    pub fn config(&self) -> &HttpClientConfig {
        &self.inner.config
    }

    /// Create a GET request builder.
    pub fn get(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Get, url.as_ref().to_string())
    }

    /// Create a POST request builder.
    pub fn post(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Post, url.as_ref().to_string())
    }

    /// Create a PUT request builder.
    pub fn put(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Put, url.as_ref().to_string())
    }

    /// Create a DELETE request builder.
    pub fn delete(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Delete, url.as_ref().to_string())
    }

    /// Create a PATCH request builder.
    pub fn patch(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Patch, url.as_ref().to_string())
    }

    /// Create a HEAD request builder.
    pub fn head(&self, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), HttpMethod::Head, url.as_ref().to_string())
    }

    /// Create a request builder with a custom method.
    pub fn request(&self, method: HttpMethod, url: impl AsRef<str>) -> HttpRequestBuilder {
        HttpRequestBuilder::new(self.clone(), method, url.as_ref().to_string())
    }

    /// Get a reference to the underlying reqwest client.
    pub(crate) fn reqwest_client(&self) -> &reqwest::Client {
        &self.inner.client
    }
}

impl std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("config", &self.inner.config)
            .finish()
    }
}

/// Authentication credentials for HTTP requests.
#[derive(Clone, Debug)]
pub enum Authentication {
    /// HTTP Basic authentication.
    Basic {
        /// Username.
        username: String,
        /// Password (optional).
        password: Option<String>,
    },
    /// Bearer token authentication.
    Bearer(String),
}
