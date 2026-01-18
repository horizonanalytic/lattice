//! Integration tests for the HTTP client.

use horizon_lattice_net::http::{HttpClient, HttpClientBuilder, HttpMethod};
use std::time::Duration;

#[tokio::test]
async fn test_client_creation() {
    let client = HttpClient::new();
    assert!(client.config().timeout.is_some());
    assert!(client.config().cookies_enabled);
}

#[tokio::test]
async fn test_client_builder() {
    let client = HttpClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .no_cookies()
        .max_redirects(5)
        .build()
        .expect("Failed to build client");

    assert_eq!(client.config().timeout, Some(Duration::from_secs(60)));
    assert!(!client.config().cookies_enabled);
    assert_eq!(client.config().max_redirects, 5);
}

#[tokio::test]
async fn test_request_builder_methods() {
    let client = HttpClient::new();

    // Test that all HTTP methods create valid request builders
    let _ = client.get("https://example.com");
    let _ = client.post("https://example.com");
    let _ = client.put("https://example.com");
    let _ = client.delete("https://example.com");
    let _ = client.patch("https://example.com");
    let _ = client.head("https://example.com");
    let _ = client.request(HttpMethod::Options, "https://example.com");
}

#[tokio::test]
async fn test_request_builder_chain() {
    let client = HttpClient::new();

    // Test builder chaining
    let request = client
        .post("https://example.com/api")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer token123")
        .query("page", "1")
        .query("limit", "10")
        .timeout(Duration::from_secs(5))
        .build();

    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.url, "https://example.com/api");
    assert!(!request.headers.is_empty());
    assert_eq!(request.query.len(), 2);
    assert_eq!(request.timeout, Some(Duration::from_secs(5)));
}

#[tokio::test]
async fn test_json_body() {
    let client = HttpClient::new();

    let request = client
        .post("https://example.com/api")
        .json(&serde_json::json!({
            "name": "test",
            "value": 42
        }))
        .build();

    matches!(request.body, horizon_lattice_net::http::RequestBody::Json(_));
}

#[tokio::test]
async fn test_form_body() {
    use std::collections::HashMap;

    let client = HttpClient::new();
    let mut form_data = HashMap::new();
    form_data.insert("username".to_string(), "testuser".to_string());
    form_data.insert("password".to_string(), "secret".to_string());

    let request = client.post("https://example.com/login").form(form_data).build();

    matches!(request.body, horizon_lattice_net::http::RequestBody::Form(_));
}

#[tokio::test]
async fn test_basic_auth() {
    let client = HttpClient::new();

    let request = client
        .get("https://example.com/api")
        .basic_auth("user", Some("pass"))
        .build();

    assert!(request.auth.is_some());
}

#[tokio::test]
async fn test_bearer_auth() {
    let client = HttpClient::new();

    let request = client
        .get("https://example.com/api")
        .bearer_auth("my-token-123")
        .build();

    assert!(request.auth.is_some());
}

#[tokio::test]
async fn test_multipart_form() {
    use horizon_lattice_net::http::MultipartForm;

    // Just verify multipart form constructs without panic
    let _form = MultipartForm::new()
        .text("field1", "value1")
        .text("field2", "value2")
        .file_bytes("file", vec![1, 2, 3, 4], "test.bin", Some("application/octet-stream"));
}

// Note: We use wiremock for mocked HTTP tests
#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let response = client
            .get(&format!("{}/test", mock_server.uri()))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        assert!(response.is_success());

        let body = response.text().await.expect("Failed to read body");
        assert_eq!(body, "Hello, World!");
    }

    #[tokio::test]
    async fn test_post_json_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/users"))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(serde_json::json!({"id": 1, "name": "John"})),
            )
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let response = client
            .post(&format!("{}/api/users", mock_server.uri()))
            .json(&serde_json::json!({"name": "John"}))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 201);

        let data: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert_eq!(data["id"], 1);
        assert_eq!(data["name"], "John");
    }

    #[tokio::test]
    async fn test_timeout() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&mock_server)
            .await;

        let client = HttpClient::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .expect("Failed to build client");

        let result = client.get(&format!("{}/slow", mock_server.uri())).send().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_status() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = HttpClient::new();
        let response = client
            .get(&format!("{}/not-found", mock_server.uri()))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 404);
        assert!(response.is_client_error());
        assert!(!response.is_success());
    }
}
