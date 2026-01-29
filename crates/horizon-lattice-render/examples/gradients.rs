//! Gradient rendering example demonstrating linear and radial gradients.
//!
//! Run with: cargo run -p horizon-lattice-render --example gradients

use std::sync::Arc;

use horizon_lattice_render::{
    Color, GpuRenderer, GradientStop, GraphicsConfig, GraphicsContext, Paint, Point, Rect,
    RenderSurface, Renderer, RoundedRect, Size, SurfaceConfig,
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

        // Begin frame with a dark background to show gradients well
        renderer.begin_frame(Color::from_rgb(0.15, 0.15, 0.18), viewport);

        // === Row 1: Linear Gradients ===

        // Horizontal linear gradient (left to right)
        let rect1 = Rect::new(50.0, 50.0, 200.0, 100.0);
        let gradient1 = Paint::linear_gradient(
            Point::new(rect1.left(), rect1.top()),
            Point::new(rect1.right(), rect1.top()),
            vec![
                GradientStop::new(0.0, Color::RED),
                GradientStop::new(1.0, Color::BLUE),
            ],
        );
        renderer.fill_rect(rect1, gradient1);

        // Vertical linear gradient (top to bottom)
        let rect2 = Rect::new(280.0, 50.0, 200.0, 100.0);
        let gradient2 = Paint::linear_gradient(
            Point::new(rect2.left(), rect2.top()),
            Point::new(rect2.left(), rect2.bottom()),
            vec![
                GradientStop::new(0.0, Color::GREEN),
                GradientStop::new(1.0, Color::YELLOW),
            ],
        );
        renderer.fill_rect(rect2, gradient2);

        // Diagonal linear gradient
        let rect3 = Rect::new(510.0, 50.0, 200.0, 100.0);
        let gradient3 = Paint::linear_gradient(
            Point::new(rect3.left(), rect3.top()),
            Point::new(rect3.right(), rect3.bottom()),
            vec![
                GradientStop::new(0.0, Color::CYAN),
                GradientStop::new(1.0, Color::MAGENTA),
            ],
        );
        renderer.fill_rect(rect3, gradient3);

        // === Row 2: Radial Gradients ===

        // Centered radial gradient
        let rect4 = Rect::new(50.0, 180.0, 200.0, 150.0);
        let gradient4 = Paint::radial_gradient(
            Point::new(
                rect4.left() + rect4.width() / 2.0,
                rect4.top() + rect4.height() / 2.0,
            ),
            75.0,
            None,
            vec![
                GradientStop::new(0.0, Color::WHITE),
                GradientStop::new(1.0, Color::from_rgb(0.2, 0.2, 0.8)),
            ],
        );
        renderer.fill_rect(rect4, gradient4);

        // Off-center radial gradient
        let rect5 = Rect::new(280.0, 180.0, 200.0, 150.0);
        let gradient5 = Paint::radial_gradient(
            Point::new(rect5.left() + 50.0, rect5.top() + 40.0), // Top-left offset
            120.0,
            None,
            vec![
                GradientStop::new(0.0, Color::YELLOW),
                GradientStop::new(1.0, Color::from_rgb(0.8, 0.2, 0.1)),
            ],
        );
        renderer.fill_rect(rect5, gradient5);

        // Small radius radial gradient (spotlight effect)
        let rect6 = Rect::new(510.0, 180.0, 200.0, 150.0);
        let gradient6 = Paint::radial_gradient(
            Point::new(
                rect6.left() + rect6.width() / 2.0,
                rect6.top() + rect6.height() / 2.0,
            ),
            50.0,
            None,
            vec![
                GradientStop::new(0.0, Color::WHITE),
                GradientStop::new(1.0, Color::from_rgb(0.1, 0.1, 0.1)),
            ],
        );
        renderer.fill_rect(rect6, gradient6);

        // === Row 3: Rounded Rectangles with Gradients ===

        // Rounded rect with horizontal gradient
        let rect7 = Rect::new(50.0, 360.0, 200.0, 100.0);
        let gradient7 = Paint::linear_gradient(
            Point::new(rect7.left(), rect7.top()),
            Point::new(rect7.right(), rect7.top()),
            vec![
                GradientStop::new(0.0, Color::from_rgb(0.9, 0.3, 0.5)),
                GradientStop::new(1.0, Color::from_rgb(0.3, 0.5, 0.9)),
            ],
        );
        renderer.fill_rounded_rect(RoundedRect::new(rect7, 20.0), gradient7);

        // Rounded rect with radial gradient
        let rect8 = Rect::new(280.0, 360.0, 150.0, 100.0);
        let gradient8 = Paint::radial_gradient(
            Point::new(
                rect8.left() + rect8.width() / 2.0,
                rect8.top() + rect8.height() / 2.0,
            ),
            80.0,
            None,
            vec![
                GradientStop::new(0.0, Color::from_rgb(1.0, 0.9, 0.3)),
                GradientStop::new(1.0, Color::from_rgb(0.1, 0.6, 0.3)),
            ],
        );
        renderer.fill_rounded_rect(RoundedRect::new(rect8, 30.0), gradient8);

        // Circle with radial gradient (button-like)
        let rect9 = Rect::new(480.0, 360.0, 100.0, 100.0);
        let gradient9 = Paint::radial_gradient(
            Point::new(rect9.left() + 30.0, rect9.top() + 25.0), // Offset for 3D effect
            70.0,
            None,
            vec![
                GradientStop::new(0.0, Color::from_rgb(0.6, 0.8, 1.0)),
                GradientStop::new(1.0, Color::from_rgb(0.1, 0.3, 0.7)),
            ],
        );
        renderer.fill_rounded_rect(RoundedRect::new(rect9, 50.0), gradient9);

        // === Row 4: Gradients with transforms ===

        // Scaled gradient
        renderer.save();
        renderer.translate(620.0, 360.0);
        renderer.scale(0.8, 0.8);
        let rect10 = Rect::new(0.0, 0.0, 150.0, 120.0);
        let gradient10 = Paint::linear_gradient(
            Point::new(0.0, 0.0),
            Point::new(150.0, 120.0),
            vec![
                GradientStop::new(0.0, Color::from_rgb(0.9, 0.5, 0.1)),
                GradientStop::new(1.0, Color::from_rgb(0.2, 0.1, 0.6)),
            ],
        );
        renderer.fill_rounded_rect(RoundedRect::new(rect10, 15.0), gradient10);
        renderer.restore();

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
            .with_title("Gradients - Horizon Lattice Render")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 500));

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("Failed to create window"),
        );

        // Create surface and renderer
        let surface = RenderSurface::new(Arc::clone(&window), SurfaceConfig::default())
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
