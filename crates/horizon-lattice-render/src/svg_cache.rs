//! SVG rasterization cache.
//!
//! This module provides caching for rasterized SVG images to avoid re-rendering
//! the same SVG at the same size multiple times.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{SvgCache, SvgCacheConfig, SvgImage};
//!
//! let config = SvgCacheConfig::default()
//!     .with_max_size_mb(50);
//!
//! let mut cache = SvgCache::new(config);
//!
//! // Load and cache an SVG rasterization
//! let svg = SvgImage::from_file("icon.svg")?;
//! let rgba = cache.get_or_render(&svg, 48, 48);
//!
//! // Second call at same size returns cached data
//! let rgba2 = cache.get_or_render(&svg, 48, 48);
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::svg::SvgImage;

/// Configuration for the SVG cache.
#[derive(Debug, Clone)]
pub struct SvgCacheConfig {
    /// Maximum cache size in bytes.
    /// When exceeded, least recently used entries are evicted.
    /// Default: 50 MB.
    pub max_size_bytes: usize,
    /// Whether to track access patterns for LRU eviction.
    /// Default: true.
    pub enable_lru: bool,
}

impl Default for SvgCacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 50 * 1024 * 1024, // 50 MB
            enable_lru: true,
        }
    }
}

impl SvgCacheConfig {
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

/// Source identifier for SVG images.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SvgSource {
    /// SVG loaded from a file path.
    File(PathBuf),
    /// SVG loaded from bytes (identified by hash).
    Bytes(u64),
}

impl SvgSource {
    /// Create a source key from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        SvgSource::File(path.as_ref().to_path_buf())
    }

    /// Create a source key from raw bytes by hashing them.
    pub fn from_bytes(data: &[u8]) -> Self {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        SvgSource::Bytes(hasher.finish())
    }
}

/// Cache key for SVG rasterizations.
///
/// Includes the source and target dimensions since the same SVG
/// may be rasterized at multiple sizes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SvgCacheKey {
    /// The SVG source (file path or bytes hash).
    pub source: SvgSource,
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
}

impl SvgCacheKey {
    /// Create a new cache key.
    pub fn new(source: SvgSource, width: u32, height: u32) -> Self {
        Self {
            source,
            width,
            height,
        }
    }

    /// Create a cache key for a file at a specific size.
    pub fn file(path: impl AsRef<Path>, width: u32, height: u32) -> Self {
        Self::new(SvgSource::from_file(path), width, height)
    }

    /// Create a cache key for bytes at a specific size.
    pub fn bytes(data: &[u8], width: u32, height: u32) -> Self {
        Self::new(SvgSource::from_bytes(data), width, height)
    }
}

/// A cached SVG rasterization entry.
#[derive(Clone)]
struct CacheEntry {
    /// The rasterized RGBA pixel data.
    rgba_data: Vec<u8>,
    /// Size of this entry in bytes.
    size_bytes: usize,
}

impl CacheEntry {
    fn new(rgba_data: Vec<u8>) -> Self {
        let size_bytes = rgba_data.len();
        Self {
            rgba_data,
            size_bytes,
        }
    }
}

/// Node in the LRU linked list.
struct LruNode {
    #[allow(dead_code)]
    key: SvgCacheKey,
    prev: Option<SvgCacheKey>,
    next: Option<SvgCacheKey>,
}

/// An LRU cache for rasterized SVG images.
///
/// This cache stores the RGBA pixel data of rasterized SVGs, keyed by
/// source and dimensions. This avoids expensive re-rasterization when
/// the same SVG is used multiple times at the same size.
///
/// # Size Estimation
///
/// The cache tracks memory usage based on the actual RGBA data size
/// (width × height × 4 bytes).
///
/// # Thread Safety
///
/// This cache is NOT thread-safe. For concurrent access, wrap it in a `Mutex`.
pub struct SvgCache {
    /// Configuration.
    config: SvgCacheConfig,
    /// Cached entries by key.
    entries: HashMap<SvgCacheKey, CacheEntry>,
    /// LRU tracking: key -> node.
    lru_nodes: HashMap<SvgCacheKey, LruNode>,
    /// Head of LRU list (most recently used).
    lru_head: Option<SvgCacheKey>,
    /// Tail of LRU list (least recently used).
    lru_tail: Option<SvgCacheKey>,
    /// Current total size in bytes.
    current_size: usize,
    /// Statistics: number of cache hits.
    hits: u64,
    /// Statistics: number of cache misses.
    misses: u64,
}

impl SvgCache {
    /// Create a new SVG cache with the given configuration.
    pub fn new(config: SvgCacheConfig) -> Self {
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

    /// Create a new SVG cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SvgCacheConfig::default())
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

    /// Insert rasterized SVG data into the cache.
    ///
    /// If the key already exists, the entry is updated and moved to the front
    /// of the LRU list. If the cache exceeds its size limit, least recently
    /// used entries are evicted.
    pub fn insert(&mut self, key: SvgCacheKey, rgba_data: Vec<u8>) {
        let entry = CacheEntry::new(rgba_data);
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

    /// Get rasterized SVG data from the cache.
    ///
    /// If found, the entry is moved to the front of the LRU list.
    /// Returns `None` if the key is not in the cache.
    pub fn get(&mut self, key: &SvgCacheKey) -> Option<&[u8]> {
        if self.entries.contains_key(key) {
            self.hits += 1;
            // Move to front of LRU list
            if self.config.enable_lru {
                self.lru_move_to_front(key.clone());
            }
            self.entries.get(key).map(|e| e.rgba_data.as_slice())
        } else {
            self.misses += 1;
            None
        }
    }

    /// Get rasterized SVG data, or render and cache it if not present.
    ///
    /// This is the primary method for using the cache. It checks if the
    /// rasterization exists, and if not, renders the SVG and caches the result.
    pub fn get_or_render(&mut self, svg: &SvgImage, width: u32, height: u32) -> Vec<u8> {
        // Create key from SVG's internal data
        // Since we can't easily get the source path, we use a hash of the natural size
        // combined with a pointer address as a unique identifier
        let key = SvgCacheKey {
            source: SvgSource::Bytes(Self::svg_hash(svg)),
            width,
            height,
        };

        // Check cache first
        if let Some(cached) = self.get(&key) {
            return cached.to_vec();
        }

        // Render and cache
        let rgba = svg.render_to_rgba(width, height);
        self.insert(key, rgba.clone());
        rgba
    }

    /// Get rasterized SVG data with a file key, or render and cache it.
    ///
    /// Use this when you have the original file path for better cache key stability.
    pub fn get_or_render_file(
        &mut self,
        svg: &SvgImage,
        path: impl AsRef<Path>,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let key = SvgCacheKey::file(path, width, height);

        // Check cache first
        if let Some(cached) = self.get(&key) {
            return cached.to_vec();
        }

        // Render and cache
        let rgba = svg.render_to_rgba(width, height);
        self.insert(key, rgba.clone());
        rgba
    }

    /// Check if a key exists in the cache (without updating LRU order).
    pub fn contains(&self, key: &SvgCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove an entry from the cache.
    ///
    /// Returns the removed RGBA data if it existed.
    pub fn remove(&mut self, key: &SvgCacheKey) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.remove(key) {
            self.current_size -= entry.size_bytes;

            // Remove from LRU list
            if self.config.enable_lru {
                self.lru_remove(key);
            }

            Some(entry.rgba_data)
        } else {
            None
        }
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
    pub fn stats(&self) -> SvgCacheStats {
        SvgCacheStats {
            entries: self.entries.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.config.max_size_bytes,
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
        }
    }

    /// Generate a hash for an SvgImage based on its natural size.
    /// This is a workaround since we don't have direct access to the source data.
    fn svg_hash(svg: &SvgImage) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        // Hash the natural size as a simple identifier
        // In practice, different SVGs with same natural size would need file paths
        let size = svg.natural_size();
        ((size.width * 1000.0) as u64).hash(&mut hasher);
        ((size.height * 1000.0) as u64).hash(&mut hasher);
        // Include pointer address for uniqueness within a session
        (svg as *const SvgImage as usize).hash(&mut hasher);
        hasher.finish()
    }

    // ========================================================================
    // LRU LIST OPERATIONS
    // ========================================================================

    /// Push a key to the front of the LRU list.
    fn lru_push_front(&mut self, key: SvgCacheKey) {
        let node = LruNode {
            key: key.clone(),
            prev: None,
            next: self.lru_head.clone(),
        };

        // Update old head's prev pointer
        if let Some(old_head) = &self.lru_head {
            if let Some(old_node) = self.lru_nodes.get_mut(old_head) {
                old_node.prev = Some(key.clone());
            }
        }

        // Update tail if this is the first entry
        if self.lru_tail.is_none() {
            self.lru_tail = Some(key.clone());
        }

        self.lru_head = Some(key.clone());
        self.lru_nodes.insert(key, node);
    }

    /// Move a key to the front of the LRU list.
    fn lru_move_to_front(&mut self, key: SvgCacheKey) {
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
    fn lru_remove(&mut self, key: &SvgCacheKey) {
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

impl Default for SvgCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl std::fmt::Debug for SvgCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvgCache")
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

/// Statistics about the SVG cache.
#[derive(Debug, Clone)]
pub struct SvgCacheStats {
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

impl SvgCacheStats {
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

    const SIMPLE_SVG: &[u8] = br#"
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" fill="red"/>
        </svg>
    "#;

    #[test]
    fn test_cache_key_equality() {
        let key1 = SvgCacheKey::file("test.svg", 48, 48);
        let key2 = SvgCacheKey::file("test.svg", 48, 48);
        let key3 = SvgCacheKey::file("test.svg", 64, 64);
        let key4 = SvgCacheKey::file("other.svg", 48, 48);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SvgCache::with_defaults();

        let key = SvgCacheKey::file("test.svg", 48, 48);
        let data = vec![0u8; 48 * 48 * 4];

        cache.insert(key.clone(), data.clone());

        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&key));

        let cached = cache.get(&key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 48 * 48 * 4);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = SvgCache::with_defaults();

        let key = SvgCacheKey::file("nonexistent.svg", 48, 48);
        let result = cache.get(&key);
        assert!(result.is_none());
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = SvgCache::with_defaults();

        let key = SvgCacheKey::file("test.svg", 48, 48);
        cache.insert(key.clone(), vec![0u8; 100]);

        // 1 hit
        let _ = cache.get(&key);
        // 1 miss
        let _ = cache.get(&SvgCacheKey::file("other.svg", 48, 48));
        // 1 hit
        let _ = cache.get(&key);

        assert_eq!(cache.hits(), 2);
        assert_eq!(cache.misses(), 1);
        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = SvgCache::with_defaults();

        let key = SvgCacheKey::file("test.svg", 48, 48);
        cache.insert(key.clone(), vec![0u8; 100]);

        assert!(cache.contains(&key));

        let removed = cache.remove(&key);
        assert!(removed.is_some());
        assert!(!cache.contains(&key));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = SvgCache::with_defaults();

        cache.insert(SvgCacheKey::file("a.svg", 32, 32), vec![0u8; 100]);
        cache.insert(SvgCacheKey::file("b.svg", 32, 32), vec![0u8; 100]);
        cache.insert(SvgCacheKey::file("c.svg", 32, 32), vec![0u8; 100]);

        assert_eq!(cache.len(), 3);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        // Create a small cache (1KB limit)
        let config = SvgCacheConfig::default().with_max_size_bytes(1024);
        let mut cache = SvgCache::new(config);

        // Each entry is 512 bytes (fits 2 in cache)
        cache.insert(SvgCacheKey::file("a.svg", 16, 8), vec![0u8; 512]);
        cache.insert(SvgCacheKey::file("b.svg", 16, 8), vec![0u8; 512]);

        assert!(cache.contains(&SvgCacheKey::file("a.svg", 16, 8)));
        assert!(cache.contains(&SvgCacheKey::file("b.svg", 16, 8)));

        // Insert third - should evict first
        cache.insert(SvgCacheKey::file("c.svg", 16, 8), vec![0u8; 512]);

        assert!(!cache.contains(&SvgCacheKey::file("a.svg", 16, 8)));
        assert!(cache.contains(&SvgCacheKey::file("b.svg", 16, 8)));
        assert!(cache.contains(&SvgCacheKey::file("c.svg", 16, 8)));
    }

    #[test]
    fn test_lru_access_order() {
        // Cache that can hold 2 entries
        let config = SvgCacheConfig::default().with_max_size_bytes(1024);
        let mut cache = SvgCache::new(config);

        // Insert a and b
        cache.insert(SvgCacheKey::file("a.svg", 16, 8), vec![0u8; 512]);
        cache.insert(SvgCacheKey::file("b.svg", 16, 8), vec![0u8; 512]);

        // Access a (moves it to front)
        let _ = cache.get(&SvgCacheKey::file("a.svg", 16, 8));

        // Insert c - should evict b (least recently used)
        cache.insert(SvgCacheKey::file("c.svg", 16, 8), vec![0u8; 512]);

        assert!(cache.contains(&SvgCacheKey::file("a.svg", 16, 8)));
        assert!(!cache.contains(&SvgCacheKey::file("b.svg", 16, 8)));
        assert!(cache.contains(&SvgCacheKey::file("c.svg", 16, 8)));
    }

    #[test]
    fn test_oversized_entry() {
        // Cache with 100 byte limit
        let config = SvgCacheConfig::default().with_max_size_bytes(100);
        let mut cache = SvgCache::new(config);

        // Try to insert a 512 byte entry - should not be inserted
        cache.insert(SvgCacheKey::file("big.svg", 16, 8), vec![0u8; 512]);

        assert!(!cache.contains(&SvgCacheKey::file("big.svg", 16, 8)));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_same_svg_different_sizes() {
        let mut cache = SvgCache::with_defaults();

        // Same SVG at different sizes should be cached separately
        cache.insert(SvgCacheKey::file("icon.svg", 24, 24), vec![0u8; 24 * 24 * 4]);
        cache.insert(SvgCacheKey::file("icon.svg", 48, 48), vec![0u8; 48 * 48 * 4]);
        cache.insert(SvgCacheKey::file("icon.svg", 96, 96), vec![0u8; 96 * 96 * 4]);

        assert_eq!(cache.len(), 3);
        assert!(cache.contains(&SvgCacheKey::file("icon.svg", 24, 24)));
        assert!(cache.contains(&SvgCacheKey::file("icon.svg", 48, 48)));
        assert!(cache.contains(&SvgCacheKey::file("icon.svg", 96, 96)));
    }

    #[test]
    fn test_stats() {
        let config = SvgCacheConfig::default().with_max_size_mb(10);
        let mut cache = SvgCache::new(config);

        cache.insert(SvgCacheKey::file("test.svg", 48, 48), vec![0u8; 48 * 48 * 4]);
        let _ = cache.get(&SvgCacheKey::file("test.svg", 48, 48));
        let _ = cache.get(&SvgCacheKey::file("missing.svg", 48, 48));

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.size_bytes, 48 * 48 * 4);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_get_or_render() {
        let svg = SvgImage::from_bytes(SIMPLE_SVG).expect("Should parse valid SVG");
        let mut cache = SvgCache::with_defaults();

        // First call should render (miss)
        let rgba1 = cache.get_or_render(&svg, 48, 48);
        assert_eq!(rgba1.len(), 48 * 48 * 4);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);

        // Second call should hit cache
        let rgba2 = cache.get_or_render(&svg, 48, 48);
        assert_eq!(rgba2.len(), 48 * 48 * 4);
        assert_eq!(rgba1, rgba2);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 1);
    }
}
