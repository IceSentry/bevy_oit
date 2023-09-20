#define_import_path bevy_oit::oit_draw_bindings

#import bevy_render::view View
#import bevy_pbr::mesh_types Mesh

@group(0) @binding(0)
var<uniform> view: View;

struct OitMaterial {
    base_color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: OitMaterial;

@group(2) @binding(0)
var<uniform> mesh: Mesh;

@group(3) @binding(0)
var<storage, read_write> layers: array<vec2<u32>>;

@group(3) @binding(1)
var<storage, read_write> layer_ids: array<atomic<i32>>;

#ifdef MULTISAMPLED
@group(4) @binding(0)
var depth_texture: texture_depth_multisampled_2d;
#else
@group(4) @binding(0)
var depth_texture: texture_depth_2d;
#endif // MULTISAMPLED

const oit_layers: i32 = #{OIT_LAYERS};