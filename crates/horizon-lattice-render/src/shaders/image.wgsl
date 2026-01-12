// Image shader for textured 2D rendering.
// Supports texture sampling with tinting and opacity.

struct Uniforms {
    // Transform matrix (4x4 for GPU compatibility)
    transform: mat4x4<f32>,
    // Viewport size for coordinate conversion
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,      // Screen position
    @location(1) uv: vec2<f32>,            // Texture coordinates
    @location(2) tint: vec4<f32>,          // Tint color (premultiplied alpha)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

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
    output.tint = input.tint;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample texture
    let tex_color = textureSample(t_diffuse, s_diffuse, input.uv);

    // Apply tint (multiply with premultiplied alpha colors)
    // If tint is white (1,1,1,1), this is a no-op
    let result = tex_color * input.tint;

    return result;
}
