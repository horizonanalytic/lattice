// Rectangle shader for 2D rendering.
// Supports solid colors, rounded corners, and borders.

struct Uniforms {
    // Transform matrix (3x2 stored as 4 vec2s for alignment)
    transform: mat4x4<f32>,
    // Viewport size for coordinate conversion
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) rect_pos: vec2<f32>,      // Top-left of rect
    @location(3) rect_size: vec2<f32>,     // Size of rect
    @location(4) corner_radii: vec4<f32>,  // TL, TR, BR, BL
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,     // Position within rect (0-1)
    @location(2) rect_size: vec2<f32>,     // Size of rect in pixels
    @location(3) corner_radii: vec4<f32>,  // TL, TR, BR, BL
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Transform position
    let world_pos = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);

    // Convert to clip space (-1 to 1)
    let clip_x = (world_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (world_pos.y / uniforms.viewport_size.y) * 2.0;

    output.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    output.color = input.color;

    // Compute local position (0-1 within the rect)
    output.local_pos = (input.position - input.rect_pos) / input.rect_size;
    output.rect_size = input.rect_size;
    output.corner_radii = input.corner_radii;

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

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let max_radius = max(max(input.corner_radii.x, input.corner_radii.y),
                         max(input.corner_radii.z, input.corner_radii.w));

    // If no rounding, just output the color
    if max_radius < 0.5 {
        return input.color;
    }

    // Calculate distance from edge
    let dist = sd_rounded_rect(input.local_pos, input.rect_size, input.corner_radii);

    // Anti-aliasing: smooth transition at edges
    // Use 1 pixel for anti-aliasing
    let aa_width = 1.0;
    let alpha = 1.0 - smoothstep(-aa_width, 0.0, dist);

    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
