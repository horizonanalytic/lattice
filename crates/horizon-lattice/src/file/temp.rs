//! Temporary files and directories with automatic cleanup.
//!
//! This module provides RAII-based temporary files and directories that are
//! automatically deleted when they go out of scope.
//!
//! # Temporary Files
//!
//! ```ignore
//! use horizon_lattice::file::TempFile;
//!
//! // Create a temp file with default settings
//! let temp = TempFile::new()?;
//! println!("Temp file at: {}", temp.path().display());
//!
//! // Write to it
//! temp.write_all(b"temporary data")?;
//!
//! // File is automatically deleted when `temp` goes out of scope
//! ```
//!
//! # Temporary Directories
//!
//! ```ignore
//! use horizon_lattice::file::TempDirectory;
//!
//! // Create a temp directory
//! let temp_dir = TempDirectory::new()?;
//!
//! // Create files inside it
//! let file_path = temp_dir.path().join("test.txt");
//! std::fs::write(&file_path, "content")?;
//!
//! // Directory and all contents are deleted when `temp_dir` goes out of scope
//! ```
//!
//! # Builder Pattern
//!
//! ```ignore
//! use horizon_lattice::file::{TempFile, TempDirectory};
//!
//! // Custom temp file with prefix, suffix, and directory
//! let temp = TempFile::builder()
//!     .prefix("myapp_")
//!     .suffix(".tmp")
//!     .in_dir("/custom/temp")
//!     .create()?;
//!
//! // Custom temp directory
//! let temp_dir = TempDirectory::builder()
//!     .prefix("myapp_cache_")
//!     .in_dir("/custom/temp")
//!     .create()?;
//! ```

use std::fs;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

use super::error::{FileError, FileResult};

// ============================================================================
// TempFile
// ============================================================================

/// A temporary file that is automatically deleted when dropped.
///
/// The file is created with a unique name in the system's temporary directory
/// (or a custom directory if specified). When the `TempFile` is dropped, the
/// file is automatically deleted.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::TempFile;
///
/// let mut temp = TempFile::new()?;
/// temp.write_all(b"temporary data")?;
///
/// // Read it back
/// temp.rewind()?;
/// let mut contents = String::new();
/// temp.read_to_string(&mut contents)?;
/// assert_eq!(contents, "temporary data");
///
/// // File is deleted when `temp` goes out of scope
/// ```
#[derive(Debug)]
pub struct TempFile {
    inner: tempfile::NamedTempFile,
}

impl TempFile {
    /// Creates a new temporary file in the system's default temp directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp file cannot be created.
    pub fn new() -> FileResult<Self> {
        let inner =
            tempfile::NamedTempFile::new().map_err(|e| FileError::from(e))?;
        Ok(Self { inner })
    }

    /// Creates a new temporary file in the specified directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp file cannot be created.
    pub fn new_in(dir: impl AsRef<Path>) -> FileResult<Self> {
        let inner = tempfile::NamedTempFile::new_in(dir.as_ref())
            .map_err(|e| FileError::from_io(e, dir.as_ref()))?;
        Ok(Self { inner })
    }

    /// Returns a builder for creating a temp file with custom options.
    pub fn builder() -> TempFileBuilder {
        TempFileBuilder::new()
    }

    /// Returns the path to the temporary file.
    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    /// Keeps the file on disk instead of deleting it when dropped.
    ///
    /// Returns the path to the persisted file.
    ///
    /// # Errors
    ///
    /// This operation cannot fail, but if you want to move the file to a
    /// specific location, you should use `persist()` instead.
    pub fn keep(self) -> PathBuf {
        let (_, path) = self.inner.keep().expect("tempfile keep should not fail");
        path
    }

    /// Persists the file to a new path, consuming the `TempFile`.
    ///
    /// This is more efficient than copying the file as it uses rename when
    /// possible.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be moved to the target path.
    pub fn persist(self, path: impl AsRef<Path>) -> FileResult<()> {
        self.inner
            .persist(path.as_ref())
            .map_err(|e| FileError::from_io(e.error, path.as_ref()))?;
        Ok(())
    }

    /// Rewinds the file to the beginning.
    ///
    /// # Errors
    ///
    /// Returns an error if seeking fails.
    pub fn rewind(&mut self) -> FileResult<()> {
        self.inner
            .rewind()
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Syncs all data and metadata to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the sync fails.
    pub fn sync_all(&self) -> FileResult<()> {
        self.inner
            .as_file()
            .sync_all()
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns the size of the temp file in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata cannot be read.
    pub fn len(&self) -> FileResult<u64> {
        self.inner
            .as_file()
            .metadata()
            .map(|m| m.len())
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns true if the temp file is empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata cannot be read.
    pub fn is_empty(&self) -> FileResult<bool> {
        Ok(self.len()? == 0)
    }
}

impl Read for TempFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for TempFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for TempFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

// ============================================================================
// TempFileBuilder
// ============================================================================

/// Builder for creating temporary files with custom options.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::TempFile;
///
/// let temp = TempFile::builder()
///     .prefix("myapp_")
///     .suffix(".txt")
///     .create()?;
/// ```
#[derive(Debug, Default)]
pub struct TempFileBuilder {
    prefix: Option<String>,
    suffix: Option<String>,
    dir: Option<PathBuf>,
}

impl TempFileBuilder {
    /// Creates a new temp file builder with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a prefix for the temp file name.
    ///
    /// The final filename will be: `{prefix}{random}{suffix}`
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets a suffix for the temp file name.
    ///
    /// The final filename will be: `{prefix}{random}{suffix}`
    ///
    /// This is useful for setting file extensions.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Sets the directory where the temp file will be created.
    ///
    /// If not set, the system's default temp directory is used.
    pub fn in_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Creates the temporary file with the configured options.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp file cannot be created.
    pub fn create(self) -> FileResult<TempFile> {
        let mut builder = tempfile::Builder::new();

        if let Some(prefix) = &self.prefix {
            builder.prefix(prefix);
        }
        if let Some(suffix) = &self.suffix {
            builder.suffix(suffix);
        }

        let inner = if let Some(dir) = &self.dir {
            builder
                .tempfile_in(dir)
                .map_err(|e| FileError::from_io(e, dir))?
        } else {
            builder
                .tempfile()
                .map_err(|e| FileError::from(e))?
        };

        Ok(TempFile { inner })
    }
}

// ============================================================================
// TempDirectory
// ============================================================================

/// A temporary directory that is automatically deleted when dropped.
///
/// The directory is created with a unique name in the system's temporary
/// directory (or a custom directory if specified). When the `TempDirectory`
/// is dropped, the directory and all its contents are recursively deleted.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::TempDirectory;
///
/// let temp_dir = TempDirectory::new()?;
///
/// // Create files inside the temp directory
/// let file_path = temp_dir.path().join("test.txt");
/// std::fs::write(&file_path, "content")?;
///
/// // Directory and all contents are deleted when `temp_dir` goes out of scope
/// ```
#[derive(Debug)]
pub struct TempDirectory {
    inner: tempfile::TempDir,
}

impl TempDirectory {
    /// Creates a new temporary directory in the system's default temp directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp directory cannot be created.
    pub fn new() -> FileResult<Self> {
        let inner = tempfile::TempDir::new().map_err(|e| FileError::from(e))?;
        Ok(Self { inner })
    }

    /// Creates a new temporary directory in the specified parent directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp directory cannot be created.
    pub fn new_in(dir: impl AsRef<Path>) -> FileResult<Self> {
        let inner = tempfile::TempDir::new_in(dir.as_ref())
            .map_err(|e| FileError::from_io(e, dir.as_ref()))?;
        Ok(Self { inner })
    }

    /// Returns a builder for creating a temp directory with custom options.
    pub fn builder() -> TempDirectoryBuilder {
        TempDirectoryBuilder::new()
    }

    /// Returns the path to the temporary directory.
    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    /// Keeps the directory on disk instead of deleting it when dropped.
    ///
    /// Returns the path to the persisted directory.
    pub fn keep(self) -> PathBuf {
        self.inner.keep()
    }

    /// Creates a file inside the temp directory.
    ///
    /// This is a convenience method that creates a file at
    /// `{temp_dir}/{name}`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created.
    pub fn create_file(&self, name: impl AsRef<Path>) -> FileResult<fs::File> {
        let path = self.inner.path().join(name.as_ref());
        fs::File::create(&path).map_err(|e| FileError::from_io(e, &path))
    }

    /// Creates a subdirectory inside the temp directory.
    ///
    /// This is a convenience method that creates a directory at
    /// `{temp_dir}/{name}`.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub fn create_dir(&self, name: impl AsRef<Path>) -> FileResult<PathBuf> {
        let path = self.inner.path().join(name.as_ref());
        fs::create_dir(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(path)
    }

    /// Creates a nested subdirectory structure inside the temp directory.
    ///
    /// This is a convenience method that creates directories at
    /// `{temp_dir}/{path}`, including any necessary parent directories.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn create_dir_all(&self, name: impl AsRef<Path>) -> FileResult<PathBuf> {
        let path = self.inner.path().join(name.as_ref());
        fs::create_dir_all(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(path)
    }

    /// Returns the path to a file or subdirectory inside the temp directory.
    ///
    /// This doesn't create anything, just returns the path.
    pub fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.inner.path().join(name.as_ref())
    }
}

// ============================================================================
// TempDirectoryBuilder
// ============================================================================

/// Builder for creating temporary directories with custom options.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::TempDirectory;
///
/// let temp_dir = TempDirectory::builder()
///     .prefix("myapp_cache_")
///     .create()?;
/// ```
#[derive(Debug, Default)]
pub struct TempDirectoryBuilder {
    prefix: Option<String>,
    suffix: Option<String>,
    dir: Option<PathBuf>,
}

impl TempDirectoryBuilder {
    /// Creates a new temp directory builder with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a prefix for the temp directory name.
    ///
    /// The final directory name will be: `{prefix}{random}{suffix}`
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets a suffix for the temp directory name.
    ///
    /// The final directory name will be: `{prefix}{random}{suffix}`
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Sets the parent directory where the temp directory will be created.
    ///
    /// If not set, the system's default temp directory is used.
    pub fn in_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Creates the temporary directory with the configured options.
    ///
    /// # Errors
    ///
    /// Returns an error if the temp directory cannot be created.
    pub fn create(self) -> FileResult<TempDirectory> {
        let mut builder = tempfile::Builder::new();

        if let Some(prefix) = &self.prefix {
            builder.prefix(prefix);
        }
        if let Some(suffix) = &self.suffix {
            builder.suffix(suffix);
        }

        let inner = if let Some(dir) = &self.dir {
            builder
                .tempdir_in(dir)
                .map_err(|e| FileError::from_io(e, dir))?
        } else {
            builder
                .tempdir()
                .map_err(|e| FileError::from(e))?
        };

        Ok(TempDirectory { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_file_create_and_auto_delete() {
        let path;
        {
            let temp = TempFile::new().unwrap();
            path = temp.path().to_path_buf();
            assert!(path.exists());
        }
        // File should be deleted after drop
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_file_write_and_read() {
        let mut temp = TempFile::new().unwrap();

        // Write data
        temp.write_all(b"test data").unwrap();
        temp.flush().unwrap();

        // Read it back
        temp.rewind().unwrap();
        let mut contents = String::new();
        temp.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "test data");
    }

    #[test]
    fn test_temp_file_keep() {
        let path = {
            let temp = TempFile::new().unwrap();
            temp.keep()
        };

        // File should still exist
        assert!(path.exists());

        // Clean up manually
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_temp_file_persist() {
        let target = std::env::temp_dir().join("horizon_test_persist.txt");
        let _ = fs::remove_file(&target);

        {
            let mut temp = TempFile::new().unwrap();
            temp.write_all(b"persisted data").unwrap();
            temp.persist(&target).unwrap();
        }

        // Target should exist with the data
        assert!(target.exists());
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "persisted data");

        // Clean up
        fs::remove_file(&target).unwrap();
    }

    #[test]
    fn test_temp_file_builder() {
        let temp = TempFile::builder()
            .prefix("horizon_")
            .suffix(".txt")
            .create()
            .unwrap();

        let name = temp.path().file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("horizon_"));
        assert!(name.ends_with(".txt"));
    }

    #[test]
    fn test_temp_file_len() {
        let mut temp = TempFile::new().unwrap();
        assert!(temp.is_empty().unwrap());

        temp.write_all(b"12345").unwrap();
        temp.flush().unwrap();

        assert_eq!(temp.len().unwrap(), 5);
        assert!(!temp.is_empty().unwrap());
    }

    #[test]
    fn test_temp_directory_create_and_auto_delete() {
        let path;
        {
            let temp_dir = TempDirectory::new().unwrap();
            path = temp_dir.path().to_path_buf();
            assert!(path.exists());
            assert!(path.is_dir());
        }
        // Directory should be deleted after drop
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_directory_with_contents() {
        let path;
        let file_path;
        {
            let temp_dir = TempDirectory::new().unwrap();
            path = temp_dir.path().to_path_buf();

            // Create a file inside
            file_path = temp_dir.join("test.txt");
            fs::write(&file_path, "content").unwrap();

            assert!(file_path.exists());
        }
        // Both directory and contents should be deleted
        assert!(!path.exists());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_temp_directory_keep() {
        let path = {
            let temp_dir = TempDirectory::new().unwrap();
            temp_dir.keep()
        };

        // Directory should still exist
        assert!(path.exists());

        // Clean up manually
        fs::remove_dir(&path).unwrap();
    }

    #[test]
    fn test_temp_directory_builder() {
        let temp_dir = TempDirectory::builder()
            .prefix("horizon_test_")
            .create()
            .unwrap();

        let name = temp_dir.path().file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("horizon_test_"));
    }

    #[test]
    fn test_temp_directory_create_file() {
        let temp_dir = TempDirectory::new().unwrap();

        let mut file = temp_dir.create_file("test.txt").unwrap();
        file.write_all(b"content").unwrap();

        let file_path = temp_dir.join("test.txt");
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "content");
    }

    #[test]
    fn test_temp_directory_create_dir() {
        let temp_dir = TempDirectory::new().unwrap();

        let sub_path = temp_dir.create_dir("subdir").unwrap();
        assert!(sub_path.exists());
        assert!(sub_path.is_dir());
    }

    #[test]
    fn test_temp_directory_create_dir_all() {
        let temp_dir = TempDirectory::new().unwrap();

        let nested_path = temp_dir.create_dir_all("a/b/c").unwrap();
        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
    }

    #[test]
    fn test_temp_file_in_custom_dir() {
        let parent = TempDirectory::new().unwrap();
        let temp = TempFile::new_in(parent.path()).unwrap();

        assert!(temp.path().starts_with(parent.path()));
    }

    #[test]
    fn test_temp_directory_in_custom_dir() {
        let parent = TempDirectory::new().unwrap();
        let temp_dir = TempDirectory::new_in(parent.path()).unwrap();

        assert!(temp_dir.path().starts_with(parent.path()));
    }
}
