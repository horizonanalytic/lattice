// Rectangle shader for 2D rendering.
// Supports solid colors, gradients, rounded corners, and borders.

// Paint type constants
const PAINT_TYPE_SOLID: u32 = 0u;
const PAINT_TYPE_LINEAR_GRADIENT: u32 = 1u;
const PAINT_TYPE_RADIAL_GRADIENT: u32 = 2u;
const PAINT_TYPE_LINEAR_GRADIENT_TEX: u32 = 3u;
const PAINT_TYPE_RADIAL_GRADIENT_TEX: u32 = 4u;

struct Uniforms {
    // Transform matrix (3x2 stored as 4 vec2s for alignment)
    transform: mat4x4<f32>,
    // Viewport size for coordinate conversion
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color0: vec4<f32>,         // Color for solid or gradient stop 0
    @location(2) rect_pos: vec2<f32>,       // Top-left of rect
    @location(3) rect_size: vec2<f32>,      // Size of rect
    @location(4) corner_radii: vec4<f32>,   // TL, TR, BR, BL
    @location(5) gradient_info: vec4<f32>,  // [paint_type, start_x, start_y, end_x]
    @location(6) gradient_end_stops: vec4<f32>, // [end_y, stop0_offset, stop1_offset, _unused]
    @location(7) color1: vec4<f32>,         // Gradient stop 1 color
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color0: vec4<f32>,
    @location(1) local_pos: vec2<f32>,      // Position within rect (0-1)
    @location(2) rect_size: vec2<f32>,      // Size of rect in pixels
    @location(3) corner_radii: vec4<f32>,   // TL, TR, BR, BL
    @location(4) gradient_info: vec4<f32>,  // [paint_type, start_x, start_y, end_x]
    @location(5) gradient_end_stops: vec4<f32>, // [end_y, stop0_offset, stop1_offset, _]
    @location(6) color1: vec4<f32>,         // Gradient stop 1 color
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Gradient texture atlas (optional, bound when using texture-based gradients)
@group(1) @binding(0)
var gradient_texture: texture_2d<f32>;
@group(1) @binding(1)
var gradient_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Transform position
    let world_pos = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);

    // Convert to clip space (-1 to 1)
    let clip_x = (world_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (world_pos.y / uniforms.viewport_size.y) * 2.0;

    output.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    output.color0 = input.color0;
    output.color1 = input.color1;

    // Compute local position (0-1 within the rect)
    output.local_pos = (input.position - input.rect_pos) / input.rect_size;
    output.rect_size = input.rect_size;
    output.corner_radii = input.corner_radii;
    output.gradient_info = input.gradient_info;
    output.gradient_end_stops = input.gradient_end_stops;

    return output;
}

// Signed distance function for a rounded rectangle
fn sd_rounded_rect(p: vec2<f32>, size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select the correct radius based on quadrant
    var r: f32;
    if p.x < 0.5 {
        if p.y < 0.5 {
            r = radii.x; // top-left
        } else {
            r = radii.w; // bottom-left
        }
    } else {
        if p.y < 0.5 {
            r = radii.y; // top-right
        } else {
            r = radii.z; // bottom-right
        }
    }

    // Convert to centered coordinates
    let half_size = size * 0.5;
    let centered = (p - 0.5) * size;

    // SDF for rounded rect
    let q = abs(centered) - half_size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

// Calculate color for linear gradient
fn linear_gradient_color(
    local_pos: vec2<f32>,
    start: vec2<f32>,
    end: vec2<f32>,
    stop0_offset: f32,
    stop1_offset: f32,
    color0: vec4<f32>,
    color1: vec4<f32>
) -> vec4<f32> {
    // Direction vector from start to end
    let dir = end - start;
    let dir_len_sq = dot(dir, dir);

    // Handle degenerate case (start == end)
    if dir_len_sq < 0.0001 {
        return color0;
    }

    // Project current position onto gradient line
    let to_pos = local_pos - start;
    var t = dot(to_pos, dir) / dir_len_sq;

    // Clamp to [0, 1] for the gradient line
    t = clamp(t, 0.0, 1.0);

    // Remap t based on stop offsets
    let stop_range = stop1_offset - stop0_offset;
    if stop_range < 0.0001 {
        if t < stop0_offset {
            return color0;
        } else {
            return color1;
        }
    }

    // Calculate interpolation factor based on stops
    let factor = clamp((t - stop0_offset) / stop_range, 0.0, 1.0);

    return mix(color0, color1, factor);
}

// Calculate color for radial gradient
fn radial_gradient_color(
    local_pos: vec2<f32>,
    center: vec2<f32>,
    radius: f32,
    stop0_offset: f32,
    stop1_offset: f32,
    color0: vec4<f32>,
    color1: vec4<f32>
) -> vec4<f32> {
    // Distance from center, normalized by radius
    let dist = length(local_pos - center);
    var t = dist / max(radius, 0.0001);

    // Clamp to [0, 1]
    t = clamp(t, 0.0, 1.0);

    // Remap t based on stop offsets
    let stop_range = stop1_offset - stop0_offset;
    if stop_range < 0.0001 {
        if t < stop0_offset {
            return color0;
        } else {
            return color1;
        }
    }

    // Calculate interpolation factor based on stops
    let factor = clamp((t - stop0_offset) / stop_range, 0.0, 1.0);

    return mix(color0, color1, factor);
}

// Calculate color for texture-based linear gradient
fn linear_gradient_color_tex(
    local_pos: vec2<f32>,
    start: vec2<f32>,
    end: vec2<f32>,
    tex_v: f32
) -> vec4<f32> {
    // Direction vector from start to end
    let dir = end - start;
    let dir_len_sq = dot(dir, dir);

    // Handle degenerate case (start == end)
    if dir_len_sq < 0.0001 {
        return textureSample(gradient_texture, gradient_sampler, vec2<f32>(0.0, tex_v));
    }

    // Project current position onto gradient line
    let to_pos = local_pos - start;
    var t = dot(to_pos, dir) / dir_len_sq;

    // Clamp to [0, 1]
    t = clamp(t, 0.0, 1.0);

    // Sample from gradient texture
    return textureSample(gradient_texture, gradient_sampler, vec2<f32>(t, tex_v));
}

// Calculate color for texture-based radial gradient
fn radial_gradient_color_tex(
    local_pos: vec2<f32>,
    center: vec2<f32>,
    radius: f32,
    tex_v: f32
) -> vec4<f32> {
    // Distance from center, normalized by radius
    let dist = length(local_pos - center);
    var t = dist / max(radius, 0.0001);

    // Clamp to [0, 1]
    t = clamp(t, 0.0, 1.0);

    // Sample from gradient texture
    return textureSample(gradient_texture, gradient_sampler, vec2<f32>(t, tex_v));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Determine the base color based on paint type
    let paint_type = u32(input.gradient_info.x);
    var base_color: vec4<f32>;

    if paint_type == PAINT_TYPE_SOLID {
        base_color = input.color0;
    } else if paint_type == PAINT_TYPE_LINEAR_GRADIENT {
        let start = vec2<f32>(input.gradient_info.y, input.gradient_info.z);
        let end = vec2<f32>(input.gradient_info.w, input.gradient_end_stops.x);
        let stop0_offset = input.gradient_end_stops.y;
        let stop1_offset = input.gradient_end_stops.z;
        base_color = linear_gradient_color(
            input.local_pos,
            start,
            end,
            stop0_offset,
            stop1_offset,
            input.color0,
            input.color1
        );
    } else if paint_type == PAINT_TYPE_RADIAL_GRADIENT {
        let center = vec2<f32>(input.gradient_info.y, input.gradient_info.z);
        let radius = input.gradient_info.w;
        let stop0_offset = input.gradient_end_stops.y;
        let stop1_offset = input.gradient_end_stops.z;
        base_color = radial_gradient_color(
            input.local_pos,
            center,
            radius,
            stop0_offset,
            stop1_offset,
            input.color0,
            input.color1
        );
    } else if paint_type == PAINT_TYPE_LINEAR_GRADIENT_TEX {
        let start = vec2<f32>(input.gradient_info.y, input.gradient_info.z);
        let end = vec2<f32>(input.gradient_info.w, input.gradient_end_stops.x);
        let tex_v = input.gradient_end_stops.w;
        base_color = linear_gradient_color_tex(input.local_pos, start, end, tex_v);
        // Apply opacity from color0.a (stored there for texture gradients)
        base_color = vec4<f32>(base_color.rgb * input.color0.a, base_color.a * input.color0.a);
    } else if paint_type == PAINT_TYPE_RADIAL_GRADIENT_TEX {
        let center = vec2<f32>(input.gradient_info.y, input.gradient_info.z);
        let radius = input.gradient_info.w;
        let tex_v = input.gradient_end_stops.w;
        base_color = radial_gradient_color_tex(input.local_pos, center, radius, tex_v);
        // Apply opacity from color0.a (stored there for texture gradients)
        base_color = vec4<f32>(base_color.rgb * input.color0.a, base_color.a * input.color0.a);
    } else {
        // Fallback to first color
        base_color = input.color0;
    }

    // Apply rounded corner masking
    let max_radius = max(max(input.corner_radii.x, input.corner_radii.y),
                         max(input.corner_radii.z, input.corner_radii.w));

    // If no rounding, just output the color
    if max_radius < 0.5 {
        return base_color;
    }

    // Calculate distance from edge
    let dist = sd_rounded_rect(input.local_pos, input.rect_size, input.corner_radii);

    // Anti-aliasing: smooth transition at edges
    // Use 1 pixel for anti-aliasing
    let aa_width = 1.0;
    let alpha = 1.0 - smoothstep(-aa_width, 0.0, dist);

    return vec4<f32>(base_color.rgb, base_color.a * alpha);
}
