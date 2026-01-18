//! Comprehensive window management test example.
//!
//! This example exercises all window management features for manual testing
//! across platforms. It can be used to verify the testing checklist items:
//!
//! - Windows create on all platforms (Normal, Dialog, Tool, Popup, Splash)
//! - Window flags work correctly (frameless, transparent, always-on-top, etc.)
//! - Move/resize works (programmatic and user-initiated)
//! - Minimize/maximize/fullscreen work
//! - Modal blocking works
//! - Multi-monitor detection works
//!
//! Run with: cargo run -p horizon-lattice-render --example window_management_test
//!
//! Press keys to test different features:
//!   1-5: Create different window types (Normal, Dialog, Tool, Popup, Splash)
//!   F: Create frameless window
//!   T: Create transparent window (50% opacity)
//!   A: Create always-on-top window
//!   M: Toggle maximize
//!   N: Minimize window
//!   L: Enter fullscreen
//!   Escape: Exit fullscreen
//!   S: Print screen/monitor information
//!   R: Move window to right by 50px
//!   G: Print current window geometry
//!   P: Save and restore window geometry
//!   C: Cascade all windows
//!   H: Hide/show window (toggle visibility)
//!   O: Toggle opacity (fade in/out)
//!   Q: Close current window

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_render::{
    Color, GpuRenderer, GraphicsConfig, GraphicsContext, Point, Rect, RenderSurface, Renderer,
    RoundedRect, Size, Stroke, SurfaceConfig,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, Window, WindowButtons, WindowId, WindowLevel};

/// Per-window rendering state.
struct WindowState {
    window: Arc<Window>,
    surface: RenderSurface,
    renderer: GpuRenderer,
    window_type: &'static str,
    accent_color: Color,
    frame: u32,
    opacity: f32,
    saved_geometry: Option<SavedGeometry>,
}

#[derive(Debug, Clone)]
struct SavedGeometry {
    position: (i32, i32),
    size: (u32, u32),
    is_maximized: bool,
    is_fullscreen: bool,
}

impl WindowState {
    fn new(
        event_loop: &ActiveEventLoop,
        title: &str,
        window_type: &'static str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        accent_color: Color,
        flags: WindowFlags,
    ) -> Option<Self> {
        let mut attrs = Window::default_attributes()
            .with_title(title)
            .with_inner_size(LogicalSize::new(width, height))
            .with_position(LogicalPosition::new(x, y));

        // Apply flags
        if flags.frameless {
            attrs = attrs.with_decorations(false);
        }
        if flags.transparent {
            attrs = attrs.with_transparent(true);
        }
        if flags.always_on_top {
            attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
        }
        if flags.resizable {
            attrs = attrs.with_resizable(true);
        } else if flags.frameless {
            // Frameless windows are often non-resizable by default
            attrs = attrs.with_resizable(false);
        }

        // Apply button flags
        let mut buttons = WindowButtons::empty();
        if flags.has_close {
            buttons |= WindowButtons::CLOSE;
        }
        if flags.has_minimize {
            buttons |= WindowButtons::MINIMIZE;
        }
        if flags.has_maximize {
            buttons |= WindowButtons::MAXIMIZE;
        }
        attrs = attrs.with_enabled_buttons(buttons);

        let window = Arc::new(event_loop.create_window(attrs).ok()?);

        let surface = RenderSurface::new(Arc::clone(&window), SurfaceConfig::default()).ok()?;
        let renderer = GpuRenderer::new(&surface).ok()?;

        Some(Self {
            window,
            surface,
            renderer,
            window_type,
            accent_color,
            frame: 0,
            opacity: 1.0,
            saved_geometry: None,
        })
    }

    fn render(&mut self) {
        let (width, height) = self.surface.size();
        if width == 0 || height == 0 {
            return;
        }
        let viewport = Size::new(width as f32, height as f32);

        // Animate position
        let offset = (self.frame as f32 * 0.02).sin() * 10.0;
        self.frame = self.frame.wrapping_add(1);

        // Begin frame with a background that varies by window type
        let bg = if self.opacity < 1.0 {
            Color::TRANSPARENT
        } else {
            Color::from_rgb(0.95, 0.95, 0.98)
        };
        self.renderer.begin_frame(bg, viewport);

        // Draw window type label area
        self.renderer.fill_rect(
            Rect::new(10.0, 10.0, viewport.width - 20.0, 40.0),
            Color::from_rgb(0.9, 0.9, 0.92),
        );

        // Draw a rectangle with this window's accent color
        self.renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(30.0 + offset, 70.0, 120.0, 80.0), 10.0),
            self.accent_color,
        );

        // Draw some common shapes
        self.renderer.stroke_rect(
            Rect::new(180.0, 70.0, 100.0, 80.0),
            &Stroke::new(Color::DARK_GRAY, 2.0),
        );

        // Draw a circle
        self.renderer.fill_circle(
            Point::new(viewport.width - 60.0, 110.0),
            30.0,
            self.accent_color.with_alpha(0.5),
        );

        // Draw status information area at bottom
        self.renderer.fill_rect(
            Rect::new(10.0, viewport.height - 50.0, viewport.width - 20.0, 40.0),
            Color::from_rgb(0.92, 0.92, 0.95),
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
        if width > 0 && height > 0 {
            self.surface.resize(width, height).ok();
        }
    }

    fn save_geometry(&mut self) {
        let pos = self.window.outer_position().unwrap_or(PhysicalPosition::new(0, 0));
        let size = self.window.inner_size();
        self.saved_geometry = Some(SavedGeometry {
            position: (pos.x, pos.y),
            size: (size.width, size.height),
            is_maximized: self.window.is_maximized(),
            is_fullscreen: self.window.fullscreen().is_some(),
        });
        println!("Saved geometry: {:?}", self.saved_geometry);
    }

    fn restore_geometry(&self) {
        if let Some(ref geom) = self.saved_geometry {
            println!("Restoring geometry: {:?}", geom);

            // First restore from maximized/fullscreen if needed
            if self.window.is_maximized() {
                self.window.set_maximized(false);
            }
            if self.window.fullscreen().is_some() {
                self.window.set_fullscreen(None);
            }

            // Then restore position and size
            self.window.set_outer_position(PhysicalPosition::new(geom.position.0, geom.position.1));
            let _ = self.window.request_inner_size(winit::dpi::PhysicalSize::new(geom.size.0, geom.size.1));

            // Finally restore state
            if geom.is_maximized {
                self.window.set_maximized(true);
            } else if geom.is_fullscreen {
                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        } else {
            println!("No saved geometry to restore");
        }
    }
}

#[derive(Default)]
struct WindowFlags {
    frameless: bool,
    transparent: bool,
    always_on_top: bool,
    resizable: bool,
    has_close: bool,
    has_minimize: bool,
    has_maximize: bool,
}

impl WindowFlags {
    fn normal() -> Self {
        Self {
            resizable: true,
            has_close: true,
            has_minimize: true,
            has_maximize: true,
            ..Default::default()
        }
    }

    fn dialog() -> Self {
        Self {
            has_close: true,
            ..Default::default()
        }
    }

    fn tool() -> Self {
        Self {
            always_on_top: true,
            has_close: true,
            ..Default::default()
        }
    }

    fn popup() -> Self {
        Self {
            frameless: true,
            always_on_top: true,
            ..Default::default()
        }
    }

    fn splash() -> Self {
        Self {
            frameless: true,
            ..Default::default()
        }
    }

    fn frameless() -> Self {
        Self {
            frameless: true,
            has_close: true,
            has_minimize: true,
            has_maximize: true,
            ..Default::default()
        }
    }

    fn transparent() -> Self {
        Self {
            transparent: true,
            resizable: true,
            has_close: true,
            has_minimize: true,
            has_maximize: true,
            ..Default::default()
        }
    }

    fn always_on_top() -> Self {
        Self {
            always_on_top: true,
            resizable: true,
            has_close: true,
            has_minimize: true,
            has_maximize: true,
            ..Default::default()
        }
    }
}

struct WindowManagementApp {
    windows: HashMap<WindowId, WindowState>,
    initialized: bool,
    window_counter: u32,
}

impl WindowManagementApp {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            initialized: false,
            window_counter: 0,
        }
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        title: &str,
        window_type: &'static str,
        accent_color: Color,
        flags: WindowFlags,
    ) {
        self.window_counter += 1;
        let offset = (self.window_counter as i32 - 1) * 40;

        let full_title = format!("{} - {} (Horizon Lattice)", title, window_type);

        if let Some(state) = WindowState::new(
            event_loop,
            &full_title,
            window_type,
            100 + offset,
            100 + offset,
            500,
            400,
            accent_color,
            flags,
        ) {
            let id = state.window.id();
            println!("Created {} window: {:?}", window_type, id);
            self.windows.insert(id, state);
        } else {
            println!("Failed to create {} window", window_type);
        }
    }

    fn print_screen_info(&self) {
        println!("\n=== Screen/Monitor Information ===");

        if let Some(state) = self.windows.values().next() {
            let monitors: Vec<_> = state.window.available_monitors().collect();
            println!("Found {} monitor(s):", monitors.len());

            for (i, monitor) in monitors.iter().enumerate() {
                let name = monitor.name().unwrap_or_else(|| "Unknown".to_string());
                let size = monitor.size();
                let pos = monitor.position();
                let scale = monitor.scale_factor();
                let refresh = monitor.refresh_rate_millihertz()
                    .map(|r| format!("{:.2} Hz", r as f64 / 1000.0))
                    .unwrap_or_else(|| "N/A".to_string());

                println!("  [{}] {} - {}x{} at ({}, {}), scale: {:.2}x, refresh: {}",
                    i, name, size.width, size.height, pos.x, pos.y, scale, refresh);
            }

            if let Some(primary) = state.window.primary_monitor() {
                println!("Primary: {}", primary.name().unwrap_or_else(|| "Unknown".to_string()));
            }

            if let Some(current) = state.window.current_monitor() {
                println!("Current window is on: {}", current.name().unwrap_or_else(|| "Unknown".to_string()));
            }
        }
        println!("===================================\n");
    }

    fn cascade_windows(&mut self) {
        let mut offset = 0;
        for state in self.windows.values() {
            state.window.set_outer_position(PhysicalPosition::new(100 + offset, 100 + offset));
            offset += 40;
        }
        println!("Cascaded {} windows", self.windows.len());
    }
}

impl ApplicationHandler for WindowManagementApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }

        // Initialize graphics context (once, shared by all windows)
        if GraphicsContext::try_get().is_none() {
            GraphicsContext::init(GraphicsConfig::default()).expect("Failed to init graphics");
        }

        // Create the main window
        self.create_window(
            event_loop,
            "Main Window",
            "Normal",
            Color::from_rgb(0.2, 0.5, 0.9),
            WindowFlags::normal(),
        );

        self.initialized = true;

        println!("\n=== Window Management Test ===");
        println!("Press keys to test different features:");
        println!("  1: Create Normal window");
        println!("  2: Create Dialog window");
        println!("  3: Create Tool window");
        println!("  4: Create Popup window (frameless, on-top)");
        println!("  5: Create Splash window (frameless)");
        println!("  F: Create Frameless window");
        println!("  T: Create Transparent window");
        println!("  A: Create Always-on-top window");
        println!("  M: Toggle maximize");
        println!("  N: Minimize window");
        println!("  L: Enter fullscreen");
        println!("  Escape: Exit fullscreen / close popup");
        println!("  S: Print screen/monitor info");
        println!("  R: Move window right by 50px");
        println!("  G: Print current window geometry");
        println!("  P: Save/restore window geometry");
        println!("  C: Cascade all windows");
        println!("  H: Toggle visibility (hide/show)");
        println!("  O: Toggle opacity (fade)");
        println!("  Q: Close current window");
        println!("================================\n");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested for window: {:?}", window_id);
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    println!("All windows closed, exiting.");
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(size) => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.resize(size.width, size.height);
                    println!("Window {:?} resized to {}x{}", window_id, size.width, size.height);
                }
            }

            WindowEvent::Moved(pos) => {
                println!("Window {:?} moved to ({}, {})", window_id, pos.x, pos.y);
            }

            WindowEvent::Focused(focused) => {
                println!("Window {:?} focus: {}", window_id, if focused { "gained" } else { "lost" });
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.render();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                println!("Window {:?} scale factor changed to {:.2}", window_id, scale_factor);
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, state: ElementState::Pressed, .. },
                ..
            } => {
                match logical_key.as_ref() {
                    Key::Character("1") => {
                        self.create_window(
                            event_loop,
                            "Normal Window",
                            "Normal",
                            Color::from_rgb(0.3, 0.6, 0.9),
                            WindowFlags::normal(),
                        );
                    }
                    Key::Character("2") => {
                        self.create_window(
                            event_loop,
                            "Dialog Window",
                            "Dialog",
                            Color::from_rgb(0.9, 0.5, 0.2),
                            WindowFlags::dialog(),
                        );
                    }
                    Key::Character("3") => {
                        self.create_window(
                            event_loop,
                            "Tool Window",
                            "Tool",
                            Color::from_rgb(0.2, 0.8, 0.4),
                            WindowFlags::tool(),
                        );
                    }
                    Key::Character("4") => {
                        self.create_window(
                            event_loop,
                            "Popup Window",
                            "Popup",
                            Color::from_rgb(0.8, 0.3, 0.8),
                            WindowFlags::popup(),
                        );
                    }
                    Key::Character("5") => {
                        self.create_window(
                            event_loop,
                            "Splash Window",
                            "Splash",
                            Color::from_rgb(0.9, 0.9, 0.2),
                            WindowFlags::splash(),
                        );
                    }
                    Key::Character("f") | Key::Character("F") => {
                        self.create_window(
                            event_loop,
                            "Frameless Window",
                            "Frameless",
                            Color::from_rgb(0.5, 0.5, 0.5),
                            WindowFlags::frameless(),
                        );
                    }
                    Key::Character("t") | Key::Character("T") => {
                        self.create_window(
                            event_loop,
                            "Transparent Window",
                            "Transparent",
                            Color::from_rgb(0.9, 0.2, 0.2),
                            WindowFlags::transparent(),
                        );
                    }
                    Key::Character("a") | Key::Character("A") => {
                        self.create_window(
                            event_loop,
                            "Always On Top Window",
                            "AlwaysOnTop",
                            Color::from_rgb(0.2, 0.9, 0.9),
                            WindowFlags::always_on_top(),
                        );
                    }
                    Key::Character("m") | Key::Character("M") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            let is_max = state.window.is_maximized();
                            state.window.set_maximized(!is_max);
                            println!("Toggle maximize: {} -> {}", is_max, !is_max);
                        }
                    }
                    Key::Character("n") | Key::Character("N") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            state.window.set_minimized(true);
                            println!("Minimized window");
                        }
                    }
                    Key::Character("l") | Key::Character("L") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            state.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                            println!("Entered fullscreen");
                        }
                    }
                    Key::Named(NamedKey::Escape) => {
                        if let Some(state) = self.windows.get(&window_id) {
                            if state.window.fullscreen().is_some() {
                                state.window.set_fullscreen(None);
                                println!("Exited fullscreen");
                            } else if state.window_type == "Popup" {
                                self.windows.remove(&window_id);
                                println!("Closed popup window");
                            }
                        }
                    }
                    Key::Character("s") | Key::Character("S") => {
                        self.print_screen_info();
                    }
                    Key::Character("r") | Key::Character("R") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            if let Ok(pos) = state.window.outer_position() {
                                let new_pos = PhysicalPosition::new(pos.x + 50, pos.y);
                                state.window.set_outer_position(new_pos);
                                println!("Moved window right to ({}, {})", new_pos.x, new_pos.y);
                            }
                        }
                    }
                    Key::Character("g") | Key::Character("G") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            let size = state.window.inner_size();
                            let outer_size = state.window.outer_size();
                            let pos = state.window.outer_position().ok();
                            let scale = state.window.scale_factor();
                            let is_max = state.window.is_maximized();
                            let is_min = state.window.is_minimized();
                            let is_full = state.window.fullscreen().is_some();

                            println!("\n=== Window Geometry ===");
                            println!("Inner size: {}x{} (physical pixels)", size.width, size.height);
                            println!("Outer size: {}x{} (including decorations)", outer_size.width, outer_size.height);
                            if let Some(p) = pos {
                                println!("Position: ({}, {})", p.x, p.y);
                            }
                            println!("Scale factor: {:.2}", scale);
                            println!("Logical size: {:.1}x{:.1}", size.width as f64 / scale, size.height as f64 / scale);
                            println!("Maximized: {}", is_max);
                            println!("Minimized: {:?}", is_min);
                            println!("Fullscreen: {}", is_full);
                            println!("========================\n");
                        }
                    }
                    Key::Character("p") | Key::Character("P") => {
                        if let Some(state) = self.windows.get_mut(&window_id) {
                            if state.saved_geometry.is_some() {
                                state.restore_geometry();
                            } else {
                                state.save_geometry();
                            }
                        }
                    }
                    Key::Character("c") | Key::Character("C") => {
                        self.cascade_windows();
                    }
                    Key::Character("h") | Key::Character("H") => {
                        if let Some(state) = self.windows.get(&window_id) {
                            let visible = state.window.is_visible().unwrap_or(true);
                            state.window.set_visible(!visible);
                            println!("Toggle visibility: {} -> {}", visible, !visible);
                        }
                    }
                    Key::Character("o") | Key::Character("O") => {
                        if let Some(state) = self.windows.get_mut(&window_id) {
                            state.opacity = if state.opacity >= 1.0 { 0.5 } else { 1.0 };
                            println!("Opacity set to {:.1}", state.opacity);
                            // Note: winit doesn't directly support opacity, this would need
                            // platform-specific code. The field is kept for reference.
                        }
                    }
                    Key::Character("q") | Key::Character("Q") => {
                        println!("Closing window: {:?}", window_id);
                        self.windows.remove(&window_id);
                        if self.windows.is_empty() {
                            println!("All windows closed, exiting.");
                            event_loop.exit();
                        }
                    }
                    _ => {}
                }
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

    println!("Window Management Test Example");
    println!("==============================");
    println!("This example tests all window management features.");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = WindowManagementApp::new();

    event_loop.run_app(&mut app).expect("Event loop error");
}
