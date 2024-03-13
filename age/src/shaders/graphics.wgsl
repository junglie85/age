struct Camera {
    view_proj: mat4x4<f32>,
}

struct Vertex {
    @builtin(vertex_index) id: u32,
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) uv: vec2<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}


struct PushConstant {
    model: mat4x4<f32>,
    color: vec4<f32>,
    info: vec4<f32>, // [0 => vertex type, 1 => thickness, 2 => unused, 3 => unused]
}

@group(0) @binding(0)
var<uniform> r_camera: Camera;

@group(1) @binding(0)
var r_sampler: sampler;
@group(1) @binding(1)
var r_texture: texture_2d<f32>;

var<push_constant> r_pc: PushConstant;

@vertex
fn vs_main(vertex: Vertex) -> VsOut {
    let ty = r_pc.info.x;
    let thickness = r_pc.info.y;
    let is_fill = select(false, true, ty >= 0.5 && ty < 1.5);
    let is_outline = select(false, true, ty >= 1.5 && ty < 2.5);

    var model = r_pc.model;
    var position = vec4(vertex.position, 0.0, 1.0);
    if is_outline {
        // Add thickness to model's scale components.
        var width = vertex.normal * thickness;
        model[0][0] += width.x;
        model[1][1] += width.y;
    }
    position = r_camera.view_proj * model * position;

    let color = r_pc.color;

    let uv = vertex.uv;

    return VsOut(position, color, uv);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let sample = textureSample(r_texture, r_sampler, in.uv) ;
    return sample * in.color;
}
