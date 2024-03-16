use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use age_math::{v2, Mat4, Vec2};
use bytemuck::{cast_slice, Pod, Zeroable};

use crate::{
    renderer::{
        self, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo, Binding, BindingType,
        Buffer, BufferInfo, BufferType, Color, DrawCommand, DrawTarget, IndexFormat, IndexedDraw,
        PipelineLayoutInfo, RenderDevice, RenderPipeline, RenderPipelineInfo, Sampler, ShaderInfo,
        Texture, TextureFormat, VertexBufferLayout, VertexFormat, VertexType,
    },
    AddressMode, FilterMode, SamplerInfo, TextureInfo, TextureView, TextureViewInfo,
};

pub struct Graphics {
    draw_state: DrawState,
    camera_bgl: BindGroupLayout,
    texture_bgl: BindGroupLayout,
    default_sampler: Sampler,
    #[allow(dead_code)]
    default_texture: Texture,
    #[allow(dead_code)]
    default_texture_view: TextureView,
    default_texture_bg: BindGroup,
    pipeline: RenderPipeline,
    camera: Camera,
    meshes: Meshes,
}

impl Graphics {
    pub const VERTEX_TYPE_FILL: f32 = 1.0;
    pub const VERTEX_TYPE_OUTLINE: f32 = 2.0;

    pub(crate) fn new(left: f32, right: f32, bottom: f32, top: f32, device: &RenderDevice) -> Self {
        let shader = device.create_shader(&ShaderInfo {
            label: Some("graphics"),
            src: include_str!("shaders/graphics.wgsl"),
        });

        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("graphics camera"),
            entries: &[BindingType::Uniform {
                min_size: std::mem::size_of::<[f32; 16]>() as u64,
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("graphics texture"),
            entries: &[
                BindingType::Sampler,
                BindingType::Texture { sample_count: 1 },
            ],
        });

        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("graphics"),
            bind_group_layouts: &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[&(0..std::mem::size_of::<PushConstant>() as u32)],
        });

        let default_sampler = device.create_sampler(&SamplerInfo {
            label: Some("graphics default"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
        });

        let default_texture = device.create_texture(&TextureInfo {
            label: Some("graphics default"),
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            renderable: false,
            sample_count: 1,
        });
        device.write_texture(&default_texture, &Color::WHITE.to_array_u8());

        let default_texture_view = default_texture.create_view(&TextureViewInfo {
            label: Some("graphics default"),
        });

        let default_texture_bg = device.create_bind_group(&BindGroupInfo {
            label: Some("graphics texture"),
            layout: &texture_bgl,
            entries: &[
                Binding::Sampler {
                    sampler: &default_sampler,
                },
                Binding::Texture {
                    texture_view: &default_texture_view,
                },
            ],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("graphics"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Rgba8Unorm,
            buffers: &[Vertex::layout()],
        });

        let camera = Camera::new(left, right, bottom, top, &camera_bgl, device);
        camera.update(device);

        let meshes = Meshes::new(device);

        Self {
            draw_state: DrawState::default(),
            camera_bgl,
            texture_bgl,
            default_sampler,
            default_texture,
            default_texture_view,
            default_texture_bg,
            pipeline,
            camera,
            meshes,
        }
    }

    pub fn default_sampler(&self) -> &Sampler {
        &self.default_sampler
    }

    pub fn texture_bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_bgl
    }

    pub(crate) fn begin_frame(&mut self, target: impl Into<DrawTarget>) {
        self.draw_state = DrawState::default();
        self.set_draw_target(target);
        self.set_camera(&self.camera.clone());
        self.set_render_pipeline(&self.pipeline.clone());
    }

    pub fn set_camera(&mut self, camera: &Camera) {
        let current_camera = match self.draw_state.cameras.iter().find(|&c| c == camera) {
            Some(camera) => camera,
            None => {
                self.draw_state.cameras.push(camera.clone());
                camera
            }
        };

        self.draw_state.current_camera = Some(current_camera.bind_group().clone());
    }

    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.draw_state.target = Some(target.into());
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.draw_state.pipeline = Some(pipeline.clone());
    }

    pub fn draw_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        color: Color,
        device: &mut RenderDevice,
    ) {
        let Some(target) = self.draw_state.target.as_ref() else {
            panic!("draw target is not set");
        };

        let Some(camera) = self.draw_state.current_camera.as_ref() else {
            panic!("camera is not set");
        };

        let Some(pipeline) = self.draw_state.pipeline.as_ref() else {
            panic!("render pipeline is not set");
        };

        let mut bind_groups = [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS];
        bind_groups[0] = Some(camera.clone());
        bind_groups[1] = Some(self.default_texture_bg.clone());

        let translation = (position - origin).floor();
        let model = Mat4::from_translation(translation.extend(0.0))
            * Mat4::from_translation(origin.extend(0.0))
            * Mat4::from_rotation_z(rotation)
            * Mat4::from_translation(-origin.extend(0.0))
            * Mat4::from_scale(scale.extend(1.0));
        let push_constant = PushConstant {
            model: model.to_cols_array(),
            color: color.to_array_f32(),
            info: [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
        };
        let push_constant_data = Some(cast_slice(&[push_constant]).to_vec());

        let mut vertex_buffers =
            [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS];
        vertex_buffers[0] = Some(self.meshes.rect.vbo.clone());

        let indexed_draw = Some(IndexedDraw {
            buffer: self.meshes.rect.ibo.clone(),
            format: IndexFormat::Uint16,
            indices: 0..self.meshes.rect.indices as u32,
        });

        // todo: this is pretty ugly, can we Default DrawCommand?
        // todo: push constants is a vec allocation each time. Can't be Any because need Pod + Zeroable. Can't be Pod + Zeroable because they need Sized, so can't be a trait object. Can allocate in command buffer then reference, but get's complicated if we ever want to combine more than one command buffer. Plus we potentially end up with lifetimes everywhere. Yay Rust!
        device.push_draw_command(DrawCommand {
            target: target.clone(),
            bind_groups,
            pipeline: pipeline.clone(),
            push_constant_data,
            vertex_buffers,
            vertices: 0..3,
            indexed_draw,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect_outline(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        thickness: f32,
        color: Color,
        device: &mut RenderDevice,
    ) {
        let Some(target) = self.draw_state.target.as_ref() else {
            panic!("draw target is not set");
        };

        let Some(camera) = self.draw_state.current_camera.as_ref() else {
            panic!("camera is not set");
        };

        let Some(pipeline) = self.draw_state.pipeline.as_ref() else {
            panic!("render pipeline is not set");
        };

        let mut bind_groups = [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS];
        bind_groups[0] = Some(camera.clone());
        bind_groups[1] = Some(self.default_texture_bg.clone());

        let translation = (position - origin).floor();
        let model = Mat4::from_translation(translation.extend(0.0))
            * Mat4::from_translation(origin.extend(0.0))
            * Mat4::from_rotation_z(rotation)
            * Mat4::from_translation(-origin.extend(0.0))
            * Mat4::from_scale(scale.extend(1.0));
        let push_constant = PushConstant {
            model: model.to_cols_array(),
            color: color.to_array_f32(),
            info: [Self::VERTEX_TYPE_OUTLINE, thickness, 0.0, 0.0],
        };
        let push_constant_data = Some(cast_slice(&[push_constant]).to_vec());

        let mut vertex_buffers =
            [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS];
        vertex_buffers[0] = Some(self.meshes.rect_outline.vbo.clone());

        let indexed_draw = Some(IndexedDraw {
            buffer: self.meshes.rect_outline.ibo.clone(),
            format: IndexFormat::Uint16,
            indices: 0..self.meshes.rect_outline.indices as u32,
        });

        // todo: this is pretty ugly, can we Default DrawCommand?
        // todo: push constants is a vec allocation each time. Can't be Any because need Pod + Zeroable. Can't be Pod + Zeroable because they need Sized, so can't be a trait object. Can allocate in command buffer then reference, but get's complicated if we ever want to combine more than one command buffer. Plus we potentially end up with lifetimes everywhere. Yay Rust!
        device.push_draw_command(DrawCommand {
            target: target.clone(),
            bind_groups,
            pipeline: pipeline.clone(),
            push_constant_data,
            vertex_buffers,
            vertices: 0..3,
            indexed_draw,
        });
    }

    // todo: extract draw_* logic into a single function that all other functions call.

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect_textured(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        bg: &BindGroup, // todo: expose bind group here or manage it internally? How would we handle non-default samplers? draw_rect_textured_ext()?
        color: Color,
        device: &mut RenderDevice,
    ) {
        let Some(target) = self.draw_state.target.as_ref() else {
            panic!("draw target is not set");
        };

        let Some(camera) = self.draw_state.current_camera.as_ref() else {
            panic!("camera is not set");
        };

        let Some(pipeline) = self.draw_state.pipeline.as_ref() else {
            panic!("render pipeline is not set");
        };

        let mut bind_groups = [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS];
        bind_groups[0] = Some(camera.clone());
        bind_groups[1] = Some(bg.clone());

        let translation = (position - origin).floor();
        let model = Mat4::from_translation(translation.extend(0.0))
            * Mat4::from_translation(origin.extend(0.0))
            * Mat4::from_rotation_z(rotation)
            * Mat4::from_translation(-origin.extend(0.0))
            * Mat4::from_scale(scale.extend(1.0));
        let push_constant = PushConstant {
            model: model.to_cols_array(),
            color: color.to_array_f32(),
            info: [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
        };
        let push_constant_data = Some(cast_slice(&[push_constant]).to_vec());

        let mut vertex_buffers =
            [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS];
        vertex_buffers[0] = Some(self.meshes.rect.vbo.clone());

        let indexed_draw = Some(IndexedDraw {
            buffer: self.meshes.rect.ibo.clone(),
            format: IndexFormat::Uint16,
            indices: 0..self.meshes.rect.indices as u32,
        });

        // todo: this is pretty ugly, can we Default DrawCommand?
        // todo: push constants is a vec allocation each time. Can't be Any because need Pod + Zeroable. Can't be Pod + Zeroable because they need Sized, so can't be a trait object. Can allocate in command buffer then reference, but get's complicated if we ever want to combine more than one command buffer. Plus we potentially end up with lifetimes everywhere. Yay Rust!
        device.push_draw_command(DrawCommand {
            target: target.clone(),
            bind_groups,
            pipeline: pipeline.clone(),
            push_constant_data,
            vertex_buffers,
            vertices: 0..3,
            indexed_draw,
        })
    }

    pub fn create_camera(
        &self,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        device: &RenderDevice,
    ) -> Camera {
        Camera::new(left, right, bottom, top, &self.camera_bgl, device)
    }

    pub fn default_camera(&self) -> &Camera {
        // todo: need to be able to control whether it updates with the view or stays fixed.
        &self.camera
    }
}

#[derive(Default)]
struct DrawState {
    cameras: Vec<Camera>,
    current_camera: Option<BindGroup>,
    target: Option<DrawTarget>,
    pipeline: Option<RenderPipeline>,
}

#[derive(Debug, Clone)]
pub struct Camera {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    pos: Vec2,
    zoom: f32,
    rotation: f32,
    ubo: Buffer,
    bg: BindGroup,
    dirty: Arc<AtomicBool>,
}

impl Camera {
    pub fn new(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        bgl: &BindGroupLayout,
        device: &RenderDevice,
    ) -> Self {
        let ubo = device.create_buffer(&BufferInfo {
            label: Some("camera"),
            size: std::mem::size_of::<[f32; 16]>() as u64,
            ty: BufferType::Uniform,
        });
        let bg = device.create_bind_group(&BindGroupInfo {
            label: Some("camera"),
            layout: bgl,
            entries: &[Binding::Uniform { buffer: &ubo }],
        });

        let dirty = Arc::new(AtomicBool::new(true));

        Self {
            left,
            right,
            bottom,
            top,
            pos: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            ubo,
            bg,
            dirty,
        }
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bg
    }

    pub fn buffer(&self) -> &Buffer {
        &self.ubo
    }

    pub fn update(&self, device: &RenderDevice) {
        if self.dirty.swap(false, Ordering::Relaxed) {
            device.write_buffer(&self.ubo, &self.view_projection_matrix().to_cols_array());
        }
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let left = self.left / self.zoom;
        let right = self.right / self.zoom;
        let bottom = self.bottom / self.zoom;
        let top = self.top / self.zoom;
        let proj = Mat4::orthographic_rh(left, right, bottom, top, 100.0, 0.0);

        let width = self.right - self.left;
        let height = self.bottom - self.top;
        let origin = self.pos + v2(width, height) / 2.0;
        let view = (Mat4::from_translation(self.pos.extend(0.0))
            * Mat4::from_translation(origin.extend(0.0))
            * Mat4::from_rotation_z(self.rotation)
            * Mat4::from_translation(-origin.extend(0.0))
            * Mat4::from_scale(Vec2::ONE.extend(1.0)))
        .inverse();

        proj * view
    }
}

impl PartialEq for Camera {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left
            && self.right == other.right
            && self.bottom == other.bottom
            && self.top == other.top
            && self.pos == other.pos
            && self.zoom == other.zoom
            && self.rotation == other.rotation
            && self.ubo == other.ubo
            && self.bg == other.bg
            && self.dirty.load(Ordering::Relaxed) == other.dirty.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstant {
    pub model: [f32; 16],
    pub color: [f32; 4],
    pub info: [f32; 4], // [0 => vertex type, 1 => thickness, 2 => unused, 3 => unused]
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: std::mem::size_of::<Self>() as u64,
            ty: VertexType::Vertex,
            formats: &[
                VertexFormat::Float32x2,
                VertexFormat::Float32x2,
                VertexFormat::Float32x2,
            ],
        }
    }
}

const fn v(position: [f32; 2], normal: [f32; 2], uv: [f32; 2]) -> Vertex {
    Vertex {
        position,
        normal,
        uv,
    }
}

struct Mesh {
    vbo: Buffer,
    ibo: Buffer,
    indices: usize,
}

impl Mesh {
    fn new(
        vertices: &[Vertex],
        indices: &[u16],
        label: Option<&str>,
        device: &RenderDevice,
    ) -> Self {
        let mut aligned_indices = indices.to_vec();
        let current_len = aligned_indices.len();
        let required_len =
            renderer::align_to(current_len, RenderDevice::COPY_BUFFER_ALIGNMENT as usize);
        if required_len != current_len {
            aligned_indices.resize(required_len, 0);
        }

        let vbo = device.create_buffer(&BufferInfo {
            label,
            size: std::mem::size_of::<Vertex>() as u64 * vertices.len() as u64,
            ty: BufferType::Vertex,
        });
        let ibo = device.create_buffer(&BufferInfo {
            label,
            size: std::mem::size_of::<u16>() as u64 * aligned_indices.len() as u64,
            ty: BufferType::Index,
        });

        device.write_buffer(&vbo, vertices);
        device.write_buffer(&ibo, indices);

        Self {
            vbo,
            ibo,
            indices: indices.len(),
        }
    }
}

struct Meshes {
    rect: Mesh,
    rect_outline: Mesh,
}

impl Meshes {
    const RECT: [Vertex; 4] = [
        v([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]),
        v([0.0, 1.0], [0.0, 0.0], [0.0, 1.0]),
        v([1.0, 1.0], [0.0, 0.0], [1.0, 1.0]),
        v([1.0, 0.0], [0.0, 0.0], [1.0, 0.0]),
    ];
    const RECT_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    fn new(device: &RenderDevice) -> Self {
        let rect = Mesh::new(&Self::RECT, &Self::RECT_INDICES, Some("rect"), device);

        let (vertices, indices) = compute_outline(&Self::RECT);
        let rect_outline = Mesh::new(&vertices, &indices, Some("rect outline"), device);

        Self { rect, rect_outline }
    }
}

fn compute_outline(vertices: &[Vertex]) -> (Vec<Vertex>, Vec<u16>) {
    let point_count = vertices.len();
    let vertex_count = point_count * 2;
    let index_count = point_count * 6;

    // Compute center of the shape, used for pointing the normals outwards.
    let center = geometric_center(vertices);

    let mut outline_vertices = vec![v([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]); vertex_count];
    let mut indices = vec![0_u16; index_count];

    for i in 0..point_count {
        // https://stackoverflow.com/questions/68973103/how-to-create-outline?noredirect=1&lq=1
        let p = if i == 0 { point_count - 1 } else { i - 1 };

        let p1 = v2(vertices[p].position[0], vertices[p].position[1]);
        let p2 = v2(vertices[i].position[0], vertices[i].position[1]);
        let p3 = v2(
            vertices[(i + 1) % point_count].position[0],
            vertices[(i + 1) % point_count].position[1],
        );

        // Compute normals.
        let mut n12 = age_math::normal(p1, p2);
        let mut n23 = age_math::normal(p2, p3);

        // Point outwards.
        // Use dot product of normal and direction of center to current point (center - p2) to decide if inward or outward.
        if n12.dot(center - p2) > 0.0 {
            n12 = -n12;
        }
        if n23.dot(center - p2) > 0.0 {
            n23 = -n23;
        }

        let normal = (n12 + n23).normalize();

        // Construct vertex array such that inside point index % 2 == 0 and outline point % 2 == 1.
        // This allows us to apply a outline thickness weighting to the correct points in the shader.
        outline_vertices[2 * i].position = p2.to_array();
        outline_vertices[2 * i].normal = [0.0; 2];
        outline_vertices[2 * i + 1].position = p2.to_array();
        outline_vertices[2 * i + 1].normal = normal.to_array();

        // Modulo vertex count because the final set of indices needs to wrap back around to the first vertices.
        indices[6 * i] = (2 * i as u16) % vertex_count as u16; // i.e. 0
        indices[6 * i + 1] = ((2 * i as u16) + 1) % vertex_count as u16; // i.e. 1
        indices[6 * i + 2] = ((2 * i as u16) + 2) % vertex_count as u16; // i.e. 2
        indices[6 * i + 3] = ((2 * i as u16) + 2) % vertex_count as u16; // i.e. 2
        indices[6 * i + 4] = ((2 * i as u16) + 1) % vertex_count as u16; // i.e. 1
        indices[6 * i + 5] = ((2 * i as u16) + 3) % vertex_count as u16; // i.e. 3
    }

    (outline_vertices, indices)
}

fn geometric_center(vertices: &[Vertex]) -> Vec2 {
    let point_count = vertices.len();

    // https://stackoverflow.com/questions/34059116/what-is-the-fastest-way-to-find-the-center-of-an-irregular-convex-polygon
    let mut sum_center = Vec2::ZERO;
    let mut sum_weight = 0.0;

    for i in 0..point_count {
        let p = if i == 0 { point_count - 1 } else { i - 1 };

        let p1 = v2(vertices[p].position[0], vertices[p].position[1]);
        let p2 = v2(vertices[i].position[0], vertices[i].position[1]);
        let p3 = v2(
            vertices[(i + 1) % point_count].position[0],
            vertices[(i + 1) % point_count].position[1],
        );

        let weight = (p2 - p3).length() + (p2 - p1).length();
        sum_center += p2 * weight;
        sum_weight += weight;
    }

    sum_center / sum_weight
}
