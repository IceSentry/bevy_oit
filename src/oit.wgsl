#ifndef MAX_CASCADES_PER_LIGHT
    #define MAX_CASCADES_PER_LIGHT 1
#endif

#ifndef MAX_DIRECTIONAL_LIGHTS
    #define MAX_DIRECTIONAL_LIGHTS 1
#endif

#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types

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

// Should these be in group 3?
@group(2) @binding(1)
var<storage, read_write> a_counter: atomic<i32>;

@group(2) @binding(2)
var<storage, read_write> layers: array<vec4<f32>>;

@vertex
fn vertex( @location(0) position: vec3<f32>) ->  @builtin(position) vec4<f32> {
    return view.view_proj * mesh.model * vec4<f32>(position, 1.0);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let screen_index = i32(position.x + position.y * view.viewport.z);

    let layer_index = screen_index;
    // TODO use pbr() here
    layers[layer_index] = material.base_color;

    return layers[layer_index];

    // atomicAdd(&a_counter, 1);
    // var counter = atomicLoad(&a_counter);
    // let r = (f32(counter) / 100.0) / 100.0;
    // if counter < 500 {
    //     return vec4(1.0, 0.0, 0.0, 1.0);
    // } else {
    //     return vec4(0.0);
    // }
    // return vec4(f32(counter) / 20000.0, 0.0, 0.0, 1.0);
}
