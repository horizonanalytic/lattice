//! Icon theme discovery and loading.
//!
//! This module handles discovering and loading icon themes from the filesystem,
//! following platform-specific conventions (freedesktop on Linux, system directories
//! on macOS/Windows).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{IconContext, IconSizeType, IconThemeDirectory, IconThemeInfo};

/// Icon theme loader that discovers and loads icon themes from the filesystem.
#[derive(Debug)]
pub struct IconThemeLoader {
    /// Search paths for icon themes
    search_paths: Vec<PathBuf>,
    /// Discovered themes (theme_id -> info)
    themes: HashMap<String, IconThemeInfo>,
    /// Default theme ID
    default_theme: Option<String>,
}

impl IconThemeLoader {
    /// Create a new icon theme loader with default platform search paths.
    pub fn new() -> Self {
        Self {
            search_paths: Self::default_search_paths(),
            themes: HashMap::new(),
            default_theme: None,
        }
    }

    /// Create a loader with custom search paths.
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths: paths,
            themes: HashMap::new(),
            default_theme: None,
        }
    }

    /// Add a search path.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Get current search paths.
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Discover all available icon themes.
    ///
    /// This scans all search paths and parses index.theme files.
    pub fn discover_themes(&mut self) -> crate::Result<usize> {
        self.themes.clear();
        let mut count = 0;

        for search_path in &self.search_paths.clone() {
            if !search_path.is_dir() {
                continue;
            }

            // Each subdirectory could be a theme
            let entries = match fs::read_dir(search_path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                // Check for index.theme
                let index_path = path.join("index.theme");
                if index_path.exists() {
                    if let Ok(info) = self.parse_theme(&path) {
                        let id = info.id.clone();
                        // Merge base paths if theme already exists
                        if let Some(existing) = self.themes.get_mut(&id) {
                            for bp in &info.base_paths {
                                if !existing.base_paths.contains(bp) {
                                    existing.base_paths.push(bp.clone());
                                }
                            }
                        } else {
                            self.themes.insert(id, info);
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Get a discovered theme by ID.
    pub fn get_theme(&self, id: &str) -> Option<&IconThemeInfo> {
        self.themes.get(id)
    }

    /// Get all discovered themes.
    pub fn themes(&self) -> impl Iterator<Item = &IconThemeInfo> {
        self.themes.values()
    }

    /// Get theme IDs.
    pub fn theme_ids(&self) -> impl Iterator<Item = &str> {
        self.themes.keys().map(|s| s.as_str())
    }

    /// Check if a theme exists.
    pub fn has_theme(&self, id: &str) -> bool {
        self.themes.contains_key(id)
    }

    /// Set the default theme.
    pub fn set_default_theme(&mut self, id: impl Into<String>) {
        self.default_theme = Some(id.into());
    }

    /// Get the default theme.
    pub fn default_theme(&self) -> Option<&IconThemeInfo> {
        self.default_theme
            .as_ref()
            .and_then(|id| self.themes.get(id))
            .or_else(|| self.themes.get("hicolor"))
    }

    /// Parse a theme directory.
    fn parse_theme(&self, theme_path: &Path) -> crate::Result<IconThemeInfo> {
        let theme_id = theme_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let index_path = theme_path.join("index.theme");
        let content = fs::read_to_string(&index_path).map_err(|e| {
            crate::Error::io(&index_path, e)
        })?;

        let mut info = IconThemeInfo::new(&theme_id);
        info.base_paths.push(theme_path.to_path_buf());

        // Parse INI-style index.theme
        let mut current_section = String::new();
        let mut directory_sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut directories_list: Vec<String> = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Key=Value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                if current_section == "Icon Theme" {
                    match key {
                        "Name" => info.name = value.to_string(),
                        "Comment" => info.comment = Some(value.to_string()),
                        "Inherits" => {
                            info.inherits = value.split(',').map(|s| s.trim().to_string()).collect();
                        }
                        "Hidden" => info.hidden = value.eq_ignore_ascii_case("true"),
                        "Example" => info.example = Some(value.to_string()),
                        "Directories" | "ScaledDirectories" => {
                            for dir in value.split(',') {
                                let dir = dir.trim().to_string();
                                if !dir.is_empty() && !directories_list.contains(&dir) {
                                    directories_list.push(dir);
                                }
                            }
                        }
                        _ => {}
                    }
                } else if !current_section.is_empty() {
                    // This is a directory section
                    directory_sections
                        .entry(current_section.clone())
                        .or_default()
                        .insert(key.to_string(), value.to_string());
                }
            }
        }

        // Parse directory sections
        for dir_path in directories_list {
            if let Some(section) = directory_sections.get(&dir_path) {
                if let Some(dir) = self.parse_directory_section(&dir_path, section) {
                    info.directories.push(dir);
                }
            }
        }

        // Use theme ID as name if name wasn't set
        if info.name.is_empty() {
            info.name = info.id.clone();
        }

        Ok(info)
    }

    /// Parse a directory section from index.theme.
    fn parse_directory_section(
        &self,
        path: &str,
        section: &HashMap<String, String>,
    ) -> Option<IconThemeDirectory> {
        // Size is required
        let size: u32 = section.get("Size")?.parse().ok()?;

        let scale = section
            .get("Scale")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let context = section
            .get("Context")
            .and_then(|s| IconContext::from_str(s));

        let size_type = section
            .get("Type")
            .map(|s| match s.to_lowercase().as_str() {
                "fixed" => IconSizeType::Fixed,
                "scalable" => IconSizeType::Scalable,
                "threshold" => IconSizeType::Threshold,
                _ => IconSizeType::Threshold,
            })
            .unwrap_or(IconSizeType::Threshold);

        let min_size = section.get("MinSize").and_then(|s| s.parse().ok());
        let max_size = section.get("MaxSize").and_then(|s| s.parse().ok());
        let threshold = section
            .get("Threshold")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);

        Some(IconThemeDirectory {
            path: path.to_string(),
            size,
            scale,
            context,
            size_type,
            min_size,
            max_size,
            threshold,
        })
    }

    /// Get default search paths for the current platform.
    #[cfg(target_os = "linux")]
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // User icons (XDG_DATA_HOME/icons or ~/.local/share/icons)
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            paths.push(PathBuf::from(data_home).join("icons"));
        } else if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(&home).join(".local/share/icons"));
            paths.push(PathBuf::from(&home).join(".icons"));
        }

        // System icons (XDG_DATA_DIRS/icons)
        if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in data_dirs.split(':') {
                paths.push(PathBuf::from(dir).join("icons"));
            }
        } else {
            paths.push(PathBuf::from("/usr/local/share/icons"));
            paths.push(PathBuf::from("/usr/share/icons"));
        }

        // Pixmaps fallback
        paths.push(PathBuf::from("/usr/share/pixmaps"));

        paths
    }

    /// Get default search paths for macOS.
    #[cfg(target_os = "macos")]
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // User Application Support
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Application Support/Icons"));
        }

        // System Application Support
        paths.push(PathBuf::from("/Library/Application Support/Icons"));

        // If running from a bundle, check Resources
        if let Ok(exe) = std::env::current_exe() {
            if let Some(bundle) = exe.parent().and_then(|p| p.parent()) {
                paths.push(bundle.join("Resources/icons"));
            }
        }

        paths
    }

    /// Get default search paths for Windows.
    #[cfg(target_os = "windows")]
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // User local app data
        if let Some(local) = dirs::data_local_dir() {
            paths.push(local.join("Icons"));
        }

        // Program files common
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.push(PathBuf::from(program_data).join("Icons"));
        }

        paths
    }

    /// Fallback for other platforms.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Try home directory
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".local/share/icons"));
            paths.push(home.join(".icons"));
        }

        paths
    }
}

impl Default for IconThemeLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = IconThemeLoader::new();
        assert!(!loader.search_paths().is_empty());
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = IconThemeLoader::new();
        let path = PathBuf::from("/custom/icons");
        loader.add_search_path(&path);
        assert!(loader.search_paths().contains(&path));

        // Adding same path twice shouldn't duplicate
        let prev_len = loader.search_paths().len();
        loader.add_search_path(&path);
        assert_eq!(loader.search_paths().len(), prev_len);
    }

    #[test]
    fn test_parse_directory_section() {
        let loader = IconThemeLoader::new();

        let mut section = HashMap::new();
        section.insert("Size".to_string(), "24".to_string());
        section.insert("Scale".to_string(), "1".to_string());
        section.insert("Context".to_string(), "actions".to_string());
        section.insert("Type".to_string(), "Fixed".to_string());

        let dir = loader
            .parse_directory_section("24x24/actions", &section)
            .unwrap();

        assert_eq!(dir.path, "24x24/actions");
        assert_eq!(dir.size, 24);
        assert_eq!(dir.scale, 1);
        assert_eq!(dir.context, Some(IconContext::Actions));
        assert_eq!(dir.size_type, IconSizeType::Fixed);
    }

    #[test]
    fn test_parse_directory_section_scalable() {
        let loader = IconThemeLoader::new();

        let mut section = HashMap::new();
        section.insert("Size".to_string(), "48".to_string());
        section.insert("Type".to_string(), "Scalable".to_string());
        section.insert("MinSize".to_string(), "16".to_string());
        section.insert("MaxSize".to_string(), "256".to_string());

        let dir = loader
            .parse_directory_section("scalable/actions", &section)
            .unwrap();

        assert_eq!(dir.size_type, IconSizeType::Scalable);
        assert_eq!(dir.min_size, Some(16));
        assert_eq!(dir.max_size, Some(256));
    }
}
