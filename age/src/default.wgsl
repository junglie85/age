struct GeometryVertex {
    @location(0) pos: vec2<f32>,
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct PushConstant {
    color: vec4<f32>,
}

var<push_constant> r_pc: PushConstant;

@vertex
fn vs_main(vertex: GeometryVertex) -> VsOut {
    let pos = vec4(vertex.pos, 0.0, 1.0);
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
