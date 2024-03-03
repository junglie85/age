struct InstanceVertex {
    @location(0) view_proj_index: u32,
    @location(1) instance_index: u32,
}

struct InstanceData {
    size: vec2<f32>,
}

@group(0) @binding(0)
var<storage, read> r_view_projection: array<mat4x4<f32>>;

@group(1) @binding(0)
var<storage, read> r_instance_data: array<InstanceData>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance_vertex: InstanceVertex) -> @builtin(position) vec4<f32> {
    let view_proj = r_view_projection[instance_vertex.view_proj_index];

    let instance = r_instance_data[instance_vertex.instance_index];
    let width = instance.size.x;
    let height = instance.size.y;

    var pos = vec4(0.0, 0.0, 0.0, 1.0);
    if index == 0 {
        pos = vec4(0.0, 0.0, 0.0, 1.0);
    } else if index == 1 {
        pos = vec4(width / 2.0, height, 0.0, 1.0);
    } else {
        pos = vec4(width, 0.0, 0.0, 1.0);
    }

    var final_pos = view_proj * pos;

    return final_pos;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0.0, 1.0, 0.0, 1.0);
}
