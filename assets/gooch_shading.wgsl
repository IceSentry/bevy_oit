#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_view_bindings view

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

@fragment
fn fragment(mesh: MeshVertexOutput) -> @location(0) vec4<f32> {
    let color = gooch_shading(color, mesh.world_normal, view.world_position);
    return color;
}

// Interpolates between a warm color and a cooler color based on the angle
// between the normal and the light.
fn gooch_shading(color: vec4<f32>, world_normal: vec3<f32>, camera_position: vec3<f32>) -> vec4<f32> {
    let light_direction = normalize(vec3(-1.0, 2.0, 1.0));
    let camera_direction = normalize(camera_position);

    let warm = vec3(0.4, 0.4, 0.0);
    let cool = vec3(0.0, 0.0, 0.4);

    let a = 0.2;
    let b = 0.6;

    // diffuse
    let gooch = dot(normalize(world_normal), light_direction) * 0.5 + 0.5;
    var gooch_color = gooch  * (warm + b * color.rgb) +
        (1.0 - gooch) * (cool + a * color.rgb);

    // specular
    let R = reflect(-light_direction, normalize(world_normal));
    let ER = clamp(dot(camera_direction, normalize(R)), 0.0, 1.0);
    let specular_strength = pow(ER, 2.0);
    let spec = gooch_color.rgb * specular_strength;

    return vec4(gooch_color.rgb + spec, color.a);
}
