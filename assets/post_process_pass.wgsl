#import bevy_pbr::utils

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct PostProcessSettings {
    intensity: f32,
    viewport_width: f32,
}
@group(0) @binding(2)
var<uniform> settings: PostProcessSettings;
@group(0) @binding(3)
var<storage, read> layers: array<vec4<f32>>;

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return FullscreenVertexOutput(clip_position, uv);
}


@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(screen_texture, texture_sampler, in.uv);
    return  color;
    // let layer_index = i32(in.position.x + in.position.y * settings.viewport_width);
    // let layer = layers[layer_index];
    // return vec4(layer.r, layer.g, layer.b, 1.0);
}