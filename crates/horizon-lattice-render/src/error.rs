//! Error types for the render crate.

use thiserror::Error;

/// Errors that can occur during graphics operations.
#[derive(Error, Debug)]
pub enum RenderError {
    /// No suitable graphics adapter was found.
    #[error("no suitable graphics adapter found")]
    NoAdapter,

    /// Failed to request a graphics device.
    #[error("failed to request graphics device: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),

    /// Failed to create a surface.
    #[error("failed to create surface: {0}")]
    SurfaceCreation(#[from] wgpu::CreateSurfaceError),

    /// Surface error during rendering.
    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    /// The surface has not been configured yet.
    #[error("surface not configured")]
    SurfaceNotConfigured,

    /// Invalid surface dimensions (zero width or height).
    #[error("invalid surface dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    /// The graphics context has not been initialized.
    #[error("graphics context not initialized")]
    NotInitialized,
}

/// Result type for render operations.
pub type RenderResult<T> = Result<T, RenderError>;
