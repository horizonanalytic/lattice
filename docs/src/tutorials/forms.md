# Tutorial: Forms and Validation

Learn to build input forms with validation and proper layout.

## What You'll Learn

- Using input widgets (LineEdit, CheckBox, ComboBox, SpinBox)
- Organizing forms with FormLayout
- Input validation patterns
- Collecting and processing form data

## Prerequisites

- Completed the [Button Clicks](./button-clicks.md) tutorial
- Understanding of layouts from [Layouts Guide](../guides/layouts.md)

## Step 1: Text Input with LineEdit

LineEdit is for single-line text input:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{LineEdit, Label, Container, Window};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Text Input")
        .with_size(400.0, 200.0);

    // Create a text input
    let mut name_input = LineEdit::new();
    name_input.set_placeholder("Enter your name...");

    // React to text changes
    name_input.text_changed.connect(|text| {
        println!("Text changed: {}", text);
    });

    // React to Enter key
    name_input.return_pressed.connect(|| {
        println!("Enter pressed!");
    });

    let label = Label::new("Name:");

    let mut layout = VBoxLayout::new();
    layout.add_widget(label.object_id());
    layout.add_widget(name_input.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### LineEdit Features

```rust,ignore
use horizon_lattice::widget::widgets::{LineEdit, EchoMode};

// Password field
let mut password = LineEdit::new()
    .with_echo_mode(EchoMode::Password);

// With initial text
let mut edit = LineEdit::with_text("Initial value");

// Read-only field
let mut display = LineEdit::new();
display.set_read_only(true);
display.set_text("Cannot edit this");

// With maximum length
let mut short = LineEdit::new();
short.set_max_length(Some(10));

// With clear button
let mut searchbox = LineEdit::new();
searchbox.set_clear_button(true);
searchbox.set_placeholder("Search...");
```

### LineEdit Signals

```rust,ignore
// Text changed (after validation passes)
edit.text_changed.connect(|text| { /* ... */ });

// Text edited (before validation, raw input)
edit.text_edited.connect(|text| { /* ... */ });

// Enter/Return key pressed
edit.return_pressed.connect(|| { /* ... */ });

// Focus lost or Enter pressed
edit.editing_finished.connect(|| { /* ... */ });

// Clear button clicked
edit.cleared.connect(|| { /* ... */ });

// Input rejected by validator
edit.input_rejected.connect(|| { /* ... */ });
```

## Step 2: Checkboxes

CheckBox provides binary or tri-state selection:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{CheckBox, CheckState, Label, Container, Window};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Checkboxes")
        .with_size(300.0, 200.0);

    // Simple checkbox
    let terms = CheckBox::new("I accept the terms and conditions");

    // Pre-checked checkbox
    let newsletter = CheckBox::new("Subscribe to newsletter")
        .with_checked(true);

    // React to state changes
    terms.state_changed().connect(|&state| {
        match state {
            CheckState::Checked => println!("Terms accepted"),
            CheckState::Unchecked => println!("Terms declined"),
            CheckState::PartiallyChecked => println!("Partial"),
        }
    });

    // Boolean signal (simpler)
    newsletter.toggled().connect(|&checked| {
        println!("Newsletter: {}", if checked { "yes" } else { "no" });
    });

    let mut layout = VBoxLayout::new();
    layout.add_widget(terms.object_id());
    layout.add_widget(newsletter.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### Tri-State Checkboxes

For "select all" patterns:

```rust,ignore
use horizon_lattice::widget::widgets::{CheckBox, CheckState};

// Enable tri-state mode
let mut select_all = CheckBox::new("Select all")
    .with_tri_state(true);

// Set partial state (e.g., when some children are checked)
select_all.set_check_state(CheckState::PartiallyChecked);

// State cycles: Unchecked -> Checked -> PartiallyChecked -> Unchecked
select_all.toggle();
```

## Step 3: Dropdown Selection with ComboBox

ComboBox provides dropdown selection:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{ComboBox, Label, Container, Window};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};
use horizon_lattice::model::StringListComboModel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Dropdown")
        .with_size(300.0, 200.0);

    // Create model with items
    let countries = vec!["United States", "Canada", "Mexico", "United Kingdom"];
    let model = StringListComboModel::from(countries);

    // Create combo box
    let mut combo = ComboBox::new()
        .with_model(Box::new(model));

    // Set default selection
    combo.set_current_index(0);

    // React to selection changes
    combo.current_index_changed.connect(|&index| {
        println!("Selected index: {}", index);
    });

    combo.current_text_changed.connect(|text| {
        println!("Selected: {}", text);
    });

    let label = Label::new("Country:");

    let mut layout = VBoxLayout::new();
    layout.add_widget(label.object_id());
    layout.add_widget(combo.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### Editable ComboBox

Allow typing to filter or enter custom values:

```rust,ignore
use horizon_lattice::widget::widgets::ComboBox;
use horizon_lattice::model::StringListComboModel;

let fruits = vec!["Apple", "Apricot", "Avocado", "Banana", "Blueberry"];
let model = StringListComboModel::from(fruits);

let mut combo = ComboBox::new()
    .with_model(Box::new(model))
    .with_editable(true)
    .with_placeholder("Type to filter...");

// Typing "Ap" filters to: Apple, Apricot
// User can also enter a custom value not in the list
```

### ComboBox Methods

```rust,ignore
// Get current selection
let index = combo.current_index();  // -1 if nothing selected
let text = combo.current_text();

// Set selection
combo.set_current_index(2);
combo.set_current_text("Canada");

// Find item
if let Some(idx) = combo.find_text("Mexico") {
    combo.set_current_index(idx as i32);
}

// Item count
let count = combo.count();

// Popup control
combo.show_popup();
combo.hide_popup();
```

## Step 4: Numeric Input with SpinBox

SpinBox is for integer input with increment/decrement:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{SpinBox, Label, Container, Window};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Numeric Input")
        .with_size(300.0, 200.0);

    // Create a spinbox with range
    let mut age = SpinBox::new()
        .with_range(0, 120)
        .with_value(25)
        .with_single_step(1);

    // React to value changes
    age.value_changed.connect(|&value| {
        println!("Age: {}", value);
    });

    let label = Label::new("Age:");

    let mut layout = VBoxLayout::new();
    layout.add_widget(label.object_id());
    layout.add_widget(age.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### SpinBox Features

```rust,ignore
use horizon_lattice::widget::widgets::SpinBox;

// With prefix and suffix
let mut price = SpinBox::new()
    .with_range(0, 9999)
    .with_prefix("$")
    .with_suffix(".00");

// With special value text (shown at minimum)
let mut quantity = SpinBox::new()
    .with_range(0, 100)
    .with_special_value_text("Auto");  // Shows "Auto" when value is 0

// Wrapping (loops from max to min)
let mut hour = SpinBox::new()
    .with_range(0, 23)
    .with_wrapping(true);  // 23 + 1 = 0

// Larger step size
let mut percent = SpinBox::new()
    .with_range(0, 100)
    .with_single_step(5)
    .with_suffix("%");

// With acceleration on hold
let mut fast = SpinBox::new()
    .with_range(0, 1000)
    .with_acceleration(true);
```

## Step 5: FormLayout

FormLayout automatically aligns label-field pairs:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Label, LineEdit, SpinBox, CheckBox, ComboBox, PushButton, Container, Window
};
use horizon_lattice::widget::layout::{FormLayout, FieldGrowthPolicy, Alignment, LayoutKind};
use horizon_lattice::model::StringListComboModel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Registration Form")
        .with_size(400.0, 350.0);

    // Create labels
    let name_label = Label::new("Full Name:");
    let email_label = Label::new("Email:");
    let age_label = Label::new("Age:");
    let country_label = Label::new("Country:");
    let subscribe_label = Label::new("Newsletter:");

    // Create fields
    let mut name_field = LineEdit::new();
    name_field.set_placeholder("Enter your name");

    let mut email_field = LineEdit::new();
    email_field.set_placeholder("user@example.com");

    let age_field = SpinBox::new()
        .with_range(13, 120)
        .with_value(18);

    let countries = vec!["United States", "Canada", "Mexico", "Other"];
    let country_model = StringListComboModel::from(countries);
    let mut country_field = ComboBox::new()
        .with_model(Box::new(country_model));
    country_field.set_current_index(0);

    let subscribe_field = CheckBox::new("Yes, send me updates");

    // Create form layout
    let mut form = FormLayout::new();

    // Add label-field pairs
    form.add_row(name_label.object_id(), name_field.object_id());
    form.add_row(email_label.object_id(), email_field.object_id());
    form.add_row(age_label.object_id(), age_field.object_id());
    form.add_row(country_label.object_id(), country_field.object_id());
    form.add_row(subscribe_label.object_id(), subscribe_field.object_id());

    // Configure layout
    form.set_label_alignment(Alignment::End);  // Right-align labels
    form.set_field_growth_policy(FieldGrowthPolicy::AllNonFixedFieldsGrow);
    form.set_horizontal_spacing(12.0);
    form.set_vertical_spacing(10.0);

    // Add submit button spanning full width
    let submit = PushButton::new("Register");
    form.add_spanning_widget(submit.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(form));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### FormLayout Configuration

```rust,ignore
use horizon_lattice::widget::layout::{FormLayout, FieldGrowthPolicy, RowWrapPolicy, Alignment};

let mut form = FormLayout::new();

// Label alignment
form.set_label_alignment(Alignment::Start);  // Left-align (Windows style)
form.set_label_alignment(Alignment::End);    // Right-align (macOS style)

// Field growth policy
form.set_field_growth_policy(FieldGrowthPolicy::FieldsStayAtSizeHint);  // Fixed width
form.set_field_growth_policy(FieldGrowthPolicy::ExpandingFieldsGrow);   // Only Expanding fields grow
form.set_field_growth_policy(FieldGrowthPolicy::AllNonFixedFieldsGrow); // All non-Fixed grow

// Row wrapping (for narrow windows)
form.set_row_wrap_policy(RowWrapPolicy::DontWrapRows);  // Label beside field
form.set_row_wrap_policy(RowWrapPolicy::WrapAllRows);   // Label above field

// Spacing
form.set_horizontal_spacing(12.0);  // Between label and field
form.set_vertical_spacing(8.0);     // Between rows
```

## Step 6: Input Validation

Use validators to constrain input:

```rust,ignore
use horizon_lattice::widget::widgets::LineEdit;
use horizon_lattice::widget::validator::{IntValidator, DoubleValidator, RegexValidator};
use std::sync::Arc;

// Integer validator (e.g., for age 0-150)
let mut age_input = LineEdit::new();
age_input.set_validator(Arc::new(IntValidator::new(0, 150)));

// Double validator (e.g., for price with 2 decimals)
let mut price_input = LineEdit::new();
price_input.set_validator(Arc::new(DoubleValidator::new(0.0, 9999.99, 2)));

// Regex validator (e.g., for email pattern)
let mut email_input = LineEdit::new();
email_input.set_validator(Arc::new(RegexValidator::new(
    r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
)));

// Handle rejected input
age_input.input_rejected.connect(|| {
    println!("Invalid age entered!");
});
```

### Validation States

```rust,ignore
use horizon_lattice::widget::validator::ValidationState;

// ValidationState::Invalid      - Clearly wrong (e.g., "abc" for number)
// ValidationState::Intermediate - Could become valid (e.g., "" or "-")
// ValidationState::Acceptable   - Valid input
```

## Step 7: Input Masks

For formatted input like phone numbers:

```rust,ignore
use horizon_lattice::widget::widgets::LineEdit;

// Phone number: (999) 999-9999
let mut phone = LineEdit::new();
phone.set_input_mask("(999) 999-9999");

// Date: YYYY-MM-DD
let mut date = LineEdit::new();
date.set_input_mask("0000-00-00");

// Time: HH:MM:SS
let mut time = LineEdit::new();
time.set_input_mask("99:99:99");

// License key (uppercase)
let mut license = LineEdit::new();
license.set_input_mask(">AAAAA-AAAAA-AAAAA");
```

### Mask Characters

| Character | Description |
|-----------|-------------|
| `9` | Digit required (0-9) |
| `0` | Digit optional |
| `A` | Letter required (a-z, A-Z) |
| `a` | Letter optional |
| `N` | Alphanumeric required |
| `n` | Alphanumeric optional |
| `X` | Any character required |
| `x` | Any character optional |
| `>` | Uppercase following |
| `<` | Lowercase following |
| `\` | Escape next character |

## Complete Example: Contact Form

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Label, LineEdit, SpinBox, CheckBox, ComboBox, PushButton,
    Container, Window, ButtonVariant
};
use horizon_lattice::widget::layout::{
    FormLayout, VBoxLayout, HBoxLayout, ContentMargins,
    FieldGrowthPolicy, Alignment, LayoutKind
};
use horizon_lattice::widget::validator::RegexValidator;
use horizon_lattice::model::StringListComboModel;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Contact Form")
        .with_size(450.0, 400.0);

    // --- Create form fields ---

    // Name (required)
    let name_label = Label::new("Name: *");
    let mut name_field = LineEdit::new();
    name_field.set_placeholder("Your full name");

    // Email (with validation)
    let email_label = Label::new("Email: *");
    let mut email_field = LineEdit::new();
    email_field.set_placeholder("user@example.com");
    email_field.set_validator(Arc::new(RegexValidator::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    )));

    // Phone (with mask)
    let phone_label = Label::new("Phone:");
    let mut phone_field = LineEdit::new();
    phone_field.set_input_mask("(999) 999-9999");

    // Age
    let age_label = Label::new("Age:");
    let age_field = SpinBox::new()
        .with_range(13, 120)
        .with_value(25);

    // Subject dropdown
    let subject_label = Label::new("Subject: *");
    let subjects = vec!["General Inquiry", "Support", "Feedback", "Other"];
    let subject_model = StringListComboModel::from(subjects);
    let mut subject_field = ComboBox::new()
        .with_model(Box::new(subject_model));
    subject_field.set_current_index(0);

    // Urgent checkbox
    let urgent_label = Label::new("Priority:");
    let urgent_field = CheckBox::new("Mark as urgent");

    // Newsletter checkbox
    let newsletter_label = Label::new("Updates:");
    let newsletter_field = CheckBox::new("Subscribe to newsletter")
        .with_checked(true);

    // --- Create form layout ---

    let mut form = FormLayout::new();
    form.set_label_alignment(Alignment::End);
    form.set_field_growth_policy(FieldGrowthPolicy::AllNonFixedFieldsGrow);
    form.set_horizontal_spacing(12.0);
    form.set_vertical_spacing(10.0);

    form.add_row(name_label.object_id(), name_field.object_id());
    form.add_row(email_label.object_id(), email_field.object_id());
    form.add_row(phone_label.object_id(), phone_field.object_id());
    form.add_row(age_label.object_id(), age_field.object_id());
    form.add_row(subject_label.object_id(), subject_field.object_id());
    form.add_row(urgent_label.object_id(), urgent_field.object_id());
    form.add_row(newsletter_label.object_id(), newsletter_field.object_id());

    // --- Create buttons ---

    let submit = PushButton::new("Submit")
        .with_default(true);
    let clear = PushButton::new("Clear")
        .with_variant(ButtonVariant::Secondary);
    let cancel = PushButton::new("Cancel")
        .with_variant(ButtonVariant::Flat);

    // --- Connect signals ---

    // Clone widgets for closures
    let name_clone = name_field.clone();
    let email_clone = email_field.clone();
    let phone_clone = phone_field.clone();
    let age_clone = age_field.clone();
    let subject_clone = subject_field.clone();
    let urgent_clone = urgent_field.clone();
    let newsletter_clone = newsletter_field.clone();

    submit.clicked().connect(move |_| {
        println!("=== Form Submitted ===");
        println!("Name: {}", name_clone.text());
        println!("Email: {}", email_clone.text());
        println!("Phone: {}", phone_clone.text());
        println!("Age: {}", age_clone.value());
        println!("Subject: {}", subject_clone.current_text());
        println!("Urgent: {}", urgent_clone.is_checked());
        println!("Newsletter: {}", newsletter_clone.is_checked());
    });

    // Clone for clear button
    let name_clear = name_field.clone();
    let email_clear = email_field.clone();
    let phone_clear = phone_field.clone();

    clear.clicked().connect(move |_| {
        name_clear.set_text("");
        email_clear.set_text("");
        phone_clear.set_text("");
    });

    cancel.clicked().connect(|_| {
        println!("Cancelled");
        Application::instance().quit();
    });

    // Email validation feedback
    email_field.input_rejected.connect(|| {
        println!("Invalid email format!");
    });

    // --- Button layout ---

    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(10.0);
    button_row.add_stretch(1);  // Push buttons to right
    button_row.add_widget(cancel.object_id());
    button_row.add_widget(clear.object_id());
    button_row.add_widget(submit.object_id());

    let mut button_container = Container::new();
    button_container.set_layout(LayoutKind::from(button_row));

    // --- Main layout ---

    let mut main_layout = VBoxLayout::new();
    main_layout.set_content_margins(ContentMargins::uniform(20.0));
    main_layout.set_spacing(20.0);

    let mut form_container = Container::new();
    form_container.set_layout(LayoutKind::from(form));

    main_layout.add_widget(form_container.object_id());
    main_layout.add_widget(button_container.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(main_layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Best Practices

1. **Use FormLayout for forms** - Automatically handles label alignment
2. **Add placeholders** - Help users understand expected input
3. **Validate early** - Use validators to prevent invalid data entry
4. **Provide feedback** - Connect to `input_rejected` to show validation errors
5. **Mark required fields** - Use asterisks or other visual indicators
6. **Group related fields** - Use nested layouts or separators
7. **Default sensible values** - Pre-fill spinboxes and comboboxes
8. **Use appropriate widgets** - SpinBox for numbers, ComboBox for fixed choices

## Next Steps

- [Lists and Models](./lists.md) - Work with list views and data models
- [Custom Widgets](./custom-widget.md) - Create your own widgets
- [Widgets Guide](../guides/widgets.md) - Deep dive into the widget system
