struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VsOut {
    let uv = vec2(f32((id << 1) & 2), f32(id & 2));
	let pos = vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);

    return VsOut(pos, uv);
}

@group(0) @binding(0)
var r_sampler: sampler;
@group(0) @binding(1)
var r_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(r_texture, r_sampler, in.uv);
}
