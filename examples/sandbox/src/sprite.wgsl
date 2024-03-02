@group(0) @binding(0)
var<storage, read> r_view_projection: array<mat4x4<f32>>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var pos = vec4(0.0, 0.0, 0.0, 1.0);
    if index == 0 {
        pos = vec4(0.0, 0.0, 0.0, 1.0);
    } else if index == 1 {
        pos = vec4(200.0, 200.0, 0.0, 1.0);
    } else {
        pos = vec4(400.0, 0.0, 0.0, 1.0);
    }
    var final_pos = r_view_projection[0] * pos;

    return final_pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0.0, 1.0, 0.0, 1.0);
}
