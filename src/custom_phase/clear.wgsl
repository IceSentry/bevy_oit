@group(0) @binding(0)
var<storage, read_write> layers: array<vec4<f32>>;

@group(1) @binding(0)
var<storage, read_write> layer_ids: array<atomic<i32>>;


struct FullscreenVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) {
    let viewport_width = 1280.0;
    let screen_index = i32(in.position.x + in.position.y * viewport_width);
    atomicStore(&layer_ids[screen_index], 0);
    layers[screen_index] = vec4(0.0);
    discard;
}
