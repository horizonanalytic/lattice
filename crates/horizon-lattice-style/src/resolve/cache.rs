//! Style caching for performance.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use horizon_lattice_core::ObjectId;
use crate::style::ComputedStyle;
use crate::selector::WidgetState;

/// Cache key for computed styles.
///
/// The key combines widget ID with relevant state to ensure cache
/// invalidation when state changes affect styling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyleCacheKey {
    widget_id: ObjectId,
    state_hash: u64,
}

impl StyleCacheKey {
    /// Create a new cache key.
    pub fn new(widget_id: ObjectId, state: &WidgetState) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        state.hovered.hash(&mut hasher);
        state.pressed.hash(&mut hasher);
        state.focused.hash(&mut hasher);
        state.enabled.hash(&mut hasher);
        state.checked.hash(&mut hasher);

        Self {
            widget_id,
            state_hash: hasher.finish(),
        }
    }
}

/// LRU-like cache for computed styles.
pub struct StyleCache {
    /// Cached computed styles.
    cache: HashMap<StyleCacheKey, ComputedStyle>,
    /// Maximum cache entries.
    max_size: usize,
}

impl StyleCache {
    /// Create a new style cache.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a cache with specific capacity.
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    /// Get a cached style.
    pub fn get(&self, key: &StyleCacheKey) -> Option<&ComputedStyle> {
        self.cache.get(key)
    }

    /// Insert a computed style into the cache.
    pub fn insert(&mut self, key: StyleCacheKey, style: ComputedStyle) {
        // Simple eviction: clear half when full
        if self.cache.len() >= self.max_size {
            self.evict_half();
        }
        self.cache.insert(key, style);
    }

    /// Invalidate cache entry for a specific widget.
    pub fn invalidate(&mut self, widget_id: ObjectId) {
        self.cache.retain(|k, _| k.widget_id != widget_id);
    }

    /// Invalidate all cached styles.
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Evict half the entries (simple LRU approximation).
    fn evict_half(&mut self) {
        let target = self.cache.len() / 2;
        let keys: Vec<_> = self.cache.keys().take(target).cloned().collect();
        for key in keys {
            self.cache.remove(&key);
        }
    }
}

impl Default for StyleCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_widget_id() -> ObjectId {
        // This is a bit hacky for testing - in real use, IDs come from the object system
        ObjectId::default()
    }

    #[test]
    fn cache_basic_operations() {
        let mut cache = StyleCache::new();
        let widget_id = make_widget_id();
        let state = WidgetState::default();
        let key = StyleCacheKey::new(widget_id, &state);

        assert!(cache.get(&key).is_none());

        cache.insert(key.clone(), ComputedStyle::default());
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn cache_state_differentiation() {
        let mut cache = StyleCache::new();
        let widget_id = make_widget_id();

        let state1 = WidgetState { hovered: false, ..Default::default() };
        let state2 = WidgetState { hovered: true, ..Default::default() };

        let key1 = StyleCacheKey::new(widget_id, &state1);
        let key2 = StyleCacheKey::new(widget_id, &state2);

        // Keys should be different due to state
        assert_ne!(key1, key2);

        let mut style1 = ComputedStyle::default();
        style1.font_size = 14.0;

        let mut style2 = ComputedStyle::default();
        style2.font_size = 16.0;

        cache.insert(key1.clone(), style1);
        cache.insert(key2.clone(), style2);

        assert_eq!(cache.get(&key1).unwrap().font_size, 14.0);
        assert_eq!(cache.get(&key2).unwrap().font_size, 16.0);
    }

    #[test]
    fn cache_invalidation() {
        let mut cache = StyleCache::new();
        let widget_id = make_widget_id();
        let state = WidgetState::default();
        let key = StyleCacheKey::new(widget_id, &state);

        cache.insert(key.clone(), ComputedStyle::default());
        assert!(cache.get(&key).is_some());

        cache.invalidate(widget_id);
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn cache_invalidate_all() {
        let mut cache = StyleCache::new();

        for _ in 0..10 {
            let widget_id = make_widget_id();
            let state = WidgetState::default();
            let key = StyleCacheKey::new(widget_id, &state);
            cache.insert(key, ComputedStyle::default());
        }

        assert!(!cache.is_empty());
        cache.invalidate_all();
        assert!(cache.is_empty());
    }
}
