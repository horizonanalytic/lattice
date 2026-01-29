//! Tests for download and upload managers.

use horizon_lattice_net::http::{
    DownloadEvent, DownloadManager, DownloadState, UploadEvent, UploadManager, UploadState,
};

#[tokio::test]
async fn test_download_manager_creation() {
    let manager = DownloadManager::new();
    assert!(manager.retry_config().max_retries == 3);
}

#[tokio::test]
async fn test_upload_manager_creation() {
    let manager = UploadManager::new();
    assert!(manager.config().chunk_size > 0);
}

#[tokio::test]
async fn test_download_state_transitions() {
    // Test that state enum values are correct
    assert_ne!(DownloadState::Pending, DownloadState::Downloading);
    assert_ne!(DownloadState::Downloading, DownloadState::Paused);
    assert_ne!(DownloadState::Paused, DownloadState::Completed);
    assert_ne!(DownloadState::Completed, DownloadState::Failed);
}

#[tokio::test]
async fn test_upload_state_transitions() {
    // Test that state enum values are correct
    assert_ne!(UploadState::Pending, UploadState::Creating);
    assert_ne!(UploadState::Creating, UploadState::Uploading);
    assert_ne!(UploadState::Uploading, UploadState::Paused);
    assert_ne!(UploadState::Paused, UploadState::Completed);
}

#[tokio::test]
async fn test_download_event_id() {
    use horizon_lattice_net::http::DownloadId;

    // Test that events carry the correct ID
    let events = [DownloadEvent::Started {
            id: unsafe { std::mem::transmute::<u64, DownloadId>(1) },
        },
        DownloadEvent::Progress {
            id: unsafe { std::mem::transmute::<u64, DownloadId>(2) },
            bytes_downloaded: 100,
            total_bytes: Some(1000),
        },
        DownloadEvent::Finished {
            id: unsafe { std::mem::transmute::<u64, DownloadId>(3) },
            path: "/tmp/test".into(),
        },
        DownloadEvent::Error {
            id: unsafe { std::mem::transmute::<u64, DownloadId>(4) },
            message: "test error".to_string(),
        }];

    for (i, event) in events.iter().enumerate() {
        let id_val: u64 = unsafe { std::mem::transmute(event.id()) };
        assert_eq!(id_val, (i + 1) as u64);
    }
}

#[tokio::test]
async fn test_upload_event_id() {
    use horizon_lattice_net::http::UploadId;

    // Test that events carry the correct ID
    let events = [UploadEvent::Started {
            id: unsafe { std::mem::transmute::<u64, UploadId>(1) },
        },
        UploadEvent::Progress {
            id: unsafe { std::mem::transmute::<u64, UploadId>(2) },
            bytes_uploaded: 100,
            total_bytes: 1000,
        },
        UploadEvent::Finished {
            id: unsafe { std::mem::transmute::<u64, UploadId>(3) },
            url: Some("https://example.com/upload/123".to_string()),
        },
        UploadEvent::Error {
            id: unsafe { std::mem::transmute::<u64, UploadId>(4) },
            message: "test error".to_string(),
        }];

    for (i, event) in events.iter().enumerate() {
        let id_val: u64 = unsafe { std::mem::transmute(event.id()) };
        assert_eq!(id_val, (i + 1) as u64);
    }
}

// Integration tests with wiremock
#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;
    use parking_lot::Mutex;
    use std::io::Write;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::NamedTempFile;
    use tokio::time::timeout;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_download_basic() {
        let mock_server = MockServer::start().await;
        let test_content = b"Hello, World! This is test content.";

        Mock::given(method("GET"))
            .and(path("/test-file.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(test_content.to_vec())
                    .insert_header("Content-Length", test_content.len().to_string()),
            )
            .mount(&mock_server)
            .await;

        let manager = DownloadManager::new();

        // Track events
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        manager.event.connect(move |event| {
            events_clone.lock().push(event.clone());
        });

        // Create temp file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        // Start download
        let url = format!("{}/test-file.txt", mock_server.uri());
        let id = manager
            .download(&url, &path)
            .expect("Failed to start download");

        // Wait for completion
        let result = timeout(Duration::from_secs(5), async {
            loop {
                let state = manager.state(id);
                if matches!(
                    state,
                    Some(DownloadState::Completed) | Some(DownloadState::Failed)
                ) {
                    return state;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert!(result.is_ok(), "Download timed out");
        assert_eq!(result.unwrap(), Some(DownloadState::Completed));

        // Verify file content
        let content = std::fs::read(&path).expect("Failed to read file");
        assert_eq!(content, test_content);

        // Verify events
        let events = events.lock();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DownloadEvent::Started { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DownloadEvent::Finished { .. }))
        );
    }

    #[tokio::test]
    async fn test_download_with_range_support() {
        let mock_server = MockServer::start().await;
        let test_content = b"0123456789ABCDEF"; // 16 bytes

        // First request without Range header
        Mock::given(method("GET"))
            .and(path("/range-test.bin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(test_content.to_vec())
                    .insert_header("Content-Length", test_content.len().to_string())
                    .insert_header("Accept-Ranges", "bytes"),
            )
            .mount(&mock_server)
            .await;

        let manager = DownloadManager::new();
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        let url = format!("{}/range-test.bin", mock_server.uri());
        let id = manager
            .download(&url, &path)
            .expect("Failed to start download");

        // Wait for completion
        let result = timeout(Duration::from_secs(5), async {
            loop {
                let state = manager.state(id);
                if matches!(
                    state,
                    Some(DownloadState::Completed) | Some(DownloadState::Failed)
                ) {
                    return state;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert_eq!(result.unwrap(), Some(DownloadState::Completed));
    }

    #[tokio::test]
    async fn test_download_cancel() {
        let mock_server = MockServer::start().await;

        // Slow response to give time to cancel
        Mock::given(method("GET"))
            .and(path("/slow-file.bin"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
            .mount(&mock_server)
            .await;

        let manager = DownloadManager::new();
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_path_buf();

        let url = format!("{}/slow-file.bin", mock_server.uri());
        let id = manager
            .download(&url, &path)
            .expect("Failed to start download");

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cancel the download
        let cancelled = manager.cancel(id);
        assert!(cancelled);

        // Check state
        let state = manager.state(id);
        assert_eq!(state, Some(DownloadState::Cancelled));
    }

    #[tokio::test]
    async fn test_upload_tus_creation() {
        let mock_server = MockServer::start().await;

        // Mock Tus creation endpoint
        Mock::given(method("POST"))
            .and(path("/uploads"))
            .and(header("Tus-Resumable", "1.0.0"))
            .and(header("Upload-Length", "13"))
            .respond_with(
                ResponseTemplate::new(201)
                    .insert_header("Location", format!("{}/uploads/abc123", mock_server.uri())),
            )
            .mount(&mock_server)
            .await;

        // Mock HEAD request for offset
        Mock::given(method("HEAD"))
            .and(path("/uploads/abc123"))
            .and(header("Tus-Resumable", "1.0.0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Upload-Offset", "0")
                    .insert_header("Upload-Length", "13"),
            )
            .mount(&mock_server)
            .await;

        // Mock PATCH request for upload
        Mock::given(method("PATCH"))
            .and(path("/uploads/abc123"))
            .and(header("Tus-Resumable", "1.0.0"))
            .and(header("Content-Type", "application/offset+octet-stream"))
            .respond_with(ResponseTemplate::new(204).insert_header("Upload-Offset", "13"))
            .mount(&mock_server)
            .await;

        let manager = UploadManager::new();

        // Create temp file with content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        temp_file
            .write_all(b"Hello, World!")
            .expect("Failed to write to temp file");
        temp_file.flush().expect("Failed to flush temp file");

        let endpoint = format!("{}/uploads", mock_server.uri());
        let id = manager
            .upload_tus(temp_file.path(), &endpoint)
            .expect("Failed to start upload");

        // Wait for completion
        let result = timeout(Duration::from_secs(5), async {
            loop {
                let state = manager.state(id);
                if matches!(
                    state,
                    Some(UploadState::Completed) | Some(UploadState::Failed)
                ) {
                    return state;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert!(result.is_ok(), "Upload timed out");
        assert_eq!(result.unwrap(), Some(UploadState::Completed));

        // Verify upload URL was stored
        let upload_url = manager.upload_url(id);
        assert!(upload_url.is_some());
        assert!(upload_url.unwrap().contains("/uploads/abc123"));
    }
}
