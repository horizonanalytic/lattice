//! File information and metadata.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::error::{FileError, FileResult};

/// Represents the type of a file system entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link.
    Symlink,
    /// An unknown or special file type.
    Other,
}

impl FileType {
    /// Returns true if this is a regular file.
    pub fn is_file(&self) -> bool {
        matches!(self, FileType::File)
    }

    /// Returns true if this is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    /// Returns true if this is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }
}

impl From<fs::FileType> for FileType {
    fn from(ft: fs::FileType) -> Self {
        if ft.is_file() {
            FileType::File
        } else if ft.is_dir() {
            FileType::Directory
        } else if ft.is_symlink() {
            FileType::Symlink
        } else {
            FileType::Other
        }
    }
}

/// File permissions information.
///
/// This provides a cross-platform abstraction over file permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// Whether the file is read-only.
    readonly: bool,
    /// Unix mode bits (only meaningful on Unix systems).
    #[cfg(unix)]
    mode: u32,
}

impl Permissions {
    /// Returns true if the file is read-only.
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// Returns the Unix mode bits.
    ///
    /// On non-Unix systems, this returns a default value (0o644 for files, 0o755 for directories).
    #[cfg(unix)]
    pub fn mode(&self) -> u32 {
        self.mode
    }

    /// Returns the Unix mode bits.
    ///
    /// On non-Unix systems, this returns a default value (0o644 for files, 0o755 for directories).
    #[cfg(not(unix))]
    pub fn mode(&self) -> u32 {
        if self.readonly {
            0o444
        } else {
            0o644
        }
    }

    /// Returns true if the owner can read the file (Unix).
    pub fn owner_can_read(&self) -> bool {
        self.mode() & 0o400 != 0
    }

    /// Returns true if the owner can write the file (Unix).
    pub fn owner_can_write(&self) -> bool {
        self.mode() & 0o200 != 0
    }

    /// Returns true if the owner can execute the file (Unix).
    pub fn owner_can_execute(&self) -> bool {
        self.mode() & 0o100 != 0
    }

    /// Returns true if the group can read the file (Unix).
    pub fn group_can_read(&self) -> bool {
        self.mode() & 0o040 != 0
    }

    /// Returns true if the group can write the file (Unix).
    pub fn group_can_write(&self) -> bool {
        self.mode() & 0o020 != 0
    }

    /// Returns true if the group can execute the file (Unix).
    pub fn group_can_execute(&self) -> bool {
        self.mode() & 0o010 != 0
    }

    /// Returns true if others can read the file (Unix).
    pub fn others_can_read(&self) -> bool {
        self.mode() & 0o004 != 0
    }

    /// Returns true if others can write the file (Unix).
    pub fn others_can_write(&self) -> bool {
        self.mode() & 0o002 != 0
    }

    /// Returns true if others can execute the file (Unix).
    pub fn others_can_execute(&self) -> bool {
        self.mode() & 0o001 != 0
    }
}

impl From<fs::Permissions> for Permissions {
    fn from(perms: fs::Permissions) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            Self {
                readonly: perms.readonly(),
                mode: perms.mode(),
            }
        }
        #[cfg(not(unix))]
        {
            Self {
                readonly: perms.readonly(),
            }
        }
    }
}

/// Information about a file or directory.
///
/// This struct provides cached file metadata, avoiding repeated system calls
/// when accessing multiple attributes.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::FileInfo;
///
/// let info = FileInfo::new("document.txt")?;
/// println!("Size: {} bytes", info.size());
/// println!("Type: {:?}", info.file_type());
/// if let Some(modified) = info.modified() {
///     println!("Modified: {:?}", modified);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// The path to the file.
    path: PathBuf,
    /// Cached metadata.
    metadata: fs::Metadata,
}

impl FileInfo {
    /// Creates a new `FileInfo` for the given path.
    ///
    /// This follows symbolic links. Use `new_no_follow` to get info about
    /// the symlink itself rather than its target.
    ///
    /// # Errors
    ///
    /// Returns an error if the file does not exist or cannot be accessed.
    pub fn new(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = fs::metadata(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self { path, metadata })
    }

    /// Creates a new `FileInfo` without following symbolic links.
    ///
    /// If the path is a symlink, this returns information about the symlink
    /// itself, not its target.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or cannot be accessed.
    pub fn new_no_follow(path: impl AsRef<Path>) -> FileResult<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = fs::symlink_metadata(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Self { path, metadata })
    }

    /// Returns the path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file type.
    pub fn file_type(&self) -> FileType {
        self.metadata.file_type().into()
    }

    /// Returns true if this is a regular file.
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    /// Returns true if this is a directory.
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    /// Returns true if this is a symbolic link.
    ///
    /// Note: This only returns true if `FileInfo::new_no_follow` was used.
    /// When using `FileInfo::new`, symlinks are followed automatically.
    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }

    /// Returns the size of the file in bytes.
    ///
    /// For directories, this returns the size of the directory entry,
    /// not the total size of its contents.
    pub fn size(&self) -> u64 {
        self.metadata.len()
    }

    /// Returns the file permissions.
    pub fn permissions(&self) -> Permissions {
        self.metadata.permissions().into()
    }

    /// Returns true if the file is read-only.
    pub fn is_readonly(&self) -> bool {
        self.metadata.permissions().readonly()
    }

    /// Returns true if the file is readable by the current user.
    ///
    /// This performs an access check beyond the file permissions.
    pub fn is_readable(&self) -> bool {
        // Try to open for reading
        fs::File::open(&self.path).is_ok()
    }

    /// Returns true if the file is writable by the current user.
    ///
    /// This performs an access check beyond the file permissions.
    pub fn is_writable(&self) -> bool {
        // Try to open for writing (append mode to avoid truncating)
        fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.path)
            .is_ok()
    }

    /// Returns true if the file is executable by the current user.
    ///
    /// On Windows, this checks if the file has an executable extension.
    /// On Unix, this checks the execute permission bits.
    #[cfg(unix)]
    pub fn is_executable(&self) -> bool {
        use std::os::unix::fs::PermissionsExt;
        self.metadata.permissions().mode() & 0o111 != 0
    }

    /// Returns true if the file is executable by the current user.
    #[cfg(not(unix))]
    pub fn is_executable(&self) -> bool {
        // On Windows, check file extension
        if let Some(ext) = self.path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com" | "ps1")
        } else {
            false
        }
    }

    /// Returns the time the file was last modified.
    ///
    /// Returns `None` if the modification time is not available.
    pub fn modified(&self) -> Option<SystemTime> {
        self.metadata.modified().ok()
    }

    /// Returns the time the file was last accessed.
    ///
    /// Returns `None` if the access time is not available.
    pub fn accessed(&self) -> Option<SystemTime> {
        self.metadata.accessed().ok()
    }

    /// Returns the time the file was created.
    ///
    /// Returns `None` if the creation time is not available (e.g., on some Unix systems).
    pub fn created(&self) -> Option<SystemTime> {
        self.metadata.created().ok()
    }

    /// Refreshes the cached metadata from disk.
    ///
    /// Call this if you need updated information after the file may have changed.
    pub fn refresh(&mut self) -> FileResult<()> {
        self.metadata = fs::metadata(&self.path).map_err(|e| FileError::from_io(e, &self.path))?;
        Ok(())
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Returns true if the given path exists.
///
/// This follows symbolic links. Use `exists_no_follow` to check the symlink itself.
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Returns true if the given path exists without following symbolic links.
pub fn exists_no_follow(path: impl AsRef<Path>) -> bool {
    fs::symlink_metadata(path.as_ref()).is_ok()
}

/// Returns true if the given path is a file.
pub fn is_file(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

/// Returns true if the given path is a directory.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

/// Returns true if the given path is a symbolic link.
pub fn is_symlink(path: impl AsRef<Path>) -> bool {
    fs::symlink_metadata(path.as_ref())
        .map(|m| m.is_symlink())
        .unwrap_or(false)
}

/// Returns the size of the file in bytes.
///
/// # Errors
///
/// Returns an error if the file does not exist or cannot be accessed.
pub fn file_size(path: impl AsRef<Path>) -> FileResult<u64> {
    let path = path.as_ref();
    fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| FileError::from_io(e, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_file_type() {
        assert!(FileType::File.is_file());
        assert!(!FileType::File.is_directory());
        assert!(FileType::Directory.is_directory());
        assert!(FileType::Symlink.is_symlink());
    }

    #[test]
    fn test_file_info() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("horizon_test_file_info.txt");

        // Create a test file
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        drop(file);

        // Test FileInfo
        let info = FileInfo::new(&test_file).unwrap();
        assert!(info.is_file());
        assert!(!info.is_dir());
        assert_eq!(info.size(), 13);
        assert!(info.is_readable());

        // Cleanup
        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_exists() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("horizon_test_exists.txt");

        assert!(!exists(&test_file));

        fs::File::create(&test_file).unwrap();
        assert!(exists(&test_file));
        assert!(is_file(&test_file));
        assert!(!is_dir(&test_file));

        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_file_size() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("horizon_test_file_size.txt");

        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"12345").unwrap();
        drop(file);

        assert_eq!(file_size(&test_file).unwrap(), 5);

        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_file_info_dir() {
        let temp_dir = std::env::temp_dir();
        let info = FileInfo::new(&temp_dir).unwrap();
        assert!(info.is_dir());
        assert!(!info.is_file());
    }

    #[test]
    fn test_permissions() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("horizon_test_permissions.txt");

        fs::File::create(&test_file).unwrap();

        let info = FileInfo::new(&test_file).unwrap();
        let perms = info.permissions();

        // Most systems create files that are owner-readable
        assert!(perms.owner_can_read());

        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_not_found() {
        let result = FileInfo::new("/nonexistent/path/file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }
}
