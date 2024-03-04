struct GeometryVertex {
    @builtin(vertex_index) id: u32,
    @location(0) position: vec2<f32>,
}

struct PushConstant {
    model: mat4x4<f32>,
    color: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> r_view_projection: array<mat4x4<f32>>;

var r_push_constant: PushConstant;

@vertex
fn vs_main(vertex: GeometryVertex) -> VsOut {
    let view_proj = r_view_projection[instance_vertex.view_proj_index];

    var position = vec4(vertex.position, 0.0, 1.0);
    var final_pos = view_proj * r_push_constant.model * position;

    return VsOut(final_pos, r_push_constant.color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
