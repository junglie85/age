struct GeometryVertex {
    @location(0) pos: vec2<f32>,
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> r_globals: array<mat4x4<f32>>;

struct PushConstant {
    color: vec4<f32>,
    model: mat4x4<f32>,
    globals_idx: u32,
}

var<push_constant> r_pc: PushConstant;

@vertex
fn vs_main(vertex: GeometryVertex) -> VsOut {
    let view_proj = r_globals[r_pc.globals_idx];
    let model = r_pc.model;

    let pos = view_proj * model * vec4(vertex.pos, 0.0, 1.0);
    let color = r_pc.color;

    return VsOut(
        pos,
        color
    );
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
