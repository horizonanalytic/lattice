//! REST API client helpers.
//!
//! This module provides convenience features for consuming REST APIs,
//! including a client builder with base URL, authentication, rate limiting,
//! retry logic, and request/response interceptors.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::http::{RestApiClient, ApiAuth};
//!
//! // Create a REST API client
//! let client = RestApiClient::builder("https://api.example.com")
//!     .bearer_auth("my-token")
//!     .rate_limit_per_second(10)
//!     .build()?;
//!
//! // Make requests - paths are appended to base URL
//! let response = client.get("/users").send().await?;
//!
//! // Parse JSON response directly
//! let users: Vec<User> = client.get("/users").json_response().await?;
//!
//! // POST with JSON body
//! let user: User = client.post("/users")
//!     .json(&CreateUser { name: "John" })
//!     .json_response()
//!     .await?;
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use parking_lot::Mutex;
use serde::{Serialize, de::DeserializeOwned};

use super::client::{HttpClient, HttpClientBuilder};
use super::download::RetryConfig;
use super::request::{HttpMethod, HttpRequest, RequestBody};
use super::response::HttpResponse;
use crate::error::{NetworkError, Result};

/// Authentication method for REST APIs.
#[derive(Clone, Debug)]
pub enum ApiAuth {
    /// Bearer token authentication (Authorization: Bearer <token>).
    Bearer(String),
    /// API key in a custom header.
    ApiKey {
        /// Header name (e.g., "X-API-Key").
        header: String,
        /// API key value.
        value: String,
    },
    /// HTTP Basic authentication.
    Basic {
        /// Username.
        username: String,
        /// Password (optional).
        password: Option<String>,
    },
}

/// Rate limit information parsed from response headers.
///
/// Standard headers supported:
/// - `X-RateLimit-Limit` or `RateLimit-Limit`: Maximum requests allowed
/// - `X-RateLimit-Remaining` or `RateLimit-Remaining`: Requests remaining in window
/// - `X-RateLimit-Reset` or `RateLimit-Reset`: Unix timestamp when limit resets
/// - `Retry-After`: Seconds to wait before retrying (on 429 responses)
#[derive(Clone, Debug, Default)]
pub struct RateLimitInfo {
    /// Maximum number of requests allowed in the current window.
    pub limit: Option<u64>,
    /// Number of requests remaining in the current window.
    pub remaining: Option<u64>,
    /// Unix timestamp when the rate limit window resets.
    pub reset_timestamp: Option<u64>,
    /// Duration to wait before retrying (from Retry-After header).
    pub retry_after: Option<Duration>,
}

impl RateLimitInfo {
    /// Parse rate limit information from response headers.
    pub fn from_response(response: &HttpResponse) -> Self {
        let headers = response.headers();

        // Parse limit
        let limit = Self::parse_header_u64(headers, "X-RateLimit-Limit")
            .or_else(|| Self::parse_header_u64(headers, "RateLimit-Limit"));

        // Parse remaining
        let remaining = Self::parse_header_u64(headers, "X-RateLimit-Remaining")
            .or_else(|| Self::parse_header_u64(headers, "RateLimit-Remaining"));

        // Parse reset timestamp
        let reset_timestamp = Self::parse_header_u64(headers, "X-RateLimit-Reset")
            .or_else(|| Self::parse_header_u64(headers, "RateLimit-Reset"));

        // Parse Retry-After (can be seconds or HTTP date, we only handle seconds)
        let retry_after = Self::parse_header_u64(headers, "Retry-After").map(Duration::from_secs);

        Self {
            limit,
            remaining,
            reset_timestamp,
            retry_after,
        }
    }

    /// Check if rate limited (remaining is 0 or response was 429).
    pub fn is_rate_limited(&self) -> bool {
        self.remaining == Some(0) || self.retry_after.is_some()
    }

    /// Get the duration to wait before the next request is allowed.
    ///
    /// Returns `None` if not rate limited or if reset time is in the past.
    pub fn wait_duration(&self) -> Option<Duration> {
        // If we have a Retry-After, use that directly
        if let Some(retry_after) = self.retry_after {
            return Some(retry_after);
        }

        // Otherwise, calculate from reset timestamp
        if let Some(reset) = self.reset_timestamp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if reset > now {
                return Some(Duration::from_secs(reset - now));
            }
        }

        None
    }

    fn parse_header_u64(headers: &http::HeaderMap, name: &str) -> Option<u64> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    }
}

/// Token bucket rate limiter.
///
/// Limits the rate of requests using a token bucket algorithm.
/// Each request consumes one token, and tokens are refilled at a fixed rate.
#[derive(Debug)]
pub struct RateLimiter {
    /// Current number of available tokens.
    tokens: AtomicU32,
    /// Maximum number of tokens (bucket size).
    max_tokens: u32,
    /// Duration between token refills.
    refill_interval: Duration,
    /// Timestamp of last refill check.
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified requests per second.
    ///
    /// The bucket size is set to allow bursting up to the per-second limit.
    pub fn new(requests_per_second: u32) -> Self {
        Self::with_burst(requests_per_second, requests_per_second)
    }

    /// Create a new rate limiter with custom burst size.
    ///
    /// # Arguments
    ///
    /// * `requests_per_second` - Sustained rate of requests
    /// * `burst_size` - Maximum burst size (bucket capacity)
    pub fn with_burst(requests_per_second: u32, burst_size: u32) -> Self {
        let refill_interval = if requests_per_second > 0 {
            Duration::from_secs_f64(1.0 / requests_per_second as f64)
        } else {
            Duration::from_secs(1)
        };

        Self {
            tokens: AtomicU32::new(burst_size),
            max_tokens: burst_size,
            refill_interval,
            last_refill: Mutex::new(Instant::now()),
        }
    }

    /// Try to acquire a token for a request.
    ///
    /// Returns `true` if a token was acquired, `false` if rate limited.
    pub fn try_acquire(&self) -> bool {
        self.refill();

        loop {
            let current = self.tokens.load(Ordering::Acquire);
            if current == 0 {
                return false;
            }

            if self
                .tokens
                .compare_exchange_weak(current, current - 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Acquire a token, waiting if necessary.
    ///
    /// This will block the current task until a token is available.
    pub async fn acquire(&self) {
        while !self.try_acquire() {
            tokio::time::sleep(self.refill_interval).await;
        }
    }

    /// Get the estimated wait time until a token is available.
    ///
    /// Returns `None` if a token is immediately available.
    pub fn wait_time(&self) -> Option<Duration> {
        self.refill();

        if self.tokens.load(Ordering::Acquire) > 0 {
            return None;
        }

        Some(self.refill_interval)
    }

    /// Refill tokens based on elapsed time.
    fn refill(&self) {
        let mut last_refill = self.last_refill.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        // Calculate how many tokens to add
        let tokens_to_add = (elapsed.as_secs_f64() / self.refill_interval.as_secs_f64()) as u32;

        if tokens_to_add > 0 {
            let current = self.tokens.load(Ordering::Acquire);
            let new_tokens = (current + tokens_to_add).min(self.max_tokens);
            self.tokens.store(new_tokens, Ordering::Release);
            *last_refill = now;
        }
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            tokens: AtomicU32::new(self.tokens.load(Ordering::Acquire)),
            max_tokens: self.max_tokens,
            refill_interval: self.refill_interval,
            last_refill: Mutex::new(*self.last_refill.lock()),
        }
    }
}

/// Type alias for request interceptors.
///
/// Request interceptors are called before each request is sent and can
/// modify the request (add headers, transform the body, etc.).
pub type RequestInterceptor = Arc<dyn Fn(&mut HttpRequest) + Send + Sync>;

/// Type alias for response interceptors.
///
/// Response interceptors are called after each successful response and can
/// inspect or validate the response. Returning an error will cause the
/// request to fail.
pub type ResponseInterceptor = Arc<dyn Fn(&HttpResponse) -> Result<()> + Send + Sync>;

/// Type alias for error transformers.
///
/// Error transformers are called when a request fails and can convert
/// the error to a different type or add context.
pub type ErrorTransformer = Arc<dyn Fn(NetworkError) -> NetworkError + Send + Sync>;

/// Internal state for interceptors.
#[derive(Clone, Default)]
struct Interceptors {
    request: Vec<RequestInterceptor>,
    response: Vec<ResponseInterceptor>,
    error: Option<ErrorTransformer>,
}

/// Builder for creating a REST API client.
pub struct RestApiClientBuilder {
    base_url: String,
    http_client: Option<HttpClient>,
    http_client_builder: Option<HttpClientBuilder>,
    default_headers: http::HeaderMap,
    auth: Option<ApiAuth>,
    rate_limiter: Option<RateLimiter>,
    retry_config: RetryConfig,
    interceptors: Interceptors,
}

impl RestApiClientBuilder {
    /// Create a new builder with the specified base URL.
    ///
    /// All request paths will be appended to this base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: None,
            http_client_builder: None,
            default_headers: http::HeaderMap::new(),
            auth: None,
            rate_limiter: None,
            retry_config: RetryConfig::default(),
            interceptors: Interceptors::default(),
        }
    }

    /// Use an existing HTTP client instead of creating a new one.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Use a custom HTTP client builder for advanced configuration.
    pub fn http_client_builder(mut self, builder: HttpClientBuilder) -> Self {
        self.http_client_builder = Some(builder);
        self
    }

    /// Add a default header that will be sent with every request.
    pub fn default_header(
        mut self,
        name: impl TryInto<http::HeaderName>,
        value: impl TryInto<http::HeaderValue>,
    ) -> Self {
        if let (Ok(name), Ok(value)) = (name.try_into(), value.try_into()) {
            self.default_headers.insert(name, value);
        }
        self
    }

    /// Set Accept header to application/json.
    pub fn accept_json(self) -> Self {
        self.default_header("Accept", "application/json")
    }

    /// Set Content-Type header to application/json.
    pub fn content_type_json(self) -> Self {
        self.default_header("Content-Type", "application/json")
    }

    /// Set both Accept and Content-Type to application/json.
    ///
    /// This is a convenience method equivalent to calling both
    /// `accept_json()` and `content_type_json()`.
    pub fn json_api(self) -> Self {
        self.accept_json().content_type_json()
    }

    /// Set bearer token authentication.
    ///
    /// Adds `Authorization: Bearer <token>` header to all requests.
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(ApiAuth::Bearer(token.into()));
        self
    }

    /// Set API key authentication.
    ///
    /// Adds the specified header with the API key to all requests.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // X-API-Key: my-secret-key
    /// client.api_key("X-API-Key", "my-secret-key")
    /// ```
    pub fn api_key(mut self, header: impl Into<String>, key: impl Into<String>) -> Self {
        self.auth = Some(ApiAuth::ApiKey {
            header: header.into(),
            value: key.into(),
        });
        self
    }

    /// Set HTTP Basic authentication.
    pub fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: Option<impl Into<String>>,
    ) -> Self {
        self.auth = Some(ApiAuth::Basic {
            username: username.into(),
            password: password.map(Into::into),
        });
        self
    }

    /// Set the authentication method.
    pub fn auth(mut self, auth: ApiAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Enable rate limiting with the specified requests per second.
    pub fn rate_limit_per_second(mut self, requests_per_second: u32) -> Self {
        self.rate_limiter = Some(RateLimiter::new(requests_per_second));
        self
    }

    /// Enable rate limiting with custom burst size.
    ///
    /// # Arguments
    ///
    /// * `requests_per_second` - Sustained rate of requests
    /// * `burst_size` - Maximum burst size allowed
    pub fn rate_limit_with_burst(mut self, requests_per_second: u32, burst_size: u32) -> Self {
        self.rate_limiter = Some(RateLimiter::with_burst(requests_per_second, burst_size));
        self
    }

    /// Set a custom rate limiter.
    pub fn rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Configure retry behavior.
    pub fn retry(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Disable automatic retries.
    pub fn no_retry(mut self) -> Self {
        self.retry_config.max_retries = 0;
        self
    }

    /// Set maximum number of retry attempts.
    pub fn max_retries(mut self, max: u32) -> Self {
        self.retry_config.max_retries = max;
        self
    }

    /// Add a request interceptor.
    ///
    /// Request interceptors are called in order before each request is sent.
    /// They can modify the request by adding headers, transforming the body, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.add_request_interceptor(|request| {
    ///     request.headers.insert("X-Request-ID", Uuid::new_v4().to_string());
    /// })
    /// ```
    pub fn add_request_interceptor<F>(mut self, interceptor: F) -> Self
    where
        F: Fn(&mut HttpRequest) + Send + Sync + 'static,
    {
        self.interceptors.request.push(Arc::new(interceptor));
        self
    }

    /// Add a response interceptor.
    ///
    /// Response interceptors are called in order after each successful response.
    /// They can inspect the response and return an error to fail the request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.add_response_interceptor(|response| {
    ///     if response.status() == 401 {
    ///         return Err(NetworkError::Authentication("Token expired".to_string()));
    ///     }
    ///     Ok(())
    /// })
    /// ```
    pub fn add_response_interceptor<F>(mut self, interceptor: F) -> Self
    where
        F: Fn(&HttpResponse) -> Result<()> + Send + Sync + 'static,
    {
        self.interceptors.response.push(Arc::new(interceptor));
        self
    }

    /// Set an error transformer.
    ///
    /// The error transformer is called when a request fails and can convert
    /// the error to a different type or add context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.error_transformer(|error| {
    ///     // Add context to all errors
    ///     NetworkError::Request(format!("API error: {}", error))
    /// })
    /// ```
    pub fn error_transformer<F>(mut self, transformer: F) -> Self
    where
        F: Fn(NetworkError) -> NetworkError + Send + Sync + 'static,
    {
        self.interceptors.error = Some(Arc::new(transformer));
        self
    }

    /// Build the REST API client.
    pub fn build(self) -> Result<RestApiClient> {
        // Get or create the HTTP client
        let http_client = if let Some(client) = self.http_client {
            client
        } else if let Some(builder) = self.http_client_builder {
            builder.build()?
        } else {
            HttpClient::new()
        };

        // Normalize base URL (remove trailing slash)
        let base_url = self.base_url.trim_end_matches('/').to_string();

        Ok(RestApiClient {
            inner: Arc::new(RestApiClientInner {
                http_client,
                base_url,
                default_headers: self.default_headers,
                auth: self.auth,
                rate_limiter: self.rate_limiter,
                retry_config: self.retry_config,
                interceptors: self.interceptors,
            }),
        })
    }
}

/// Internal state for the REST API client.
struct RestApiClientInner {
    http_client: HttpClient,
    base_url: String,
    default_headers: http::HeaderMap,
    auth: Option<ApiAuth>,
    rate_limiter: Option<RateLimiter>,
    retry_config: RetryConfig,
    interceptors: Interceptors,
}

/// A REST API client with convenience features.
///
/// Provides a high-level interface for consuming REST APIs with:
/// - Base URL configuration
/// - Default headers (JSON by default)
/// - Authentication (Bearer, API Key, Basic)
/// - Rate limiting
/// - Automatic retry with exponential backoff
/// - Request/response interceptors
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::http::RestApiClient;
///
/// let client = RestApiClient::builder("https://api.example.com")
///     .bearer_auth("my-token")
///     .json_api()  // Set Accept and Content-Type to application/json
///     .rate_limit_per_second(10)
///     .build()?;
///
/// // GET /users
/// let users: Vec<User> = client.get("/users").json_response().await?;
///
/// // POST /users with JSON body
/// let user: User = client.post("/users")
///     .json(&CreateUser { name: "John" })
///     .json_response()
///     .await?;
/// ```
#[derive(Clone)]
pub struct RestApiClient {
    inner: Arc<RestApiClientInner>,
}

impl RestApiClient {
    /// Create a new builder for configuring a REST API client.
    pub fn builder(base_url: impl Into<String>) -> RestApiClientBuilder {
        RestApiClientBuilder::new(base_url)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Get a reference to the underlying HTTP client.
    pub fn http_client(&self) -> &HttpClient {
        &self.inner.http_client
    }

    /// Create a GET request builder.
    pub fn get(&self, path: &str) -> RestApiRequestBuilder {
        self.request(HttpMethod::Get, path)
    }

    /// Create a POST request builder.
    pub fn post(&self, path: &str) -> RestApiRequestBuilder {
        self.request(HttpMethod::Post, path)
    }

    /// Create a PUT request builder.
    pub fn put(&self, path: &str) -> RestApiRequestBuilder {
        self.request(HttpMethod::Put, path)
    }

    /// Create a DELETE request builder.
    pub fn delete(&self, path: &str) -> RestApiRequestBuilder {
        self.request(HttpMethod::Delete, path)
    }

    /// Create a PATCH request builder.
    pub fn patch(&self, path: &str) -> RestApiRequestBuilder {
        self.request(HttpMethod::Patch, path)
    }

    /// Create a request builder with a custom method.
    pub fn request(&self, method: HttpMethod, path: &str) -> RestApiRequestBuilder {
        // Construct full URL
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        let url = format!("{}{}", self.inner.base_url, path);

        RestApiRequestBuilder {
            client: self.clone(),
            method,
            url,
            headers: self.inner.default_headers.clone(),
            query: Vec::new(),
            body: RequestBody::None,
            timeout: None,
            auth_override: None,
        }
    }
}

impl std::fmt::Debug for RestApiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestApiClient")
            .field("base_url", &self.inner.base_url)
            .field("has_auth", &self.inner.auth.is_some())
            .field("has_rate_limiter", &self.inner.rate_limiter.is_some())
            .finish()
    }
}

/// Builder for REST API requests.
pub struct RestApiRequestBuilder {
    client: RestApiClient,
    method: HttpMethod,
    url: String,
    headers: http::HeaderMap,
    query: Vec<(String, String)>,
    body: RequestBody,
    timeout: Option<Duration>,
    auth_override: Option<ApiAuth>,
}

impl RestApiRequestBuilder {
    /// Add a header to the request.
    pub fn header(
        mut self,
        name: impl TryInto<http::HeaderName>,
        value: impl TryInto<http::HeaderValue>,
    ) -> Self {
        if let (Ok(name), Ok(value)) = (name.try_into(), value.try_into()) {
            self.headers.insert(name, value);
        }
        self
    }

    /// Add multiple headers to the request.
    pub fn headers(mut self, headers: http::HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add a query parameter.
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Add multiple query parameters.
    pub fn query_pairs(mut self, pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        self.query.extend(pairs);
        self
    }

    /// Set a JSON body from a serializable value.
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        match serde_json::to_value(body) {
            Ok(value) => self.body = RequestBody::Json(value),
            Err(e) => {
                tracing::error!(target: "horizon_lattice_net::rest_api", "Failed to serialize JSON body: {}", e);
            }
        }
        self
    }

    /// Set a plain text body.
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.body = RequestBody::Text(body.into());
        self
    }

    /// Set a raw binary body.
    pub fn bytes(mut self, body: impl Into<Bytes>) -> Self {
        self.body = RequestBody::Bytes(body.into());
        self
    }

    /// Override bearer token authentication for this request.
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_override = Some(ApiAuth::Bearer(token.into()));
        self
    }

    /// Override API key authentication for this request.
    pub fn api_key(mut self, header: impl Into<String>, key: impl Into<String>) -> Self {
        self.auth_override = Some(ApiAuth::ApiKey {
            header: header.into(),
            value: key.into(),
        });
        self
    }

    /// Set a timeout for this specific request.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the request without sending it.
    pub fn build(self) -> HttpRequest {
        let auth = self
            .auth_override
            .as_ref()
            .or(self.client.inner.auth.as_ref());

        let auth = auth.map(|a| match a {
            ApiAuth::Bearer(token) => super::client::Authentication::Bearer(token.clone()),
            ApiAuth::Basic { username, password } => super::client::Authentication::Basic {
                username: username.clone(),
                password: password.clone(),
            },
            ApiAuth::ApiKey { .. } => {
                // API key auth is handled via headers, not the auth field
                super::client::Authentication::Bearer(String::new())
            }
        });

        let mut headers = self.headers;

        // Add API key header if using API key auth
        if let Some(ApiAuth::ApiKey { header, value }) =
            self.auth_override
                .as_ref()
                .or(self.client.inner.auth.as_ref())
            && let (Ok(name), Ok(val)) = (
                http::HeaderName::try_from(header.as_str()),
                http::HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(name, val);
            }

        HttpRequest {
            method: self.method,
            url: self.url,
            headers,
            query: self.query,
            body: self.body,
            timeout: self.timeout,
            auth: match auth {
                Some(super::client::Authentication::Bearer(ref t)) if t.is_empty() => None,
                other => other,
            },
        }
    }

    /// Send the request and return the response.
    pub async fn send(self) -> Result<HttpResponse> {
        let client = self.client.clone();
        let inner = &client.inner;

        // Build the request
        let mut request = self.build();

        // Apply request interceptors
        for interceptor in &inner.interceptors.request {
            interceptor(&mut request);
        }

        // Apply rate limiting
        if let Some(ref rate_limiter) = inner.rate_limiter {
            rate_limiter.acquire().await;
        }

        // Execute with retry logic
        let result =
            Self::execute_with_retry(&inner.http_client, request, &inner.retry_config).await;

        // Transform error if needed
        

        match result {
            Ok(response) => {
                // Apply response interceptors
                for interceptor in &inner.interceptors.response {
                    interceptor(&response)?;
                }
                Ok(response)
            }
            Err(e) => {
                if let Some(ref transformer) = inner.interceptors.error {
                    Err(transformer(e))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Send the request and parse the response as JSON.
    ///
    /// This is a convenience method equivalent to:
    /// ```ignore
    /// let response = request.send().await?;
    /// let data: T = response.json().await?;
    /// ```
    pub async fn json_response<T: DeserializeOwned>(self) -> Result<T> {
        let response = self.send().await?;
        response.json().await
    }

    /// Execute request with retry logic.
    async fn execute_with_retry(
        http_client: &HttpClient,
        request: HttpRequest,
        retry_config: &RetryConfig,
    ) -> Result<HttpResponse> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(retry_config.initial_delay_ms);

        loop {
            // Clone the request for this attempt
            let request_clone = HttpRequest {
                method: request.method,
                url: request.url.clone(),
                headers: request.headers.clone(),
                query: request.query.clone(),
                body: request.body.clone(),
                timeout: request.timeout,
                auth: request.auth.clone(),
            };

            let result = Self::execute_request(http_client, request_clone).await;

            match result {
                Ok(response) => {
                    // Check for rate limit response (429)
                    if response.status() == 429 {
                        let rate_info = RateLimitInfo::from_response(&response);

                        if attempts < retry_config.max_retries {
                            // Wait based on Retry-After or backoff
                            let wait = rate_info.retry_after.unwrap_or(delay);
                            tokio::time::sleep(wait).await;

                            attempts += 1;
                            delay = Self::next_delay(delay, retry_config);
                            continue;
                        }
                    }

                    // Check for server errors (5xx) that might be transient
                    if response.is_server_error() && attempts < retry_config.max_retries {
                        tokio::time::sleep(delay).await;
                        attempts += 1;
                        delay = Self::next_delay(delay, retry_config);
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    // Retry on connection errors
                    let is_retryable =
                        matches!(e, NetworkError::Connection(_) | NetworkError::Timeout);

                    if is_retryable && attempts < retry_config.max_retries {
                        tokio::time::sleep(delay).await;
                        attempts += 1;
                        delay = Self::next_delay(delay, retry_config);
                        continue;
                    }

                    return Err(e);
                }
            }
        }
    }

    /// Execute a single request.
    async fn execute_request(
        http_client: &HttpClient,
        request: HttpRequest,
    ) -> Result<HttpResponse> {
        // Build the URL with query parameters
        let mut url = url::Url::parse(&request.url)?;
        for (key, value) in &request.query {
            url.query_pairs_mut().append_pair(key, value);
        }

        // Build the reqwest request
        let mut req_builder = http_client
            .reqwest_client()
            .request(request.method.to_reqwest(), url);

        // Add headers
        for (name, value) in request.headers.iter() {
            req_builder = req_builder.header(name, value);
        }

        // Add authentication
        if let Some(auth) = &request.auth {
            match auth {
                super::client::Authentication::Basic { username, password } => {
                    req_builder = req_builder.basic_auth(username, password.as_ref());
                }
                super::client::Authentication::Bearer(token) => {
                    req_builder = req_builder.bearer_auth(token);
                }
            }
        }

        // Add timeout
        if let Some(timeout) = request.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        // Add body
        match request.body {
            RequestBody::None => {}
            RequestBody::Text(text) => {
                req_builder = req_builder.body(text);
            }
            RequestBody::Json(value) => {
                req_builder = req_builder.json(&value);
            }
            RequestBody::Form(data) => {
                req_builder = req_builder.form(&data);
            }
            RequestBody::Bytes(bytes) => {
                req_builder = req_builder.body(bytes);
            }
        }

        // Send the request
        let response = req_builder.send().await?;
        Ok(HttpResponse::from_reqwest(response))
    }

    /// Calculate the next delay with exponential backoff.
    fn next_delay(current: Duration, config: &RetryConfig) -> Duration {
        let next = current.mul_f64(config.backoff_multiplier);
        let max = Duration::from_millis(config.max_delay_ms);
        next.min(max)
    }
}
