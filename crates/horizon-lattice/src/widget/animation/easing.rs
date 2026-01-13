//! Easing functions for smooth animations.
//!
//! Easing functions map a linear progress value (0.0 to 1.0) to a transformed
//! value that creates smoother, more natural-looking animations.

use std::f32::consts::PI;

/// Available easing functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Linear interpolation (no easing).
    #[default]
    Linear,
    /// Quadratic ease-in (starts slow, accelerates).
    EaseIn,
    /// Quadratic ease-out (starts fast, decelerates).
    EaseOut,
    /// Quadratic ease-in-out (smooth start and end).
    EaseInOut,
    /// Cubic ease-in (more pronounced than quadratic).
    EaseInCubic,
    /// Cubic ease-out (more pronounced than quadratic).
    EaseOutCubic,
    /// Cubic ease-in-out (more pronounced than quadratic).
    EaseInOutCubic,
    /// Sinusoidal ease-in.
    EaseInSine,
    /// Sinusoidal ease-out.
    EaseOutSine,
    /// Sinusoidal ease-in-out.
    EaseInOutSine,
}

/// Apply an easing function to a progress value.
///
/// # Arguments
///
/// * `easing` - The easing function to apply
/// * `t` - Progress value from 0.0 to 1.0
///
/// # Returns
///
/// The eased value, typically in the range 0.0 to 1.0.
///
/// # Example
///
/// ```
/// use horizon_lattice::widget::animation::{ease, Easing};
///
/// // Linear: output equals input
/// assert_eq!(ease(Easing::Linear, 0.5), 0.5);
///
/// // Ease-in: slower at start
/// assert!(ease(Easing::EaseIn, 0.5) < 0.5);
///
/// // Ease-out: slower at end
/// assert!(ease(Easing::EaseOut, 0.5) > 0.5);
/// ```
#[inline]
pub fn ease(easing: Easing, t: f32) -> f32 {
    // Clamp input to valid range
    let t = t.clamp(0.0, 1.0);

    match easing {
        Easing::Linear => t,
        Easing::EaseIn => ease_in_quad(t),
        Easing::EaseOut => ease_out_quad(t),
        Easing::EaseInOut => ease_in_out_quad(t),
        Easing::EaseInCubic => ease_in_cubic(t),
        Easing::EaseOutCubic => ease_out_cubic(t),
        Easing::EaseInOutCubic => ease_in_out_cubic(t),
        Easing::EaseInSine => ease_in_sine(t),
        Easing::EaseOutSine => ease_out_sine(t),
        Easing::EaseInOutSine => ease_in_out_sine(t),
    }
}

/// Interpolate between two values using an easing function.
///
/// # Arguments
///
/// * `easing` - The easing function to apply
/// * `start` - Starting value
/// * `end` - Ending value
/// * `t` - Progress value from 0.0 to 1.0
///
/// # Returns
///
/// The interpolated value between `start` and `end`.
#[inline]
pub fn lerp_eased(easing: Easing, start: f32, end: f32, t: f32) -> f32 {
    let eased_t = ease(easing, t);
    start + (end - start) * eased_t
}

// =============================================================================
// Quadratic Easing
// =============================================================================

#[inline]
fn ease_in_quad(t: f32) -> f32 {
    t * t
}

#[inline]
fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

#[inline]
fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

// =============================================================================
// Cubic Easing
// =============================================================================

#[inline]
fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

#[inline]
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[inline]
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// =============================================================================
// Sinusoidal Easing
// =============================================================================

#[inline]
fn ease_in_sine(t: f32) -> f32 {
    1.0 - ((t * PI) / 2.0).cos()
}

#[inline]
fn ease_out_sine(t: f32) -> f32 {
    ((t * PI) / 2.0).sin()
}

#[inline]
fn ease_in_out_sine(t: f32) -> f32 {
    -((PI * t).cos() - 1.0) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        assert_eq!(ease(Easing::Linear, 0.0), 0.0);
        assert_eq!(ease(Easing::Linear, 0.5), 0.5);
        assert_eq!(ease(Easing::Linear, 1.0), 1.0);
    }

    #[test]
    fn test_ease_in() {
        assert_eq!(ease(Easing::EaseIn, 0.0), 0.0);
        assert!(ease(Easing::EaseIn, 0.5) < 0.5); // Slower at start
        assert_eq!(ease(Easing::EaseIn, 1.0), 1.0);
    }

    #[test]
    fn test_ease_out() {
        assert_eq!(ease(Easing::EaseOut, 0.0), 0.0);
        assert!(ease(Easing::EaseOut, 0.5) > 0.5); // Faster at start
        assert_eq!(ease(Easing::EaseOut, 1.0), 1.0);
    }

    #[test]
    fn test_ease_in_out() {
        assert_eq!(ease(Easing::EaseInOut, 0.0), 0.0);
        assert_eq!(ease(Easing::EaseInOut, 0.5), 0.5); // Midpoint unchanged
        assert_eq!(ease(Easing::EaseInOut, 1.0), 1.0);
    }

    #[test]
    fn test_clamp() {
        // Values outside 0-1 should be clamped
        assert_eq!(ease(Easing::Linear, -0.5), 0.0);
        assert_eq!(ease(Easing::Linear, 1.5), 1.0);
    }

    #[test]
    fn test_lerp_eased() {
        // Linear interpolation from 100 to 200
        assert_eq!(lerp_eased(Easing::Linear, 100.0, 200.0, 0.0), 100.0);
        assert_eq!(lerp_eased(Easing::Linear, 100.0, 200.0, 0.5), 150.0);
        assert_eq!(lerp_eased(Easing::Linear, 100.0, 200.0, 1.0), 200.0);
    }

    #[test]
    fn test_cubic_more_pronounced() {
        // Cubic should be more pronounced than quadratic
        let quad_mid = ease(Easing::EaseIn, 0.5);
        let cubic_mid = ease(Easing::EaseInCubic, 0.5);
        assert!(cubic_mid < quad_mid); // Cubic is even slower at start
    }

    #[test]
    fn test_sine_boundaries() {
        // Sine easing should also start and end at 0 and 1
        assert!((ease(Easing::EaseInSine, 0.0) - 0.0).abs() < 0.001);
        assert!((ease(Easing::EaseInSine, 1.0) - 1.0).abs() < 0.001);
        assert!((ease(Easing::EaseOutSine, 0.0) - 0.0).abs() < 0.001);
        assert!((ease(Easing::EaseOutSine, 1.0) - 1.0).abs() < 0.001);
    }
}
