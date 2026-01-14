// Text shader for glyph rendering.
// Supports both grayscale antialiased text and color emoji.

struct Uniforms {
    // Transform matrix (4x4 for GPU compatibility)
    transform: mat4x4<f32>,
    // Viewport size for coordinate conversion
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,      // Screen position
    @location(1) uv: vec2<f32>,            // Texture coordinates in atlas
    @location(2) color: vec4<f32>,         // Text color (premultiplied alpha)
    @location(3) flags: u32,               // Flags: bit 0 = is_color_glyph
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) flags: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var s_glyph_atlas: sampler;

// Flag constants
const FLAG_COLOR_GLYPH: u32 = 1u;
const FLAG_SUBPIXEL: u32 = 2u;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Transform position
    let world_pos = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);

    // Convert to clip space (-1 to 1)
    let clip_x = (world_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (world_pos.y / uniforms.viewport_size.y) * 2.0;

    output.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    output.flags = input.flags;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample glyph from atlas
    let tex = textureSample(t_glyph_atlas, s_glyph_atlas, input.uv);

    // Check if this is a color glyph (emoji)
    let is_color = (input.flags & FLAG_COLOR_GLYPH) != 0u;

    if is_color {
        // Color glyph (emoji): use texture color directly
        // The emoji color is already premultiplied
        return tex;
    } else {
        // Grayscale text: texture is (1,1,1,alpha) where alpha is coverage
        // Apply text color with coverage from texture alpha
        let coverage = tex.a;

        // Output premultiplied alpha
        // text_color is already premultiplied, multiply by coverage
        return input.color * coverage;
    }
}

// Subpixel fragment shader variant
// This would be used for LCD subpixel antialiasing
@fragment
fn fs_subpixel(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample glyph from atlas
    let tex = textureSample(t_glyph_atlas, s_glyph_atlas, input.uv);

    // Check if this is a color glyph (emoji)
    let is_color = (input.flags & FLAG_COLOR_GLYPH) != 0u;

    if is_color {
        // Color glyph: use texture color directly
        return tex;
    } else {
        // Subpixel text: RGB channels contain per-subpixel coverage
        // Each channel (R, G, B) is independently blended
        let r_coverage = tex.r;
        let g_coverage = tex.g;
        let b_coverage = tex.b;

        // Apply text color with per-channel coverage
        // For subpixel blending, we need special handling
        // This outputs individual RGB coverage with alpha
        let result = vec4<f32>(
            input.color.r * r_coverage,
            input.color.g * g_coverage,
            input.color.b * b_coverage,
            max(max(r_coverage, g_coverage), b_coverage) * input.color.a
        );

        return result;
    }
}
