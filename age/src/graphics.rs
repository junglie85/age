use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use age_math::{v2, Mat4, Vec2};
use bytemuck::{cast_slice, Pod, Zeroable};

use crate::renderer::{
    self, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo, Binding, BindingType,
    Buffer, BufferInfo, BufferType, Color, DrawCommand, DrawTarget, IndexFormat, IndexedDraw,
    PipelineLayoutInfo, RenderDevice, RenderPipeline, RenderPipelineInfo, ShaderInfo,
    TextureFormat, VertexBufferLayout, VertexFormat, VertexType,
};

pub struct Graphics {
    draw_state: DrawState,
    camera_bgl: BindGroupLayout,
    triangle_pipeline: RenderPipeline,
    camera: Camera,
    meshes: Meshes,
}

impl Graphics {
    pub(crate) fn new(left: f32, right: f32, bottom: f32, top: f32, device: &RenderDevice) -> Self {
        let shader = device.create_shader(&ShaderInfo {
            label: Some("triangle"),
            src: include_str!("shaders/triangle.wgsl"),
        });

        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("camera"),
            entries: &[BindingType::Uniform {
                min_size: std::mem::size_of::<[f32; 16]>() as u64,
            }],
        });

        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("triangle"),
            bind_group_layouts: &[&camera_bgl],
            push_constant_ranges: &[&(0..std::mem::size_of::<PushConstant>() as u32)],
        });
        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("triangle"),
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
            triangle_pipeline,
            camera,
            meshes,
        }
    }

    pub(crate) fn begin_frame(&mut self, target: impl Into<DrawTarget>) {
        self.draw_state = DrawState::default();
        self.set_draw_target(target);
        self.set_camera(&self.camera.clone());
        self.set_render_pipeline(&self.triangle_pipeline.clone());
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

    pub fn draw_filled_triangle(
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

        let model = Mat4::from_translation(position.extend(0.0) - origin.extend(0.0))
            * Mat4::from_translation(origin.extend(0.0))
            * Mat4::from_rotation_z(rotation)
            * Mat4::from_translation(-origin.extend(0.0))
            * Mat4::from_scale(scale.extend(1.0));
        let push_constant = PushConstant {
            model: model.to_cols_array(),
            color: color.to_array_f32(),
        };
        let push_constant_data = Some(cast_slice(&[push_constant]).to_vec());

        let mut vertex_buffers =
            [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS];
        vertex_buffers[0] = Some(self.meshes.triangle.vbo.clone());

        let indexed_draw = Some(IndexedDraw {
            buffer: self.meshes.triangle.ibo.clone(),
            format: IndexFormat::Uint16,
            indices: 0..self.meshes.triangle.indices as u32,
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
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl Vertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: std::mem::size_of::<Self>() as u64,
            ty: VertexType::Vertex,
            formats: &[VertexFormat::Float32x2],
        }
    }
}

const fn v(pos: [f32; 2]) -> Vertex {
    Vertex { position: pos }
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
    triangle: Mesh,
}

impl Meshes {
    const TRIANGLE: [Vertex; 3] = [v([0.0, 0.0]), v([0.5, 1.0]), v([1.0, 0.0])];
    const TRIANGLE_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    fn new(device: &RenderDevice) -> Self {
        let triangle = Mesh::new(
            &Self::TRIANGLE,
            &Self::TRIANGLE_INDICES,
            Some("triangle"),
            device,
        );

        Self { triangle }
    }
}
