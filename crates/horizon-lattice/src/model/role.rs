//! Data roles for item models.
//!
//! Roles define what type of data is being requested or set on a model item.
//! Each item can have multiple pieces of data associated with it, distinguished
//! by their role.

use horizon_lattice_render::{Color, Font, Icon};

/// Standard roles for accessing different aspects of item data.
///
/// When querying data from a model via `ItemModel::data()`, the role specifies
/// what information is being requested. Each item can have data for multiple
/// roles.
///
/// # Standard Roles
///
/// - **Display**: The primary text to show (e.g., item label)
/// - **Decoration**: Icon or image to display alongside text
/// - **Edit**: Value for editing (may differ from display text)
/// - **ToolTip**: Text shown when hovering over the item
/// - **StatusTip**: Text shown in the status bar
/// - **WhatsThis**: Extended help text
/// - **Font**: Custom font for rendering
/// - **TextAlignment**: Text alignment flags
/// - **BackgroundColor**: Background color
/// - **ForegroundColor**: Text/foreground color
/// - **CheckState**: Checkbox state (unchecked, checked, partial)
/// - **SizeHint**: Size hint for the item
/// - **UserRole**: First role available for custom data
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{ItemModel, ModelIndex, ItemRole};
///
/// // Get display text
/// let text = model.data(index, ItemRole::Display);
///
/// // Get custom data
/// let custom = model.data(index, ItemRole::User(0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ItemRole {
    /// Primary text to display. Should return `String`.
    Display = 0,

    /// Icon or decoration to show. Should return `Icon`.
    Decoration = 1,

    /// Value for editing (may be richer than display text). Type depends on item.
    Edit = 2,

    /// Tooltip text shown on hover. Should return `String`.
    ToolTip = 3,

    /// Text shown in status bar when item is selected. Should return `String`.
    StatusTip = 4,

    /// Extended "What's This?" help text. Should return `String`.
    WhatsThis = 5,

    /// Custom font for this item. Should return `Font`.
    Font = 6,

    /// Text alignment for this item. Should return `TextAlignment`.
    TextAlignment = 7,

    /// Background color for the item. Should return `Color`.
    BackgroundColor = 8,

    /// Foreground (text) color for the item. Should return `Color`.
    ForegroundColor = 9,

    /// Check state for checkable items. Should return `CheckState`.
    CheckState = 10,

    /// Size hint for the item. Should return `Size`.
    SizeHint = 11,

    /// Access key (mnemonic) for the item. Should return `String`.
    AccessibleText = 12,

    /// Accessible description for screen readers. Should return `String`.
    AccessibleDescription = 13,

    /// First role available for application-specific data.
    /// Use `ItemRole::User(n)` for custom roles where n >= 0.
    User(u32) = 256,
}

impl ItemRole {
    /// Returns `true` if this is a user-defined role.
    #[inline]
    pub fn is_user_role(&self) -> bool {
        matches!(self, ItemRole::User(_))
    }

    /// Returns the numeric value of this role.
    ///
    /// Standard roles have fixed values 0-255.
    /// User roles have values >= 256.
    pub fn value(&self) -> u32 {
        match self {
            ItemRole::Display => 0,
            ItemRole::Decoration => 1,
            ItemRole::Edit => 2,
            ItemRole::ToolTip => 3,
            ItemRole::StatusTip => 4,
            ItemRole::WhatsThis => 5,
            ItemRole::Font => 6,
            ItemRole::TextAlignment => 7,
            ItemRole::BackgroundColor => 8,
            ItemRole::ForegroundColor => 9,
            ItemRole::CheckState => 10,
            ItemRole::SizeHint => 11,
            ItemRole::AccessibleText => 12,
            ItemRole::AccessibleDescription => 13,
            ItemRole::User(n) => 256 + n,
        }
    }

    /// Creates an ItemRole from a numeric value.
    ///
    /// Returns `None` for reserved but undefined role values (14-255).
    pub fn from_value(value: u32) -> Option<Self> {
        match value {
            0 => Some(ItemRole::Display),
            1 => Some(ItemRole::Decoration),
            2 => Some(ItemRole::Edit),
            3 => Some(ItemRole::ToolTip),
            4 => Some(ItemRole::StatusTip),
            5 => Some(ItemRole::WhatsThis),
            6 => Some(ItemRole::Font),
            7 => Some(ItemRole::TextAlignment),
            8 => Some(ItemRole::BackgroundColor),
            9 => Some(ItemRole::ForegroundColor),
            10 => Some(ItemRole::CheckState),
            11 => Some(ItemRole::SizeHint),
            12 => Some(ItemRole::AccessibleText),
            13 => Some(ItemRole::AccessibleDescription),
            14..=255 => None, // Reserved for future standard roles
            n => Some(ItemRole::User(n - 256)),
        }
    }
}

/// Alignment flags for text within an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextAlignment {
    /// Horizontal alignment.
    pub horizontal: HorizontalAlignment,
    /// Vertical alignment.
    pub vertical: VerticalAlignment,
}

impl TextAlignment {
    /// Creates a new alignment with the specified horizontal and vertical values.
    pub fn new(horizontal: HorizontalAlignment, vertical: VerticalAlignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    /// Left-aligned, vertically centered (common default).
    pub const fn left() -> Self {
        Self {
            horizontal: HorizontalAlignment::Left,
            vertical: VerticalAlignment::Center,
        }
    }

    /// Centered horizontally and vertically.
    pub const fn center() -> Self {
        Self {
            horizontal: HorizontalAlignment::Center,
            vertical: VerticalAlignment::Center,
        }
    }

    /// Right-aligned, vertically centered.
    pub const fn right() -> Self {
        Self {
            horizontal: HorizontalAlignment::Right,
            vertical: VerticalAlignment::Center,
        }
    }
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HorizontalAlignment {
    /// Align to the left edge.
    #[default]
    Left,
    /// Align to the center.
    Center,
    /// Align to the right edge.
    Right,
    /// Justify text (stretch to fill width).
    Justify,
}

/// Vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlignment {
    /// Align to the top edge.
    Top,
    /// Align to the center.
    #[default]
    Center,
    /// Align to the bottom edge.
    Bottom,
    /// Align to the text baseline.
    Baseline,
}

/// Check state for checkable items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CheckState {
    /// Item is unchecked.
    #[default]
    Unchecked,
    /// Item is partially checked (for tri-state checkboxes).
    PartiallyChecked,
    /// Item is checked.
    Checked,
}

impl CheckState {
    /// Returns `true` if the item is checked (fully or partially).
    pub fn is_checked(&self) -> bool {
        !matches!(self, CheckState::Unchecked)
    }

    /// Returns `true` if the item is fully checked.
    pub fn is_fully_checked(&self) -> bool {
        matches!(self, CheckState::Checked)
    }

    /// Toggles between Unchecked and Checked.
    /// PartiallyChecked becomes Checked.
    pub fn toggle(&self) -> CheckState {
        match self {
            CheckState::Unchecked => CheckState::Checked,
            CheckState::PartiallyChecked | CheckState::Checked => CheckState::Unchecked,
        }
    }
}

/// Type-erased container for item data.
///
/// `ItemData` can hold any type of data associated with an item role.
/// It provides type-safe access through the `as_*` methods and the
/// generic `downcast` method.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::ItemData;
///
/// // Create from a string
/// let data = ItemData::from("Hello");
/// assert_eq!(data.as_string(), Some("Hello"));
///
/// // Create from a color
/// let data = ItemData::from(Color::RED);
/// assert!(data.as_color().is_some());
///
/// // Downcast to arbitrary type
/// let data = ItemData::new(42u32);
/// assert_eq!(data.downcast::<u32>(), Some(&42));
/// ```
#[derive(Debug, Default)]
pub enum ItemData {
    /// No data.
    #[default]
    None,
    /// String data (for Display, ToolTip, etc.).
    String(String),
    /// Integer data.
    Int(i64),
    /// Floating point data.
    Float(f64),
    /// Boolean data.
    Bool(bool),
    /// Color data.
    Color(Color),
    /// Font data.
    Font(Font),
    /// Icon data.
    Icon(Icon),
    /// Text alignment data.
    TextAlignment(TextAlignment),
    /// Check state data.
    CheckState(CheckState),
    /// Size data (width, height).
    Size(f32, f32),
    /// Custom data (type-erased).
    Custom(Box<dyn std::any::Any + Send + Sync>),
}

impl Clone for ItemData {
    fn clone(&self) -> Self {
        match self {
            ItemData::None => ItemData::None,
            ItemData::String(s) => ItemData::String(s.clone()),
            ItemData::Int(n) => ItemData::Int(*n),
            ItemData::Float(n) => ItemData::Float(*n),
            ItemData::Bool(b) => ItemData::Bool(*b),
            ItemData::Color(c) => ItemData::Color(*c),
            ItemData::Font(f) => ItemData::Font(f.clone()),
            ItemData::Icon(i) => ItemData::Icon(i.clone()),
            ItemData::TextAlignment(a) => ItemData::TextAlignment(*a),
            ItemData::CheckState(s) => ItemData::CheckState(*s),
            ItemData::Size(w, h) => ItemData::Size(*w, *h),
            // Custom data cannot be cloned; becomes None
            ItemData::Custom(_) => ItemData::None,
        }
    }
}

impl ItemData {
    /// Creates new custom data from any type.
    pub fn new<T: std::any::Any + Send + Sync + 'static>(value: T) -> Self {
        ItemData::Custom(Box::new(value))
    }

    /// Returns `true` if this is `ItemData::None`.
    pub fn is_none(&self) -> bool {
        matches!(self, ItemData::None)
    }

    /// Returns `true` if this contains some data.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Attempts to get the data as a string slice.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ItemData::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Attempts to get the data as an owned string.
    pub fn into_string(self) -> Option<String> {
        match self {
            ItemData::String(s) => Some(s),
            _ => None,
        }
    }

    /// Attempts to get the data as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ItemData::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Attempts to get the data as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ItemData::Float(n) => Some(*n),
            _ => None,
        }
    }

    /// Attempts to get the data as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ItemData::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Attempts to get the data as a color.
    pub fn as_color(&self) -> Option<&Color> {
        match self {
            ItemData::Color(c) => Some(c),
            _ => None,
        }
    }

    /// Attempts to get the data as a font.
    pub fn as_font(&self) -> Option<&Font> {
        match self {
            ItemData::Font(f) => Some(f),
            _ => None,
        }
    }

    /// Attempts to get the data as an icon.
    pub fn as_icon(&self) -> Option<&Icon> {
        match self {
            ItemData::Icon(i) => Some(i),
            _ => None,
        }
    }

    /// Attempts to get the data as text alignment.
    pub fn as_text_alignment(&self) -> Option<TextAlignment> {
        match self {
            ItemData::TextAlignment(a) => Some(*a),
            _ => None,
        }
    }

    /// Attempts to get the data as check state.
    pub fn as_check_state(&self) -> Option<CheckState> {
        match self {
            ItemData::CheckState(s) => Some(*s),
            _ => None,
        }
    }

    /// Attempts to get the data as a size tuple.
    pub fn as_size(&self) -> Option<(f32, f32)> {
        match self {
            ItemData::Size(w, h) => Some((*w, *h)),
            _ => None,
        }
    }

    /// Attempts to downcast custom data to the specified type.
    pub fn downcast<T: std::any::Any>(&self) -> Option<&T> {
        match self {
            ItemData::Custom(data) => data.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Attempts to downcast and take ownership of custom data.
    pub fn downcast_into<T: std::any::Any>(self) -> Option<T> {
        match self {
            ItemData::Custom(data) => data.downcast::<T>().ok().map(|b| *b),
            _ => None,
        }
    }
}

impl From<String> for ItemData {
    fn from(s: String) -> Self {
        ItemData::String(s)
    }
}

impl From<&str> for ItemData {
    fn from(s: &str) -> Self {
        ItemData::String(s.to_string())
    }
}

impl From<i64> for ItemData {
    fn from(n: i64) -> Self {
        ItemData::Int(n)
    }
}

impl From<i32> for ItemData {
    fn from(n: i32) -> Self {
        ItemData::Int(n as i64)
    }
}

impl From<f64> for ItemData {
    fn from(n: f64) -> Self {
        ItemData::Float(n)
    }
}

impl From<f32> for ItemData {
    fn from(n: f32) -> Self {
        ItemData::Float(n as f64)
    }
}

impl From<bool> for ItemData {
    fn from(b: bool) -> Self {
        ItemData::Bool(b)
    }
}

impl From<Color> for ItemData {
    fn from(c: Color) -> Self {
        ItemData::Color(c)
    }
}

impl From<Font> for ItemData {
    fn from(f: Font) -> Self {
        ItemData::Font(f)
    }
}

impl From<Icon> for ItemData {
    fn from(i: Icon) -> Self {
        ItemData::Icon(i)
    }
}

impl From<TextAlignment> for ItemData {
    fn from(a: TextAlignment) -> Self {
        ItemData::TextAlignment(a)
    }
}

impl From<CheckState> for ItemData {
    fn from(s: CheckState) -> Self {
        ItemData::CheckState(s)
    }
}

impl From<Option<String>> for ItemData {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(s) => ItemData::String(s),
            None => ItemData::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_role_values() {
        assert_eq!(ItemRole::Display.value(), 0);
        assert_eq!(ItemRole::Decoration.value(), 1);
        assert_eq!(ItemRole::User(0).value(), 256);
        assert_eq!(ItemRole::User(10).value(), 266);
    }

    #[test]
    fn test_item_role_from_value() {
        assert_eq!(ItemRole::from_value(0), Some(ItemRole::Display));
        assert_eq!(ItemRole::from_value(10), Some(ItemRole::CheckState));
        assert_eq!(ItemRole::from_value(256), Some(ItemRole::User(0)));
        assert_eq!(ItemRole::from_value(100), None); // Reserved
    }

    #[test]
    fn test_check_state_toggle() {
        assert_eq!(CheckState::Unchecked.toggle(), CheckState::Checked);
        assert_eq!(CheckState::Checked.toggle(), CheckState::Unchecked);
        assert_eq!(CheckState::PartiallyChecked.toggle(), CheckState::Unchecked);
    }

    #[test]
    fn test_item_data_string() {
        let data = ItemData::from("hello");
        assert_eq!(data.as_string(), Some("hello"));
        assert!(data.as_int().is_none());
    }

    #[test]
    fn test_item_data_custom() {
        #[derive(Debug, PartialEq)]
        struct MyData(u32);

        let data = ItemData::new(MyData(42));
        assert_eq!(data.downcast::<MyData>(), Some(&MyData(42)));
        assert!(data.downcast::<u32>().is_none());
    }

    #[test]
    fn test_text_alignment() {
        let align = TextAlignment::center();
        assert_eq!(align.horizontal, HorizontalAlignment::Center);
        assert_eq!(align.vertical, VerticalAlignment::Center);
    }
}
