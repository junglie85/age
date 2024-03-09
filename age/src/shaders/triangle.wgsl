struct Camera {
    view_proj: mat4x4<f32>,
}

struct Vertex {
    @builtin(vertex_index) id: u32,
    @location(0) position: vec2<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> r_camera: Camera;

@vertex
fn vs_main(vertex: Vertex) -> VsOut {
    let position = r_camera.view_proj * vec4(vertex.position, 0.0, 1.0);
    let color = vec4(0.0, 1.0, 0.0, 1.0);

    return VsOut(position, color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
