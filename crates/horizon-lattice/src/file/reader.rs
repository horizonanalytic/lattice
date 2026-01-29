//! File reading operations.

use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::error::{FileError, FileResult};

/// A file handle for reading operations.
///
/// This wraps a standard library file handle with additional convenience methods
/// for common read patterns.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::File;
///
/// // Read a file line by line
/// let file = File::open("log.txt")?;
/// for line in file.lines() {
///     println!("{}", line?);
/// }
///
/// // Read in chunks
/// let mut file = File::open("large.bin")?;
/// let mut buffer = [0u8; 4096];
/// while let Some(n) = file.read_chunk(&mut buffer)? {
///     // Process n bytes...
/// }
/// ```
pub struct File {
    /// The underlying file handle.
    inner: fs::File,
    /// The path to the file (for error messages).
    path: PathBuf,
}

impl File {
    /// Opens a file for reading.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let inner = fs::File::open(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self { inner, path })
    }

    /// Returns the path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the size of the file in bytes.
    pub fn size(&self) -> FileResult<u64> {
        self.inner
            .metadata()
            .map(|m| m.len())
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Reads the entire file contents as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not valid UTF-8.
    pub fn read_to_string(&mut self) -> FileResult<String> {
        let mut contents = String::new();
        self.inner
            .read_to_string(&mut contents)
            .map_err(|e| FileError::from_io(e, &self.path))?;
        Ok(contents)
    }

    /// Reads the entire file contents as bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_to_end(&mut self) -> FileResult<Vec<u8>> {
        let mut contents = Vec::new();
        self.inner
            .read_to_end(&mut contents)
            .map_err(|e| FileError::from_io(e, &self.path))?;
        Ok(contents)
    }

    /// Reads a chunk of bytes into the provided buffer.
    ///
    /// Returns `Some(n)` where `n` is the number of bytes read, or `None` if EOF.
    /// This is useful for processing large files without loading them entirely into memory.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut file = File::open("large.bin")?;
    /// let mut buffer = [0u8; 4096];
    /// while let Some(n) = file.read_chunk(&mut buffer)? {
    ///     process_chunk(&buffer[..n]);
    /// }
    /// ```
    pub fn read_chunk(&mut self, buf: &mut [u8]) -> FileResult<Option<usize>> {
        match self.inner.read(buf) {
            Ok(0) => Ok(None),
            Ok(n) => Ok(Some(n)),
            Err(e) => Err(FileError::from_io(e, &self.path)),
        }
    }

    /// Reads exactly the specified number of bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if EOF is reached before reading the requested bytes.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> FileResult<()> {
        self.inner
            .read_exact(buf)
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Returns an iterator over the lines in the file.
    ///
    /// Each line is returned without the trailing newline character.
    ///
    /// # Example
    ///
    /// ```ignore
    /// for line in File::open("data.txt")?.lines() {
    ///     println!("{}", line?);
    /// }
    /// ```
    pub fn lines(self) -> LineIterator {
        LineIterator::new(self)
    }

    /// Returns a buffered reader for this file.
    ///
    /// Use this when you need fine-grained control over buffered reading.
    pub fn buffered(self) -> BufReader<fs::File> {
        BufReader::new(self.inner)
    }

    /// Seeks to a position in the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the seek fails.
    pub fn seek(&mut self, pos: SeekFrom) -> FileResult<u64> {
        self.inner
            .seek(pos)
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Seeks to the beginning of the file.
    pub fn rewind(&mut self) -> FileResult<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    /// Returns the current position in the file.
    pub fn position(&mut self) -> FileResult<u64> {
        self.inner
            .stream_position()
            .map_err(|e| FileError::from_io(e, &self.path))
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

// ============================================================================
// Line Iterator
// ============================================================================

/// An iterator over the lines in a file.
///
/// Each item is a `FileResult<String>` containing a line without its trailing newline.
pub struct LineIterator {
    reader: BufReader<fs::File>,
    path: PathBuf,
    line_buffer: String,
}

impl LineIterator {
    fn new(file: File) -> Self {
        Self {
            path: file.path,
            reader: BufReader::new(file.inner),
            line_buffer: String::new(),
        }
    }
}

impl Iterator for LineIterator {
    type Item = FileResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_buffer.clear();
        match self.reader.read_line(&mut self.line_buffer) {
            Ok(0) => None,
            Ok(_) => {
                // Remove trailing newline
                if self.line_buffer.ends_with('\n') {
                    self.line_buffer.pop();
                    if self.line_buffer.ends_with('\r') {
                        self.line_buffer.pop();
                    }
                }
                Some(Ok(std::mem::take(&mut self.line_buffer)))
            }
            Err(e) => Some(Err(FileError::from_io(e, &self.path))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file(name: &str, content: &[u8]) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("horizon_test_{}", name));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    fn cleanup(path: &Path) {
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_open_file() {
        let path = create_test_file("open.txt", b"test content");

        let file = File::open(&path).unwrap();
        assert_eq!(file.path(), path);

        cleanup(&path);
    }

    #[test]
    fn test_read_to_string() {
        let path = create_test_file("read_string.txt", b"Hello, World!");

        let mut file = File::open(&path).unwrap();
        let content = file.read_to_string().unwrap();
        assert_eq!(content, "Hello, World!");

        cleanup(&path);
    }

    #[test]
    fn test_read_to_end() {
        let path = create_test_file("read_bytes.bin", &[0x00, 0x01, 0x02, 0x03]);

        let mut file = File::open(&path).unwrap();
        let bytes = file.read_to_end().unwrap();
        assert_eq!(bytes, vec![0x00, 0x01, 0x02, 0x03]);

        cleanup(&path);
    }

    #[test]
    fn test_read_chunk() {
        let path = create_test_file("read_chunk.bin", b"0123456789");

        let mut file = File::open(&path).unwrap();
        let mut buffer = [0u8; 4];

        let n = file.read_chunk(&mut buffer).unwrap();
        assert_eq!(n, Some(4));
        assert_eq!(&buffer, b"0123");

        let n = file.read_chunk(&mut buffer).unwrap();
        assert_eq!(n, Some(4));
        assert_eq!(&buffer, b"4567");

        let n = file.read_chunk(&mut buffer).unwrap();
        assert_eq!(n, Some(2));
        assert_eq!(&buffer[..2], b"89");

        let n = file.read_chunk(&mut buffer).unwrap();
        assert_eq!(n, None);

        cleanup(&path);
    }

    #[test]
    fn test_lines() {
        let path = create_test_file("lines.txt", b"line 1\nline 2\nline 3");

        let file = File::open(&path).unwrap();
        let lines: Vec<String> = file.lines().map(|r| r.unwrap()).collect();

        assert_eq!(lines, vec!["line 1", "line 2", "line 3"]);

        cleanup(&path);
    }

    #[test]
    fn test_lines_crlf() {
        let path = create_test_file("lines_crlf.txt", b"line 1\r\nline 2\r\n");

        let file = File::open(&path).unwrap();
        let lines: Vec<String> = file.lines().map(|r| r.unwrap()).collect();

        assert_eq!(lines, vec!["line 1", "line 2"]);

        cleanup(&path);
    }

    #[test]
    fn test_seek_rewind() {
        let path = create_test_file("seek.txt", b"Hello, World!");

        let mut file = File::open(&path).unwrap();

        // Read first 5 bytes
        let mut buf = [0u8; 5];
        file.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello");

        // Check position
        assert_eq!(file.position().unwrap(), 5);

        // Rewind and read again
        file.rewind().unwrap();
        file.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello");

        cleanup(&path);
    }

    #[test]
    fn test_file_size() {
        let path = create_test_file("size.txt", b"12345");

        let file = File::open(&path).unwrap();
        assert_eq!(file.size().unwrap(), 5);

        cleanup(&path);
    }

    #[test]
    fn test_file_not_found() {
        let result = File::open("/nonexistent/path/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }
}
