//! Error types for file operations.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Error type for file operations.
#[derive(Debug)]
pub struct FileError {
    /// The kind of error that occurred.
    kind: FileErrorKind,
    /// The path involved in the error, if any.
    path: Option<PathBuf>,
    /// The underlying source error, if any.
    source: Option<io::Error>,
}

/// The kind of file error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileErrorKind {
    /// File or directory not found.
    NotFound,
    /// Permission denied.
    PermissionDenied,
    /// File already exists (when creating exclusively).
    AlreadyExists,
    /// Invalid path or filename.
    InvalidPath,
    /// The path is a directory, not a file.
    IsDirectory,
    /// The path is a file, not a directory.
    NotDirectory,
    /// Device or resource is busy.
    ResourceBusy,
    /// No space left on device.
    NoSpace,
    /// Read-only filesystem.
    ReadOnly,
    /// Operation would block (for non-blocking I/O).
    WouldBlock,
    /// The operation was interrupted.
    Interrupted,
    /// Invalid data or encoding.
    InvalidData,
    /// The file is too large.
    TooLarge,
    /// An unknown or unclassified error occurred.
    Other,
}

impl FileError {
    /// Creates a new file error.
    pub fn new(kind: FileErrorKind, path: Option<PathBuf>, source: Option<io::Error>) -> Self {
        Self { kind, path, source }
    }

    /// Creates a file error from an I/O error and path.
    pub fn from_io(err: io::Error, path: impl Into<PathBuf>) -> Self {
        let kind = match err.kind() {
            io::ErrorKind::NotFound => FileErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => FileErrorKind::PermissionDenied,
            io::ErrorKind::AlreadyExists => FileErrorKind::AlreadyExists,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => {
                FileErrorKind::InvalidPath
            }
            io::ErrorKind::IsADirectory => FileErrorKind::IsDirectory,
            io::ErrorKind::NotADirectory => FileErrorKind::NotDirectory,
            io::ErrorKind::ResourceBusy => FileErrorKind::ResourceBusy,
            io::ErrorKind::StorageFull => FileErrorKind::NoSpace,
            io::ErrorKind::ReadOnlyFilesystem => FileErrorKind::ReadOnly,
            io::ErrorKind::WouldBlock => FileErrorKind::WouldBlock,
            io::ErrorKind::Interrupted => FileErrorKind::Interrupted,
            io::ErrorKind::InvalidData => FileErrorKind::InvalidData,
            io::ErrorKind::FileTooLarge => FileErrorKind::TooLarge,
            _ => FileErrorKind::Other,
        };
        Self {
            kind,
            path: Some(path.into()),
            source: Some(err),
        }
    }

    /// Creates a "not found" error for the given path.
    pub fn not_found(path: impl Into<PathBuf>) -> Self {
        Self::new(FileErrorKind::NotFound, Some(path.into()), None)
    }

    /// Creates a "permission denied" error for the given path.
    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        Self::new(FileErrorKind::PermissionDenied, Some(path.into()), None)
    }

    /// Creates an "invalid path" error for the given path.
    pub fn invalid_path(path: impl Into<PathBuf>) -> Self {
        Self::new(FileErrorKind::InvalidPath, Some(path.into()), None)
    }

    /// Creates an "invalid data" error with a custom message.
    pub fn invalid_data(message: &str) -> Self {
        Self::new(
            FileErrorKind::InvalidData,
            None,
            Some(io::Error::new(io::ErrorKind::InvalidData, message)),
        )
    }

    /// Returns the kind of error.
    pub fn kind(&self) -> FileErrorKind {
        self.kind
    }

    /// Returns the path involved in the error, if any.
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// Returns the underlying I/O error, if any.
    pub fn io_error(&self) -> Option<&io::Error> {
        self.source.as_ref()
    }

    /// Returns true if this error indicates the file was not found.
    pub fn is_not_found(&self) -> bool {
        self.kind == FileErrorKind::NotFound
    }

    /// Returns true if this error indicates permission was denied.
    pub fn is_permission_denied(&self) -> bool {
        self.kind == FileErrorKind::PermissionDenied
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", self.kind, path.display()),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl fmt::Display for FileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileErrorKind::NotFound => write!(f, "file not found"),
            FileErrorKind::PermissionDenied => write!(f, "permission denied"),
            FileErrorKind::AlreadyExists => write!(f, "file already exists"),
            FileErrorKind::InvalidPath => write!(f, "invalid path"),
            FileErrorKind::IsDirectory => write!(f, "is a directory"),
            FileErrorKind::NotDirectory => write!(f, "not a directory"),
            FileErrorKind::ResourceBusy => write!(f, "resource busy"),
            FileErrorKind::NoSpace => write!(f, "no space left on device"),
            FileErrorKind::ReadOnly => write!(f, "read-only filesystem"),
            FileErrorKind::WouldBlock => write!(f, "operation would block"),
            FileErrorKind::Interrupted => write!(f, "operation interrupted"),
            FileErrorKind::InvalidData => write!(f, "invalid data"),
            FileErrorKind::TooLarge => write!(f, "file too large"),
            FileErrorKind::Other => write!(f, "file error"),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> Self {
        let kind = match err.kind() {
            io::ErrorKind::NotFound => FileErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => FileErrorKind::PermissionDenied,
            io::ErrorKind::AlreadyExists => FileErrorKind::AlreadyExists,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => {
                FileErrorKind::InvalidPath
            }
            io::ErrorKind::IsADirectory => FileErrorKind::IsDirectory,
            io::ErrorKind::NotADirectory => FileErrorKind::NotDirectory,
            io::ErrorKind::ResourceBusy => FileErrorKind::ResourceBusy,
            io::ErrorKind::StorageFull => FileErrorKind::NoSpace,
            io::ErrorKind::ReadOnlyFilesystem => FileErrorKind::ReadOnly,
            io::ErrorKind::WouldBlock => FileErrorKind::WouldBlock,
            io::ErrorKind::Interrupted => FileErrorKind::Interrupted,
            io::ErrorKind::InvalidData => FileErrorKind::InvalidData,
            io::ErrorKind::FileTooLarge => FileErrorKind::TooLarge,
            _ => FileErrorKind::Other,
        };
        Self {
            kind,
            path: None,
            source: Some(err),
        }
    }
}

/// A specialized Result type for file operations.
pub type FileResult<T> = Result<T, FileError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FileError::not_found("/path/to/file.txt");
        assert_eq!(err.to_string(), "file not found: /path/to/file.txt");
    }

    #[test]
    fn test_error_kind() {
        let err = FileError::permission_denied("/restricted");
        assert_eq!(err.kind(), FileErrorKind::PermissionDenied);
        assert!(err.is_permission_denied());
        assert!(!err.is_not_found());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
        let file_err: FileError = io_err.into();
        assert_eq!(file_err.kind(), FileErrorKind::NotFound);
    }

    #[test]
    fn test_from_io_error_with_path() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "test");
        let file_err = FileError::from_io(io_err, "/path/to/file");
        assert_eq!(file_err.kind(), FileErrorKind::PermissionDenied);
        assert_eq!(
            file_err.path().map(|p| p.to_string_lossy().to_string()),
            Some("/path/to/file".to_string())
        );
    }
}
