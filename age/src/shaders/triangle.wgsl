struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VsOut {
    var pos = vec4(0.0, 0.0, 0.0, 1.0);
    if id == 0 {
        pos = vec4(0.0, 0.5, 0.0, 1.0);
    } else if id == 1 {
        pos = vec4(-0.5, -0.5, 0.0, 1.0);
    } else if id == 2 {
        pos = vec4(0.5, -0.5, 0.0, 1.0);
    }

    let color = vec4(0.0, 1.0, 0.0, 1.0);

    return VsOut(pos, color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
