//! HTTP request types and builder.

use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use serde::Serialize;

use crate::error::Result;
use super::client::{Authentication, HttpClient};
use super::response::HttpResponse;

/// HTTP request methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// HTTP GET method.
    Get,
    /// HTTP POST method.
    Post,
    /// HTTP PUT method.
    Put,
    /// HTTP DELETE method.
    Delete,
    /// HTTP PATCH method.
    Patch,
    /// HTTP HEAD method.
    Head,
    /// HTTP OPTIONS method.
    Options,
}

impl HttpMethod {
    /// Convert to reqwest method.
    pub(crate) fn to_reqwest(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Delete => reqwest::Method::DELETE,
            Self::Patch => reqwest::Method::PATCH,
            Self::Head => reqwest::Method::HEAD,
            Self::Options => reqwest::Method::OPTIONS,
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Patch => write!(f, "PATCH"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

/// The body of an HTTP request.
#[derive(Clone, Debug)]
pub enum RequestBody {
    /// No body.
    None,
    /// Plain text body.
    Text(String),
    /// JSON body (serialized from a value).
    Json(serde_json::Value),
    /// URL-encoded form data.
    Form(HashMap<String, String>),
    /// Raw binary body.
    Bytes(Bytes),
}

impl Default for RequestBody {
    fn default() -> Self {
        Self::None
    }
}

/// A built HTTP request ready to be sent.
#[derive(Debug)]
pub struct HttpRequest {
    /// The HTTP method.
    pub method: HttpMethod,
    /// The request URL.
    pub url: String,
    /// Request headers.
    pub headers: http::HeaderMap,
    /// Query parameters.
    pub query: Vec<(String, String)>,
    /// Request body.
    pub body: RequestBody,
    /// Request timeout override.
    pub timeout: Option<Duration>,
    /// Authentication.
    pub auth: Option<Authentication>,
}

/// Builder for constructing HTTP requests.
pub struct HttpRequestBuilder {
    client: HttpClient,
    method: HttpMethod,
    url: String,
    headers: http::HeaderMap,
    query: Vec<(String, String)>,
    body: RequestBody,
    timeout: Option<Duration>,
    auth: Option<Authentication>,
}

impl HttpRequestBuilder {
    /// Create a new request builder.
    pub(crate) fn new(client: HttpClient, method: HttpMethod, url: String) -> Self {
        Self {
            client,
            method,
            url,
            headers: http::HeaderMap::new(),
            query: Vec::new(),
            body: RequestBody::None,
            timeout: None,
            auth: None,
        }
    }

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

    /// Set a plain text body.
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.body = RequestBody::Text(body.into());
        self
    }

    /// Set a JSON body from a serializable value.
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        match serde_json::to_value(body) {
            Ok(value) => self.body = RequestBody::Json(value),
            Err(e) => {
                tracing::error!(target: "horizon_lattice_net::http", "Failed to serialize JSON body: {}", e);
            }
        }
        self
    }

    /// Set a URL-encoded form body.
    pub fn form(mut self, data: HashMap<String, String>) -> Self {
        self.body = RequestBody::Form(data);
        self
    }

    /// Set a raw binary body.
    pub fn bytes(mut self, body: impl Into<Bytes>) -> Self {
        self.body = RequestBody::Bytes(body.into());
        self
    }

    /// Set basic authentication.
    pub fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: Option<impl Into<String>>,
    ) -> Self {
        self.auth = Some(Authentication::Basic {
            username: username.into(),
            password: password.map(Into::into),
        });
        self
    }

    /// Set bearer token authentication.
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(Authentication::Bearer(token.into()));
        self
    }

    /// Set a timeout for this specific request.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the request without sending it.
    pub fn build(self) -> HttpRequest {
        HttpRequest {
            method: self.method,
            url: self.url,
            headers: self.headers,
            query: self.query,
            body: self.body,
            timeout: self.timeout,
            auth: self.auth,
        }
    }

    /// Send the request and wait for the response.
    pub async fn send(self) -> Result<HttpResponse> {
        let client = self.client.clone();
        let request = self.build();

        // Build the URL with query parameters
        let mut url = url::Url::parse(&request.url)?;
        for (key, value) in &request.query {
            url.query_pairs_mut().append_pair(key, value);
        }

        // Build the reqwest request
        let mut req_builder = client
            .reqwest_client()
            .request(request.method.to_reqwest(), url);

        // Add headers
        for (name, value) in request.headers.iter() {
            req_builder = req_builder.header(name, value);
        }

        // Add authentication
        if let Some(auth) = &request.auth {
            match auth {
                Authentication::Basic { username, password } => {
                    req_builder = req_builder.basic_auth(username, password.as_ref());
                }
                Authentication::Bearer(token) => {
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
}

/// Multipart form data for file uploads.
pub struct MultipartForm {
    inner: reqwest::multipart::Form,
}

impl MultipartForm {
    /// Create a new empty multipart form.
    pub fn new() -> Self {
        Self {
            inner: reqwest::multipart::Form::new(),
        }
    }

    /// Add a text field to the form.
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.text(name.into(), value.into());
        self
    }

    /// Add a file field from bytes.
    pub fn file_bytes(
        mut self,
        name: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
        filename: impl Into<String>,
        mime_type: Option<&str>,
    ) -> Self {
        let bytes_vec: Vec<u8> = bytes.into();
        let filename_str: String = filename.into();
        let part = reqwest::multipart::Part::bytes(bytes_vec.clone())
            .file_name(filename_str.clone());
        // Apply mime type if provided (mime_str consumes self and returns Result<Part>)
        let part = match mime_type {
            Some(mime) => part.mime_str(mime).unwrap_or_else(|e| {
                tracing::warn!(target: "horizon_lattice_net::http", "Invalid MIME type '{}': {}", mime, e);
                // Recreate part without the mime type since the original was consumed
                reqwest::multipart::Part::bytes(bytes_vec).file_name(filename_str)
            }),
            None => part,
        };
        self.inner = self.inner.part(name.into(), part);
        self
    }

    /// Convert to the internal reqwest form.
    pub(crate) fn into_reqwest(self) -> reqwest::multipart::Form {
        self.inner
    }
}

impl Default for MultipartForm {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRequestBuilder {
    /// Set a multipart form body for file uploads.
    pub async fn multipart(self, form: MultipartForm) -> Result<HttpResponse> {
        let client = self.client.clone();

        // Build the URL with query parameters
        let mut url = url::Url::parse(&self.url)?;
        for (key, value) in &self.query {
            url.query_pairs_mut().append_pair(key, value);
        }

        // Build the reqwest request
        let mut req_builder = client
            .reqwest_client()
            .request(self.method.to_reqwest(), url)
            .multipart(form.into_reqwest());

        // Add headers
        for (name, value) in self.headers.iter() {
            req_builder = req_builder.header(name, value);
        }

        // Add authentication
        if let Some(auth) = &self.auth {
            match auth {
                Authentication::Basic { username, password } => {
                    req_builder = req_builder.basic_auth(username, password.as_ref());
                }
                Authentication::Bearer(token) => {
                    req_builder = req_builder.bearer_auth(token);
                }
            }
        }

        // Add timeout
        if let Some(timeout) = self.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        // Send the request
        let response = req_builder.send().await?;
        Ok(HttpResponse::from_reqwest(response))
    }
}
