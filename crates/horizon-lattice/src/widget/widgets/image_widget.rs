//! Image display widget.
//!
//! The ImageWidget displays static or animated images with support for:
//! - Multiple image sources (file, bytes, URL)
//! - Animated images (GIF) with playback control
//! - Various scaling modes (Fit, Fill, Stretch, Tile)
//! - Loading state with placeholder
//! - Error state handling
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ImageWidget;
//! use horizon_lattice_render::ImageScaleMode;
//!
//! // Create an image widget from a file
//! let mut image = ImageWidget::new();
//! image.set_source_file("photo.png");
//!
//! // Set scaling mode
//! image.set_scale_mode(ImageScaleMode::Fit);
//!
//! // For animated GIFs
//! let mut gif = ImageWidget::new();
//! gif.set_source_file("animation.gif");
//! gif.set_auto_play(true); // Auto-play animations
//!
//! // Handle loading completion
//! gif.loaded.connect(|()| {
//!     println!("Image loaded!");
//! });
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    AnimatedImage, AnimationController, AsyncImageHandle, AsyncImageLoader, Color, HorizontalAlign,
    Image, ImageManager, ImageScaleMode, LoadingState, Rect, Renderer, Size, VerticalAlign,
};

use crate::widget::{FocusPolicy, PaintContext, SizeHint, Widget, WidgetBase};

/// The content source for an ImageWidget.
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum ImageSource {
    /// No image set.
    #[default]
    None,
    /// Load from a file path.
    File(PathBuf),
    /// Load from bytes in memory.
    Bytes(Vec<u8>),
    /// Load from a URL (requires networking feature).
    #[cfg(feature = "networking")]
    Url(String),
}


/// The current state of the ImageWidget.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum ImageWidgetState {
    /// No image source set.
    #[default]
    Empty,
    /// Image is being loaded.
    Loading,
    /// Image loaded successfully.
    Ready,
    /// Image failed to load.
    Error(String),
}


/// Internal image content storage.
#[derive(Default)]
enum ImageContent {
    /// No content.
    #[default]
    None,
    /// Static image.
    Static(Image),
    /// Animated image with controller.
    Animated {
        image: AnimatedImage,
        controller: AnimationController,
        /// Cached frames uploaded to GPU (lazily populated).
        gpu_frames: Vec<Option<Image>>,
    },
}


/// A widget for displaying images.
///
/// ImageWidget supports both static and animated images with various scaling
/// modes and loading states.
///
/// # Loading Images
///
/// Images can be loaded from multiple sources:
///
/// ```ignore
/// let mut widget = ImageWidget::new();
///
/// // From file
/// widget.set_source_file("image.png");
///
/// // From bytes
/// widget.set_source_bytes(png_data);
///
/// // From URL (with networking feature)
/// #[cfg(feature = "networking")]
/// widget.set_source_url("https://example.com/image.png");
///
/// // Pre-loaded Image
/// widget.set_image(loaded_image);
/// ```
///
/// # Animation Control
///
/// For animated images (GIF), playback can be controlled:
///
/// ```ignore
/// widget.set_auto_play(true);  // Auto-play when loaded
/// widget.play();               // Start playback
/// widget.pause();              // Pause playback
/// widget.stop();               // Stop and reset to first frame
/// widget.set_speed(2.0);       // 2x playback speed
/// ```
///
/// # Signals
///
/// - `loaded`: Emitted when image loads successfully
/// - `error`: Emitted when image fails to load, with error message
/// - `state_changed`: Emitted when widget state changes
/// - `frame_changed`: Emitted when animation frame changes
pub struct ImageWidget {
    /// Widget base for common functionality.
    base: WidgetBase,

    /// The image source to load from.
    source: ImageSource,

    /// Current widget state.
    state: ImageWidgetState,

    /// The actual image content.
    content: ImageContent,

    /// Placeholder image shown during loading.
    placeholder: Option<Image>,

    /// Image shown when loading fails.
    error_image: Option<Image>,

    /// How to scale the image within the widget bounds.
    scale_mode: ImageScaleMode,

    /// Horizontal alignment when image doesn't fill width (for Fit mode).
    horizontal_align: HorizontalAlign,

    /// Vertical alignment when image doesn't fill height (for Fit mode).
    vertical_align: VerticalAlign,

    /// Whether to automatically play animations when loaded.
    auto_play: bool,

    /// Handle for async loading (if in progress).
    async_handle: Option<AsyncImageHandle>,

    /// Background color (shown behind transparent images or in empty areas).
    background_color: Option<Color>,

    /// Fixed size override (None = use image's natural size).
    fixed_size: Option<Size>,

    /// Signal emitted when image loads successfully.
    pub loaded: Signal<()>,

    /// Signal emitted when image fails to load.
    pub error: Signal<String>,

    /// Signal emitted when state changes.
    pub state_changed: Signal<ImageWidgetState>,

    /// Signal emitted when animation frame changes.
    pub frame_changed: Signal<usize>,
}

impl ImageWidget {
    /// Create a new empty ImageWidget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);

        Self {
            base,
            source: ImageSource::None,
            state: ImageWidgetState::Empty,
            content: ImageContent::None,
            placeholder: None,
            error_image: None,
            scale_mode: ImageScaleMode::Fit,
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Middle,
            auto_play: true,
            async_handle: None,
            background_color: None,
            fixed_size: None,
            loaded: Signal::new(),
            error: Signal::new(),
            state_changed: Signal::new(),
            frame_changed: Signal::new(),
        }
    }

    /// Create an ImageWidget with a pre-loaded image.
    pub fn from_image(image: Image) -> Self {
        let mut widget = Self::new();
        widget.set_image(image);
        widget
    }

    /// Create an ImageWidget with an animated image.
    pub fn from_animated(animated: AnimatedImage) -> Self {
        let mut widget = Self::new();
        widget.set_animated_image(animated);
        widget
    }

    // =========================================================================
    // Source Setting Methods
    // =========================================================================

    /// Set the image from a pre-loaded Image.
    ///
    /// This immediately displays the image without any loading state.
    pub fn set_image(&mut self, image: Image) {
        self.source = ImageSource::None;
        self.async_handle = None;
        self.content = ImageContent::Static(image);
        self.set_state(ImageWidgetState::Ready);
        self.base.update();
    }

    /// Set the image from a pre-loaded AnimatedImage.
    ///
    /// If `auto_play` is true (default), the animation will start playing
    /// immediately.
    pub fn set_animated_image(&mut self, animated: AnimatedImage) {
        self.source = ImageSource::None;
        self.async_handle = None;

        let controller = if self.auto_play {
            AnimationController::new(&animated)
        } else {
            AnimationController::new_paused(&animated)
        };

        let frame_count = animated.frame_count();
        self.content = ImageContent::Animated {
            image: animated,
            controller,
            gpu_frames: vec![None; frame_count],
        };

        self.set_state(ImageWidgetState::Ready);
        self.base.update();
    }

    /// Set the image source to a file path.
    ///
    /// The image will be loaded asynchronously. Use the `loaded` signal
    /// to be notified when loading completes.
    pub fn set_source_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.source = ImageSource::File(path);
        self.start_loading();
    }

    /// Set the image source to bytes in memory.
    ///
    /// The image will be decoded asynchronously.
    pub fn set_source_bytes(&mut self, bytes: impl Into<Vec<u8>>) {
        self.source = ImageSource::Bytes(bytes.into());
        self.start_loading();
    }

    /// Set the image source to a URL.
    ///
    /// The image will be downloaded and decoded asynchronously.
    #[cfg(feature = "networking")]
    pub fn set_source_url(&mut self, url: impl Into<String>) {
        self.source = ImageSource::Url(url.into());
        self.start_loading();
    }

    /// Clear the current image.
    pub fn clear(&mut self) {
        self.source = ImageSource::None;
        self.async_handle = None;
        self.content = ImageContent::None;
        self.set_state(ImageWidgetState::Empty);
        self.base.update();
    }

    /// Start loading from the current source.
    fn start_loading(&mut self) {
        self.content = ImageContent::None;
        self.async_handle = None;
        self.set_state(ImageWidgetState::Loading);
        self.base.update();
    }

    /// Set the widget state and emit signal.
    fn set_state(&mut self, new_state: ImageWidgetState) {
        if self.state != new_state {
            self.state = new_state.clone();
            self.state_changed.emit(new_state);
        }
    }

    // =========================================================================
    // Display Options
    // =========================================================================

    /// Get the current scale mode.
    pub fn scale_mode(&self) -> ImageScaleMode {
        self.scale_mode
    }

    /// Set the scale mode.
    pub fn set_scale_mode(&mut self, mode: ImageScaleMode) {
        if self.scale_mode != mode {
            self.scale_mode = mode;
            self.base.update();
        }
    }

    /// Set the scale mode (builder pattern).
    pub fn with_scale_mode(mut self, mode: ImageScaleMode) -> Self {
        self.scale_mode = mode;
        self
    }

    /// Get the horizontal alignment.
    pub fn horizontal_align(&self) -> HorizontalAlign {
        self.horizontal_align
    }

    /// Set the horizontal alignment (for Fit mode).
    pub fn set_horizontal_align(&mut self, align: HorizontalAlign) {
        if self.horizontal_align != align {
            self.horizontal_align = align;
            self.base.update();
        }
    }

    /// Set the horizontal alignment (builder pattern).
    pub fn with_horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    /// Get the vertical alignment.
    pub fn vertical_align(&self) -> VerticalAlign {
        self.vertical_align
    }

    /// Set the vertical alignment (for Fit mode).
    pub fn set_vertical_align(&mut self, align: VerticalAlign) {
        if self.vertical_align != align {
            self.vertical_align = align;
            self.base.update();
        }
    }

    /// Set the vertical alignment (builder pattern).
    pub fn with_vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Get the background color.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        self.background_color = color;
        self.base.update();
    }

    /// Set the background color (builder pattern).
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Get the fixed size.
    pub fn fixed_size(&self) -> Option<Size> {
        self.fixed_size
    }

    /// Set a fixed size for the widget.
    ///
    /// When set, the widget will always report this size regardless of
    /// the actual image dimensions.
    pub fn set_fixed_size(&mut self, size: Option<Size>) {
        self.fixed_size = size;
        self.base.update();
    }

    /// Set a fixed size (builder pattern).
    pub fn with_fixed_size(mut self, width: f32, height: f32) -> Self {
        self.fixed_size = Some(Size::new(width, height));
        self
    }

    // =========================================================================
    // Placeholder and Error Images
    // =========================================================================

    /// Set the placeholder image shown during loading.
    pub fn set_placeholder(&mut self, image: Option<Image>) {
        self.placeholder = image;
        if self.state == ImageWidgetState::Loading {
            self.base.update();
        }
    }

    /// Set the placeholder image (builder pattern).
    pub fn with_placeholder(mut self, image: Image) -> Self {
        self.placeholder = Some(image);
        self
    }

    /// Set the error image shown when loading fails.
    pub fn set_error_image(&mut self, image: Option<Image>) {
        self.error_image = image;
        if matches!(self.state, ImageWidgetState::Error(_)) {
            self.base.update();
        }
    }

    /// Set the error image (builder pattern).
    pub fn with_error_image(mut self, image: Image) -> Self {
        self.error_image = Some(image);
        self
    }

    // =========================================================================
    // Animation Control
    // =========================================================================

    /// Get whether auto-play is enabled for animations.
    pub fn auto_play(&self) -> bool {
        self.auto_play
    }

    /// Set whether to automatically play animations when loaded.
    pub fn set_auto_play(&mut self, auto_play: bool) {
        self.auto_play = auto_play;
    }

    /// Set auto-play (builder pattern).
    pub fn with_auto_play(mut self, auto_play: bool) -> Self {
        self.auto_play = auto_play;
        self
    }

    /// Check if the current image is animated.
    pub fn is_animated(&self) -> bool {
        matches!(self.content, ImageContent::Animated { .. })
    }

    /// Check if animation is currently playing.
    pub fn is_playing(&self) -> bool {
        if let ImageContent::Animated { controller, .. } = &self.content {
            controller.is_playing()
        } else {
            false
        }
    }

    /// Start animation playback.
    pub fn play(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.play();
            self.base.update();
        }
    }

    /// Pause animation playback.
    pub fn pause(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.pause();
        }
    }

    /// Stop animation and reset to first frame.
    pub fn stop(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.stop();
            controller.reset();
            self.base.update();
        }
    }

    /// Toggle animation playback.
    pub fn toggle_playback(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.toggle();
            self.base.update();
        }
    }

    /// Get the animation playback speed.
    pub fn speed(&self) -> f64 {
        if let ImageContent::Animated { controller, .. } = &self.content {
            controller.speed()
        } else {
            1.0
        }
    }

    /// Set the animation playback speed.
    ///
    /// Values > 1.0 speed up, values < 1.0 slow down.
    pub fn set_speed(&mut self, speed: f64) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.set_speed(speed);
        }
    }

    /// Get the current animation frame index.
    pub fn current_frame(&self) -> usize {
        if let ImageContent::Animated { controller, .. } = &self.content {
            controller.current_frame()
        } else {
            0
        }
    }

    /// Get the total number of frames in the animation.
    pub fn frame_count(&self) -> usize {
        if let ImageContent::Animated { image, .. } = &self.content {
            image.frame_count()
        } else {
            1
        }
    }

    /// Jump to a specific frame.
    pub fn goto_frame(&mut self, frame: usize) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            controller.goto_frame(frame);
            self.base.update();
        }
    }

    /// Advance to the next frame (for manual stepping).
    pub fn next_frame(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            let old_frame = controller.current_frame();
            controller.next_frame();
            if controller.current_frame() != old_frame {
                self.frame_changed.emit(controller.current_frame());
            }
            self.base.update();
        }
    }

    /// Go back to the previous frame.
    pub fn prev_frame(&mut self) {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            let old_frame = controller.current_frame();
            controller.prev_frame();
            if controller.current_frame() != old_frame {
                self.frame_changed.emit(controller.current_frame());
            }
            self.base.update();
        }
    }

    // =========================================================================
    // State Query Methods
    // =========================================================================

    /// Get the current widget state.
    pub fn state(&self) -> &ImageWidgetState {
        &self.state
    }

    /// Check if the widget is empty (no image set).
    pub fn is_empty(&self) -> bool {
        matches!(self.state, ImageWidgetState::Empty)
    }

    /// Check if the widget is loading.
    pub fn is_loading(&self) -> bool {
        matches!(self.state, ImageWidgetState::Loading)
    }

    /// Check if the widget has an image ready.
    pub fn is_ready(&self) -> bool {
        matches!(self.state, ImageWidgetState::Ready)
    }

    /// Check if loading failed.
    pub fn has_error(&self) -> bool {
        matches!(self.state, ImageWidgetState::Error(_))
    }

    /// Get the error message if loading failed.
    pub fn error_message(&self) -> Option<&str> {
        if let ImageWidgetState::Error(msg) = &self.state {
            Some(msg)
        } else {
            None
        }
    }

    /// Get the natural size of the current image.
    pub fn image_size(&self) -> Option<Size> {
        match &self.content {
            ImageContent::None => None,
            ImageContent::Static(img) => Some(img.size()),
            ImageContent::Animated { image, .. } => {
                Some(Size::new(image.width() as f32, image.height() as f32))
            }
        }
    }

    // =========================================================================
    // Update Method (called from event loop)
    // =========================================================================

    /// Update animation state.
    ///
    /// Call this method each frame with the elapsed time to advance animations.
    /// Returns `true` if the display needs to be repainted.
    pub fn update_animation(&mut self, delta: Duration) -> bool {
        if let ImageContent::Animated { controller, .. } = &mut self.content {
            let old_frame = controller.current_frame();
            let changed = controller.update(delta);
            if changed {
                let new_frame = controller.current_frame();
                if new_frame != old_frame {
                    self.frame_changed.emit(new_frame);
                }
                self.base.update();
            }
            changed
        } else {
            false
        }
    }

    /// Process async loading results.
    ///
    /// Call this method each frame to check for completed async loads.
    /// Pass the AsyncImageLoader and ImageManager from your application.
    ///
    /// Returns `true` if the image state changed.
    pub fn process_async_load(
        &mut self,
        async_loader: &AsyncImageLoader,
        _image_manager: &mut ImageManager,
    ) -> bool {
        if let Some(handle) = &self.async_handle
            && let Some(state) = async_loader.state(handle) {
                match state {
                    LoadingState::Loading => return false,
                    LoadingState::Ready(image) => {
                        self.content = ImageContent::Static(image.clone());
                        self.async_handle = None;
                        self.set_state(ImageWidgetState::Ready);
                        self.loaded.emit(());
                        self.base.update();
                        return true;
                    }
                    LoadingState::Failed(err) => {
                        self.async_handle = None;
                        self.set_state(ImageWidgetState::Error(err.clone()));
                        self.error.emit(err.clone());
                        self.base.update();
                        return true;
                    }
                }
            }
        false
    }

    /// Load from current source synchronously (blocking).
    ///
    /// For most applications, prefer async loading with `set_source_*` methods.
    /// This method is useful for small images or during initialization.
    pub fn load_sync(&mut self, image_manager: &mut ImageManager) -> Result<(), String> {
        match &self.source {
            ImageSource::None => {
                self.clear();
                Ok(())
            }
            ImageSource::File(path) => {
                // Check if it might be animated (GIF)
                let is_gif = path
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("gif"))
                    .unwrap_or(false);

                if is_gif {
                    match AnimatedImage::from_file(path) {
                        Ok(animated) => {
                            if animated.frame_count() > 1 {
                                self.set_animated_image(animated);
                            } else {
                                // Single-frame GIF, treat as static
                                match image_manager.load_file(path) {
                                    Ok(img) => self.set_image(img),
                                    Err(e) => {
                                        let msg = e.to_string();
                                        self.set_state(ImageWidgetState::Error(msg.clone()));
                                        self.error.emit(msg.clone());
                                        return Err(msg);
                                    }
                                }
                            }
                            Ok(())
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            self.set_state(ImageWidgetState::Error(msg.clone()));
                            self.error.emit(msg.clone());
                            Err(msg)
                        }
                    }
                } else {
                    match image_manager.load_file(path) {
                        Ok(img) => {
                            self.set_image(img);
                            Ok(())
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            self.set_state(ImageWidgetState::Error(msg.clone()));
                            self.error.emit(msg.clone());
                            Err(msg)
                        }
                    }
                }
            }
            ImageSource::Bytes(bytes) => match image_manager.load_bytes(bytes) {
                Ok(img) => {
                    self.set_image(img);
                    Ok(())
                }
                Err(e) => {
                    let msg = e.to_string();
                    self.set_state(ImageWidgetState::Error(msg.clone()));
                    self.error.emit(msg.clone());
                    Err(msg)
                }
            },
            #[cfg(feature = "networking")]
            ImageSource::Url(_) => {
                // URL loading requires async
                Err("URL loading requires async. Use set_source_url() instead.".to_string())
            }
        }
    }

    // =========================================================================
    // Internal Rendering Helpers
    // =========================================================================

    /// Calculate the destination rectangle for the image based on scale mode.
    fn calculate_dest_rect(&self, widget_rect: Rect, image_size: Size) -> Rect {
        match self.scale_mode {
            ImageScaleMode::Stretch => widget_rect,
            ImageScaleMode::Fit => {
                let widget_aspect = widget_rect.width() / widget_rect.height();
                let image_aspect = image_size.width / image_size.height;

                let (scaled_width, scaled_height) = if image_aspect > widget_aspect {
                    // Image is wider - fit to width
                    let scaled_width = widget_rect.width();
                    let scaled_height = scaled_width / image_aspect;
                    (scaled_width, scaled_height)
                } else {
                    // Image is taller - fit to height
                    let scaled_height = widget_rect.height();
                    let scaled_width = scaled_height * image_aspect;
                    (scaled_width, scaled_height)
                };

                let x = match self.horizontal_align {
                    HorizontalAlign::Left => widget_rect.left(),
                    HorizontalAlign::Center => {
                        widget_rect.left() + (widget_rect.width() - scaled_width) / 2.0
                    }
                    HorizontalAlign::Right => widget_rect.right() - scaled_width,
                    HorizontalAlign::Justified => widget_rect.left(),
                };

                let y = match self.vertical_align {
                    VerticalAlign::Top => widget_rect.top(),
                    VerticalAlign::Middle => {
                        widget_rect.top() + (widget_rect.height() - scaled_height) / 2.0
                    }
                    VerticalAlign::Bottom => widget_rect.bottom() - scaled_height,
                };

                Rect::new(x, y, scaled_width, scaled_height)
            }
            ImageScaleMode::Fill => {
                let widget_aspect = widget_rect.width() / widget_rect.height();
                let image_aspect = image_size.width / image_size.height;

                let (scaled_width, scaled_height) = if image_aspect < widget_aspect {
                    // Image is narrower - fill to width
                    let scaled_width = widget_rect.width();
                    let scaled_height = scaled_width / image_aspect;
                    (scaled_width, scaled_height)
                } else {
                    // Image is shorter - fill to height
                    let scaled_height = widget_rect.height();
                    let scaled_width = scaled_height * image_aspect;
                    (scaled_width, scaled_height)
                };

                let x = widget_rect.left() + (widget_rect.width() - scaled_width) / 2.0;
                let y = widget_rect.top() + (widget_rect.height() - scaled_height) / 2.0;

                Rect::new(x, y, scaled_width, scaled_height)
            }
            ImageScaleMode::Tile => {
                // For tile mode, we draw at original size starting from top-left
                // The actual tiling is handled in the draw call
                Rect::new(
                    widget_rect.left(),
                    widget_rect.top(),
                    image_size.width,
                    image_size.height,
                )
            }
        }
    }

    /// Get the image to display for the current frame (static or animated).
    fn get_display_image(&self) -> Option<&Image> {
        match &self.content {
            ImageContent::None => None,
            ImageContent::Static(img) => Some(img),
            ImageContent::Animated {
                gpu_frames,
                controller,
                ..
            } => {
                let frame_idx = controller.current_frame();
                gpu_frames.get(frame_idx).and_then(|opt| opt.as_ref())
            }
        }
    }
}

impl Default for ImageWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ImageWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ImageWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // If fixed size is set, use that (creates a fixed size hint)
        if let Some(fixed) = self.fixed_size {
            return SizeHint::fixed(fixed);
        }

        // Otherwise, use image size
        let size = match &self.content {
            ImageContent::None => {
                // Use placeholder size if available
                if let Some(placeholder) = &self.placeholder {
                    placeholder.size()
                } else {
                    // Default minimum size
                    Size::new(100.0, 100.0)
                }
            }
            ImageContent::Static(img) => img.size(),
            ImageContent::Animated { image, .. } => {
                Size::new(image.width() as f32, image.height() as f32)
            }
        };

        SizeHint::new(size).with_minimum_dimensions(16.0, 16.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if let Some(bg_color) = self.background_color {
            ctx.renderer().fill_rect(rect, bg_color);
        }

        // Determine which image to draw
        let (image, image_size) = match &self.state {
            ImageWidgetState::Empty => {
                // Nothing to draw
                return;
            }
            ImageWidgetState::Loading => {
                // Draw placeholder if available
                if let Some(placeholder) = &self.placeholder {
                    (placeholder, placeholder.size())
                } else {
                    // Could draw a loading indicator here
                    return;
                }
            }
            ImageWidgetState::Ready => {
                if let Some(img) = self.get_display_image() {
                    (img, img.size())
                } else {
                    return;
                }
            }
            ImageWidgetState::Error(_) => {
                // Draw error image if available
                if let Some(error_img) = &self.error_image {
                    (error_img, error_img.size())
                } else {
                    // Could draw an error indicator here
                    return;
                }
            }
        };

        // Calculate destination rectangle
        let dest_rect = self.calculate_dest_rect(rect, image_size);

        // Draw the image
        match self.scale_mode {
            ImageScaleMode::Tile => {
                // For tiling, we need to clip and draw multiple times
                ctx.renderer().save();
                ctx.renderer().clip_rect(rect);

                let mut y = rect.top();
                while y < rect.bottom() {
                    let mut x = rect.left();
                    while x < rect.right() {
                        let tile_rect = Rect::new(x, y, image_size.width, image_size.height);
                        ctx.renderer()
                            .draw_image(image, tile_rect, ImageScaleMode::Stretch);
                        x += image_size.width;
                    }
                    y += image_size.height;
                }

                ctx.renderer().restore();
            }
            ImageScaleMode::Fill => {
                // For fill mode, clip to widget bounds to hide overflow
                ctx.renderer().save();
                ctx.renderer().clip_rect(rect);
                ctx.renderer()
                    .draw_image(image, dest_rect, ImageScaleMode::Stretch);
                ctx.renderer().restore();
            }
            _ => {
                ctx.renderer()
                    .draw_image(image, dest_rect, ImageScaleMode::Stretch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn init_test() {
        init_global_registry();
    }

    #[test]
    fn test_image_widget_default_state() {
        init_test();
        let widget = ImageWidget::new();
        assert!(widget.is_empty());
        assert!(!widget.is_loading());
        assert!(!widget.is_ready());
        assert!(!widget.has_error());
        assert!(widget.auto_play());
    }

    #[test]
    fn test_image_widget_scale_mode() {
        init_test();
        let mut widget = ImageWidget::new();
        assert_eq!(widget.scale_mode(), ImageScaleMode::Fit);

        widget.set_scale_mode(ImageScaleMode::Fill);
        assert_eq!(widget.scale_mode(), ImageScaleMode::Fill);

        let widget2 = ImageWidget::new().with_scale_mode(ImageScaleMode::Stretch);
        assert_eq!(widget2.scale_mode(), ImageScaleMode::Stretch);
    }

    #[test]
    fn test_image_widget_alignment() {
        init_test();
        let widget = ImageWidget::new()
            .with_horizontal_align(HorizontalAlign::Right)
            .with_vertical_align(VerticalAlign::Bottom);

        assert_eq!(widget.horizontal_align(), HorizontalAlign::Right);
        assert_eq!(widget.vertical_align(), VerticalAlign::Bottom);
    }

    #[test]
    fn test_image_widget_fixed_size() {
        init_test();
        let widget = ImageWidget::new().with_fixed_size(200.0, 150.0);
        assert_eq!(widget.fixed_size(), Some(Size::new(200.0, 150.0)));
    }

    #[test]
    fn test_image_widget_clear() {
        init_test();
        let mut widget = ImageWidget::new();
        widget.source = ImageSource::File(PathBuf::from("test.png"));
        widget.state = ImageWidgetState::Ready;

        widget.clear();
        assert!(widget.is_empty());
        assert!(matches!(widget.source, ImageSource::None));
    }

    #[test]
    fn test_dest_rect_calculation_fit() {
        init_test();
        let widget = ImageWidget::new().with_scale_mode(ImageScaleMode::Fit);

        // Image wider than widget
        let widget_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let image_size = Size::new(200.0, 100.0);
        let dest = widget.calculate_dest_rect(widget_rect, image_size);

        // Should fit to width (100), height should be 50, centered vertically
        assert_eq!(dest.width(), 100.0);
        assert_eq!(dest.height(), 50.0);
        assert_eq!(dest.top(), 25.0); // Centered

        // Image taller than widget
        let image_size = Size::new(50.0, 200.0);
        let dest = widget.calculate_dest_rect(widget_rect, image_size);

        // Should fit to height (100), width should be 25, centered horizontally
        assert_eq!(dest.height(), 100.0);
        assert_eq!(dest.width(), 25.0);
        assert_eq!(dest.left(), 37.5); // Centered
    }

    #[test]
    fn test_dest_rect_calculation_stretch() {
        init_test();
        let widget = ImageWidget::new().with_scale_mode(ImageScaleMode::Stretch);

        let widget_rect = Rect::new(10.0, 20.0, 100.0, 80.0);
        let image_size = Size::new(50.0, 50.0);
        let dest = widget.calculate_dest_rect(widget_rect, image_size);

        // Should fill entire widget rect
        assert_eq!(dest, widget_rect);
    }
}
