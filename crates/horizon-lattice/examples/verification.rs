//! Horizon Lattice Verification Example
//!
//! Comprehensive test application for runtime verification of:
//! - Rendering system (shapes, colors, gradients)
//! - Mouse/keyboard input handling
//! - Localization formatting (printed to console)
//!
//! Run with: cargo run -p horizon-lattice --example verification

use std::sync::Arc;

use horizon_lattice::platform::{
    DateLength, DateTimeFormatter, NumberFormatter, TimeLength,
};
use horizon_lattice::render::{
    Color, GpuRenderer, GradientStop, GraphicsConfig, GraphicsContext, Paint, Point, Rect,
    RenderSurface, Renderer, RoundedRect, Size, Stroke, SurfaceConfig,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Application state
struct App {
    window: Option<Arc<Window>>,
    surface: Option<RenderSurface>,
    renderer: Option<GpuRenderer>,
    /// Mouse position for hover tracking
    mouse_pos: Point,
    /// Click count for testing
    click_count: u32,
    /// Current locale for testing
    current_locale: String,
    /// Available locales to cycle through
    locales: Vec<&'static str>,
    /// Current locale index
    locale_index: usize,
    /// Slider value (0.0 - 1.0)
    slider_value: f32,
    /// Whether slider is being dragged
    slider_dragging: bool,
    /// Active "tab" (render test mode)
    active_mode: usize,
    /// Hovered button index
    hovered_button: Option<usize>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            renderer: None,
            mouse_pos: Point::ZERO,
            click_count: 0,
            current_locale: "en-US".to_string(),
            locales: vec!["en-US", "en-GB", "de-DE", "fr-FR", "ja-JP", "es-ES"],
            locale_index: 0,
            slider_value: 0.5,
            slider_dragging: false,
            active_mode: 0,
            hovered_button: None,
        }
    }

    /// Button rectangles for hit testing
    fn button_rects() -> Vec<Rect> {
        vec![
            Rect::new(20.0, 420.0, 100.0, 36.0),  // Click counter
            Rect::new(130.0, 420.0, 100.0, 36.0), // Next locale
            Rect::new(240.0, 420.0, 100.0, 36.0), // Next mode
            Rect::new(350.0, 420.0, 100.0, 36.0), // Reset
        ]
    }

    fn slider_track_rect() -> Rect {
        Rect::new(20.0, 380.0, 300.0, 8.0)
    }

    fn print_localization_test(&self) {
        let num_formatter = NumberFormatter::with_locale(&self.current_locale);
        let dt_formatter = DateTimeFormatter::with_locale(&self.current_locale);
        let now = chrono::Local::now();

        println!("\n=== Localization Test ({}) ===", self.current_locale);
        println!("Number Formatting:");
        println!("  Integer 1234567: {}", num_formatter.format_integer(1234567));
        println!("  Float 1234567.89: {}", num_formatter.format(1234567.89));
        println!("  Precision 3: {}", num_formatter.format_with_precision(1234.56789, 3));

        println!("\nDate Formatting:");
        println!("  Short: {}", dt_formatter.format_date(&now, DateLength::Short));
        println!("  Medium: {}", dt_formatter.format_date(&now, DateLength::Medium));
        println!("  Long: {}", dt_formatter.format_date(&now, DateLength::Long));
        println!("  Full: {}", dt_formatter.format_date(&now, DateLength::Full));

        println!("\nTime Formatting:");
        println!("  Short: {}", dt_formatter.format_time(&now, TimeLength::Short));
        println!("  Medium: {}", dt_formatter.format_time(&now, TimeLength::Medium));
        println!("=====================================\n");
    }

    fn render(&mut self) {
        // Extract what we need from self before borrowing renderer
        let mouse_pos = self.mouse_pos;
        let click_count = self.click_count;
        let slider_value = self.slider_value;
        let slider_dragging = self.slider_dragging;
        let active_mode = self.active_mode;
        let hovered_button = self.hovered_button;
        let current_locale = self.current_locale.clone();

        let Some(surface) = &mut self.surface else { return };
        let Some(renderer) = &mut self.renderer else { return };

        // Use fixed content size matching window logical size
        let content_size = Size::new(480.0, 520.0);

        // Physical size of the surface
        let (phys_width, phys_height) = surface.size();
        let viewport = Size::new(phys_width as f32, phys_height as f32);

        // Begin frame with background
        renderer.begin_frame(Color::from_rgb8(245, 245, 250), viewport);

        // Draw prominent mode indicator panel at top
        let mode_colors = [
            Color::from_rgb8(0, 122, 255),   // Blue - Shapes
            Color::from_rgb8(255, 100, 100), // Red - Gradients
            Color::from_rgb8(100, 200, 100), // Green - Transforms
        ];
        let current_mode_color = mode_colors[active_mode % 3];

        // Mode indicator bar (use content width)
        renderer.fill_rect(
            Rect::new(0.0, 0.0, content_size.width, 10.0),
            current_mode_color,
        );

        // Mode label area with icon representing each mode
        let mode_icon_x = content_size.width - 80.0;
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(mode_icon_x - 10.0, 15.0, 80.0, 35.0), 8.0),
            Color::from_rgba8(0, 0, 0, 40),
        );

        // Draw mode icon
        match active_mode % 3 {
            0 => {
                // Shapes: square + circle
                renderer.fill_rect(
                    Rect::new(mode_icon_x, 22.0, 20.0, 20.0),
                    current_mode_color,
                );
                renderer.fill_circle(Point::new(mode_icon_x + 45.0, 32.0), 10.0, current_mode_color);
            }
            1 => {
                // Gradients: gradient bar
                renderer.fill_rect(
                    Rect::new(mode_icon_x, 25.0, 55.0, 15.0),
                    Paint::linear_gradient(
                        Point::new(mode_icon_x, 25.0),
                        Point::new(mode_icon_x + 55.0, 25.0),
                        vec![
                            GradientStop { offset: 0.0, color: Color::from_rgb8(255, 100, 100) },
                            GradientStop { offset: 0.5, color: Color::from_rgb8(100, 200, 255) },
                            GradientStop { offset: 1.0, color: Color::from_rgb8(100, 200, 100) },
                        ],
                    ),
                );
            }
            2 => {
                // Transforms: rotated squares
                renderer.save();
                renderer.translate(mode_icon_x + 28.0, 32.0);
                renderer.rotate(0.3);
                renderer.fill_rect(Rect::new(-10.0, -10.0, 20.0, 20.0), Color::from_rgba8(100, 200, 100, 180));
                renderer.restore();
                renderer.save();
                renderer.translate(mode_icon_x + 28.0, 32.0);
                renderer.rotate(-0.2);
                renderer.fill_rect(Rect::new(-8.0, -8.0, 16.0, 16.0), current_mode_color);
                renderer.restore();
            }
            _ => {}
        }

        // Click counter display (top left)
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(10.0, 15.0, 70.0, 35.0), 8.0),
            Color::from_rgba8(0, 0, 0, 40),
        );
        // Show count as filled circles (up to 10)
        for i in 0..10 {
            let x = 20.0 + (i % 5) as f32 * 10.0;
            let y = 25.0 + (i / 5) as f32 * 12.0;
            let filled = (i as u32) < click_count % 11;
            let color = if filled {
                Color::WHITE
            } else {
                Color::from_rgba8(255, 255, 255, 60)
            };
            renderer.fill_circle(Point::new(x, y), 3.5, color);
        }

        // Draw content based on active mode
        match active_mode % 3 {
            0 => Self::draw_shapes(renderer),
            1 => Self::draw_gradients(renderer),
            2 => Self::draw_transforms(renderer),
            _ => {}
        }

        // Draw interactive elements
        Self::draw_slider(renderer, slider_value, slider_dragging);
        Self::draw_buttons(renderer, hovered_button, click_count);
        Self::draw_mouse_indicator(renderer, mouse_pos);

        // Draw status area at the bottom of the content area
        Self::draw_status(renderer, content_size, active_mode, click_count, slider_value, &current_locale);

        renderer.end_frame();
        if let Err(e) = renderer.render_to_surface(surface) {
            eprintln!("Render error: {e}");
        }
    }

    fn draw_shapes(renderer: &mut GpuRenderer) {
        // Row 1: Filled shapes
        renderer.fill_rect(
            Rect::new(20.0, 30.0, 80.0, 60.0),
            Color::from_rgb8(255, 100, 100),
        );
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(120.0, 30.0, 80.0, 60.0), 15.0),
            Color::from_rgb8(100, 180, 255),
        );
        renderer.fill_circle(
            Point::new(280.0, 60.0),
            30.0,
            Color::from_rgb8(100, 200, 100),
        );

        // Row 2: Stroked shapes
        renderer.stroke_rect(
            Rect::new(20.0, 110.0, 80.0, 60.0),
            &Stroke::new(Color::from_rgb8(80, 80, 90), 2.0),
        );
        renderer.stroke_rounded_rect(
            RoundedRect::new(Rect::new(120.0, 110.0, 80.0, 60.0), 15.0),
            &Stroke::new(Color::from_rgb8(200, 100, 50), 3.0),
        );
        renderer.stroke_circle(
            Point::new(280.0, 140.0),
            30.0,
            &Stroke::new(Color::from_rgb8(150, 50, 150), 2.0),
        );

        // Row 3: Lines and polyline
        renderer.draw_line(
            Point::new(20.0, 200.0),
            Point::new(150.0, 250.0),
            &Stroke::new(Color::from_rgb8(255, 150, 50), 2.0),
        );

        let points = [
            Point::new(180.0, 200.0),
            Point::new(230.0, 250.0),
            Point::new(280.0, 210.0),
            Point::new(330.0, 260.0),
            Point::new(380.0, 220.0),
        ];
        renderer.draw_polyline(&points, &Stroke::new(Color::from_rgb8(50, 150, 200), 2.0));

        // Row 4: Clipping and opacity demo
        renderer.save();
        renderer.clip_rect(Rect::new(20.0, 280.0, 80.0, 60.0));
        renderer.fill_rect(
            Rect::new(0.0, 260.0, 140.0, 100.0),
            Color::from_rgb8(255, 200, 100),
        );
        renderer.restore();

        renderer.set_opacity(0.5);
        renderer.fill_rect(
            Rect::new(120.0, 280.0, 80.0, 60.0),
            Color::from_rgb8(100, 100, 255),
        );
        renderer.set_opacity(1.0);

        renderer.fill_rect(
            Rect::new(220.0, 280.0, 80.0, 60.0),
            Color::from_rgba8(255, 0, 0, 128),
        );
    }

    fn draw_gradients(renderer: &mut GpuRenderer) {
        // Horizontal gradient
        renderer.fill_rect(
            Rect::new(20.0, 30.0, 160.0, 80.0),
            Paint::linear_gradient(
                Point::new(20.0, 30.0),
                Point::new(180.0, 30.0),
                vec![
                    GradientStop { offset: 0.0, color: Color::from_rgb8(255, 100, 150) },
                    GradientStop { offset: 1.0, color: Color::from_rgb8(100, 200, 255) },
                ],
            ),
        );

        // Vertical gradient
        renderer.fill_rect(
            Rect::new(200.0, 30.0, 160.0, 80.0),
            Paint::linear_gradient(
                Point::new(200.0, 30.0),
                Point::new(200.0, 110.0),
                vec![
                    GradientStop { offset: 0.0, color: Color::from_rgb8(50, 50, 200) },
                    GradientStop { offset: 1.0, color: Color::from_rgb8(200, 50, 50) },
                ],
            ),
        );

        // Multi-stop rainbow gradient
        renderer.fill_rect(
            Rect::new(20.0, 130.0, 340.0, 80.0),
            Paint::linear_gradient(
                Point::new(20.0, 130.0),
                Point::new(360.0, 130.0),
                vec![
                    GradientStop { offset: 0.0, color: Color::RED },
                    GradientStop { offset: 0.25, color: Color::from_rgb8(255, 165, 0) },
                    GradientStop { offset: 0.5, color: Color::YELLOW },
                    GradientStop { offset: 0.75, color: Color::GREEN },
                    GradientStop { offset: 1.0, color: Color::BLUE },
                ],
            ),
        );

        // Diagonal gradient
        renderer.fill_rect(
            Rect::new(20.0, 230.0, 160.0, 100.0),
            Paint::linear_gradient(
                Point::new(20.0, 230.0),
                Point::new(180.0, 330.0),
                vec![
                    GradientStop { offset: 0.0, color: Color::from_rgb8(255, 255, 255) },
                    GradientStop { offset: 0.5, color: Color::from_rgb8(100, 100, 200) },
                    GradientStop { offset: 1.0, color: Color::from_rgb8(0, 0, 100) },
                ],
            ),
        );

        // Gradient in rounded rect
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(200.0, 230.0, 160.0, 100.0), 20.0),
            Paint::linear_gradient(
                Point::new(200.0, 230.0),
                Point::new(360.0, 330.0),
                vec![
                    GradientStop { offset: 0.0, color: Color::from_rgb8(255, 200, 100) },
                    GradientStop { offset: 1.0, color: Color::from_rgb8(200, 100, 50) },
                ],
            ),
        );
    }

    fn draw_transforms(renderer: &mut GpuRenderer) {
        // Basic rotation
        renderer.save();
        renderer.translate(80.0, 80.0);
        renderer.rotate(0.3);
        renderer.fill_rect(
            Rect::new(-40.0, -30.0, 80.0, 60.0),
            Color::from_rgb8(255, 100, 100),
        );
        renderer.restore();

        // Scaling
        renderer.save();
        renderer.translate(220.0, 80.0);
        renderer.scale(1.5, 0.8);
        renderer.fill_rect(
            Rect::new(-30.0, -30.0, 60.0, 60.0),
            Color::from_rgb8(100, 180, 255),
        );
        renderer.restore();

        // Combined transforms
        renderer.save();
        renderer.translate(350.0, 80.0);
        renderer.rotate(-0.2);
        renderer.scale(1.2, 1.2);
        renderer.fill_rounded_rect(
            RoundedRect::new(Rect::new(-35.0, -25.0, 70.0, 50.0), 10.0),
            Color::from_rgb8(100, 200, 100),
        );
        renderer.restore();

        // Nested transforms
        for i in 0..5 {
            let angle = i as f32 * 0.15;
            let scale = 1.0 - i as f32 * 0.15;
            let alpha = 255 - i as u8 * 40;

            renderer.save();
            renderer.translate(150.0, 250.0);
            renderer.rotate(angle);
            renderer.scale(scale, scale);
            renderer.fill_rect(
                Rect::new(-40.0, -40.0, 80.0, 80.0),
                Color::from_rgba8(100, 150, 255, alpha),
            );
            renderer.restore();
        }

        // Clipping with transform
        renderer.save();
        renderer.translate(350.0, 250.0);
        renderer.rotate(0.4);
        renderer.clip_rect(Rect::new(-30.0, -30.0, 60.0, 60.0));
        renderer.fill_circle(Point::ZERO, 50.0, Color::from_rgb8(255, 200, 100));
        renderer.restore();
    }

    fn draw_slider(renderer: &mut GpuRenderer, slider_value: f32, slider_dragging: bool) {
        let track = Self::slider_track_rect();

        // Track background
        renderer.fill_rounded_rect(
            RoundedRect::new(track, 4.0),
            Color::from_rgb8(200, 200, 205),
        );

        // Filled portion
        let filled_width = track.width() * slider_value;
        renderer.fill_rounded_rect(
            RoundedRect::new(
                Rect::new(track.origin.x, track.origin.y, filled_width, track.height()),
                4.0,
            ),
            Color::from_rgb8(0, 122, 255),
        );

        // Thumb
        let thumb_x = track.origin.x + (track.width() - 16.0) * slider_value;
        let thumb_center = Point::new(thumb_x + 8.0, track.center().y);
        let thumb_color = if slider_dragging {
            Color::from_rgb8(0, 100, 220)
        } else {
            Color::WHITE
        };
        renderer.fill_circle(thumb_center, 10.0, thumb_color);
        renderer.stroke_circle(
            thumb_center,
            10.0,
            &Stroke::new(Color::from_rgb8(0, 122, 255), 2.0),
        );
    }

    fn draw_buttons(renderer: &mut GpuRenderer, hovered_button: Option<usize>, click_count: u32) {
        let buttons = Self::button_rects();

        for (i, rect) in buttons.iter().enumerate() {
            let is_hovered = hovered_button == Some(i);

            let bg_color = if is_hovered {
                Color::from_rgb8(0, 132, 255)
            } else {
                Color::from_rgb8(0, 122, 255)
            };

            renderer.fill_rounded_rect(RoundedRect::new(*rect, 8.0), bg_color);

            // Simple visual indicator using shapes instead of text
            let center = rect.center();
            match i {
                0 => {
                    // Click counter - show number of dots
                    let count = (click_count % 5) as i32;
                    for j in 0..count {
                        let x = center.x - 20.0 + j as f32 * 10.0;
                        renderer.fill_circle(Point::new(x, center.y), 4.0, Color::WHITE);
                    }
                    if count == 0 {
                        renderer.fill_circle(center, 4.0, Color::from_rgba8(255, 255, 255, 128));
                    }
                }
                1 => {
                    // Locale - globe icon (circle with lines)
                    renderer.stroke_circle(center, 12.0, &Stroke::new(Color::WHITE, 2.0));
                    renderer.draw_line(
                        Point::new(center.x - 12.0, center.y),
                        Point::new(center.x + 12.0, center.y),
                        &Stroke::new(Color::WHITE, 1.0),
                    );
                    renderer.draw_line(
                        Point::new(center.x, center.y - 12.0),
                        Point::new(center.x, center.y + 12.0),
                        &Stroke::new(Color::WHITE, 1.0),
                    );
                }
                2 => {
                    // Mode - arrow triangle
                    let points = [
                        Point::new(center.x - 8.0, center.y - 10.0),
                        Point::new(center.x + 8.0, center.y),
                        Point::new(center.x - 8.0, center.y + 10.0),
                    ];
                    renderer.draw_polyline(&points, &Stroke::new(Color::WHITE, 2.0));
                }
                3 => {
                    // Reset - X
                    renderer.draw_line(
                        Point::new(center.x - 8.0, center.y - 8.0),
                        Point::new(center.x + 8.0, center.y + 8.0),
                        &Stroke::new(Color::WHITE, 2.0),
                    );
                    renderer.draw_line(
                        Point::new(center.x + 8.0, center.y - 8.0),
                        Point::new(center.x - 8.0, center.y + 8.0),
                        &Stroke::new(Color::WHITE, 2.0),
                    );
                }
                _ => {}
            }
        }
    }

    fn draw_mouse_indicator(renderer: &mut GpuRenderer, mouse_pos: Point) {
        // Draw a small indicator at mouse position
        renderer.fill_circle(mouse_pos, 5.0, Color::from_rgba8(255, 0, 0, 128));
    }

    fn draw_status(renderer: &mut GpuRenderer, viewport: Size, active_mode: usize, click_count: u32, slider_value: f32, current_locale: &str) {
        // Status bar background
        renderer.fill_rect(
            Rect::new(0.0, viewport.height - 24.0, viewport.width, 24.0),
            Color::from_rgb8(235, 235, 240),
        );

        // Mode indicators (colored squares)
        for i in 0..3 {
            let x = 10.0 + i as f32 * 30.0;
            let color = if i == active_mode % 3 {
                Color::from_rgb8(0, 122, 255)
            } else {
                Color::from_rgb8(180, 180, 185)
            };
            renderer.fill_rounded_rect(
                RoundedRect::new(Rect::new(x, viewport.height - 18.0, 20.0, 12.0), 2.0),
                color,
            );
        }

        // Click count indicator
        let count_x = 120.0;
        for i in 0..5 {
            let filled = i < (click_count % 6) as usize;
            let color = if filled {
                Color::from_rgb8(0, 200, 100)
            } else {
                Color::from_rgb8(200, 200, 205)
            };
            renderer.fill_circle(
                Point::new(count_x + i as f32 * 12.0, viewport.height - 12.0),
                4.0,
                color,
            );
        }

        // Slider value indicator
        let slider_indicator_x = 200.0;
        let bar_width = 60.0 * slider_value;
        renderer.fill_rounded_rect(
            RoundedRect::new(
                Rect::new(slider_indicator_x, viewport.height - 16.0, 60.0, 8.0),
                2.0,
            ),
            Color::from_rgb8(200, 200, 205),
        );
        renderer.fill_rounded_rect(
            RoundedRect::new(
                Rect::new(slider_indicator_x, viewport.height - 16.0, bar_width, 8.0),
                2.0,
            ),
            Color::from_rgb8(0, 122, 255),
        );

        // Locale indicator (flag-like colored stripes)
        let locale_x = 280.0;
        let locale_colors = match current_locale {
            "en-US" => [Color::from_rgb8(60, 60, 150), Color::WHITE, Color::from_rgb8(180, 50, 50)],
            "en-GB" => [Color::from_rgb8(0, 36, 125), Color::WHITE, Color::from_rgb8(200, 16, 46)],
            "de-DE" => [Color::BLACK, Color::from_rgb8(221, 0, 0), Color::from_rgb8(255, 206, 0)],
            "fr-FR" => [Color::from_rgb8(0, 35, 149), Color::WHITE, Color::from_rgb8(237, 41, 57)],
            "ja-JP" => [Color::WHITE, Color::from_rgb8(188, 0, 45), Color::WHITE],
            "es-ES" => [Color::from_rgb8(170, 21, 27), Color::from_rgb8(241, 191, 0), Color::from_rgb8(170, 21, 27)],
            _ => [Color::GRAY, Color::GRAY, Color::GRAY],
        };
        for (i, color) in locale_colors.iter().enumerate() {
            renderer.fill_rect(
                Rect::new(locale_x + i as f32 * 10.0, viewport.height - 18.0, 10.0, 12.0),
                *color,
            );
        }
    }

    fn handle_click(&mut self, pos: Point) {
        let buttons = Self::button_rects();

        for (i, rect) in buttons.iter().enumerate() {
            if rect.contains(pos) {
                match i {
                    0 => {
                        self.click_count += 1;
                        println!("Click count: {}", self.click_count);
                    }
                    1 => {
                        self.locale_index = (self.locale_index + 1) % self.locales.len();
                        self.current_locale = self.locales[self.locale_index].to_string();
                        self.print_localization_test();
                    }
                    2 => {
                        self.active_mode = (self.active_mode + 1) % 3;
                        let mode_names = ["Shapes", "Gradients", "Transforms"];
                        println!("Mode: {}", mode_names[self.active_mode]);
                    }
                    3 => {
                        self.click_count = 0;
                        self.slider_value = 0.5;
                        self.active_mode = 0;
                        self.locale_index = 0;
                        self.current_locale = "en-US".to_string();
                        println!("Reset!");
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check slider
        let track_rect = Self::slider_track_rect();
        let expanded_track = Rect::new(
            track_rect.origin.x - 10.0,
            track_rect.origin.y - 10.0,
            track_rect.width() + 20.0,
            track_rect.height() + 20.0,
        );
        if expanded_track.contains(pos) {
            self.slider_value = ((pos.x - track_rect.origin.x) / track_rect.width()).clamp(0.0, 1.0);
            println!("Slider: {:.2}", self.slider_value);
        }
    }

    fn update_hover(&mut self, pos: Point) {
        self.mouse_pos = pos;
        self.hovered_button = None;

        for (i, rect) in Self::button_rects().iter().enumerate() {
            if rect.contains(pos) {
                self.hovered_button = Some(i);
                break;
            }
        }

        // Update slider if dragging
        if self.slider_dragging {
            let track = Self::slider_track_rect();
            self.slider_value = ((pos.x - track.origin.x) / track.width()).clamp(0.0, 1.0);
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

        // Create window with fixed physical size (no DPI scaling confusion)
        let attrs = Window::default_attributes()
            .with_title("Horizon Lattice - Verification")
            .with_inner_size(winit::dpi::PhysicalSize::new(480, 520))
            .with_resizable(false);

        let window = Arc::new(event_loop.create_window(attrs).expect("Failed to create window"));

        // Create surface and renderer
        let surface = RenderSurface::new(Arc::clone(&window), SurfaceConfig::default())
            .expect("Failed to create surface");

        let renderer = GpuRenderer::new(&surface).expect("Failed to create renderer");

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);

        // Print initial localization test
        self.print_localization_test();
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
            WindowEvent::CursorMoved { position, .. } => {
                // Position is in physical pixels, use directly
                self.update_hover(Point::new(position.x as f32, position.y as f32));
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            // Check if starting slider drag
                            let track_rect = Self::slider_track_rect();
                            let expanded_track = Rect::new(
                                track_rect.origin.x - 10.0,
                                track_rect.origin.y - 10.0,
                                track_rect.width() + 20.0,
                                track_rect.height() + 20.0,
                            );
                            if expanded_track.contains(self.mouse_pos) {
                                self.slider_dragging = true;
                                self.slider_value = ((self.mouse_pos.x - track_rect.origin.x) / track_rect.width()).clamp(0.0, 1.0);
                            }
                        }
                        ElementState::Released => {
                            if !self.slider_dragging {
                                self.handle_click(self.mouse_pos);
                            }
                            self.slider_dragging = false;
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        Key::Named(NamedKey::Tab) | Key::Named(NamedKey::ArrowRight) => {
                            self.active_mode = (self.active_mode + 1) % 3;
                            let mode_names = ["Shapes", "Gradients", "Transforms"];
                            println!("Mode: {}", mode_names[self.active_mode]);
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.active_mode = if self.active_mode == 0 { 2 } else { self.active_mode - 1 };
                            let mode_names = ["Shapes", "Gradients", "Transforms"];
                            println!("Mode: {}", mode_names[self.active_mode]);
                        }
                        Key::Character(ref c) if c == " " => {
                            self.click_count += 1;
                            println!("Click count: {}", self.click_count);
                        }
                        Key::Character(ref c) if c == "l" || c == "L" => {
                            self.locale_index = (self.locale_index + 1) % self.locales.len();
                            self.current_locale = self.locales[self.locale_index].to_string();
                            self.print_localization_test();
                        }
                        _ => {}
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
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

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║       Horizon Lattice Verification Application            ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║ Tests:                                                    ║");
    println!("║   • Rendering: shapes, gradients, transforms, clipping    ║");
    println!("║   • Input: mouse tracking, clicks, keyboard               ║");
    println!("║   • Localization: number/date/time formatting             ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║ Controls:                                                 ║");
    println!("║   Tab/Arrows - Switch render mode                         ║");
    println!("║   Space      - Increment click counter                    ║");
    println!("║   L          - Cycle through locales                      ║");
    println!("║   Escape     - Quit                                       ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::new();

    event_loop.run_app(&mut app).expect("Event loop error");
}
