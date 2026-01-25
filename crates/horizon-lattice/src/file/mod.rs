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
//!
//! // Load from file
//! let settings = Settings::load_json("config.json")?;
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

mod directory;
mod error;
mod info;
mod operations;
mod path;
mod reader;
mod settings;
mod watcher;
mod writer;

pub use directory::{
    count_entries, create_dir, create_dir_all, dir_size, is_dir_empty, list_dir, list_dir_glob,
    read_dir, read_dir_recursive, remove_dir, remove_dir_all, DirEntry, DirIterator,
    FilteredDirIterator, GlobDirIterator, WalkDir, WalkDirOptions, WalkEntry,
};
pub use error::{FileError, FileErrorKind, FileResult};
pub use info::{
    exists, exists_no_follow, file_size, is_dir, is_file, is_symlink, FileInfo, FileType,
    Permissions,
};
pub use operations::{
    append_bytes, append_text, atomic_write, copy_file, read_bytes, read_lines, read_text,
    remove_file, rename_file, write_bytes, write_text,
};
pub use path::{
    absolute_path, cache_dir, canonicalize, config_dir, data_dir, data_local_dir, desktop_dir,
    documents_dir, downloads_dir, extension, file_name, file_name_os, file_stem, home_dir,
    is_absolute, is_relative, join_path, join_paths, music_dir, normalize_path, parent,
    pictures_dir, relative_to, temp_dir, videos_dir, with_extension, with_file_name, AppPaths,
    PathBuilder,
};
pub use reader::{File, LineIterator};
pub use watcher::{FileWatchEvent, FileWatcher, WatchEventKind, WatchOptions};
pub use settings::{FromSettingsValue, Settings, SettingsFormat, SettingsValue, SharedSettings};
pub use writer::{AtomicWriter, FileWriter};
