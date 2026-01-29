//! File URI encoding and decoding per RFC 8089.
//!
//! This module provides utilities for converting between filesystem paths and `file://` URIs,
//! as well as handling the `text/uri-list` MIME format per RFC 2483.
//!
//! # File URI Format (RFC 8089)
//!
//! File URIs have the form:
//! - Unix: `file:///path/to/file` (three slashes for local files)
//! - Windows: `file:///C:/path/to/file` (drive letter becomes first path segment)
//!
//! Special characters are percent-encoded:
//! - Space → `%20`
//! - Hash → `%23`
//! - Question mark → `%3F`
//! - Percent → `%25`
//!
//! # URI List Format (RFC 2483)
//!
//! The `text/uri-list` MIME type uses a simple format:
//! - One URI per line
//! - Lines starting with `#` are comments
//! - Lines terminated with CRLF (`\r\n`)
//!
//! # Example
//!
//! ```
//! use horizon_lattice::platform::file_uri;
//!
//! // Convert path to URI
//! # #[cfg(unix)]
//! let uri = file_uri::path_to_uri("/home/user/document.txt");
//! # #[cfg(unix)]
//! assert_eq!(uri, "file:///home/user/document.txt");
//!
//! // Convert URI back to path
//! # #[cfg(unix)]
//! let path = file_uri::uri_to_path("file:///home/user/document.txt");
//! # #[cfg(unix)]
//! assert_eq!(path, Some(std::path::PathBuf::from("/home/user/document.txt")));
//!
//! // Parse a URI list
//! let list = "file:///path/one.txt\r\nfile:///path/two.txt\r\n";
//! let paths = file_uri::parse_uri_list(list);
//! assert_eq!(paths.len(), 2);
//! ```

use std::path::{Path, PathBuf};

/// Characters that must be percent-encoded in file URIs (per RFC 3986 / RFC 8089).
const ENCODE_SET: &[char] = &[
    ' ', '#', '?', '%', '[', ']', '@', '!', '$', '&', '\'', '(', ')', '*', '+', ',', ';', '=',
];

/// Converts a filesystem path to a `file://` URI string.
///
/// # Platform Behavior
///
/// - **Unix**: Paths like `/home/user/file.txt` become `file:///home/user/file.txt`
/// - **Windows**: Paths like `C:\Users\file.txt` become `file:///C:/Users/file.txt`
///
/// # Encoding
///
/// Special characters are percent-encoded per RFC 3986:
/// - Space → `%20`
/// - Hash → `%23`
/// - Non-ASCII characters → UTF-8 percent-encoded
///
/// # Example
///
/// ```
/// use horizon_lattice::platform::file_uri::path_to_uri;
///
/// # #[cfg(unix)]
/// assert_eq!(path_to_uri("/home/user/my file.txt"), "file:///home/user/my%20file.txt");
/// ```
pub fn path_to_uri(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();

    // Convert path to string, handling platform differences
    #[cfg(windows)]
    let path_str = {
        // On Windows, convert backslashes to forward slashes
        let p = path.to_string_lossy();
        p.replace('\\', "/")
    };

    #[cfg(not(windows))]
    let path_str = path.to_string_lossy().into_owned();

    // Build the URI with proper encoding
    let mut result = String::with_capacity(path_str.len() + 10);
    result.push_str("file://");

    // For Windows paths with drive letters (e.g., C:/), add extra slash
    // For Unix absolute paths, the leading / is part of the path
    #[cfg(windows)]
    {
        // Windows: file:///C:/path/to/file
        if path_str.chars().nth(1) == Some(':') {
            result.push('/');
        }
    }

    // Encode path components
    for ch in path_str.chars() {
        if ch == '/' {
            // Forward slashes are path separators, don't encode
            result.push('/');
        } else if ENCODE_SET.contains(&ch) {
            // Percent-encode special characters
            percent_encode_char(&mut result, ch);
        } else if ch.is_ascii() {
            result.push(ch);
        } else {
            // Non-ASCII: UTF-8 percent-encode
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            for byte in encoded.bytes() {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }

    result
}

/// Converts a `file://` URI to a filesystem path.
///
/// # Returns
///
/// - `Some(PathBuf)` if the URI is a valid file URI
/// - `None` if the URI is malformed or not a file URI
///
/// # Platform Behavior
///
/// - **Unix**: `file:///home/user/file.txt` → `/home/user/file.txt`
/// - **Windows**: `file:///C:/Users/file.txt` → `C:\Users\file.txt`
///
/// # Example
///
/// ```
/// use horizon_lattice::platform::file_uri::uri_to_path;
/// use std::path::PathBuf;
///
/// # #[cfg(unix)]
/// assert_eq!(
///     uri_to_path("file:///home/user/file.txt"),
///     Some(PathBuf::from("/home/user/file.txt"))
/// );
///
/// // Percent-encoded characters are decoded
/// # #[cfg(unix)]
/// assert_eq!(
///     uri_to_path("file:///home/user/my%20file.txt"),
///     Some(PathBuf::from("/home/user/my file.txt"))
/// );
///
/// // Non-file URIs return None
/// assert_eq!(uri_to_path("https://example.com"), None);
/// ```
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    // Must start with file://
    let path_part = uri.strip_prefix("file://")?;

    // Handle file:/// (empty authority) - most common for local files
    let path_part = if path_part.starts_with('/') {
        #[cfg(windows)]
        {
            // On Windows, file:///C:/path → skip the leading slash before drive letter
            if path_part.len() > 2 && path_part.chars().nth(2) == Some(':') {
                &path_part[1..]
            } else {
                path_part
            }
        }
        #[cfg(not(windows))]
        {
            path_part
        }
    } else {
        // Handle file://host/path (remote file) - prepend slash for path
        // For simplicity, we treat these as local paths
        path_part
    };

    // Percent-decode the path
    let decoded = percent_decode(path_part)?;

    #[cfg(windows)]
    {
        // Convert forward slashes back to backslashes on Windows
        let native_path = decoded.replace('/', "\\");
        Some(PathBuf::from(native_path))
    }

    #[cfg(not(windows))]
    {
        Some(PathBuf::from(decoded))
    }
}

/// Formats a list of paths as a `text/uri-list` string (RFC 2483).
///
/// The output format is:
/// - One URI per line
/// - Lines terminated with CRLF (`\r\n`)
///
/// # Example
///
/// ```
/// use horizon_lattice::platform::file_uri::format_uri_list;
/// use std::path::PathBuf;
///
/// let paths = vec![
///     PathBuf::from("/home/user/file1.txt"),
///     PathBuf::from("/home/user/file2.txt"),
/// ];
/// let list = format_uri_list(&paths);
/// # #[cfg(unix)]
/// assert!(list.contains("file:///home/user/file1.txt"));
/// ```
pub fn format_uri_list(paths: &[PathBuf]) -> String {
    let mut result = String::new();
    for path in paths {
        result.push_str(&path_to_uri(path));
        result.push_str("\r\n");
    }
    result
}

/// Parses a `text/uri-list` string (RFC 2483) into paths.
///
/// # Format
///
/// - One URI per line
/// - Lines starting with `#` are ignored (comments)
/// - Both CRLF and LF line endings are accepted
/// - Only `file://` URIs are converted to paths; other URIs are skipped
///
/// # Example
///
/// ```
/// use horizon_lattice::platform::file_uri::parse_uri_list;
///
/// let list = "# Comment\r\nfile:///home/user/file.txt\r\nhttps://example.com\r\n";
/// let paths = parse_uri_list(list);
/// # #[cfg(unix)]
/// assert_eq!(paths.len(), 1);
/// ```
pub fn parse_uri_list(uri_list: &str) -> Vec<PathBuf> {
    uri_list
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            uri_to_path(line)
        })
        .collect()
}

/// Parses a `text/uri-list` string and returns all URIs (not just file:// URIs).
///
/// This is useful when you need to handle both file paths and web URLs.
///
/// # Example
///
/// ```
/// use horizon_lattice::platform::file_uri::parse_uri_list_raw;
///
/// let list = "file:///home/file.txt\r\nhttps://example.com\r\n";
/// let uris = parse_uri_list_raw(list);
/// assert_eq!(uris.len(), 2);
/// ```
pub fn parse_uri_list_raw(uri_list: &str) -> Vec<String> {
    uri_list
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect()
}

/// Checks if a string is a valid `file://` URI.
pub fn is_file_uri(uri: &str) -> bool {
    uri.starts_with("file://")
}

/// Percent-encodes a single character into the target string.
fn percent_encode_char(target: &mut String, ch: char) {
    let mut buf = [0u8; 4];
    let encoded = ch.encode_utf8(&mut buf);
    for byte in encoded.bytes() {
        target.push('%');
        target.push_str(&format!("{:02X}", byte));
    }
}

/// Percent-decodes a string.
fn percent_decode(input: &str) -> Option<String> {
    let mut result = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Read two hex digits
            let hex1 = chars.next()?;
            let hex2 = chars.next()?;
            let hex_str: String = [hex1, hex2].iter().collect();
            let byte = u8::from_str_radix(&hex_str, 16).ok()?;
            result.push(byte);
        } else {
            // Regular character - encode as UTF-8
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            result.extend_from_slice(encoded.as_bytes());
        }
    }

    String::from_utf8(result).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_uri_simple() {
        #[cfg(unix)]
        {
            assert_eq!(
                path_to_uri("/home/user/file.txt"),
                "file:///home/user/file.txt"
            );
            assert_eq!(path_to_uri("/"), "file:///");
        }

        #[cfg(windows)]
        {
            assert_eq!(
                path_to_uri("C:\\Users\\file.txt"),
                "file:///C:/Users/file.txt"
            );
            assert_eq!(
                path_to_uri("C:/Users/file.txt"),
                "file:///C:/Users/file.txt"
            );
        }
    }

    #[test]
    fn test_path_to_uri_encoding() {
        #[cfg(unix)]
        {
            assert_eq!(
                path_to_uri("/home/user/my file.txt"),
                "file:///home/user/my%20file.txt"
            );
            assert_eq!(
                path_to_uri("/home/user/file#1.txt"),
                "file:///home/user/file%231.txt"
            );
            assert_eq!(
                path_to_uri("/home/user/file?.txt"),
                "file:///home/user/file%3F.txt"
            );
        }
    }

    #[test]
    fn test_uri_to_path_simple() {
        #[cfg(unix)]
        {
            assert_eq!(
                uri_to_path("file:///home/user/file.txt"),
                Some(PathBuf::from("/home/user/file.txt"))
            );
            assert_eq!(uri_to_path("file:///"), Some(PathBuf::from("/")));
        }

        #[cfg(windows)]
        {
            assert_eq!(
                uri_to_path("file:///C:/Users/file.txt"),
                Some(PathBuf::from("C:\\Users\\file.txt"))
            );
        }
    }

    #[test]
    fn test_uri_to_path_decoding() {
        #[cfg(unix)]
        {
            assert_eq!(
                uri_to_path("file:///home/user/my%20file.txt"),
                Some(PathBuf::from("/home/user/my file.txt"))
            );
            assert_eq!(
                uri_to_path("file:///home/user/file%231.txt"),
                Some(PathBuf::from("/home/user/file#1.txt"))
            );
        }
    }

    #[test]
    fn test_uri_to_path_invalid() {
        assert_eq!(uri_to_path("https://example.com"), None);
        assert_eq!(uri_to_path("not a uri"), None);
        assert_eq!(uri_to_path(""), None);
    }

    #[test]
    fn test_roundtrip() {
        #[cfg(unix)]
        {
            let paths = [
                "/home/user/file.txt",
                "/home/user/my file.txt",
                "/home/user/file#1.txt",
                "/tmp/test",
            ];
            for path_str in paths {
                let path = PathBuf::from(path_str);
                let uri = path_to_uri(&path);
                let recovered = uri_to_path(&uri);
                assert_eq!(recovered, Some(path), "Roundtrip failed for {}", path_str);
            }
        }
    }

    #[test]
    fn test_format_uri_list() {
        #[cfg(unix)]
        {
            let paths = vec![
                PathBuf::from("/home/user/file1.txt"),
                PathBuf::from("/home/user/file2.txt"),
            ];
            let list = format_uri_list(&paths);
            assert!(list.contains("file:///home/user/file1.txt\r\n"));
            assert!(list.contains("file:///home/user/file2.txt\r\n"));
        }
    }

    #[test]
    fn test_parse_uri_list() {
        #[cfg(unix)]
        {
            let list =
                "# Comment\r\nfile:///home/user/file1.txt\r\nfile:///home/user/file2.txt\r\n";
            let paths = parse_uri_list(list);
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0], PathBuf::from("/home/user/file1.txt"));
            assert_eq!(paths[1], PathBuf::from("/home/user/file2.txt"));
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_parse_uri_list_mixed() {
        // Should only return file:// URIs as paths
        let list = "file:///home/file.txt\r\nhttps://example.com\r\n";
        let paths = parse_uri_list(list);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_parse_uri_list_raw() {
        let list = "file:///home/file.txt\r\nhttps://example.com\r\n";
        let uris = parse_uri_list_raw(list);
        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0], "file:///home/file.txt");
        assert_eq!(uris[1], "https://example.com");
    }

    #[test]
    fn test_is_file_uri() {
        assert!(is_file_uri("file:///home/user/file.txt"));
        assert!(is_file_uri("file://localhost/path"));
        assert!(!is_file_uri("https://example.com"));
        assert!(!is_file_uri("/local/path"));
    }

    #[test]
    fn test_unicode_path() {
        #[cfg(unix)]
        {
            let path = PathBuf::from("/home/用户/文件.txt");
            let uri = path_to_uri(&path);
            // Unicode should be percent-encoded
            assert!(uri.starts_with("file:///home/"));
            assert!(uri.contains("%"));
            // Should roundtrip correctly
            let recovered = uri_to_path(&uri);
            assert_eq!(recovered, Some(path));
        }
    }
}
