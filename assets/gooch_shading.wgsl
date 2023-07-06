#import bevy_pbr::mesh_vertex_output MeshVertexOutput

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

@fragment
fn fragment(mesh: MeshVertexOutput) -> @location(0) vec4<f32> {
    // Gooch shading!
    // Interpolates between white and a cooler color based on the angle
    // between the normal and the light.
    let light = normalize(vec3(-1.0, 2.0, 1.0));
    let warmth = dot(normalize(mesh.world_normal), light) * 0.5 + 0.5;
    let shading =  mix(vec3(0.0, 0.25, 0.75), vec3(1.0, 1.0, 1.0), warmth);
    return vec4(color.rgb * shading, color.a);
}