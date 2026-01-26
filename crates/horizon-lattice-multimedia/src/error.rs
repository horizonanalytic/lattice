//! Error types for the multimedia module.

use std::fmt;

/// Multimedia-specific errors.
#[derive(Debug, Clone)]
pub enum MultimediaError {
    /// Failed to load audio file.
    AudioLoad(String),
    /// Playback error occurred.
    Playback(String),
    /// Audio device error.
    Device(String),
    /// Unsupported audio format.
    UnsupportedFormat(String),
    /// Seek operation failed.
    Seek(String),
    /// I/O error.
    Io(String),
    /// Audio stream ended unexpectedly.
    StreamEnded,
    /// No audio output device available.
    NoOutputDevice,
    /// Invalid audio data.
    InvalidData(String),
}

impl fmt::Display for MultimediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AudioLoad(msg) => write!(f, "Failed to load audio: {msg}"),
            Self::Playback(msg) => write!(f, "Playback error: {msg}"),
            Self::Device(msg) => write!(f, "Audio device error: {msg}"),
            Self::UnsupportedFormat(msg) => write!(f, "Unsupported audio format: {msg}"),
            Self::Seek(msg) => write!(f, "Seek error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::StreamEnded => write!(f, "Audio stream ended unexpectedly"),
            Self::NoOutputDevice => write!(f, "No audio output device available"),
            Self::InvalidData(msg) => write!(f, "Invalid audio data: {msg}"),
        }
    }
}

impl std::error::Error for MultimediaError {}

impl From<std::io::Error> for MultimediaError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<rodio::StreamError> for MultimediaError {
    fn from(err: rodio::StreamError) -> Self {
        Self::Device(err.to_string())
    }
}

impl From<rodio::PlayError> for MultimediaError {
    fn from(err: rodio::PlayError) -> Self {
        Self::Playback(err.to_string())
    }
}

impl From<rodio::decoder::DecoderError> for MultimediaError {
    fn from(err: rodio::decoder::DecoderError) -> Self {
        Self::AudioLoad(err.to_string())
    }
}

/// A specialized Result type for multimedia operations.
pub type Result<T> = std::result::Result<T, MultimediaError>;
