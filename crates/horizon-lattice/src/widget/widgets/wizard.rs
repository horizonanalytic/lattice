//! Wizard dialog implementation.
//!
//! This module provides [`Wizard`], a multi-page dialog for guiding users through
//! a step-by-step process with support for page validation and dynamic page ordering.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Wizard, WizardPage, WizardStyle};
//!
//! // Create a wizard with pages
//! let mut wizard = Wizard::new("Setup Wizard")
//!     .with_style(WizardStyle::Modern)
//!     .with_size(600.0, 450.0);
//!
//! // Add pages
//! wizard.add_page(WizardPage::new("Welcome")
//!     .with_subtitle("Introduction to the setup process")
//!     .with_content(welcome_widget_id));
//!
//! wizard.add_page(WizardPage::new("Configuration")
//!     .with_subtitle("Configure your settings")
//!     .with_content(config_widget_id)
//!     .with_validator(|_| {
//!         // Validation logic
//!         ValidationResult::valid()
//!     }));
//!
//! // Connect to signals
//! wizard.finished.connect(|&accepted| {
//!     if accepted {
//!         println!("Wizard completed!");
//!     }
//! });
//!
//! wizard.open();
//! ```

use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, RoundedRect, Stroke, TextLayout,
    TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

use super::dialog::DialogResult;

// ============================================================================
// Constants
// ============================================================================

/// Default wizard width.
const DEFAULT_WIDTH: f32 = 600.0;

/// Default wizard height.
const DEFAULT_HEIGHT: f32 = 450.0;

/// Sidebar width for modern style.
const SIDEBAR_WIDTH: f32 = 200.0;

/// Step indicator height.
const STEP_HEIGHT: f32 = 48.0;

/// Navigation button area height.
const NAV_BUTTON_HEIGHT: f32 = 56.0;

/// Button width.
const BUTTON_WIDTH: f32 = 90.0;

/// Button height.
const BUTTON_HEIGHT: f32 = 32.0;

/// Padding around content.
const CONTENT_PADDING: f32 = 24.0;

/// Step icon size.
const STEP_ICON_SIZE: f32 = 28.0;

// ============================================================================
// WizardStyle
// ============================================================================

/// The visual style of a wizard dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStyle {
    /// Classic wizard with navigation buttons only (Back, Next, Finish, Cancel).
    #[default]
    Classic,
    /// Modern wizard with a sidebar showing all steps and their status.
    Modern,
}

// ============================================================================
// ValidationResult
// ============================================================================

/// The result of validating a wizard page.
///
/// Provides rich validation feedback including error messages and field identifiers.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed.
    valid: bool,
    /// Error messages for validation failures.
    errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with a single error message.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![ValidationError::new(message)],
        }
    }

    /// Create a failed validation result with a field-specific error.
    pub fn field_error(field_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![ValidationError::with_field(field_id, message)],
        }
    }

    /// Create a validation result with multiple errors.
    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        let valid = errors.is_empty();
        Self { valid, errors }
    }

    /// Check if the validation passed.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get the validation errors.
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Get the first error message, if any.
    pub fn first_error_message(&self) -> Option<&str> {
        self.errors.first().map(|e| e.message.as_str())
    }

    /// Add an error to the result.
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.valid = false;
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        if !self.errors.is_empty() {
            self.valid = false;
        }
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::valid()
    }
}

// ============================================================================
// ValidationError
// ============================================================================

/// A validation error with message and optional field identifier.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The error message.
    pub message: String,
    /// Optional field identifier for highlighting.
    pub field_id: Option<String>,
}

impl ValidationError {
    /// Create a validation error with just a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field_id: None,
        }
    }

    /// Create a validation error with a field identifier.
    pub fn with_field(field_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field_id: Some(field_id.into()),
        }
    }
}

// ============================================================================
// PageValidator
// ============================================================================

/// A function that validates a wizard page.
pub type PageValidator = Arc<dyn Fn(&WizardPage) -> ValidationResult + Send + Sync>;

// ============================================================================
// PageCondition
// ============================================================================

/// A function that determines if a page should be shown based on wizard state.
pub type PageCondition = Arc<dyn Fn(&Wizard) -> bool + Send + Sync>;

// ============================================================================
// WizardPage
// ============================================================================

/// A page within a wizard dialog.
///
/// Each page represents a step in the wizard process with its own title,
/// content, and optional validation.
#[derive(Clone)]
pub struct WizardPage {
    /// Unique identifier for the page.
    id: String,
    /// The page title.
    title: String,
    /// Optional subtitle or description.
    subtitle: Option<String>,
    /// The content widget ID.
    content_widget: Option<ObjectId>,
    /// Whether this page is a commit point (can't go back after).
    is_commit_point: bool,
    /// Whether this page has been completed.
    completed: bool,
    /// The validator function for this page.
    validator: Option<PageValidator>,
    /// Condition for whether this page should be shown.
    condition: Option<PageCondition>,
    /// User data associated with this page.
    user_data: Option<String>,
    /// Current validation result (cached).
    last_validation: ValidationResult,
}

impl WizardPage {
    /// Create a new wizard page with a title.
    pub fn new(title: impl Into<String>) -> Self {
        let title_str = title.into();
        Self {
            id: title_str.to_lowercase().replace(' ', "_"),
            title: title_str,
            subtitle: None,
            content_widget: None,
            is_commit_point: false,
            completed: false,
            validator: None,
            condition: None,
            user_data: None,
            last_validation: ValidationResult::valid(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the page ID using builder pattern.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the subtitle using builder pattern.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the content widget using builder pattern.
    pub fn with_content(mut self, widget_id: ObjectId) -> Self {
        self.content_widget = Some(widget_id);
        self
    }

    /// Set this page as a commit point using builder pattern.
    ///
    /// A commit point is a page that, once passed, cannot be navigated back to.
    pub fn as_commit_point(mut self) -> Self {
        self.is_commit_point = true;
        self
    }

    /// Set the validator function using builder pattern.
    pub fn with_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&WizardPage) -> ValidationResult + Send + Sync + 'static,
    {
        self.validator = Some(Arc::new(validator));
        self
    }

    /// Set the condition for showing this page using builder pattern.
    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&Wizard) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    /// Set user data using builder pattern.
    pub fn with_user_data(mut self, data: impl Into<String>) -> Self {
        self.user_data = Some(data.into());
        self
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the page ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the page title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the page title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Get the page subtitle.
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// Set the page subtitle.
    pub fn set_subtitle(&mut self, subtitle: Option<String>) {
        self.subtitle = subtitle;
    }

    /// Get the content widget ID.
    pub fn content_widget(&self) -> Option<ObjectId> {
        self.content_widget
    }

    /// Set the content widget.
    pub fn set_content_widget(&mut self, widget_id: Option<ObjectId>) {
        self.content_widget = widget_id;
    }

    /// Check if this page is a commit point.
    pub fn is_commit_point(&self) -> bool {
        self.is_commit_point
    }

    /// Set whether this page is a commit point.
    pub fn set_commit_point(&mut self, is_commit: bool) {
        self.is_commit_point = is_commit;
    }

    /// Check if this page has been completed.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Set whether this page is completed.
    pub fn set_completed(&mut self, completed: bool) {
        self.completed = completed;
    }

    /// Get the user data.
    pub fn user_data(&self) -> Option<&str> {
        self.user_data.as_deref()
    }

    /// Set the user data.
    pub fn set_user_data(&mut self, data: Option<String>) {
        self.user_data = data;
    }

    /// Get the last validation result.
    pub fn last_validation(&self) -> &ValidationResult {
        &self.last_validation
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate this page.
    ///
    /// Returns a ValidationResult indicating whether the page is valid.
    pub fn validate(&mut self) -> ValidationResult {
        let result = if let Some(ref validator) = self.validator {
            validator(self)
        } else {
            ValidationResult::valid()
        };
        self.last_validation = result.clone();
        result
    }

    /// Check if this page should be shown given the wizard state.
    pub fn should_show(&self, wizard: &Wizard) -> bool {
        if let Some(ref condition) = self.condition {
            condition(wizard)
        } else {
            true
        }
    }
}

impl std::fmt::Debug for WizardPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WizardPage")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("content_widget", &self.content_widget)
            .field("is_commit_point", &self.is_commit_point)
            .field("completed", &self.completed)
            .field("has_validator", &self.validator.is_some())
            .field("has_condition", &self.condition.is_some())
            .finish()
    }
}

// ============================================================================
// WizardButton
// ============================================================================

/// A button in the wizard navigation area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardButton {
    /// The Back button.
    Back,
    /// The Next button.
    Next,
    /// The Finish button.
    Finish,
    /// The Cancel button.
    Cancel,
    /// A custom button.
    Custom(u32),
}

// ============================================================================
// HitPart
// ============================================================================

/// Identifies which part of the wizard is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum HitPart {
    #[default]
    None,
    /// Back button.
    BackButton,
    /// Next button.
    NextButton,
    /// Finish button.
    FinishButton,
    /// Cancel button.
    CancelButton,
    /// A step in the sidebar (modern style).
    SidebarStep(usize),
    /// Title bar close button.
    CloseButton,
    /// Title bar drag area.
    TitleBar,
}

// ============================================================================
// Wizard
// ============================================================================

/// A multi-page wizard dialog.
///
/// Wizard provides a step-by-step user interface for complex tasks.
/// It supports:
///
/// - Multiple pages with navigation (Back, Next, Finish, Cancel)
/// - Two styles: Classic (buttons only) and Modern (sidebar)
/// - Page validation with rich error feedback
/// - Commit points that prevent backward navigation
/// - Conditional page visibility for dynamic flows
///
/// # Signals
///
/// - `current_page_changed(i32)`: Emitted when the current page changes
/// - `page_added(i32)`: Emitted when a page is added
/// - `page_removed(i32)`: Emitted when a page is removed
/// - `finished(bool)`: Emitted when the wizard is finished (true if accepted)
/// - `accepted()`: Emitted when the wizard is accepted
/// - `rejected()`: Emitted when the wizard is rejected
/// - `help_requested()`: Emitted when help is requested
/// - `validation_failed(ValidationResult)`: Emitted when validation fails
pub struct Wizard {
    /// Widget base.
    base: WidgetBase,

    /// The wizard title.
    title: String,

    /// Current visual style.
    style: WizardStyle,

    /// All registered pages.
    pages: Vec<WizardPage>,

    /// Current page index in the visible pages list.
    current_page: i32,

    /// History of visited pages for back navigation.
    page_history: Vec<usize>,

    /// Whether the wizard is currently open.
    is_open: bool,

    /// The dialog result.
    result: DialogResult,

    /// Index of the first commit point reached.
    first_commit_reached: Option<usize>,

    // Geometry
    /// Total size of the wizard.
    size: (f32, f32),

    /// Title bar height.
    title_bar_height: f32,

    // Interaction state
    /// Currently hovered part.
    hover_part: HitPart,
    /// Currently pressed part.
    pressed_part: HitPart,
    /// Whether dragging the title bar.
    dragging: bool,
    /// Drag start position.
    drag_start: Point,
    /// Drag start geometry.
    drag_start_geometry: Rect,

    // Visual styling
    /// Background color.
    background_color: Color,
    /// Sidebar background (modern style).
    sidebar_color: Color,
    /// Title bar color.
    title_bar_color: Color,
    /// Title bar active color.
    title_bar_active_color: Color,
    /// Border color.
    border_color: Color,
    /// Button background color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,
    /// Button pressed color.
    button_pressed_color: Color,
    /// Button disabled color.
    button_disabled_color: Color,
    /// Primary button color.
    primary_button_color: Color,
    /// Primary button hover color.
    primary_button_hover_color: Color,
    /// Text color.
    text_color: Color,
    /// Secondary text color.
    secondary_text_color: Color,
    /// Active step color (modern style).
    active_step_color: Color,
    /// Completed step color.
    completed_step_color: Color,
    /// Error color for validation.
    error_color: Color,
    /// Separator color.
    separator_color: Color,

    // Close button state
    close_button_hovered: bool,
    close_button_pressed: bool,
    close_button_hover_color: Color,

    // Whether the wizard is active.
    active: bool,

    // Signals
    /// Signal emitted when the current page changes.
    pub current_page_changed: Signal<i32>,
    /// Signal emitted when a page is added.
    pub page_added: Signal<i32>,
    /// Signal emitted when a page is removed.
    pub page_removed: Signal<i32>,
    /// Signal emitted when the wizard is finished.
    pub finished: Signal<bool>,
    /// Signal emitted when the wizard is accepted.
    pub accepted: Signal<()>,
    /// Signal emitted when the wizard is rejected.
    pub rejected: Signal<()>,
    /// Signal emitted when help is requested.
    pub help_requested: Signal<()>,
    /// Signal emitted when validation fails.
    pub validation_failed: Signal<ValidationResult>,
    /// Signal emitted when about to show.
    pub about_to_show: Signal<()>,
    /// Signal emitted when about to hide.
    pub about_to_hide: Signal<()>,
}

impl Wizard {
    /// Create a new wizard with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));
        base.hide();

        Self {
            base,
            title: title.into(),
            style: WizardStyle::Classic,
            pages: Vec::new(),
            current_page: -1,
            page_history: Vec::new(),
            is_open: false,
            result: DialogResult::Rejected,
            first_commit_reached: None,
            size: (DEFAULT_WIDTH, DEFAULT_HEIGHT),
            title_bar_height: 28.0,
            hover_part: HitPart::None,
            pressed_part: HitPart::None,
            dragging: false,
            drag_start: Point::ZERO,
            drag_start_geometry: Rect::ZERO,
            background_color: Color::WHITE,
            sidebar_color: Color::from_rgb8(245, 247, 250),
            title_bar_color: Color::from_rgb8(240, 240, 240),
            title_bar_active_color: Color::from_rgb8(200, 220, 240),
            border_color: Color::from_rgb8(180, 180, 180),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            button_disabled_color: Color::from_rgb8(200, 200, 200),
            primary_button_color: Color::from_rgb8(51, 122, 183),
            primary_button_hover_color: Color::from_rgb8(40, 96, 144),
            text_color: Color::from_rgb8(40, 40, 40),
            secondary_text_color: Color::from_rgb8(120, 120, 120),
            active_step_color: Color::from_rgb8(51, 122, 183),
            completed_step_color: Color::from_rgb8(92, 184, 92),
            error_color: Color::from_rgb8(217, 83, 79),
            separator_color: Color::from_rgb8(220, 220, 220),
            close_button_hovered: false,
            close_button_pressed: false,
            close_button_hover_color: Color::from_rgb8(232, 17, 35),
            active: false,
            current_page_changed: Signal::new(),
            page_added: Signal::new(),
            page_removed: Signal::new(),
            finished: Signal::new(),
            accepted: Signal::new(),
            rejected: Signal::new(),
            help_requested: Signal::new(),
            validation_failed: Signal::new(),
            about_to_show: Signal::new(),
            about_to_hide: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the wizard size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = (width, height);
        self.base
            .set_size(horizon_lattice_render::Size::new(width, height));
        self
    }

    /// Set the wizard style using builder pattern.
    pub fn with_style(mut self, style: WizardStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the wizard title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the wizard title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.base.update();
    }

    // =========================================================================
    // Style
    // =========================================================================

    /// Get the wizard style.
    pub fn style(&self) -> WizardStyle {
        self.style
    }

    /// Set the wizard style.
    pub fn set_style(&mut self, style: WizardStyle) {
        self.style = style;
        self.base.update();
    }

    // =========================================================================
    // Page Management
    // =========================================================================

    /// Add a page to the wizard.
    ///
    /// Returns the index of the new page.
    pub fn add_page(&mut self, page: WizardPage) -> i32 {
        let index = self.pages.len() as i32;
        self.pages.push(page);

        if self.current_page < 0 && !self.pages.is_empty() {
            self.current_page = 0;
        }

        self.page_added.emit(index);
        self.base.update();
        index
    }

    /// Insert a page at the specified index.
    pub fn insert_page(&mut self, index: i32, page: WizardPage) -> i32 {
        let insert_pos = if index < 0 {
            0
        } else {
            (index as usize).min(self.pages.len())
        };

        self.pages.insert(insert_pos, page);

        if self.current_page < 0 && !self.pages.is_empty() {
            self.current_page = 0;
        } else if self.current_page >= insert_pos as i32 {
            self.current_page += 1;
        }

        self.page_added.emit(insert_pos as i32);
        self.base.update();
        insert_pos as i32
    }

    /// Remove a page at the specified index.
    ///
    /// Returns the removed page, if any.
    pub fn remove_page(&mut self, index: i32) -> Option<WizardPage> {
        if index < 0 || index as usize >= self.pages.len() {
            return None;
        }

        let page = self.pages.remove(index as usize);

        // Update current page index
        if self.pages.is_empty() {
            self.current_page = -1;
        } else if self.current_page >= self.pages.len() as i32 {
            self.current_page = self.pages.len() as i32 - 1;
        }

        self.page_removed.emit(index);
        self.base.update();
        Some(page)
    }

    /// Get the number of pages.
    pub fn page_count(&self) -> i32 {
        self.pages.len() as i32
    }

    /// Get a reference to a page by index.
    pub fn page(&self, index: i32) -> Option<&WizardPage> {
        if index < 0 {
            None
        } else {
            self.pages.get(index as usize)
        }
    }

    /// Get a mutable reference to a page by index.
    pub fn page_mut(&mut self, index: i32) -> Option<&mut WizardPage> {
        if index < 0 {
            None
        } else {
            self.pages.get_mut(index as usize)
        }
    }

    /// Find a page by ID.
    pub fn page_by_id(&self, id: &str) -> Option<&WizardPage> {
        self.pages.iter().find(|p| p.id() == id)
    }

    /// Find a page by ID (mutable).
    pub fn page_by_id_mut(&mut self, id: &str) -> Option<&mut WizardPage> {
        self.pages.iter_mut().find(|p| p.id() == id)
    }

    /// Get the index of a page by ID.
    pub fn page_index_by_id(&self, id: &str) -> i32 {
        self.pages
            .iter()
            .position(|p| p.id() == id)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    // =========================================================================
    // Current Page
    // =========================================================================

    /// Get the current page index.
    pub fn current_page_index(&self) -> i32 {
        self.current_page
    }

    /// Get the current page.
    pub fn current_page(&self) -> Option<&WizardPage> {
        self.page(self.current_page)
    }

    /// Get the current page (mutable).
    pub fn current_page_mut(&mut self) -> Option<&mut WizardPage> {
        self.page_mut(self.current_page)
    }

    /// Set the current page by index.
    ///
    /// This bypasses validation. Use `next()` and `back()` for navigation
    /// that respects validation.
    pub fn set_current_page(&mut self, index: i32) -> bool {
        if index < 0 || index as usize >= self.pages.len() {
            return false;
        }

        if index == self.current_page {
            return false;
        }

        self.current_page = index;
        self.current_page_changed.emit(index);
        self.base.update();
        true
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Get the list of visible page indices based on conditions.
    fn visible_pages(&self) -> Vec<usize> {
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, page)| page.should_show(self))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get the index of the next visible page.
    fn next_visible_page(&self) -> Option<usize> {
        let visible = self.visible_pages();
        let current_pos = visible
            .iter()
            .position(|&i| i == self.current_page as usize)?;
        visible.get(current_pos + 1).copied()
    }

    /// Get the index of the previous visible page.
    fn prev_visible_page(&self) -> Option<usize> {
        let visible = self.visible_pages();
        let current_pos = visible
            .iter()
            .position(|&i| i == self.current_page as usize)?;
        if current_pos > 0 {
            visible.get(current_pos - 1).copied()
        } else {
            None
        }
    }

    /// Check if we can navigate to the next page.
    pub fn can_go_next(&self) -> bool {
        self.next_visible_page().is_some()
    }

    /// Check if we can navigate to the previous page.
    pub fn can_go_back(&self) -> bool {
        // Can't go back past a commit point
        if let Some(commit_idx) = self.first_commit_reached
            && self.current_page as usize <= commit_idx {
                return false;
            }
        self.prev_visible_page().is_some()
    }

    /// Check if the current page is the last visible page.
    pub fn is_last_page(&self) -> bool {
        self.next_visible_page().is_none()
    }

    /// Navigate to the next page.
    ///
    /// Validates the current page first. Returns false if validation fails
    /// or if there is no next page.
    pub fn next(&mut self) -> bool {
        // Validate current page
        if let Some(page) = self.current_page_mut() {
            let result = page.validate();
            if !result.is_valid() {
                self.validation_failed.emit(result);
                return false;
            }
            page.set_completed(true);

            // Check for commit point
            if page.is_commit_point() && self.first_commit_reached.is_none() {
                self.first_commit_reached = Some(self.current_page as usize);
            }
        }

        // Navigate to next visible page
        if let Some(next_idx) = self.next_visible_page() {
            self.page_history.push(self.current_page as usize);
            self.current_page = next_idx as i32;
            self.current_page_changed.emit(self.current_page);
            self.base.update();
            true
        } else {
            false
        }
    }

    /// Navigate to the previous page.
    ///
    /// Returns false if there is no previous page or if we're past a commit point.
    pub fn back(&mut self) -> bool {
        if !self.can_go_back() {
            return false;
        }

        // Use history if available
        if let Some(prev_idx) = self.page_history.pop() {
            self.current_page = prev_idx as i32;
            self.current_page_changed.emit(self.current_page);
            self.base.update();
            return true;
        }

        // Otherwise, find previous visible page
        if let Some(prev_idx) = self.prev_visible_page() {
            self.current_page = prev_idx as i32;
            self.current_page_changed.emit(self.current_page);
            self.base.update();
            true
        } else {
            false
        }
    }

    /// Finish the wizard.
    ///
    /// Validates the current page and accepts the wizard if valid.
    pub fn finish(&mut self) -> bool {
        // Validate current page
        if let Some(page) = self.current_page_mut() {
            let result = page.validate();
            if !result.is_valid() {
                self.validation_failed.emit(result);
                return false;
            }
            page.set_completed(true);
        }

        self.accept();
        true
    }

    /// Restart the wizard from the first page.
    pub fn restart(&mut self) {
        self.current_page = if self.pages.is_empty() { -1 } else { 0 };
        self.page_history.clear();
        self.first_commit_reached = None;

        for page in &mut self.pages {
            page.set_completed(false);
        }

        self.current_page_changed.emit(self.current_page);
        self.base.update();
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the wizard.
    pub fn open(&mut self) {
        if self.is_open {
            return;
        }

        self.is_open = true;
        self.result = DialogResult::Rejected;
        self.restart();

        self.about_to_show.emit(());
        self.base.show();
        self.active = true;
        self.base.update();
    }

    /// Check if the wizard is open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Accept the wizard.
    pub fn accept(&mut self) {
        self.done(DialogResult::Accepted);
    }

    /// Reject the wizard.
    pub fn reject(&mut self) {
        self.done(DialogResult::Rejected);
    }

    /// Close the wizard with a result.
    fn done(&mut self, result: DialogResult) {
        if !self.is_open {
            return;
        }

        self.result = result;

        match result {
            DialogResult::Accepted => {
                self.accepted.emit(());
                self.finished.emit(true);
            }
            DialogResult::Rejected => {
                self.rejected.emit(());
                self.finished.emit(false);
            }
        }

        self.about_to_hide.emit(());
        self.is_open = false;
        self.base.hide();
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.result
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    fn dialog_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.size.0, self.size.1)
    }

    fn title_bar_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.size.0, self.title_bar_height)
    }

    fn close_button_rect(&self) -> Rect {
        let button_size = 20.0;
        let padding = (self.title_bar_height - button_size) / 2.0;
        Rect::new(
            self.size.0 - padding - button_size,
            padding,
            button_size,
            button_size,
        )
    }

    fn sidebar_rect(&self) -> Rect {
        if self.style == WizardStyle::Classic {
            return Rect::ZERO;
        }
        Rect::new(
            0.0,
            self.title_bar_height,
            SIDEBAR_WIDTH,
            self.size.1 - self.title_bar_height - NAV_BUTTON_HEIGHT,
        )
    }

    fn content_rect(&self) -> Rect {
        let left = if self.style == WizardStyle::Modern {
            SIDEBAR_WIDTH
        } else {
            0.0
        };
        let top = self.title_bar_height;
        let bottom = NAV_BUTTON_HEIGHT;

        Rect::new(
            left + CONTENT_PADDING,
            top + CONTENT_PADDING,
            self.size.0 - left - CONTENT_PADDING * 2.0,
            self.size.1 - top - bottom - CONTENT_PADDING * 2.0,
        )
    }

    fn nav_button_area_rect(&self) -> Rect {
        Rect::new(
            0.0,
            self.size.1 - NAV_BUTTON_HEIGHT,
            self.size.0,
            NAV_BUTTON_HEIGHT,
        )
    }

    fn back_button_rect(&self) -> Rect {
        let nav_rect = self.nav_button_area_rect();
        let y = nav_rect.origin.y + (NAV_BUTTON_HEIGHT - BUTTON_HEIGHT) / 2.0;
        Rect::new(CONTENT_PADDING, y, BUTTON_WIDTH, BUTTON_HEIGHT)
    }

    fn cancel_button_rect(&self) -> Rect {
        let nav_rect = self.nav_button_area_rect();
        let y = nav_rect.origin.y + (NAV_BUTTON_HEIGHT - BUTTON_HEIGHT) / 2.0;
        let x = self.size.0 - CONTENT_PADDING - BUTTON_WIDTH;
        Rect::new(x, y, BUTTON_WIDTH, BUTTON_HEIGHT)
    }

    fn finish_button_rect(&self) -> Rect {
        let cancel_rect = self.cancel_button_rect();
        let x = cancel_rect.origin.x - 8.0 - BUTTON_WIDTH;
        Rect::new(x, cancel_rect.origin.y, BUTTON_WIDTH, BUTTON_HEIGHT)
    }

    fn next_button_rect(&self) -> Rect {
        let finish_rect = self.finish_button_rect();
        let x = finish_rect.origin.x - 8.0 - BUTTON_WIDTH;
        Rect::new(x, finish_rect.origin.y, BUTTON_WIDTH, BUTTON_HEIGHT)
    }

    fn step_rect(&self, index: usize) -> Rect {
        let sidebar = self.sidebar_rect();
        Rect::new(
            sidebar.origin.x,
            sidebar.origin.y + index as f32 * STEP_HEIGHT,
            SIDEBAR_WIDTH,
            STEP_HEIGHT,
        )
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> HitPart {
        // Close button
        if self.close_button_rect().contains(pos) {
            return HitPart::CloseButton;
        }

        // Title bar (for dragging)
        if self.title_bar_rect().contains(pos) {
            return HitPart::TitleBar;
        }

        // Sidebar steps (modern style)
        if self.style == WizardStyle::Modern {
            let sidebar = self.sidebar_rect();
            if sidebar.contains(pos) {
                let visible = self.visible_pages();
                for (vis_idx, &page_idx) in visible.iter().enumerate() {
                    let step_rect = self.step_rect(vis_idx);
                    if step_rect.contains(pos) {
                        // Check if we can navigate to this step
                        if self.can_navigate_to_step(page_idx) {
                            return HitPart::SidebarStep(page_idx);
                        }
                    }
                }
            }
        }

        // Navigation buttons
        if self.can_go_back() && self.back_button_rect().contains(pos) {
            return HitPart::BackButton;
        }

        if self.can_go_next() && self.next_button_rect().contains(pos) {
            return HitPart::NextButton;
        }

        if self.is_last_page() && self.finish_button_rect().contains(pos) {
            return HitPart::FinishButton;
        }

        if self.cancel_button_rect().contains(pos) {
            return HitPart::CancelButton;
        }

        HitPart::None
    }

    fn can_navigate_to_step(&self, target_idx: usize) -> bool {
        let current = self.current_page as usize;

        // Can always go to current
        if target_idx == current {
            return true;
        }

        // Going back: check for commit points
        if target_idx < current {
            if let Some(commit_idx) = self.first_commit_reached
                && target_idx < commit_idx {
                    return false;
                }
            // Can only go back to completed pages
            return self
                .pages
                .get(target_idx)
                .map(|p| p.is_completed())
                .unwrap_or(false);
        }

        // Going forward: can only go to next page (must use Next button)
        false
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let hit = self.hit_test(pos);

        match hit {
            HitPart::CloseButton => {
                self.close_button_pressed = true;
                self.base.update();
                true
            }
            HitPart::TitleBar => {
                self.dragging = true;
                self.drag_start = event.global_pos;
                self.drag_start_geometry = self.base.geometry();
                true
            }
            HitPart::BackButton
            | HitPart::NextButton
            | HitPart::FinishButton
            | HitPart::CancelButton
            | HitPart::SidebarStep(_) => {
                self.pressed_part = hit;
                self.base.update();
                true
            }
            HitPart::None => false,
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Handle close button
        if self.close_button_pressed {
            self.close_button_pressed = false;
            if self.close_button_rect().contains(pos) {
                self.reject();
            }
            self.base.update();
            return true;
        }

        // Handle dragging
        if self.dragging {
            self.dragging = false;
            return true;
        }

        // Handle button/step releases
        let hit = self.hit_test(pos);
        if hit == self.pressed_part {
            match hit {
                HitPart::BackButton => {
                    self.back();
                }
                HitPart::NextButton => {
                    self.next();
                }
                HitPart::FinishButton => {
                    self.finish();
                }
                HitPart::CancelButton => {
                    self.reject();
                }
                HitPart::SidebarStep(idx) => {
                    if idx < self.current_page as usize {
                        // Navigate back through history
                        while self.current_page as usize > idx && self.can_go_back() {
                            self.back();
                        }
                    }
                }
                _ => {}
            }
        }

        self.pressed_part = HitPart::None;
        self.base.update();
        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Handle dragging
        if self.dragging {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );
            let new_pos = Point::new(
                self.drag_start_geometry.origin.x + delta.x,
                self.drag_start_geometry.origin.y + delta.y,
            );
            self.base.set_pos(new_pos);
            return true;
        }

        // Update hover states
        let hit = self.hit_test(pos);
        let new_close_hover = hit == HitPart::CloseButton;

        let changed = self.hover_part != hit || self.close_button_hovered != new_close_hover;

        self.hover_part = hit;
        self.close_button_hovered = new_close_hover;

        if changed {
            self.base.update();
        }

        changed
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Escape => {
                self.reject();
                true
            }
            Key::Enter if !event.is_repeat => {
                if self.is_last_page() {
                    self.finish();
                } else {
                    self.next();
                }
                true
            }
            _ => false,
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_backdrop(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let backdrop_rect = Rect::new(
            -rect.origin.x,
            -rect.origin.y,
            rect.origin.x * 2.0 + rect.width() + 2000.0,
            rect.origin.y * 2.0 + rect.height() + 2000.0,
        );
        ctx.renderer()
            .fill_rect(backdrop_rect, Color::from_rgba8(0, 0, 0, 80));
    }

    fn paint_title_bar(&self, ctx: &mut PaintContext<'_>) {
        let title_rect = self.title_bar_rect();

        // Background
        let bg_color = if self.active {
            self.title_bar_active_color
        } else {
            self.title_bar_color
        };
        ctx.renderer().fill_rect(title_rect, bg_color);

        // Title text
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout = TextLayout::with_options(
            &mut font_system,
            &self.title,
            &font,
            TextLayoutOptions::new(),
        );

        let text_y = (self.title_bar_height - layout.height()) / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(12.0, text_y),
                self.text_color,
            );
        }

        // Close button
        self.paint_close_button(ctx);
    }

    fn paint_close_button(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.close_button_rect();

        // Background
        let bg = if self.close_button_pressed {
            self.button_pressed_color
        } else if self.close_button_hovered {
            self.close_button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(rect, bg);

        // X icon
        let icon_margin = 5.0;
        let x1 = rect.origin.x + icon_margin;
        let y1 = rect.origin.y + icon_margin;
        let x2 = rect.origin.x + rect.width() - icon_margin;
        let y2 = rect.origin.y + rect.height() - icon_margin;

        let icon_color = if self.close_button_hovered {
            Color::WHITE
        } else {
            Color::from_rgb8(80, 80, 80)
        };
        let stroke = Stroke::new(icon_color, 1.5);

        ctx.renderer()
            .draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
        ctx.renderer()
            .draw_line(Point::new(x2, y1), Point::new(x1, y2), &stroke);
    }

    fn paint_sidebar(&self, ctx: &mut PaintContext<'_>) {
        if self.style != WizardStyle::Modern {
            return;
        }

        let sidebar_rect = self.sidebar_rect();
        ctx.renderer().fill_rect(sidebar_rect, self.sidebar_color);

        // Draw separator line
        let stroke = Stroke::new(self.separator_color, 1.0);
        let right_x = sidebar_rect.origin.x + sidebar_rect.width();
        let bottom_y = sidebar_rect.origin.y + sidebar_rect.height();
        ctx.renderer().draw_line(
            Point::new(right_x, sidebar_rect.origin.y),
            Point::new(right_x, bottom_y),
            &stroke,
        );

        // Draw steps
        let visible = self.visible_pages();
        for (vis_idx, &page_idx) in visible.iter().enumerate() {
            self.paint_step(ctx, vis_idx, page_idx);
        }
    }

    fn paint_step(&self, ctx: &mut PaintContext<'_>, vis_idx: usize, page_idx: usize) {
        let page = match self.pages.get(page_idx) {
            Some(p) => p,
            None => return,
        };

        let step_rect = self.step_rect(vis_idx);
        let is_current = page_idx == self.current_page as usize;
        let is_completed = page.is_completed();
        let is_hovered = self.hover_part == HitPart::SidebarStep(page_idx);
        let can_navigate = self.can_navigate_to_step(page_idx);

        // Background for current/hovered
        if is_current {
            ctx.renderer()
                .fill_rect(step_rect, Color::from_rgba8(51, 122, 183, 30));
        } else if is_hovered && can_navigate {
            ctx.renderer()
                .fill_rect(step_rect, Color::from_rgba8(0, 0, 0, 10));
        }

        // Step number circle
        let circle_x = step_rect.origin.x + 16.0;
        let circle_y = step_rect.origin.y + STEP_HEIGHT / 2.0;
        let circle_radius = STEP_ICON_SIZE / 2.0;

        let circle_color = if is_current {
            self.active_step_color
        } else if is_completed {
            self.completed_step_color
        } else {
            self.separator_color
        };

        // Draw circle
        let circle_rect = Rect::new(
            circle_x - circle_radius,
            circle_y - circle_radius,
            STEP_ICON_SIZE,
            STEP_ICON_SIZE,
        );
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(circle_rect, circle_radius), circle_color);

        // Draw step number or checkmark
        let mut font_system = FontSystem::new();

        if is_completed && !is_current {
            // Draw checkmark
            let font = Font::new(FontFamily::SansSerif, 14.0);
            let layout =
                TextLayout::with_options(&mut font_system, "✓", &font, TextLayoutOptions::new());
            let text_x = circle_x - layout.width() / 2.0;
            let text_y = circle_y - layout.height() / 2.0;
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    Color::WHITE,
                );
            }
        } else {
            // Draw number
            let num_str = format!("{}", vis_idx + 1);
            let font = Font::new(FontFamily::SansSerif, 12.0);
            let layout = TextLayout::with_options(
                &mut font_system,
                &num_str,
                &font,
                TextLayoutOptions::new(),
            );
            let text_x = circle_x - layout.width() / 2.0;
            let text_y = circle_y - layout.height() / 2.0;
            let text_color = if is_current || is_completed {
                Color::WHITE
            } else {
                self.secondary_text_color
            };
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    text_color,
                );
            }
        }

        // Draw step title
        let title_x = circle_x + circle_radius + 12.0;
        let title_color = if is_current {
            self.active_step_color
        } else if !can_navigate {
            self.secondary_text_color
        } else {
            self.text_color
        };

        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout = TextLayout::with_options(
            &mut font_system,
            page.title(),
            &font,
            TextLayoutOptions::new(),
        );
        let title_y = circle_y - layout.height() / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(title_x, title_y),
                title_color,
            );
        }
    }

    fn paint_content_area(&self, ctx: &mut PaintContext<'_>) {
        let content_left = if self.style == WizardStyle::Modern {
            SIDEBAR_WIDTH
        } else {
            0.0
        };

        // Content background
        let content_bg_rect = Rect::new(
            content_left,
            self.title_bar_height,
            self.size.0 - content_left,
            self.size.1 - self.title_bar_height - NAV_BUTTON_HEIGHT,
        );
        ctx.renderer()
            .fill_rect(content_bg_rect, self.background_color);

        // Draw page title and subtitle
        if let Some(page) = self.current_page() {
            let mut font_system = FontSystem::new();

            // Page title
            let title_font = Font::new(FontFamily::SansSerif, 18.0);
            let title_layout = TextLayout::with_options(
                &mut font_system,
                page.title(),
                &title_font,
                TextLayoutOptions::new(),
            );
            let title_pos = Point::new(
                content_left + CONTENT_PADDING,
                self.title_bar_height + CONTENT_PADDING,
            );
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &title_layout,
                    title_pos,
                    self.text_color,
                );
            }

            // Page subtitle
            if let Some(subtitle) = page.subtitle() {
                let subtitle_font = Font::new(FontFamily::SansSerif, 13.0);
                let subtitle_layout = TextLayout::with_options(
                    &mut font_system,
                    subtitle,
                    &subtitle_font,
                    TextLayoutOptions::new(),
                );
                let subtitle_pos =
                    Point::new(title_pos.x, title_pos.y + title_layout.height() + 4.0);
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        &mut font_system,
                        &subtitle_layout,
                        subtitle_pos,
                        self.secondary_text_color,
                    );
                }
            }

            // Validation errors
            let validation = page.last_validation();
            if !validation.is_valid()
                && let Some(error_msg) = validation.first_error_message() {
                    let error_font = Font::new(FontFamily::SansSerif, 12.0);
                    let error_layout = TextLayout::with_options(
                        &mut font_system,
                        error_msg,
                        &error_font,
                        TextLayoutOptions::new(),
                    );
                    let error_y =
                        self.size.1 - NAV_BUTTON_HEIGHT - CONTENT_PADDING - error_layout.height();
                    let error_pos = Point::new(content_left + CONTENT_PADDING, error_y);
                    if let Ok(mut text_renderer) = TextRenderer::new() {
                        let _ = text_renderer.prepare_layout(
                            &mut font_system,
                            &error_layout,
                            error_pos,
                            self.error_color,
                        );
                    }
                }
        }
    }

    fn paint_nav_buttons(&self, ctx: &mut PaintContext<'_>) {
        let nav_rect = self.nav_button_area_rect();

        // Background
        ctx.renderer().fill_rect(nav_rect, self.background_color);

        // Separator
        let stroke = Stroke::new(self.separator_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(0.0, nav_rect.origin.y),
            Point::new(self.size.0, nav_rect.origin.y),
            &stroke,
        );

        // Back button
        if self.can_go_back() {
            self.paint_button(
                ctx,
                self.back_button_rect(),
                "Back",
                false,
                HitPart::BackButton,
            );
        } else {
            self.paint_button_disabled(ctx, self.back_button_rect(), "Back");
        }

        // Next button
        if self.can_go_next() {
            self.paint_button(
                ctx,
                self.next_button_rect(),
                "Next",
                true,
                HitPart::NextButton,
            );
        }

        // Finish button
        if self.is_last_page() {
            self.paint_button(
                ctx,
                self.finish_button_rect(),
                "Finish",
                true,
                HitPart::FinishButton,
            );
        }

        // Cancel button
        self.paint_button(
            ctx,
            self.cancel_button_rect(),
            "Cancel",
            false,
            HitPart::CancelButton,
        );
    }

    fn paint_button(
        &self,
        ctx: &mut PaintContext<'_>,
        rect: Rect,
        label: &str,
        primary: bool,
        hit_part: HitPart,
    ) {
        let is_hovered = self.hover_part == hit_part;
        let is_pressed = self.pressed_part == hit_part;

        // Background
        let bg_color = if primary {
            if is_pressed {
                self.primary_button_hover_color
            } else if is_hovered {
                self.primary_button_hover_color
            } else {
                self.primary_button_color
            }
        } else if is_pressed {
            self.button_pressed_color
        } else if is_hovered {
            self.button_hover_color
        } else {
            self.button_color
        };

        let rounded = RoundedRect::new(rect, 4.0);
        ctx.renderer().fill_rounded_rect(rounded, bg_color);

        // Border for non-primary
        if !primary {
            let stroke = Stroke::new(self.border_color, 1.0);
            ctx.renderer().stroke_rounded_rect(rounded, &stroke);
        }

        // Label
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout =
            TextLayout::with_options(&mut font_system, label, &font, TextLayoutOptions::new());

        let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
        let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

        let text_color = if primary {
            Color::WHITE
        } else {
            self.text_color
        };
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }
    }

    fn paint_button_disabled(&self, ctx: &mut PaintContext<'_>, rect: Rect, label: &str) {
        // Background
        let rounded = RoundedRect::new(rect, 4.0);
        ctx.renderer()
            .fill_rounded_rect(rounded, self.button_disabled_color);

        // Label
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout =
            TextLayout::with_options(&mut font_system, label, &font, TextLayoutOptions::new());

        let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
        let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                self.secondary_text_color,
            );
        }
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.dialog_rect();
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);
    }
}

impl Object for Wizard {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Wizard {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(self.size.0, self.size.1).with_minimum_dimensions(400.0, 300.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if !self.is_open {
            return;
        }

        self.paint_backdrop(ctx);
        self.paint_sidebar(ctx);
        self.paint_content_area(ctx);
        self.paint_title_bar(ctx);
        self.paint_nav_buttons(ctx);
        self.paint_border(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Leave(_) => {
                if self.close_button_hovered || self.hover_part != HitPart::None {
                    self.close_button_hovered = false;
                    self.hover_part = HitPart::None;
                    self.base.update();
                }
                false
            }
            WidgetEvent::FocusIn(_) => {
                self.active = true;
                self.base.update();
                true
            }
            WidgetEvent::FocusOut(_) => {
                self.active = false;
                self.base.update();
                true
            }
            _ => false,
        }
    }
}

impl Default for Wizard {
    fn default() -> Self {
        Self::new("Wizard")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.is_valid());
        assert!(valid.errors().is_empty());

        let invalid = ValidationResult::invalid("Error message");
        assert!(!invalid.is_valid());
        assert_eq!(invalid.first_error_message(), Some("Error message"));

        let field_error = ValidationResult::field_error("email", "Invalid email");
        assert!(!field_error.is_valid());
        assert_eq!(field_error.errors()[0].field_id, Some("email".to_string()));
    }

    #[test]
    fn test_wizard_page_creation() {
        setup();
        let page = WizardPage::new("Test Page")
            .with_subtitle("A test subtitle")
            .with_id("test_page");

        assert_eq!(page.id(), "test_page");
        assert_eq!(page.title(), "Test Page");
        assert_eq!(page.subtitle(), Some("A test subtitle"));
        assert!(!page.is_commit_point());
        assert!(!page.is_completed());
    }

    #[test]
    fn test_wizard_page_validation() {
        setup();
        let mut page =
            WizardPage::new("Test").with_validator(|_| ValidationResult::invalid("Always fails"));

        let result = page.validate();
        assert!(!result.is_valid());
        assert_eq!(result.first_error_message(), Some("Always fails"));
    }

    #[test]
    fn test_wizard_creation() {
        setup();
        let wizard = Wizard::new("Test Wizard").with_style(WizardStyle::Modern);

        assert_eq!(wizard.title(), "Test Wizard");
        assert_eq!(wizard.style(), WizardStyle::Modern);
        assert_eq!(wizard.page_count(), 0);
        assert!(!wizard.is_open());
    }

    #[test]
    fn test_wizard_page_management() {
        setup();
        let mut wizard = Wizard::new("Test");

        let idx1 = wizard.add_page(WizardPage::new("Page 1"));
        let idx2 = wizard.add_page(WizardPage::new("Page 2"));

        assert_eq!(wizard.page_count(), 2);
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(wizard.current_page_index(), 0);

        assert_eq!(wizard.page(0).map(|p| p.title()), Some("Page 1"));
        assert_eq!(wizard.page(1).map(|p| p.title()), Some("Page 2"));
    }

    #[test]
    fn test_wizard_navigation() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));
        wizard.add_page(WizardPage::new("Page 2"));
        wizard.add_page(WizardPage::new("Page 3"));

        wizard.open();
        assert_eq!(wizard.current_page_index(), 0);
        assert!(wizard.can_go_next());
        assert!(!wizard.can_go_back());

        wizard.next();
        assert_eq!(wizard.current_page_index(), 1);
        assert!(wizard.can_go_next());
        assert!(wizard.can_go_back());

        wizard.next();
        assert_eq!(wizard.current_page_index(), 2);
        assert!(!wizard.can_go_next());
        assert!(wizard.is_last_page());

        wizard.back();
        assert_eq!(wizard.current_page_index(), 1);
    }

    #[test]
    fn test_wizard_commit_point() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));
        wizard.add_page(WizardPage::new("Page 2").as_commit_point());
        wizard.add_page(WizardPage::new("Page 3"));

        wizard.open();
        wizard.next(); // To page 2 (commit point)
        wizard.next(); // To page 3

        assert_eq!(wizard.current_page_index(), 2);
        // Should be able to go back to page 2 but not page 1
        assert!(wizard.can_go_back());
        wizard.back();
        assert_eq!(wizard.current_page_index(), 1);
        assert!(!wizard.can_go_back()); // Can't go back past commit point
    }

    #[test]
    fn test_wizard_signals() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));
        wizard.add_page(WizardPage::new("Page 2"));

        let page_changed = Arc::new(AtomicI32::new(-1));
        let page_changed_clone = page_changed.clone();

        wizard.current_page_changed.connect(move |&idx| {
            page_changed_clone.store(idx, Ordering::SeqCst);
        });

        wizard.open();
        wizard.next();

        assert_eq!(page_changed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_wizard_finish() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));

        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = finished.clone();

        wizard.finished.connect(move |&accepted| {
            finished_clone.store(accepted, Ordering::SeqCst);
        });

        wizard.open();
        wizard.finish();

        assert!(finished.load(Ordering::SeqCst));
        assert!(!wizard.is_open());
        assert_eq!(wizard.result(), DialogResult::Accepted);
    }

    #[test]
    fn test_wizard_validation_blocks_navigation() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(
            WizardPage::new("Page 1")
                .with_validator(|_| ValidationResult::invalid("Validation failed")),
        );
        wizard.add_page(WizardPage::new("Page 2"));

        let validation_failed = Arc::new(AtomicBool::new(false));
        let validation_failed_clone = validation_failed.clone();

        wizard.validation_failed.connect(move |_| {
            validation_failed_clone.store(true, Ordering::SeqCst);
        });

        wizard.open();
        let navigated = wizard.next();

        assert!(!navigated);
        assert!(validation_failed.load(Ordering::SeqCst));
        assert_eq!(wizard.current_page_index(), 0);
    }

    #[test]
    fn test_wizard_conditional_pages() {
        setup();
        let show_page2 = Arc::new(AtomicBool::new(false));
        let show_page2_clone = show_page2.clone();

        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));
        wizard.add_page(
            WizardPage::new("Page 2")
                .with_condition(move |_| show_page2_clone.load(Ordering::SeqCst)),
        );
        wizard.add_page(WizardPage::new("Page 3"));

        wizard.open();

        // With page 2 hidden, next should go to page 3
        wizard.next();
        assert_eq!(wizard.current_page_index(), 2);

        // Reset and enable page 2
        wizard.restart();
        show_page2.store(true, Ordering::SeqCst);

        wizard.next();
        assert_eq!(wizard.current_page_index(), 1); // Now goes to page 2
    }

    #[test]
    fn test_wizard_restart() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));
        wizard.add_page(WizardPage::new("Page 2"));

        wizard.open();
        wizard.next();
        assert_eq!(wizard.current_page_index(), 1);

        wizard.restart();
        assert_eq!(wizard.current_page_index(), 0);
        assert!(!wizard.page(0).unwrap().is_completed());
    }

    #[test]
    fn test_wizard_reject() {
        setup();
        let mut wizard = Wizard::new("Test");
        wizard.add_page(WizardPage::new("Page 1"));

        let rejected = Arc::new(AtomicBool::new(false));
        let rejected_clone = rejected.clone();

        wizard.rejected.connect(move |()| {
            rejected_clone.store(true, Ordering::SeqCst);
        });

        wizard.open();
        wizard.reject();

        assert!(rejected.load(Ordering::SeqCst));
        assert!(!wizard.is_open());
        assert_eq!(wizard.result(), DialogResult::Rejected);
    }
}
