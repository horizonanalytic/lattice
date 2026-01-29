//! Multi-window example demonstrating per-window render state.
//!
//! This example creates two windows, each with its own RenderSurface and GpuRenderer,
//! demonstrating that the graphics architecture supports independent per-window rendering.
//!
//! Run with: cargo run -p horizon-lattice-render --example multi_window

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_render::{
    Color, GpuRenderer, GraphicsConfig, GraphicsContext, Point, Rect, RenderSurface, Renderer,
    RoundedRect, Size, Stroke, SurfaceConfig,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Per-window rendering state.
struct WindowState {
    window: Arc<Window>,
    surface: RenderSurface,
    renderer: GpuRenderer,
    /// Unique color for this window to demonstrate independence.
    accent_color: Color,
    /// Animation frame counter.
    frame: u32,
}

impl WindowState {
    fn new(event_loop: &ActiveEventLoop, title: &str, x: i32, y: i32, accent_color: Color) -> Self {
        let attrs = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(400, 300))
            .with_position(winit::dpi::LogicalPosition::new(x, y));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        let surface = RenderSurface::new(Arc::clone(&window), SurfaceConfig::default())
            .expect("Failed to create surface");

        let renderer = GpuRenderer::new(&surface).expect("Failed to create renderer");

        Self {
            window,
            surface,
            renderer,
            accent_color,
            frame: 0,
        }
    }

    fn render(&mut self) {
        let (width, height) = self.surface.size();
        let viewport = Size::new(width as f32, height as f32);

        // Animate position
        let offset = (self.frame as f32 * 0.02).sin() * 20.0;
        self.frame = self.frame.wrapping_add(1);

        // Begin frame with a light background
        self.renderer
            .begin_frame(Color::from_rgb(0.95, 0.95, 0.98), viewport);

        // Draw a rectangle with this window's accent color
        self.renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(50.0 + offset, 50.0, 150.0, 100.0), 15.0),
            self.accent_color,
        );

        // Draw some common shapes to show both windows work
        self.renderer.stroke_rect(
            Rect::new(220.0, 50.0, 100.0, 80.0),
            &Stroke::new(Color::DARK_GRAY, 2.0),
        );

        // Draw lines with the accent color
        self.renderer.draw_line(
            Point::new(50.0, 180.0),
            Point::new(350.0, 180.0 + offset),
            &Stroke::new(self.accent_color, 3.0),
        );

        // Draw a circle
        self.renderer.fill_circle(
            Point::new(300.0, 120.0),
            30.0,
            self.accent_color.with_alpha(0.5),
        );

        // End frame and render
        self.renderer.end_frame();
        if let Err(e) = self.renderer.render_to_surface(&mut self.surface) {
            eprintln!("Render error: {e}");
        }

        // Request next frame
        self.window.request_redraw();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.surface.resize(width, height).ok();
    }
}

struct MultiWindowApp {
    windows: HashMap<WindowId, WindowState>,
    initialized: bool,
}

impl MultiWindowApp {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            initialized: false,
        }
    }
}

impl ApplicationHandler for MultiWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }

        // Initialize graphics context (once, shared by all windows)
        if GraphicsContext::try_get().is_none() {
            GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
        }

        // Create first window (red accent)
        let window1 = WindowState::new(
            event_loop,
            "Window 1 - Red (Horizon Lattice)",
            100,
            100,
            Color::from_rgb(0.9, 0.2, 0.2),
        );
        let id1 = window1.window.id();
        self.windows.insert(id1, window1);

        // Create second window (blue accent)
        let window2 = WindowState::new(
            event_loop,
            "Window 2 - Blue (Horizon Lattice)",
            550,
            100,
            Color::from_rgb(0.2, 0.4, 0.9),
        );
        let id2 = window2.window.id();
        self.windows.insert(id2, window2);

        self.initialized = true;

        println!(
            "Created {} windows with independent render state",
            self.windows.len()
        );
        println!("Each window has its own RenderSurface and GpuRenderer");
        println!("The GraphicsContext (GPU device/queue) is shared between all windows");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window_state) = self.windows.get_mut(&window_id) else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                window_state.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                window_state.render();
            }
            _ => {}
        }
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Multi-window example");
    println!("====================");
    println!("This demonstrates per-window render state with a shared graphics context.");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = MultiWindowApp::new();

    event_loop.run_app(&mut app).expect("Event loop error");
}
