//! Sound effects module for Horizon Lattice.
//!
//! This module provides low-latency sound effect playback with support for
//! pre-loading and multiple simultaneous instances of the same sound.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_multimedia::sound_effects::SoundPool;
//!
//! // Create a sound pool
//! let mut pool = SoundPool::new()?;
//!
//! // Pre-load sounds
//! pool.load("explosion", "assets/explosion.wav")?;
//! pool.load("laser", "assets/laser.ogg")?;
//!
//! // Play sounds (can overlap)
//! pool.play("explosion")?;
//! pool.play("laser")?;
//! pool.play("laser")?; // Multiple lasers at once
//!
//! // Control volume
//! pool.set_volume(0.8); // Global volume
//! pool.set_sound_volume("explosion", 1.2); // Per-sound volume
//!
//! // Limit concurrent instances
//! pool.set_max_instances("laser", 4);
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

use horizon_lattice_core::signal::{ConnectionId, Signal};

use crate::error::{MultimediaError, Result};

/// Default maximum concurrent instances per sound.
const DEFAULT_MAX_INSTANCES: usize = 8;

/// Cleanup interval for the background thread.
const CLEANUP_INTERVAL_MS: u64 = 50;

/// Internal state for a loaded sound effect.
struct SoundEntry {
    /// Raw audio data for creating new playback instances.
    data: Vec<u8>,
    /// Per-sound volume multiplier.
    volume: f32,
    /// Maximum concurrent instances allowed.
    max_instances: usize,
    /// Currently active sinks playing this sound.
    active_sinks: Vec<Sink>,
}

impl SoundEntry {
    fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            volume: 1.0,
            max_instances: DEFAULT_MAX_INSTANCES,
            active_sinks: Vec::new(),
        }
    }

    /// Remove finished sinks and return the count of cleaned up sinks.
    fn cleanup_finished(&mut self) -> usize {
        let before = self.active_sinks.len();
        self.active_sinks.retain(|sink| !sink.empty());
        before - self.active_sinks.len()
    }

    /// Check if we can play another instance.
    fn can_play(&self) -> bool {
        self.active_sinks.len() < self.max_instances
    }

    /// Get the count of currently playing instances.
    fn playing_count(&self) -> usize {
        self.active_sinks.iter().filter(|s| !s.empty()).count()
    }
}

/// Internal state shared between the pool and cleanup thread.
struct PoolState {
    /// Loaded sound entries by ID.
    sounds: HashMap<String, SoundEntry>,
    /// Global volume multiplier.
    global_volume: f32,
}

impl PoolState {
    fn new() -> Self {
        Self {
            sounds: HashMap::new(),
            global_volume: 1.0,
        }
    }
}

/// Shared signals for the sound pool.
struct PoolSignals {
    /// Emitted when a sound finishes playing.
    /// The parameter is the sound ID.
    finished: Signal<String>,
    /// Emitted when an error occurs.
    error: Signal<String>,
}

/// A pool of pre-loaded sound effects for low-latency playback.
///
/// `SoundPool` allows you to load sounds upfront and play them instantly
/// by ID. Multiple instances of the same sound can play simultaneously,
/// with configurable limits per sound.
///
/// # Thread Safety
///
/// `SoundPool` is designed to be used from a single thread. The playback
/// of sounds happens on background audio threads managed by rodio.
///
/// # Signals
///
/// - `finished`: Emitted when a sound instance finishes playing.
/// - `error`: Emitted when an error occurs during playback.
///
/// # Example
///
/// ```ignore
/// let mut pool = SoundPool::new()?;
///
/// // Load sounds
/// pool.load("click", "sounds/click.wav")?;
/// pool.load("beep", "sounds/beep.ogg")?;
///
/// // Connect to finished signal
/// pool.on_finished(|sound_id| {
///     println!("Sound finished: {}", sound_id);
/// });
///
/// // Play sounds
/// pool.play("click")?;
/// pool.play("beep")?;
/// ```
pub struct SoundPool {
    /// The output stream (must be kept alive for audio to play).
    _stream: OutputStream,
    /// Handle for the output stream (used for creating sinks).
    stream_handle: OutputStreamHandle,
    /// Internal state protected by mutex.
    state: Arc<Mutex<PoolState>>,
    /// Flag to stop the cleanup thread.
    stop_cleanup: Arc<AtomicBool>,
    /// Cleanup thread handle.
    cleanup_handle: Option<std::thread::JoinHandle<()>>,
    /// Shared signals.
    signals: Arc<PoolSignals>,
}

impl SoundPool {
    /// Create a new sound pool.
    ///
    /// Initializes the audio output device. Returns an error if no
    /// audio output device is available.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pool = SoundPool::new()?;
    /// ```
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| MultimediaError::Device(e.to_string()))?;

        let state = Arc::new(Mutex::new(PoolState::new()));
        let stop_cleanup = Arc::new(AtomicBool::new(false));

        let signals = Arc::new(PoolSignals {
            finished: Signal::new(),
            error: Signal::new(),
        });

        let mut pool = Self {
            _stream: stream,
            stream_handle,
            state,
            stop_cleanup,
            cleanup_handle: None,
            signals,
        };

        pool.start_cleanup_thread();

        Ok(pool)
    }

    /// Start the background cleanup thread.
    fn start_cleanup_thread(&mut self) {
        let state = self.state.clone();
        let stop_flag = self.stop_cleanup.clone();
        let signals = self.signals.clone();

        let handle = std::thread::spawn(move || {
            // Track which sounds were playing in the last iteration
            let mut was_playing: HashMap<String, usize> = HashMap::new();

            while !stop_flag.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(CLEANUP_INTERVAL_MS));

                let mut pool_state = state.lock();

                for (id, entry) in pool_state.sounds.iter_mut() {
                    let finished_count = entry.cleanup_finished();

                    // Emit finished signals for each completed instance
                    if finished_count > 0 {
                        for _ in 0..finished_count {
                            signals.finished.emit(id.clone());
                        }
                    }

                    // Update tracking
                    let current_count = entry.playing_count();
                    was_playing.insert(id.clone(), current_count);
                }
            }
        });

        self.cleanup_handle = Some(handle);
    }

    /// Load a sound effect from a file path.
    ///
    /// The sound is decoded and stored in memory for instant playback.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for this sound
    /// * `path` - Path to the audio file
    ///
    /// # Supported Formats
    ///
    /// WAV, MP3, OGG Vorbis, FLAC, AAC/M4A
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.load("explosion", "assets/explosion.wav")?;
    /// ```
    pub fn load<P: AsRef<Path>>(&mut self, id: &str, path: P) -> Result<()> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        self.load_bytes(id, data)
    }

    /// Load a sound effect from a byte buffer.
    ///
    /// The format is auto-detected from the data.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for this sound
    /// * `data` - Raw audio file data
    ///
    /// # Example
    ///
    /// ```ignore
    /// let audio_data = std::fs::read("explosion.wav")?;
    /// pool.load_bytes("explosion", audio_data)?;
    /// ```
    pub fn load_bytes(&mut self, id: &str, data: Vec<u8>) -> Result<()> {
        // Validate the audio data by attempting to decode it
        let cursor = Cursor::new(data.clone());
        let _source = Decoder::new(cursor)?;

        let mut state = self.state.lock();
        state.sounds.insert(id.to_string(), SoundEntry::new(data));

        Ok(())
    }

    /// Play a loaded sound effect.
    ///
    /// Creates a new playback instance. Multiple instances of the same
    /// sound can play simultaneously up to the configured limit.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the sound to play
    ///
    /// # Errors
    ///
    /// Returns an error if the sound ID is not found or if the maximum
    /// concurrent instances limit has been reached.
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.play("explosion")?;
    /// pool.play("explosion")?; // Can overlap
    /// ```
    pub fn play(&self, id: &str) -> Result<()> {
        let mut state = self.state.lock();

        // Get global volume before mutable borrow
        let global_volume = state.global_volume;

        let entry = state
            .sounds
            .get_mut(id)
            .ok_or_else(|| MultimediaError::AudioLoad(format!("Sound not found: {}", id)))?;

        // Clean up finished sinks first
        entry.cleanup_finished();

        // Check concurrent limit
        if !entry.can_play() {
            return Err(MultimediaError::Playback(format!(
                "Maximum concurrent instances ({}) reached for sound: {}",
                entry.max_instances, id
            )));
        }

        // Create a new sink and play
        let cursor = Cursor::new(entry.data.clone());
        let source = Decoder::new(cursor)?;

        let sink = Sink::try_new(&self.stream_handle)?;
        sink.set_volume(entry.volume * global_volume);
        sink.append(source);
        sink.play();

        entry.active_sinks.push(sink);

        Ok(())
    }

    /// Stop all playing instances of a specific sound.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the sound to stop
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.stop("explosion");
    /// ```
    pub fn stop(&self, id: &str) {
        let mut state = self.state.lock();

        if let Some(entry) = state.sounds.get_mut(id) {
            for sink in entry.active_sinks.drain(..) {
                sink.stop();
            }
        }
    }

    /// Stop all currently playing sounds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.stop_all();
    /// ```
    pub fn stop_all(&self) {
        let mut state = self.state.lock();

        for entry in state.sounds.values_mut() {
            for sink in entry.active_sinks.drain(..) {
                sink.stop();
            }
        }
    }

    /// Set the global volume for all sounds.
    ///
    /// This is a multiplier applied on top of per-sound volumes.
    ///
    /// # Arguments
    ///
    /// * `volume` - Volume level (0.0 = muted, 1.0 = normal)
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.set_volume(0.5); // 50% global volume
    /// ```
    pub fn set_volume(&self, volume: f32) {
        let mut state = self.state.lock();
        state.global_volume = volume.max(0.0);

        // Update all currently playing sounds
        for entry in state.sounds.values() {
            let effective_volume = entry.volume * state.global_volume;
            for sink in &entry.active_sinks {
                sink.set_volume(effective_volume);
            }
        }
    }

    /// Get the global volume.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let vol = pool.volume();
    /// ```
    pub fn volume(&self) -> f32 {
        let state = self.state.lock();
        state.global_volume
    }

    /// Set the volume for a specific sound.
    ///
    /// This is multiplied with the global volume.
    ///
    /// # Arguments
    ///
    /// * `id` - The sound identifier
    /// * `volume` - Volume level (0.0 = muted, 1.0 = normal, >1.0 = amplified)
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.set_sound_volume("explosion", 1.5); // Louder explosions
    /// ```
    pub fn set_sound_volume(&self, id: &str, volume: f32) {
        let mut state = self.state.lock();

        // Get global volume before mutable borrow
        let global_volume = state.global_volume;

        if let Some(entry) = state.sounds.get_mut(id) {
            entry.volume = volume.max(0.0);
            let effective_volume = entry.volume * global_volume;

            // Update currently playing instances
            for sink in &entry.active_sinks {
                sink.set_volume(effective_volume);
            }
        }
    }

    /// Get the volume for a specific sound.
    ///
    /// Returns `None` if the sound is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(vol) = pool.sound_volume("explosion") {
    ///     println!("Explosion volume: {}", vol);
    /// }
    /// ```
    pub fn sound_volume(&self, id: &str) -> Option<f32> {
        let state = self.state.lock();
        state.sounds.get(id).map(|e| e.volume)
    }

    /// Set the maximum concurrent instances for a sound.
    ///
    /// If more instances than the new limit are currently playing,
    /// excess instances will be stopped.
    ///
    /// # Arguments
    ///
    /// * `id` - The sound identifier
    /// * `max` - Maximum concurrent instances (must be >= 1)
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.set_max_instances("laser", 4);
    /// ```
    pub fn set_max_instances(&self, id: &str, max: usize) {
        let max = max.max(1);
        let mut state = self.state.lock();

        if let Some(entry) = state.sounds.get_mut(id) {
            entry.max_instances = max;

            // Stop excess instances if over the new limit
            while entry.active_sinks.len() > max {
                if let Some(sink) = entry.active_sinks.pop() {
                    sink.stop();
                }
            }
        }
    }

    /// Get the maximum concurrent instances for a sound.
    ///
    /// Returns `None` if the sound is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(max) = pool.max_instances("laser") {
    ///     println!("Max laser instances: {}", max);
    /// }
    /// ```
    pub fn max_instances(&self, id: &str) -> Option<usize> {
        let state = self.state.lock();
        state.sounds.get(id).map(|e| e.max_instances)
    }

    /// Get the count of currently playing instances for a sound.
    ///
    /// Returns 0 if the sound is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let count = pool.playing_count("explosion");
    /// println!("{} explosions playing", count);
    /// ```
    pub fn playing_count(&self, id: &str) -> usize {
        let mut state = self.state.lock();

        if let Some(entry) = state.sounds.get_mut(id) {
            entry.cleanup_finished();
            entry.playing_count()
        } else {
            0
        }
    }

    /// Check if a sound is currently playing.
    ///
    /// Returns `false` if the sound is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if pool.is_playing("explosion") {
    ///     println!("Explosion in progress!");
    /// }
    /// ```
    pub fn is_playing(&self, id: &str) -> bool {
        self.playing_count(id) > 0
    }

    /// Unload a sound from the pool.
    ///
    /// Stops any playing instances and frees the memory.
    ///
    /// # Arguments
    ///
    /// * `id` - The sound identifier
    ///
    /// # Returns
    ///
    /// `true` if the sound was found and removed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.unload("explosion");
    /// ```
    pub fn unload(&mut self, id: &str) -> bool {
        let mut state = self.state.lock();

        if let Some(mut entry) = state.sounds.remove(id) {
            // Stop all playing instances
            for sink in entry.active_sinks.drain(..) {
                sink.stop();
            }
            true
        } else {
            false
        }
    }

    /// Check if a sound is loaded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if pool.is_loaded("explosion") {
    ///     pool.play("explosion")?;
    /// }
    /// ```
    pub fn is_loaded(&self, id: &str) -> bool {
        let state = self.state.lock();
        state.sounds.contains_key(id)
    }

    /// Get a list of all loaded sound IDs.
    ///
    /// # Example
    ///
    /// ```ignore
    /// for id in pool.loaded_sounds() {
    ///     println!("Loaded: {}", id);
    /// }
    /// ```
    pub fn loaded_sounds(&self) -> Vec<String> {
        let state = self.state.lock();
        state.sounds.keys().cloned().collect()
    }

    /// Connect a callback to the finished signal.
    ///
    /// The callback is invoked when a sound instance finishes playing.
    /// The parameter is the sound ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// pool.on_finished(|sound_id| {
    ///     println!("Sound finished: {}", sound_id);
    /// });
    /// ```
    pub fn on_finished<F>(&self, callback: F) -> ConnectionId
    where
        F: Fn(&String) + Send + Sync + 'static,
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
    /// pool.on_error(|msg| {
    ///     eprintln!("Sound error: {}", msg);
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
}

impl Drop for SoundPool {
    fn drop(&mut self) {
        // Signal cleanup thread to stop
        self.stop_cleanup.store(true, Ordering::SeqCst);

        // Wait for cleanup thread to finish
        if let Some(handle) = self.cleanup_handle.take() {
            let _ = handle.join();
        }

        // Stop all sounds
        self.stop_all();
    }
}

// SoundPool is Send but not Sync
unsafe impl Send for SoundPool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_pool_creation() {
        // This test may fail in CI environments without audio hardware
        if let Ok(pool) = SoundPool::new() {
            assert_eq!(pool.volume(), 1.0);
            assert!(pool.loaded_sounds().is_empty());
        }
    }

    #[test]
    fn test_default_max_instances() {
        assert_eq!(DEFAULT_MAX_INSTANCES, 8);
    }

    #[test]
    fn test_sound_entry_can_play() {
        let entry = SoundEntry::new(Vec::new());
        assert!(entry.can_play());
        assert_eq!(entry.max_instances, DEFAULT_MAX_INSTANCES);
    }

    #[test]
    fn test_sound_entry_max_instances() {
        let mut entry = SoundEntry::new(Vec::new());
        entry.max_instances = 2;

        // Without any active sinks, should be able to play
        assert!(entry.can_play());
        assert_eq!(entry.playing_count(), 0);
    }

    #[test]
    fn test_volume_clamping() {
        if let Ok(pool) = SoundPool::new() {
            pool.set_volume(-1.0);
            assert_eq!(pool.volume(), 0.0);

            pool.set_volume(0.5);
            assert_eq!(pool.volume(), 0.5);

            pool.set_volume(2.0);
            assert_eq!(pool.volume(), 2.0);
        }
    }

    #[test]
    fn test_load_nonexistent_file() {
        if let Ok(mut pool) = SoundPool::new() {
            let result = pool.load("test", "nonexistent_file.wav");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_play_unloaded_sound() {
        if let Ok(pool) = SoundPool::new() {
            let result = pool.play("nonexistent");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_unload_nonexistent() {
        if let Ok(mut pool) = SoundPool::new() {
            assert!(!pool.unload("nonexistent"));
        }
    }

    #[test]
    fn test_is_loaded() {
        if let Ok(pool) = SoundPool::new() {
            assert!(!pool.is_loaded("test"));
        }
    }

    #[test]
    fn test_stop_nonexistent() {
        if let Ok(pool) = SoundPool::new() {
            // Should not panic
            pool.stop("nonexistent");
        }
    }

    #[test]
    fn test_playing_count_nonexistent() {
        if let Ok(pool) = SoundPool::new() {
            assert_eq!(pool.playing_count("nonexistent"), 0);
        }
    }
}
