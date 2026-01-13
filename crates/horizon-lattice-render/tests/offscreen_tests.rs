//! Integration tests for offscreen rendering.
//!
//! These tests require a GPU. Run with:
//! ```
//! cargo test --package horizon-lattice-render -- --ignored
//! ```

use horizon_lattice_render::{
    capture::BufferDimensions, Color, GpuRenderer, GraphicsConfig, GraphicsContext,
    OffscreenConfig, OffscreenSurface, Rect, Renderer, Size,
};

#[test]
fn test_offscreen_config_creation() {
    let config = OffscreenConfig::new(1920, 1080);
    assert_eq!(config.width, 1920);
    assert_eq!(config.height, 1080);
    assert!(config.format.is_none());
}

#[test]
fn test_offscreen_config_default() {
    let config = OffscreenConfig::default();
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
}

#[test]
fn test_offscreen_config_with_format() {
    let config = OffscreenConfig::new(100, 100).with_format(wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(config.format, Some(wgpu::TextureFormat::Rgba8Unorm));
}

#[test]
fn test_buffer_dimensions_alignment() {
    // Test various widths and verify padding
    let dims = BufferDimensions::new(100, 100);
    assert_eq!(dims.unpadded_bytes_per_row, 400);
    // Should be aligned to 256 bytes
    assert_eq!(dims.padded_bytes_per_row % 256, 0);
    assert!(dims.padded_bytes_per_row >= dims.unpadded_bytes_per_row);

    // Test a width that's already aligned (64 * 4 = 256)
    let dims_aligned = BufferDimensions::new(64, 100);
    assert_eq!(dims_aligned.unpadded_bytes_per_row, 256);
    assert_eq!(dims_aligned.padded_bytes_per_row, 256);
}

#[test]
fn test_buffer_dimensions_size() {
    let dims = BufferDimensions::new(100, 200);
    assert_eq!(dims.buffer_size(), dims.padded_bytes_per_row as u64 * 200);
}

#[test]
#[ignore = "requires GPU"]
fn test_offscreen_surface_creation() {
    // Initialize graphics context
    if GraphicsContext::try_get().is_none() {
        GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    }

    // Create offscreen surface
    let config = OffscreenConfig::new(256, 256);
    let surface = OffscreenSurface::new(config).expect("Failed to create offscreen surface");

    assert_eq!(surface.width(), 256);
    assert_eq!(surface.height(), 256);
    assert_eq!(surface.size(), (256, 256));
    assert!(surface.format().is_srgb());

    println!("Offscreen surface created: {:?}", surface);
}

#[test]
#[ignore = "requires GPU"]
fn test_offscreen_resize() {
    if GraphicsContext::try_get().is_none() {
        GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    }

    let mut surface =
        OffscreenSurface::new(OffscreenConfig::new(100, 100)).expect("Failed to create surface");

    assert_eq!(surface.size(), (100, 100));

    surface.resize(200, 150).expect("Resize should succeed");
    assert_eq!(surface.size(), (200, 150));

    // Test that same-size resize is a no-op
    surface.resize(200, 150).expect("Same-size resize should succeed");
    assert_eq!(surface.size(), (200, 150));

    // Test that zero-size resize fails
    let result = surface.resize(0, 100);
    assert!(result.is_err(), "Zero-size resize should fail");
}

#[test]
#[ignore = "requires GPU"]
fn test_offscreen_render_and_read() {
    if GraphicsContext::try_get().is_none() {
        GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    }

    let surface =
        OffscreenSurface::new(OffscreenConfig::new(64, 64)).expect("Failed to create surface");

    let mut renderer =
        GpuRenderer::new_offscreen(&surface).expect("Failed to create offscreen renderer");

    // Render a simple red rectangle
    let viewport = Size::new(64.0, 64.0);
    renderer.begin_frame(Color::WHITE, viewport);
    renderer.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), Color::RED);
    renderer.end_frame();
    renderer
        .render_to_offscreen(&surface)
        .expect("Failed to render");

    // Read back pixels
    let pixels = surface.read_pixels().expect("Failed to read pixels");

    // Verify size: 64 * 64 * 4 bytes (RGBA)
    assert_eq!(pixels.len(), 64 * 64 * 4);

    // The first pixel should be red-ish (exact values depend on sRGB conversion)
    // In premultiplied sRGB, red should have high R, low G, low B
    assert!(pixels[0] > 200, "Red channel should be high");
    assert!(pixels[1] < 50, "Green channel should be low");
    assert!(pixels[2] < 50, "Blue channel should be low");
    assert_eq!(pixels[3], 255, "Alpha should be fully opaque");

    println!("First pixel RGBA: {}, {}, {}, {}", pixels[0], pixels[1], pixels[2], pixels[3]);
    println!("Offscreen render and read test passed");
}

#[test]
#[ignore = "requires GPU"]
fn test_multiple_offscreen_surfaces() {
    if GraphicsContext::try_get().is_none() {
        GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    }

    // Create multiple offscreen surfaces (tests resource sharing)
    let surface1 =
        OffscreenSurface::new(OffscreenConfig::new(100, 100)).expect("Failed to create surface 1");
    let surface2 =
        OffscreenSurface::new(OffscreenConfig::new(200, 200)).expect("Failed to create surface 2");

    // Create renderers for both
    let mut renderer1 =
        GpuRenderer::new_offscreen(&surface1).expect("Failed to create renderer 1");
    let mut renderer2 =
        GpuRenderer::new_offscreen(&surface2).expect("Failed to create renderer 2");

    // Render different content to each
    renderer1.begin_frame(Color::RED, Size::new(100.0, 100.0));
    renderer1.end_frame();
    renderer1
        .render_to_offscreen(&surface1)
        .expect("Failed to render to surface 1");

    renderer2.begin_frame(Color::BLUE, Size::new(200.0, 200.0));
    renderer2.end_frame();
    renderer2
        .render_to_offscreen(&surface2)
        .expect("Failed to render to surface 2");

    // Read back and verify they're different
    let pixels1 = surface1.read_pixels().expect("Failed to read surface 1");
    let pixels2 = surface2.read_pixels().expect("Failed to read surface 2");

    assert_eq!(pixels1.len(), 100 * 100 * 4);
    assert_eq!(pixels2.len(), 200 * 200 * 4);

    // First pixel of surface1 should be red-ish
    assert!(pixels1[0] > 200, "Surface 1 should be red");
    // First pixel of surface2 should be blue-ish
    assert!(pixels2[2] > 200, "Surface 2 should be blue");

    println!("Multiple offscreen surfaces test passed");
}

#[test]
#[ignore = "requires GPU"]
fn test_gpu_memory_churn() {
    // This test creates and destroys resources repeatedly to check for memory leaks.
    // While we can't directly measure GPU memory, we can ensure no errors/panics occur
    // during resource churn and that rendering continues to work correctly.

    if GraphicsContext::try_get().is_none() {
        GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    }

    const ITERATIONS: usize = 50;
    const SURFACES_PER_ITERATION: usize = 5;

    for iter in 0..ITERATIONS {
        // Create multiple surfaces and renderers
        let mut surfaces = Vec::new();
        let mut renderers = Vec::new();

        for i in 0..SURFACES_PER_ITERATION {
            let size = 64 + (i * 32) as u32; // Varying sizes
            let surface = OffscreenSurface::new(OffscreenConfig::new(size, size))
                .expect("Failed to create surface");
            let renderer =
                GpuRenderer::new_offscreen(&surface).expect("Failed to create renderer");
            surfaces.push(surface);
            renderers.push(renderer);
        }

        // Render to each surface
        for (surface, renderer) in surfaces.iter().zip(renderers.iter_mut()) {
            let (w, h) = surface.size();
            let viewport = Size::new(w as f32, h as f32);
            renderer.begin_frame(Color::BLUE, viewport);

            // Draw some shapes to use GPU resources
            for j in 0..10 {
                let offset = j as f32 * 5.0;
                renderer.fill_rect(
                    Rect::new(offset, offset, 20.0, 20.0),
                    Color::from_rgba8(255, (j * 25) as u8, 0, 255),
                );
            }

            renderer.end_frame();
            renderer
                .render_to_offscreen(surface)
                .expect("Failed to render");
        }

        // Read pixels to ensure rendering completed
        for surface in &surfaces {
            let _pixels = surface.read_pixels().expect("Failed to read pixels");
        }

        // Surfaces and renderers are dropped here, releasing GPU resources

        if iter % 10 == 0 {
            println!("GPU memory churn test: iteration {}/{}", iter + 1, ITERATIONS);
        }
    }

    // Final verification: create a new surface and render to it successfully
    let final_surface = OffscreenSurface::new(OffscreenConfig::new(128, 128))
        .expect("Final surface creation should work after churn");
    let mut final_renderer =
        GpuRenderer::new_offscreen(&final_surface).expect("Final renderer should work");

    final_renderer.begin_frame(Color::GREEN, Size::new(128.0, 128.0));
    final_renderer.fill_rect(Rect::new(10.0, 10.0, 100.0, 100.0), Color::RED);
    final_renderer.end_frame();
    final_renderer
        .render_to_offscreen(&final_surface)
        .expect("Final render should succeed");

    let final_pixels = final_surface.read_pixels().expect("Final read should succeed");
    assert_eq!(final_pixels.len(), 128 * 128 * 4);

    println!(
        "GPU memory churn test completed: {} iterations, {} surfaces each",
        ITERATIONS, SURFACES_PER_ITERATION
    );
}
