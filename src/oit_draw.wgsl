#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_pbr::mesh_types Mesh

#import bevy_oit::oit_draw_bindings view, material, mesh, layers, layer_ids, oit_layers

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
fn fragment(
    @builtin(sample_mask) sample_mask: u32,
    in: VertexOutput
) -> @location(0) vec4<f32> {
    oit_draw_start(in.position, sample_mask);

    // TODO this shading should be user customizable
    let color = gooch_shading(
        material.base_color,
        in.world_normal,
        view.world_position,
    );

    return oit_draw_end(in.position, color);

    // we don't need to actually output anything, but early depth test doesn't seem to work
    // with fragment shaders with no output
    // return vec4(0.0);
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

fn oit_draw_start(position: vec4f, sample_mask: u32) {
    // This feels super hacky
    // sample_mask contains a bit for the sample index
    // so if MSAA == 8 then any bit between 0 and 8 bits might be enabled
    //
    // We only want to render 1 sample so we skip any samples that isn't the last one
#ifdef MSAA
    let msaa_mask = 1u << (#{MSAA}u - 1u);
    if sample_mask < msaa_mask {
        discard;
    }
#endif
}

fn oit_draw_end(position: vec4f, color: vec4f) -> vec4<f32> {
    let screen_index = i32(floor(position.x) + floor(position.y) * view.viewport.z);
    let buffer_size = i32(view.viewport.z * view.viewport.w);

    var layer_id = atomicAdd(&layer_ids[screen_index], 1);
    if layer_id >= oit_layers {
        atomicStore(&layer_ids[screen_index], oit_layers);
        layer_id = oit_layers;

        // tail blend
        // TODO this doesn't seem to work correctly right now
        // return color;
        // discard;
        return vec4(0.0);
    }

    let layer_index = screen_index + layer_id * buffer_size;
    let packed_color = pack4x8unorm(color);
    let depth = bitcast<u32>(position.z);
    layers[layer_index] = vec2(packed_color, depth);

    // we don't want to actually render anything here
    return vec4(0.0);
}