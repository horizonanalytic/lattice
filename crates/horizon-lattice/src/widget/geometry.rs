//! Size hints and size policies for widget layout.
//!
//! This module provides the types used for layout negotiation between widgets
//! and their parent layouts, inspired by Qt's QSizePolicy system.

use horizon_lattice_render::Size;

/// Size policy determines how a widget should behave when space is allocated.
///
/// This is similar to Qt's `QSizePolicy::Policy`. The policy tells layout managers
/// how the widget wants to be sized relative to its size hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SizePolicy {
    /// The widget cannot grow or shrink. It always stays at its size hint.
    Fixed = 0,

    /// The size hint is the minimum size. The widget can grow but there's no
    /// benefit in making it larger than the size hint.
    Minimum = 1,

    /// The size hint is the maximum size. The widget can shrink but cannot
    /// grow larger than the size hint.
    Maximum = 2,

    /// The size hint is preferred but the widget can both grow and shrink.
    /// This is the default policy for most widgets.
    #[default]
    Preferred = 3,

    /// The widget wants to grow and take up as much space as possible.
    /// It can also shrink if needed.
    Expanding = 4,

    /// The size hint is the minimum and the widget wants to grow.
    /// Combines `Minimum` behavior with `Expanding` desire.
    MinimumExpanding = 5,

    /// The size hint is ignored. The widget will take whatever space is
    /// available, similar to `Expanding` but without a preferred size.
    Ignored = 6,
}

impl SizePolicy {
    /// Returns true if the policy allows the widget to grow.
    #[inline]
    pub fn can_grow(self) -> bool {
        !matches!(self, Self::Fixed | Self::Maximum)
    }

    /// Returns true if the policy allows the widget to shrink.
    #[inline]
    pub fn can_shrink(self) -> bool {
        !matches!(self, Self::Fixed | Self::Minimum | Self::MinimumExpanding)
    }

    /// Returns true if the widget actively wants more space.
    #[inline]
    pub fn wants_to_grow(self) -> bool {
        matches!(self, Self::Expanding | Self::MinimumExpanding | Self::Ignored)
    }
}

/// Combined horizontal and vertical size policies with stretch factors.
///
/// This is similar to Qt's `QSizePolicy` class which combines policies for
/// both dimensions along with stretch factors for proportional sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizePolicyPair {
    /// Horizontal size policy.
    pub horizontal: SizePolicy,

    /// Vertical size policy.
    pub vertical: SizePolicy,

    /// Horizontal stretch factor (0-255).
    ///
    /// When multiple widgets have the same policy in a layout, the stretch
    /// factor determines how extra space is distributed. A widget with
    /// stretch 2 gets twice as much extra space as one with stretch 1.
    pub horizontal_stretch: u8,

    /// Vertical stretch factor (0-255).
    pub vertical_stretch: u8,

    /// Whether the widget's height depends on its width.
    ///
    /// This is useful for widgets that wrap text or maintain aspect ratio.
    /// When true, layouts should call `height_for_width()` to get the
    /// appropriate height for a given width.
    pub height_for_width: bool,

    /// Whether the widget's width depends on its height.
    ///
    /// This is the inverse of `height_for_width`, useful for vertically
    /// oriented content.
    pub width_for_height: bool,
}

impl Default for SizePolicyPair {
    fn default() -> Self {
        Self {
            horizontal: SizePolicy::Preferred,
            vertical: SizePolicy::Preferred,
            horizontal_stretch: 0,
            vertical_stretch: 0,
            height_for_width: false,
            width_for_height: false,
        }
    }
}

impl SizePolicyPair {
    /// Create a new size policy pair with the specified policies.
    pub fn new(horizontal: SizePolicy, vertical: SizePolicy) -> Self {
        Self {
            horizontal,
            vertical,
            ..Default::default()
        }
    }

    /// Create a policy with the same value for both dimensions.
    pub fn uniform(policy: SizePolicy) -> Self {
        Self::new(policy, policy)
    }

    /// Create a fixed size policy (widget cannot resize).
    pub fn fixed() -> Self {
        Self::uniform(SizePolicy::Fixed)
    }

    /// Create a preferred size policy (default).
    pub fn preferred() -> Self {
        Self::uniform(SizePolicy::Preferred)
    }

    /// Create an expanding size policy (widget wants more space).
    pub fn expanding() -> Self {
        Self::uniform(SizePolicy::Expanding)
    }

    /// Set the horizontal stretch factor.
    pub fn with_horizontal_stretch(mut self, stretch: u8) -> Self {
        self.horizontal_stretch = stretch;
        self
    }

    /// Set the vertical stretch factor.
    pub fn with_vertical_stretch(mut self, stretch: u8) -> Self {
        self.vertical_stretch = stretch;
        self
    }

    /// Set both stretch factors.
    pub fn with_stretch(mut self, horizontal: u8, vertical: u8) -> Self {
        self.horizontal_stretch = horizontal;
        self.vertical_stretch = vertical;
        self
    }

    /// Enable height-for-width mode.
    pub fn with_height_for_width(mut self) -> Self {
        self.height_for_width = true;
        self
    }

    /// Enable width-for-height mode.
    pub fn with_width_for_height(mut self) -> Self {
        self.width_for_height = true;
        self
    }

    /// Transpose the policy (swap horizontal and vertical).
    pub fn transposed(self) -> Self {
        Self {
            horizontal: self.vertical,
            vertical: self.horizontal,
            horizontal_stretch: self.vertical_stretch,
            vertical_stretch: self.horizontal_stretch,
            height_for_width: self.width_for_height,
            width_for_height: self.height_for_width,
        }
    }
}

/// Size hint containing the preferred, minimum, and maximum sizes for a widget.
///
/// This is used by layout managers to determine how to size and position widgets.
/// Each widget provides a size hint based on its content and styling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeHint {
    /// The preferred size for the widget to display optimally.
    pub preferred: Size,

    /// The minimum acceptable size. If `None`, the widget has no minimum
    /// constraint (can shrink to zero).
    pub minimum: Option<Size>,

    /// The maximum size the widget should be. If `None`, the widget has no
    /// maximum constraint (can grow indefinitely).
    pub maximum: Option<Size>,
}

impl Default for SizeHint {
    fn default() -> Self {
        Self {
            preferred: Size::ZERO,
            minimum: None,
            maximum: None,
        }
    }
}

impl SizeHint {
    /// Create a new size hint with the specified preferred size.
    pub fn new(preferred: Size) -> Self {
        Self {
            preferred,
            minimum: None,
            maximum: None,
        }
    }

    /// Create a size hint with explicit width and height.
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        Self::new(Size::new(width, height))
    }

    /// Create a fixed size hint (preferred = minimum = maximum).
    pub fn fixed(size: Size) -> Self {
        Self {
            preferred: size,
            minimum: Some(size),
            maximum: Some(size),
        }
    }

    /// Create a size hint from dimensions with fixed sizing.
    pub fn fixed_dimensions(width: f32, height: f32) -> Self {
        Self::fixed(Size::new(width, height))
    }

    /// Set the minimum size.
    pub fn with_minimum(mut self, minimum: Size) -> Self {
        self.minimum = Some(minimum);
        self
    }

    /// Set the maximum size.
    pub fn with_maximum(mut self, maximum: Size) -> Self {
        self.maximum = Some(maximum);
        self
    }

    /// Set minimum dimensions.
    pub fn with_minimum_dimensions(mut self, width: f32, height: f32) -> Self {
        self.minimum = Some(Size::new(width, height));
        self
    }

    /// Set maximum dimensions.
    pub fn with_maximum_dimensions(mut self, width: f32, height: f32) -> Self {
        self.maximum = Some(Size::new(width, height));
        self
    }

    /// Get the effective minimum size (returns zero if not set).
    pub fn effective_minimum(&self) -> Size {
        self.minimum.unwrap_or(Size::ZERO)
    }

    /// Get the effective maximum size (returns a very large size if not set).
    pub fn effective_maximum(&self) -> Size {
        self.maximum.unwrap_or(Size::new(f32::MAX, f32::MAX))
    }

    /// Constrain a size to be within the minimum and maximum bounds.
    pub fn constrain(&self, size: Size) -> Size {
        let min = self.effective_minimum();
        let max = self.effective_maximum();

        Size::new(
            size.width.clamp(min.width, max.width),
            size.height.clamp(min.height, max.height),
        )
    }

    /// Check if the widget has a fixed width (minimum == maximum in width).
    pub fn has_fixed_width(&self) -> bool {
        match (self.minimum, self.maximum) {
            (Some(min), Some(max)) => (min.width - max.width).abs() < f32::EPSILON,
            _ => false,
        }
    }

    /// Check if the widget has a fixed height (minimum == maximum in height).
    pub fn has_fixed_height(&self) -> bool {
        match (self.minimum, self.maximum) {
            (Some(min), Some(max)) => (min.height - max.height).abs() < f32::EPSILON,
            _ => false,
        }
    }

    /// Check if the widget has a completely fixed size.
    pub fn is_fixed(&self) -> bool {
        self.has_fixed_width() && self.has_fixed_height()
    }

    /// Expand the size hint to include another size hint.
    ///
    /// The resulting hint's preferred size is the component-wise maximum,
    /// the minimum is the component-wise maximum of minimums, and the
    /// maximum is the component-wise minimum of maximums.
    pub fn expanded_to(&self, other: &SizeHint) -> SizeHint {
        let preferred = Size::new(
            self.preferred.width.max(other.preferred.width),
            self.preferred.height.max(other.preferred.height),
        );

        let minimum = match (self.minimum, other.minimum) {
            (Some(a), Some(b)) => Some(Size::new(a.width.max(b.width), a.height.max(b.height))),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let maximum = match (self.maximum, other.maximum) {
            (Some(a), Some(b)) => Some(Size::new(a.width.min(b.width), a.height.min(b.height))),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        SizeHint {
            preferred,
            minimum,
            maximum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_policy_can_grow() {
        assert!(!SizePolicy::Fixed.can_grow());
        assert!(SizePolicy::Minimum.can_grow());
        assert!(!SizePolicy::Maximum.can_grow());
        assert!(SizePolicy::Preferred.can_grow());
        assert!(SizePolicy::Expanding.can_grow());
    }

    #[test]
    fn test_size_policy_can_shrink() {
        assert!(!SizePolicy::Fixed.can_shrink());
        assert!(!SizePolicy::Minimum.can_shrink());
        assert!(SizePolicy::Maximum.can_shrink());
        assert!(SizePolicy::Preferred.can_shrink());
        assert!(SizePolicy::Expanding.can_shrink());
    }

    #[test]
    fn test_size_policy_wants_to_grow() {
        assert!(!SizePolicy::Fixed.wants_to_grow());
        assert!(!SizePolicy::Preferred.wants_to_grow());
        assert!(SizePolicy::Expanding.wants_to_grow());
        assert!(SizePolicy::MinimumExpanding.wants_to_grow());
        assert!(SizePolicy::Ignored.wants_to_grow());
    }

    #[test]
    fn test_size_policy_pair_default() {
        let policy = SizePolicyPair::default();
        assert_eq!(policy.horizontal, SizePolicy::Preferred);
        assert_eq!(policy.vertical, SizePolicy::Preferred);
        assert_eq!(policy.horizontal_stretch, 0);
        assert_eq!(policy.vertical_stretch, 0);
    }

    #[test]
    fn test_size_policy_pair_transposed() {
        let policy = SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding)
            .with_stretch(1, 2)
            .with_height_for_width();

        let transposed = policy.transposed();
        assert_eq!(transposed.horizontal, SizePolicy::Expanding);
        assert_eq!(transposed.vertical, SizePolicy::Fixed);
        assert_eq!(transposed.horizontal_stretch, 2);
        assert_eq!(transposed.vertical_stretch, 1);
        assert!(transposed.width_for_height);
        assert!(!transposed.height_for_width);
    }

    #[test]
    fn test_size_hint_constrain() {
        let hint = SizeHint::new(Size::new(100.0, 100.0))
            .with_minimum(Size::new(50.0, 50.0))
            .with_maximum(Size::new(200.0, 200.0));

        // Within bounds
        assert_eq!(
            hint.constrain(Size::new(150.0, 150.0)),
            Size::new(150.0, 150.0)
        );

        // Below minimum
        assert_eq!(
            hint.constrain(Size::new(25.0, 25.0)),
            Size::new(50.0, 50.0)
        );

        // Above maximum
        assert_eq!(
            hint.constrain(Size::new(300.0, 300.0)),
            Size::new(200.0, 200.0)
        );
    }

    #[test]
    fn test_size_hint_fixed() {
        let hint = SizeHint::fixed(Size::new(100.0, 50.0));
        assert!(hint.is_fixed());
        assert!(hint.has_fixed_width());
        assert!(hint.has_fixed_height());
    }

    #[test]
    fn test_size_hint_expanded_to() {
        let hint1 = SizeHint::new(Size::new(100.0, 50.0))
            .with_minimum(Size::new(50.0, 25.0));
        let hint2 = SizeHint::new(Size::new(80.0, 100.0))
            .with_maximum(Size::new(200.0, 200.0));

        let expanded = hint1.expanded_to(&hint2);
        assert_eq!(expanded.preferred, Size::new(100.0, 100.0));
        assert_eq!(expanded.minimum, Some(Size::new(50.0, 25.0)));
        assert_eq!(expanded.maximum, Some(Size::new(200.0, 200.0)));
    }
}
