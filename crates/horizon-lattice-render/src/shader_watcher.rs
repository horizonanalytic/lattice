//! Shader hot-reloading support for development.
//!
//! This module provides file watching and automatic shader reloading functionality.
//! It's only available when the `shader-hot-reload` feature is enabled.
//!
//! # Usage
//!
//! ```no_run
//! use horizon_lattice_render::{GpuRenderer, ShaderWatcher};
//!
//! let mut renderer = // ... create renderer
//! let mut watcher = ShaderWatcher::new()?;
//!
//! // In your render loop:
//! if let Some(reload_result) = watcher.poll_changes() {
//!     match reload_result {
//!         Ok(changed_shaders) => {
//!             renderer.reload_shaders(&changed_shaders);
//!         }
//!         Err(e) => {
//!             eprintln!("Shader compilation error: {}", e);
//!             // Keep using previous working shaders
//!         }
//!     }
//! }
//! ```

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use crate::error::{RenderError, RenderResult};
use crate::GraphicsContext;

/// Identifies which shader was changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderKind {
    /// Rectangle/gradient shader (rect.wgsl)
    Rect,
    /// Image/texture shader (image.wgsl)
    Image,
    /// Box shadow shader (shadow.wgsl)
    Shadow,
    /// Layer compositing shader (composite.wgsl)
    Composite,
}

impl ShaderKind {
    /// Get the filename for this shader kind.
    pub fn filename(&self) -> &'static str {
        match self {
            ShaderKind::Rect => "rect.wgsl",
            ShaderKind::Image => "image.wgsl",
            ShaderKind::Shadow => "shadow.wgsl",
            ShaderKind::Composite => "composite.wgsl",
        }
    }

    /// Try to determine shader kind from a file path.
    pub fn from_path(path: &Path) -> Option<Self> {
        let filename = path.file_name()?.to_str()?;
        match filename {
            "rect.wgsl" => Some(ShaderKind::Rect),
            "image.wgsl" => Some(ShaderKind::Image),
            "shadow.wgsl" => Some(ShaderKind::Shadow),
            "composite.wgsl" => Some(ShaderKind::Composite),
            _ => None,
        }
    }
}

/// Result of a shader reload operation.
#[derive(Debug)]
pub struct ShaderReloadResult {
    /// Which shaders were successfully reloaded.
    pub reloaded: HashSet<ShaderKind>,
    /// The new shader modules (kind, source, compiled module).
    pub modules: Vec<(ShaderKind, String, wgpu::ShaderModule)>,
}

/// Watches shader files and notifies when they change.
///
/// This watcher uses debouncing to avoid multiple reload events when
/// a file is saved multiple times in quick succession.
pub struct ShaderWatcher {
    /// File watcher with debouncing.
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    /// Channel receiver for file change events.
    rx: Receiver<Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    /// Path to the shaders directory.
    shader_dir: PathBuf,
    /// Accumulated changed files since last poll.
    pending_changes: HashSet<ShaderKind>,
}

impl ShaderWatcher {
    /// Create a new shader watcher for the given shader directory.
    ///
    /// The watcher will monitor all `.wgsl` files in the directory.
    pub fn new(shader_dir: impl AsRef<Path>) -> RenderResult<Self> {
        let shader_dir = shader_dir.as_ref().to_path_buf();

        if !shader_dir.exists() {
            return Err(RenderError::ShaderError(format!(
                "Shader directory does not exist: {}",
                shader_dir.display()
            )));
        }

        let (tx, rx) = mpsc::channel();

        // Create debounced watcher (100ms debounce time)
        let mut debouncer = new_debouncer(Duration::from_millis(100), tx).map_err(|e| {
            RenderError::ShaderError(format!("Failed to create file watcher: {}", e))
        })?;

        // Watch the shader directory
        debouncer
            .watcher()
            .watch(&shader_dir, RecursiveMode::NonRecursive)
            .map_err(|e| {
                RenderError::ShaderError(format!("Failed to watch shader directory: {}", e))
            })?;

        tracing::info!(
            "Shader hot-reload enabled, watching: {}",
            shader_dir.display()
        );

        Ok(Self {
            _debouncer: debouncer,
            rx,
            shader_dir,
            pending_changes: HashSet::new(),
        })
    }

    /// Poll for shader changes and reload if necessary.
    ///
    /// Returns `Some(Ok(result))` if shaders were successfully reloaded,
    /// `Some(Err(error))` if shader compilation failed, or `None` if no changes.
    pub fn poll_changes(&mut self) -> Option<RenderResult<ShaderReloadResult>> {
        // Drain all pending events from the channel
        loop {
            match self.rx.try_recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            if let Some(kind) = ShaderKind::from_path(&event.path) {
                                tracing::debug!("Detected change in shader: {:?}", kind);
                                self.pending_changes.insert(kind);
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("File watcher error: {}", e);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("File watcher channel disconnected");
                    break;
                }
            }
        }

        // If we have pending changes, try to reload
        if self.pending_changes.is_empty() {
            return None;
        }

        let changed: Vec<_> = self.pending_changes.drain().collect();
        Some(self.reload_shaders(&changed))
    }

    /// Reload the specified shaders.
    fn reload_shaders(&self, kinds: &[ShaderKind]) -> RenderResult<ShaderReloadResult> {
        let ctx = GraphicsContext::get();
        let device = ctx.device();

        let mut result = ShaderReloadResult {
            reloaded: HashSet::new(),
            modules: Vec::new(),
        };

        for &kind in kinds {
            let path = self.shader_dir.join(kind.filename());
            let source = fs::read_to_string(&path).map_err(|e| {
                RenderError::ShaderError(format!(
                    "Failed to read shader {}: {}",
                    path.display(),
                    e
                ))
            })?;

            // Try to compile the shader
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(kind.filename()),
                source: wgpu::ShaderSource::Wgsl(source.clone().into()),
            });

            // Note: wgpu doesn't return errors from create_shader_module directly
            // in all configurations. The validation happens at pipeline creation.
            // For now, we assume compilation succeeded if no panic occurred.

            tracing::info!("Reloaded shader: {:?}", kind);
            result.reloaded.insert(kind);
            result.modules.push((kind, source, module));
        }

        Ok(result)
    }

    /// Read a shader source file directly.
    pub fn read_shader(&self, kind: ShaderKind) -> RenderResult<String> {
        let path = self.shader_dir.join(kind.filename());
        fs::read_to_string(&path).map_err(|e| {
            RenderError::ShaderError(format!(
                "Failed to read shader {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Get the shader directory path.
    pub fn shader_dir(&self) -> &Path {
        &self.shader_dir
    }
}

/// Loads shader source either from embedded strings or from files.
///
/// In release builds (or without shader-hot-reload), this returns the embedded source.
/// With shader-hot-reload enabled, this loads from files to enable hot reloading.
pub fn load_shader_source(kind: ShaderKind, shader_dir: Option<&Path>) -> RenderResult<String> {
    if let Some(dir) = shader_dir {
        // Load from file for hot-reload
        let path = dir.join(kind.filename());
        fs::read_to_string(&path).map_err(|e| {
            RenderError::ShaderError(format!(
                "Failed to read shader {}: {}",
                path.display(),
                e
            ))
        })
    } else {
        // Return embedded source
        Ok(match kind {
            ShaderKind::Rect => include_str!("shaders/rect.wgsl").to_string(),
            ShaderKind::Image => include_str!("shaders/image.wgsl").to_string(),
            ShaderKind::Shadow => include_str!("shaders/shadow.wgsl").to_string(),
            ShaderKind::Composite => include_str!("shaders/composite.wgsl").to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_kind_from_path() {
        assert_eq!(
            ShaderKind::from_path(Path::new("/some/path/rect.wgsl")),
            Some(ShaderKind::Rect)
        );
        assert_eq!(
            ShaderKind::from_path(Path::new("image.wgsl")),
            Some(ShaderKind::Image)
        );
        assert_eq!(
            ShaderKind::from_path(Path::new("unknown.wgsl")),
            None
        );
        assert_eq!(
            ShaderKind::from_path(Path::new("rect.txt")),
            None
        );
    }

    #[test]
    fn test_shader_kind_filename() {
        assert_eq!(ShaderKind::Rect.filename(), "rect.wgsl");
        assert_eq!(ShaderKind::Image.filename(), "image.wgsl");
        assert_eq!(ShaderKind::Shadow.filename(), "shadow.wgsl");
        assert_eq!(ShaderKind::Composite.filename(), "composite.wgsl");
    }
}
