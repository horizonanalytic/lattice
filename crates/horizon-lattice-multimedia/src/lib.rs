//! Multimedia module for Horizon Lattice.
//!
//! This crate provides multimedia capabilities for Horizon Lattice applications:
//!
//! - **Audio Playback**: Load and play audio files with signal-based state notifications
//! - **High-Precision Timers** (feature `high-precision-timers`): Sub-millisecond accurate timing
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
//!
//! # High-Precision Timers
//!
//! When the `high-precision-timers` feature is enabled, this crate provides
//! sub-millisecond accurate timing using native sleep combined with spin-waiting:
//!
//! ```ignore
//! use horizon_lattice_multimedia::timers::{HighPrecisionTimer, precise_sleep};
//! use std::time::Duration;
//!
//! // One-shot precise sleep
//! precise_sleep(Duration::from_micros(500));
//!
//! // Interval timer for game loops, A/V sync, etc.
//! let timer = HighPrecisionTimer::new(Duration::from_millis(16))?; // ~60 FPS
//! timer.on_tick(|event| {
//!     println!("Tick {}, drift: {:?}", event.tick_count, event.drift);
//! });
//! timer.start()?;
//! ```

mod error;
pub mod audio;

#[cfg(feature = "high-precision-timers")]
pub mod timers;

pub use error::{MultimediaError, Result};

// Re-export commonly used types at the crate root
pub use audio::{AudioMetadata, AudioPlayer, PlaybackState};

// Re-export timer types when feature is enabled
#[cfg(feature = "high-precision-timers")]
pub use timers::{
    HighPrecisionTimer, PreciseSleeper, SpinStrategyConfig, TimerConfig, TimerEvent,
    precise_sleep, precise_sleep_ns, precise_sleep_s,
};
