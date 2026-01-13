//! Stencil-based clipping support.
//!
//! This module provides stencil buffer management for advanced clipping operations
//! including rounded rectangle clips and arbitrary path clips.

use crate::types::{Rect, RoundedRect, Size};

/// Depth/stencil texture format used for clipping.
pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// Manages a depth/stencil texture for clipping operations.
#[derive(Debug)]
pub struct StencilTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: Size,
}

impl StencilTexture {
    /// Create a new stencil texture with the given dimensions.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("stencil_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_STENCIL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            size: Size::new(width as f32, height as f32),
        }
    }

    /// Resize the stencil texture.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width as f32 != self.size.width || height as f32 != self.size.height {
            *self = Self::new(device, width, height);
        }
    }

    /// Get the texture view for render pass attachment.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Get the current size.
    pub fn size(&self) -> Size {
        self.size
    }
}

/// A clip shape that can be rendered to the stencil buffer.
#[derive(Debug, Clone)]
pub enum ClipShape {
    /// A rounded rectangle clip.
    RoundedRect(RoundedRect),
    /// A regular rectangle clip (optimized path).
    Rect(Rect),
}

impl ClipShape {
    /// Get the bounding rectangle of this clip shape.
    pub fn bounds(&self) -> Rect {
        match self {
            ClipShape::RoundedRect(rr) => rr.rect,
            ClipShape::Rect(r) => *r,
        }
    }

    /// Check if this is a simple rectangle (no rounding).
    pub fn is_rect(&self) -> bool {
        match self {
            ClipShape::RoundedRect(rr) => rr.radii.is_zero(),
            ClipShape::Rect(_) => true,
        }
    }
}

impl From<RoundedRect> for ClipShape {
    fn from(rr: RoundedRect) -> Self {
        if rr.radii.is_zero() {
            ClipShape::Rect(rr.rect)
        } else {
            ClipShape::RoundedRect(rr)
        }
    }
}

impl From<Rect> for ClipShape {
    fn from(r: Rect) -> Self {
        ClipShape::Rect(r)
    }
}

/// Manages the stencil clip stack for nested clipping.
#[derive(Debug)]
pub struct ClipStack {
    /// Current clip depth (stencil reference value).
    depth: u32,
    /// Stack of clip shapes for potential restoration.
    shapes: Vec<ClipShape>,
    /// Maximum supported clip depth (8-bit stencil = 255).
    max_depth: u32,
}

impl Default for ClipStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipStack {
    /// Create a new empty clip stack.
    pub fn new() -> Self {
        Self {
            depth: 0,
            shapes: Vec::new(),
            max_depth: 255, // 8-bit stencil
        }
    }

    /// Get the current clip depth (stencil reference value).
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Check if there are any active clips.
    pub fn has_clips(&self) -> bool {
        self.depth > 0
    }

    /// Push a new clip shape onto the stack.
    ///
    /// Returns the new stencil reference value, or None if max depth exceeded.
    pub fn push(&mut self, shape: ClipShape) -> Option<u32> {
        if self.depth >= self.max_depth {
            return None;
        }
        self.depth += 1;
        self.shapes.push(shape);
        Some(self.depth)
    }

    /// Pop the top clip shape from the stack.
    ///
    /// Returns the popped shape and the new stencil reference value.
    pub fn pop(&mut self) -> Option<(ClipShape, u32)> {
        if self.depth == 0 {
            return None;
        }
        let shape = self.shapes.pop()?;
        self.depth -= 1;
        Some((shape, self.depth))
    }

    /// Reset the clip stack to empty.
    pub fn reset(&mut self) {
        self.depth = 0;
        self.shapes.clear();
    }

    /// Get the stack of shapes (for debugging).
    pub fn shapes(&self) -> &[ClipShape] {
        &self.shapes
    }
}

/// Stencil face state for pushing clips (incrementing stencil).
pub fn push_stencil_state() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::IncrementClamp,
    }
}

/// Stencil face state for testing clips (drawing content).
pub fn test_stencil_state() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::LessEqual,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

/// Stencil face state for popping clips (decrementing stencil).
pub fn pop_stencil_state() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::DecrementClamp,
    }
}

/// Create depth/stencil state for a content pipeline (reads stencil, doesn't write).
pub fn content_depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_STENCIL_FORMAT,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: test_stencil_state(),
            back: test_stencil_state(),
            read_mask: 0xff,
            write_mask: 0x00, // Don't write to stencil during content rendering
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

/// Create depth/stencil state for pushing clips (increments stencil).
pub fn push_clip_depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_STENCIL_FORMAT,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: push_stencil_state(),
            back: push_stencil_state(),
            read_mask: 0xff,
            write_mask: 0xff,
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

/// Create depth/stencil state for popping clips (decrements stencil).
pub fn pop_clip_depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_STENCIL_FORMAT,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: pop_stencil_state(),
            back: pop_stencil_state(),
            read_mask: 0xff,
            write_mask: 0xff,
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_stack_push_pop() {
        let mut stack = ClipStack::new();
        assert_eq!(stack.depth(), 0);
        assert!(!stack.has_clips());

        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let depth = stack.push(ClipShape::Rect(rect));
        assert_eq!(depth, Some(1));
        assert_eq!(stack.depth(), 1);
        assert!(stack.has_clips());

        let (shape, new_depth) = stack.pop().unwrap();
        assert!(matches!(shape, ClipShape::Rect(_)));
        assert_eq!(new_depth, 0);
        assert!(!stack.has_clips());
    }

    #[test]
    fn test_clip_stack_nested() {
        let mut stack = ClipStack::new();

        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(10.0, 10.0, 80.0, 80.0);
        let r3 = Rect::new(20.0, 20.0, 60.0, 60.0);

        assert_eq!(stack.push(r1.into()), Some(1));
        assert_eq!(stack.push(r2.into()), Some(2));
        assert_eq!(stack.push(r3.into()), Some(3));
        assert_eq!(stack.depth(), 3);

        stack.pop();
        assert_eq!(stack.depth(), 2);
        stack.pop();
        assert_eq!(stack.depth(), 1);
        stack.pop();
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_clip_stack_reset() {
        let mut stack = ClipStack::new();
        stack.push(Rect::new(0.0, 0.0, 100.0, 100.0).into());
        stack.push(Rect::new(0.0, 0.0, 50.0, 50.0).into());

        stack.reset();
        assert_eq!(stack.depth(), 0);
        assert!(stack.shapes().is_empty());
    }

    #[test]
    fn test_clip_shape_from_rounded_rect() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);

        // Zero radius becomes a rect
        let rr_zero = RoundedRect::new(rect, 0.0);
        let shape_zero: ClipShape = rr_zero.into();
        assert!(shape_zero.is_rect());

        // Non-zero radius stays rounded
        let rr_rounded = RoundedRect::new(rect, 10.0);
        let shape_rounded: ClipShape = rr_rounded.into();
        assert!(!shape_rounded.is_rect());
    }
}
