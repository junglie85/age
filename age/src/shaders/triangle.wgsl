struct Camera {
    view_proj: mat4x4<f32>,
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> r_camera: Camera;

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VsOut {
    var pos = vec4(0.0, 0.0, 0.0, 1.0);
    if id == 0 {
        pos = vec4(0.0, 0.0, 0.0, 1.0);
    } else if id == 1 {
        pos = vec4(100.0, 100.0, 0.0, 1.0);
    } else if id == 2 {
        pos = vec4(200.0, 0.0, 0.0, 1.0);
    }
    pos = r_camera.view_proj * pos;

    let color = vec4(0.0, 1.0, 0.0, 1.0);

    return VsOut(pos, color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
