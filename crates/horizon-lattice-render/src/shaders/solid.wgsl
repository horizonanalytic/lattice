// Simple solid color shader for backgrounds and decorations.
// No texture sampling - just outputs vertex color.

struct Uniforms {
    // Transform matrix
    transform: mat4x4<f32>,
    // Viewport size for coordinate conversion
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
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

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Just output the vertex color with premultiplied alpha
    return input.color;
}
