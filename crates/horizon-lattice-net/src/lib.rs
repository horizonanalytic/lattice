//! Networking module for Horizon Lattice.
//!
//! This crate provides networking capabilities for Horizon Lattice applications:
//!
//! - **HTTP Client**: Full-featured HTTP client with async support
//! - **WebSocket**: Real-time bidirectional communication (planned)
//! - **TCP/UDP Sockets**: Low-level socket communication (planned)
//!
//! # HTTP Client
//!
//! The HTTP client provides a high-level API for making HTTP requests:
//!
//! ```ignore
//! use horizon_lattice_net::http::HttpClient;
//!
//! // Create a client
//! let client = HttpClient::new();
//!
//! // Make a request
//! let response = client.get("https://api.example.com/data")
//!     .header("Accept", "application/json")
//!     .send()
//!     .await?;
//!
//! // Read the response
//! let data: MyData = response.json().await?;
//! ```
//!
//! ## Request Methods
//!
//! The client supports all common HTTP methods:
//!
//! - `get(url)` - GET request
//! - `post(url)` - POST request
//! - `put(url)` - PUT request
//! - `delete(url)` - DELETE request
//! - `patch(url)` - PATCH request
//! - `head(url)` - HEAD request
//!
//! ## Request Bodies
//!
//! ```ignore
//! // JSON body
//! client.post("/api/users")
//!     .json(&serde_json::json!({"name": "John"}))
//!     .send()
//!     .await?;
//!
//! // Form data
//! let mut form = HashMap::new();
//! form.insert("username".to_string(), "john".to_string());
//! client.post("/login").form(form).send().await?;
//!
//! // Multipart (file upload)
//! let form = MultipartForm::new()
//!     .text("name", "John")
//!     .file_bytes("avatar", file_bytes, "avatar.png", Some("image/png"));
//! client.post("/upload").multipart(form).await?;
//! ```
//!
//! ## Configuration
//!
//! ```ignore
//! let client = HttpClient::builder()
//!     .timeout(Duration::from_secs(60))
//!     .user_agent("MyApp/1.0")
//!     .no_cookies()
//!     .build()?;
//! ```
//!
//! # Signal-Based Async
//!
//! For GUI integration, use `AsyncHttpClient` which emits signals:
//!
//! ```ignore
//! use horizon_lattice_net::http::{AsyncHttpClient, RequestStatus};
//!
//! let client = AsyncHttpClient::new();
//!
//! // Connect to the completion signal
//! client.request_finished.connect(|status| {
//!     match status {
//!         RequestStatus::Success { id, status_code, .. } => {
//!             println!("Request {:?} completed with status {}", id, status_code);
//!         }
//!         RequestStatus::Error { id, message } => {
//!             println!("Request {:?} failed: {}", id, message);
//!         }
//!         RequestStatus::Cancelled { id } => {
//!             println!("Request {:?} was cancelled", id);
//!         }
//!     }
//! });
//!
//! // Start request (returns immediately)
//! let handle = client.get_async("https://api.example.com/data");
//!
//! // Cancel if needed
//! handle.cancel();
//! ```

mod error;
pub mod http;

pub use error::{NetworkError, Result};

// Re-export commonly used types at the crate root
pub use http::{
    AsyncHttpClient, Authentication, HttpClient, HttpClientBuilder, HttpMethod, HttpRequest,
    HttpRequestBuilder, HttpResponse, MultipartForm, RequestBody, RequestHandle, RequestId,
    RequestStatus, ResponseBody, TransferProgress,
};
