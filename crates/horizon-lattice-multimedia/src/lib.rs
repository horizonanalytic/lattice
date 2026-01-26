//! Multimedia module for Horizon Lattice.
//!
//! This crate provides multimedia capabilities for Horizon Lattice applications:
//!
//! - **Audio Playback**: Load and play audio files with signal-based state notifications
//!
//! # Audio Playback
//!
//! The audio player provides a high-level API for playing audio files:
//!
//! ```ignore
//! use horizon_lattice_multimedia::AudioPlayer;
//!
//! // Create a player
//! let player = AudioPlayer::new()?;
//!
//! // Connect to state changes
//! player.on_state_changed(|state| {
//!     println!("State: {:?}", state);
//! });
//!
//! // Load and play
//! player.load_file("music.mp3")?;
//! player.play();
//!
//! // Control playback
//! player.set_volume(0.8);
//! player.pause();
//! player.set_looping(true);
//! player.play();
//! ```
//!
//! ## Supported Formats
//!
//! - WAV
//! - MP3
//! - OGG Vorbis
//! - FLAC
//! - AAC/M4A (via Symphonia backend)

mod error;
pub mod audio;

pub use error::{MultimediaError, Result};

// Re-export commonly used types at the crate root
pub use audio::{AudioMetadata, AudioPlayer, PlaybackState};
