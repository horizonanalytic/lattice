//! Convenience functions for common file operations.
//!
//! These functions provide a simple API for one-shot file operations.
//! For more complex scenarios, use `File` and `FileWriter` directly.

use std::fs;
use std::path::Path;

use super::error::{FileError, FileResult};
use super::reader::File;
use super::writer::{AtomicWriter, FileWriter};

// ============================================================================
// Reading Functions
// ============================================================================

/// Reads the entire contents of a file as a string.
///
/// # Example
///
/// ```ignore
/// let content = read_text("config.txt")?;
/// println!("{}", content);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - The file cannot be read
/// - The file is not valid UTF-8
pub fn read_text(path: impl AsRef<Path>) -> FileResult<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| FileError::from_io(e, path))
}

/// Reads the entire contents of a file as bytes.
///
/// # Example
///
/// ```ignore
/// let bytes = read_bytes("data.bin")?;
/// println!("{} bytes", bytes.len());
/// ```
///
/// # Errors
///
/// Returns an error if the file does not exist or cannot be read.
pub fn read_bytes(path: impl AsRef<Path>) -> FileResult<Vec<u8>> {
    let path = path.as_ref();
    fs::read(path).map_err(|e| FileError::from_io(e, path))
}

/// Reads a file line by line and returns a vector of lines.
///
/// Lines are returned without their trailing newline characters.
///
/// # Example
///
/// ```ignore
/// let lines = read_lines("log.txt")?;
/// for line in lines {
///     println!("{}", line);
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be read or a line is not valid UTF-8.
pub fn read_lines(path: impl AsRef<Path>) -> FileResult<Vec<String>> {
    let file = File::open(path)?;
    file.lines().collect()
}

// ============================================================================
// Writing Functions
// ============================================================================

/// Writes a string to a file, creating it if it doesn't exist.
///
/// If the file already exists, its contents are replaced.
///
/// # Example
///
/// ```ignore
/// write_text("output.txt", "Hello, World!")?;
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn write_text(path: impl AsRef<Path>, contents: impl AsRef<str>) -> FileResult<()> {
    let path = path.as_ref();
    fs::write(path, contents.as_ref()).map_err(|e| FileError::from_io(e, path))
}

/// Writes bytes to a file, creating it if it doesn't exist.
///
/// If the file already exists, its contents are replaced.
///
/// # Example
///
/// ```ignore
/// write_bytes("data.bin", &[0x00, 0x01, 0x02])?;
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn write_bytes(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> FileResult<()> {
    let path = path.as_ref();
    fs::write(path, contents.as_ref()).map_err(|e| FileError::from_io(e, path))
}

/// Appends a string to a file, creating it if it doesn't exist.
///
/// # Example
///
/// ```ignore
/// append_text("log.txt", "New log entry\n")?;
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn append_text(path: impl AsRef<Path>, contents: impl AsRef<str>) -> FileResult<()> {
    let mut writer = FileWriter::append(path)?;
    writer.write_str(contents.as_ref())
}

/// Appends bytes to a file, creating it if it doesn't exist.
///
/// # Example
///
/// ```ignore
/// append_bytes("data.bin", &[0x03, 0x04])?;
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn append_bytes(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> FileResult<()> {
    let mut writer = FileWriter::append(path)?;
    writer.write_all(contents.as_ref())
}

// ============================================================================
// File Operations
// ============================================================================

/// Copies a file from one location to another.
///
/// If the destination exists, it will be overwritten.
///
/// # Example
///
/// ```ignore
/// copy_file("source.txt", "dest.txt")?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The source file does not exist
/// - The source cannot be read
/// - The destination cannot be written
pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> FileResult<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::copy(from, to).map_err(|e| {
        // Try to provide the most relevant path in the error
        if !from.exists() {
            FileError::from_io(e, from)
        } else {
            FileError::from_io(e, to)
        }
    })
}

/// Renames or moves a file.
///
/// This works across directories on the same filesystem. For cross-filesystem
/// moves, use `copy_file` followed by `remove_file`.
///
/// # Example
///
/// ```ignore
/// rename_file("old_name.txt", "new_name.txt")?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The source file does not exist
/// - The rename operation fails (e.g., cross-filesystem move)
pub fn rename_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> FileResult<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    fs::rename(from, to).map_err(|e| {
        if !from.exists() {
            FileError::from_io(e, from)
        } else {
            FileError::from_io(e, to)
        }
    })
}

/// Removes a file.
///
/// # Example
///
/// ```ignore
/// remove_file("temp.txt")?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - The file cannot be removed (permission denied, etc.)
pub fn remove_file(path: impl AsRef<Path>) -> FileResult<()> {
    let path = path.as_ref();
    fs::remove_file(path).map_err(|e| FileError::from_io(e, path))
}

/// Writes a file atomically using a temporary file and rename.
///
/// This is useful for configuration files and other critical data where
/// partial writes could cause corruption. The write either succeeds
/// completely or leaves any existing file untouched.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::atomic_write;
///
/// atomic_write("config.json", |w| {
///     w.write_all(b"{\"version\": 1}")
/// })?;
/// ```
///
/// # How it works
///
/// 1. Creates a temporary file in the same directory as the target
/// 2. Writes data to the temporary file
/// 3. Syncs the temporary file to disk
/// 4. Atomically renames the temporary file to the target path
///
/// If any step fails, the original file (if any) is left unchanged.
pub fn atomic_write<F>(path: impl AsRef<Path>, f: F) -> FileResult<()>
where
    F: FnOnce(&mut AtomicWriter) -> FileResult<()>,
{
    AtomicWriter::write(path, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("horizon_ops_test_{}", name))
    }

    fn cleanup(path: &std::path::Path) {
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_read_write_text() {
        let path = temp_path("rw_text.txt");
        cleanup(&path);

        write_text(&path, "Hello, World!").unwrap();
        let content = read_text(&path).unwrap();
        assert_eq!(content, "Hello, World!");

        cleanup(&path);
    }

    #[test]
    fn test_read_write_bytes() {
        let path = temp_path("rw_bytes.bin");
        cleanup(&path);

        let data = vec![0x00, 0x01, 0x02, 0x03, 0xFF];
        write_bytes(&path, &data).unwrap();
        let content = read_bytes(&path).unwrap();
        assert_eq!(content, data);

        cleanup(&path);
    }

    #[test]
    fn test_read_lines() {
        let path = temp_path("read_lines.txt");
        cleanup(&path);

        write_text(&path, "line 1\nline 2\nline 3").unwrap();
        let lines = read_lines(&path).unwrap();
        assert_eq!(lines, vec!["line 1", "line 2", "line 3"]);

        cleanup(&path);
    }

    #[test]
    fn test_append_text() {
        let path = temp_path("append_text.txt");
        cleanup(&path);

        write_text(&path, "first\n").unwrap();
        append_text(&path, "second\n").unwrap();

        let content = read_text(&path).unwrap();
        assert_eq!(content, "first\nsecond\n");

        cleanup(&path);
    }

    #[test]
    fn test_append_bytes() {
        let path = temp_path("append_bytes.bin");
        cleanup(&path);

        write_bytes(&path, &[0x01, 0x02]).unwrap();
        append_bytes(&path, &[0x03, 0x04]).unwrap();

        let content = read_bytes(&path).unwrap();
        assert_eq!(content, vec![0x01, 0x02, 0x03, 0x04]);

        cleanup(&path);
    }

    #[test]
    fn test_copy_file() {
        let src = temp_path("copy_src.txt");
        let dst = temp_path("copy_dst.txt");
        cleanup(&src);
        cleanup(&dst);

        write_text(&src, "copy me").unwrap();
        let bytes_copied = copy_file(&src, &dst).unwrap();
        assert_eq!(bytes_copied, 7);

        let content = read_text(&dst).unwrap();
        assert_eq!(content, "copy me");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn test_rename_file() {
        let src = temp_path("rename_src.txt");
        let dst = temp_path("rename_dst.txt");
        cleanup(&src);
        cleanup(&dst);

        write_text(&src, "rename me").unwrap();
        rename_file(&src, &dst).unwrap();

        assert!(!src.exists());
        let content = read_text(&dst).unwrap();
        assert_eq!(content, "rename me");

        cleanup(&dst);
    }

    #[test]
    fn test_remove_file() {
        let path = temp_path("remove_me.txt");
        cleanup(&path);

        write_text(&path, "delete me").unwrap();
        assert!(path.exists());

        remove_file(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_atomic_write() {
        let path = temp_path("atomic_write_ops.txt");
        cleanup(&path);

        atomic_write(&path, |w| w.write_all(b"atomic content")).unwrap();

        let content = read_text(&path).unwrap();
        assert_eq!(content, "atomic content");

        cleanup(&path);
    }

    #[test]
    fn test_read_nonexistent() {
        let result = read_text("/nonexistent/path/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }
}
