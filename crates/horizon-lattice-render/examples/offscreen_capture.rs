//! Offscreen rendering and screenshot capture example.
//!
//! This example demonstrates headless rendering without a window,
//! useful for testing, server-side rendering, and screenshot capture.
//!
//! Run with: cargo run -p horizon-lattice-render --example offscreen_capture

use horizon_lattice_render::{
    Color, GpuRenderer, GraphicsConfig, GraphicsContext, OffscreenConfig, OffscreenSurface,
    Rect, Renderer, RoundedRect, Size, Stroke, Point,
};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Offscreen capture example");
    println!("=========================");
    println!();

    // Initialize graphics context (no window needed!)
    GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
    println!("Graphics context initialized (headless mode)");

    // Create an offscreen surface (renders to a texture, not a window)
    let config = OffscreenConfig::new(800, 600);
    let surface = OffscreenSurface::new(config).expect("Failed to create offscreen surface");
    println!("Created offscreen surface: {}x{}", surface.width(), surface.height());

    // Create a renderer for the offscreen surface
    let mut renderer = GpuRenderer::new_offscreen(&surface).expect("Failed to create renderer");
    println!("Created GPU renderer for offscreen surface");

    // Render some shapes (same API as windowed rendering!)
    let viewport = Size::new(surface.width() as f32, surface.height() as f32);
    renderer.begin_frame(Color::from_rgb(0.95, 0.95, 0.98), viewport);

    // Draw a red rectangle
    renderer.fill_rect(Rect::new(50.0, 50.0, 200.0, 150.0), Color::RED);

    // Draw a blue rounded rectangle
    renderer.fill_rounded_rect(
        RoundedRect::new(Rect::new(300.0, 50.0, 200.0, 150.0), 20.0),
        Color::BLUE,
    );

    // Draw a green rectangle with transform
    renderer.save();
    renderer.translate(550.0, 50.0);
    renderer.rotate(0.2);
    renderer.fill_rect(Rect::new(0.0, 0.0, 150.0, 100.0), Color::GREEN);
    renderer.restore();

    // Draw stroked shapes
    renderer.stroke_rect(
        Rect::new(50.0, 250.0, 200.0, 150.0),
        &Stroke::new(Color::BLACK, 3.0),
    );

    renderer.stroke_rounded_rect(
        RoundedRect::new(Rect::new(300.0, 250.0, 200.0, 150.0), 15.0),
        &Stroke::new(Color::DARK_GRAY, 2.0),
    );

    // Draw lines
    renderer.draw_line(
        Point::new(50.0, 450.0),
        Point::new(750.0, 550.0),
        &Stroke::new(Color::MAGENTA, 2.0),
    );

    // Draw circles with transparency
    renderer.fill_circle(Point::new(600.0, 350.0), 60.0, Color::CYAN);
    renderer.set_opacity(0.5);
    renderer.fill_circle(Point::new(650.0, 380.0), 50.0, Color::YELLOW);
    renderer.set_opacity(1.0);

    // Draw text label rectangle
    renderer.fill_rect(Rect::new(50.0, 500.0, 300.0, 40.0), Color::from_rgb(0.2, 0.2, 0.2));

    renderer.end_frame();
    println!("Frame rendered (batched)");

    // Render to the offscreen surface
    renderer.render_to_offscreen(&surface).expect("Failed to render");
    println!("Render submitted to GPU");

    // Save to file
    let output_path = "offscreen_capture.png";
    surface.save_to_file(output_path).expect("Failed to save image");
    println!();
    println!("Screenshot saved to: {}", output_path);
    println!();

    // Also demonstrate reading raw pixels
    let pixels = surface.read_pixels().expect("Failed to read pixels");
    println!("Read {} bytes of pixel data ({}x{} RGBA)",
             pixels.len(),
             surface.width(),
             surface.height());

    // Verify pixel data
    let expected_size = (surface.width() * surface.height() * 4) as usize;
    assert_eq!(pixels.len(), expected_size, "Pixel data size mismatch");
    println!("Pixel data verified");

    println!();
    println!("Done! Open {} to see the rendered image.", output_path);
}
