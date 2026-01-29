//! HTTP client for Horizon Lattice.
//!
//! This module provides a high-level HTTP client with async support and
//! signal-based completion notification.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::http::{HttpClient, HttpMethod};
//!
//! // Create a client with default settings
//! let client = HttpClient::new();
//!
//! // Make a GET request
//! let response = client.get("https://api.example.com/users").send().await?;
//! println!("Status: {}", response.status());
//! println!("Body: {}", response.text().await?);
//!
//! // Make a POST request with JSON
//! let response = client
//!     .post("https://api.example.com/users")
//!     .json(&serde_json::json!({"name": "John"}))
//!     .send()
//!     .await?;
//! ```
//!
//! # Async with Signals
//!
//! For GUI integration, use `AsyncHttpClient` which emits signals on completion:
//!
//! ```ignore
//! use horizon_lattice_net::http::{AsyncHttpClient, RequestResult};
//!
//! let client = AsyncHttpClient::new();
//!
//! // Connect to completion signal
//! client.request_finished.connect(|result| {
//!     if let RequestResult::Success(response) = result {
//!         println!("Got response: {}", response.status());
//!     }
//! });
//!
//! // Start async request
//! let handle = client.get_async("https://api.example.com/data");
//! ```

mod async_client;
mod client;
mod download;
mod request;
mod response;
mod rest_api;
mod upload;

pub use async_client::{AsyncHttpClient, RequestHandle, RequestId, RequestStatus, runtime};
pub use client::{Authentication, HttpClient, HttpClientBuilder, HttpClientConfig};
pub use download::{DownloadEvent, DownloadId, DownloadManager, DownloadState, RetryConfig};
pub use request::{HttpMethod, HttpRequest, HttpRequestBuilder, MultipartForm, RequestBody};
pub use response::{HttpResponse, ResponseBody, TransferProgress};
pub use rest_api::{
    ApiAuth, ErrorTransformer, RateLimitInfo, RateLimiter, RequestInterceptor, ResponseInterceptor,
    RestApiClient, RestApiClientBuilder, RestApiRequestBuilder,
};
pub use upload::{UploadConfig, UploadEvent, UploadId, UploadManager, UploadState};
