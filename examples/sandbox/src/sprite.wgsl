struct GeometryVertex {
    @builtin(vertex_index) id: u32,
    @location(0) position: vec2<f32>,
}

struct InstanceVertex {
    @location(1) view_proj_index: u32,
    @location(2) instance_index: u32,
}

struct InstanceData {
    size: vec2<f32>,
    _pad1: vec2<f32>,
    color: vec4<f32>,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> r_view_projection: array<mat4x4<f32>>;

@group(1) @binding(0)
var<storage, read> r_instance_data: array<InstanceData>;

@vertex
fn vs_main(vertex: GeometryVertex, instance_vertex: InstanceVertex) -> VsOut {
    let view_proj = r_view_projection[instance_vertex.view_proj_index];

    let instance = r_instance_data[instance_vertex.instance_index];
    let width = instance.size.x;
    let height = instance.size.y;

    var pos = vec4(vertex.position.x * width, vertex.position.y * height, 0.0, 1.0);

    var final_pos = view_proj * pos;

    return VsOut(final_pos, instance.color);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
