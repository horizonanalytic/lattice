//! Simple shapes example demonstrating the Renderer API.
//!
//! Run with: cargo run -p horizon-lattice-render --example simple_shapes

use std::sync::Arc;

use horizon_lattice_render::{
    Color, GpuRenderer, GraphicsConfig, GraphicsContext, Point, Rect, RenderSurface, Renderer,
    RoundedRect, Size, Stroke, SurfaceConfig,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Arc<Window>>,
    surface: Option<RenderSurface>,
    renderer: Option<GpuRenderer>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            renderer: None,
        }
    }

    fn render(&mut self) {
        let Some(surface) = &mut self.surface else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let (width, height) = surface.size();
        let viewport = Size::new(width as f32, height as f32);

        // Begin frame with a light background
        renderer.begin_frame(Color::from_rgb(0.95, 0.95, 0.98), viewport);

        // Draw a red rectangle
        renderer.fill_rect(Rect::new(50.0, 50.0, 150.0, 100.0), Color::RED);

        // Draw a blue rounded rectangle
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(250.0, 50.0, 150.0, 100.0), 20.0),
            Color::BLUE,
        );

        // Draw a green rectangle with transform
        renderer.save();
        renderer.translate(450.0, 50.0);
        renderer.rotate(0.1); // Small rotation
        renderer.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::GREEN);
        renderer.restore();

        // Draw stroked rectangles
        renderer.stroke_rect(
            Rect::new(50.0, 200.0, 150.0, 100.0),
            &Stroke::new(Color::BLACK, 2.0),
        );

        renderer.stroke_rounded_rect(
            RoundedRect::new(Rect::new(250.0, 200.0, 150.0, 100.0), 15.0),
            &Stroke::new(Color::DARK_GRAY, 3.0),
        );

        // Draw lines
        renderer.draw_line(
            Point::new(450.0, 200.0),
            Point::new(600.0, 300.0),
            &Stroke::new(Color::MAGENTA, 2.0),
        );

        // Draw a polyline
        let points = [
            Point::new(50.0, 350.0),
            Point::new(150.0, 400.0),
            Point::new(100.0, 450.0),
            Point::new(200.0, 380.0),
        ];
        renderer.draw_polyline(&points, &Stroke::new(Color::CYAN, 2.0));

        // Draw with clipping
        renderer.save();
        renderer.clip_rect(Rect::new(250.0, 350.0, 100.0, 100.0));
        renderer.fill_rect(Rect::new(200.0, 330.0, 200.0, 200.0), Color::YELLOW);
        renderer.restore();

        // Draw ellipse/circles
        renderer.fill_circle(Point::new(500.0, 400.0), 40.0, Color::from_rgb(1.0, 0.5, 0.0));

        // Draw with opacity
        renderer.set_opacity(0.5);
        renderer.fill_rect(Rect::new(550.0, 350.0, 100.0, 100.0), Color::BLUE);
        renderer.set_opacity(1.0);

        // End frame and render
        renderer.end_frame();
        if let Err(e) = renderer.render_to_surface(surface) {
            eprintln!("Render error: {e}");
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Initialize graphics context
        if GraphicsContext::try_get().is_none() {
            GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
        }

        // Create window
        let attrs = Window::default_attributes()
            .with_title("Simple Shapes - Horizon Lattice Render")
            .with_inner_size(winit::dpi::LogicalSize::new(700, 500));

        let window = Arc::new(event_loop.create_window(attrs).expect("Failed to create window"));

        // Create surface and renderer
        let surface =
            RenderSurface::new(Arc::clone(&window), SurfaceConfig::default())
                .expect("Failed to create surface");

        let renderer = GpuRenderer::new(&surface).expect("Failed to create renderer");

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(size.width, size.height).ok();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
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

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("Event loop error");
}
