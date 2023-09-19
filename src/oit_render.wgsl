#import bevy_render::view  View

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<storage, read_write> layers: array<vec2<u32>>;

@group(1) @binding(1)
var<storage, read_write> layer_ids: array<atomic<i32>>;

var<private> fragment_list: array<vec2<u32>, 32>;

const oit_layers: i32 = #{OIT_LAYERS};

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let buffer_size = i32(view.viewport.z * view.viewport.w);
    let screen_index = i32(floor(in.position.x) + floor(in.position.y)* view.viewport.z);

    let counter = atomicLoad(&layer_ids[screen_index]);
    if counter == 0 {
        clear(screen_index);
        discard;
    } else {
        let final_color = sort(screen_index, buffer_size);
        clear(screen_index);
        return final_color;
    }

    // show layer density
    // clear(screen_index);
    // if counter == 0 {
    //     discard;
    // } else {
    //     let x = f32(counter) / f32(oit_layers);
    //     return vec4(x, x, x, 0.1);
    // }
}

fn clear(screen_index: i32) {
    atomicStore(&layer_ids[screen_index], 0);
    layers[screen_index] = vec2(0u);
}

fn sort(screen_index: i32, buffer_size: i32) -> vec4<f32> {
    var counter = atomicLoad(&layer_ids[screen_index]);

    // fill list
    for (var i = 0; i < counter; i += 1){
       fragment_list[i] = layers[screen_index + buffer_size * i];
    }

    // bubble sort
    for (var i = counter; i >= 0; i -= 1){
        for (var j = 0; j < i; j += 1) {
            if bitcast<f32>(fragment_list[j].y) < bitcast<f32>(fragment_list[j + 1].y) {
                let temp = fragment_list[j + 1];
                fragment_list[j + 1] = fragment_list[j];
                fragment_list[j] = temp;
            }
        }
    }

    // resolve blend
    var final_color = vec4(0.0);
    for (var i = 0; i <= counter; i += 1) {
        let color = unpack4x8unorm(fragment_list[i].r);
        var base_color = vec4(color.rgb * color.a, color.a);
        final_color = blend(final_color, base_color);
    }

    return final_color;
}

// OVER operator using premultiplied alpha
// see: https://en.wikipedia.org/wiki/Alpha_compositing
fn blend(color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let final_color = color_a.rgb + (1.0 - color_a.a) * color_b.rgb;
    let alpha = color_a.a + (1.0 - color_a.a) * color_b.a;
    return vec4(final_color.rgb, alpha);
}