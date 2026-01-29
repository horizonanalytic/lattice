//! Font system management and font database access.

use std::path::Path;

use fontdb::ID as FontFaceId;

use super::types::{FontMetrics, FontQuery, FontStretch, FontStyle, FontWeight};

/// Configuration for initializing the font system.
#[derive(Debug, Clone)]
pub struct FontSystemConfig {
    /// Whether to load system fonts on initialization.
    pub load_system_fonts: bool,
    /// Locale string for text shaping (e.g., "en-US").
    pub locale: String,
    /// Default serif font family name.
    pub serif_family: Option<String>,
    /// Default sans-serif font family name.
    pub sans_serif_family: Option<String>,
    /// Default monospace font family name.
    pub monospace_family: Option<String>,
    /// Default cursive font family name.
    pub cursive_family: Option<String>,
    /// Default fantasy font family name.
    pub fantasy_family: Option<String>,
}

impl Default for FontSystemConfig {
    fn default() -> Self {
        Self {
            load_system_fonts: true,
            locale: sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string()),
            serif_family: None,
            sans_serif_family: None,
            monospace_family: None,
            cursive_family: None,
            fantasy_family: None,
        }
    }
}

impl FontSystemConfig {
    /// Create a new configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to load system fonts on initialization.
    pub fn load_system_fonts(mut self, load: bool) -> Self {
        self.load_system_fonts = load;
        self
    }

    /// Set the locale for text shaping.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }

    /// Set the default serif font family.
    pub fn serif_family(mut self, family: impl Into<String>) -> Self {
        self.serif_family = Some(family.into());
        self
    }

    /// Set the default sans-serif font family.
    pub fn sans_serif_family(mut self, family: impl Into<String>) -> Self {
        self.sans_serif_family = Some(family.into());
        self
    }

    /// Set the default monospace font family.
    pub fn monospace_family(mut self, family: impl Into<String>) -> Self {
        self.monospace_family = Some(family.into());
        self
    }
}

/// The font system manages font loading, enumeration, and matching.
///
/// This is the central hub for all font-related operations. It wraps
/// cosmic-text's FontSystem and provides a higher-level API.
///
/// # Thread Safety
///
/// `FontSystem` is not `Sync` because cosmic-text's internal font system
/// uses interior mutability for caching. For multi-threaded applications,
/// wrap it in a `Mutex` or use one FontSystem per thread.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{FontSystem, FontQuery, FontFamily, FontWeight};
///
/// // Create with default settings (loads system fonts)
/// let font_system = FontSystem::new();
///
/// // Query for a font
/// let query = FontQuery::new()
///     .family(FontFamily::Name("Arial".into()))
///     .fallback(FontFamily::SansSerif)
///     .weight(FontWeight::NORMAL);
///
/// if let Some(face_id) = font_system.query(&query) {
///     // Use the font face
///     println!("Found font face: {:?}", face_id);
/// }
/// ```
pub struct FontSystem {
    inner: cosmic_text::FontSystem,
}

impl FontSystem {
    /// Create a new font system with default configuration.
    ///
    /// This will automatically load all system fonts, which may take
    /// around 1 second depending on the number of fonts installed.
    pub fn new() -> Self {
        Self::with_config(FontSystemConfig::default())
    }

    /// Create a new font system with custom configuration.
    pub fn with_config(config: FontSystemConfig) -> Self {
        let mut inner = if config.load_system_fonts {
            cosmic_text::FontSystem::new()
        } else {
            let db = fontdb::Database::new();
            cosmic_text::FontSystem::new_with_locale_and_db(config.locale.clone(), db)
        };

        // Apply default family overrides
        let db = inner.db_mut();
        if let Some(ref family) = config.serif_family {
            db.set_serif_family(family);
        }
        if let Some(ref family) = config.sans_serif_family {
            db.set_sans_serif_family(family);
        }
        if let Some(ref family) = config.monospace_family {
            db.set_monospace_family(family);
        }
        if let Some(ref family) = config.cursive_family {
            db.set_cursive_family(family);
        }
        if let Some(ref family) = config.fantasy_family {
            db.set_fantasy_family(family);
        }

        Self { inner }
    }

    /// Get a reference to the underlying cosmic-text font system.
    ///
    /// This is useful for advanced operations like creating text buffers.
    pub fn inner(&self) -> &cosmic_text::FontSystem {
        &self.inner
    }

    /// Get a mutable reference to the underlying cosmic-text font system.
    pub fn inner_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.inner
    }

    /// Get a reference to the font database.
    pub fn database(&self) -> &fontdb::Database {
        self.inner.db()
    }

    /// Get a mutable reference to the font database.
    pub fn database_mut(&mut self) -> &mut fontdb::Database {
        self.inner.db_mut()
    }

    /// Load system fonts into the database.
    ///
    /// This is automatically called if `load_system_fonts` is true in the config.
    /// Call this manually if you created a FontSystem without system fonts.
    pub fn load_system_fonts(&mut self) {
        self.inner.db_mut().load_system_fonts();
    }

    /// Load a font file from disk.
    ///
    /// Returns the font face IDs that were loaded (a font file may contain multiple faces).
    pub fn load_font_file(&mut self, path: impl AsRef<Path>) -> Result<(), FontLoadError> {
        self.inner
            .db_mut()
            .load_font_file(path.as_ref())
            .map_err(|e| FontLoadError::IoError(e.to_string()))
    }

    /// Load font data from memory.
    ///
    /// The data should be the raw contents of a TTF, OTF, TTC, or OTC file.
    pub fn load_font_data(&mut self, data: Vec<u8>) {
        self.inner.db_mut().load_font_data(data);
    }

    /// Load font data from a shared Arc.
    ///
    /// This is more efficient when the same font data is used in multiple places.
    pub fn load_font_source(&mut self, source: fontdb::Source) {
        self.inner.db_mut().load_font_source(source);
    }

    /// Load all fonts from a directory.
    ///
    /// This recursively scans the directory for TTF, OTF, TTC, and OTC files.
    pub fn load_fonts_dir(&mut self, path: impl AsRef<Path>) {
        self.inner.db_mut().load_fonts_dir(path);
    }

    /// Query for a font matching the given criteria.
    ///
    /// Returns the ID of the best matching font face, or `None` if no match was found.
    pub fn query(&self, query: &FontQuery) -> Option<FontFaceId> {
        if query.families.is_empty() {
            return None;
        }

        // Build fontdb families slice
        let families: Vec<fontdb::Family<'_>> =
            query.families.iter().map(|f| f.to_fontdb()).collect();

        let fontdb_query = fontdb::Query {
            families: &families,
            weight: query.weight.to_fontdb(),
            stretch: query.stretch.to_fontdb(),
            style: query.style.to_fontdb(),
        };

        self.inner.db().query(&fontdb_query)
    }

    /// Get information about a font face.
    pub fn face_info(&self, face_id: FontFaceId) -> Option<FontFaceInfo> {
        self.inner.db().face(face_id).map(|face| FontFaceInfo {
            id: face.id,
            families: face.families.iter().map(|(name, _)| name.clone()).collect(),
            weight: FontWeight::from_fontdb(face.weight),
            style: FontStyle::from_fontdb(face.style),
            stretch: FontStretch::from_fontdb(face.stretch),
            monospaced: face.monospaced,
            post_script_name: face.post_script_name.clone(),
        })
    }

    /// Get font metrics for a specific font face.
    ///
    /// This requires parsing the font file, so the result is computed on each call.
    pub fn face_metrics(&self, face_id: FontFaceId) -> Option<FontMetrics> {
        self.inner
            .db()
            .with_face_data(face_id, |data, face_index| {
                ttf_parser::Face::parse(data, face_index)
                    .ok()
                    .map(|face| FontMetrics {
                        units_per_em: face.units_per_em(),
                        ascent: face.ascender(),
                        descent: face.descender(),
                        line_gap: face.line_gap(),
                        underline_position: face
                            .underline_metrics()
                            .map(|m| m.position)
                            .unwrap_or(0),
                        underline_thickness: face
                            .underline_metrics()
                            .map(|m| m.thickness)
                            .unwrap_or(0),
                        strikeout_position: face
                            .strikeout_metrics()
                            .map(|m| m.position)
                            .unwrap_or(0),
                        strikeout_thickness: face
                            .strikeout_metrics()
                            .map(|m| m.thickness)
                            .unwrap_or(0),
                        x_height: face.x_height(),
                        cap_height: face.capital_height(),
                    })
            })
            .flatten()
    }

    /// Iterate over all loaded font faces.
    pub fn faces(&self) -> impl Iterator<Item = FontFaceInfo> + '_ {
        self.inner.db().faces().map(|face| FontFaceInfo {
            id: face.id,
            families: face.families.iter().map(|(name, _)| name.clone()).collect(),
            weight: FontWeight::from_fontdb(face.weight),
            style: FontStyle::from_fontdb(face.style),
            stretch: FontStretch::from_fontdb(face.stretch),
            monospaced: face.monospaced,
            post_script_name: face.post_script_name.clone(),
        })
    }

    /// Get the number of loaded font faces.
    pub fn face_count(&self) -> usize {
        self.inner.db().faces().count()
    }

    /// Check if a font family exists in the database.
    pub fn has_family(&self, family: &str) -> bool {
        self.inner
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == family))
    }

    /// Get all unique font family names in the database.
    pub fn family_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .inner
            .db()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Set the default serif font family.
    pub fn set_serif_family(&mut self, family: impl AsRef<str>) {
        self.inner.db_mut().set_serif_family(family.as_ref());
    }

    /// Set the default sans-serif font family.
    pub fn set_sans_serif_family(&mut self, family: impl AsRef<str>) {
        self.inner.db_mut().set_sans_serif_family(family.as_ref());
    }

    /// Set the default monospace font family.
    pub fn set_monospace_family(&mut self, family: impl AsRef<str>) {
        self.inner.db_mut().set_monospace_family(family.as_ref());
    }

    /// Set the default cursive font family.
    pub fn set_cursive_family(&mut self, family: impl AsRef<str>) {
        self.inner.db_mut().set_cursive_family(family.as_ref());
    }

    /// Set the default fantasy font family.
    pub fn set_fantasy_family(&mut self, family: impl AsRef<str>) {
        self.inner.db_mut().set_fantasy_family(family.as_ref());
    }

    /// Access font data for a specific face.
    ///
    /// The callback receives the raw font data and the face index within the file.
    pub fn with_face_data<T>(
        &self,
        face_id: FontFaceId,
        f: impl FnOnce(&[u8], u32) -> T,
    ) -> Option<T> {
        self.inner.db().with_face_data(face_id, f)
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FontSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontSystem")
            .field("face_count", &self.face_count())
            .finish()
    }
}

/// Information about a loaded font face.
#[derive(Debug, Clone)]
pub struct FontFaceInfo {
    /// The unique identifier for this font face.
    pub id: FontFaceId,
    /// The family names this face belongs to.
    pub families: Vec<String>,
    /// The weight of this face.
    pub weight: FontWeight,
    /// The style of this face.
    pub style: FontStyle,
    /// The stretch of this face.
    pub stretch: FontStretch,
    /// Whether this is a monospaced font.
    pub monospaced: bool,
    /// The PostScript name of this face.
    pub post_script_name: String,
}

/// Error type for font loading operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FontLoadError {
    /// An I/O error occurred while loading the font.
    #[error("I/O error: {0}")]
    IoError(String),
    /// The font file format is invalid or unsupported.
    #[error("Invalid font format: {0}")]
    InvalidFormat(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_system_creation() {
        // Create without system fonts for faster testing
        let config = FontSystemConfig::new().load_system_fonts(false);
        let font_system = FontSystem::with_config(config);
        assert_eq!(font_system.face_count(), 0);
    }

    #[test]
    fn font_system_config_builder() {
        let config = FontSystemConfig::new()
            .load_system_fonts(false)
            .locale("fr-FR")
            .serif_family("Georgia")
            .sans_serif_family("Arial")
            .monospace_family("Consolas");

        assert!(!config.load_system_fonts);
        assert_eq!(config.locale, "fr-FR");
        assert_eq!(config.serif_family, Some("Georgia".to_string()));
        assert_eq!(config.sans_serif_family, Some("Arial".to_string()));
        assert_eq!(config.monospace_family, Some("Consolas".to_string()));
    }
}
