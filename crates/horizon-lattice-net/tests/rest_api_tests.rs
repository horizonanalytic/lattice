//! Tests for REST API client helpers.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use horizon_lattice_net::http::{
    HttpMethod, RateLimitInfo, RateLimiter, RestApiClient, RetryConfig,
};

#[test]
fn test_rest_api_client_builder() {
    let client = RestApiClient::builder("https://api.example.com")
        .bearer_auth("my-token")
        .json_api()
        .build()
        .expect("Failed to build client");

    assert_eq!(client.base_url(), "https://api.example.com");
}

#[test]
fn test_rest_api_client_base_url_normalization() {
    // Trailing slash should be removed
    let client = RestApiClient::builder("https://api.example.com/")
        .build()
        .expect("Failed to build client");

    assert_eq!(client.base_url(), "https://api.example.com");
}

#[test]
fn test_rest_api_request_builder() {
    let client = RestApiClient::builder("https://api.example.com")
        .bearer_auth("token123")
        .build()
        .expect("Failed to build client");

    // Test path appending
    let request = client.get("/users").build();
    assert_eq!(request.url, "https://api.example.com/users");
    assert_eq!(request.method, HttpMethod::Get);

    // Test path without leading slash
    let request = client.get("users").build();
    assert_eq!(request.url, "https://api.example.com/users");

    // Test POST with query params
    let request = client
        .post("/users")
        .query("include", "profile")
        .build();
    assert_eq!(request.url, "https://api.example.com/users");
    assert_eq!(request.query.len(), 1);
}

#[test]
fn test_rest_api_all_methods() {
    let client = RestApiClient::builder("https://api.example.com")
        .build()
        .expect("Failed to build client");

    assert_eq!(client.get("/").build().method, HttpMethod::Get);
    assert_eq!(client.post("/").build().method, HttpMethod::Post);
    assert_eq!(client.put("/").build().method, HttpMethod::Put);
    assert_eq!(client.delete("/").build().method, HttpMethod::Delete);
    assert_eq!(client.patch("/").build().method, HttpMethod::Patch);
}

#[test]
fn test_api_key_auth() {
    let client = RestApiClient::builder("https://api.example.com")
        .api_key("X-API-Key", "secret-key")
        .build()
        .expect("Failed to build client");

    let request = client.get("/users").build();
    // API key should be in headers
    assert!(request.headers.get("X-API-Key").is_some());
}

#[test]
fn test_auth_override() {
    let client = RestApiClient::builder("https://api.example.com")
        .bearer_auth("default-token")
        .build()
        .expect("Failed to build client");

    // Override auth on request
    let request = client
        .get("/users")
        .bearer_auth("override-token")
        .build();

    // The override token should be used
    match request.auth {
        Some(horizon_lattice_net::http::Authentication::Bearer(token)) => {
            assert_eq!(token, "override-token");
        }
        _ => panic!("Expected bearer auth"),
    }
}

#[test]
fn test_rate_limiter_creation() {
    let limiter = RateLimiter::new(10); // 10 requests per second

    // Should be able to acquire initial tokens
    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());
}

#[test]
fn test_rate_limiter_burst() {
    let limiter = RateLimiter::with_burst(1, 3); // 1/s with burst of 3

    // Should allow burst
    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());
    assert!(limiter.try_acquire());

    // Bucket should be empty now
    assert!(!limiter.try_acquire());
}

#[test]
fn test_rate_limiter_wait_time() {
    let limiter = RateLimiter::with_burst(1, 1);

    // First one should succeed
    assert!(limiter.try_acquire());

    // Should report wait time needed
    assert!(limiter.wait_time().is_some());
}

#[test]
fn test_rate_limit_info_parsing() {
    // Test with no headers (empty info)
    let info = RateLimitInfo::default();
    assert!(!info.is_rate_limited());
    assert!(info.wait_duration().is_none());

    // Test with remaining = 0
    let info = RateLimitInfo {
        limit: Some(100),
        remaining: Some(0),
        reset_timestamp: None,
        retry_after: None,
    };
    assert!(info.is_rate_limited());

    // Test with retry-after
    let info = RateLimitInfo {
        limit: None,
        remaining: None,
        reset_timestamp: None,
        retry_after: Some(Duration::from_secs(30)),
    };
    assert!(info.is_rate_limited());
    assert_eq!(info.wait_duration(), Some(Duration::from_secs(30)));
}

#[test]
fn test_retry_config() {
    // Build client with custom retry config
    let client = RestApiClient::builder("https://api.example.com")
        .retry(RetryConfig {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
        })
        .build()
        .expect("Failed to build client");

    assert_eq!(client.base_url(), "https://api.example.com");
}

#[test]
fn test_no_retry() {
    // Build client with no retries
    let client = RestApiClient::builder("https://api.example.com")
        .no_retry()
        .build()
        .expect("Failed to build client");

    assert_eq!(client.base_url(), "https://api.example.com");
}

#[test]
fn test_request_interceptor() {
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    let client = RestApiClient::builder("https://api.example.com")
        .add_request_interceptor(move |_request| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .build()
        .expect("Failed to build client");

    // Build a request (interceptor runs on send, not build)
    let _request = client.get("/users").build();

    // Counter should still be 0 since we only built, not sent
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[test]
fn test_json_body() {
    let client = RestApiClient::builder("https://api.example.com")
        .build()
        .expect("Failed to build client");

    let request = client
        .post("/users")
        .json(&serde_json::json!({
            "name": "John",
            "email": "john@example.com"
        }))
        .build();

    matches!(request.body, horizon_lattice_net::http::RequestBody::Json(_));
}

#[test]
fn test_client_is_clone() {
    let client = RestApiClient::builder("https://api.example.com")
        .build()
        .expect("Failed to build client");

    let client2 = client.clone();
    assert_eq!(client.base_url(), client2.base_url());
}

#[test]
fn test_client_is_debug() {
    let client = RestApiClient::builder("https://api.example.com")
        .bearer_auth("token")
        .rate_limit_per_second(10)
        .build()
        .expect("Failed to build client");

    let debug_str = format!("{:?}", client);
    assert!(debug_str.contains("RestApiClient"));
    assert!(debug_str.contains("has_auth"));
    assert!(debug_str.contains("has_rate_limiter"));
}
