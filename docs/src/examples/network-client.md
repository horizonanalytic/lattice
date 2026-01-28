# Example: Network Client

A network client demonstrating async HTTP requests, threading, and live updates.

## Overview

This example builds a REST API client with:
- Async HTTP requests using the thread pool
- Live response display with syntax highlighting
- Request history with caching
- Progress indication for long requests

## Key Concepts

- **ThreadPool**: Background HTTP requests
- **Worker**: Long-running network operations
- **Signals**: Progress and completion updates
- **TreeView**: Request history

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Window, Container, Label, TextEdit, PushButton, ComboBox,
    LineEdit, TreeView, Splitter, ProgressBar, TabWidget,
    ButtonVariant, GroupBox
};
use horizon_lattice::widget::layout::{
    VBoxLayout, HBoxLayout, FormLayout, LayoutKind, ContentMargins
};
use horizon_lattice::concurrent::{ThreadPool, Worker, CancellationToken};
use horizon_lattice::model::{TreeModel, TreeNode};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct RequestEntry {
    method: String,
    url: String,
    status: Option<u16>,
    duration_ms: u64,
    response: String,
    timestamp: String,
}

struct HttpClient {
    timeout: Duration,
}

impl HttpClient {
    fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }

    fn request(&self, method: &str, url: &str, body: Option<&str>)
        -> Result<(u16, String, Duration), String>
    {
        let start = Instant::now();

        // Simulated HTTP request - in real implementation, use reqwest or ureq
        // This is a placeholder for demonstration
        std::thread::sleep(Duration::from_millis(500));

        let response = match method {
            "GET" => format!(r#"{{"message": "GET response from {}"}}"#, url),
            "POST" => format!(r#"{{"message": "Created", "data": {}}}"#, body.unwrap_or("{}")),
            "PUT" => format!(r#"{{"message": "Updated"}}"#),
            "DELETE" => format!(r#"{{"message": "Deleted"}}"#),
            _ => r#"{"error": "Unknown method"}"#.to_string(),
        };

        Ok((200, response, start.elapsed()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("HTTP Client")
        .with_size(1000.0, 700.0);

    let pool = ThreadPool::new(4);
    let history: Arc<Mutex<Vec<RequestEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let current_cancel: Arc<Mutex<Option<CancellationToken>>> = Arc::new(Mutex::new(None));

    // Request builder section
    let method_combo = ComboBox::new();
    method_combo.add_items(&["GET", "POST", "PUT", "DELETE", "PATCH"]);

    let url_input = LineEdit::new();
    url_input.set_placeholder("https://api.example.com/endpoint");

    let send_btn = PushButton::new("Send")
        .with_variant(ButtonVariant::Primary);
    let cancel_btn = PushButton::new("Cancel");
    cancel_btn.set_enabled(false);

    // Headers
    let mut headers_form = FormLayout::new();
    let content_type = ComboBox::new();
    content_type.add_items(&["application/json", "text/plain", "application/xml"]);
    headers_form.add_row(Label::new("Content-Type:"), content_type.clone());

    let auth_header = LineEdit::new();
    auth_header.set_placeholder("Bearer token...");
    headers_form.add_row(Label::new("Authorization:"), auth_header);

    let mut headers_group = GroupBox::new("Headers");
    headers_group.set_layout(LayoutKind::from(headers_form));

    // Request body
    let body_edit = TextEdit::new();
    body_edit.set_placeholder("Request body (JSON)...");

    // Response display
    let response_edit = TextEdit::new();
    response_edit.set_read_only(true);

    let status_label = Label::new("Ready");
    let progress = ProgressBar::new();
    progress.set_range(0, 100);
    progress.set_value(0);

    // History tree
    let history_model: Arc<Mutex<TreeModel<String>>> =
        Arc::new(Mutex::new(TreeModel::new()));
    let history_tree = TreeView::new()
        .with_model(history_model.lock().unwrap().clone());

    // URL bar layout
    let mut url_row = HBoxLayout::new();
    url_row.set_spacing(4.0);
    url_row.add_widget(method_combo.object_id());
    url_row.add_widget(url_input.object_id());
    url_row.add_widget(send_btn.object_id());
    url_row.add_widget(cancel_btn.object_id());

    let mut url_container = Container::new();
    url_container.set_layout(LayoutKind::from(url_row));

    // Tabs for body/headers
    let mut tabs = TabWidget::new();

    let mut body_container = Container::new();
    let mut body_layout = VBoxLayout::new();
    body_layout.add_widget(body_edit.object_id());
    body_container.set_layout(LayoutKind::from(body_layout));

    tabs.add_tab("Body", body_container);
    tabs.add_tab("Headers", headers_group);

    // Request panel
    let mut request_layout = VBoxLayout::new();
    request_layout.set_spacing(8.0);
    request_layout.add_widget(url_container.object_id());
    request_layout.add_widget(tabs.object_id());

    let mut request_panel = Container::new();
    request_panel.set_layout(LayoutKind::from(request_layout));

    // Response panel
    let mut response_layout = VBoxLayout::new();
    response_layout.set_spacing(8.0);
    response_layout.add_widget(status_label.object_id());
    response_layout.add_widget(progress.object_id());
    response_layout.add_widget(response_edit.object_id());

    let mut response_panel = Container::new();
    response_panel.set_layout(LayoutKind::from(response_layout));

    // Send button handler
    let url = url_input.clone();
    let method = method_combo.clone();
    let body = body_edit.clone();
    let response = response_edit.clone();
    let status = status_label.clone();
    let prog = progress.clone();
    let send = send_btn.clone();
    let cancel = cancel_btn.clone();
    let cancel_token = current_cancel.clone();
    let hist = history.clone();
    let hist_model = history_model.clone();
    let pool_clone = pool.clone();

    send_btn.clicked().connect(move |_| {
        let url_text = url.text();
        let method_text = method.current_text();
        let body_text = body.text();

        if url_text.is_empty() {
            status.set_text("Please enter a URL");
            return;
        }

        // Disable send, enable cancel
        send.set_enabled(false);
        cancel.set_enabled(true);
        status.set_text("Sending request...");
        prog.set_value(0);

        // Create cancellation token
        let token = CancellationToken::new();
        *cancel_token.lock().unwrap() = Some(token.clone());

        // Clone for closure
        let response_clone = response.clone();
        let status_clone = status.clone();
        let prog_clone = prog.clone();
        let send_clone = send.clone();
        let cancel_clone = cancel.clone();
        let hist_clone = hist.clone();
        let hist_model_clone = hist_model.clone();
        let method_for_history = method_text.clone();
        let url_for_history = url_text.clone();

        pool_clone.spawn(move || {
            let client = HttpClient::new();

            // Simulate progress
            for i in 0..5 {
                if token.is_cancelled() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
                prog_clone.set_value((i + 1) * 20);
            }

            let body_opt = if body_text.is_empty() {
                None
            } else {
                Some(body_text.as_str())
            };

            match client.request(&method_for_history, &url_for_history, body_opt) {
                Ok((code, body, duration)) => {
                    // Format JSON for display
                    let formatted = body; // Could use serde_json for pretty printing

                    response_clone.set_text(&formatted);
                    status_clone.set_text(&format!(
                        "Status: {} | Time: {}ms",
                        code,
                        duration.as_millis()
                    ));

                    // Add to history
                    let entry = RequestEntry {
                        method: method_for_history.clone(),
                        url: url_for_history.clone(),
                        status: Some(code),
                        duration_ms: duration.as_millis() as u64,
                        response: formatted,
                        timestamp: "now".to_string(), // Would use actual timestamp
                    };
                    hist_clone.lock().unwrap().push(entry);

                    // Update tree model
                    let label = format!("{} {} - {}ms",
                        method_for_history, url_for_history, duration.as_millis());
                    hist_model_clone.lock().unwrap().add_root(label);
                }
                Err(e) => {
                    response_clone.set_text(&format!("Error: {}", e));
                    status_clone.set_text("Request failed");
                }
            }

            prog_clone.set_value(100);
            send_clone.set_enabled(true);
            cancel_clone.set_enabled(false);
        });
    });

    // Cancel button handler
    let cancel_token = current_cancel.clone();
    let send = send_btn.clone();
    let cancel = cancel_btn.clone();
    let status = status_label.clone();

    cancel_btn.clicked().connect(move |_| {
        if let Some(token) = cancel_token.lock().unwrap().take() {
            token.cancel();
            status.set_text("Request cancelled");
            send.set_enabled(true);
            cancel.set_enabled(false);
        }
    });

    // History item click - load previous request
    let response = response_edit.clone();
    let hist = history.clone();

    history_tree.clicked.connect(move |index| {
        let entries = hist.lock().unwrap();
        if let Some(entry) = entries.get(index.row() as usize) {
            response.set_text(&entry.response);
        }
    });

    // Main splitter: request/response on top, history on bottom
    let mut top_splitter = Splitter::horizontal();
    top_splitter.add_widget(request_panel.object_id());
    top_splitter.add_widget(response_panel.object_id());
    top_splitter.set_sizes(&[500, 500]);

    let mut main_splitter = Splitter::vertical();
    main_splitter.add_widget(top_splitter.object_id());
    main_splitter.add_widget(history_tree.object_id());
    main_splitter.set_sizes(&[500, 200]);

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(8.0));
    layout.add_widget(main_splitter.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Features Demonstrated

| Feature | Description |
|---------|-------------|
| **ThreadPool** | Background HTTP requests |
| **CancellationToken** | Cancel in-flight requests |
| **ComboBox** | HTTP method selection |
| **TextEdit** | Request body and response display |
| **ProgressBar** | Request progress indication |
| **TreeView** | Request history |
| **Splitter** | Resizable panels |
| **TabWidget** | Body/Headers tabs |

## HTTP Client Patterns

### Async Request Pattern

```rust,ignore
use horizon_lattice::concurrent::{ThreadPool, CancellationToken};

let pool = ThreadPool::new(4);
let token = CancellationToken::new();

// Store token for cancellation
let token_for_cancel = token.clone();

pool.spawn(move || {
    // Check cancellation periodically
    for _ in 0..10 {
        if token.is_cancelled() {
            return;
        }
        // Do work...
    }
});

// Later, cancel if needed
token_for_cancel.cancel();
```

### Progress Reporting

```rust,ignore
use horizon_lattice::concurrent::{ThreadPool, ProgressReporter};

let pool = ThreadPool::new(4);
let (reporter, receiver) = ProgressReporter::new();

pool.spawn(move || {
    for i in 0..100 {
        reporter.set_progress(i as f32 / 100.0);
        // Do work...
    }
});

// In UI thread
receiver.progress_changed.connect(|&progress| {
    progress_bar.set_value((progress * 100.0) as i32);
});
```

## Exercises

1. **Add request persistence**: Save/load request collections
2. **Add response formatting**: Pretty-print JSON/XML
3. **Add authentication presets**: OAuth, Basic Auth, API Key
4. **Add environment variables**: Variable substitution in URLs
5. **Add response testing**: Assertions on response data

## Related Examples

- [Text Editor](./text-editor.md) - File operations
- [Settings Dialog](./settings-dialog.md) - Configuration
