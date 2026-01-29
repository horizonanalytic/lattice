//! File writing operations.

use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::error::{FileError, FileResult};

/// A file handle for writing operations.
///
/// This wraps a standard library file handle with additional convenience methods
/// for common write patterns.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::FileWriter;
///
/// // Create and write to a file
/// let mut file = FileWriter::create("output.txt")?;
/// file.write_all(b"Hello, World!")?;
///
/// // Append to a file
/// let mut file = FileWriter::append("log.txt")?;
/// file.write_line("New log entry")?;
/// ```
pub struct FileWriter {
    /// The underlying file handle, wrapped in a buffer.
    inner: BufWriter<fs::File>,
    /// The path to the file (for error messages).
    path: PathBuf,
}

impl FileWriter {
    /// Creates a new file for writing, truncating if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created.
    pub fn create(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = fs::File::create(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self {
            inner: BufWriter::new(file),
            path,
        })
    }

    /// Opens or creates a file for appending.
    ///
    /// Writes will be added to the end of the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or created.
    pub fn append(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self {
            inner: BufWriter::new(file),
            path,
        })
    }

    /// Creates a new file, failing if it already exists.
    ///
    /// This is useful when you want to ensure you're not overwriting an existing file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file already exists or cannot be created.
    pub fn create_new(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self {
            inner: BufWriter::new(file),
            path,
        })
    }

    /// Returns the path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes all bytes to the file.
    ///
    /// # Errors
    ///
    /// Returns an error if not all bytes could be written.
    pub fn write_all(&mut self, buf: &[u8]) -> FileResult<()> {
        self.inner
            .write_all(buf)
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Writes a string to the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the string could not be written.
    pub fn write_str(&mut self, s: &str) -> FileResult<()> {
        self.write_all(s.as_bytes())
    }

    /// Writes a string followed by a newline.
    ///
    /// # Errors
    ///
    /// Returns an error if the line could not be written.
    pub fn write_line(&mut self, s: &str) -> FileResult<()> {
        self.write_str(s)?;
        self.write_all(b"\n")
    }

    /// Flushes buffered data to disk.
    ///
    /// This is called automatically when the writer is dropped, but you can
    /// call it explicitly to ensure data is written at a specific point.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush fails.
    pub fn flush(&mut self) -> FileResult<()> {
        self.inner
            .flush()
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Syncs all data and metadata to disk.
    ///
    /// This is more thorough than `flush()` and ensures the data is durably
    /// stored on the underlying storage device.
    ///
    /// # Errors
    ///
    /// Returns an error if the sync fails.
    pub fn sync_all(&mut self) -> FileResult<()> {
        self.inner
            .flush()
            .map_err(|e| FileError::from_io(e, &self.path))?;
        self.inner
            .get_ref()
            .sync_all()
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Syncs data (but not necessarily metadata) to disk.
    ///
    /// This is faster than `sync_all()` but may not update file metadata like
    /// modification time.
    ///
    /// # Errors
    ///
    /// Returns an error if the sync fails.
    pub fn sync_data(&mut self) -> FileResult<()> {
        self.inner
            .flush()
            .map_err(|e| FileError::from_io(e, &self.path))?;
        self.inner
            .get_ref()
            .sync_data()
            .map_err(|e| FileError::from_io(e, &self.path))
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl std::fmt::Debug for FileWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileWriter")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

// ============================================================================
// Atomic Writer
// ============================================================================

/// Performs atomic file writes using a temporary file and rename.
///
/// This is useful for configuration files and other data where partial writes
/// could cause corruption. The write either succeeds completely or leaves the
/// original file untouched.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::AtomicWriter;
///
/// AtomicWriter::write("config.json", |f| {
///     f.write_all(b"{\"key\": \"value\"}")
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
/// If any step fails, the original file is left unchanged.
pub struct AtomicWriter {
    /// The target path for the final file.
    target_path: PathBuf,
    /// The temporary file path.
    temp_path: PathBuf,
    /// The writer for the temporary file (Option to allow taking ownership).
    writer: Option<BufWriter<fs::File>>,
    /// Whether the write has been committed.
    committed: bool,
}

impl AtomicWriter {
    /// Creates a new atomic writer for the given target path.
    ///
    /// This creates a temporary file in the same directory as the target.
    ///
    /// # Errors
    ///
    /// Returns an error if the temporary file cannot be created.
    pub fn new(path: impl AsRef<Path>) -> FileResult<Self> {
        let target_path = path.as_ref().to_path_buf();

        // Create temp file in same directory to ensure same filesystem
        let parent = target_path.parent().unwrap_or(Path::new("."));
        let file_name = target_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        // Generate unique temp filename
        let temp_name = format!(".{}.tmp.{}", file_name, std::process::id());
        let temp_path = parent.join(&temp_name);

        let file = fs::File::create(&temp_path).map_err(|e| FileError::from_io(e, &target_path))?;

        Ok(Self {
            target_path,
            temp_path,
            writer: Some(BufWriter::new(file)),
            committed: false,
        })
    }

    /// Performs an atomic write with a closure.
    ///
    /// This is the recommended way to use `AtomicWriter` for simple cases.
    ///
    /// # Example
    ///
    /// ```ignore
    /// AtomicWriter::write("config.json", |w| {
    ///     w.write_all(b"{\"version\": 1}")
    /// })?;
    /// ```
    pub fn write<F>(path: impl AsRef<Path>, f: F) -> FileResult<()>
    where
        F: FnOnce(&mut AtomicWriter) -> FileResult<()>,
    {
        let mut writer = Self::new(path)?;
        f(&mut writer)?;
        writer.commit()
    }

    /// Returns the target path.
    pub fn target_path(&self) -> &Path {
        &self.target_path
    }

    /// Returns the temporary file path.
    pub fn temp_path(&self) -> &Path {
        &self.temp_path
    }

    /// Writes all bytes to the temporary file.
    pub fn write_all(&mut self, buf: &[u8]) -> FileResult<()> {
        self.writer
            .as_mut()
            .expect("AtomicWriter already consumed")
            .write_all(buf)
            .map_err(|e| FileError::from_io(e, &self.target_path))
    }

    /// Writes a string to the temporary file.
    pub fn write_str(&mut self, s: &str) -> FileResult<()> {
        self.write_all(s.as_bytes())
    }

    /// Commits the write, replacing the target file atomically.
    ///
    /// This flushes and syncs the temporary file, then renames it to the
    /// target path. If successful, any previous file at the target path
    /// is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush, sync, or rename fails. If an error
    /// occurs, the temporary file is removed and the original file (if any)
    /// is left unchanged.
    pub fn commit(mut self) -> FileResult<()> {
        // Take ownership of the writer
        let mut writer = self.writer.take().expect("AtomicWriter already consumed");

        // Flush and sync
        writer
            .flush()
            .map_err(|e| FileError::from_io(e, &self.target_path))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|e| FileError::from_io(e, &self.target_path))?;

        // Drop the writer to close the file handle before renaming
        drop(writer);

        // Atomic rename
        fs::rename(&self.temp_path, &self.target_path)
            .map_err(|e| FileError::from_io(e, &self.target_path))?;

        self.committed = true;
        Ok(())
    }

    /// Aborts the write, removing the temporary file.
    ///
    /// This is called automatically if the writer is dropped without committing.
    pub fn abort(mut self) {
        self.committed = true; // Prevent double cleanup in Drop
        let _ = fs::remove_file(&self.temp_path);
    }
}

impl Write for AtomicWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer
            .as_mut()
            .ok_or_else(|| io::Error::other("AtomicWriter already consumed"))?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer
            .as_mut()
            .ok_or_else(|| io::Error::other("AtomicWriter already consumed"))?
            .flush()
    }
}

impl Drop for AtomicWriter {
    fn drop(&mut self) {
        if !self.committed {
            // Clean up temp file on failure
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

impl std::fmt::Debug for AtomicWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AtomicWriter")
            .field("target_path", &self.target_path)
            .field("temp_path", &self.temp_path)
            .field("committed", &self.committed)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("horizon_test_{}", name))
    }

    fn cleanup(path: &Path) {
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_file_writer_create() {
        let path = temp_path("writer_create.txt");
        cleanup(&path);

        {
            let mut writer = FileWriter::create(&path).unwrap();
            writer.write_all(b"Hello").unwrap();
            writer.write_str(", World!").unwrap();
        }

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");

        cleanup(&path);
    }

    #[test]
    fn test_file_writer_append() {
        let path = temp_path("writer_append.txt");
        cleanup(&path);

        // Create initial content
        fs::write(&path, "Line 1\n").unwrap();

        // Append more content
        {
            let mut writer = FileWriter::append(&path).unwrap();
            writer.write_line("Line 2").unwrap();
        }

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Line 1\nLine 2\n");

        cleanup(&path);
    }

    #[test]
    fn test_file_writer_create_new() {
        let path = temp_path("writer_create_new.txt");
        cleanup(&path);

        // Should succeed when file doesn't exist
        {
            let mut writer = FileWriter::create_new(&path).unwrap();
            writer.write_all(b"content").unwrap();
        }

        // Should fail when file exists
        let result = FileWriter::create_new(&path);
        assert!(result.is_err());

        cleanup(&path);
    }

    #[test]
    fn test_atomic_writer() {
        let path = temp_path("atomic_write.txt");
        cleanup(&path);

        // Write atomically
        AtomicWriter::write(&path, |w| w.write_all(b"atomic content")).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "atomic content");

        cleanup(&path);
    }

    #[test]
    fn test_atomic_writer_abort() {
        let path = temp_path("atomic_abort.txt");
        cleanup(&path);

        // Create original content
        fs::write(&path, "original").unwrap();

        // Start atomic write but abort
        {
            let mut writer = AtomicWriter::new(&path).unwrap();
            writer.write_all(b"new content").unwrap();
            // Drop without commit - should abort and clean up
        }

        // Original content should be unchanged
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "original");

        cleanup(&path);
    }

    #[test]
    fn test_atomic_writer_replaces_existing() {
        let path = temp_path("atomic_replace.txt");
        cleanup(&path);

        // Create original content
        fs::write(&path, "original").unwrap();

        // Write atomically (should replace)
        AtomicWriter::write(&path, |w| w.write_all(b"replaced")).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "replaced");

        cleanup(&path);
    }

    #[test]
    fn test_write_line() {
        let path = temp_path("write_line.txt");
        cleanup(&path);

        {
            let mut writer = FileWriter::create(&path).unwrap();
            writer.write_line("first").unwrap();
            writer.write_line("second").unwrap();
        }

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "first\nsecond\n");

        cleanup(&path);
    }
}
