use std::process::ExitCode;

use age::{
    math::Mat4, App, BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc,
    BindingResource, BindingType, Buffer, BufferDesc, BufferType, Camera, Color, Error, Game,
    PipelineLayoutDesc, RenderPipeline, RenderPipelineDesc, ShaderDesc, TextureFormat,
    VertexBufferLayout, VertexBufferLayoutDesc, VertexFormat, VertexType,
};

struct Sandbox {
    #[allow(dead_code)]
    global_bgl: BindGroupLayout,
    pipeline: RenderPipeline,
    view_proj_uniform: Buffer,
    global_bg: BindGroup,
    instance_buffer: Buffer,
}

impl Game for Sandbox {
    fn on_start(app: &mut App) -> Result<Self, Error> {
        let global_bgl = app.device.create_bind_group_layout(&BindGroupLayoutDesc {
            label: Some("global"),
            entries: &[BindingType::Storage {
                read_only: true,
                min_size: std::mem::size_of::<Mat4>(),
            }],
        });

        let layout = app.device.create_pipeline_layout(&PipelineLayoutDesc {
            label: Some("sprite forward"),
            bind_group_layouts: &[&global_bgl],
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
            buffers: &[InstanceVertex::layout()],
        });

        // ---

        let view_proj_uniform = app.device.create_buffer(&BufferDesc {
            label: Some("view proj"),
            size: std::mem::size_of::<Mat4>(),
            ty: BufferType::Storage,
        });
        let global_bg = app.device.create_bind_group(&BindGroupDesc {
            label: Some("globals"),
            layout: &global_bgl,
            entries: &[BindingResource::Buffer(&view_proj_uniform)],
        });

        let instance_buffer = app.device.create_buffer(&BufferDesc {
            label: Some("instances"),
            size: std::mem::size_of::<InstanceVertex>(),
            ty: BufferType::Vertex,
        });

        Ok(Self {
            global_bgl,
            pipeline,
            view_proj_uniform,
            global_bg,
            instance_buffer,
        })
    }

    fn on_update(&mut self, app: &mut App) {
        let (width, height) = app.window.get_size();
        let camera = Camera::new(0.0, width as f32, height as f32, 0.0);
        let view_projections = vec![camera.get_view_projection_matrix()];
        app.device
            .write_buffer(&self.view_proj_uniform, &view_projections);

        let instances = vec![InstanceVertex {
            view_proj_index: (view_projections.len() - 1) as u32,
        }];
        app.device.write_buffer(&self.instance_buffer, &instances);

        let mut buf = app.interface.get_command_buffer();
        buf.begin_render_pass(&app.window, Some(Color::RED));
        buf.set_bind_group(0, &self.global_bg);
        buf.set_vertex_buffer(0, &self.instance_buffer);
        buf.set_render_pipeline(&self.pipeline); // this will come from the sprite's material. could be a default pipeline based on the renderer/pass type?
        buf.draw(0..3, 0..1);

        app.proxy.enqueue(buf);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InstanceVertex {
    pub view_proj_index: u32,
}

impl InstanceVertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout::new(&VertexBufferLayoutDesc {
            stride: std::mem::size_of::<Self>(),
            ty: VertexType::Instance,
            attribute_offset: 0,
            attributes: &[VertexFormat::Uint32],
        })
    }
}
