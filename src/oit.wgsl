#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_render::view  View
#import bevy_pbr::mesh_types Mesh

// viewport(x_origin, y_origin, width, height)
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<uniform> mesh: Mesh;

struct Material {
    base_color: vec4<f32>,
}
@group(2) @binding(0)
var<uniform> material: Material;

struct OitSettings {
    oit_layers: u32
}
@group(2) @binding(1)
var<uniform> settings: OitSettings;

// TODO use u32 instead
@group(2) @binding(2)
var<storage, read_write> a_counter: array<atomic<i32>>;

@group(2) @binding(3)
var<storage, read_write> layers: array<vec4<f32>>;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct MeshVertexOutput {
    // this is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,
    // @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) ->  MeshVertexOutput {
    var out: MeshVertexOutput;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal);
    out.position = view.view_proj * mesh.model * vec4(vertex.position, 1.0);
    return out;
}

// Gooch shading!
// Interpolates between white and a cooler color based on the angle
// between the normal and the light.
fn gooch_shading(normal: vec3<f32>) -> vec3<f32> {
  let light = normalize(vec3(-1.0, 2.0, 1.0));
  let warmth = dot(normalize(normal), light) * 0.5 + 0.5;
  return mix(vec3(0.0, 0.25, 0.75), vec3(1.0, 1.0, 1.0), warmth);
}

@fragment
fn fragment(mesh: MeshVertexOutput) -> @location(0) vec4<f32> {
    let screen_index = i32(floor(mesh.position.x + mesh.position.y * view.viewport.z));

    let buffer_size = i32(floor(view.viewport.z * view.viewport.w));

    // TODO look into tail blending when counter becomes larger than oit_layers
    var abidx = atomicLoad(&a_counter[screen_index]);
    abidx += 1;
    abidx = clamp(abidx, 0, i32(settings.oit_layers));
    atomicStore(&a_counter[screen_index], abidx);

    let layer_index = screen_index + (abidx - 1) * buffer_size;

    // TODO use pbr() here or any kind of simple shading
    let shading = gooch_shading(mesh.world_normal);
    var color = material.base_color.rgb;
    color *= shading;
    layers[i32((mesh.position.x + mesh.position.y * view.viewport.z) + (f32(abidx - 1) * view.viewport.z * view.viewport.w))] = vec4(color, mesh.position.z);

    // we don't want to actually render anything here
    discard;
}
