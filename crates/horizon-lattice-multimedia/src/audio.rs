//! Audio playback module for Horizon Lattice.
//!
//! This module provides audio playback capabilities with a signal-based API
//! suitable for GUI applications.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_multimedia::audio::{AudioPlayer, PlaybackState};
//!
//! // Create a player
//! let player = AudioPlayer::new()?;
//!
//! // Connect to state changes
//! player.on_state_changed(|state| {
//!     println!("State changed to: {:?}", state);
//! });
//!
//! // Load and play a file
//! player.load_file("music.mp3")?;
//! player.play();
//!
//! // Control playback
//! player.set_volume(0.8);
//! player.pause();
//! player.play();
//! ```

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use horizon_lattice_core::signal::{ConnectionId, Signal};

use crate::error::{MultimediaError, Result};

/// The current state of audio playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// No audio is loaded.
    Stopped,
    /// Audio is loaded and playing.
    Playing,
    /// Audio is loaded but paused.
    Paused,
    /// Audio finished playing naturally.
    Finished,
}

/// Metadata about a loaded audio file.
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// Duration of the audio in seconds, if known.
    pub duration: Option<Duration>,
    /// Sample rate in Hz.
    pub sample_rate: Option<u32>,
    /// Number of audio channels.
    pub channels: Option<u16>,
}

impl Default for AudioMetadata {
    fn default() -> Self {
        Self {
            duration: None,
            sample_rate: None,
            channels: None,
        }
    }
}

/// Internal state shared between the player and the monitoring thread.
struct PlayerState {
    /// The audio sink for playback control.
    sink: Sink,
    /// Current playback state.
    state: PlaybackState,
    /// Whether looping is enabled.
    looping: bool,
    /// Audio metadata.
    metadata: AudioMetadata,
    /// Source data for looping.
    source_data: Option<Vec<u8>>,
}

/// Shared signals that can be accessed from multiple threads.
struct SharedSignals {
    /// Emitted when playback state changes.
    state_changed: Signal<PlaybackState>,
    /// Emitted when audio finishes playing.
    finished: Signal<()>,
    /// Emitted when an error occurs.
    error: Signal<String>,
    /// Emitted when duration is known after loading.
    duration_changed: Signal<Option<Duration>>,
}

/// An audio player with signal-based notifications.
///
/// `AudioPlayer` provides a high-level API for audio playback suitable for
/// GUI applications. State changes are communicated via signals rather than
/// blocking calls.
///
/// # Signals
///
/// - `state_changed`: Emitted when playback state changes.
/// - `finished`: Emitted when audio finishes playing (not looping).
/// - `error`: Emitted when an error occurs during playback.
/// - `duration_changed`: Emitted when a new audio file is loaded with its duration.
pub struct AudioPlayer {
    /// The output stream (must be kept alive for audio to play).
    _stream: OutputStream,
    /// Internal player state.
    state: Arc<Mutex<PlayerState>>,
    /// Handle for the output stream (stored for creating new sinks).
    stream_handle: OutputStreamHandle,
    /// Flag to stop the monitoring thread.
    stop_monitor: Arc<AtomicBool>,
    /// Monitor thread handle.
    monitor_handle: Option<std::thread::JoinHandle<()>>,
    /// Shared signals for cross-thread communication.
    signals: Arc<SharedSignals>,
}

impl AudioPlayer {
    /// Create a new audio player.
    ///
    /// This initializes the audio output device. Returns an error if no
    /// audio output device is available.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let player = AudioPlayer::new()?;
    /// ```
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| MultimediaError::Device(e.to_string()))?;

        let sink = Sink::try_new(&stream_handle)?;
        sink.pause(); // Start paused

        let state = Arc::new(Mutex::new(PlayerState {
            sink,
            state: PlaybackState::Stopped,
            looping: false,
            metadata: AudioMetadata::default(),
            source_data: None,
        }));

        let stop_monitor = Arc::new(AtomicBool::new(false));

        let signals = Arc::new(SharedSignals {
            state_changed: Signal::new(),
            finished: Signal::new(),
            error: Signal::new(),
            duration_changed: Signal::new(),
        });

        // Clone stream_handle for later use
        let stream_handle_clone = stream_handle.clone();

        let mut player = Self {
            _stream: stream,
            state,
            stream_handle: stream_handle_clone,
            stop_monitor,
            monitor_handle: None,
            signals,
        };

        player.start_monitor();

        Ok(player)
    }

    /// Start the background monitoring thread.
    fn start_monitor(&mut self) {
        let state = self.state.clone();
        let stop_flag = self.stop_monitor.clone();
        let signals = self.signals.clone();
        let stream_handle = self.stream_handle.clone();

        let handle = std::thread::spawn(move || {
            let mut last_state = PlaybackState::Stopped;

            while !stop_flag.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));

                let mut player_state = state.lock();

                // Check if playback finished
                if player_state.state == PlaybackState::Playing && player_state.sink.empty() {
                    if player_state.looping {
                        // Restart playback from the beginning
                        if let Some(ref data) = player_state.source_data {
                            let cursor = Cursor::new(data.clone());
                            if let Ok(source) = Decoder::new(cursor) {
                                // Create a new sink for looped playback
                                if let Ok(new_sink) = Sink::try_new(&stream_handle) {
                                    new_sink.append(source);
                                    new_sink.play();
                                    player_state.sink = new_sink;
                                }
                            }
                        }
                    } else {
                        player_state.state = PlaybackState::Finished;
                    }
                }

                let current_state = player_state.state;
                drop(player_state);

                // Emit state change if different
                if current_state != last_state {
                    signals.state_changed.emit(current_state);
                    if current_state == PlaybackState::Finished {
                        signals.finished.emit(());
                    }
                    last_state = current_state;
                }
            }
        });

        self.monitor_handle = Some(handle);
    }

    /// Connect a callback to the state changed signal.
    ///
    /// The callback is invoked whenever the playback state changes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let conn_id = player.on_state_changed(|state| {
    ///     println!("State: {:?}", state);
    /// });
    /// ```
    pub fn on_state_changed<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&PlaybackState) + Send + Sync + 'static,
    {
        self.signals.state_changed.connect(callback)
    }

    /// Disconnect a state changed callback.
    pub fn disconnect_state_changed(&self, id: ConnectionId) -> bool {
        self.signals.state_changed.disconnect(id)
    }

    /// Connect a callback to the finished signal.
    ///
    /// The callback is invoked when audio finishes playing (not looping).
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.on_finished(|_| {
    ///     println!("Audio finished");
    /// });
    /// ```
    pub fn on_finished<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&()) + Send + Sync + 'static,
    {
        self.signals.finished.connect(callback)
    }

    /// Disconnect a finished callback.
    pub fn disconnect_finished(&self, id: ConnectionId) -> bool {
        self.signals.finished.disconnect(id)
    }

    /// Connect a callback to the error signal.
    ///
    /// The callback is invoked when an error occurs during playback.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.on_error(|msg| {
    ///     eprintln!("Error: {}", msg);
    /// });
    /// ```
    pub fn on_error<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&String) + Send + Sync + 'static,
    {
        self.signals.error.connect(callback)
    }

    /// Disconnect an error callback.
    pub fn disconnect_error(&self, id: ConnectionId) -> bool {
        self.signals.error.disconnect(id)
    }

    /// Connect a callback to the duration changed signal.
    ///
    /// The callback is invoked when a new audio file is loaded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.on_duration_changed(|duration| {
    ///     if let Some(d) = duration {
    ///         println!("Duration: {:?}", d);
    ///     }
    /// });
    /// ```
    pub fn on_duration_changed<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&Option<Duration>) + Send + Sync + 'static,
    {
        self.signals.duration_changed.connect(callback)
    }

    /// Disconnect a duration changed callback.
    pub fn disconnect_duration_changed(&self, id: ConnectionId) -> bool {
        self.signals.duration_changed.disconnect(id)
    }

    /// Load audio from a file path.
    ///
    /// Supported formats: WAV, MP3, OGG Vorbis, FLAC, and AAC/M4A.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.load_file("music.mp3")?;
    /// ```
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read the entire file into memory for seeking and looping support
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        self.load_bytes(data)
    }

    /// Load audio from a byte buffer.
    ///
    /// The format is auto-detected from the data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let audio_data = std::fs::read("music.mp3")?;
    /// player.load_bytes(audio_data)?;
    /// ```
    pub fn load_bytes(&self, data: Vec<u8>) -> Result<()> {
        let cursor = Cursor::new(data.clone());
        let source = Decoder::new(cursor)?;

        // Extract metadata
        let sample_rate = source.sample_rate();
        let channels = source.channels();

        // Try to get duration
        let duration = source.total_duration();

        let metadata = AudioMetadata {
            duration,
            sample_rate: Some(sample_rate),
            channels: Some(channels),
        };

        // Create a fresh cursor for the sink
        let cursor = Cursor::new(data.clone());
        let source = Decoder::new(cursor)?;

        let mut state = self.state.lock();

        // Stop current playback
        state.sink.stop();

        // Create new sink
        let sink = Sink::try_new(&self.stream_handle)?;
        sink.append(source);
        sink.pause();

        state.sink = sink;
        state.state = PlaybackState::Stopped;
        state.metadata = metadata.clone();
        state.source_data = Some(data);

        drop(state);

        self.signals.duration_changed.emit(metadata.duration);
        self.signals.state_changed.emit(PlaybackState::Stopped);

        Ok(())
    }

    /// Start or resume playback.
    ///
    /// If stopped, starts from the beginning. If paused, resumes from
    /// the current position.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.play();
    /// ```
    pub fn play(&self) {
        let mut state = self.state.lock();
        if state.state == PlaybackState::Stopped || state.state == PlaybackState::Finished {
            // Restart from beginning if stopped or finished
            if let Some(ref data) = state.source_data {
                let cursor = Cursor::new(data.clone());
                if let Ok(source) = Decoder::new(cursor) {
                    if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
                        new_sink.append(source);
                        new_sink.play();
                        state.sink = new_sink;
                        state.state = PlaybackState::Playing;
                    }
                }
            }
        } else if state.state == PlaybackState::Paused {
            state.sink.play();
            state.state = PlaybackState::Playing;
        }
    }

    /// Pause playback.
    ///
    /// Audio can be resumed from the current position with `play()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.pause();
    /// ```
    pub fn pause(&self) {
        let mut state = self.state.lock();
        if state.state == PlaybackState::Playing {
            state.sink.pause();
            state.state = PlaybackState::Paused;
        }
    }

    /// Stop playback and reset to the beginning.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.stop();
    /// ```
    pub fn stop(&self) {
        let mut state = self.state.lock();
        state.sink.stop();
        state.state = PlaybackState::Stopped;
    }

    /// Toggle between play and pause.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.toggle_playback();
    /// ```
    pub fn toggle_playback(&self) {
        let state = self.state.lock();
        let current_state = state.state;
        drop(state);

        match current_state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused | PlaybackState::Stopped | PlaybackState::Finished => {
                self.play()
            }
        }
    }

    /// Set the playback volume.
    ///
    /// Volume is a multiplier where 1.0 is normal volume. Values above 1.0
    /// will amplify the audio.
    ///
    /// # Arguments
    ///
    /// * `volume` - Volume level (0.0 = muted, 1.0 = normal, >1.0 = amplified)
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.set_volume(0.5); // 50% volume
    /// ```
    pub fn set_volume(&self, volume: f32) {
        let state = self.state.lock();
        state.sink.set_volume(volume.max(0.0));
    }

    /// Get the current volume level.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let vol = player.volume();
    /// ```
    pub fn volume(&self) -> f32 {
        let state = self.state.lock();
        state.sink.volume()
    }

    /// Set whether playback should loop.
    ///
    /// When looping is enabled, playback will restart from the beginning
    /// when the audio finishes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.set_looping(true);
    /// ```
    pub fn set_looping(&self, looping: bool) {
        let mut state = self.state.lock();
        state.looping = looping;
    }

    /// Check if looping is enabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if player.is_looping() {
    ///     println!("Audio will loop");
    /// }
    /// ```
    pub fn is_looping(&self) -> bool {
        let state = self.state.lock();
        state.looping
    }

    /// Get the current playback state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// match player.state() {
    ///     PlaybackState::Playing => println!("Playing"),
    ///     PlaybackState::Paused => println!("Paused"),
    ///     _ => {}
    /// }
    /// ```
    pub fn state(&self) -> PlaybackState {
        let state = self.state.lock();
        state.state
    }

    /// Get metadata about the loaded audio.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(duration) = player.metadata().duration {
    ///     println!("Duration: {:?}", duration);
    /// }
    /// ```
    pub fn metadata(&self) -> AudioMetadata {
        let state = self.state.lock();
        state.metadata.clone()
    }

    /// Get the duration of the loaded audio, if known.
    ///
    /// Not all audio formats provide duration information upfront.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(duration) = player.duration() {
    ///     println!("Duration: {:?}", duration);
    /// }
    /// ```
    pub fn duration(&self) -> Option<Duration> {
        let state = self.state.lock();
        state.metadata.duration
    }

    /// Set the playback speed.
    ///
    /// A speed of 1.0 is normal speed, 0.5 is half speed, 2.0 is double speed.
    ///
    /// # Arguments
    ///
    /// * `speed` - Speed multiplier (must be > 0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.set_speed(1.5); // 50% faster
    /// ```
    pub fn set_speed(&self, speed: f32) {
        let state = self.state.lock();
        state.sink.set_speed(speed.max(0.01));
    }

    /// Get the current playback speed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let speed = player.speed();
    /// ```
    pub fn speed(&self) -> f32 {
        let state = self.state.lock();
        state.sink.speed()
    }

    /// Check if audio is currently loaded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if player.has_audio() {
    ///     player.play();
    /// }
    /// ```
    pub fn has_audio(&self) -> bool {
        let state = self.state.lock();
        state.source_data.is_some()
    }

    /// Clear the loaded audio.
    ///
    /// Stops playback and releases the audio data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// player.clear();
    /// ```
    pub fn clear(&self) {
        let mut state = self.state.lock();
        state.sink.stop();
        state.state = PlaybackState::Stopped;
        state.source_data = None;
        state.metadata = AudioMetadata::default();
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop_monitor.store(true, Ordering::SeqCst);
        if let Some(handle) = self.monitor_handle.take() {
            let _ = handle.join();
        }
    }
}

// AudioPlayer is Send but not Sync (the stream handle is not thread-safe)
// However, we can mark it as Send since we protect access with Mutex
unsafe impl Send for AudioPlayer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_creation() {
        // This test may fail in CI environments without audio hardware
        if let Ok(player) = AudioPlayer::new() {
            assert_eq!(player.state(), PlaybackState::Stopped);
            assert!(!player.has_audio());
            assert_eq!(player.volume(), 1.0);
            assert!(!player.is_looping());
        }
    }

    #[test]
    fn test_playback_state_enum() {
        assert_eq!(PlaybackState::Stopped, PlaybackState::Stopped);
        assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
    }

    #[test]
    fn test_audio_metadata_default() {
        let metadata = AudioMetadata::default();
        assert!(metadata.duration.is_none());
        assert!(metadata.sample_rate.is_none());
        assert!(metadata.channels.is_none());
    }

    #[test]
    fn test_volume_clamping() {
        if let Ok(player) = AudioPlayer::new() {
            player.set_volume(-1.0);
            assert_eq!(player.volume(), 0.0);

            player.set_volume(0.5);
            assert_eq!(player.volume(), 0.5);

            player.set_volume(2.0);
            assert_eq!(player.volume(), 2.0);
        }
    }

    #[test]
    fn test_looping_toggle() {
        if let Ok(player) = AudioPlayer::new() {
            assert!(!player.is_looping());
            player.set_looping(true);
            assert!(player.is_looping());
            player.set_looping(false);
            assert!(!player.is_looping());
        }
    }

    #[test]
    fn test_speed_setting() {
        if let Ok(player) = AudioPlayer::new() {
            assert_eq!(player.speed(), 1.0);
            player.set_speed(2.0);
            assert_eq!(player.speed(), 2.0);
            player.set_speed(0.5);
            assert_eq!(player.speed(), 0.5);
        }
    }
}
