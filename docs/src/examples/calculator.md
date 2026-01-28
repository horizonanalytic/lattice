# Example: Calculator

A functional calculator demonstrating button grids, state management, and signal handling.

## Overview

This example builds a basic calculator with:
- 4x5 button grid for digits and operations
- Display label showing current value
- Keyboard input support
- Basic arithmetic operations (+, -, *, /)

## Key Concepts

- **GridLayout**: Arranging buttons in a 2D grid
- **Signal connections**: Handling button clicks
- **State management**: Tracking calculator state with Arc/Mutex
- **Keyboard events**: Accepting keyboard input

## Implementation

### Calculator State

```rust,ignore
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct CalculatorState {
    display: String,
    operand: Option<f64>,
    operator: Option<char>,
    clear_on_next: bool,
}

impl CalculatorState {
    fn new() -> Self {
        Self {
            display: "0".to_string(),
            operand: None,
            operator: None,
            clear_on_next: false,
        }
    }

    fn input_digit(&mut self, digit: char) {
        if self.clear_on_next {
            self.display = String::new();
            self.clear_on_next = false;
        }
        if self.display == "0" && digit != '.' {
            self.display = digit.to_string();
        } else if digit == '.' && self.display.contains('.') {
            // Ignore duplicate decimal
        } else {
            self.display.push(digit);
        }
    }

    fn input_operator(&mut self, op: char) {
        let current = self.display.parse::<f64>().unwrap_or(0.0);

        if let (Some(operand), Some(prev_op)) = (self.operand, self.operator) {
            let result = Self::calculate(operand, current, prev_op);
            self.display = Self::format_result(result);
            self.operand = Some(result);
        } else {
            self.operand = Some(current);
        }

        self.operator = Some(op);
        self.clear_on_next = true;
    }

    fn calculate(a: f64, b: f64, op: char) -> f64 {
        match op {
            '+' => a + b,
            '-' => a - b,
            '*' => a * b,
            '/' => if b != 0.0 { a / b } else { f64::NAN },
            _ => b,
        }
    }

    fn equals(&mut self) {
        if let (Some(operand), Some(op)) = (self.operand, self.operator) {
            let current = self.display.parse::<f64>().unwrap_or(0.0);
            let result = Self::calculate(operand, current, op);
            self.display = Self::format_result(result);
            self.operand = None;
            self.operator = None;
            self.clear_on_next = true;
        }
    }

    fn clear(&mut self) {
        self.display = "0".to_string();
        self.operand = None;
        self.operator = None;
        self.clear_on_next = false;
    }

    fn format_result(value: f64) -> String {
        if value.is_nan() {
            "Error".to_string()
        } else if value.fract() == 0.0 && value.abs() < 1e10 {
            format!("{:.0}", value)
        } else {
            format!("{:.8}", value).trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }
}
```

### Button Factory

```rust,ignore
use horizon_lattice::widget::widgets::{PushButton, ButtonVariant};
use horizon_lattice::render::Color;

fn create_digit_button(digit: &str) -> PushButton {
    PushButton::new(digit)
        .with_variant(ButtonVariant::Secondary)
}

fn create_operator_button(op: &str) -> PushButton {
    PushButton::new(op)
        .with_variant(ButtonVariant::Primary)
}

fn create_special_button(text: &str) -> PushButton {
    PushButton::new(text)
        .with_variant(ButtonVariant::Outlined)
}
```

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Window, Container, Label, PushButton, ButtonVariant
};
use horizon_lattice::widget::layout::{GridLayout, VBoxLayout, ContentMargins, LayoutKind};
use horizon_lattice::widget::{Widget, WidgetEvent, Key};
use horizon_lattice::render::Color;
use horizon_lattice_style::{Style, LengthValue};
use std::sync::{Arc, Mutex};

// Calculator state (from above)
#[derive(Clone)]
struct CalculatorState {
    display: String,
    operand: Option<f64>,
    operator: Option<char>,
    clear_on_next: bool,
}

impl CalculatorState {
    fn new() -> Self {
        Self {
            display: "0".to_string(),
            operand: None,
            operator: None,
            clear_on_next: false,
        }
    }

    fn input_digit(&mut self, digit: char) {
        if self.clear_on_next {
            self.display = String::new();
            self.clear_on_next = false;
        }
        if self.display == "0" && digit != '.' {
            self.display = digit.to_string();
        } else if digit == '.' && self.display.contains('.') {
            // Ignore duplicate decimal
        } else {
            self.display.push(digit);
        }
    }

    fn input_operator(&mut self, op: char) {
        let current = self.display.parse::<f64>().unwrap_or(0.0);
        if let (Some(operand), Some(prev_op)) = (self.operand, self.operator) {
            let result = Self::calculate(operand, current, prev_op);
            self.display = Self::format_result(result);
            self.operand = Some(result);
        } else {
            self.operand = Some(current);
        }
        self.operator = Some(op);
        self.clear_on_next = true;
    }

    fn calculate(a: f64, b: f64, op: char) -> f64 {
        match op {
            '+' => a + b,
            '-' => a - b,
            '*' => a * b,
            '/' => if b != 0.0 { a / b } else { f64::NAN },
            _ => b,
        }
    }

    fn equals(&mut self) {
        if let (Some(operand), Some(op)) = (self.operand, self.operator) {
            let current = self.display.parse::<f64>().unwrap_or(0.0);
            let result = Self::calculate(operand, current, op);
            self.display = Self::format_result(result);
            self.operand = None;
            self.operator = None;
            self.clear_on_next = true;
        }
    }

    fn clear(&mut self) {
        self.display = "0".to_string();
        self.operand = None;
        self.operator = None;
        self.clear_on_next = false;
    }

    fn format_result(value: f64) -> String {
        if value.is_nan() {
            "Error".to_string()
        } else if value.fract() == 0.0 && value.abs() < 1e10 {
            format!("{:.0}", value)
        } else {
            format!("{:.8}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Calculator")
        .with_size(280.0, 400.0);

    // Shared state
    let state = Arc::new(Mutex::new(CalculatorState::new()));

    // Display label
    let mut display = Label::new("0");
    display.set_style(
        Style::new()
            .font_size(LengthValue::Px(32.0))
            .padding_all(LengthValue::Px(16.0))
            .background_color(Color::from_rgb8(40, 40, 40))
            .color(Color::WHITE)
            .build()
    );

    // Create buttons
    let buttons = [
        ("C", 0, 0, ButtonVariant::Outlined),
        ("+/-", 0, 1, ButtonVariant::Outlined),
        ("%", 0, 2, ButtonVariant::Outlined),
        ("/", 0, 3, ButtonVariant::Primary),
        ("7", 1, 0, ButtonVariant::Secondary),
        ("8", 1, 1, ButtonVariant::Secondary),
        ("9", 1, 2, ButtonVariant::Secondary),
        ("*", 1, 3, ButtonVariant::Primary),
        ("4", 2, 0, ButtonVariant::Secondary),
        ("5", 2, 1, ButtonVariant::Secondary),
        ("6", 2, 2, ButtonVariant::Secondary),
        ("-", 2, 3, ButtonVariant::Primary),
        ("1", 3, 0, ButtonVariant::Secondary),
        ("2", 3, 1, ButtonVariant::Secondary),
        ("3", 3, 2, ButtonVariant::Secondary),
        ("+", 3, 3, ButtonVariant::Primary),
        ("0", 4, 0, ButtonVariant::Secondary), // Will span 2 columns
        (".", 4, 2, ButtonVariant::Secondary),
        ("=", 4, 3, ButtonVariant::Primary),
    ];

    // Build grid layout
    let mut grid = GridLayout::new();
    grid.set_horizontal_spacing(4.0);
    grid.set_vertical_spacing(4.0);

    for (text, row, col, variant) in buttons {
        let button = PushButton::new(text).with_variant(variant);

        // Connect button to calculator logic
        let state_clone = state.clone();
        let display_clone = display.clone();
        let text_owned = text.to_string();

        button.clicked().connect(move |_| {
            let mut calc = state_clone.lock().unwrap();
            let ch = text_owned.chars().next().unwrap();

            match text_owned.as_str() {
                "C" => calc.clear(),
                "=" => calc.equals(),
                "+/-" => {
                    if let Ok(val) = calc.display.parse::<f64>() {
                        calc.display = CalculatorState::format_result(-val);
                    }
                }
                "%" => {
                    if let Ok(val) = calc.display.parse::<f64>() {
                        calc.display = CalculatorState::format_result(val / 100.0);
                    }
                }
                "+" | "-" | "*" | "/" => calc.input_operator(ch),
                _ => calc.input_digit(ch),
            }

            display_clone.set_text(&calc.display);
        });

        // Special case: "0" spans 2 columns
        if text == "0" {
            grid.add_widget_spanning(button.object_id(), row, col, 1, 2);
        } else {
            grid.add_widget_at(button.object_id(), row, col);
        }
    }

    let mut grid_container = Container::new();
    grid_container.set_layout(LayoutKind::from(grid));

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(8.0));
    layout.set_spacing(8.0);
    layout.add_widget(display.object_id());
    layout.add_widget(grid_container.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Features Demonstrated

| Feature | Description |
|---------|-------------|
| **GridLayout** | 4-column button grid with cell spanning |
| **State Management** | Arc/Mutex for shared calculator state |
| **Signal Connections** | Button clicks update display |
| **Button Variants** | Visual distinction between button types |
| **Inline Styling** | Custom display label styling |
| **Builder Pattern** | Fluent widget configuration |

## Exercises

1. **Add keyboard support**: Handle number keys, operators, Enter for equals, Escape for clear
2. **Add memory functions**: M+, M-, MR, MC buttons
3. **Add scientific functions**: sin, cos, tan, sqrt, power
4. **Add history**: Show previous calculations
5. **Add parentheses**: Support expression grouping

## Related Examples

- [Settings Dialog](./settings-dialog.md) - More complex layout patterns
- [Text Editor](./text-editor.md) - Keyboard input handling
