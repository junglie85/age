struct Camera {
    view_proj: mat4x4<f32>,
}

struct Vertex {
    @builtin(vertex_index) id: u32,
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}


struct PushConstant {
    model: mat4x4<f32>,
    color: vec4<f32>,
    info: vec4<f32>, // [0 => vertex type, 1 => thickness, 2 => unused, 3 => unused]
}

@group(0) @binding(0)
var<uniform> r_camera: Camera;

var<push_constant> r_pc: PushConstant;

@vertex
fn vs_main(vertex: Vertex) -> VsOut {
    // todo: select the vertex type and apply thickness if outline vertex.
    let position = r_camera.view_proj * r_pc.model * vec4(vertex.position, 0.0, 1.0);
    let color = r_pc.color;

    return VsOut(position, color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
