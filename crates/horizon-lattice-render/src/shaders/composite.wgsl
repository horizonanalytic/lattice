// Compositing shader for layer rendering.
// Composites a layer texture onto the target with opacity and positioning.

struct Uniforms {
    // Output viewport size
    viewport_size: vec2<f32>,
    // Layer position offset in pixels
    layer_offset: vec2<f32>,
    // Layer size in pixels
    layer_size: vec2<f32>,
    // Layer opacity
    opacity: f32,
    _padding: f32,
}

struct VertexInput {
    @location(0) position: vec2<f32>,  // Normalized position (0-1)
    @location(1) uv: vec2<f32>,        // Texture coordinates
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_layer: texture_2d<f32>;

@group(1) @binding(1)
var s_layer: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Calculate screen position from normalized input and layer position/size
    let screen_x = uniforms.layer_offset.x + input.position.x * uniforms.layer_size.x;
    let screen_y = uniforms.layer_offset.y + input.position.y * uniforms.layer_size.y;

    // Convert to clip space (-1 to 1)
    let clip_x = (screen_x / uniforms.viewport_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (screen_y / uniforms.viewport_size.y) * 2.0;

    output.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    output.uv = input.uv;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample layer texture
    let layer_color = textureSample(t_layer, s_layer, input.uv);

    // Apply layer opacity (premultiplied alpha)
    let result = layer_color * uniforms.opacity;

    return result;
}
