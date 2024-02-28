@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    if index == 0 {
        return vec4(0.0, 0.5, 0.0, 1.0);
    } else if index == 1 {
        return vec4(-0.5, -0.5, 0.0, 1.0);
    } else {
        return vec4(0.5, -0.5, 0.0, 1.0);
    }
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0.0, 1.0, 0.0, 1.0);
}
