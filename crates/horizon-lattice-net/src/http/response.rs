//! HTTP response types.

use bytes::Bytes;
use serde::de::DeserializeOwned;

use crate::error::{NetworkError, Result};

/// An HTTP response from a request.
pub struct HttpResponse {
    inner: reqwest::Response,
}

impl HttpResponse {
    /// Create from a reqwest response.
    pub(crate) fn from_reqwest(response: reqwest::Response) -> Self {
        Self { inner: response }
    }

    /// Get the HTTP status code.
    pub fn status(&self) -> u16 {
        self.inner.status().as_u16()
    }

    /// Check if the response indicates success (2xx status).
    pub fn is_success(&self) -> bool {
        self.inner.status().is_success()
    }

    /// Check if the response is a client error (4xx status).
    pub fn is_client_error(&self) -> bool {
        self.inner.status().is_client_error()
    }

    /// Check if the response is a server error (5xx status).
    pub fn is_server_error(&self) -> bool {
        self.inner.status().is_server_error()
    }

    /// Get the response headers.
    pub fn headers(&self) -> &http::HeaderMap {
        self.inner.headers()
    }

    /// Get a specific header value.
    pub fn header(&self, name: impl AsRef<str>) -> Option<&str> {
        self.inner
            .headers()
            .get(name.as_ref())
            .and_then(|v| v.to_str().ok())
    }

    /// Get the Content-Type header value.
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Get the Content-Length header value.
    pub fn content_length(&self) -> Option<u64> {
        self.inner.content_length()
    }

    /// Get the final URL after redirects.
    pub fn url(&self) -> &str {
        self.inner.url().as_str()
    }

    /// Get the response body as text.
    pub async fn text(self) -> Result<String> {
        Ok(self.inner.text().await?)
    }

    /// Get the response body as raw bytes.
    pub async fn bytes(self) -> Result<Bytes> {
        Ok(self.inner.bytes().await?)
    }

    /// Parse the response body as JSON.
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        Ok(self.inner.json().await?)
    }

    /// Get a streaming response body for large downloads.
    pub fn bytes_stream(self) -> ResponseBody {
        ResponseBody {
            inner: ResponseBodyInner::Stream(self.inner),
            total_size: None,
            bytes_received: 0,
        }
    }

    /// Check if the status code indicates success, returning an error if not.
    pub fn error_for_status(self) -> Result<Self> {
        let status = self.status();
        if self.is_success() {
            Ok(self)
        } else {
            Err(NetworkError::HttpStatus {
                status,
                message: None,
            })
        }
    }

    /// Check if the status code indicates success, consuming the body for the error message.
    pub async fn error_for_status_with_body(self) -> Result<Self> {
        let status = self.status();
        if self.is_success() {
            Ok(self)
        } else {
            // Try to get the body for the error message
            let message = self.text().await.ok();
            Err(NetworkError::HttpStatus { status, message })
        }
    }
}

impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status())
            .field("url", &self.url())
            .finish()
    }
}

enum ResponseBodyInner {
    Stream(reqwest::Response),
}

/// A streaming response body with progress tracking.
pub struct ResponseBody {
    inner: ResponseBodyInner,
    total_size: Option<u64>,
    bytes_received: u64,
}

impl ResponseBody {
    /// Get the total size of the response, if known.
    pub fn total_size(&self) -> Option<u64> {
        self.total_size
    }

    /// Get the number of bytes received so far.
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    /// Read the next chunk of data.
    ///
    /// Returns `None` when the stream is complete.
    pub async fn next_chunk(&mut self) -> Result<Option<Bytes>> {
        match &mut self.inner {
            ResponseBodyInner::Stream(response) => {
                // Get content length on first call
                if self.total_size.is_none() {
                    self.total_size = response.content_length();
                }

                match response.chunk().await? {
                    Some(chunk) => {
                        self.bytes_received += chunk.len() as u64;
                        Ok(Some(chunk))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// Collect all remaining chunks into a single buffer.
    pub async fn collect(mut self) -> Result<Bytes> {
        let mut buffer = Vec::new();
        while let Some(chunk) = self.next_chunk().await? {
            buffer.extend_from_slice(&chunk);
        }
        Ok(Bytes::from(buffer))
    }

    /// Download to a writer (e.g., a file) with optional progress callback.
    pub async fn download_to<W, F>(mut self, mut writer: W, mut on_progress: F) -> Result<u64>
    where
        W: std::io::Write,
        F: FnMut(u64, Option<u64>),
    {
        let mut total = 0u64;
        while let Some(chunk) = self.next_chunk().await? {
            writer.write_all(&chunk)?;
            total += chunk.len() as u64;
            on_progress(total, self.total_size);
        }
        writer.flush()?;
        Ok(total)
    }
}

/// Progress information for downloads/uploads.
#[derive(Clone, Debug)]
pub struct TransferProgress {
    /// Number of bytes transferred so far.
    pub bytes_transferred: u64,
    /// Total number of bytes, if known.
    pub total_bytes: Option<u64>,
}

impl TransferProgress {
    /// Get the progress as a percentage (0.0 to 1.0), if total is known.
    pub fn fraction(&self) -> Option<f64> {
        self.total_bytes.map(|total| {
            if total == 0 {
                1.0
            } else {
                self.bytes_transferred as f64 / total as f64
            }
        })
    }

    /// Get the progress as a percentage (0 to 100), if total is known.
    pub fn percent(&self) -> Option<u8> {
        self.fraction().map(|f| (f * 100.0).min(100.0) as u8)
    }
}
