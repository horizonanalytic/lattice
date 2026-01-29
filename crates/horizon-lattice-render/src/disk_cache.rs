//! Disk-based image cache for downloaded images.
//!
//! This module provides persistent caching for images downloaded from URLs.
//! It stores the raw downloaded bytes on disk to avoid re-downloading.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{DiskImageCache, DiskCacheConfig};
//!
//! let config = DiskCacheConfig::default()
//!     .with_max_size_mb(500);
//!
//! let cache = DiskImageCache::new(config)?;
//!
//! // Check if URL is cached
//! if let Some(data) = cache.get("https://example.com/image.png")? {
//!     // Use cached bytes
//! } else {
//!     // Download and cache
//!     let data = download_image(url);
//!     cache.insert("https://example.com/image.png", &data)?;
//! }
//! ```

use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::{RenderError, RenderResult};

/// Configuration for the disk image cache.
#[derive(Debug, Clone)]
pub struct DiskCacheConfig {
    /// Root directory for the cache.
    /// Default: system temp directory / "horizon-lattice-image-cache".
    pub cache_dir: PathBuf,
    /// Maximum cache size in bytes.
    /// Default: 500 MB.
    pub max_size_bytes: u64,
    /// Time-to-live for cache entries.
    /// Entries older than this will be considered stale.
    /// Default: 7 days. Set to None for no expiration.
    pub ttl: Option<Duration>,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        let cache_dir = std::env::temp_dir().join("horizon-lattice-image-cache");
        Self {
            cache_dir,
            max_size_bytes: 500 * 1024 * 1024, // 500 MB
            ttl: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
        }
    }
}

impl DiskCacheConfig {
    /// Set the cache directory.
    #[must_use]
    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = path.into();
        self
    }

    /// Set the maximum cache size in megabytes.
    #[must_use]
    pub fn with_max_size_mb(mut self, mb: u64) -> Self {
        self.max_size_bytes = mb * 1024 * 1024;
        self
    }

    /// Set the maximum cache size in bytes.
    #[must_use]
    pub fn with_max_size_bytes(mut self, bytes: u64) -> Self {
        self.max_size_bytes = bytes;
        self
    }

    /// Set the time-to-live for cache entries.
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Disable time-to-live (entries never expire).
    #[must_use]
    pub fn without_ttl(mut self) -> Self {
        self.ttl = None;
        self
    }
}

/// Metadata for a cache entry.
#[derive(Debug, Clone)]
struct CacheEntryMeta {
    /// Size of the cached file in bytes.
    size: u64,
    /// When the entry was created.
    created: SystemTime,
    /// When the entry was last accessed.
    last_accessed: SystemTime,
}

/// A disk-based cache for downloaded images.
///
/// This cache stores downloaded image data on disk, keyed by URL. It uses
/// a hash of the URL as the filename to avoid filesystem issues with
/// special characters.
///
/// # Size Management
///
/// When the cache exceeds its size limit, the oldest entries (by last access
/// time) are automatically evicted.
///
/// # Thread Safety
///
/// This cache is NOT thread-safe. For concurrent access, wrap it in a `Mutex`
/// or use separate caches per thread.
pub struct DiskImageCache {
    /// Configuration.
    config: DiskCacheConfig,
    /// Metadata for cached entries (URL -> metadata).
    entries: HashMap<String, CacheEntryMeta>,
    /// Current total size in bytes.
    current_size: u64,
}

impl DiskImageCache {
    /// Create a new disk cache with the given configuration.
    ///
    /// Creates the cache directory if it doesn't exist.
    pub fn new(config: DiskCacheConfig) -> RenderResult<Self> {
        // Create cache directory
        fs::create_dir_all(&config.cache_dir).map_err(|e| {
            RenderError::ImageLoad(format!(
                "Failed to create cache directory {:?}: {}",
                config.cache_dir, e
            ))
        })?;

        let mut cache = Self {
            config,
            entries: HashMap::new(),
            current_size: 0,
        };

        // Load existing cache entries
        cache.scan_cache_dir()?;

        Ok(cache)
    }

    /// Create a new disk cache with default configuration.
    pub fn with_defaults() -> RenderResult<Self> {
        Self::new(DiskCacheConfig::default())
    }

    /// Scan the cache directory and load metadata for existing entries.
    fn scan_cache_dir(&mut self) -> RenderResult<()> {
        let entries = fs::read_dir(&self.config.cache_dir).map_err(|e| {
            RenderError::ImageLoad(format!(
                "Failed to read cache directory {:?}: {}",
                self.config.cache_dir, e
            ))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // The filename is the hash, we need to read the URL from metadata
                    // For simplicity, we just track the file by its hash
                    if let Ok(metadata) = entry.metadata() {
                        let size = metadata.len();
                        let created = metadata.created().unwrap_or(SystemTime::UNIX_EPOCH);
                        let last_accessed = metadata.accessed().unwrap_or(created);

                        // Use the filename as a pseudo-URL for tracking
                        let url = file_name.to_string();
                        self.entries.insert(
                            url,
                            CacheEntryMeta {
                                size,
                                created,
                                last_accessed,
                            },
                        );
                        self.current_size += size;
                    }
                }
        }

        Ok(())
    }

    /// Get the path for a cached URL.
    #[allow(dead_code)] // Reserved for future cache retrieval API
    fn cache_path(&self, url: &str) -> PathBuf {
        let hash = Self::hash_url(url);
        self.config.cache_dir.join(hash)
    }

    /// Hash a URL to create a safe filename.
    fn hash_url(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Get the current cache size in bytes.
    #[inline]
    pub fn size_bytes(&self) -> u64 {
        self.current_size
    }

    /// Get the maximum cache size in bytes.
    #[inline]
    pub fn max_size_bytes(&self) -> u64 {
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

    /// Get the cache directory path.
    #[inline]
    pub fn cache_dir(&self) -> &Path {
        &self.config.cache_dir
    }

    /// Check if a URL is cached.
    pub fn contains(&self, url: &str) -> bool {
        let hash = Self::hash_url(url);
        if let Some(meta) = self.entries.get(&hash) {
            // Check TTL
            if let Some(ttl) = self.config.ttl
                && let Ok(age) = meta.created.elapsed()
                    && age > ttl {
                        return false;
                    }
            true
        } else {
            false
        }
    }

    /// Get cached data for a URL.
    ///
    /// Returns `None` if the URL is not cached or the entry has expired.
    pub fn get(&mut self, url: &str) -> RenderResult<Option<Vec<u8>>> {
        let hash = Self::hash_url(url);

        // Check if entry exists
        if let Some(meta) = self.entries.get(&hash) {
            // Check TTL
            if let Some(ttl) = self.config.ttl
                && let Ok(age) = meta.created.elapsed()
                    && age > ttl {
                        // Entry has expired, remove it
                        self.remove(url)?;
                        return Ok(None);
                    }

            // Read the file
            let path = self.config.cache_dir.join(&hash);
            let mut file = File::open(&path).map_err(|e| {
                RenderError::ImageLoad(format!("Failed to open cache file {:?}: {}", path, e))
            })?;

            let mut data = Vec::new();
            file.read_to_end(&mut data).map_err(|e| {
                RenderError::ImageLoad(format!("Failed to read cache file {:?}: {}", path, e))
            })?;

            // Update last accessed time in our tracking
            if let Some(entry) = self.entries.get_mut(&hash) {
                entry.last_accessed = SystemTime::now();
            }

            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Insert data into the cache for a URL.
    ///
    /// If the cache exceeds its size limit, oldest entries are evicted.
    pub fn insert(&mut self, url: &str, data: &[u8]) -> RenderResult<()> {
        let hash = Self::hash_url(url);
        let size = data.len() as u64;

        // Remove existing entry if present
        if self.entries.contains_key(&hash) {
            self.remove(url)?;
        }

        // Evict entries if necessary
        while self.current_size + size > self.config.max_size_bytes && !self.entries.is_empty() {
            self.evict_oldest()?;
        }

        // Don't insert if single entry is larger than max size
        if size > self.config.max_size_bytes {
            return Ok(());
        }

        // Write the file
        let path = self.config.cache_dir.join(&hash);
        let mut file = File::create(&path).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to create cache file {:?}: {}", path, e))
        })?;

        file.write_all(data).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to write cache file {:?}: {}", path, e))
        })?;

        // Track the entry
        let now = SystemTime::now();
        self.entries.insert(
            hash,
            CacheEntryMeta {
                size,
                created: now,
                last_accessed: now,
            },
        );
        self.current_size += size;

        Ok(())
    }

    /// Remove a cached entry.
    pub fn remove(&mut self, url: &str) -> RenderResult<bool> {
        let hash = Self::hash_url(url);

        if let Some(meta) = self.entries.remove(&hash) {
            self.current_size -= meta.size;

            // Delete the file
            let path = self.config.cache_dir.join(&hash);
            if path.exists() {
                fs::remove_file(&path).map_err(|e| {
                    RenderError::ImageLoad(format!("Failed to delete cache file {:?}: {}", path, e))
                })?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Evict the oldest entry (by last access time).
    fn evict_oldest(&mut self) -> RenderResult<()> {
        // Find the oldest entry
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, meta)| meta.last_accessed)
            .map(|(hash, _)| hash.clone());

        if let Some(hash) = oldest
            && let Some(meta) = self.entries.remove(&hash) {
                self.current_size -= meta.size;

                // Delete the file
                let path = self.config.cache_dir.join(&hash);
                if path.exists() {
                    let _ = fs::remove_file(&path);
                }
            }

        Ok(())
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) -> RenderResult<()> {
        // Delete all files
        for hash in self.entries.keys() {
            let path = self.config.cache_dir.join(hash);
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }

        self.entries.clear();
        self.current_size = 0;

        Ok(())
    }

    /// Remove all expired entries.
    pub fn prune_expired(&mut self) -> RenderResult<usize> {
        let ttl = match self.config.ttl {
            Some(ttl) => ttl,
            None => return Ok(0), // No TTL, nothing to prune
        };

        let mut expired = Vec::new();

        for (hash, meta) in &self.entries {
            if let Ok(age) = meta.created.elapsed()
                && age > ttl {
                    expired.push(hash.clone());
                }
        }

        let count = expired.len();

        for hash in expired {
            if let Some(meta) = self.entries.remove(&hash) {
                self.current_size -= meta.size;

                let path = self.config.cache_dir.join(&hash);
                if path.exists() {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        Ok(count)
    }

    /// Get cache statistics.
    pub fn stats(&self) -> DiskCacheStats {
        DiskCacheStats {
            entries: self.entries.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.config.max_size_bytes,
            cache_dir: self.config.cache_dir.clone(),
        }
    }
}

impl std::fmt::Debug for DiskImageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskImageCache")
            .field("entries", &self.entries.len())
            .field("size_mb", &(self.current_size as f64 / 1024.0 / 1024.0))
            .field(
                "max_size_mb",
                &(self.config.max_size_bytes as f64 / 1024.0 / 1024.0),
            )
            .field("cache_dir", &self.config.cache_dir)
            .finish()
    }
}

/// Statistics about the disk cache.
#[derive(Debug, Clone)]
pub struct DiskCacheStats {
    /// Number of entries in the cache.
    pub entries: usize,
    /// Current size in bytes.
    pub size_bytes: u64,
    /// Maximum size in bytes.
    pub max_size_bytes: u64,
    /// Cache directory path.
    pub cache_dir: PathBuf,
}

impl DiskCacheStats {
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_cache_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!(
            "horizon-test-cache-{}-{}-{}",
            pid, timestamp, counter
        ))
    }

    #[test]
    fn test_disk_cache_insert_and_get() {
        let cache_dir = temp_cache_dir();
        let config = DiskCacheConfig::default().with_cache_dir(&cache_dir);
        let mut cache = DiskImageCache::new(config).unwrap();

        let url = "https://example.com/test.png";
        let data = b"test image data";

        cache.insert(url, data).unwrap();
        assert!(cache.contains(url));

        let retrieved = cache.get(url).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_miss() {
        let cache_dir = temp_cache_dir();
        let config = DiskCacheConfig::default().with_cache_dir(&cache_dir);
        let mut cache = DiskImageCache::new(config).unwrap();

        let result = cache.get("https://example.com/nonexistent.png").unwrap();
        assert!(result.is_none());

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_remove() {
        let cache_dir = temp_cache_dir();
        let config = DiskCacheConfig::default().with_cache_dir(&cache_dir);
        let mut cache = DiskImageCache::new(config).unwrap();

        let url = "https://example.com/test.png";
        cache.insert(url, b"data").unwrap();
        assert!(cache.contains(url));

        let removed = cache.remove(url).unwrap();
        assert!(removed);
        assert!(!cache.contains(url));

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_clear() {
        let cache_dir = temp_cache_dir();
        let config = DiskCacheConfig::default().with_cache_dir(&cache_dir);
        let mut cache = DiskImageCache::new(config).unwrap();

        cache.insert("https://a.com/1.png", b"a").unwrap();
        cache.insert("https://b.com/2.png", b"b").unwrap();
        cache.insert("https://c.com/3.png", b"c").unwrap();

        assert_eq!(cache.len(), 3);

        cache.clear().unwrap();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_eviction() {
        let cache_dir = temp_cache_dir();
        // Cache that can only hold ~100 bytes
        let config = DiskCacheConfig::default()
            .with_cache_dir(&cache_dir)
            .with_max_size_bytes(100);
        let mut cache = DiskImageCache::new(config).unwrap();

        // Insert data that fills the cache
        cache.insert("https://a.com/1.png", &[0u8; 50]).unwrap();
        cache.insert("https://b.com/2.png", &[0u8; 50]).unwrap();

        // Insert more data - should evict oldest
        cache.insert("https://c.com/3.png", &[0u8; 50]).unwrap();

        assert_eq!(cache.len(), 2);
        assert!(cache.size_bytes() <= 100);

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_ttl() {
        let cache_dir = temp_cache_dir();
        // Very short TTL for testing
        let config = DiskCacheConfig::default()
            .with_cache_dir(&cache_dir)
            .with_ttl(Duration::from_millis(50));
        let mut cache = DiskImageCache::new(config).unwrap();

        let url = "https://example.com/test.png";
        cache.insert(url, b"data").unwrap();
        assert!(cache.contains(url));

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(100));

        // Entry should be expired
        let result = cache.get(url).unwrap();
        assert!(result.is_none());

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_disk_cache_stats() {
        let cache_dir = temp_cache_dir();
        let config = DiskCacheConfig::default()
            .with_cache_dir(&cache_dir)
            .with_max_size_mb(10);
        let mut cache = DiskImageCache::new(config).unwrap();

        cache
            .insert("https://example.com/test.png", b"test data")
            .unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.size_bytes, 9); // "test data" is 9 bytes
        assert_eq!(stats.cache_dir, cache_dir);

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }

    #[test]
    fn test_oversized_entry() {
        let cache_dir = temp_cache_dir();
        // Cache with 10 byte limit
        let config = DiskCacheConfig::default()
            .with_cache_dir(&cache_dir)
            .with_max_size_bytes(10);
        let mut cache = DiskImageCache::new(config).unwrap();

        // Try to insert 100 bytes - should not be inserted
        cache
            .insert("https://example.com/big.png", &[0u8; 100])
            .unwrap();

        assert_eq!(cache.len(), 0);

        // Cleanup
        let _ = fs::remove_dir_all(&cache_dir);
    }
}
