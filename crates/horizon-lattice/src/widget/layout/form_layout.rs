//! Form layout for arranging label-field pairs.
//!
//! `FormLayout` manages forms with label-field pairs, providing automatic
//! alignment and platform-appropriate styling. It abstracts away the details
//! of form layout, allowing developers to focus on the semantic structure.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//!
//! let mut form = FormLayout::new();
//! form.add_row(name_label, name_field);
//! form.add_row(email_label, email_field);
//! form.add_spanning_widget(submit_button);
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::ContentMargins;
use super::base::LayoutBase;
use super::box_layout::Alignment;
use super::item::LayoutItem;
use super::traits::Layout;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// Controls how rows wrap in the form layout.
///
/// This policy determines whether labels and fields appear side-by-side
/// or stacked vertically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowWrapPolicy {
    /// Labels and fields are always side-by-side.
    /// This is the default for most desktop platforms.
    #[default]
    DontWrapRows,

    /// Labels get a fixed width; fields wrap below if they don't fit.
    /// Useful for narrow screens or long field content.
    WrapLongRows,

    /// Fields always appear below their labels.
    /// Common on mobile devices or when vertical space is abundant.
    WrapAllRows,
}

/// Controls how fields grow to fill available space.
///
/// Different platforms have different conventions for field sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldGrowthPolicy {
    /// Fields never expand beyond their size hint.
    /// Default on macOS (Aqua style).
    FieldsStayAtSizeHint,

    /// Only fields with Expanding size policy grow.
    /// Default on Plastique style.
    #[default]
    ExpandingFieldsGrow,

    /// All fields that aren't Fixed grow to fill space.
    /// Common on Windows and GNOME.
    AllNonFixedFieldsGrow,
}

/// Role of an item in a form layout row.
///
/// Used when querying or modifying specific parts of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormItemRole {
    /// The label column (left side).
    Label,
    /// The field column (right side).
    Field,
    /// A widget spanning both columns.
    Spanning,
}

/// A row in the form layout.
///
/// Each row can be either a label-field pair or a spanning widget.
#[derive(Debug, Clone)]
pub enum FormRow {
    /// A label-field pair.
    LabelField {
        /// The label item (typically a text label widget).
        label: LayoutItem,
        /// The field item (input widget, combo box, etc.).
        field: LayoutItem,
    },
    /// A widget or layout spanning both columns.
    Spanning {
        /// The spanning item.
        item: LayoutItem,
    },
}

impl FormRow {
    /// Check if this row has a label (i.e., is a LabelField row).
    pub fn has_label(&self) -> bool {
        matches!(self, FormRow::LabelField { .. })
    }

    /// Get the label item if this is a LabelField row.
    pub fn label(&self) -> Option<&LayoutItem> {
        match self {
            FormRow::LabelField { label, .. } => Some(label),
            FormRow::Spanning { .. } => None,
        }
    }

    /// Get the field item if this is a LabelField row.
    pub fn field(&self) -> Option<&LayoutItem> {
        match self {
            FormRow::LabelField { field, .. } => Some(field),
            FormRow::Spanning { .. } => None,
        }
    }

    /// Get the spanning item if this is a Spanning row.
    pub fn spanning_item(&self) -> Option<&LayoutItem> {
        match self {
            FormRow::LabelField { .. } => None,
            FormRow::Spanning { item } => Some(item),
        }
    }

    /// Get all widget IDs in this row.
    pub fn widget_ids(&self) -> Vec<ObjectId> {
        match self {
            FormRow::LabelField { label, field } => {
                let mut ids = Vec::new();
                if let Some(id) = label.widget_id() {
                    ids.push(id);
                }
                if let Some(id) = field.widget_id() {
                    ids.push(id);
                }
                ids
            }
            FormRow::Spanning { item } => {
                if let Some(id) = item.widget_id() {
                    vec![id]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

/// A form layout that arranges label-field pairs.
///
/// `FormLayout` provides a convenient way to lay out forms with labels
/// on the left and corresponding input fields on the right. It handles:
///
/// - Automatic label column width calculation
/// - Platform-appropriate label alignment
/// - Field growth policies
/// - Row wrapping for narrow displays
/// - Spanning widgets for buttons or grouped controls
///
/// # Layout Algorithm
///
/// 1. Calculate the maximum label width across all rows
/// 2. Assign field widths based on the growth policy
/// 3. Position rows vertically with spacing
/// 4. Apply label alignment within the label column
///
/// # Example
///
/// ```ignore
/// let mut form = FormLayout::new();
///
/// // Add label-field pairs
/// form.add_row(username_label, username_input);
/// form.add_row(password_label, password_input);
///
/// // Add a spanning button at the bottom
/// form.add_spanning_widget(submit_button);
/// ```
#[derive(Debug, Clone)]
pub struct FormLayout {
    /// Common layout functionality.
    base: LayoutBase,

    /// Form rows (label-field pairs or spanning items).
    rows: Vec<FormRow>,

    /// Calculated geometries for labels.
    label_geometries: Vec<Option<Rect>>,

    /// Calculated geometries for fields/spanning items.
    field_geometries: Vec<Rect>,

    /// How labels are aligned within the label column.
    label_alignment: Alignment,

    /// How the form is aligned within available space.
    form_alignment: Alignment,

    /// How fields grow to fill available space.
    field_growth_policy: FieldGrowthPolicy,

    /// How rows wrap.
    row_wrap_policy: RowWrapPolicy,

    /// Horizontal spacing between labels and fields.
    horizontal_spacing: f32,

    /// Vertical spacing between rows.
    vertical_spacing: f32,
}

impl FormLayout {
    /// Create a new form layout with default settings.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
            rows: Vec::new(),
            label_geometries: Vec::new(),
            field_geometries: Vec::new(),
            label_alignment: Alignment::End, // Right-align labels by default
            form_alignment: Alignment::Start,
            field_growth_policy: FieldGrowthPolicy::default(),
            row_wrap_policy: RowWrapPolicy::default(),
            horizontal_spacing: 12.0, // Space between label and field
            vertical_spacing: 8.0,    // Space between rows
        }
    }

    /// Get a reference to the underlying layout base.
    #[inline]
    pub fn base(&self) -> &LayoutBase {
        &self.base
    }

    /// Get a mutable reference to the underlying layout base.
    #[inline]
    pub fn base_mut(&mut self) -> &mut LayoutBase {
        &mut self.base
    }

    // =========================================================================
    // Row Management
    // =========================================================================

    /// Add a label-field row using widget IDs.
    ///
    /// The label is typically a text label, and the field is an input widget.
    pub fn add_row(&mut self, label: ObjectId, field: ObjectId) {
        self.add_row_items(LayoutItem::Widget(label), LayoutItem::Widget(field));
    }

    /// Add a label-field row using layout items.
    ///
    /// This allows using spacers or nested layouts as labels or fields.
    pub fn add_row_items(&mut self, label: LayoutItem, field: LayoutItem) {
        // Add items to base for proper tracking
        self.base.add_item(label.clone());
        self.base.add_item(field.clone());

        self.rows.push(FormRow::LabelField { label, field });
        self.label_geometries.push(Some(Rect::ZERO));
        self.field_geometries.push(Rect::ZERO);
        self.base.invalidate();
    }

    /// Add a widget that spans both columns.
    ///
    /// Useful for buttons, separators, or grouped controls.
    pub fn add_spanning_widget(&mut self, widget: ObjectId) {
        self.add_spanning_item(LayoutItem::Widget(widget));
    }

    /// Add an item that spans both columns.
    pub fn add_spanning_item(&mut self, item: LayoutItem) {
        self.base.add_item(item.clone());
        self.rows.push(FormRow::Spanning { item });
        self.label_geometries.push(None);
        self.field_geometries.push(Rect::ZERO);
        self.base.invalidate();
    }

    /// Insert a label-field row at a specific position.
    pub fn insert_row(&mut self, row_index: usize, label: ObjectId, field: ObjectId) {
        let label_item = LayoutItem::Widget(label);
        let field_item = LayoutItem::Widget(field);

        // Insert items into base at appropriate positions
        let base_index = self.row_to_base_index(row_index);
        self.base.insert_item(base_index, label_item.clone());
        self.base.insert_item(base_index + 1, field_item.clone());

        self.rows.insert(
            row_index,
            FormRow::LabelField {
                label: label_item,
                field: field_item,
            },
        );
        self.label_geometries.insert(row_index, Some(Rect::ZERO));
        self.field_geometries.insert(row_index, Rect::ZERO);
        self.base.invalidate();
    }

    /// Remove a row at the specified index.
    ///
    /// Returns the removed row, or None if the index is out of bounds.
    pub fn remove_row(&mut self, row_index: usize) -> Option<FormRow> {
        if row_index >= self.rows.len() {
            return None;
        }

        // Remove items from base
        let base_index = self.row_to_base_index(row_index);
        let row = &self.rows[row_index];
        match row {
            FormRow::LabelField { .. } => {
                self.base.remove_item(base_index + 1);
                self.base.remove_item(base_index);
            }
            FormRow::Spanning { .. } => {
                self.base.remove_item(base_index);
            }
        }

        self.label_geometries.remove(row_index);
        self.field_geometries.remove(row_index);
        let removed = self.rows.remove(row_index);
        self.base.invalidate();
        Some(removed)
    }

    /// Get the number of rows in the form.
    #[inline]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get a row by index.
    pub fn row_at(&self, index: usize) -> Option<&FormRow> {
        self.rows.get(index)
    }

    /// Check if the form is empty.
    #[inline]
    pub fn is_form_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get the item in a specific role within a row.
    pub fn item_at(&self, row: usize, role: FormItemRole) -> Option<&LayoutItem> {
        self.rows.get(row).and_then(|r| match role {
            FormItemRole::Label => r.label(),
            FormItemRole::Field => r.field(),
            FormItemRole::Spanning => r.spanning_item(),
        })
    }

    // =========================================================================
    // Configuration
    // =========================================================================

    /// Get the label alignment.
    #[inline]
    pub fn label_alignment(&self) -> Alignment {
        self.label_alignment
    }

    /// Set the label alignment within the label column.
    ///
    /// Common values:
    /// - `Alignment::End` (right-align) - macOS Aqua, KDE
    /// - `Alignment::Start` (left-align) - Windows, GNOME
    pub fn set_label_alignment(&mut self, alignment: Alignment) {
        if self.label_alignment != alignment {
            self.label_alignment = alignment;
            self.base.invalidate();
        }
    }

    /// Get the form alignment.
    #[inline]
    pub fn form_alignment(&self) -> Alignment {
        self.form_alignment
    }

    /// Set how the form is aligned within available space.
    pub fn set_form_alignment(&mut self, alignment: Alignment) {
        if self.form_alignment != alignment {
            self.form_alignment = alignment;
            self.base.invalidate();
        }
    }

    /// Get the field growth policy.
    #[inline]
    pub fn field_growth_policy(&self) -> FieldGrowthPolicy {
        self.field_growth_policy
    }

    /// Set the field growth policy.
    pub fn set_field_growth_policy(&mut self, policy: FieldGrowthPolicy) {
        if self.field_growth_policy != policy {
            self.field_growth_policy = policy;
            self.base.invalidate();
        }
    }

    /// Get the row wrap policy.
    #[inline]
    pub fn row_wrap_policy(&self) -> RowWrapPolicy {
        self.row_wrap_policy
    }

    /// Set the row wrap policy.
    pub fn set_row_wrap_policy(&mut self, policy: RowWrapPolicy) {
        if self.row_wrap_policy != policy {
            self.row_wrap_policy = policy;
            self.base.invalidate();
        }
    }

    /// Get the horizontal spacing (between label and field).
    #[inline]
    pub fn horizontal_spacing(&self) -> f32 {
        self.horizontal_spacing
    }

    /// Set the horizontal spacing between label and field.
    pub fn set_horizontal_spacing(&mut self, spacing: f32) {
        if (self.horizontal_spacing - spacing).abs() > f32::EPSILON {
            self.horizontal_spacing = spacing;
            self.base.invalidate();
        }
    }

    /// Get the vertical spacing (between rows).
    #[inline]
    pub fn vertical_spacing(&self) -> f32 {
        self.vertical_spacing
    }

    /// Set the vertical spacing between rows.
    pub fn set_vertical_spacing(&mut self, spacing: f32) {
        if (self.vertical_spacing - spacing).abs() > f32::EPSILON {
            self.vertical_spacing = spacing;
            self.base.invalidate();
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Convert a row index to a base item index.
    fn row_to_base_index(&self, row_index: usize) -> usize {
        let mut base_idx = 0;
        for row in self.rows.iter().take(row_index) {
            match row {
                FormRow::LabelField { .. } => base_idx += 2,
                FormRow::Spanning { .. } => base_idx += 1,
            }
        }
        base_idx
    }

    /// Calculate the maximum label width across all rows.
    fn calculate_label_width<S: WidgetAccess>(&self, storage: &S) -> f32 {
        let mut max_width: f32 = 0.0;

        for row in &self.rows {
            if let FormRow::LabelField { label, .. } = row
                && self.base.is_item_visible(storage, label)
            {
                let hint = self.base.get_item_size_hint(storage, label);
                max_width = max_width.max(hint.preferred.width);
            }
        }

        max_width
    }

    /// Check if a field should grow based on the current policy.
    fn should_field_grow<S: WidgetAccess>(&self, storage: &S, field: &LayoutItem) -> bool {
        let policy = self.base.get_item_size_policy(storage, field);
        match self.field_growth_policy {
            FieldGrowthPolicy::FieldsStayAtSizeHint => false,
            FieldGrowthPolicy::ExpandingFieldsGrow => policy.horizontal.wants_to_grow(),
            FieldGrowthPolicy::AllNonFixedFieldsGrow => policy.horizontal != SizePolicy::Fixed,
        }
    }

    /// Calculate the row height for a label-field row.
    fn calculate_row_height<S: WidgetAccess>(
        &self,
        storage: &S,
        label: &LayoutItem,
        field: &LayoutItem,
        wrap: bool,
    ) -> f32 {
        let label_hint = self.base.get_item_size_hint(storage, label);
        let field_hint = self.base.get_item_size_hint(storage, field);

        if wrap {
            // Stacked: label height + spacing + field height
            label_hint.preferred.height + self.vertical_spacing + field_hint.preferred.height
        } else {
            // Side by side: max of both heights
            label_hint.preferred.height.max(field_hint.preferred.height)
        }
    }

    /// Calculate the aggregate size hint for the layout.
    fn calculate_size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        let label_width = self.calculate_label_width(storage);
        let margins = self.base.content_margins();
        let wrap_all = self.row_wrap_policy == RowWrapPolicy::WrapAllRows;

        let mut total_height: f32 = 0.0;
        let mut max_field_width: f32 = 0.0;
        let mut visible_count = 0;

        for row in &self.rows {
            let (row_height, field_width) = match row {
                FormRow::LabelField { label, field } => {
                    if !self.base.is_item_visible(storage, label)
                        && !self.base.is_item_visible(storage, field)
                    {
                        continue;
                    }
                    let height = self.calculate_row_height(storage, label, field, wrap_all);
                    let field_hint = self.base.get_item_size_hint(storage, field);
                    (height, field_hint.preferred.width)
                }
                FormRow::Spanning { item } => {
                    if !self.base.is_item_visible(storage, item) {
                        continue;
                    }
                    let hint = self.base.get_item_size_hint(storage, item);
                    (hint.preferred.height, hint.preferred.width)
                }
            };

            total_height += row_height;
            max_field_width = max_field_width.max(field_width);
            visible_count += 1;
        }

        // Add vertical spacing between rows
        if visible_count > 1 {
            total_height += self.vertical_spacing * (visible_count - 1) as f32;
        }

        // Calculate total width
        let content_width = if wrap_all {
            // Wrapped: width is max of label width and field width
            label_width.max(max_field_width)
        } else {
            // Side by side: label + spacing + field
            label_width + self.horizontal_spacing + max_field_width
        };

        let total_width = content_width + margins.horizontal();
        total_height += margins.vertical();

        SizeHint::new(Size::new(total_width, total_height))
    }
}

impl Layout for FormLayout {
    // =========================================================================
    // Item Management - Delegate to LayoutBase with form-specific logic
    // =========================================================================

    fn add_item(&mut self, item: LayoutItem) {
        // Default: add as spanning item
        self.add_spanning_item(item);
    }

    fn insert_item(&mut self, index: usize, item: LayoutItem) {
        // For raw item insertion, add as spanning at the corresponding row
        let row_index = self.base_index_to_row(index);
        self.rows
            .insert(row_index, FormRow::Spanning { item: item.clone() });
        self.label_geometries.insert(row_index, None);
        self.field_geometries.insert(row_index, Rect::ZERO);
        self.base.insert_item(index, item);
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        // Find which row this index belongs to
        let row_index = self.base_index_to_row(index);
        if row_index < self.rows.len() {
            self.remove_row(row_index);
            // Return the first item of the removed row
            self.base.remove_item(index)
        } else {
            None
        }
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        // Find row containing this widget
        for (row_idx, row) in self.rows.iter().enumerate() {
            let contains = match row {
                FormRow::LabelField { label, field } => {
                    label.widget_id() == Some(widget) || field.widget_id() == Some(widget)
                }
                FormRow::Spanning { item } => item.widget_id() == Some(widget),
            };
            if contains {
                self.remove_row(row_idx);
                return true;
            }
        }
        false
    }

    fn item_count(&self) -> usize {
        self.base.item_count()
    }

    fn item_at(&self, index: usize) -> Option<&LayoutItem> {
        self.base.item_at(index)
    }

    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> {
        self.base.item_at_mut(index)
    }

    fn clear(&mut self) {
        self.rows.clear();
        self.label_geometries.clear();
        self.field_geometries.clear();
        self.base.clear();
    }

    // =========================================================================
    // Size Hints & Policies
    // =========================================================================

    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        if let Some(cached) = self.base.cached_size_hint() {
            return cached;
        }
        self.calculate_size_hint(storage)
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        if let Some(cached) = self.base.cached_minimum_size() {
            return cached;
        }
        self.size_hint(storage).effective_minimum()
    }

    fn size_policy(&self) -> SizePolicyPair {
        SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
    }

    // =========================================================================
    // Geometry & Margins - Delegate to LayoutBase
    // =========================================================================

    fn geometry(&self) -> Rect {
        self.base.geometry()
    }

    fn set_geometry(&mut self, rect: Rect) {
        self.base.set_geometry(rect);
    }

    fn content_margins(&self) -> ContentMargins {
        self.base.content_margins()
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.base.set_content_margins(margins);
    }

    fn spacing(&self) -> f32 {
        self.vertical_spacing
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.set_vertical_spacing(spacing);
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, _available: Size) -> Size {
        let content_rect = self.base.content_rect();
        let content_width = content_rect.width();
        let wrap_all = self.row_wrap_policy == RowWrapPolicy::WrapAllRows;

        // Calculate label column width
        let label_width = self.calculate_label_width(storage);

        // Calculate field column width
        let field_width = if wrap_all {
            content_width
        } else {
            (content_width - label_width - self.horizontal_spacing).max(0.0)
        };

        let mut y_pos = content_rect.origin.y;

        for (row_idx, row) in self.rows.iter().enumerate() {
            match row {
                FormRow::LabelField { label, field } => {
                    if !self.base.is_item_visible(storage, label)
                        && !self.base.is_item_visible(storage, field)
                    {
                        continue;
                    }

                    let label_hint = self.base.get_item_size_hint(storage, label);
                    let field_hint = self.base.get_item_size_hint(storage, field);

                    if wrap_all {
                        // Stacked layout: label above field

                        // Label geometry
                        let label_height = label_hint.preferred.height;
                        let label_x = match self.label_alignment {
                            Alignment::Start => content_rect.origin.x,
                            Alignment::End => {
                                content_rect.origin.x + content_width - label_hint.preferred.width
                            }
                            Alignment::Center => {
                                content_rect.origin.x
                                    + (content_width - label_hint.preferred.width) / 2.0
                            }
                            Alignment::Stretch => content_rect.origin.x,
                        };
                        let label_w = if self.label_alignment == Alignment::Stretch {
                            content_width
                        } else {
                            label_hint.preferred.width
                        };

                        self.label_geometries[row_idx] =
                            Some(Rect::new(label_x, y_pos, label_w, label_height));

                        y_pos += label_height + self.vertical_spacing;

                        // Field geometry (full width)
                        let field_height = field_hint.preferred.height;
                        let actual_field_width = if self.should_field_grow(storage, field) {
                            content_width
                        } else {
                            field_hint.preferred.width.min(content_width)
                        };

                        self.field_geometries[row_idx] = Rect::new(
                            content_rect.origin.x,
                            y_pos,
                            actual_field_width,
                            field_height,
                        );

                        y_pos += field_height;
                    } else {
                        // Side-by-side layout

                        let row_height =
                            label_hint.preferred.height.max(field_hint.preferred.height);

                        // Label geometry (aligned within label column)
                        let label_x = match self.label_alignment {
                            Alignment::Start => content_rect.origin.x,
                            Alignment::End => {
                                content_rect.origin.x + label_width - label_hint.preferred.width
                            }
                            Alignment::Center => {
                                content_rect.origin.x
                                    + (label_width - label_hint.preferred.width) / 2.0
                            }
                            Alignment::Stretch => content_rect.origin.x,
                        };
                        let label_w = if self.label_alignment == Alignment::Stretch {
                            label_width
                        } else {
                            label_hint.preferred.width
                        };

                        // Vertically center label in row
                        let label_y = y_pos + (row_height - label_hint.preferred.height) / 2.0;

                        self.label_geometries[row_idx] = Some(Rect::new(
                            label_x,
                            label_y,
                            label_w,
                            label_hint.preferred.height,
                        ));

                        // Field geometry
                        let field_x = content_rect.origin.x + label_width + self.horizontal_spacing;
                        let actual_field_width = if self.should_field_grow(storage, field) {
                            field_width
                        } else {
                            field_hint.preferred.width.min(field_width)
                        };

                        // Vertically center field in row
                        let field_y = y_pos + (row_height - field_hint.preferred.height) / 2.0;

                        self.field_geometries[row_idx] = Rect::new(
                            field_x,
                            field_y,
                            actual_field_width,
                            field_hint.preferred.height,
                        );

                        y_pos += row_height;
                    }
                }
                FormRow::Spanning { item } => {
                    if !self.base.is_item_visible(storage, item) {
                        continue;
                    }

                    let hint = self.base.get_item_size_hint(storage, item);
                    let actual_width = if self.should_field_grow(storage, item) {
                        content_width
                    } else {
                        hint.preferred.width.min(content_width)
                    };

                    self.label_geometries[row_idx] = None;
                    self.field_geometries[row_idx] = Rect::new(
                        content_rect.origin.x,
                        y_pos,
                        actual_width,
                        hint.preferred.height,
                    );

                    y_pos += hint.preferred.height;
                }
            }

            // Add vertical spacing between rows
            y_pos += self.vertical_spacing;
        }

        // Cache the calculated size hint
        let size_hint = self.calculate_size_hint(storage);
        self.base.set_cached_size_hint(size_hint);
        self.base
            .set_cached_minimum_size(size_hint.effective_minimum());

        self.base.mark_valid();
        self.base.geometry().size
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (row_idx, row) in self.rows.iter().enumerate() {
            match row {
                FormRow::LabelField { label, field } => {
                    if let Some(label_geo) = self.label_geometries[row_idx] {
                        LayoutBase::apply_item_geometry(storage, label, label_geo);
                    }
                    LayoutBase::apply_item_geometry(storage, field, self.field_geometries[row_idx]);
                }
                FormRow::Spanning { item } => {
                    LayoutBase::apply_item_geometry(storage, item, self.field_geometries[row_idx]);
                }
            }
        }
    }

    // =========================================================================
    // Invalidation - Delegate to LayoutBase
    // =========================================================================

    fn invalidate(&mut self) {
        self.base.invalidate();
    }

    fn needs_recalculation(&self) -> bool {
        self.base.needs_recalculation()
    }

    // =========================================================================
    // Ownership - Delegate to LayoutBase
    // =========================================================================

    fn parent_widget(&self) -> Option<ObjectId> {
        self.base.parent_widget()
    }

    fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.base.set_parent_widget(parent);
    }
}

impl FormLayout {
    /// Convert a base item index to a row index.
    fn base_index_to_row(&self, base_index: usize) -> usize {
        let mut current_base_idx = 0;
        for (row_idx, row) in self.rows.iter().enumerate() {
            let items_in_row = match row {
                FormRow::LabelField { .. } => 2,
                FormRow::Spanning { .. } => 1,
            };
            if base_index < current_base_idx + items_in_row {
                return row_idx;
            }
            current_base_idx += items_in_row;
        }
        self.rows.len()
    }
}

impl Default for FormLayout {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::geometry::SizeHint;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{Object, ObjectId, init_global_registry};
    use std::collections::HashMap;

    /// Mock widget for testing layouts.
    struct MockWidget {
        base: WidgetBase,
        mock_size_hint: SizeHint,
    }

    impl MockWidget {
        fn new(size_hint: SizeHint) -> Self {
            Self {
                base: WidgetBase::new::<Self>(),
                mock_size_hint: size_hint,
            }
        }
    }

    impl Object for MockWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for MockWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            self.mock_size_hint
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}
    }

    /// Mock widget storage for testing.
    struct MockStorage {
        widgets: HashMap<ObjectId, MockWidget>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                widgets: HashMap::new(),
            }
        }

        fn add(&mut self, widget: MockWidget) -> ObjectId {
            let id = widget.object_id();
            self.widgets.insert(id, widget);
            id
        }
    }

    impl WidgetAccess for MockStorage {
        fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget> {
            self.widgets.get(&id).map(|w| w as &dyn Widget)
        }

        fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(&id).map(|w| w as &mut dyn Widget)
        }
    }

    #[test]
    fn test_form_layout_creation() {
        init_global_registry();

        let form = FormLayout::new();
        assert_eq!(form.row_count(), 0);
        assert!(form.is_form_empty());
        assert_eq!(form.label_alignment(), Alignment::End);
        assert_eq!(
            form.field_growth_policy(),
            FieldGrowthPolicy::ExpandingFieldsGrow
        );
    }

    #[test]
    fn test_form_layout_add_row() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.add_row(label, field);

        assert_eq!(form.row_count(), 1);
        assert!(!form.is_form_empty());
        assert!(form.row_at(0).unwrap().has_label());
    }

    #[test]
    fn test_form_layout_add_spanning() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let button = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));

        let mut form = FormLayout::new();
        form.add_spanning_widget(button);

        assert_eq!(form.row_count(), 1);
        assert!(!form.row_at(0).unwrap().has_label());
    }

    #[test]
    fn test_form_layout_remove_row() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label1 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field1 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));
        let label2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.add_row(label1, field1);
        form.add_row(label2, field2);

        assert_eq!(form.row_count(), 2);

        form.remove_row(0);
        assert_eq!(form.row_count(), 1);
    }

    #[test]
    fn test_form_layout_size_hint() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.set_content_margins(ContentMargins::uniform(0.0));
        form.add_row(label, field);

        let hint = form.size_hint(&storage);
        // Width = label(80) + spacing(12) + field(150) = 242
        assert_eq!(hint.preferred.width, 242.0);
        // Height = max(20, 25) = 25
        assert_eq!(hint.preferred.height, 25.0);
    }

    #[test]
    fn test_form_layout_calculate_and_apply() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.set_content_margins(ContentMargins::uniform(0.0));
        form.set_label_alignment(Alignment::End);
        form.add_row(label, field);

        form.set_geometry(Rect::new(0.0, 0.0, 300.0, 50.0));
        form.calculate(&storage, Size::new(300.0, 50.0));
        form.apply(&mut storage);

        let label_widget = storage.widgets.get(&label).unwrap();
        let field_widget = storage.widgets.get(&field).unwrap();

        // Label should be right-aligned in its column
        assert!(label_widget.geometry().origin.x >= 0.0);
        // Field should start after label column + spacing
        assert!(field_widget.geometry().origin.x >= 80.0 + 12.0);
    }

    #[test]
    fn test_form_layout_wrap_all_rows() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.set_content_margins(ContentMargins::uniform(0.0));
        form.set_row_wrap_policy(RowWrapPolicy::WrapAllRows);
        form.add_row(label, field);

        let hint = form.size_hint(&storage);
        // Width = max(label, field) = 150
        assert_eq!(hint.preferred.width, 150.0);
        // Height = label(20) + spacing(8) + field(25) = 53
        assert_eq!(hint.preferred.height, 53.0);
    }

    #[test]
    fn test_form_row_widget_ids() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let label = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 20.0))));
        let field = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 25.0))));

        let mut form = FormLayout::new();
        form.add_row(label, field);

        let row = form.row_at(0).unwrap();
        let ids = row.widget_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&label));
        assert!(ids.contains(&field));
    }

    #[test]
    fn test_label_alignment_options() {
        init_global_registry();

        let mut form = FormLayout::new();

        form.set_label_alignment(Alignment::Start);
        assert_eq!(form.label_alignment(), Alignment::Start);

        form.set_label_alignment(Alignment::End);
        assert_eq!(form.label_alignment(), Alignment::End);

        form.set_label_alignment(Alignment::Center);
        assert_eq!(form.label_alignment(), Alignment::Center);
    }

    #[test]
    fn test_field_growth_policy_options() {
        init_global_registry();

        let mut form = FormLayout::new();

        form.set_field_growth_policy(FieldGrowthPolicy::FieldsStayAtSizeHint);
        assert_eq!(
            form.field_growth_policy(),
            FieldGrowthPolicy::FieldsStayAtSizeHint
        );

        form.set_field_growth_policy(FieldGrowthPolicy::AllNonFixedFieldsGrow);
        assert_eq!(
            form.field_growth_policy(),
            FieldGrowthPolicy::AllNonFixedFieldsGrow
        );
    }
}
