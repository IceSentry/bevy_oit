struct ClearSettings {
    viewport_width: f32
}
@group(0) @binding(0)
var<uniform> settings: ClearSettings;

// TODO use u32 instead
@group(0) @binding(1)
var<storage, read_write> a_counter: array<atomic<i32>>;

@group(1) @binding(0)
var<storage, read_write> layers: array<vec4<f32>>;

struct FullscreenVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) {
    let screen_index = i32(in.position.x + in.position.y * settings.viewport_width);
    atomicStore(&a_counter[screen_index], 0);
    layers[screen_index] = vec4(0.0);
    discard;
}
