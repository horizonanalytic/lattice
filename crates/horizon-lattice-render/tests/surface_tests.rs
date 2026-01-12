//! Integration tests for surface management.
//!
//! These tests require a GPU and window system. Run with:
//! ```
//! cargo test --package horizon-lattice-render -- --ignored
//! ```

use std::sync::Arc;

use horizon_lattice_render::{
    GraphicsConfig, GraphicsContext, PresentMode, RenderSurface, SurfaceConfig,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Test application handler that creates a surface and runs basic tests.
struct TestApp {
    surface: Option<RenderSurface>,
    tests_run: bool,
}

impl TestApp {
    fn new() -> Self {
        Self {
            surface: None,
            tests_run: false,
        }
    }
}

impl ApplicationHandler for TestApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize graphics context
        if GraphicsContext::try_get().is_none() {
            GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
        }

        // Create window
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Surface Test")
                        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0)),
                )
                .expect("Failed to create window"),
        );

        // Create surface
        let surface =
            RenderSurface::new(window, SurfaceConfig::default()).expect("Failed to create surface");

        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let surface = match &mut self.surface {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                surface.resize(size.width, size.height).ok();
            }
            WindowEvent::RedrawRequested => {
                if !self.tests_run {
                    self.tests_run = true;

                    // Run tests
                    run_surface_tests(surface);

                    // Exit after tests
                    event_loop.exit();
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(surface) = &self.surface {
            surface.request_redraw();
        }
    }
}

fn run_surface_tests(surface: &mut RenderSurface) {
    let ctx = GraphicsContext::get();

    // Test 1: Verify surface is created with correct initial size
    let (width, height) = surface.size();
    assert!(width > 0, "Surface width should be > 0");
    assert!(height > 0, "Surface height should be > 0");
    println!("Test 1 PASSED: Initial surface size {}x{}", width, height);

    // Test 2: Verify surface format is sRGB (preferred)
    let format = surface.format();
    println!("Test 2: Surface format is {:?}", format);
    assert!(
        format.is_srgb() || !surface.capabilities().formats.iter().any(|f| f.is_srgb()),
        "Should prefer sRGB format when available"
    );
    println!("Test 2 PASSED: Format selection correct");

    // Test 3: Test resize handling
    surface.resize(640, 480).expect("Resize should succeed");
    assert_eq!(surface.size(), (640, 480), "Size should be updated");
    println!("Test 3 PASSED: Resize to 640x480");

    // Test 4: Test zero-size resize (should be skipped gracefully)
    surface.resize(0, 0).expect("Zero resize should not error");
    assert_eq!(
        surface.size(),
        (640, 480),
        "Size should remain unchanged on zero resize"
    );
    println!("Test 4 PASSED: Zero resize handled");

    // Test 5: Get current frame and render
    if let Some(frame) = surface
        .get_current_frame()
        .expect("Should get current frame")
    {
        let mut encoder = ctx.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test_encoder"),
        });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        ctx.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        println!("Test 5 PASSED: Frame rendered and presented");
    }

    // Test 6: Verify adapter info is available
    let info = ctx.adapter_info();
    println!(
        "Test 6 PASSED: Adapter: {} ({:?})",
        info.name, info.backend
    );

    println!("\n=== All surface tests PASSED ===");
}

#[test]
#[ignore = "requires GPU and window system"]
fn test_surface_creation_and_rendering() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = TestApp::new();
    event_loop.run_app(&mut app).expect("Event loop failed");
}

#[test]
fn test_graphics_config_default_values() {
    let config = GraphicsConfig::default();
    assert_eq!(config.backends, wgpu::Backends::PRIMARY);
    assert_eq!(
        config.power_preference,
        wgpu::PowerPreference::HighPerformance
    );
    assert_eq!(config.required_features, wgpu::Features::empty());
}

#[test]
fn test_surface_config_default_values() {
    let config = SurfaceConfig::default();
    assert_eq!(config.present_mode, PresentMode::AutoVsync);
    assert!(config.format.is_none());
    assert!(config.alpha_mode.is_none());
    assert_eq!(config.max_frame_latency, 2);
}

#[test]
fn test_present_mode_variants() {
    assert_eq!(PresentMode::default(), PresentMode::AutoVsync);

    // Test all variants can be constructed
    let _auto_vsync = PresentMode::AutoVsync;
    let _auto_no_vsync = PresentMode::AutoNoVsync;
    let _fifo = PresentMode::Fifo;
    let _low_latency = PresentMode::LowLatencyVsync;
    let _immediate = PresentMode::Immediate;
}
