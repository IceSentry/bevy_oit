#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_render::view View
#import bevy_pbr::mesh_types Mesh

@group(0) @binding(0)
var<uniform> view: View;

struct CustomMaterial {
    base_color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@group(2) @binding(0)
var<uniform> mesh: Mesh;

@group(3) @binding(0)
var<storage, read_write> layers: array<vec4<f32>>;

@group(4) @binding(0)
var<storage, read_write> layer_ids: array<atomic<i32>>;

const oit_layers: i32 = #{OIT_LAYERS};

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(mesh.model, vec4(vertex.position, 1.0));
    out.position = mesh_functions::mesh_position_world_to_clip(out.world_position);
    out.world_normal = mesh_normal_local_to_world(mesh, vertex.normal);
    return out;
}

// WARN This is a copy of mesh_functions::mesh_normal_local_to_world but it doesn't assume that mesh is present
fn mesh_normal_local_to_world(mesh: Mesh, vertex_normal: vec3<f32>) -> vec3<f32> {
    return normalize(
        mat3x3(
            mesh.inverse_transpose_model[0].xyz,
            mesh.inverse_transpose_model[1].xyz,
            mesh.inverse_transpose_model[2].xyz
        ) * vertex_normal
    );
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
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_index = i32(floor(in.position.x + in.position.y * view.viewport.z));

    let buffer_size = i32(floor(view.viewport.z * view.viewport.w));

    // TODO look into tail blending when counter becomes larger than oit_layers
    var abidx = atomicLoad(&layer_ids[screen_index]);
    abidx += 1;
    abidx = clamp(abidx, 0, i32(oit_layers));
    atomicStore(&layer_ids[screen_index], abidx);

    let layer_index = screen_index + (abidx - 1) * buffer_size;

    let shading = gooch_shading(in.world_normal);
    var color = material.base_color.rgb;
    color *= shading;
    layers[i32((in.position.x + in.position.y * view.viewport.z) + (f32(abidx - 1) * view.viewport.z * view.viewport.w))] = vec4(color, in.position.z);

    // we don't want to actually render anything here
    discard;
    // return material.color;
}

