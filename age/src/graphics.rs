use crate::{
    gen_vec::{GenIdx, GenVec},
    math::{v2, Mat4, Vec2f},
    renderer::{
        cast_slice, BindGroupDesc, BindGroupId, BindGroupLayoutDesc, BindGroupLayoutId,
        BindingResource, BindingType, BufferDesc, BufferId, BufferUsages, CommandBuffer,
        DrawCommand, DrawTarget, GeometryVertex, PipelineLayoutDesc, PipelineLayoutId, RenderData,
        RenderPipelineDesc, RenderPipelineId, Renderer, ShaderDesc, ShaderId, TextureFormat,
    },
    Color,
};

pub struct Graphics {
    default_pl: PipelineLayoutId,
    default_pipeline: RenderPipelineId,
    default_shader: ShaderId,
    default_material: MaterialId,
    default_view: View,
    #[allow(dead_code)]
    globals_bgl: BindGroupLayoutId,
    globals_bg: BindGroupId,
    globals_sbo: BufferId,

    materials: GenVec<Material>,

    draw_target: DrawTarget,
    clear_color: Option<Color>,
    needs_render_pass: bool,
    draws: CommandBuffer,
    views: Vec<View>,
}

impl Graphics {
    pub(crate) fn new(renderer: &mut Renderer, default_view: View) -> Self {
        let label = Some("graphics default");

        let globals_bgl = renderer.create_bind_group_layout(&BindGroupLayoutDesc {
            label,
            entries: &[BindingType::StorageBuffer {
                read_only: true,
                min_size: std::mem::size_of::<Mat4>(),
            }],
        });

        let default_pl = renderer.create_pipeline_layout(&PipelineLayoutDesc {
            label,
            bind_group_layouts: &[globals_bgl],
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

        let globals_sbo = renderer.create_buffer(&BufferDesc {
            label,
            size: std::mem::size_of::<Mat4>(),
            usage: BufferUsages::STORAGE,
        });

        let globals_bg = renderer.create_bind_group(&BindGroupDesc {
            label,
            layout: globals_bgl,
            resources: &[BindingResource::StorageBuffer(globals_sbo)],
        });

        let mut graphics = Self {
            // default_bgl,
            default_pl,
            default_pipeline,
            default_shader,
            default_material: MaterialId::INVALID,
            default_view,
            globals_bgl,
            globals_bg,
            globals_sbo,

            materials: GenVec::default(),

            draw_target: DrawTarget::INVALID,
            clear_color: None,
            needs_render_pass: true,
            draws: CommandBuffer::default(),
            views: Vec::new(),
        };

        graphics.default_material = graphics.create_material(&MaterialDesc {
            label,
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

    pub fn default_pipeline(&self) -> RenderPipelineId {
        self.default_pipeline
    }

    pub fn default_pipeline_layout(&self) -> PipelineLayoutId {
        self.default_pl
    }

    pub fn default_shader(&self) -> ShaderId {
        self.default_shader
    }

    pub fn globals_bind_group_layout(&self) -> BindGroupLayoutId {
        self.globals_bgl
    }

    pub(crate) fn data(&self) -> RenderData {
        // todo: where does the buffer get resized if the data is larger?
        let mut data = Vec::with_capacity(std::mem::size_of::<[f32; 16]>() * self.views.len());
        for v in self.views.iter() {
            data.extend(cast_slice(&v.view_projection().to_cols_array()));
        }

        RenderData {
            dest: self.globals_sbo,
            size: std::mem::size_of::<Mat4>() * self.views.len(),
            data,
        }
    }

    pub(crate) fn draws(&self) -> &CommandBuffer {
        &self.draws
    }

    pub(crate) fn reset(&mut self) {
        self.draws.clear();
        self.views.clear();
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

            // todo: these need to move to a per-scene ubo.
            globals_bg: self.globals_bg,

            // todo: these need to move to a per-object ubo.
            color: sprite.color,
            model: sprite.get_transform(),
            globals_idx: self.views.len() - 1,
        });
    }

    pub fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T) {
        self.draw_target = target.into();
        self.clear_color = None;
        self.needs_render_pass = true;
    }

    pub fn get_default_view(&self) -> View {
        self.default_view
    }

    pub fn set_view(&mut self, view: View) {
        self.views.push(view);
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
    origin: Vec2f,
    position: Vec2f,
    rotation: f32,
    scale: Vec2f,

    mesh: Mesh,
}

impl Sprite {
    const INDICES: [u16; 8] = [0, 1, 2, 0, 2, 3, 0, 0]; // todo: Index alignment.
    const VERTICES: [GeometryVertex; 4] = [
        GeometryVertex { pos: [0.0, 0.0] },
        GeometryVertex { pos: [1.0, 0.0] },
        GeometryVertex { pos: [1.0, 1.0] },
        GeometryVertex { pos: [0.0, 1.0] },
    ];

    pub fn from_image(
        renderer: &mut Renderer,
        width: u32,
        height: u32,
        material: MaterialId,
    ) -> Self {
        let vertices = Self::VERTICES
            .iter()
            .map(|v| {
                let mut v = *v;
                v.pos[0] *= width as f32;
                v.pos[1] *= height as f32;
                v
            })
            .collect::<Vec<_>>();

        let vbo = renderer.create_buffer(&BufferDesc {
            label: Some("sprite"),
            size: std::mem::size_of::<[GeometryVertex; 4]>(),
            usage: BufferUsages::VERTEX,
        });
        renderer.write_buffer(vbo, &vertices);

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
            origin: Vec2f::ZERO,
            position: Vec2f::ZERO,
            rotation: 0.0,
            scale: Vec2f::ONE,
            mesh,
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn get_transform(&self) -> Mat4 {
        Mat4::translation(self.position)
            * Mat4::translation(self.origin)
            * Mat4::rotation(self.rotation)
            * Mat4::translation(-self.origin)
            * Mat4::scale(self.scale)
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

#[derive(Debug, Clone, Copy)]
pub struct View {
    width: u32,
    height: u32,
    position: Vec2f,
    rotation: f32,
    zoom: f32,
}

impl View {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            position: Vec2f::ZERO,
            rotation: 0.0,
            zoom: 1.0,
        }
    }

    pub fn get_position(&self) -> Vec2f {
        self.position
    }

    pub fn set_position(&mut self, position: Vec2f) {
        self.position = position;
    }

    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
    }

    pub fn view_projection(&self) -> Mat4 {
        let width = self.width as f32 / self.zoom;
        let height = self.height as f32 / self.zoom;
        let proj = Mat4::ortho(width, height, 0.0, 100.0);

        let origin = self.position + v2(self.width as f32, self.height as f32) / 2.0;
        let view = (Mat4::translation(self.position)
            * Mat4::translation(origin)
            * Mat4::rotation(self.rotation)
            * Mat4::translation(-origin)
            * Mat4::scale(Vec2f::ONE))
        .inverse();

        proj * view
    }
}
