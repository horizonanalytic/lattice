//! Directory operations and iteration.
//!
//! This module provides cross-platform directory operations including listing,
//! creation, removal, and recursive traversal with optional filtering.
//!
//! # Listing Directories
//!
//! ```ignore
//! use horizon_lattice::file::{read_dir, read_dir_recursive, DirEntry};
//!
//! // List directory entries
//! for entry in read_dir("src")? {
//!     let entry = entry?;
//!     println!("{}: {:?}", entry.name(), entry.file_type()?);
//! }
//!
//! // Recursive listing
//! for entry in read_dir_recursive("src")? {
//!     let entry = entry?;
//!     println!("{}", entry.path().display());
//! }
//!
//! // Filtered listing with glob pattern
//! for entry in read_dir("src")?.filter_glob("*.rs")? {
//!     println!("{}", entry?.name());
//! }
//! ```
//!
//! # Directory Operations
//!
//! ```ignore
//! use horizon_lattice::file::{create_dir, create_dir_all, remove_dir, remove_dir_all};
//!
//! // Create a single directory
//! create_dir("new_folder")?;
//!
//! // Create a directory tree
//! create_dir_all("path/to/nested/folder")?;
//!
//! // Remove an empty directory
//! remove_dir("empty_folder")?;
//!
//! // Remove a directory and all its contents
//! remove_dir_all("folder_with_contents")?;
//! ```

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::error::{FileError, FileErrorKind, FileResult};
use super::info::{FileType, Permissions};

// ============================================================================
// DirEntry
// ============================================================================

/// A directory entry returned from reading a directory.
///
/// This wraps `std::fs::DirEntry` with additional convenience methods.
#[derive(Debug)]
pub struct DirEntry {
    inner: fs::DirEntry,
}

impl DirEntry {
    /// Creates a new `DirEntry` from a `std::fs::DirEntry`.
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
    pub fn file_type(&self) -> FileResult<FileType> {
        self.inner
            .file_type()
            .map(FileType::from)
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns true if this entry is a file.
    pub fn is_file(&self) -> FileResult<bool> {
        Ok(self.file_type()?.is_file())
    }

    /// Returns true if this entry is a directory.
    pub fn is_dir(&self) -> FileResult<bool> {
        Ok(self.file_type()?.is_directory())
    }

    /// Returns true if this entry is a symbolic link.
    pub fn is_symlink(&self) -> FileResult<bool> {
        Ok(self.file_type()?.is_symlink())
    }

    /// Returns the metadata for this entry.
    ///
    /// This does not follow symbolic links.
    pub fn metadata(&self) -> FileResult<fs::Metadata> {
        self.inner
            .metadata()
            .map_err(|e| FileError::from_io(e, self.path()))
    }

    /// Returns the size of the file in bytes.
    pub fn size(&self) -> FileResult<u64> {
        Ok(self.metadata()?.len())
    }

    /// Returns the file permissions.
    pub fn permissions(&self) -> FileResult<Permissions> {
        Ok(self.metadata()?.permissions().into())
    }

    /// Returns the time this entry was last modified.
    pub fn modified(&self) -> FileResult<Option<SystemTime>> {
        Ok(self.metadata()?.modified().ok())
    }

    /// Returns a reference to the underlying `std::fs::DirEntry`.
    pub fn as_std(&self) -> &fs::DirEntry {
        &self.inner
    }

    /// Consumes this entry and returns the underlying `std::fs::DirEntry`.
    pub fn into_std(self) -> fs::DirEntry {
        self.inner
    }
}

// ============================================================================
// DirIterator - Basic directory iterator
// ============================================================================

/// An iterator over the entries in a directory.
///
/// This iterator yields `FileResult<DirEntry>` items.
pub struct DirIterator {
    inner: fs::ReadDir,
    path: PathBuf,
}

impl DirIterator {
    /// Creates a new directory iterator.
    fn new(path: PathBuf, inner: fs::ReadDir) -> Self {
        Self { inner, path }
    }

    /// Returns the path being iterated.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Filters entries using a predicate function.
    ///
    /// The predicate receives a reference to each `DirEntry` and returns
    /// `true` to include it or `false` to skip it.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // List only files
    /// for entry in read_dir("src")?.filter_by(|e| e.is_file().unwrap_or(false)) {
    ///     println!("{}", entry?.name());
    /// }
    /// ```
    pub fn filter_by<F>(self, predicate: F) -> FilteredDirIterator<F>
    where
        F: FnMut(&DirEntry) -> bool,
    {
        FilteredDirIterator {
            inner: self,
            predicate,
        }
    }

    /// Filters entries using a glob pattern.
    ///
    /// Supported glob syntax:
    /// - `*` matches any sequence of characters except path separators
    /// - `?` matches any single character except path separators
    /// - `[abc]` matches any character in the brackets
    /// - `[a-z]` matches any character in the range
    /// - `[!abc]` or `[^abc]` matches any character not in the brackets
    ///
    /// # Example
    ///
    /// ```ignore
    /// // List all Rust source files
    /// for entry in read_dir("src")?.filter_glob("*.rs")? {
    ///     println!("{}", entry?.name());
    /// }
    /// ```
    pub fn filter_glob(self, pattern: &str) -> FileResult<GlobDirIterator> {
        let regex = glob_to_regex(pattern)?;
        Ok(GlobDirIterator {
            inner: self,
            pattern: regex,
        })
    }

    /// Collects all entries into a vector.
    ///
    /// This is a convenience method that collects successful entries,
    /// ignoring any errors.
    pub fn collect_ok(self) -> Vec<DirEntry> {
        self.filter_map(Result::ok).collect()
    }

    /// Collects all entry paths into a vector.
    ///
    /// This is a convenience method that collects paths from successful entries.
    pub fn collect_paths(self) -> Vec<PathBuf> {
        self.filter_map(Result::ok).map(|e| e.path()).collect()
    }

    /// Collects all entry names into a vector.
    ///
    /// This is a convenience method that collects names from successful entries.
    pub fn collect_names(self) -> Vec<String> {
        self.filter_map(Result::ok).map(|e| e.name()).collect()
    }
}

impl Iterator for DirIterator {
    type Item = FileResult<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result
                .map(DirEntry::new)
                .map_err(|e| FileError::from_io(e, &self.path))
        })
    }
}

// ============================================================================
// FilteredDirIterator - Predicate-filtered iterator
// ============================================================================

/// An iterator that filters directory entries using a predicate.
pub struct FilteredDirIterator<F> {
    inner: DirIterator,
    predicate: F,
}

impl<F> Iterator for FilteredDirIterator<F>
where
    F: FnMut(&DirEntry) -> bool,
{
    type Item = FileResult<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(Ok(entry)) => {
                    if (self.predicate)(&entry) {
                        return Some(Ok(entry));
                    }
                    // Skip this entry and continue
                }
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

// ============================================================================
// GlobDirIterator - Glob pattern filtered iterator
// ============================================================================

/// An iterator that filters directory entries using a glob pattern.
pub struct GlobDirIterator {
    inner: DirIterator,
    pattern: regex::Regex,
}

impl Iterator for GlobDirIterator {
    type Item = FileResult<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(Ok(entry)) => {
                    if self.pattern.is_match(&entry.name()) {
                        return Some(Ok(entry));
                    }
                    // Skip this entry and continue
                }
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

// ============================================================================
// WalkDir - Recursive directory iterator
// ============================================================================

/// Configuration options for recursive directory walking.
#[derive(Debug, Clone)]
pub struct WalkDirOptions {
    /// Maximum depth to descend into subdirectories (None = unlimited).
    pub max_depth: Option<usize>,
    /// Whether to follow symbolic links to directories.
    pub follow_symlinks: bool,
    /// Whether to include directories in the output.
    pub include_dirs: bool,
    /// Whether to include files in the output.
    pub include_files: bool,
    /// Whether to yield the root directory itself.
    pub include_root: bool,
    /// Optional glob pattern to filter entries.
    pub glob_pattern: Option<String>,
    /// Whether to skip hidden files (starting with '.').
    pub skip_hidden: bool,
}

impl Default for WalkDirOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            follow_symlinks: false,
            include_dirs: true,
            include_files: true,
            include_root: false,
            glob_pattern: None,
            skip_hidden: false,
        }
    }
}

impl WalkDirOptions {
    /// Creates a new `WalkDirOptions` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum depth to descend.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Sets whether to follow symbolic links.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Sets whether to include directories in output.
    pub fn include_dirs(mut self, include: bool) -> Self {
        self.include_dirs = include;
        self
    }

    /// Sets whether to include files in output.
    pub fn include_files(mut self, include: bool) -> Self {
        self.include_files = include;
        self
    }

    /// Sets whether to include the root directory.
    pub fn include_root(mut self, include: bool) -> Self {
        self.include_root = include;
        self
    }

    /// Sets a glob pattern to filter entries.
    pub fn glob(mut self, pattern: impl Into<String>) -> Self {
        self.glob_pattern = Some(pattern.into());
        self
    }

    /// Sets whether to skip hidden files.
    pub fn skip_hidden(mut self, skip: bool) -> Self {
        self.skip_hidden = skip;
        self
    }

    /// Only include files (exclude directories).
    pub fn files_only(mut self) -> Self {
        self.include_files = true;
        self.include_dirs = false;
        self
    }

    /// Only include directories (exclude files).
    pub fn dirs_only(mut self) -> Self {
        self.include_files = false;
        self.include_dirs = true;
        self
    }
}

/// Entry type for recursive directory walking.
#[derive(Debug)]
pub struct WalkEntry {
    /// The path to this entry.
    path: PathBuf,
    /// The depth of this entry relative to the root.
    depth: usize,
    /// The file type of this entry.
    file_type: FileType,
    /// Cached metadata (lazily loaded).
    metadata: Option<fs::Metadata>,
}

impl WalkEntry {
    /// Creates a new walk entry.
    fn new(path: PathBuf, depth: usize, file_type: FileType) -> Self {
        Self {
            path,
            depth,
            file_type,
            metadata: None,
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
    pub fn metadata(&mut self) -> FileResult<&fs::Metadata> {
        if self.metadata.is_none() {
            self.metadata = Some(
                fs::symlink_metadata(&self.path).map_err(|e| FileError::from_io(e, &self.path))?,
            );
        }
        Ok(self.metadata.as_ref().unwrap())
    }

    /// Returns the size of the file in bytes.
    pub fn size(&mut self) -> FileResult<u64> {
        Ok(self.metadata()?.len())
    }

    /// Consumes the entry and returns the path.
    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

/// A recursive directory iterator.
///
/// This iterator walks a directory tree lazily, yielding entries as they
/// are discovered. It uses a breadth-first traversal by default.
pub struct WalkDir {
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

impl WalkDir {
    /// Creates a new recursive directory walker.
    pub fn new(path: impl AsRef<Path>) -> FileResult<Self> {
        Self::with_options(path, WalkDirOptions::default())
    }

    /// Creates a new recursive directory walker with options.
    pub fn with_options(path: impl AsRef<Path>, options: WalkDirOptions) -> FileResult<Self> {
        let root = path.as_ref().to_path_buf();

        // Verify root exists and is a directory
        let metadata = fs::metadata(&root).map_err(|e| FileError::from_io(e, &root))?;
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
        // Start with depth 1 since entries directly in root are at depth 1
        // (root itself is depth 0)
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

    /// Collects all entries into a vector.
    ///
    /// This consumes the iterator and collects successful entries.
    pub fn collect_ok(self) -> Vec<WalkEntry> {
        self.filter_map(Result::ok).collect()
    }

    /// Collects all paths into a vector.
    pub fn collect_paths(self) -> Vec<PathBuf> {
        self.filter_map(Result::ok).map(|e| e.into_path()).collect()
    }

    /// Returns true if an entry should be included based on options.
    fn should_include(&self, entry: &WalkEntry) -> bool {
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
        if let Some(ref regex) = self.glob_regex
            && !regex.is_match(&entry.name())
        {
            return false;
        }

        true
    }

    /// Returns true if a directory should be descended into.
    fn should_descend(&self, entry: &WalkEntry, depth: usize) -> bool {
        // Check depth limit
        if let Some(max_depth) = self.options.max_depth
            && depth >= max_depth
        {
            return false;
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

impl Iterator for WalkDir {
    type Item = FileResult<WalkEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        // Yield root directory if configured and not yet yielded
        if self.options.include_root && !self.yielded_root {
            self.yielded_root = true;
            if self.options.include_dirs {
                let entry = WalkEntry::new(self.root.clone(), 0, FileType::Directory);
                if self.should_include(&entry) {
                    return Some(Ok(entry));
                }
            }
        }

        loop {
            // If we have a current directory iterator, try to get the next entry
            if let Some((ref mut read_dir, ref dir_path, depth)) = self.current {
                match read_dir.next() {
                    Some(Ok(fs_entry)) => {
                        let entry = DirEntry::new(fs_entry);

                        // Get file type
                        let file_type = match entry.file_type() {
                            Ok(ft) => ft,
                            Err(e) => return Some(Err(e)),
                        };

                        let walk_entry = WalkEntry::new(entry.path(), depth, file_type);

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
                    Some(Err(e)) => {
                        return Some(Err(FileError::from_io(e, dir_path)));
                    }
                    None => {
                        // Current directory exhausted, move to next
                        self.current = None;
                    }
                }
            }

            // Get next directory from queue
            match self.queue.pop_front() {
                Some((path, depth)) => {
                    match fs::read_dir(&path) {
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
}

// ============================================================================
// Glob Pattern Conversion
// ============================================================================

/// Converts a glob pattern to a regex pattern.
///
/// Supported glob syntax:
/// - `*` matches any sequence of characters except path separators
/// - `?` matches any single character except path separators
/// - `[abc]` matches any character in the brackets
/// - `[a-z]` matches any character in the range
/// - `[!abc]` or `[^abc]` matches any character not in the brackets
/// - `**` matches any sequence of characters including path separators
pub(crate) fn glob_to_regex(pattern: &str) -> FileResult<regex::Regex> {
    let mut regex = String::with_capacity(pattern.len() * 2);
    regex.push('^');

    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '*' => {
                // Check for **
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    // ** matches anything including path separators
                    regex.push_str(".*");
                    i += 2;
                } else {
                    // * matches anything except path separators
                    regex.push_str("[^/\\\\]*");
                    i += 1;
                }
            }
            '?' => {
                // ? matches any single character except path separators
                regex.push_str("[^/\\\\]");
                i += 1;
            }
            '[' => {
                // Character class
                regex.push('[');
                i += 1;

                // Handle negation
                if i < chars.len() && (chars[i] == '!' || chars[i] == '^') {
                    regex.push('^');
                    i += 1;
                }

                // Copy characters until ]
                while i < chars.len() && chars[i] != ']' {
                    let c = chars[i];
                    // Escape special regex characters inside character class
                    if c == '\\' || c == '^' || c == '-' {
                        regex.push('\\');
                    }
                    regex.push(c);
                    i += 1;
                }

                if i < chars.len() {
                    regex.push(']');
                    i += 1;
                }
            }
            '.' | '+' | '(' | ')' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                // Escape regex special characters
                regex.push('\\');
                regex.push(chars[i]);
                i += 1;
            }
            c => {
                regex.push(c);
                i += 1;
            }
        }
    }

    regex.push('$');

    regex::Regex::new(&regex).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            None,
            Some(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid glob pattern '{}': {}", pattern, e),
            )),
        )
    })
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Reads a directory and returns an iterator over its entries.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::read_dir;
///
/// for entry in read_dir("src")? {
///     let entry = entry?;
///     println!("{}", entry.name());
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the path does not exist, is not a directory,
/// or cannot be read.
pub fn read_dir(path: impl AsRef<Path>) -> FileResult<DirIterator> {
    let path = path.as_ref().to_path_buf();
    let inner = fs::read_dir(&path).map_err(|e| FileError::from_io(e, &path))?;
    Ok(DirIterator::new(path, inner))
}

/// Recursively reads a directory and returns an iterator over all entries.
///
/// This is a convenience function that creates a `WalkDir` with default options.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::read_dir_recursive;
///
/// for entry in read_dir_recursive("src")? {
///     let entry = entry?;
///     println!("{}: depth {}", entry.path().display(), entry.depth());
/// }
/// ```
pub fn read_dir_recursive(path: impl AsRef<Path>) -> FileResult<WalkDir> {
    WalkDir::new(path)
}

/// Creates a directory.
///
/// # Errors
///
/// Returns an error if:
/// - The parent directory does not exist
/// - A file or directory already exists at the path
/// - Permission is denied
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::create_dir;
///
/// create_dir("new_folder")?;
/// ```
pub fn create_dir(path: impl AsRef<Path>) -> FileResult<()> {
    let path = path.as_ref();
    fs::create_dir(path).map_err(|e| FileError::from_io(e, path))
}

/// Creates a directory and all parent directories.
///
/// This function will create all missing parent directories as needed.
/// If the directory already exists, this function succeeds silently.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::create_dir_all;
///
/// create_dir_all("path/to/nested/folder")?;
/// ```
pub fn create_dir_all(path: impl AsRef<Path>) -> FileResult<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).map_err(|e| FileError::from_io(e, path))
}

/// Removes an empty directory.
///
/// # Errors
///
/// Returns an error if:
/// - The directory does not exist
/// - The directory is not empty
/// - Permission is denied
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::remove_dir;
///
/// remove_dir("empty_folder")?;
/// ```
pub fn remove_dir(path: impl AsRef<Path>) -> FileResult<()> {
    let path = path.as_ref();
    fs::remove_dir(path).map_err(|e| FileError::from_io(e, path))
}

/// Removes a directory and all its contents.
///
/// This function recursively removes all files and subdirectories.
/// Use with caution as this operation cannot be undone.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::remove_dir_all;
///
/// remove_dir_all("folder_with_contents")?;
/// ```
pub fn remove_dir_all(path: impl AsRef<Path>) -> FileResult<()> {
    let path = path.as_ref();
    fs::remove_dir_all(path).map_err(|e| FileError::from_io(e, path))
}

/// Lists all entries in a directory as paths.
///
/// This is a convenience function that collects all directory entry paths.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::list_dir;
///
/// let entries = list_dir("src")?;
/// for path in entries {
///     println!("{}", path.display());
/// }
/// ```
pub fn list_dir(path: impl AsRef<Path>) -> FileResult<Vec<PathBuf>> {
    Ok(read_dir(path)?.collect_paths())
}

/// Lists all entries in a directory matching a glob pattern.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::list_dir_glob;
///
/// let rust_files = list_dir_glob("src", "*.rs")?;
/// ```
pub fn list_dir_glob(path: impl AsRef<Path>, pattern: &str) -> FileResult<Vec<PathBuf>> {
    Ok(read_dir(path)?
        .filter_glob(pattern)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect())
}

/// Checks if a directory is empty.
///
/// # Errors
///
/// Returns an error if the path does not exist or is not a directory.
pub fn is_dir_empty(path: impl AsRef<Path>) -> FileResult<bool> {
    let path = path.as_ref();
    let mut iter = fs::read_dir(path).map_err(|e| FileError::from_io(e, path))?;
    Ok(iter.next().is_none())
}

/// Calculates the total size of a directory recursively.
///
/// This includes all files in all subdirectories.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::dir_size;
///
/// let size = dir_size("my_folder")?;
/// println!("Total size: {} bytes", size);
/// ```
pub fn dir_size(path: impl AsRef<Path>) -> FileResult<u64> {
    let mut total = 0u64;

    for entry in WalkDir::with_options(&path, WalkDirOptions::new().files_only())? {
        if let Ok(mut entry) = entry
            && let Ok(size) = entry.size()
        {
            total += size;
        }
    }

    Ok(total)
}

/// Counts the number of entries in a directory.
///
/// # Arguments
///
/// * `path` - The directory path
/// * `recursive` - Whether to count entries in subdirectories
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::count_entries;
///
/// let count = count_entries("src", false)?;
/// println!("Direct entries: {}", count);
///
/// let total = count_entries("src", true)?;
/// println!("Total entries (recursive): {}", total);
/// ```
pub fn count_entries(path: impl AsRef<Path>, recursive: bool) -> FileResult<usize> {
    if recursive {
        Ok(WalkDir::new(&path)?.filter_map(Result::ok).count())
    } else {
        Ok(read_dir(&path)?.filter_map(Result::ok).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Creates a unique test directory with a standard structure.
    /// Returns the TempDir (which auto-cleans on drop) and the path.
    fn setup_test_dir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Create test structure
        fs::create_dir_all(test_dir.join("subdir1")).unwrap();
        fs::create_dir_all(test_dir.join("subdir2/nested")).unwrap();

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

    #[test]
    fn test_read_dir() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> = read_dir(&test_dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Should have: file1.txt, file2.rs, subdir1, subdir2, .hidden
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn test_read_dir_filtered() {
        let (_temp_dir, test_dir) = setup_test_dir();

        // Filter to only files
        let files: Vec<_> = read_dir(&test_dir)
            .unwrap()
            .filter_by(|e| e.is_file().unwrap_or(false))
            .filter_map(Result::ok)
            .collect();

        // Should have: file1.txt, file2.rs, .hidden
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_read_dir_glob() {
        let (_temp_dir, test_dir) = setup_test_dir();

        // Find .txt files
        let txt_files: Vec<_> = read_dir(&test_dir)
            .unwrap()
            .filter_glob("*.txt")
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert_eq!(txt_files.len(), 1);
        assert_eq!(txt_files[0].name(), "file1.txt");

        // Find .rs files
        let rs_files: Vec<_> = read_dir(&test_dir)
            .unwrap()
            .filter_glob("*.rs")
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert_eq!(rs_files.len(), 1);
        assert_eq!(rs_files[0].name(), "file2.rs");
    }

    #[test]
    fn test_walk_dir_recursive() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> = WalkDir::new(&test_dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Should include all files and directories recursively
        // subdir1, subdir2, subdir2/nested, file1.txt, file2.rs, .hidden,
        // subdir1/nested_file.txt, subdir2/nested/deep.txt
        assert!(entries.len() >= 8);
    }

    #[test]
    fn test_walk_dir_files_only() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let files: Vec<_> = WalkDir::with_options(&test_dir, WalkDirOptions::new().files_only())
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // All entries should be files
        assert!(files.iter().all(|e| e.is_file()));

        // Should have: file1.txt, file2.rs, .hidden, nested_file.txt, deep.txt
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn test_walk_dir_max_depth() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> = WalkDir::with_options(&test_dir, WalkDirOptions::new().max_depth(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Should not include deep nested files
        let paths: Vec<_> = entries.iter().map(|e| e.path().to_path_buf()).collect();
        assert!(!paths.iter().any(|p| p.ends_with("deep.txt")));
    }

    #[test]
    fn test_walk_dir_skip_hidden() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> =
            WalkDir::with_options(&test_dir, WalkDirOptions::new().skip_hidden(true))
                .unwrap()
                .filter_map(Result::ok)
                .collect();

        // Should not include .hidden
        let names: Vec<_> = entries.iter().map(|e| e.name()).collect();
        assert!(!names.contains(&".hidden".to_string()));
    }

    #[test]
    fn test_walk_dir_with_glob() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let txt_files: Vec<_> =
            WalkDir::with_options(&test_dir, WalkDirOptions::new().glob("*.txt"))
                .unwrap()
                .filter_map(Result::ok)
                .collect();

        // Should find file1.txt, nested_file.txt, deep.txt
        assert_eq!(txt_files.len(), 3);
        assert!(txt_files.iter().all(|e| e.name().ends_with(".txt")));
    }

    #[test]
    fn test_create_and_remove_dir() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("subdir");

        // Create directory
        create_dir(&test_dir).unwrap();
        assert!(test_dir.is_dir());

        // Remove directory
        remove_dir(&test_dir).unwrap();
        assert!(!test_dir.exists());
    }

    #[test]
    fn test_create_dir_all() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("nested/deeply");

        // Create nested directories
        create_dir_all(&test_dir).unwrap();
        assert!(test_dir.is_dir());
    }

    #[test]
    fn test_remove_dir_all() {
        let (temp_dir, test_dir) = setup_test_dir();

        // Verify it has contents
        assert!(!is_dir_empty(&test_dir).unwrap());

        // Remove all
        remove_dir_all(&test_dir).unwrap();
        assert!(!test_dir.exists());

        // Keep temp_dir alive until end
        drop(temp_dir);
    }

    #[test]
    fn test_is_dir_empty() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("empty_dir");
        fs::create_dir(&test_dir).unwrap();

        assert!(is_dir_empty(&test_dir).unwrap());

        // Add a file
        File::create(test_dir.join("file.txt")).unwrap();
        assert!(!is_dir_empty(&test_dir).unwrap());
    }

    #[test]
    fn test_dir_size() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let size = dir_size(&test_dir).unwrap();

        // Total size should be sum of all file contents
        // "hello" (5) + "fn main() {}" (12) + "nested content" (14) +
        // "deep content" (12) + "hidden" (6) = 49
        assert_eq!(size, 49);
    }

    #[test]
    fn test_count_entries() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let direct_count = count_entries(&test_dir, false).unwrap();
        assert_eq!(direct_count, 5); // file1.txt, file2.rs, subdir1, subdir2, .hidden

        let recursive_count = count_entries(&test_dir, true).unwrap();
        assert!(recursive_count >= 8); // All files and dirs
    }

    #[test]
    fn test_list_dir_glob() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let txt_files = list_dir_glob(&test_dir, "*.txt").unwrap();
        assert_eq!(txt_files.len(), 1);
    }

    #[test]
    fn test_glob_patterns() {
        // Test glob to regex conversion
        let pattern = glob_to_regex("*.rs").unwrap();
        assert!(pattern.is_match("main.rs"));
        assert!(pattern.is_match("lib.rs"));
        assert!(!pattern.is_match("main.txt"));

        let pattern = glob_to_regex("test?.txt").unwrap();
        assert!(pattern.is_match("test1.txt"));
        assert!(pattern.is_match("testA.txt"));
        assert!(!pattern.is_match("test12.txt"));

        let pattern = glob_to_regex("[abc].txt").unwrap();
        assert!(pattern.is_match("a.txt"));
        assert!(pattern.is_match("b.txt"));
        assert!(!pattern.is_match("d.txt"));
    }

    #[test]
    fn test_dir_entry_metadata() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> = read_dir(&test_dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        for entry in entries {
            // Should be able to get file type
            assert!(entry.file_type().is_ok());

            // Should be able to get metadata
            assert!(entry.metadata().is_ok());
        }
    }

    #[test]
    fn test_walk_entry_depth() {
        let (_temp_dir, test_dir) = setup_test_dir();

        let entries: Vec<_> = WalkDir::new(&test_dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        // Check depths are correct
        for entry in &entries {
            let path = entry.path();
            let relative = path.strip_prefix(&test_dir).unwrap();
            let expected_depth = relative.components().count();
            assert_eq!(entry.depth(), expected_depth);
        }
    }
}
