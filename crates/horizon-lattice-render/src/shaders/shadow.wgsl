// Box shadow shader using analytical Gaussian blur approximation.
//
// Based on Evan Wallace's "Fast Rounded Rectangle Shadows" technique:
// https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
//
// This shader computes soft shadows in O(1) using the error function (erf)
// to analytically integrate the Gaussian convolution with a rectangle.

struct Uniforms {
    transform: mat4x4<f32>,
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,        // Quad vertex position
    @location(1) color: vec4<f32>,           // Shadow color (premultiplied alpha)
    @location(2) rect_center: vec2<f32>,     // Center of the shadow-casting rect
    @location(3) rect_half_size: vec2<f32>,  // Half-size of rect (after spread applied)
    @location(4) shadow_params: vec4<f32>,   // [sigma, corner_radius, offset_x, offset_y]
    @location(5) flags: vec4<f32>,           // [inset, unused, unused, unused]
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_pos: vec2<f32>,        // Position in world/pixel coords
    @location(2) rect_center: vec2<f32>,
    @location(3) rect_half_size: vec2<f32>,
    @location(4) shadow_params: vec4<f32>,
    @location(5) flags: vec4<f32>,
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
    output.world_pos = input.position;
    output.rect_center = input.rect_center;
    output.rect_half_size = input.rect_half_size;
    output.shadow_params = input.shadow_params;
    output.flags = input.flags;

    return output;
}

// Approximation of the error function (erf)
// Using a polynomial approximation that's efficient on GPUs
fn erf_approx(x: vec2<f32>) -> vec2<f32> {
    let s = sign(x);
    let a = abs(x);
    let t = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    let t2 = t * t;
    return s - s / (t2 * t2);
}

// Single-value erf for scalar inputs
fn erf_scalar(x: f32) -> f32 {
    let s = sign(x);
    let a = abs(x);
    let t = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    let t2 = t * t;
    return s - s / (t2 * t2);
}

// Gaussian function
fn gaussian(x: f32, sigma: f32) -> f32 {
    let inv_sqrt_2pi = 0.3989422804014327;
    return inv_sqrt_2pi / sigma * exp(-0.5 * (x * x) / (sigma * sigma));
}

// Calculate shadow alpha for a sharp (non-rounded) rectangle
// This is O(1) using the closed-form integral of Gaussian × box
fn box_shadow_alpha(p: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>, sigma: f32) -> f32 {
    let d = p - center;
    let low = -half_size - d;
    let high = half_size - d;

    let inv_sqrt2_sigma = 0.7071067811865476 / sigma;
    let integral = 0.5 + 0.5 * erf_approx(vec2<f32>(high.x, high.y) * inv_sqrt2_sigma)
                 - (0.5 + 0.5 * erf_approx(vec2<f32>(low.x, low.y) * inv_sqrt2_sigma));

    // The 2D shadow is the product of 1D shadows in X and Y
    return integral.x * integral.y;
}

// Calculate shadow alpha for X dimension of a rounded rectangle
// For a given Y position, compute the X extent of the rounded shape
fn rounded_box_shadow_x(x: f32, y: f32, sigma: f32, corner: f32, half_size: vec2<f32>) -> f32 {
    // Calculate how much the corner cuts into the rectangle at this Y
    let delta = min(half_size.y - corner - abs(y), 0.0);

    // The curved edge position: starts at half_size.x - corner, curves outward
    let curved = half_size.x - corner + sqrt(max(0.0, corner * corner - delta * delta));

    // Integrate the Gaussian across the X extent [-curved, curved]
    let inv_sqrt2_sigma = 0.7071067811865476 / sigma;
    let integral = 0.5 + 0.5 * erf_approx(vec2<f32>(x + curved, -(x - curved)) * inv_sqrt2_sigma);

    return integral.x - (1.0 - integral.y);
}

// Calculate shadow alpha for a rounded rectangle using numerical integration in Y
fn rounded_box_shadow_alpha(p: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>, sigma: f32, corner: f32) -> f32 {
    // Local position relative to rect center
    let local = p - center;

    // If corner radius is negligible, use the fast box shadow
    if corner < 0.5 {
        return box_shadow_alpha(p, center, half_size, sigma);
    }

    // Clamp corner radius to half the smaller dimension
    let max_corner = min(half_size.x, half_size.y);
    let r = min(corner, max_corner);

    // Integration range: we only need to integrate where the Gaussian has significant weight
    // 3 sigma captures 99.7% of the Gaussian
    let low = local.y - half_size.y;
    let high = local.y + half_size.y;
    let start_y = clamp(-3.0 * sigma, low, high);
    let end_y = clamp(3.0 * sigma, low, high);

    // Use 4 samples for integration (good quality/performance tradeoff)
    let step = (end_y - start_y) / 4.0;
    var y = start_y + step * 0.5;
    var value = 0.0;

    // Numerical integration using midpoint rule
    for (var i = 0; i < 4; i++) {
        let weight = gaussian(y, sigma);
        let x_contrib = rounded_box_shadow_x(local.x, local.y - y, sigma, r, half_size);
        value += x_contrib * weight * step;
        y += step;
    }

    return value;
}

// Calculate inset shadow alpha
fn inset_shadow_alpha(p: vec2<f32>, center: vec2<f32>, half_size: vec2<f32>, sigma: f32, corner: f32, offset: vec2<f32>) -> f32 {
    // For inset shadows, we compute the shadow of the "hole" and invert it
    // The offset is reversed for inset shadows
    let shifted_center = center - offset;

    // Compute the outer shadow
    let outer = rounded_box_shadow_alpha(p, shifted_center, half_size, sigma, corner);

    // Invert: the inset shadow is where the outer shadow ISN'T
    let inset = 1.0 - outer;

    // Mask to only show inside the original (unshifted) shape
    // Use SDF to determine if we're inside the rect
    let local = p - center;
    let q = abs(local) - half_size + corner;
    let dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - corner;

    // Only show shadow inside the shape
    let inside_mask = smoothstep(1.0, -1.0, dist);

    return inset * inside_mask;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sigma = input.shadow_params.x;
    let corner = input.shadow_params.y;
    let offset = vec2<f32>(input.shadow_params.z, input.shadow_params.w);
    let is_inset = input.flags.x > 0.5;

    var alpha: f32;

    if is_inset {
        alpha = inset_shadow_alpha(
            input.world_pos,
            input.rect_center,
            input.rect_half_size,
            sigma,
            corner,
            offset
        );
    } else {
        // Apply offset to the shadow position
        let shadow_center = input.rect_center + offset;
        alpha = rounded_box_shadow_alpha(
            input.world_pos,
            shadow_center,
            input.rect_half_size,
            sigma,
            corner
        );
    }

    // Apply shadow color with computed alpha
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
