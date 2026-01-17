//! Accessibility roles for widgets.

use accesskit::Role;

/// The accessibility role of a widget.
///
/// This enum provides a simplified set of roles commonly used in GUI toolkits.
/// It maps to the more comprehensive AccessKit `Role` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AccessibleRole {
    /// A generic widget with no specific role.
    #[default]
    Unknown,

    /// A window or top-level container.
    Window,

    /// A dialog box.
    Dialog,

    /// A push button.
    Button,

    /// A checkbox that can be checked or unchecked.
    CheckBox,

    /// A radio button (mutually exclusive selection).
    RadioButton,

    /// A single-line text input field.
    TextInput,

    /// A multi-line text editing area.
    TextArea,

    /// A static text label.
    Label,

    /// A hyperlink.
    Link,

    /// An image.
    Image,

    /// A progress indicator.
    ProgressBar,

    /// A slider for selecting a value from a range.
    Slider,

    /// A spin box for numeric input.
    SpinBox,

    /// A combo box / dropdown.
    ComboBox,

    /// A list of items.
    List,

    /// An item within a list.
    ListItem,

    /// A tree view.
    Tree,

    /// An item within a tree.
    TreeItem,

    /// A table/grid.
    Table,

    /// A row within a table.
    TableRow,

    /// A cell within a table.
    TableCell,

    /// A column header.
    ColumnHeader,

    /// A row header.
    RowHeader,

    /// A menu bar.
    MenuBar,

    /// A menu (popup or submenu).
    Menu,

    /// A menu item.
    MenuItem,

    /// A menu item with a checkbox.
    MenuItemCheckBox,

    /// A menu item with a radio button.
    MenuItemRadio,

    /// A toolbar.
    ToolBar,

    /// A status bar.
    StatusBar,

    /// A tab list container.
    TabList,

    /// A single tab.
    Tab,

    /// The content panel of a tab.
    TabPanel,

    /// A scroll bar.
    ScrollBar,

    /// A scrollable area.
    ScrollArea,

    /// A splitter/divider.
    Splitter,

    /// A group box or frame.
    Group,

    /// A tooltip.
    Tooltip,

    /// A separator line.
    Separator,

    /// A calendar widget.
    Calendar,

    /// A date picker.
    DatePicker,

    /// A time picker.
    TimePicker,

    /// A color picker.
    ColorPicker,

    /// An alert/message box.
    Alert,

    /// A generic container (like a panel or frame).
    Container,

    /// A pane within a layout.
    Pane,
}

impl AccessibleRole {
    /// Convert to AccessKit's Role enum.
    pub fn to_accesskit_role(self) -> Role {
        match self {
            AccessibleRole::Unknown => Role::Unknown,
            AccessibleRole::Window => Role::Window,
            AccessibleRole::Dialog => Role::Dialog,
            AccessibleRole::Button => Role::Button,
            AccessibleRole::CheckBox => Role::CheckBox,
            AccessibleRole::RadioButton => Role::RadioButton,
            AccessibleRole::TextInput => Role::TextInput,
            AccessibleRole::TextArea => Role::MultilineTextInput,
            AccessibleRole::Label => Role::Label,
            AccessibleRole::Link => Role::Link,
            AccessibleRole::Image => Role::Image,
            AccessibleRole::ProgressBar => Role::ProgressIndicator,
            AccessibleRole::Slider => Role::Slider,
            AccessibleRole::SpinBox => Role::SpinButton,
            AccessibleRole::ComboBox => Role::ComboBox,
            AccessibleRole::List => Role::List,
            AccessibleRole::ListItem => Role::ListItem,
            AccessibleRole::Tree => Role::Tree,
            AccessibleRole::TreeItem => Role::TreeItem,
            AccessibleRole::Table => Role::Table,
            AccessibleRole::TableRow => Role::Row,
            AccessibleRole::TableCell => Role::Cell,
            AccessibleRole::ColumnHeader => Role::ColumnHeader,
            AccessibleRole::RowHeader => Role::RowHeader,
            AccessibleRole::MenuBar => Role::MenuBar,
            AccessibleRole::Menu => Role::Menu,
            AccessibleRole::MenuItem => Role::MenuItem,
            AccessibleRole::MenuItemCheckBox => Role::MenuItemCheckBox,
            AccessibleRole::MenuItemRadio => Role::MenuItemRadio,
            AccessibleRole::ToolBar => Role::Toolbar,
            AccessibleRole::StatusBar => Role::Status,
            AccessibleRole::TabList => Role::TabList,
            AccessibleRole::Tab => Role::Tab,
            AccessibleRole::TabPanel => Role::TabPanel,
            AccessibleRole::ScrollBar => Role::ScrollBar,
            AccessibleRole::ScrollArea => Role::ScrollView,
            AccessibleRole::Splitter => Role::Splitter,
            AccessibleRole::Group => Role::Group,
            AccessibleRole::Tooltip => Role::Tooltip,
            AccessibleRole::Separator => Role::Splitter, // No direct Separator, use Splitter
            AccessibleRole::Calendar => Role::Grid,
            AccessibleRole::DatePicker => Role::TextInput, // No DatePicker, use TextInput
            AccessibleRole::TimePicker => Role::TextInput, // No TimePicker, use TextInput
            AccessibleRole::ColorPicker => Role::ColorWell,
            AccessibleRole::Alert => Role::Alert,
            AccessibleRole::Container => Role::GenericContainer,
            AccessibleRole::Pane => Role::Pane,
        }
    }
}

impl From<AccessibleRole> for Role {
    fn from(role: AccessibleRole) -> Self {
        role.to_accesskit_role()
    }
}
