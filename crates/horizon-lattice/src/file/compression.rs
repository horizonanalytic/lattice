//! File compression and archive support.
//!
//! This module provides compression and archive operations for Gzip, ZIP, and TAR formats.
//! All operations follow the same patterns as other file format modules.
//!
//! # Gzip Compression
//!
//! ```ignore
//! use horizon_lattice::file::compression::{compress_gzip, decompress_gzip, GzipOptions};
//!
//! // Compress data
//! let compressed = compress_gzip(b"Hello, world!")?;
//!
//! // Decompress data
//! let decompressed = decompress_gzip(&compressed)?;
//! assert_eq!(decompressed, b"Hello, world!");
//!
//! // Compress with custom level
//! let options = GzipOptions::new().level(CompressionLevel::Best);
//! let compressed = compress_gzip_with_options(b"data", &options)?;
//!
//! // File operations
//! compress_gzip_file("input.txt", "output.gz")?;
//! decompress_gzip_file("input.gz", "output.txt")?;
//! ```
//!
//! # ZIP Archives
//!
//! ```ignore
//! use horizon_lattice::file::compression::{create_zip, extract_zip, ZipOptions};
//!
//! // Create a ZIP archive from files
//! create_zip("archive.zip", &["file1.txt", "file2.txt"])?;
//!
//! // Extract a ZIP archive
//! extract_zip("archive.zip", "output_dir")?;
//!
//! // List contents of a ZIP archive
//! let entries = list_zip("archive.zip")?;
//! for entry in entries {
//!     println!("{}: {} bytes", entry.name, entry.size);
//! }
//! ```
//!
//! # TAR Archives
//!
//! ```ignore
//! use horizon_lattice::file::compression::{create_tar, extract_tar, TarOptions};
//!
//! // Create a TAR archive
//! create_tar("archive.tar", &["file1.txt", "dir/"])?;
//!
//! // Extract a TAR archive
//! extract_tar("archive.tar", "output_dir")?;
//!
//! // Create a gzipped TAR archive
//! create_tar_gz("archive.tar.gz", &["file1.txt", "dir/"])?;
//! extract_tar_gz("archive.tar.gz", "output_dir")?;
//! ```

use std::fs;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_bytes};

// ============================================================================
// Compression Level
// ============================================================================

/// Compression level for algorithms that support it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// No compression (fastest, largest output).
    None,
    /// Fast compression (speed over size).
    Fast,
    /// Default compression (balanced).
    #[default]
    Default,
    /// Best compression (size over speed).
    Best,
    /// Custom compression level (0-9 for most algorithms).
    Custom(u32),
}

impl CompressionLevel {
    /// Converts to flate2 compression level.
    fn to_flate2(self) -> flate2::Compression {
        match self {
            CompressionLevel::None => flate2::Compression::none(),
            CompressionLevel::Fast => flate2::Compression::fast(),
            CompressionLevel::Default => flate2::Compression::default(),
            CompressionLevel::Best => flate2::Compression::best(),
            CompressionLevel::Custom(n) => flate2::Compression::new(n.min(9)),
        }
    }
}

// ============================================================================
// Gzip Support
// ============================================================================

/// Configuration options for Gzip compression.
#[derive(Debug, Clone)]
pub struct GzipOptions {
    /// Compression level.
    level: CompressionLevel,
}

impl GzipOptions {
    /// Creates default Gzip options.
    pub fn new() -> Self {
        Self {
            level: CompressionLevel::Default,
        }
    }

    /// Sets the compression level.
    pub fn level(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }
}

impl Default for GzipOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Compresses data using Gzip.
///
/// # Example
///
/// ```ignore
/// let compressed = compress_gzip(b"Hello, world!")?;
/// ```
pub fn compress_gzip(data: &[u8]) -> FileResult<Vec<u8>> {
    compress_gzip_with_options(data, &GzipOptions::default())
}

/// Compresses data using Gzip with custom options.
pub fn compress_gzip_with_options(data: &[u8], options: &GzipOptions) -> FileResult<Vec<u8>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), options.level.to_flate2());
    encoder.write_all(data).map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            None,
            Some(e),
        )
    })?;
    encoder.finish().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            None,
            Some(e),
        )
    })
}

/// Decompresses Gzip data.
///
/// # Example
///
/// ```ignore
/// let decompressed = decompress_gzip(&compressed_data)?;
/// ```
pub fn decompress_gzip(data: &[u8]) -> FileResult<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            None,
            Some(e),
        )
    })?;
    Ok(result)
}

/// Compresses a file using Gzip.
///
/// # Example
///
/// ```ignore
/// compress_gzip_file("input.txt", "output.gz")?;
/// ```
pub fn compress_gzip_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> FileResult<()> {
    compress_gzip_file_with_options(input_path, output_path, &GzipOptions::default())
}

/// Compresses a file using Gzip with custom options.
pub fn compress_gzip_file_with_options(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    options: &GzipOptions,
) -> FileResult<()> {
    let data = read_bytes(&input_path)?;
    let compressed = compress_gzip_with_options(&data, options)?;
    atomic_write(&output_path, |writer| writer.write_all(&compressed))
}

/// Decompresses a Gzip file.
///
/// # Example
///
/// ```ignore
/// decompress_gzip_file("input.gz", "output.txt")?;
/// ```
pub fn decompress_gzip_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> FileResult<()> {
    let data = read_bytes(&input_path)?;
    let decompressed = decompress_gzip(&data)?;
    atomic_write(&output_path, |writer| writer.write_all(&decompressed))
}

/// Reads and decompresses a Gzip file, returning the decompressed bytes.
///
/// # Example
///
/// ```ignore
/// let data = read_gzip("data.gz")?;
/// ```
pub fn read_gzip(path: impl AsRef<Path>) -> FileResult<Vec<u8>> {
    let data = read_bytes(&path)?;
    decompress_gzip(&data).map_err(|e| {
        // Add path context to the error
        FileError::new(
            e.kind(),
            Some(path.as_ref().to_path_buf()),
            None,
        )
    })
}

/// Compresses and writes data to a Gzip file.
///
/// # Example
///
/// ```ignore
/// write_gzip("data.gz", b"Hello, world!")?;
/// ```
pub fn write_gzip(path: impl AsRef<Path>, data: &[u8]) -> FileResult<()> {
    write_gzip_with_options(path, data, &GzipOptions::default())
}

/// Compresses and writes data to a Gzip file with custom options.
pub fn write_gzip_with_options(
    path: impl AsRef<Path>,
    data: &[u8],
    options: &GzipOptions,
) -> FileResult<()> {
    let compressed = compress_gzip_with_options(data, options)?;
    atomic_write(&path, |writer| writer.write_all(&compressed))
}

// ============================================================================
// ZIP Archive Support
// ============================================================================

/// Configuration options for ZIP operations.
#[derive(Debug, Clone)]
pub struct ZipOptions {
    /// Compression level for files.
    level: CompressionLevel,
    /// Whether to preserve file permissions (Unix only).
    preserve_permissions: bool,
}

impl ZipOptions {
    /// Creates default ZIP options.
    pub fn new() -> Self {
        Self {
            level: CompressionLevel::Default,
            preserve_permissions: true,
        }
    }

    /// Sets the compression level.
    pub fn level(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets whether to preserve file permissions.
    pub fn preserve_permissions(mut self, preserve: bool) -> Self {
        self.preserve_permissions = preserve;
        self
    }
}

impl Default for ZipOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about an entry in a ZIP archive.
#[derive(Debug, Clone)]
pub struct ZipEntry {
    /// The name/path of the entry within the archive.
    pub name: String,
    /// The uncompressed size in bytes.
    pub size: u64,
    /// The compressed size in bytes.
    pub compressed_size: u64,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

/// Creates a ZIP archive from the specified files.
///
/// # Example
///
/// ```ignore
/// create_zip("archive.zip", &["file1.txt", "file2.txt"])?;
/// ```
pub fn create_zip<P, I, S>(output_path: P, paths: I) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    create_zip_with_options(output_path, paths, &ZipOptions::default())
}

/// Creates a ZIP archive with custom options.
pub fn create_zip_with_options<P, I, S>(
    output_path: P,
    paths: I,
    options: &ZipOptions,
) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let output_path = output_path.as_ref();
    let file = fs::File::create(output_path)
        .map_err(|e| FileError::from_io(e, output_path))?;

    let mut zip = zip::ZipWriter::new(file);
    let compression = match options.level {
        CompressionLevel::None => zip::CompressionMethod::Stored,
        _ => zip::CompressionMethod::Deflated,
    };

    let zip_options = zip::write::SimpleFileOptions::default()
        .compression_method(compression);

    for path in paths {
        let path = path.as_ref();
        add_path_to_zip(&mut zip, path, path, &zip_options, options.preserve_permissions)?;
    }

    zip.finish().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(output_path.to_path_buf()),
            Some(io::Error::new(io::ErrorKind::Other, e.to_string())),
        )
    })?;

    Ok(())
}

/// Recursively adds a path to the ZIP archive.
fn add_path_to_zip<W: Write + io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &Path,
    base_path: &Path,
    options: &zip::write::SimpleFileOptions,
    _preserve_permissions: bool,
) -> FileResult<()> {
    let name = path
        .strip_prefix(base_path.parent().unwrap_or(Path::new("")))
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();

    if path.is_dir() {
        // Add directory entry
        let dir_name = if name.ends_with('/') { name } else { format!("{}/", name) };
        zip.add_directory(&dir_name, *options).map_err(|e| {
            FileError::new(
                FileErrorKind::Other,
                Some(path.to_path_buf()),
                Some(io::Error::new(io::ErrorKind::Other, e.to_string())),
            )
        })?;

        // Recursively add contents
        let entries = fs::read_dir(path)
            .map_err(|e| FileError::from_io(e, path))?;

        for entry in entries {
            let entry = entry.map_err(|e| FileError::from_io(e, path))?;
            add_path_to_zip(zip, &entry.path(), base_path, options, _preserve_permissions)?;
        }
    } else {
        // Add file
        zip.start_file(&name, *options).map_err(|e| {
            FileError::new(
                FileErrorKind::Other,
                Some(path.to_path_buf()),
                Some(io::Error::new(io::ErrorKind::Other, e.to_string())),
            )
        })?;

        let data = fs::read(path)
            .map_err(|e| FileError::from_io(e, path))?;
        zip.write_all(&data).map_err(|e| {
            FileError::new(
                FileErrorKind::Other,
                Some(path.to_path_buf()),
                Some(e),
            )
        })?;
    }

    Ok(())
}

/// Extracts a ZIP archive to the specified directory.
///
/// # Example
///
/// ```ignore
/// extract_zip("archive.zip", "output_dir")?;
/// ```
pub fn extract_zip(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> FileResult<()> {
    extract_zip_with_options(archive_path, output_dir, &ZipOptions::default())
}

/// Extracts a ZIP archive with custom options.
pub fn extract_zip_with_options(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    _options: &ZipOptions,
) -> FileResult<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let mut archive = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(archive_path.to_path_buf()),
            Some(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)
        .map_err(|e| FileError::from_io(e, output_dir))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(archive_path.to_path_buf()),
                Some(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
            )
        })?;

        // Sanitize the file name to prevent path traversal
        let name = file.enclosed_name()
            .ok_or_else(|| FileError::invalid_data("Invalid file name in ZIP archive"))?;

        let out_path = output_dir.join(name);

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| FileError::from_io(e, &out_path))?;
        } else {
            // Create parent directories if needed
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| FileError::from_io(e, parent))?;
            }

            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| FileError::from_io(e, &out_path))?;

            io::copy(&mut file, &mut out_file).map_err(|e| {
                FileError::new(
                    FileErrorKind::Other,
                    Some(out_path.clone()),
                    Some(e),
                )
            })?;
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&out_path, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    Ok(())
}

/// Lists the contents of a ZIP archive.
///
/// # Example
///
/// ```ignore
/// let entries = list_zip("archive.zip")?;
/// for entry in entries {
///     println!("{}: {} bytes", entry.name, entry.size);
/// }
/// ```
pub fn list_zip(archive_path: impl AsRef<Path>) -> FileResult<Vec<ZipEntry>> {
    let archive_path = archive_path.as_ref();
    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let mut archive = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(archive_path.to_path_buf()),
            Some(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let file = archive.by_index_raw(i).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(archive_path.to_path_buf()),
                Some(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
            )
        })?;

        entries.push(ZipEntry {
            name: file.name().to_string(),
            size: file.size(),
            compressed_size: file.compressed_size(),
            is_dir: file.is_dir(),
        });
    }

    Ok(entries)
}

// ============================================================================
// TAR Archive Support
// ============================================================================

/// Configuration options for TAR operations.
#[derive(Debug, Clone)]
pub struct TarOptions {
    /// Whether to preserve file permissions.
    preserve_permissions: bool,
    /// Whether to follow symlinks when creating archives.
    follow_symlinks: bool,
}

impl TarOptions {
    /// Creates default TAR options.
    pub fn new() -> Self {
        Self {
            preserve_permissions: true,
            follow_symlinks: false,
        }
    }

    /// Sets whether to preserve file permissions.
    pub fn preserve_permissions(mut self, preserve: bool) -> Self {
        self.preserve_permissions = preserve;
        self
    }

    /// Sets whether to follow symlinks.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }
}

impl Default for TarOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about an entry in a TAR archive.
#[derive(Debug, Clone)]
pub struct TarEntry {
    /// The name/path of the entry within the archive.
    pub name: String,
    /// The size in bytes.
    pub size: u64,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Whether this entry is a symlink.
    pub is_symlink: bool,
}

/// Creates a TAR archive from the specified files.
///
/// # Example
///
/// ```ignore
/// create_tar("archive.tar", &["file1.txt", "file2.txt"])?;
/// ```
pub fn create_tar<P, I, S>(output_path: P, paths: I) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    create_tar_with_options(output_path, paths, &TarOptions::default())
}

/// Creates a TAR archive with custom options.
pub fn create_tar_with_options<P, I, S>(
    output_path: P,
    paths: I,
    options: &TarOptions,
) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let output_path = output_path.as_ref();
    let file = fs::File::create(output_path)
        .map_err(|e| FileError::from_io(e, output_path))?;

    let mut builder = tar::Builder::new(file);
    builder.follow_symlinks(options.follow_symlinks);

    for path in paths {
        let path = path.as_ref();
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        if path.is_dir() {
            builder.append_dir_all(&name, path).map_err(|e| {
                FileError::new(
                    FileErrorKind::Other,
                    Some(path.to_path_buf()),
                    Some(e),
                )
            })?;
        } else {
            builder.append_path_with_name(path, &name).map_err(|e| {
                FileError::new(
                    FileErrorKind::Other,
                    Some(path.to_path_buf()),
                    Some(e),
                )
            })?;
        }
    }

    builder.finish().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(output_path.to_path_buf()),
            Some(e),
        )
    })?;

    Ok(())
}

/// Extracts a TAR archive to the specified directory.
///
/// # Example
///
/// ```ignore
/// extract_tar("archive.tar", "output_dir")?;
/// ```
pub fn extract_tar(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> FileResult<()> {
    extract_tar_with_options(archive_path, output_dir, &TarOptions::default())
}

/// Extracts a TAR archive with custom options.
pub fn extract_tar_with_options(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    options: &TarOptions,
) -> FileResult<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let mut archive = tar::Archive::new(BufReader::new(file));
    archive.set_preserve_permissions(options.preserve_permissions);

    archive.unpack(output_dir).map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(archive_path.to_path_buf()),
            Some(e),
        )
    })?;

    Ok(())
}

/// Lists the contents of a TAR archive.
///
/// # Example
///
/// ```ignore
/// let entries = list_tar("archive.tar")?;
/// for entry in entries {
///     println!("{}: {} bytes", entry.name, entry.size);
/// }
/// ```
pub fn list_tar(archive_path: impl AsRef<Path>) -> FileResult<Vec<TarEntry>> {
    let archive_path = archive_path.as_ref();
    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let mut archive = tar::Archive::new(BufReader::new(file));
    let mut entries = Vec::new();

    for entry in archive.entries().map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(archive_path.to_path_buf()),
            Some(e),
        )
    })? {
        let entry = entry.map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(archive_path.to_path_buf()),
                Some(e),
            )
        })?;

        let header = entry.header();
        entries.push(TarEntry {
            name: entry.path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size: header.size().unwrap_or(0),
            is_dir: header.entry_type().is_dir(),
            is_symlink: header.entry_type().is_symlink(),
        });
    }

    Ok(entries)
}

// ============================================================================
// Gzipped TAR (.tar.gz / .tgz) Support
// ============================================================================

/// Creates a gzipped TAR archive from the specified files.
///
/// # Example
///
/// ```ignore
/// create_tar_gz("archive.tar.gz", &["file1.txt", "dir/"])?;
/// ```
pub fn create_tar_gz<P, I, S>(output_path: P, paths: I) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    create_tar_gz_with_options(output_path, paths, &TarOptions::default(), &GzipOptions::default())
}

/// Creates a gzipped TAR archive with custom options.
pub fn create_tar_gz_with_options<P, I, S>(
    output_path: P,
    paths: I,
    tar_options: &TarOptions,
    gzip_options: &GzipOptions,
) -> FileResult<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let output_path = output_path.as_ref();
    let file = fs::File::create(output_path)
        .map_err(|e| FileError::from_io(e, output_path))?;

    let encoder = flate2::write::GzEncoder::new(file, gzip_options.level.to_flate2());
    let mut builder = tar::Builder::new(encoder);
    builder.follow_symlinks(tar_options.follow_symlinks);

    for path in paths {
        let path = path.as_ref();
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        if path.is_dir() {
            builder.append_dir_all(&name, path).map_err(|e| {
                FileError::new(
                    FileErrorKind::Other,
                    Some(path.to_path_buf()),
                    Some(e),
                )
            })?;
        } else {
            builder.append_path_with_name(path, &name).map_err(|e| {
                FileError::new(
                    FileErrorKind::Other,
                    Some(path.to_path_buf()),
                    Some(e),
                )
            })?;
        }
    }

    let encoder = builder.into_inner().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(output_path.to_path_buf()),
            Some(e),
        )
    })?;

    encoder.finish().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(output_path.to_path_buf()),
            Some(e),
        )
    })?;

    Ok(())
}

/// Extracts a gzipped TAR archive to the specified directory.
///
/// # Example
///
/// ```ignore
/// extract_tar_gz("archive.tar.gz", "output_dir")?;
/// ```
pub fn extract_tar_gz(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> FileResult<()> {
    extract_tar_gz_with_options(archive_path, output_dir, &TarOptions::default())
}

/// Extracts a gzipped TAR archive with custom options.
pub fn extract_tar_gz_with_options(
    archive_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    options: &TarOptions,
) -> FileResult<()> {
    let archive_path = archive_path.as_ref();
    let output_dir = output_dir.as_ref();

    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(options.preserve_permissions);

    archive.unpack(output_dir).map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            Some(archive_path.to_path_buf()),
            Some(e),
        )
    })?;

    Ok(())
}

/// Lists the contents of a gzipped TAR archive.
///
/// # Example
///
/// ```ignore
/// let entries = list_tar_gz("archive.tar.gz")?;
/// ```
pub fn list_tar_gz(archive_path: impl AsRef<Path>) -> FileResult<Vec<TarEntry>> {
    let archive_path = archive_path.as_ref();
    let file = fs::File::open(archive_path)
        .map_err(|e| FileError::from_io(e, archive_path))?;

    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder);
    let mut entries = Vec::new();

    for entry in archive.entries().map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(archive_path.to_path_buf()),
            Some(e),
        )
    })? {
        let entry = entry.map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(archive_path.to_path_buf()),
                Some(e),
            )
        })?;

        let header = entry.header();
        entries.push(TarEntry {
            name: entry.path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            size: header.size().unwrap_or(0),
            is_dir: header.entry_type().is_dir(),
            is_symlink: header.entry_type().is_symlink(),
        });
    }

    Ok(entries)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("horizon_compression_test_{}", name))
    }

    fn cleanup(path: &std::path::Path) {
        if path.is_dir() {
            fs::remove_dir_all(path).ok();
        } else {
            fs::remove_file(path).ok();
        }
    }

    // ========================================================================
    // Gzip Tests
    // ========================================================================

    #[test]
    fn test_gzip_compress_decompress() {
        let data = b"Hello, world! This is test data for compression.";
        let compressed = compress_gzip(data).unwrap();

        // Compressed should be non-empty
        assert!(!compressed.is_empty());

        let decompressed = decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_compression_levels() {
        let data = b"Hello, world! ".repeat(100);

        let none = compress_gzip_with_options(&data, &GzipOptions::new().level(CompressionLevel::None)).unwrap();
        let fast = compress_gzip_with_options(&data, &GzipOptions::new().level(CompressionLevel::Fast)).unwrap();
        let best = compress_gzip_with_options(&data, &GzipOptions::new().level(CompressionLevel::Best)).unwrap();

        // Best compression should produce smallest output
        assert!(best.len() <= fast.len());
        // No compression should produce largest output
        assert!(none.len() >= fast.len());

        // All should decompress correctly
        assert_eq!(decompress_gzip(&none).unwrap(), data);
        assert_eq!(decompress_gzip(&fast).unwrap(), data);
        assert_eq!(decompress_gzip(&best).unwrap(), data);
    }

    #[test]
    fn test_gzip_empty_data() {
        let data = b"";
        let compressed = compress_gzip(data).unwrap();
        let decompressed = decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_file_operations() {
        let input_path = temp_path("gzip_input.txt");
        let compressed_path = temp_path("gzip_output.gz");
        let output_path = temp_path("gzip_restored.txt");
        cleanup(&input_path);
        cleanup(&compressed_path);
        cleanup(&output_path);

        let test_data = b"File compression test data";
        fs::write(&input_path, test_data).unwrap();

        compress_gzip_file(&input_path, &compressed_path).unwrap();
        assert!(compressed_path.exists());

        decompress_gzip_file(&compressed_path, &output_path).unwrap();
        assert!(output_path.exists());

        let restored = fs::read(&output_path).unwrap();
        assert_eq!(restored, test_data);

        cleanup(&input_path);
        cleanup(&compressed_path);
        cleanup(&output_path);
    }

    #[test]
    fn test_read_write_gzip() {
        let path = temp_path("rw_gzip.gz");
        cleanup(&path);

        let data = b"Read/write gzip test";
        write_gzip(&path, data).unwrap();

        let read_back = read_gzip(&path).unwrap();
        assert_eq!(read_back, data);

        cleanup(&path);
    }

    #[test]
    fn test_decompress_invalid_data() {
        let invalid_data = b"not valid gzip data";
        let result = decompress_gzip(invalid_data);
        assert!(result.is_err());
    }

    // ========================================================================
    // ZIP Tests
    // ========================================================================

    #[test]
    fn test_zip_create_extract() {
        let test_dir = temp_path("zip_test_dir");
        let archive_path = temp_path("test.zip");
        let extract_dir = temp_path("zip_extract_dir");
        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);

        // Create test files
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(test_dir.join("file1.txt"), "Content 1").unwrap();
        fs::write(test_dir.join("file2.txt"), "Content 2").unwrap();

        // Create archive
        create_zip(&archive_path, &[&test_dir]).unwrap();
        assert!(archive_path.exists());

        // Extract archive
        extract_zip(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.exists());

        // Verify contents
        let dir_name = test_dir.file_name().unwrap().to_string_lossy();
        let file1_content = fs::read_to_string(extract_dir.join(&*dir_name).join("file1.txt")).unwrap();
        let file2_content = fs::read_to_string(extract_dir.join(&*dir_name).join("file2.txt")).unwrap();
        assert_eq!(file1_content, "Content 1");
        assert_eq!(file2_content, "Content 2");

        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);
    }

    #[test]
    fn test_zip_list() {
        let test_dir = temp_path("zip_list_test_dir");
        let archive_path = temp_path("list_test.zip");
        cleanup(&test_dir);
        cleanup(&archive_path);

        // Create test files
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(test_dir.join("file.txt"), "Content").unwrap();

        // Create archive
        create_zip(&archive_path, &[&test_dir]).unwrap();

        // List contents
        let entries = list_zip(&archive_path).unwrap();
        assert!(!entries.is_empty());

        // Should have directory and file entries
        let has_dir = entries.iter().any(|e| e.is_dir);
        let has_file = entries.iter().any(|e| !e.is_dir && e.name.ends_with("file.txt"));
        assert!(has_dir || has_file);

        cleanup(&test_dir);
        cleanup(&archive_path);
    }

    #[test]
    fn test_zip_single_file() {
        let file_path = temp_path("single_zip_file.txt");
        let archive_path = temp_path("single.zip");
        let extract_dir = temp_path("single_zip_extract");
        cleanup(&file_path);
        cleanup(&archive_path);
        cleanup(&extract_dir);

        fs::write(&file_path, "Single file content").unwrap();

        create_zip(&archive_path, &[&file_path]).unwrap();
        extract_zip(&archive_path, &extract_dir).unwrap();

        // The extracted file name includes the full temp_path prefix
        let file_name = file_path.file_name().unwrap();
        let extracted_content = fs::read_to_string(extract_dir.join(file_name)).unwrap();
        assert_eq!(extracted_content, "Single file content");

        cleanup(&file_path);
        cleanup(&archive_path);
        cleanup(&extract_dir);
    }

    // ========================================================================
    // TAR Tests
    // ========================================================================

    #[test]
    fn test_tar_create_extract() {
        let test_dir = temp_path("tar_test_dir");
        let archive_path = temp_path("test.tar");
        let extract_dir = temp_path("tar_extract_dir");
        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);

        // Create test files
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(test_dir.join("file1.txt"), "TAR Content 1").unwrap();
        fs::write(test_dir.join("file2.txt"), "TAR Content 2").unwrap();

        // Create archive
        create_tar(&archive_path, &[&test_dir]).unwrap();
        assert!(archive_path.exists());

        // Extract archive
        extract_tar(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.exists());

        // Verify contents
        let dir_name = test_dir.file_name().unwrap().to_string_lossy();
        let file1_content = fs::read_to_string(extract_dir.join(&*dir_name).join("file1.txt")).unwrap();
        let file2_content = fs::read_to_string(extract_dir.join(&*dir_name).join("file2.txt")).unwrap();
        assert_eq!(file1_content, "TAR Content 1");
        assert_eq!(file2_content, "TAR Content 2");

        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);
    }

    #[test]
    fn test_tar_list() {
        let file_path = temp_path("tar_list_file.txt");
        let archive_path = temp_path("list_test.tar");
        cleanup(&file_path);
        cleanup(&archive_path);

        fs::write(&file_path, "TAR list content").unwrap();

        create_tar(&archive_path, &[&file_path]).unwrap();

        let entries = list_tar(&archive_path).unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.contains("tar_list_file.txt")));

        cleanup(&file_path);
        cleanup(&archive_path);
    }

    // ========================================================================
    // TAR.GZ Tests
    // ========================================================================

    #[test]
    fn test_tar_gz_create_extract() {
        let test_dir = temp_path("tar_gz_test_dir");
        let archive_path = temp_path("test.tar.gz");
        let extract_dir = temp_path("tar_gz_extract_dir");
        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);

        // Create test files
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(test_dir.join("file1.txt"), "TAR.GZ Content 1").unwrap();
        fs::create_dir_all(test_dir.join("subdir")).unwrap();
        fs::write(test_dir.join("subdir").join("nested.txt"), "Nested content").unwrap();

        // Create archive
        create_tar_gz(&archive_path, &[&test_dir]).unwrap();
        assert!(archive_path.exists());

        // Extract archive
        extract_tar_gz(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.exists());

        // Verify contents
        let dir_name = test_dir.file_name().unwrap().to_string_lossy();
        let file1_content = fs::read_to_string(extract_dir.join(&*dir_name).join("file1.txt")).unwrap();
        let nested_content = fs::read_to_string(
            extract_dir.join(&*dir_name).join("subdir").join("nested.txt")
        ).unwrap();
        assert_eq!(file1_content, "TAR.GZ Content 1");
        assert_eq!(nested_content, "Nested content");

        cleanup(&test_dir);
        cleanup(&archive_path);
        cleanup(&extract_dir);
    }

    #[test]
    fn test_tar_gz_list() {
        let file_path = temp_path("tar_gz_list_file.txt");
        let archive_path = temp_path("list_test.tar.gz");
        cleanup(&file_path);
        cleanup(&archive_path);

        fs::write(&file_path, "TAR.GZ list content").unwrap();

        create_tar_gz(&archive_path, &[&file_path]).unwrap();

        let entries = list_tar_gz(&archive_path).unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.name.contains("tar_gz_list_file.txt")));

        cleanup(&file_path);
        cleanup(&archive_path);
    }

    #[test]
    fn test_tar_gz_compression_effective() {
        let file_path = temp_path("large_file.txt");
        let tar_path = temp_path("large.tar");
        let tar_gz_path = temp_path("large.tar.gz");
        cleanup(&file_path);
        cleanup(&tar_path);
        cleanup(&tar_gz_path);

        // Create a file with repetitive content (compresses well)
        let content = "Repetitive content! ".repeat(1000);
        fs::write(&file_path, &content).unwrap();

        create_tar(&tar_path, &[&file_path]).unwrap();
        create_tar_gz(&tar_gz_path, &[&file_path]).unwrap();

        let tar_size = fs::metadata(&tar_path).unwrap().len();
        let tar_gz_size = fs::metadata(&tar_gz_path).unwrap().len();

        // Gzipped tar should be smaller
        assert!(tar_gz_size < tar_size);

        cleanup(&file_path);
        cleanup(&tar_path);
        cleanup(&tar_gz_path);
    }

    // ========================================================================
    // Options Tests
    // ========================================================================

    #[test]
    fn test_gzip_options_builder() {
        let options = GzipOptions::new()
            .level(CompressionLevel::Best);

        assert_eq!(options.level, CompressionLevel::Best);
    }

    #[test]
    fn test_zip_options_builder() {
        let options = ZipOptions::new()
            .level(CompressionLevel::Fast)
            .preserve_permissions(false);

        assert_eq!(options.level, CompressionLevel::Fast);
        assert!(!options.preserve_permissions);
    }

    #[test]
    fn test_tar_options_builder() {
        let options = TarOptions::new()
            .preserve_permissions(false)
            .follow_symlinks(true);

        assert!(!options.preserve_permissions);
        assert!(options.follow_symlinks);
    }
}
