@group(0) @binding(0)
var<storage, read_write> layers: array<vec4<f32>>;

@group(1) @binding(0)
var<storage, read_write> layer_ids: array<atomic<i32>>;

var<private> fragment_list: array<vec4<f32>, 32>;

const oit_layers: i32 = #{OIT_LAYERS};

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let viewport_width = 1280.0;
    let viewport_height = 720.0;
    let buffer_size = i32(viewport_width * viewport_height);
    let screen_index = i32(floor(in.position.x) + floor(in.position.y)* viewport_width);

    // show top layer
    // let color = layers[screen_index];
    // if all(color == vec4(0.0)) {
    //     discard;
    // } else {
    //     return color;
    // }

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
    // let counter = atomicLoad(&layer_ids[screen_index]);
    // if counter == 0 {
    //     discard;
    // } else {
    //     let x = f32(counter) / 8.0;
    //     return vec4(x, x, x, 1.0);
    // }
}

fn clear(screen_index: i32) {
    atomicStore(&layer_ids[screen_index], 0);
    layers[screen_index] = vec4(0.0);
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
            if fragment_list[j].w > fragment_list[j + 1].w {
                let temp = fragment_list[j + 1];
                fragment_list[j + 1] = fragment_list[j];
                fragment_list[j] = temp;
            }
        }
    }

    // resolve blend
    var final_color = vec4(0.0);
    let alpha = 0.1; // TODO should not be fixed
    for (var i = 0; i <= counter; i += 1) {
    // for (var i = counter; i >= 0; i -= 1) {
        let frag = fragment_list[i];

        var base_color = vec4(frag.rgb * alpha, alpha);

        // OVER operator using premultiplied alpha
        // see: https://en.wikipedia.org/wiki/Alpha_compositing
        var final_color_rgb = final_color.rgb;
        final_color_rgb += (1.0 - final_color.a) * base_color.rgb;
        final_color.a += (1.0 - final_color.a) * base_color.a;
        final_color.r = final_color_rgb.r;
        final_color.g = final_color_rgb.g;
        final_color.b = final_color_rgb.b;
    }
    // TODO consider blending with background manually
    // final_color += (1.0 - final_color.a) * background;

    return final_color;
}