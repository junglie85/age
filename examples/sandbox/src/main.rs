use std::process::ExitCode;

use age::{
    math::Mat4, App, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc,
    BindingResource, BindingType, Buffer, BufferDesc, BufferType, Camera, Color, Error, Game,
    PipelineLayoutDesc, RenderPipeline, RenderPipelineDesc, ShaderDesc, TextureFormat,
    VertexBufferLayout, VertexBufferLayoutDesc, VertexFormat, VertexType,
};

struct Sandbox {
    geometry_vertices: Vec<GeometryVertex>,
    #[allow(dead_code)]
    global_bgl: BindGroupLayout,
    #[allow(dead_code)]
    instance_bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    view_proj_storage: Buffer,
    global_bg: BindGroup,
    instance_data_storage: Buffer,
    instance_bg: BindGroup,
    geometry_buffer: Buffer,
    instance_buffer: Buffer,
}

impl Game for Sandbox {
    fn on_start(app: &mut App) -> Result<Self, Error> {
        let geometry_vertices = Vec::from_iter(TRIANGLE);

        let global_bgl = app.device.create_bind_group_layout(&BindGroupLayoutDesc {
            label: Some("global"),
            entries: &[BindingType::Storage {
                read_only: true,
                min_size: std::mem::size_of::<Mat4>(),
            }],
        });
        let instance_bgl = app.device.create_bind_group_layout(&BindGroupLayoutDesc {
            label: Some("instance data"),
            entries: &[BindingType::Storage {
                read_only: true,
                min_size: std::mem::size_of::<InstanceData>(),
            }],
        });

        let layout = app.device.create_pipeline_layout(&PipelineLayoutDesc {
            label: Some("sprite forward"),
            bind_group_layouts: &[&global_bgl, &instance_bgl],
        });
        let shader = app.device.create_shader(&ShaderDesc {
            label: Some("sprite forward"),
            src: include_str!("sprite.wgsl"),
        });
        let pipeline = app.device.create_render_pipelne(&RenderPipelineDesc {
            label: Some("sprite forward"),
            layout: &layout,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Bgra8Unorm,
            buffers: &[GeometryVertex::layout(), InstanceVertex::layout()],
        });

        // ---

        let view_proj_storage = app.device.create_buffer(&BufferDesc {
            label: Some("view proj"),
            size: std::mem::size_of::<Mat4>(),
            ty: BufferType::Storage,
        });
        let global_bg = app.device.create_bind_group(&BindGroupDesc {
            label: Some("globals"),
            layout: &global_bgl,
            entries: &[BindingResource::Buffer(&view_proj_storage)],
        });

        let instance_data_storage = app.device.create_buffer(&BufferDesc {
            label: Some("instance data"),
            size: std::mem::size_of::<InstanceData>(),
            ty: BufferType::Storage,
        });
        let instance_bg = app.device.create_bind_group(&BindGroupDesc {
            label: Some("instance data"),
            layout: &instance_bgl,
            entries: &[BindingResource::Buffer(&instance_data_storage)],
        });

        let geometry_buffer = app.device.create_buffer(&BufferDesc {
            label: Some("geometry"),
            size: std::mem::size_of::<GeometryVertex>() * geometry_vertices.len(),
            ty: BufferType::Vertex,
        });
        app.device
            .write_buffer(&geometry_buffer, &geometry_vertices);

        let instance_buffer = app.device.create_buffer(&BufferDesc {
            label: Some("instances"),
            size: std::mem::size_of::<InstanceVertex>(),
            ty: BufferType::Vertex,
        });

        Ok(Self {
            geometry_vertices,
            global_bgl,
            instance_bgl,
            pipeline,
            view_proj_storage,
            global_bg,
            instance_data_storage,
            instance_bg,
            geometry_buffer,
            instance_buffer,
        })
    }

    fn on_update(&mut self, app: &mut App) {
        let (width, height) = app.window.get_size();
        let camera = Camera::new(0.0, width as f32, height as f32, 0.0);
        let view_projections = vec![camera.get_view_projection_matrix()];
        app.device
            .write_buffer(&self.view_proj_storage, &view_projections);

        let instance_data = [InstanceData {
            size: [400.0, 200.0],
            _pad1: [0.0; 2],
            color: Color::BLUE.to_array_f32(),
        }];
        app.device
            .write_buffer(&self.instance_data_storage, &instance_data);

        let instances = vec![InstanceVertex {
            view_proj_index: (view_projections.len() - 1) as u32,
            instance_index: (instance_data.len() - 1) as u32,
        }];
        app.device.write_buffer(&self.instance_buffer, &instances);

        let mut buf = app.interface.get_command_buffer();
        buf.begin_render_pass(&app.window, Some(Color::RED));
        buf.set_bind_group(0, &self.global_bg);
        buf.set_bind_group(1, &self.instance_bg);
        buf.set_vertex_buffer(0, &self.geometry_buffer);
        buf.set_vertex_buffer(1, &self.instance_buffer);
        buf.set_render_pipeline(&self.pipeline); // this will come from the sprite's material. could be a default pipeline based on the renderer/pass type?

        // todo: next index buffer.
        buf.draw(0..self.geometry_vertices.len(), 0..instances.len());

        app.proxy.enqueue(buf);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}

const TRIANGLE: [GeometryVertex; 3] = [
    GeometryVertex {
        position: [0.0, 0.0],
    },
    GeometryVertex {
        position: [0.5, 1.0],
    },
    GeometryVertex {
        position: [1.0, 0.0],
    },
];

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GeometryVertex {
    pub position: [f32; 2],
}

impl GeometryVertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::new(&VertexBufferLayoutDesc {
            stride: std::mem::size_of::<Self>(),
            ty: VertexType::Vertex,
            attribute_offset: 0,
            attributes: &[VertexFormat::Float32x2],
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InstanceVertex {
    pub view_proj_index: u32,
    pub instance_index: u32,
}

impl InstanceVertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::new(&VertexBufferLayoutDesc {
            stride: std::mem::size_of::<Self>(),
            ty: VertexType::Instance,
            attribute_offset: GeometryVertex::layout().len(),
            attributes: &[VertexFormat::Uint32, VertexFormat::Uint32],
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InstanceData {
    pub size: [f32; 2],
    pub _pad1: [f32; 2],
    pub color: [f32; 4],
}
