#import bevy_pbr::mesh_functions as mesh_functions
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

@group(4) @binding(0)
var depth_texture: texture_depth_2d;

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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // manual depth testing
    // TODO figure out why early z depth test wasn't triggered
    let depth_sample = textureLoad(depth_texture, vec2<i32>(in.position.xy), 0);
    if in.position.z < depth_sample {
        discard;
    }

    let color = gooch_shading(
        material.base_color,
        in.world_normal,
        view.world_position,
    );

    let screen_index = i32(floor(in.position.x) + floor(in.position.y) * view.viewport.z);
    let buffer_size = i32(view.viewport.z * view.viewport.w);

    var layer_id = atomicAdd(&layer_ids[screen_index], 1);
    if layer_id >= oit_layers {
        atomicStore(&layer_ids[screen_index], oit_layers);
        layer_id = oit_layers;

        // tail blend
        // TODO this doesn't seem to work correctly right now
        // return vec4(color, material.base_color.a);
        discard;
    }

    let layer_index = screen_index + layer_id * buffer_size;
    let packed_color = pack4x8unorm(color);
    let depth = bitcast<u32>(in.position.z);
    layers[layer_index] = vec2(packed_color, depth);

    // we don't want to actually render anything here
    discard;
}

// Interpolates between a warm color and a cooler color based on the angle
// between the normal and the light.
fn gooch_shading(color: vec4<f32>, world_normal: vec3<f32>, camera_position: vec3<f32>) -> vec4<f32> {
    let light_direction = normalize(vec3(-1.0, 2.0, 1.0));
    let camera_direction = normalize(camera_position);

    let warm = vec3(0.4, 0.4, 0.0);
    let cool = vec3(0.0, 0.0, 0.4);

    let a = 0.2;
    let b = 0.8;

    // diffuse
    let gooch = dot(normalize(world_normal), light_direction) * 0.5 + 0.5;
    var gooch_color = gooch  * (warm + b * color.rgb) +
        (1.0 - gooch) * (cool + a * color.rgb);

    // specular
    let R = reflect(-light_direction, normalize(world_normal));
    let ER = clamp(dot(camera_direction, normalize(R)), 0.0, 1.0);
    let specular_strength = pow(ER, 2.0);
    let spec = gooch_color * specular_strength;

    return vec4(gooch_color.rgb + spec, color.a);
}
