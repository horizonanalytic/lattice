//! Async directory operations and iteration.
//!
//! This module provides async versions of directory operations using tokio's
//! async file system APIs. These are suitable for use in async contexts where
//! blocking I/O is not acceptable.
//!
//! # Async Directory Listing
//!
//! ```ignore
//! use horizon_lattice::file::{read_dir_async, AsyncDirEntry};
//! use tokio_stream::StreamExt;
//!
//! // List directory entries asynchronously
//! let mut entries = read_dir_async("src").await?;
//! while let Some(entry) = entries.next().await {
//!     let entry = entry?;
//!     println!("{}", entry.name());
//! }
//! ```
//!
//! # Async Recursive Directory Walking
//!
//! ```ignore
//! use horizon_lattice::file::{AsyncWalkDir, WalkDirOptions};
//! use tokio_stream::StreamExt;
//!
//! // Walk directory recursively
//! let mut walker = AsyncWalkDir::new("src").await?;
//! while let Some(entry) = walker.next().await {
//!     let entry = entry?;
//!     println!("{}: depth {}", entry.path().display(), entry.depth());
//! }
//!
//! // With options
//! let options = WalkDirOptions::new()
//!     .files_only()
//!     .glob("*.rs")
//!     .skip_hidden(true);
//! let mut walker = AsyncWalkDir::with_options("src", options).await?;
//! while let Some(entry) = walker.next().await {
//!     println!("{}", entry?.path().display());
//! }
//! ```

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;

use tokio::fs;
use tokio_stream::Stream;

use super::directory::{glob_to_regex, WalkDirOptions};
use super::error::{FileError, FileErrorKind, FileResult};
use super::info::{FileType, Permissions};

// ============================================================================
// AsyncDirEntry
// ============================================================================

/// An async directory entry returned from reading a directory.
///
/// This wraps `tokio::fs::DirEntry` with additional convenience methods.
#[derive(Debug)]
pub struct AsyncDirEntry {
    inner: fs::DirEntry,
}

impl AsyncDirEntry {
    /// Creates a new `AsyncDirEntry` from a `tokio::fs::DirEntry`.
    fn new(inner: fs::DirEntry) -> Self {
        Self { inner }
    }

    /// Returns the full path to this entry.
    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }

    /// Returns the file name of this entry.
    pub fn name(&self) -> String {
        self.inner.file_name().to_string_lossy().into_owned()
    }

    /// Returns the file name as an `OsString`.
    pub fn file_name(&self) -> std::ffi::OsString {
        self.inner.file_name()
    }

    /// Returns the file type for this entry.
    ///
    /// This does not follow symbolic links; if this entry is a symlink,
    /// `FileType::Symlink` is returned.
    pub async fn file_type(&self) -> FileResult<FileType> {
        self.inner
            .file_type()
            .await
            .map(FileType::from)
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns true if this entry is a file.
    pub async fn is_file(&self) -> FileResult<bool> {
        Ok(self.file_type().await?.is_file())
    }

    /// Returns true if this entry is a directory.
    pub async fn is_dir(&self) -> FileResult<bool> {
        Ok(self.file_type().await?.is_directory())
    }

    /// Returns true if this entry is a symbolic link.
    pub async fn is_symlink(&self) -> FileResult<bool> {
        Ok(self.file_type().await?.is_symlink())
    }

    /// Returns the metadata for this entry.
    ///
    /// This does not follow symbolic links.
    pub async fn metadata(&self) -> FileResult<std::fs::Metadata> {
        self.inner
            .metadata()
            .await
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns the size of the file in bytes.
    pub async fn size(&self) -> FileResult<u64> {
        Ok(self.metadata().await?.len())
    }

    /// Returns the file permissions.
    pub async fn permissions(&self) -> FileResult<Permissions> {
        Ok(self.metadata().await?.permissions().into())
    }

    /// Returns the time this entry was last modified.
    pub async fn modified(&self) -> FileResult<Option<SystemTime>> {
        Ok(self.metadata().await?.modified().ok())
    }
}

// ============================================================================
// AsyncDirIterator - Async directory stream
// ============================================================================

/// An async stream over the entries in a directory.
///
/// This stream yields `FileResult<AsyncDirEntry>` items.
pub struct AsyncDirIterator {
    inner: fs::ReadDir,
    path: PathBuf,
}

impl AsyncDirIterator {
    /// Creates a new async directory iterator.
    async fn new(path: PathBuf) -> FileResult<Self> {
        let inner = fs::read_dir(&path)
            .await
            .map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self { inner, path })
    }

    /// Returns the path being iterated.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the next entry in the directory.
    pub async fn next(&mut self) -> Option<FileResult<AsyncDirEntry>> {
        match self.inner.next_entry().await {
            Ok(Some(entry)) => Some(Ok(AsyncDirEntry::new(entry))),
            Ok(None) => None,
            Err(e) => Some(Err(FileError::from_io(e, &self.path))),
        }
    }

    /// Collects all entries into a vector.
    ///
    /// This is a convenience method that collects successful entries,
    /// ignoring any errors.
    pub async fn collect_ok(mut self) -> Vec<AsyncDirEntry> {
        let mut entries = Vec::new();
        while let Some(result) = self.next().await {
            if let Ok(entry) = result {
                entries.push(entry);
            }
        }
        entries
    }

    /// Collects all entry paths into a vector.
    ///
    /// This is a convenience method that collects paths from successful entries.
    pub async fn collect_paths(mut self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Some(result) = self.next().await {
            if let Ok(entry) = result {
                paths.push(entry.path());
            }
        }
        paths
    }

    /// Collects all entry names into a vector.
    ///
    /// This is a convenience method that collects names from successful entries.
    pub async fn collect_names(mut self) -> Vec<String> {
        let mut names = Vec::new();
        while let Some(result) = self.next().await {
            if let Ok(entry) = result {
                names.push(entry.name());
            }
        }
        names
    }
}

// Implement Stream for AsyncDirIterator using a boxed future
impl Stream for AsyncDirIterator {
    type Item = FileResult<AsyncDirEntry>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let path = this.path.clone();

        let fut = this.inner.next_entry();
        tokio::pin!(fut);

        match fut.poll(cx) {
            Poll::Ready(Ok(Some(entry))) => Poll::Ready(Some(Ok(AsyncDirEntry::new(entry)))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(FileError::from_io(e, path)))),
            Poll::Pending => Poll::Pending,
        }
    }
}

// ============================================================================
// AsyncWalkEntry - Entry type for async recursive walking
// ============================================================================

/// Entry type for async recursive directory walking.
#[derive(Debug)]
pub struct AsyncWalkEntry {
    /// The path to this entry.
    path: PathBuf,
    /// The depth of this entry relative to the root.
    depth: usize,
    /// The file type of this entry.
    file_type: FileType,
}

impl AsyncWalkEntry {
    /// Creates a new walk entry.
    fn new(path: PathBuf, depth: usize, file_type: FileType) -> Self {
        Self {
            path,
            depth,
            file_type,
        }
    }

    /// Returns the full path to this entry.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file name of this entry.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Returns the depth of this entry relative to the walk root.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the file type of this entry.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns true if this entry is a file.
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    /// Returns true if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type.is_directory()
    }

    /// Returns true if this entry is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }

    /// Returns the metadata for this entry.
    pub async fn metadata(&self) -> FileResult<std::fs::Metadata> {
        tokio::fs::symlink_metadata(&self.path)
            .await
            .map_err(|e| FileError::from_io(e, &self.path))
    }

    /// Returns the size of the file in bytes.
    pub async fn size(&self) -> FileResult<u64> {
        Ok(self.metadata().await?.len())
    }

    /// Consumes the entry and returns the path.
    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

// ============================================================================
// AsyncWalkDir - Async recursive directory walker
// ============================================================================

/// An async recursive directory walker.
///
/// This walker traverses a directory tree asynchronously, yielding entries as they
/// are discovered. It uses a breadth-first traversal.
///
/// Unlike a synchronous iterator, this walker provides an async `next()` method
/// that should be called in a loop with `.await`.
pub struct AsyncWalkDir {
    /// Queue of directories to visit (path, depth).
    queue: VecDeque<(PathBuf, usize)>,
    /// Current directory iterator.
    current: Option<(fs::ReadDir, PathBuf, usize)>,
    /// Configuration options.
    options: WalkDirOptions,
    /// Compiled glob pattern (if any).
    glob_regex: Option<regex::Regex>,
    /// Whether we've yielded the root yet.
    yielded_root: bool,
    /// The root path.
    root: PathBuf,
}

impl AsyncWalkDir {
    /// Creates a new async recursive directory walker.
    pub async fn new(path: impl AsRef<Path>) -> FileResult<Self> {
        Self::with_options(path, WalkDirOptions::default()).await
    }

    /// Creates a new async recursive directory walker with options.
    pub async fn with_options(path: impl AsRef<Path>, options: WalkDirOptions) -> FileResult<Self> {
        let root = path.as_ref().to_path_buf();

        // Verify root exists and is a directory
        let metadata = fs::metadata(&root)
            .await
            .map_err(|e| FileError::from_io(e, &root))?;
        if !metadata.is_dir() {
            return Err(FileError::new(
                FileErrorKind::NotDirectory,
                Some(root),
                None,
            ));
        }

        // Compile glob pattern if provided
        let glob_regex = if let Some(ref pattern) = options.glob_pattern {
            Some(glob_to_regex(pattern)?)
        } else {
            None
        };

        let mut queue = VecDeque::new();
        queue.push_back((root.clone(), 1));

        Ok(Self {
            queue,
            current: None,
            options,
            glob_regex,
            yielded_root: false,
            root,
        })
    }

    /// Returns the next entry in the directory walk.
    ///
    /// This is the primary way to iterate over the walker. Call this method
    /// in a loop with `.await` to get each entry.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut walker = AsyncWalkDir::new("src").await?;
    /// while let Some(entry) = walker.next().await {
    ///     let entry = entry?;
    ///     println!("{}", entry.path().display());
    /// }
    /// ```
    pub async fn next(&mut self) -> Option<FileResult<AsyncWalkEntry>> {
        // Yield root directory if configured and not yet yielded
        if self.options.include_root && !self.yielded_root {
            self.yielded_root = true;
            if self.options.include_dirs {
                let entry = AsyncWalkEntry::new(self.root.clone(), 0, FileType::Directory);
                if self.should_include(&entry) {
                    return Some(Ok(entry));
                }
            }
        }

        loop {
            // If we have a current directory iterator, try to get the next entry
            if let Some((ref mut read_dir, ref dir_path, depth)) = self.current {
                match read_dir.next_entry().await {
                    Ok(Some(fs_entry)) => {
                        let entry = AsyncDirEntry::new(fs_entry);

                        // Get file type
                        let file_type = match entry.file_type().await {
                            Ok(ft) => ft,
                            Err(e) => return Some(Err(e)),
                        };

                        let walk_entry = AsyncWalkEntry::new(entry.path(), depth, file_type);

                        // Queue subdirectory for later if we should descend
                        if self.should_descend(&walk_entry, depth) {
                            self.queue.push_back((walk_entry.path.clone(), depth + 1));
                        }

                        // Return entry if it matches filters
                        if self.should_include(&walk_entry) {
                            return Some(Ok(walk_entry));
                        }

                        // Otherwise continue to next entry
                        continue;
                    }
                    Ok(None) => {
                        // Current directory exhausted, move to next
                        self.current = None;
                    }
                    Err(e) => {
                        return Some(Err(FileError::from_io(e, dir_path.as_path())));
                    }
                }
            }

            // Get next directory from queue
            match self.queue.pop_front() {
                Some((path, depth)) => {
                    match fs::read_dir(&path).await {
                        Ok(read_dir) => {
                            self.current = Some((read_dir, path, depth));
                            // Continue loop to process entries
                        }
                        Err(e) => {
                            return Some(Err(FileError::from_io(e, path)));
                        }
                    }
                }
                None => {
                    // No more directories to process
                    return None;
                }
            }
        }
    }

    /// Collects all entries into a vector.
    ///
    /// This consumes the walker and collects successful entries.
    pub async fn collect_ok(mut self) -> Vec<AsyncWalkEntry> {
        let mut entries = Vec::new();
        while let Some(result) = self.next().await {
            if let Ok(entry) = result {
                entries.push(entry);
            }
        }
        entries
    }

    /// Collects all paths into a vector.
    pub async fn collect_paths(mut self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Some(result) = self.next().await {
            if let Ok(entry) = result {
                paths.push(entry.into_path());
            }
        }
        paths
    }

    /// Returns true if an entry should be included based on options.
    fn should_include(&self, entry: &AsyncWalkEntry) -> bool {
        // Check file/dir inclusion
        if entry.is_file() && !self.options.include_files {
            return false;
        }
        if entry.is_dir() && !self.options.include_dirs {
            return false;
        }

        // Check hidden files
        if self.options.skip_hidden {
            let name = entry.name();
            if name.starts_with('.') && !name.is_empty() {
                return false;
            }
        }

        // Check glob pattern
        if let Some(ref regex) = self.glob_regex {
            if !regex.is_match(&entry.name()) {
                return false;
            }
        }

        true
    }

    /// Returns true if a directory should be descended into.
    fn should_descend(&self, entry: &AsyncWalkEntry, depth: usize) -> bool {
        // Check depth limit
        if let Some(max_depth) = self.options.max_depth {
            if depth >= max_depth {
                return false;
            }
        }

        // Check if it's a directory
        if !entry.is_dir() {
            return false;
        }

        // Check symlinks
        if entry.is_symlink() && !self.options.follow_symlinks {
            return false;
        }

        // Check hidden directories
        if self.options.skip_hidden && entry.name().starts_with('.') {
            return false;
        }

        true
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Reads a directory asynchronously and returns an async iterator over its entries.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::read_dir_async;
///
/// let mut entries = read_dir_async("src").await?;
/// while let Some(entry) = entries.next().await {
///     let entry = entry?;
///     println!("{}", entry.name());
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the path does not exist, is not a directory,
/// or cannot be read.
pub async fn read_dir_async(path: impl AsRef<Path>) -> FileResult<AsyncDirIterator> {
    AsyncDirIterator::new(path.as_ref().to_path_buf()).await
}

/// Recursively reads a directory asynchronously and returns an async walker.
///
/// This is a convenience function that creates an `AsyncWalkDir` with default options.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::read_dir_recursive_async;
///
/// let mut walker = read_dir_recursive_async("src").await?;
/// while let Some(entry) = walker.next().await {
///     let entry = entry?;
///     println!("{}: depth {}", entry.path().display(), entry.depth());
/// }
/// ```
pub async fn read_dir_recursive_async(path: impl AsRef<Path>) -> FileResult<AsyncWalkDir> {
    AsyncWalkDir::new(path).await
}

/// Lists all entries in a directory asynchronously as paths.
///
/// This is a convenience function that collects all directory entry paths.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::list_dir_async;
///
/// let entries = list_dir_async("src").await?;
/// for path in entries {
///     println!("{}", path.display());
/// }
/// ```
pub async fn list_dir_async(path: impl AsRef<Path>) -> FileResult<Vec<PathBuf>> {
    Ok(read_dir_async(path).await?.collect_paths().await)
}

/// Checks if a directory is empty asynchronously.
///
/// # Errors
///
/// Returns an error if the path does not exist or is not a directory.
pub async fn is_dir_empty_async(path: impl AsRef<Path>) -> FileResult<bool> {
    let mut iter = read_dir_async(path).await?;
    Ok(iter.next().await.is_none())
}

/// Calculates the total size of a directory recursively (async).
///
/// This includes all files in all subdirectories.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::dir_size_async;
///
/// let size = dir_size_async("my_folder").await?;
/// println!("Total size: {} bytes", size);
/// ```
pub async fn dir_size_async(path: impl AsRef<Path>) -> FileResult<u64> {
    let mut total = 0u64;
    let mut walker = AsyncWalkDir::with_options(&path, WalkDirOptions::new().files_only()).await?;

    while let Some(result) = walker.next().await {
        if let Ok(entry) = result {
            if let Ok(size) = entry.size().await {
                total += size;
            }
        }
    }

    Ok(total)
}

/// Counts the number of entries in a directory asynchronously.
///
/// # Arguments
///
/// * `path` - The directory path
/// * `recursive` - Whether to count entries in subdirectories
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::count_entries_async;
///
/// let count = count_entries_async("src", false).await?;
/// println!("Direct entries: {}", count);
///
/// let total = count_entries_async("src", true).await?;
/// println!("Total entries (recursive): {}", total);
/// ```
pub async fn count_entries_async(path: impl AsRef<Path>, recursive: bool) -> FileResult<usize> {
    if recursive {
        let walker = AsyncWalkDir::new(&path).await?;
        Ok(walker.collect_ok().await.len())
    } else {
        let iter = read_dir_async(&path).await?;
        Ok(iter.collect_ok().await.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Creates a unique test directory with a standard structure.
    fn setup_test_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Create test structure
        std::fs::create_dir_all(test_dir.join("subdir1")).unwrap();
        std::fs::create_dir_all(test_dir.join("subdir2/nested")).unwrap();

        // Create test files
        File::create(test_dir.join("file1.txt"))
            .unwrap()
            .write_all(b"hello")
            .unwrap();
        File::create(test_dir.join("file2.rs"))
            .unwrap()
            .write_all(b"fn main() {}")
            .unwrap();
        File::create(test_dir.join("subdir1/nested_file.txt"))
            .unwrap()
            .write_all(b"nested content")
            .unwrap();
        File::create(test_dir.join("subdir2/nested/deep.txt"))
            .unwrap()
            .write_all(b"deep content")
            .unwrap();
        File::create(test_dir.join(".hidden"))
            .unwrap()
            .write_all(b"hidden")
            .unwrap();

        (temp_dir, test_dir)
    }

    #[tokio::test]
    async fn test_read_dir_async() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = read_dir_async(&test_dir).await.unwrap().collect_ok().await;

        // Should have: file1.txt, file2.rs, subdir1, subdir2, .hidden
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_async_walk_dir_recursive() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = AsyncWalkDir::new(&test_dir)
            .await
            .unwrap()
            .collect_ok()
            .await;

        // Should include all files and directories recursively
        assert!(entries.len() >= 8);
    }

    #[tokio::test]
    async fn test_async_walk_dir_files_only() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let files = AsyncWalkDir::with_options(&test_dir, WalkDirOptions::new().files_only())
            .await
            .unwrap()
            .collect_ok()
            .await;

        // All entries should be files
        assert!(files.iter().all(|e| e.is_file()));

        // Should have: file1.txt, file2.rs, .hidden, nested_file.txt, deep.txt
        assert_eq!(files.len(), 5);
    }

    #[tokio::test]
    async fn test_async_walk_dir_max_depth() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = AsyncWalkDir::with_options(&test_dir, WalkDirOptions::new().max_depth(1))
            .await
            .unwrap()
            .collect_ok()
            .await;

        // Should not include deep nested files
        let paths: Vec<_> = entries.iter().map(|e| e.path().to_path_buf()).collect();
        assert!(!paths.iter().any(|p| p.ends_with("deep.txt")));
    }

    #[tokio::test]
    async fn test_async_walk_dir_skip_hidden() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries =
            AsyncWalkDir::with_options(&test_dir, WalkDirOptions::new().skip_hidden(true))
                .await
                .unwrap()
                .collect_ok()
                .await;

        // Should not include .hidden
        let names: Vec<_> = entries.iter().map(|e| e.name()).collect();
        assert!(!names.contains(&".hidden".to_string()));
    }

    #[tokio::test]
    async fn test_async_walk_dir_with_glob() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let txt_files =
            AsyncWalkDir::with_options(&test_dir, WalkDirOptions::new().glob("*.txt"))
                .await
                .unwrap()
                .collect_ok()
                .await;

        // Should find file1.txt, nested_file.txt, deep.txt
        assert_eq!(txt_files.len(), 3);
        assert!(txt_files.iter().all(|e| e.name().ends_with(".txt")));
    }

    #[tokio::test]
    async fn test_list_dir_async() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = list_dir_async(&test_dir).await.unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_is_dir_empty_async() {
        let temp_dir = TempDir::new().unwrap();
        let empty_dir = temp_dir.path().join("empty");
        std::fs::create_dir(&empty_dir).unwrap();

        assert!(is_dir_empty_async(&empty_dir).await.unwrap());

        // Add a file
        File::create(empty_dir.join("file.txt")).unwrap();
        assert!(!is_dir_empty_async(&empty_dir).await.unwrap());
    }

    #[tokio::test]
    async fn test_dir_size_async() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let size = dir_size_async(&test_dir).await.unwrap();

        // Total size should be sum of all file contents
        // "hello" (5) + "fn main() {}" (12) + "nested content" (14) +
        // "deep content" (12) + "hidden" (6) = 49
        assert_eq!(size, 49);
    }

    #[tokio::test]
    async fn test_count_entries_async() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let direct_count = count_entries_async(&test_dir, false).await.unwrap();
        assert_eq!(direct_count, 5);

        let recursive_count = count_entries_async(&test_dir, true).await.unwrap();
        assert!(recursive_count >= 8);
    }

    #[tokio::test]
    async fn test_async_dir_entry_metadata() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = read_dir_async(&test_dir).await.unwrap().collect_ok().await;

        for entry in entries {
            // Should be able to get file type
            assert!(entry.file_type().await.is_ok());

            // Should be able to get metadata
            assert!(entry.metadata().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_async_walk_entry_depth() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries = AsyncWalkDir::new(&test_dir)
            .await
            .unwrap()
            .collect_ok()
            .await;

        // Check depths are correct
        for entry in &entries {
            let path = entry.path();
            let relative = path.strip_prefix(&test_dir).unwrap();
            let expected_depth = relative.components().count();
            assert_eq!(entry.depth(), expected_depth);
        }
    }
}
