#import bevy_pbr::utils

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

struct PostProcessSettings {
    viewport_width: f32,
    viewport_height: f32,
    oit_layers: u32,
}
@group(0) @binding(2)
var<uniform> settings: PostProcessSettings;
@group(0) @binding(3)
var<storage, read> a_counter: array<atomic<i32>>;
@group(0) @binding(4)
var<storage, read> layers: array<vec4<f32>>;

var<private> fragment_list: array<vec4<f32>, 32>; // TODO need to make sure this is bigger than oit_layers

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

    let buffer_size = i32(settings.viewport_width * settings.viewport_height);
    let screen_index = i32(in.position.x + in.position.y * settings.viewport_width);
    let counter = atomicLoad(&a_counter[screen_index]);

    // resolve closest
    // var min = vec4(0.0);
    // for (var i = 0; i < counter; i += 1){
    //     let val = layers[screen_index + buffer_size * i];
    //     if val.w > min.w {
    //         min = val;
    //     }
    // }
    // if counter == 0 && all(min == vec4(0.0)){
    //     return color;
    // } else {
    //     return min;
    // }

    let final_color = sort(screen_index, buffer_size, color);
    if counter > 0 {
        return final_color;
    }else {
        return color;
    }

    // show layer density
    // if counter == 0 {
    //     return color;
    // } else {
    //     let x = f32(counter) / f32(settings.oit_layers);
    //     return vec4(x, x, x, 1.0);
    // }
}

fn sort(screen_index: i32, buffer_size: i32, background_color: vec4<f32>) -> vec4<f32> {
    let counter = atomicLoad(&a_counter[screen_index]);

    // fill list
    for (var i = 0; i < counter; i += 1){
       fragment_list[i] = layers[screen_index + buffer_size * i];
    }

    // bubble sort

    for (var i = 0; i < counter - 1; i += 1){
        var swapped = false;
        for (var j = 0; j < counter - i - 1; j += 1) {
            if fragment_list[j].w > fragment_list[j + 1].w {
                let temp = fragment_list[j + 1];
                fragment_list[j + 1] = fragment_list[j];
                fragment_list[j] = temp;
                swapped = true;
            }
        }
        if swapped {
            break;
        }
    }

    for (var i = counter; i >= 0; i -= 1){
        for (var j = 0; j <= i; j += 1) {
            if fragment_list[j].w < fragment_list[j + 1].w {
                let temp = fragment_list[j + 1];
                fragment_list[j + 1] = fragment_list[j];
                fragment_list[j] = temp;
            }
        }
    }
    // return fragment_list[0];

    // resolve blend
    var final_color = vec4(0.0);

    let sigma = 30.0; // No idea what that is
    var thickness = fragment_list[0].w / 2.0;
    let alpha = 0.5; // TODO should probably not be fixed

    for (var i = 0; i < counter; i += 1) {
        let frag = fragment_list[i];
        var col = vec4(frag.rgb, alpha);

        let col_rgb = col.rgb * col.a;
        col = vec4(col_rgb.rgb, col.a);

        final_color += col * (1.0 - final_color.a);
    }
    final_color += background_color * (1.0 - final_color.a);

    return final_color;
}