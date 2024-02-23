use crate::{
    gen_vec::{GenIdx, GenVec},
    renderer::{
        BufferDesc, BufferId, BufferUsages, CommandBuffer, DrawCommand, DrawTarget, GeometryVertex,
        PipelineLayoutDesc, PipelineLayoutId, RenderPipelineDesc, RenderPipelineId, Renderer,
        ShaderDesc, ShaderId, TextureFormat,
    },
    Color,
};

pub struct Graphics {
    // default_bgl: BindGroupLayoutId,
    default_pl: PipelineLayoutId,
    default_pipeline: RenderPipelineId,
    default_shader: ShaderId,

    default_material: MaterialId,

    materials: GenVec<Material>,

    draw_target: DrawTarget,
    clear_color: Option<Color>,
    needs_render_pass: bool,
    draws: CommandBuffer,
}

impl Graphics {
    pub(crate) fn new(renderer: &mut Renderer) -> Self {
        let label = Some("graphics default");

        // let default_bgl = renderer.create_bind_group_layout(&BindGroupLayoutDesc {
        //     label,
        //     entries: &[
        //         BindingType::Sampler,
        //         BindingType::Texture {
        //             multisampled: false,
        //         },
        //     ],
        // });

        let default_pl = renderer.create_pipeline_layout(&PipelineLayoutDesc {
            label,
            // bind_group_layouts: &[default_bgl],
            bind_group_layouts: &[],
        });

        let default_shader = renderer.create_shader(ShaderDesc {
            label,
            source: include_str!("default.wgsl"),
        });

        let default_pipeline = renderer.create_render_pipeline(&RenderPipelineDesc {
            label,
            layout: default_pl,
            shader: default_shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            buffers: &[renderer.geometry_vertex_buffer_layout()],
            color_target_format: TextureFormat::Rgba8Unorm,
        });

        let mut graphics = Self {
            // default_bgl,
            default_pl,
            default_pipeline,
            default_shader,
            default_material: MaterialId::INVALID,
            materials: GenVec::default(),

            draw_target: DrawTarget::INVALID,
            clear_color: None,
            needs_render_pass: true,
            draws: CommandBuffer::default(),
        };

        graphics.default_material = graphics.create_material(&MaterialDesc {
            label: Some("graphics default"),
            pipeline: graphics.default_pipeline(),
        });

        graphics
    }

    pub fn create_material(&mut self, desc: &MaterialDesc) -> MaterialId {
        let material = Material {
            label: desc.label.map(|s| s.to_string()),
            pipeline: desc.pipeline,
        };

        MaterialId(self.materials.add(material))
    }

    pub fn default_material(&self) -> MaterialId {
        self.default_material
    }

    // pub fn default_bind_group_layout(&self) -> BindGroupLayoutId {
    //     self.default_bgl
    // }

    pub fn default_pipeline(&self) -> RenderPipelineId {
        self.default_pipeline
    }

    pub fn default_pipeline_layout(&self) -> PipelineLayoutId {
        self.default_pl
    }

    pub fn default_shader(&self) -> ShaderId {
        self.default_shader
    }

    pub(crate) fn draws(&self) -> &CommandBuffer {
        &self.draws
    }

    pub(crate) fn draws_mut(&mut self) -> &mut CommandBuffer {
        &mut self.draws
    }
}

impl Graphics {
    pub fn clear(&mut self, color: Color) {
        self.clear_color = Some(color);
        self.needs_render_pass = true;
        self.push_render_pass();
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite) {
        self.push_draw_command(DrawCommand {
            pipeline: self.materials[sprite.mesh.material.0].pipeline,
            vbo: sprite.mesh.buffers.vbo,
            ibo: sprite.mesh.buffers.ibo,
            index_count: 6,
        });
    }

    pub fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T) {
        self.draw_target = target.into();
        self.clear_color = None;
        self.needs_render_pass = true;
    }

    fn push_draw_command(&mut self, draw: DrawCommand) {
        if self.needs_render_pass {
            self.push_render_pass();
        }

        self.draws.record(draw);
    }

    fn push_render_pass(&mut self) {
        self.needs_render_pass = false;
        self.draws
            .set_render_pass(self.draw_target.texture_view(), self.clear_color);
    }
}

#[derive(Clone)]
pub struct Sprite {
    color: Color,
    width: u32,
    height: u32,

    mesh: Mesh,
}

impl Sprite {
    const INDICES: [u16; 8] = [0, 1, 2, 0, 2, 3, 0, 0]; // Index alignment.
    const VERTICES: [GeometryVertex; 4] = [
        GeometryVertex { pos: [0.0, 0.0] },
        GeometryVertex { pos: [0.0, 0.5] },
        GeometryVertex { pos: [0.5, 0.5] },
        GeometryVertex { pos: [0.5, 0.0] },
    ];

    pub fn from_image(
        renderer: &mut Renderer,
        width: u32,
        height: u32,
        material: MaterialId,
    ) -> Self {
        let vbo = renderer.create_buffer(&BufferDesc {
            label: Some("sprite"),
            size: std::mem::size_of::<[GeometryVertex; 4]>(),
            usage: BufferUsages::VERTEX,
        });
        renderer.write_buffer(vbo, &Self::VERTICES);

        let ibo = renderer.create_buffer(&BufferDesc {
            label: Some("sprite"),
            size: std::mem::size_of::<[u16; 8]>(),
            usage: BufferUsages::INDEX,
        });
        renderer.write_buffer(ibo, &Self::INDICES);

        let buffers = MeshBuffers { vbo, ibo };

        // let material = Material { pipeline };
        let mesh = Mesh { buffers, material };
        Self {
            color: Color::GREEN,
            width,
            height, // texture: Texture {},
            mesh,
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(GenIdx);

impl MaterialId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

pub struct MaterialDesc<'desc> {
    label: Option<&'desc str>,
    pipeline: RenderPipelineId,
}

pub struct Material {
    label: Option<String>,
    pipeline: RenderPipelineId,
    // texture: TextureId
}

#[derive(Clone)]
struct MeshBuffers {
    vbo: BufferId,
    ibo: BufferId,
}

#[derive(Clone)]
pub struct Mesh {
    buffers: MeshBuffers,
    material: MaterialId,
}
