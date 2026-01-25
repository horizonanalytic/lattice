//! Memory-mapped file support.
//!
//! Memory-mapped files allow accessing file contents directly through memory addresses,
//! providing efficient access to large files without loading them entirely into RAM.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::file::{MappedFile, MappedFileMut, map_file};
//!
//! // Read-only mapping
//! let mapped = MappedFile::open("large_file.bin")?;
//! let first_byte = mapped[0];
//! let slice = &mapped[100..200];
//!
//! // Mutable mapping
//! let mut mapped = MappedFileMut::open("data.bin")?;
//! mapped[0] = 0xFF;
//! mapped.flush()?;
//!
//! // Create new file with specified size
//! let mut mapped = MappedFileMut::create("new_file.bin", 1024)?;
//! mapped.as_mut_slice().fill(0);
//! mapped.flush()?;
//! ```

use std::fs::{self, OpenOptions};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapMut};

use super::error::{FileError, FileResult};

/// Options for configuring memory-mapped file creation.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::{MappedFile, MmapOptions};
///
/// // Map only a portion of the file
/// let options = MmapOptions::new()
///     .offset(1024)
///     .len(4096);
/// let mapped = MappedFile::with_options("large.bin", &options)?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct MmapOptions {
    /// Byte offset into the file to start the mapping.
    offset: u64,
    /// Length of the mapping in bytes. If None, maps to end of file.
    len: Option<usize>,
    /// If true, read-ahead the file contents (MAP_POPULATE on Linux).
    populate: bool,
}

impl MmapOptions {
    /// Creates a new `MmapOptions` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the byte offset into the file to start the mapping.
    ///
    /// Default is 0 (start of file).
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the length of the mapping in bytes.
    ///
    /// If not set, the mapping extends to the end of the file.
    pub fn len(mut self, len: usize) -> Self {
        self.len = Some(len);
        self
    }

    /// Enables read-ahead for the mapping.
    ///
    /// When enabled, the OS will read-ahead the file contents into memory,
    /// reducing page faults during sequential access. This corresponds to
    /// `MAP_POPULATE` on Linux and has no effect on Windows.
    pub fn populate(mut self, populate: bool) -> Self {
        self.populate = populate;
        self
    }

    /// Returns the configured offset.
    pub fn get_offset(&self) -> u64 {
        self.offset
    }

    /// Returns the configured length, if any.
    pub fn get_len(&self) -> Option<usize> {
        self.len
    }

    /// Returns whether populate is enabled.
    pub fn get_populate(&self) -> bool {
        self.populate
    }
}

/// A read-only memory-mapped file.
///
/// This provides zero-copy access to file contents through memory mapping.
/// The file contents can be accessed as a byte slice.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::MappedFile;
///
/// let mapped = MappedFile::open("data.bin")?;
/// println!("File size: {} bytes", mapped.len());
/// println!("First byte: {}", mapped[0]);
///
/// // Iterate over contents
/// for byte in mapped.iter() {
///     // process byte
/// }
/// ```
pub struct MappedFile {
    mmap: Mmap,
    path: PathBuf,
}

impl MappedFile {
    /// Opens a file and creates a read-only memory mapping.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, cannot be opened,
    /// or cannot be memory-mapped.
    pub fn open(path: impl AsRef<Path>) -> FileResult<Self> {
        Self::with_options(path, &MmapOptions::default())
    }

    /// Opens a file and creates a read-only memory mapping with custom options.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, cannot be opened,
    /// or cannot be memory-mapped.
    pub fn with_options(path: impl AsRef<Path>, options: &MmapOptions) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = fs::File::open(&path).map_err(|e| FileError::from_io(e, &path))?;

        let mut mmap_opts = memmap2::MmapOptions::new();
        mmap_opts.offset(options.offset);

        if let Some(len) = options.len {
            mmap_opts.len(len);
        }

        if options.populate {
            mmap_opts.populate();
        }

        // SAFETY: Memory mapping is inherently unsafe because the underlying file
        // could be modified by another process while mapped. However, this is
        // acceptable for read-only mappings where we document that the file should
        // not be modified externally during the mapping's lifetime.
        let mmap = unsafe { mmap_opts.map(&file) }.map_err(|e| FileError::from_io(e, &path))?;

        Ok(Self { mmap, path })
    }

    /// Returns the path to the mapped file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the length of the mapping in bytes.
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Returns true if the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Returns the mapped contents as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }
}

impl Deref for MappedFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mmap
    }
}

impl AsRef<[u8]> for MappedFile {
    fn as_ref(&self) -> &[u8] {
        &self.mmap
    }
}

impl std::fmt::Debug for MappedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedFile")
            .field("path", &self.path)
            .field("len", &self.mmap.len())
            .finish()
    }
}

/// A mutable memory-mapped file.
///
/// This provides read-write access to file contents through memory mapping.
/// Changes to the mapping are automatically written back to the file.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::MappedFileMut;
///
/// // Open existing file for modification
/// let mut mapped = MappedFileMut::open("data.bin")?;
/// mapped[0] = 0xFF;
/// mapped.flush()?;
///
/// // Create new file with specified size
/// let mut mapped = MappedFileMut::create("new.bin", 1024)?;
/// mapped.as_mut_slice().fill(0x00);
/// mapped.flush()?;
/// ```
pub struct MappedFileMut {
    mmap: MmapMut,
    path: PathBuf,
}

impl MappedFileMut {
    /// Opens an existing file and creates a mutable memory mapping.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, cannot be opened for writing,
    /// or cannot be memory-mapped.
    pub fn open(path: impl AsRef<Path>) -> FileResult<Self> {
        Self::with_options(path, &MmapOptions::default())
    }

    /// Creates a new file with the specified length and maps it.
    ///
    /// If the file already exists, it will be truncated to the specified length.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or mapped.
    pub fn create(path: impl AsRef<Path>, len: u64) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| FileError::from_io(e, &path))?;

        file.set_len(len).map_err(|e| FileError::from_io(e, &path))?;

        // SAFETY: We just created the file and have exclusive access.
        let mmap = unsafe { MmapMut::map_mut(&file) }.map_err(|e| FileError::from_io(e, &path))?;

        Ok(Self { mmap, path })
    }

    /// Opens an existing file and creates a mutable memory mapping with custom options.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist, cannot be opened for writing,
    /// or cannot be memory-mapped.
    pub fn with_options(path: impl AsRef<Path>, options: &MmapOptions) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| FileError::from_io(e, &path))?;

        let mut mmap_opts = memmap2::MmapOptions::new();
        mmap_opts.offset(options.offset);

        if let Some(len) = options.len {
            mmap_opts.len(len);
        }

        if options.populate {
            mmap_opts.populate();
        }

        // SAFETY: Memory mapping is inherently unsafe because multiple processes
        // could map the same file. Users should ensure exclusive access when using
        // mutable mappings.
        let mmap =
            unsafe { mmap_opts.map_mut(&file) }.map_err(|e| FileError::from_io(e, &path))?;

        Ok(Self { mmap, path })
    }

    /// Returns the path to the mapped file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the length of the mapping in bytes.
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Returns true if the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Returns the mapped contents as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }

    /// Returns the mapped contents as a mutable byte slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.mmap
    }

    /// Flushes outstanding memory map modifications to disk.
    ///
    /// This is a synchronous operation that waits until the flush completes.
    /// When this returns successfully, all outstanding modifications have been
    /// written to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush fails.
    pub fn flush(&self) -> FileResult<()> {
        self.mmap.flush().map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Asynchronously flushes outstanding memory map modifications to disk.
    ///
    /// This returns immediately after initiating the flush. Use this when you
    /// want to ensure data will eventually be written but don't need to wait.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush cannot be initiated.
    pub fn flush_async(&self) -> FileResult<()> {
        self.mmap
            .flush_async()
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Flushes a specific range of the memory map to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush fails or the range is invalid.
    pub fn flush_range(&self, offset: usize, len: usize) -> FileResult<()> {
        self.mmap
            .flush_range(offset, len)
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Makes the memory map read-only.
    ///
    /// This consumes the mutable mapping and returns a read-only mapping.
    pub fn make_read_only(self) -> FileResult<MappedFile> {
        let mmap = self
            .mmap
            .make_read_only()
            .map_err(|e| FileError::from_io(e, &self.path))?;
        Ok(MappedFile {
            mmap,
            path: self.path,
        })
    }
}

impl Deref for MappedFileMut {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mmap
    }
}

impl DerefMut for MappedFileMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mmap
    }
}

impl AsRef<[u8]> for MappedFileMut {
    fn as_ref(&self) -> &[u8] {
        &self.mmap
    }
}

impl AsMut<[u8]> for MappedFileMut {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
}

impl std::fmt::Debug for MappedFileMut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedFileMut")
            .field("path", &self.path)
            .field("len", &self.mmap.len())
            .finish()
    }
}

/// Opens a file and creates a read-only memory mapping.
///
/// This is a convenience function equivalent to `MappedFile::open()`.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::map_file;
///
/// let mapped = map_file("data.bin")?;
/// println!("First 10 bytes: {:?}", &mapped[..10]);
/// ```
pub fn map_file(path: impl AsRef<Path>) -> FileResult<MappedFile> {
    MappedFile::open(path)
}

/// Opens a file and creates a mutable memory mapping.
///
/// This is a convenience function equivalent to `MappedFileMut::open()`.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::map_file_mut;
///
/// let mut mapped = map_file_mut("data.bin")?;
/// mapped[0] = 0xFF;
/// mapped.flush()?;
/// ```
pub fn map_file_mut(path: impl AsRef<Path>) -> FileResult<MappedFileMut> {
    MappedFileMut::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file(name: &str, content: &[u8]) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("horizon_mmap_test_{}", name));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        file.sync_all().unwrap();
        path
    }

    fn cleanup(path: &Path) {
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_mapped_file_open() {
        let content = b"Hello, memory-mapped world!";
        let path = create_test_file("read.bin", content);

        let mapped = MappedFile::open(&path).unwrap();
        assert_eq!(mapped.path(), path);
        assert_eq!(mapped.len(), content.len());
        assert!(!mapped.is_empty());
        assert_eq!(mapped.as_slice(), content);

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_deref() {
        let content = b"Test content";
        let path = create_test_file("deref.bin", content);

        let mapped = MappedFile::open(&path).unwrap();
        // Test Deref to [u8]
        assert_eq!(&mapped[..4], b"Test");
        assert_eq!(mapped[0], b'T');

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_with_offset() {
        let content = b"Hello, World!";
        let path = create_test_file("offset.bin", content);

        let options = MmapOptions::new().offset(7);
        let mapped = MappedFile::with_options(&path, &options).unwrap();
        assert_eq!(mapped.as_slice(), b"World!");

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_with_len() {
        let content = b"Hello, World!";
        let path = create_test_file("len.bin", content);

        let options = MmapOptions::new().len(5);
        let mapped = MappedFile::with_options(&path, &options).unwrap();
        assert_eq!(mapped.len(), 5);
        assert_eq!(mapped.as_slice(), b"Hello");

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_mut_open() {
        let content = b"Original content";
        let path = create_test_file("mut_open.bin", content);

        let mut mapped = MappedFileMut::open(&path).unwrap();
        assert_eq!(mapped.len(), content.len());

        // Modify the first byte
        mapped[0] = b'X';
        mapped.flush().unwrap();

        // Verify the change was written
        let data = fs::read(&path).unwrap();
        assert_eq!(data[0], b'X');

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_mut_create() {
        let path = std::env::temp_dir().join("horizon_mmap_test_create.bin");
        cleanup(&path);

        let mut mapped = MappedFileMut::create(&path, 100).unwrap();
        assert_eq!(mapped.len(), 100);

        // Fill with pattern
        mapped.as_mut_slice().fill(0xAB);
        mapped.flush().unwrap();

        // Verify file size and content
        let data = fs::read(&path).unwrap();
        assert_eq!(data.len(), 100);
        assert!(data.iter().all(|&b| b == 0xAB));

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_mut_flush_async() {
        let content = b"Async flush test";
        let path = create_test_file("flush_async.bin", content);

        let mut mapped = MappedFileMut::open(&path).unwrap();
        mapped[0] = b'Z';
        mapped.flush_async().unwrap();

        cleanup(&path);
    }

    #[test]
    fn test_mapped_file_not_found() {
        let result = MappedFile::open("/nonexistent/path/file.bin");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }

    #[test]
    fn test_map_file_convenience() {
        let content = b"Convenience function test";
        let path = create_test_file("convenience.bin", content);

        let mapped = map_file(&path).unwrap();
        assert_eq!(mapped.as_slice(), content);

        cleanup(&path);
    }

    #[test]
    fn test_map_file_mut_convenience() {
        let content = b"Mutable convenience test";
        let path = create_test_file("mut_convenience.bin", content);

        let mut mapped = map_file_mut(&path).unwrap();
        mapped[0] = b'N';
        mapped.flush().unwrap();

        let data = fs::read(&path).unwrap();
        assert_eq!(data[0], b'N');

        cleanup(&path);
    }

    #[test]
    fn test_make_read_only() {
        let content = b"Make read only test";
        let path = create_test_file("make_ro.bin", content);

        let mapped_mut = MappedFileMut::open(&path).unwrap();
        let mapped = mapped_mut.make_read_only().unwrap();
        assert_eq!(mapped.as_slice(), content);

        cleanup(&path);
    }

    #[test]
    fn test_mmap_options_builder() {
        let options = MmapOptions::new().offset(100).len(200).populate(true);

        assert_eq!(options.get_offset(), 100);
        assert_eq!(options.get_len(), Some(200));
        assert!(options.get_populate());
    }
}
