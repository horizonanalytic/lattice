//! Image caching with LRU eviction.
//!
//! This module provides in-memory caching for decoded images with configurable
//! size limits and automatic LRU (Least Recently Used) eviction.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{ImageCache, ImageCacheConfig, ImageBuffer};
//!
//! let config = ImageCacheConfig::default()
//!     .with_max_size_mb(100);
//!
//! let mut cache = ImageCache::new(config);
//!
//! // Load and cache an image
//! let buffer = ImageBuffer::from_file("photo.jpg")?;
//! cache.insert_file("photo.jpg", buffer.clone());
//!
//! // Retrieve from cache (moves entry to front of LRU)
//! if let Some(cached) = cache.get_file("photo.jpg") {
//!     println!("Got cached image: {}x{}", cached.width(), cached.height());
//! }
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::image_buffer::ImageBuffer;

/// Configuration for the image cache.
#[derive(Debug, Clone)]
pub struct ImageCacheConfig {
    /// Maximum cache size in bytes.
    /// When exceeded, least recently used entries are evicted.
    /// Default: 100 MB.
    pub max_size_bytes: usize,
    /// Whether to track access patterns for LRU eviction.
    /// Default: true.
    pub enable_lru: bool,
}

impl Default for ImageCacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 100 * 1024 * 1024, // 100 MB
            enable_lru: true,
        }
    }
}

impl ImageCacheConfig {
    /// Set the maximum cache size in megabytes.
    #[must_use]
    pub fn with_max_size_mb(mut self, mb: usize) -> Self {
        self.max_size_bytes = mb * 1024 * 1024;
        self
    }

    /// Set the maximum cache size in bytes.
    #[must_use]
    pub fn with_max_size_bytes(mut self, bytes: usize) -> Self {
        self.max_size_bytes = bytes;
        self
    }

    /// Enable or disable LRU eviction tracking.
    #[must_use]
    pub fn with_lru(mut self, enable: bool) -> Self {
        self.enable_lru = enable;
        self
    }
}

/// Key type for cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    /// Cache key for a file path.
    File(PathBuf),
    /// Cache key for a URL.
    Url(String),
    /// Cache key for raw bytes (uses a hash of the data).
    BytesHash(u64),
    /// Custom string key.
    Custom(String),
}

impl CacheKey {
    /// Create a cache key from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        CacheKey::File(path.as_ref().to_path_buf())
    }

    /// Create a cache key from a URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        CacheKey::Url(url.into())
    }

    /// Create a cache key from raw bytes by hashing them.
    pub fn from_bytes(data: &[u8]) -> Self {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        CacheKey::BytesHash(hasher.finish())
    }

    /// Create a custom cache key.
    pub fn custom(key: impl Into<String>) -> Self {
        CacheKey::Custom(key.into())
    }
}

/// A cached image entry with metadata.
#[derive(Clone)]
struct CacheEntry {
    /// The cached image buffer.
    buffer: ImageBuffer,
    /// Size of this entry in bytes (estimated from dimensions).
    size_bytes: usize,
}

impl CacheEntry {
    fn new(buffer: ImageBuffer) -> Self {
        // Estimate size: width * height * 4 bytes (RGBA)
        let size_bytes = (buffer.width() as usize) * (buffer.height() as usize) * 4;
        Self { buffer, size_bytes }
    }
}

/// Node in the LRU linked list.
struct LruNode {
    #[allow(dead_code)]
    key: CacheKey,
    prev: Option<CacheKey>,
    next: Option<CacheKey>,
}

/// An LRU (Least Recently Used) cache for decoded images.
///
/// This cache stores decoded [`ImageBuffer`] objects in memory with automatic
/// eviction when the size limit is exceeded. The least recently used entries
/// are evicted first.
///
/// # Size Estimation
///
/// The cache estimates memory usage based on image dimensions (width × height × 4 bytes
/// for RGBA). This is an approximation as actual memory usage may vary depending on
/// the internal image format.
///
/// # Thread Safety
///
/// This cache is NOT thread-safe. For concurrent access, wrap it in a `Mutex` or
/// use separate caches per thread.
pub struct ImageCache {
    /// Configuration.
    config: ImageCacheConfig,
    /// Cached entries by key.
    entries: HashMap<CacheKey, CacheEntry>,
    /// LRU tracking: key -> node.
    lru_nodes: HashMap<CacheKey, LruNode>,
    /// Head of LRU list (most recently used).
    lru_head: Option<CacheKey>,
    /// Tail of LRU list (least recently used).
    lru_tail: Option<CacheKey>,
    /// Current total size in bytes.
    current_size: usize,
    /// Statistics: number of cache hits.
    hits: u64,
    /// Statistics: number of cache misses.
    misses: u64,
}

impl ImageCache {
    /// Create a new image cache with the given configuration.
    pub fn new(config: ImageCacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            lru_nodes: HashMap::new(),
            lru_head: None,
            lru_tail: None,
            current_size: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Create a new image cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ImageCacheConfig::default())
    }

    /// Get the current cache size in bytes.
    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.current_size
    }

    /// Get the maximum cache size in bytes.
    #[inline]
    pub fn max_size_bytes(&self) -> usize {
        self.config.max_size_bytes
    }

    /// Get the number of entries in the cache.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the cache hit count.
    #[inline]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get the cache miss count.
    #[inline]
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Get the cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Insert an image into the cache with the given key.
    ///
    /// If the key already exists, the entry is updated and moved to the front
    /// of the LRU list. If the cache exceeds its size limit, least recently
    /// used entries are evicted.
    pub fn insert(&mut self, key: CacheKey, buffer: ImageBuffer) {
        let entry = CacheEntry::new(buffer);
        let entry_size = entry.size_bytes;

        // Remove existing entry if present
        if self.entries.contains_key(&key) {
            self.remove(&key);
        }

        // Evict entries if necessary to make room
        while self.current_size + entry_size > self.config.max_size_bytes
            && !self.entries.is_empty()
        {
            if let Some(tail_key) = self.lru_tail.clone() {
                self.remove(&tail_key);
            } else {
                break;
            }
        }

        // Don't insert if single entry is larger than max size
        if entry_size > self.config.max_size_bytes {
            return;
        }

        // Insert the entry
        self.entries.insert(key.clone(), entry);
        self.current_size += entry_size;

        // Add to LRU list at head
        if self.config.enable_lru {
            self.lru_push_front(key);
        }
    }

    /// Insert an image using a file path as the key.
    pub fn insert_file(&mut self, path: impl AsRef<Path>, buffer: ImageBuffer) {
        self.insert(CacheKey::from_file(path), buffer);
    }

    /// Insert an image using a URL as the key.
    pub fn insert_url(&mut self, url: impl Into<String>, buffer: ImageBuffer) {
        self.insert(CacheKey::from_url(url), buffer);
    }

    /// Insert an image using a bytes hash as the key.
    pub fn insert_bytes(&mut self, data: &[u8], buffer: ImageBuffer) {
        self.insert(CacheKey::from_bytes(data), buffer);
    }

    /// Get an image from the cache.
    ///
    /// If found, the entry is moved to the front of the LRU list.
    /// Returns `None` if the key is not in the cache.
    pub fn get(&mut self, key: &CacheKey) -> Option<&ImageBuffer> {
        if self.entries.contains_key(key) {
            self.hits += 1;
            // Move to front of LRU list
            if self.config.enable_lru {
                self.lru_move_to_front(key.clone());
            }
            self.entries.get(key).map(|e| &e.buffer)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Get an image by file path.
    pub fn get_file(&mut self, path: impl AsRef<Path>) -> Option<&ImageBuffer> {
        self.get(&CacheKey::from_file(path))
    }

    /// Get an image by URL.
    pub fn get_url(&mut self, url: &str) -> Option<&ImageBuffer> {
        self.get(&CacheKey::from_url(url))
    }

    /// Get an image by bytes hash.
    pub fn get_bytes(&mut self, data: &[u8]) -> Option<&ImageBuffer> {
        self.get(&CacheKey::from_bytes(data))
    }

    /// Check if a key exists in the cache (without updating LRU order).
    pub fn contains(&self, key: &CacheKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Check if a file path exists in the cache.
    pub fn contains_file(&self, path: impl AsRef<Path>) -> bool {
        self.contains(&CacheKey::from_file(path))
    }

    /// Check if a URL exists in the cache.
    pub fn contains_url(&self, url: &str) -> bool {
        self.contains(&CacheKey::from_url(url))
    }

    /// Remove an entry from the cache.
    ///
    /// Returns the removed image buffer if it existed.
    pub fn remove(&mut self, key: &CacheKey) -> Option<ImageBuffer> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size_bytes;

            // Remove from LRU list
            if self.config.enable_lru {
                self.lru_remove(key);
            }

            Some(entry.buffer)
        } else {
            None
        }
    }

    /// Remove an entry by file path.
    pub fn remove_file(&mut self, path: impl AsRef<Path>) -> Option<ImageBuffer> {
        self.remove(&CacheKey::from_file(path))
    }

    /// Remove an entry by URL.
    pub fn remove_url(&mut self, url: &str) -> Option<ImageBuffer> {
        self.remove(&CacheKey::from_url(url))
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_nodes.clear();
        self.lru_head = None;
        self.lru_tail = None;
        self.current_size = 0;
    }

    /// Reset cache statistics.
    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> ImageCacheStats {
        ImageCacheStats {
            entries: self.entries.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.config.max_size_bytes,
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
        }
    }

    // ========================================================================
    // LRU LIST OPERATIONS
    // ========================================================================

    /// Push a key to the front of the LRU list.
    fn lru_push_front(&mut self, key: CacheKey) {
        let node = LruNode {
            key: key.clone(),
            prev: None,
            next: self.lru_head.clone(),
        };

        // Update old head's prev pointer
        if let Some(old_head) = &self.lru_head
            && let Some(old_node) = self.lru_nodes.get_mut(old_head) {
                old_node.prev = Some(key.clone());
            }

        // Update tail if this is the first entry
        if self.lru_tail.is_none() {
            self.lru_tail = Some(key.clone());
        }

        self.lru_head = Some(key.clone());
        self.lru_nodes.insert(key, node);
    }

    /// Move a key to the front of the LRU list.
    fn lru_move_to_front(&mut self, key: CacheKey) {
        // If already at head, nothing to do
        if self.lru_head.as_ref() == Some(&key) {
            return;
        }

        // Remove from current position
        self.lru_remove(&key);

        // Add to front
        self.lru_push_front(key);
    }

    /// Remove a key from the LRU list.
    fn lru_remove(&mut self, key: &CacheKey) {
        if let Some(node) = self.lru_nodes.remove(key) {
            // Update prev node's next pointer
            if let Some(prev_key) = &node.prev {
                if let Some(prev_node) = self.lru_nodes.get_mut(prev_key) {
                    prev_node.next = node.next.clone();
                }
            } else {
                // This was the head
                self.lru_head = node.next.clone();
            }

            // Update next node's prev pointer
            if let Some(next_key) = &node.next {
                if let Some(next_node) = self.lru_nodes.get_mut(next_key) {
                    next_node.prev = node.prev.clone();
                }
            } else {
                // This was the tail
                self.lru_tail = node.prev.clone();
            }
        }
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl std::fmt::Debug for ImageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageCache")
            .field("entries", &self.entries.len())
            .field("size_mb", &(self.current_size as f64 / 1024.0 / 1024.0))
            .field(
                "max_size_mb",
                &(self.config.max_size_bytes as f64 / 1024.0 / 1024.0),
            )
            .field("hit_rate", &format!("{:.1}%", self.hit_rate() * 100.0))
            .finish()
    }
}

/// Statistics about the image cache.
#[derive(Debug, Clone)]
pub struct ImageCacheStats {
    /// Number of entries in the cache.
    pub entries: usize,
    /// Current size in bytes.
    pub size_bytes: usize,
    /// Maximum size in bytes.
    pub max_size_bytes: usize,
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Cache hit rate (0.0 to 1.0).
    pub hit_rate: f64,
}

impl ImageCacheStats {
    /// Get the current size in megabytes.
    pub fn size_mb(&self) -> f64 {
        self.size_bytes as f64 / 1024.0 / 1024.0
    }

    /// Get the maximum size in megabytes.
    pub fn max_size_mb(&self) -> f64 {
        self.max_size_bytes as f64 / 1024.0 / 1024.0
    }

    /// Get the usage percentage (0.0 to 100.0).
    pub fn usage_percent(&self) -> f64 {
        if self.max_size_bytes == 0 {
            0.0
        } else {
            (self.size_bytes as f64 / self.max_size_bytes as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    fn make_test_buffer(width: u32, height: u32) -> ImageBuffer {
        ImageBuffer::from_color(width, height, Color::RED)
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = ImageCache::with_defaults();

        let buffer = make_test_buffer(100, 100);
        cache.insert_file("test.png", buffer.clone());

        assert_eq!(cache.len(), 1);
        assert!(cache.contains_file("test.png"));

        let cached = cache.get_file("test.png");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().width(), 100);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ImageCache::with_defaults();

        let result = cache.get_file("nonexistent.png");
        assert!(result.is_none());
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = ImageCache::with_defaults();

        let buffer = make_test_buffer(10, 10);
        cache.insert_file("test.png", buffer);

        // 1 hit
        let _ = cache.get_file("test.png");
        // 1 miss
        let _ = cache.get_file("other.png");
        // 1 hit
        let _ = cache.get_file("test.png");

        assert_eq!(cache.hits(), 2);
        assert_eq!(cache.misses(), 1);
        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = ImageCache::with_defaults();

        let buffer = make_test_buffer(100, 100);
        cache.insert_file("test.png", buffer);

        assert!(cache.contains_file("test.png"));

        let removed = cache.remove_file("test.png");
        assert!(removed.is_some());
        assert!(!cache.contains_file("test.png"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ImageCache::with_defaults();

        cache.insert_file("a.png", make_test_buffer(10, 10));
        cache.insert_file("b.png", make_test_buffer(10, 10));
        cache.insert_file("c.png", make_test_buffer(10, 10));

        assert_eq!(cache.len(), 3);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        // Create a small cache (40KB limit)
        let config = ImageCacheConfig::default().with_max_size_bytes(40_000);
        let mut cache = ImageCache::new(config);

        // Each 100x100 image is ~40KB (100*100*4 = 40000)
        // Insert first image - should fit
        cache.insert_file("a.png", make_test_buffer(100, 100));
        assert!(cache.contains_file("a.png"));

        // Insert second image - should evict first
        cache.insert_file("b.png", make_test_buffer(100, 100));
        assert!(!cache.contains_file("a.png"));
        assert!(cache.contains_file("b.png"));
    }

    #[test]
    fn test_lru_access_order() {
        // Cache that can hold 2 images
        let config = ImageCacheConfig::default().with_max_size_bytes(80_000);
        let mut cache = ImageCache::new(config);

        // Insert a and b
        cache.insert_file("a.png", make_test_buffer(100, 100));
        cache.insert_file("b.png", make_test_buffer(100, 100));

        // Access a (moves it to front)
        let _ = cache.get_file("a.png");

        // Insert c - should evict b (least recently used)
        cache.insert_file("c.png", make_test_buffer(100, 100));

        assert!(cache.contains_file("a.png"));
        assert!(!cache.contains_file("b.png"));
        assert!(cache.contains_file("c.png"));
    }

    #[test]
    fn test_oversized_entry() {
        // Cache with 1KB limit
        let config = ImageCacheConfig::default().with_max_size_bytes(1024);
        let mut cache = ImageCache::new(config);

        // Try to insert a 40KB image - should not be inserted
        cache.insert_file("big.png", make_test_buffer(100, 100));

        assert!(!cache.contains_file("big.png"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_keys() {
        let mut cache = ImageCache::with_defaults();

        // File key
        cache.insert(CacheKey::from_file("test.png"), make_test_buffer(10, 10));
        assert!(cache.contains(&CacheKey::from_file("test.png")));

        // URL key
        cache.insert(
            CacheKey::from_url("https://example.com/img.png"),
            make_test_buffer(10, 10),
        );
        assert!(cache.contains_url("https://example.com/img.png"));

        // Bytes key
        let data = b"test data";
        cache.insert(CacheKey::from_bytes(data), make_test_buffer(10, 10));
        assert!(cache.contains(&CacheKey::from_bytes(data)));

        // Custom key
        cache.insert(CacheKey::custom("my-key"), make_test_buffer(10, 10));
        assert!(cache.contains(&CacheKey::custom("my-key")));
    }

    #[test]
    fn test_update_existing_entry() {
        let mut cache = ImageCache::with_defaults();

        // Insert initial
        cache.insert_file("test.png", make_test_buffer(10, 10));
        let initial_size = cache.size_bytes();

        // Update with larger image
        cache.insert_file("test.png", make_test_buffer(20, 20));

        // Should still be 1 entry but different size
        assert_eq!(cache.len(), 1);
        assert!(cache.size_bytes() > initial_size);
    }

    #[test]
    fn test_stats() {
        let config = ImageCacheConfig::default().with_max_size_mb(10);
        let mut cache = ImageCache::new(config);

        cache.insert_file("test.png", make_test_buffer(100, 100));
        let _ = cache.get_file("test.png");
        let _ = cache.get_file("missing.png");

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.size_bytes, 100 * 100 * 4);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }
}
