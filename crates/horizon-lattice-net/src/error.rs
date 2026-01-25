//! Error types for the networking module.

use std::fmt;

/// Network-specific errors.
#[derive(Debug, Clone)]
pub enum NetworkError {
    /// HTTP request failed.
    Request(String),
    /// Invalid URL provided.
    InvalidUrl(String),
    /// Request timed out.
    Timeout,
    /// Connection refused or failed.
    Connection(String),
    /// TLS/SSL error.
    Tls(String),
    /// Invalid header name or value.
    InvalidHeader(String),
    /// JSON serialization/deserialization error.
    Json(String),
    /// I/O error.
    Io(String),
    /// Request was cancelled.
    Cancelled,
    /// Invalid response body.
    InvalidBody(String),
    /// HTTP error status (4xx or 5xx).
    HttpStatus {
        /// The HTTP status code.
        status: u16,
        /// Optional error message from the response body.
        message: Option<String>,
    },
    /// Redirect limit exceeded.
    TooManyRedirects,
    /// Proxy configuration error.
    Proxy(String),
    /// Authentication failed.
    Authentication(String),
    /// WebSocket error.
    WebSocket(String),
    /// TCP socket error.
    TcpSocket(String),
    /// UDP socket error.
    UdpSocket(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(msg) => write!(f, "HTTP request error: {msg}"),
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {msg}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Connection(msg) => write!(f, "Connection error: {msg}"),
            Self::Tls(msg) => write!(f, "TLS error: {msg}"),
            Self::InvalidHeader(msg) => write!(f, "Invalid header: {msg}"),
            Self::Json(msg) => write!(f, "JSON error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Cancelled => write!(f, "Request was cancelled"),
            Self::InvalidBody(msg) => write!(f, "Invalid response body: {msg}"),
            Self::HttpStatus { status, message } => {
                if let Some(msg) = message {
                    write!(f, "HTTP {status}: {msg}")
                } else {
                    write!(f, "HTTP {status}")
                }
            }
            Self::TooManyRedirects => write!(f, "Too many redirects"),
            Self::Proxy(msg) => write!(f, "Proxy error: {msg}"),
            Self::Authentication(msg) => write!(f, "Authentication error: {msg}"),
            Self::WebSocket(msg) => write!(f, "WebSocket error: {msg}"),
            Self::TcpSocket(msg) => write!(f, "TCP socket error: {msg}"),
            Self::UdpSocket(msg) => write!(f, "UDP socket error: {msg}"),
        }
    }
}

impl std::error::Error for NetworkError {}

impl From<reqwest::Error> for NetworkError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else if err.is_connect() {
            Self::Connection(err.to_string())
        } else if err.is_redirect() {
            Self::TooManyRedirects
        } else {
            Self::Request(err.to_string())
        }
    }
}

impl From<url::ParseError> for NetworkError {
    fn from(err: url::ParseError) -> Self {
        Self::InvalidUrl(err.to_string())
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<http::header::InvalidHeaderName> for NetworkError {
    fn from(err: http::header::InvalidHeaderName) -> Self {
        Self::InvalidHeader(err.to_string())
    }
}

impl From<http::header::InvalidHeaderValue> for NetworkError {
    fn from(err: http::header::InvalidHeaderValue) -> Self {
        Self::InvalidHeader(err.to_string())
    }
}

/// A specialized Result type for network operations.
pub type Result<T> = std::result::Result<T, NetworkError>;
