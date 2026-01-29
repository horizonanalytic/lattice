//! File I/O operations and utilities.
//!
//! This module provides cross-platform file operations for reading, writing,
//! and querying file metadata. It wraps Rust's standard library file APIs
//! with ergonomic conveniences suitable for GUI applications.
//!
//! # Reading Files
//!
//! ```ignore
//! use horizon_lattice::file::{File, read_text, read_bytes};
//!
//! // Read entire file as text
//! let content = read_text("config.txt")?;
//!
//! // Read entire file as bytes
//! let bytes = read_bytes("data.bin")?;
//!
//! // Read file line by line
//! for line in File::open("log.txt")?.lines() {
//!     println!("{}", line?);
//! }
//!
//! // Read file in chunks
//! let mut file = File::open("large.bin")?;
//! let mut buffer = [0u8; 4096];
//! while let Some(bytes_read) = file.read_chunk(&mut buffer)? {
//!     // Process chunk...
//! }
//! ```
//!
//! # Writing Files
//!
//! ```ignore
//! use horizon_lattice::file::{File, write_text, write_bytes};
//!
//! // Write entire file
//! write_text("output.txt", "Hello, world!")?;
//! write_bytes("data.bin", &[0x00, 0x01, 0x02])?;
//!
//! // Append to file
//! let mut file = File::append("log.txt")?;
//! file.write_all(b"New log entry\n")?;
//!
//! // Atomic write (safe for config files)
//! File::atomic_write("config.json", |f| {
//!     f.write_all(b"{\"key\": \"value\"}")
//! })?;
//! ```
//!
//! # Memory-Mapped Files
//!
//! ```ignore
//! use horizon_lattice::file::{MappedFile, MappedFileMut, MmapOptions};
//!
//! // Read-only memory mapping (zero-copy access)
//! let mapped = MappedFile::open("large_file.bin")?;
//! let first_byte = mapped[0];
//! let slice = &mapped[100..200];
//!
//! // Mutable memory mapping
//! let mut mapped = MappedFileMut::open("data.bin")?;
//! mapped[0] = 0xFF;
//! mapped.flush()?;
//!
//! // Create new file with specified size
//! let mut mapped = MappedFileMut::create("new_file.bin", 1024)?;
//! mapped.as_mut_slice().fill(0);
//! mapped.flush()?;
//!
//! // Advanced options (offset, length, populate)
//! let options = MmapOptions::new().offset(1024).len(4096).populate(true);
//! let mapped = MappedFile::with_options("large.bin", &options)?;
//! ```
//!
//! # File Information
//!
//! ```ignore
//! use horizon_lattice::file::FileInfo;
//!
//! let info = FileInfo::new("document.txt")?;
//! println!("Size: {} bytes", info.size());
//! println!("Modified: {:?}", info.modified());
//! println!("Is file: {}", info.is_file());
//! println!("Readable: {}", info.is_readable());
//! ```
//!
//! # Embedded Resources
//!
//! ```ignore
//! use include_dir::{include_dir, Dir};
//! use horizon_lattice::file::{ResourceManager, EmbeddedDir};
//!
//! // Embed assets at compile time
//! static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");
//!
//! // Register with the resource manager
//! ResourceManager::global().register_embedded("", EmbeddedDir::new(&ASSETS));
//!
//! // Access resources by path
//! if let Some(data) = ResourceManager::global().get(":/images/icon.png") {
//!     // data is &'static [u8]
//! }
//!
//! // Get text resources
//! if let Some(css) = ResourceManager::global().get_text(":/styles/main.css") {
//!     // css is &'static str
//! }
//! ```
//!
//! # Settings/Preferences
//!
//! ```ignore
//! use horizon_lattice::file::{Settings, SettingsFormat};
//!
//! // Create settings and set values
//! let settings = Settings::new();
//! settings.set("app.window.width", 1024);
//! settings.set("app.theme.name", "dark");
//!
//! // Get values with type inference
//! let width: i32 = settings.get("app.window.width").unwrap();
//! let theme: String = settings.get_or("app.theme.name", "light".to_string());
//!
//! // Persist to file
//! settings.save_json("config.json")?;
//! settings.save_toml("config.toml")?;
//! settings.save_ini("config.ini")?;
//!
//! // Load from file
//! let settings = Settings::load_json("config.json")?;
//! let settings = Settings::load_ini("config.ini")?;
//!
//! // Enable auto-save
//! settings.set_auto_save("config.json", SettingsFormat::Json);
//!
//! // Listen for changes
//! settings.changed().connect(|key| {
//!     println!("Setting changed: {}", key);
//! });
//! ```
//!
//! # Directory Operations
//!
//! ```ignore
//! use horizon_lattice::file::{read_dir, read_dir_recursive, create_dir_all, WalkDirOptions};
//!
//! // List directory entries
//! for entry in read_dir("src")? {
//!     let entry = entry?;
//!     println!("{}: {:?}", entry.name(), entry.file_type()?);
//! }
//!
//! // Filter with glob pattern
//! for entry in read_dir("src")?.filter_glob("*.rs")? {
//!     println!("{}", entry?.name());
//! }
//!
//! // Recursive listing with options
//! let walker = WalkDir::with_options("src", WalkDirOptions::new()
//!     .files_only()
//!     .glob("*.rs")
//!     .skip_hidden(true))?;
//!
//! for entry in walker {
//!     let entry = entry?;
//!     println!("{} (depth {})", entry.path().display(), entry.depth());
//! }
//!
//! // Create nested directories
//! create_dir_all("path/to/nested/folder")?;
//! ```
//!
//! # Async Directory Operations
//!
//! ```ignore
//! use horizon_lattice::file::{read_dir_async, AsyncWalkDir, WalkDirOptions};
//!
//! // Async directory listing
//! let mut entries = read_dir_async("src").await?;
//! while let Some(entry) = entries.next().await {
//!     let entry = entry?;
//!     println!("{}", entry.name());
//! }
//!
//! // Async recursive walk with options
//! let mut walker = AsyncWalkDir::with_options("src", WalkDirOptions::new()
//!     .files_only()
//!     .glob("*.rs")
//!     .skip_hidden(true)).await?;
//!
//! while let Some(entry) = walker.next().await {
//!     let entry = entry?;
//!     println!("{} (depth {})", entry.path().display(), entry.depth());
//! }
//! ```
//!
//! # Temporary Files and Directories
//!
//! ```ignore
//! use horizon_lattice::file::{TempFile, TempDirectory};
//!
//! // Create a temp file (auto-deleted on drop)
//! let mut temp = TempFile::new()?;
//! temp.write_all(b"temporary data")?;
//!
//! // Create a temp directory (auto-deleted with contents on drop)
//! let temp_dir = TempDirectory::new()?;
//! let file_path = temp_dir.join("test.txt");
//! std::fs::write(&file_path, "content")?;
//!
//! // Builder pattern for customization
//! let temp = TempFile::builder()
//!     .prefix("myapp_")
//!     .suffix(".tmp")
//!     .create()?;
//!
//! // Keep a temp file (prevent auto-deletion)
//! let path = temp.keep();
//! ```

mod async_directory;
pub mod compression;
pub mod csv_support;
mod directory;
mod error;
mod info;
pub mod ini_support;
pub mod json;
mod mmap;
mod operations;
mod path;
mod reader;
mod resource;
mod settings;
mod temp;
pub mod toml_support;
mod watcher;
mod writer;
pub mod xml_support;

pub use async_directory::{
    AsyncDirEntry, AsyncDirIterator, AsyncWalkDir, AsyncWalkEntry, count_entries_async,
    dir_size_async, is_dir_empty_async, list_dir_async, read_dir_async, read_dir_recursive_async,
};
pub use compression::{
    CompressionLevel, GzipOptions, TarEntry, TarOptions, ZipEntry, ZipOptions, compress_gzip,
    compress_gzip_file, compress_gzip_file_with_options, compress_gzip_with_options, create_tar,
    create_tar_gz, create_tar_gz_with_options, create_tar_with_options, create_zip,
    create_zip_with_options, decompress_gzip, decompress_gzip_file, extract_tar, extract_tar_gz,
    extract_tar_gz_with_options, extract_tar_with_options, extract_zip, extract_zip_with_options,
    list_tar, list_tar_gz, list_zip, read_gzip, write_gzip, write_gzip_with_options,
};
pub use directory::{
    DirEntry, DirIterator, FilteredDirIterator, GlobDirIterator, WalkDir, WalkDirOptions,
    WalkEntry, count_entries, create_dir, create_dir_all, dir_size, is_dir_empty, list_dir,
    list_dir_glob, read_dir, read_dir_recursive, remove_dir, remove_dir_all,
};
pub use error::{FileError, FileErrorKind, FileResult};
pub use info::{
    FileInfo, FileType, Permissions, exists, exists_no_follow, file_size, is_dir, is_file,
    is_symlink,
};
pub use mmap::{MappedFile, MappedFileMut, MmapOptions, map_file, map_file_mut};
pub use operations::{
    append_bytes, append_text, atomic_write, copy_file, read_bytes, read_lines, read_text,
    remove_file, rename_file, write_bytes, write_text,
};
pub use path::{
    AppPaths, PathBuilder, absolute_path, cache_dir, canonicalize, config_dir, data_dir,
    data_local_dir, desktop_dir, documents_dir, downloads_dir, extension, file_name, file_name_os,
    file_stem, home_dir, is_absolute, is_relative, join_path, join_paths, music_dir,
    normalize_path, parent, pictures_dir, relative_to, temp_dir, videos_dir, with_extension,
    with_file_name,
};
pub use reader::{File, LineIterator};
pub use resource::{
    EmbeddedDir, IncludeDir, LazyResource, ResourceEntry, ResourceManager, ResourcePath,
    TypedLazyResource,
};
pub use settings::{FromSettingsValue, Settings, SettingsFormat, SettingsValue, SharedSettings};
pub use temp::{TempDirectory, TempDirectoryBuilder, TempFile, TempFileBuilder};
pub use watcher::{FileWatchEvent, FileWatcher, WatchEventKind, WatchOptions};
pub use writer::{AtomicWriter, FileWriter};
