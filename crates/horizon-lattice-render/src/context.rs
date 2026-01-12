//! Graphics context managing shared GPU resources.
//!
//! The [`GraphicsContext`] is the central point for GPU resource management.
//! It owns the wgpu instance, adapter, device, and queue, which are shared
//! across all rendering surfaces.

use std::sync::{Arc, OnceLock};

use tracing::{debug, info};

use crate::error::{RenderError, RenderResult};

/// Global graphics context instance.
static GRAPHICS_CONTEXT: OnceLock<GraphicsContext> = OnceLock::new();

/// Configuration options for graphics context initialization.
#[derive(Debug, Clone)]
pub struct GraphicsConfig {
    /// Preferred GPU backends to use.
    pub backends: wgpu::Backends,
    /// Power preference for adapter selection.
    pub power_preference: wgpu::PowerPreference,
    /// Required device features.
    pub required_features: wgpu::Features,
    /// Required device limits.
    pub required_limits: wgpu::Limits,
    /// Enable debug validation layers.
    pub debug_validation: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            backends: wgpu::Backends::PRIMARY,
            power_preference: wgpu::PowerPreference::HighPerformance,
            required_features: wgpu::Features::empty(),
            required_limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                wgpu::Limits::default()
            },
            debug_validation: cfg!(debug_assertions),
        }
    }
}

/// Shared GPU resources used across all rendering surfaces.
///
/// This struct holds the wgpu instance, adapter, device, and queue.
/// These resources are created once and shared across all windows/surfaces.
#[derive(Debug)]
pub struct GpuResources {
    /// The wgpu instance for creating surfaces and requesting adapters.
    pub instance: wgpu::Instance,
    /// The graphics adapter (represents a physical GPU).
    pub adapter: wgpu::Adapter,
    /// The logical device for creating GPU resources.
    pub device: wgpu::Device,
    /// The command queue for submitting GPU work.
    pub queue: wgpu::Queue,
}

impl GpuResources {
    /// Create new GPU resources with the given configuration.
    fn new(config: &GraphicsConfig) -> RenderResult<Self> {
        let instance_flags = if config.debug_validation {
            wgpu::InstanceFlags::debugging()
        } else {
            wgpu::InstanceFlags::empty()
        };

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: config.backends,
            flags: instance_flags,
            ..Default::default()
        });

        info!(
            target: "horizon_lattice_render::context",
            backends = ?config.backends,
            "created wgpu instance"
        );

        // Request adapter (blocking for now, async could be added later)
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: config.power_preference,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or(RenderError::NoAdapter)?;

        let adapter_info = adapter.get_info();
        info!(
            target: "horizon_lattice_render::context",
            name = adapter_info.name,
            backend = ?adapter_info.backend,
            device_type = ?adapter_info.device_type,
            "selected graphics adapter"
        );

        // Request device and queue
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("horizon-lattice-device"),
                required_features: config.required_features,
                required_limits: config.required_limits.clone(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))?;

        debug!(
            target: "horizon_lattice_render::context",
            "created graphics device and queue"
        );

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}

/// The main graphics context for the application.
///
/// This is a singleton that manages all GPU resources. It must be initialized
/// before any rendering operations can occur.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::{GraphicsContext, GraphicsConfig};
///
/// // Initialize with default config
/// let ctx = GraphicsContext::init(GraphicsConfig::default()).unwrap();
///
/// // Or get the existing instance later
/// let ctx = GraphicsContext::get();
/// ```
pub struct GraphicsContext {
    /// The shared GPU resources.
    resources: Arc<GpuResources>,
    /// Configuration used to create this context.
    config: GraphicsConfig,
}

impl GraphicsContext {
    /// Initialize the global graphics context.
    ///
    /// This must be called once before any rendering operations.
    /// Subsequent calls will return an error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The context has already been initialized
    /// - No suitable graphics adapter was found
    /// - Device creation failed
    pub fn init(config: GraphicsConfig) -> RenderResult<&'static GraphicsContext> {
        let resources = GpuResources::new(&config)?;

        let context = GraphicsContext {
            resources: Arc::new(resources),
            config,
        };

        GRAPHICS_CONTEXT
            .set(context)
            .map_err(|_| RenderError::NotInitialized)?;

        Ok(GRAPHICS_CONTEXT.get().unwrap())
    }

    /// Get the global graphics context.
    ///
    /// Returns `None` if [`init`](Self::init) has not been called.
    pub fn try_get() -> Option<&'static GraphicsContext> {
        GRAPHICS_CONTEXT.get()
    }

    /// Get the global graphics context.
    ///
    /// # Panics
    ///
    /// Panics if [`init`](Self::init) has not been called.
    pub fn get() -> &'static GraphicsContext {
        GRAPHICS_CONTEXT
            .get()
            .expect("GraphicsContext not initialized. Call GraphicsContext::init() first.")
    }

    /// Get the wgpu instance.
    pub fn instance(&self) -> &wgpu::Instance {
        &self.resources.instance
    }

    /// Get the graphics adapter.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.resources.adapter
    }

    /// Get the logical device.
    pub fn device(&self) -> &wgpu::Device {
        &self.resources.device
    }

    /// Get the command queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.resources.queue
    }

    /// Get shared access to all GPU resources.
    pub fn resources(&self) -> Arc<GpuResources> {
        Arc::clone(&self.resources)
    }

    /// Get the configuration used to create this context.
    pub fn config(&self) -> &GraphicsConfig {
        &self.config
    }

    /// Get information about the graphics adapter.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.resources.adapter.get_info()
    }

    /// Poll the device for completed work.
    ///
    /// This should be called periodically to process GPU callbacks.
    pub fn poll(&self, maintain: wgpu::Maintain) -> wgpu::MaintainResult {
        self.resources.device.poll(maintain)
    }
}

impl std::fmt::Debug for GraphicsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let info = self.adapter_info();
        f.debug_struct("GraphicsContext")
            .field("adapter", &info.name)
            .field("backend", &info.backend)
            .field("device_type", &info.device_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a GPU and can't run in CI without special setup.
    // They're marked as ignored by default.

    #[test]
    #[ignore = "requires GPU"]
    fn test_graphics_config_default() {
        let config = GraphicsConfig::default();
        assert_eq!(config.backends, wgpu::Backends::PRIMARY);
        assert_eq!(config.power_preference, wgpu::PowerPreference::HighPerformance);
    }
}
