//! Icon resolution and caching.
//!
//! This module provides the [`IconResolver`] which handles looking up icons
//! by name and resolving them to actual image files, with caching for performance.

use std::collections::HashMap;
use std::path::PathBuf;

use horizon_lattice_render::{Icon, IconSize, IconSource};

use super::loader::IconThemeLoader;
use super::types::{IconLookup, IconThemeInfo};

/// Cache key for resolved icons.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    name: String,
    size: IconSize,
    scale: u32,
}

/// Cached resolution result.
#[derive(Debug, Clone)]
enum CacheEntry {
    /// Found icon at this path
    Found(PathBuf),
    /// Icon not found
    NotFound,
}

/// Icon resolver with caching.
///
/// The resolver handles looking up icon names in themes, following the
/// inheritance chain, and caching results for performance.
#[derive(Debug)]
pub struct IconResolver {
    /// Theme loader for discovering themes
    loader: IconThemeLoader,
    /// Current theme ID
    current_theme: String,
    /// Resolution cache
    cache: HashMap<CacheKey, CacheEntry>,
    /// Maximum cache entries
    cache_limit: usize,
}

impl IconResolver {
    /// Create a new icon resolver.
    pub fn new() -> Self {
        Self {
            loader: IconThemeLoader::new(),
            current_theme: "hicolor".to_string(),
            cache: HashMap::new(),
            cache_limit: 1000,
        }
    }

    /// Create a resolver with a custom theme loader.
    pub fn with_loader(loader: IconThemeLoader) -> Self {
        Self {
            loader,
            current_theme: "hicolor".to_string(),
            cache: HashMap::new(),
            cache_limit: 1000,
        }
    }

    /// Get the theme loader.
    pub fn loader(&self) -> &IconThemeLoader {
        &self.loader
    }

    /// Get mutable access to the theme loader.
    pub fn loader_mut(&mut self) -> &mut IconThemeLoader {
        &mut self.loader
    }

    /// Discover themes from search paths.
    pub fn discover_themes(&mut self) -> crate::Result<usize> {
        self.loader.discover_themes()
    }

    /// Set the current theme.
    ///
    /// This clears the resolution cache.
    pub fn set_theme(&mut self, theme_id: impl Into<String>) -> crate::Result<()> {
        let theme_id = theme_id.into();

        if !self.loader.has_theme(&theme_id) {
            return Err(crate::Error::invalid_value(
                "theme_id",
                format!("Theme '{}' not found", theme_id),
            ));
        }

        self.current_theme = theme_id;
        self.clear_cache();
        Ok(())
    }

    /// Get the current theme ID.
    pub fn current_theme_id(&self) -> &str {
        &self.current_theme
    }

    /// Get the current theme info.
    pub fn current_theme(&self) -> Option<&IconThemeInfo> {
        self.loader.get_theme(&self.current_theme)
    }

    /// Clear the resolution cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Set the cache limit.
    pub fn set_cache_limit(&mut self, limit: usize) {
        self.cache_limit = limit;
        // Trim cache if needed
        while self.cache.len() > self.cache_limit {
            // Remove arbitrary entry (HashMap doesn't preserve order)
            if let Some(key) = self.cache.keys().next().cloned() {
                self.cache.remove(&key);
            }
        }
    }

    /// Resolve an icon by name and size.
    ///
    /// Returns the path to the icon file, or None if not found.
    pub fn resolve_path(&mut self, lookup: &IconLookup) -> Option<PathBuf> {
        let cache_key = CacheKey {
            name: lookup.name.as_str().to_string(),
            size: lookup.size,
            scale: lookup.scale,
        };

        // Check cache first
        if let Some(entry) = self.cache.get(&cache_key) {
            return match entry {
                CacheEntry::Found(path) => Some(path.clone()),
                CacheEntry::NotFound => None,
            };
        }

        // Resolve through theme chain
        let result = self.resolve_uncached(lookup);

        // Cache the result
        if self.cache.len() < self.cache_limit {
            let entry = match &result {
                Some(path) => CacheEntry::Found(path.clone()),
                None => CacheEntry::NotFound,
            };
            self.cache.insert(cache_key, entry);
        }

        result
    }

    /// Resolve an icon and return an Icon object.
    pub fn resolve(&mut self, lookup: &IconLookup) -> Option<Icon> {
        self.resolve_path(lookup).map(Icon::from_path)
    }

    /// Resolve an icon source.
    pub fn resolve_source(&mut self, lookup: &IconLookup) -> Option<IconSource> {
        self.resolve_path(lookup).map(IconSource::Path)
    }

    /// Resolve without caching.
    fn resolve_uncached(&self, lookup: &IconLookup) -> Option<PathBuf> {
        // Build the inheritance chain
        let mut themes_to_check = vec![self.current_theme.clone()];
        let mut visited = std::collections::HashSet::new();
        let mut idx = 0;

        while idx < themes_to_check.len() {
            let theme_id = &themes_to_check[idx];
            if visited.insert(theme_id.clone())
                && let Some(theme) = self.loader.get_theme(theme_id)
            {
                for parent in &theme.inherits {
                    if !visited.contains(parent) {
                        themes_to_check.push(parent.clone());
                    }
                }
            }
            idx += 1;
        }

        // Always check hicolor as final fallback
        if !visited.contains("hicolor") {
            themes_to_check.push("hicolor".to_string());
        }

        // Search through themes
        for theme_id in themes_to_check {
            if let Some(theme) = self.loader.get_theme(&theme_id)
                && let Some(path) = self.find_icon_in_theme(theme, lookup)
            {
                return Some(path);
            }
        }

        None
    }

    /// Find an icon in a specific theme.
    fn find_icon_in_theme(&self, theme: &IconThemeInfo, lookup: &IconLookup) -> Option<PathBuf> {
        let target_size = lookup.size.as_pixels();
        let icon_name = lookup.name.as_str();

        // Get matching directories, sorted by size distance
        let mut directories: Vec<_> = theme
            .directories
            .iter()
            .filter(|d| {
                d.scale == lookup.scale
                    && (lookup.context.is_none()
                        || d.context == lookup.context
                        || d.context.is_none())
            })
            .collect();

        // Sort by size distance (closest first)
        directories.sort_by_key(|d| d.size_distance(target_size));

        // If forcing exact size, filter to exact matches only
        if lookup.force_size {
            directories.retain(|d| d.size == target_size);
        }

        // Check each directory
        for dir in directories {
            for base_path in &theme.base_paths {
                let dir_path = base_path.join(&dir.path);

                // Try common extensions
                for ext in &["png", "svg", "xpm"] {
                    let icon_path = dir_path.join(format!("{}.{}", icon_name, ext));
                    if icon_path.exists() {
                        return Some(icon_path);
                    }
                }
            }
        }

        None
    }

    /// Resolve an icon with a simple API.
    pub fn get_icon(&mut self, name: &str, size: IconSize) -> Option<Icon> {
        let lookup = IconLookup::new(name, size);
        self.resolve(&lookup)
    }

    /// Get an icon path with a simple API.
    pub fn get_icon_path(&mut self, name: &str, size: IconSize) -> Option<PathBuf> {
        let lookup = IconLookup::new(name, size);
        self.resolve_path(&lookup)
    }

    /// Check if an icon exists.
    pub fn has_icon(&mut self, name: &str, size: IconSize) -> bool {
        self.get_icon_path(name, size).is_some()
    }

    /// Get all available sizes for an icon name.
    pub fn available_sizes(&self, name: &str) -> Vec<IconSize> {
        let mut sizes = Vec::new();

        // Check current theme and inheritance chain
        let mut themes_to_check = vec![self.current_theme.clone()];
        let mut visited = std::collections::HashSet::new();
        let mut idx = 0;

        while idx < themes_to_check.len() {
            let theme_id = &themes_to_check[idx];
            if visited.insert(theme_id.clone())
                && let Some(theme) = self.loader.get_theme(theme_id)
            {
                for parent in &theme.inherits {
                    if !visited.contains(parent) {
                        themes_to_check.push(parent.clone());
                    }
                }
            }
            idx += 1;
        }

        // Check each theme
        for theme_id in themes_to_check {
            if let Some(theme) = self.loader.get_theme(&theme_id) {
                for dir in &theme.directories {
                    if let Some(size) = IconSize::from_pixels(dir.size) {
                        // Check if icon actually exists in this directory
                        for base_path in &theme.base_paths {
                            let dir_path = base_path.join(&dir.path);
                            for ext in &["png", "svg", "xpm"] {
                                let icon_path = dir_path.join(format!("{}.{}", name, ext));
                                if icon_path.exists() && !sizes.contains(&size) {
                                    sizes.push(size);
                                }
                            }
                        }
                    }
                }
            }
        }

        sizes.sort();
        sizes
    }
}

impl Default for IconResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to look up a standard icon.
pub fn lookup_standard_icon(name: &str, size: IconSize) -> Option<PathBuf> {
    let mut resolver = IconResolver::new();
    let _ = resolver.discover_themes();
    resolver.get_icon_path(name, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = IconResolver::new();
        assert_eq!(resolver.current_theme_id(), "hicolor");
    }

    #[test]
    fn test_cache_key() {
        let key1 = CacheKey {
            name: "document-save".to_string(),
            size: IconSize::Size24,
            scale: 1,
        };
        let key2 = CacheKey {
            name: "document-save".to_string(),
            size: IconSize::Size24,
            scale: 1,
        };
        let key3 = CacheKey {
            name: "document-save".to_string(),
            size: IconSize::Size32,
            scale: 1,
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_set_cache_limit() {
        let mut resolver = IconResolver::new();
        resolver.set_cache_limit(100);
        assert_eq!(resolver.cache_limit, 100);
    }

    #[test]
    fn test_clear_cache() {
        let mut resolver = IconResolver::new();
        // Add some fake cache entries
        resolver.cache.insert(
            CacheKey {
                name: "test".to_string(),
                size: IconSize::Size24,
                scale: 1,
            },
            CacheEntry::NotFound,
        );
        assert!(!resolver.cache.is_empty());

        resolver.clear_cache();
        assert!(resolver.cache.is_empty());
    }
}
