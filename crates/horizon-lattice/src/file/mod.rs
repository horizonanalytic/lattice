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

mod error;
mod info;
mod operations;
mod reader;
mod writer;

pub use error::{FileError, FileErrorKind, FileResult};
pub use info::{exists, exists_no_follow, file_size, is_dir, is_file, is_symlink, FileInfo, FileType, Permissions};
pub use operations::{
    append_bytes, append_text, atomic_write, copy_file, read_bytes, read_lines, read_text,
    remove_file, rename_file, write_bytes, write_text,
};
pub use reader::{File, LineIterator};
pub use writer::{AtomicWriter, FileWriter};
