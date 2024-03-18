use std::{
    borrow::Cow,
    hash::Hash,
    num::NonZeroU64,
    ops::{Add, Deref, Range, Rem, Sub},
    sync::Arc,
};

use bytemuck::cast_slice;
use wgpu::{
    BindGroupDescriptor, BindGroupLayoutDescriptor, BindingResource, BlendState, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CreateSurfaceError, Extent3d, Face, FragmentState, FrontFace,
    ImageDataLayout, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode,
    PresentMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, Surface, SurfaceConfiguration, SurfaceError,
    SurfaceTexture, TextureAspect, TextureDescriptor, TextureDimension, TextureSampleType,
    TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::window::Window;

use crate::{AgeError, AgeResult};

pub struct RenderDevice {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    command_buffer: CommandBuffer,
}

impl RenderDevice {
    pub const COPY_BUFFER_ALIGNMENT: u64 = wgpu::COPY_BUFFER_ALIGNMENT;
    pub const MAX_BIND_GROUPS: usize = 2;
    pub const EMPTY_BIND_GROUP: Option<BindGroup> = None;
    pub const MAX_VERTEX_BUFFERS: usize = 2;
    pub const EMPTY_VERTEX_BUFFER: Option<Buffer> = None;

    pub(crate) fn new() -> AgeResult<Self> {
        let flags = if cfg!(debug_assertions) {
            wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
        } else {
            wgpu::InstanceFlags::empty()
        };

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            flags,
            ..Default::default()
        });

        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })) {
                Some(adapter) => adapter,
                None => {
                    return Err("failed to get graphics adapter".into());
                }
            };

        let required_features = wgpu::Features::PUSH_CONSTANTS;
        assert!(adapter.features().contains(required_features));

        let required_limits = wgpu::Limits {
            max_push_constant_size: 128,
            ..Default::default()
        };
        let mut in_limits = true;
        required_limits.check_limits_with_fail_fn(
            &adapter.limits(),
            false,
            |name, wanted, allowed| {
                eprintln!(
                    "limit '{}' failed, wanted {} but allowed {}",
                    name, wanted, allowed
                );
                in_limits = false;
            },
        );
        assert!(in_limits);

        let (device, queue) = match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features,
                required_limits,
            },
            None,
        )) {
            Ok((device, queue)) => (device, queue),
            Err(_) => {
                return Err("failed to get graphics queue".into());
            }
        };

        let command_buffer = CommandBuffer::new();

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            command_buffer,
        })
    }

    pub(crate) fn begin_frame(&mut self) {
        // Can't clear here because we hold on to a surface texture view which prevents recreating the window surface.
        // self.command_buffer.clear();
    }

    #[allow(unused_assignments)]
    pub(crate) fn end_frame(&mut self) {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("end frame"),
            });

        let mut current_rpass = None;
        let mut current_target = None;
        let mut current_bind_groups = [Self::EMPTY_BIND_GROUP; Self::MAX_BIND_GROUPS];
        let mut current_pipeline = None;
        let mut current_vertex_buffers = [Self::EMPTY_VERTEX_BUFFER; Self::MAX_VERTEX_BUFFERS];
        let mut current_index_buffer: Option<Buffer> = None;

        for DrawCommand {
            clear_color,
            target,
            bind_groups,
            pipeline,
            push_constant_data,
            vertex_buffers,
            vertices,
            indexed_draw,
        } in self.command_buffer.commands.iter()
        {
            if Some(&target.color_target) != current_target.as_ref() || clear_color.is_some() {
                if Some(&target.color_target) != current_target.as_ref() {
                    current_target = Some(target.color_target.clone());
                }

                let view = &target.color_target;

                // This assignment is unused but we need to drop the current render pass because
                // it has an exclusive borrow of encoder.
                current_rpass = None;

                let ops = Operations {
                    load: match clear_color {
                        Some(color) => LoadOp::Clear((*color).into()),
                        None => LoadOp::Load,
                    },
                    store: StoreOp::Store,
                };

                current_rpass = Some(encoder.begin_render_pass(&RenderPassDescriptor {
                    label: target.label(),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }));
            }

            let Some(pass) = current_rpass.as_mut() else {
                unreachable!("render pass will always be set by this point");
            };

            if Some(pipeline) != current_pipeline.as_ref() {
                current_pipeline = Some(pipeline.clone());
                pass.set_pipeline(pipeline);
            }

            for (i, bind_group) in bind_groups.iter().enumerate() {
                if &current_bind_groups[i] != bind_group {
                    current_bind_groups[i] = bind_group.clone();
                    if let Some(bg) = bind_group.as_ref() {
                        pass.set_bind_group(i as u32, bg, &[]);
                    }
                }
            }

            if let Some(data) = push_constant_data {
                pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, data);
            }

            for (i, buffer) in vertex_buffers.iter().enumerate() {
                if &current_vertex_buffers[i] != buffer {
                    current_vertex_buffers[i] = buffer.clone();
                    if let Some(buf) = buffer.as_ref() {
                        pass.set_vertex_buffer(i as u32, buf.slice(..));
                    }
                }
            }

            if let Some(IndexedDraw {
                buffer,
                format,
                indices,
            }) = indexed_draw
            {
                if Some(buffer) != current_index_buffer.as_ref() {
                    current_index_buffer = Some(buffer.clone());
                    pass.set_index_buffer(buffer.slice(..), format.into());
                }

                pass.draw_indexed(indices.clone(), 0, 0..1);
            } else {
                pass.draw(vertices.clone(), 0..1);
            }
        }

        current_index_buffer = None;
        current_vertex_buffers.iter_mut().for_each(|b| *b = None);
        current_pipeline = None;
        current_bind_groups.iter_mut().for_each(|bg| *bg = None);
        current_target = None;
        current_rpass = None;

        self.queue.submit([encoder.finish()]);
        self.command_buffer.clear()
    }

    pub fn create_bind_group(&self, info: &BindGroupInfo) -> BindGroup {
        let mut entries = Vec::with_capacity(info.entries.len());
        for (i, entry) in info.entries.iter().enumerate() {
            let resource = match *entry {
                Binding::Sampler { sampler } => BindingResource::Sampler(sampler),
                Binding::Texture { texture_view } => BindingResource::TextureView(texture_view),
                Binding::Uniform { buffer } => {
                    BindingResource::Buffer(buffer.as_entire_buffer_binding())
                }
            };

            entries.push(wgpu::BindGroupEntry {
                binding: i as u32,
                resource,
            });
        }

        let bg = self.device.create_bind_group(&BindGroupDescriptor {
            label: info.label,
            layout: info.layout,
            entries: &entries,
        });

        BindGroup { bg: Arc::new(bg) }
    }

    pub fn create_bind_group_layout(&self, info: &BindGroupLayoutInfo) -> BindGroupLayout {
        let mut entries = Vec::with_capacity(info.entries.len());
        for (i, entry) in info.entries.iter().enumerate() {
            let ty = match *entry {
                BindingType::Sampler => {
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                }

                BindingType::Texture { sample_count } => wgpu::BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: sample_count > 1,
                },

                BindingType::Uniform { min_size } => wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(min_size),
                },
            };

            entries.push(wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty,
                count: None,
            });
        }

        let layout = self
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: info.label,
                entries: &entries,
            });

        BindGroupLayout {
            layout: Arc::new(layout),
        }
    }

    pub fn create_buffer(&self, info: &BufferInfo) -> Buffer {
        let mut usage = BufferUsages::COPY_DST;
        usage |= match info.ty {
            BufferType::Index => BufferUsages::INDEX,
            BufferType::Uniform => BufferUsages::UNIFORM,
            BufferType::Vertex => BufferUsages::VERTEX,
        };

        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: info.label,
            size: info.size,
            usage,
            mapped_at_creation: false,
        });

        Buffer {
            buffer: Arc::new(buffer),
        }
    }

    pub fn create_pipeline_layout(&self, info: &PipelineLayoutInfo) -> PipelineLayout {
        let bgls = info
            .bind_group_layouts
            .iter()
            .map(|bgl| &*bgl.layout)
            .collect::<Vec<_>>();

        let pcrs = info
            .push_constant_ranges
            .iter()
            .map(|&range| wgpu::PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: range.clone(),
            })
            .collect::<Vec<_>>();

        let layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: info.label,
                bind_group_layouts: &bgls,
                push_constant_ranges: &pcrs,
            });

        PipelineLayout {
            layout: Arc::new(layout),
        }
    }

    pub fn create_render_pipeline(&self, info: &RenderPipelineInfo) -> RenderPipeline {
        let attribs_len = info.buffers.iter().map(|b| b.formats.len()).sum();
        let mut attributes = Vec::with_capacity(attribs_len);
        let mut offset = 0;
        let mut shader_location = 0;
        for buffer in info.buffers.iter() {
            for format in buffer.formats.iter() {
                attributes.push(wgpu::VertexAttribute {
                    format: format.into(),
                    offset,
                    shader_location,
                });

                offset += format.size();
                shader_location += 1;
            }
        }

        let mut buffers = Vec::with_capacity(info.buffers.len());
        let mut start = 0;
        for buffer in info.buffers.iter() {
            let end = buffer.formats.len();
            buffers.push(wgpu::VertexBufferLayout {
                array_stride: buffer.stride,
                step_mode: buffer.ty.into(),
                attributes: &attributes[start..end],
            });

            start += end;
        }

        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: info.label,
                layout: Some(info.layout),
                vertex: VertexState {
                    module: info.shader,
                    entry_point: info.vs_main,
                    buffers: &buffers,
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    polygon_mode: PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    module: info.shader,
                    entry_point: info.fs_main,
                    targets: &[Some(ColorTargetState {
                        format: info.format.into(),
                        blend: Some(BlendState::ALPHA_BLENDING), // todo: blend states
                        write_mask: ColorWrites::ALL,            // todo: blend color mask
                    })],
                }),
                multiview: None,
            });

        RenderPipeline {
            pipeline: Arc::new(pipeline),
            format: info.format,
        }
    }

    pub fn create_sampler(&self, info: &SamplerInfo) -> Sampler {
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: info.label,
            address_mode_u: info.address_mode_u.into(),
            address_mode_v: info.address_mode_v.into(),
            mag_filter: info.mag_filter.into(),
            min_filter: info.min_filter.into(),
            ..Default::default()
        });

        Sampler {
            sampler: Arc::new(sampler),
        }
    }

    pub fn create_shader(&self, info: &ShaderInfo) -> Shader {
        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label: info.label,
            source: ShaderSource::Wgsl(Cow::Borrowed(info.src)),
        });

        Shader {
            shader: Arc::new(shader),
        }
    }

    pub fn create_texture(&self, info: &TextureInfo) -> Texture {
        let mut usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
        if info.renderable {
            usage |= TextureUsages::RENDER_ATTACHMENT;
        }

        let texture = self.device.create_texture(&TextureDescriptor {
            label: info.label,
            size: Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: info.sample_count,
            dimension: TextureDimension::D2,
            format: info.format.into(),
            usage,
            view_formats: &[info.format.into()], // todo: handle srgb / non-srgb
        });

        Texture {
            texture: Arc::new(texture),
            sample_count: info.sample_count,
            format: info.format,
            is_renderable: info.renderable,
            label: info.label.map(|s| s.to_string()),
        }
    }

    pub fn push_draw_command(&mut self, draw: DrawCommand) {
        self.command_buffer.push(draw);
    }

    pub fn write_buffer<T: bytemuck::Pod + bytemuck::Zeroable>(&self, buffer: &Buffer, data: &[T]) {
        let Some(size) = NonZeroU64::new(std::mem::size_of::<T>() as u64 * data.len() as u64)
        else {
            eprintln!("attempted to write zero bytes to buffer");
            return;
        };

        if let Some(mut buf) = self.queue.write_buffer_with(buffer, 0, size) {
            buf.copy_from_slice(cast_slice(data));
        }
    }

    pub fn write_texture<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        texture: &Texture,
        data: &[T],
    ) {
        let size = texture.texture.size();

        self.queue.write_texture(
            texture.texture.as_image_copy(),
            cast_slice(data),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size.width * texture.format().bytes_per_pixel()),
                rows_per_image: Some(size.height),
            },
            size,
        );
    }
}

pub struct BindGroupLayoutInfo<'info> {
    pub label: Option<&'info str>,
    pub entries: &'info [BindingType],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutId(wgpu::Id<wgpu::BindGroupLayout>);

#[derive(Clone)]
pub struct BindGroupLayout {
    layout: Arc<wgpu::BindGroupLayout>,
}

impl BindGroupLayout {
    pub fn id(&self) -> BindGroupLayoutId {
        BindGroupLayoutId(self.layout.global_id())
    }
}

impl Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl PartialEq for BindGroupLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub struct BindGroupInfo<'info> {
    pub label: Option<&'info str>,
    pub layout: &'info BindGroupLayout,
    pub entries: &'info [Binding<'info>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupId(wgpu::Id<wgpu::BindGroup>);

#[derive(Debug, Clone)]
pub struct BindGroup {
    bg: Arc<wgpu::BindGroup>,
}

impl BindGroup {
    pub fn id(&self) -> BindGroupId {
        BindGroupId(self.bg.global_id())
    }
}

impl Deref for BindGroup {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bg
    }
}

impl PartialEq for BindGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

#[derive(Debug, Clone)]
pub enum Binding<'a> {
    Sampler { sampler: &'a Sampler },
    Texture { texture_view: &'a TextureView },
    Uniform { buffer: &'a Buffer },
}

#[derive(Debug, Clone)]
pub enum BindingType {
    Sampler,
    Texture { sample_count: u32 },
    Uniform { min_size: u64 },
}

pub struct BufferInfo<'info> {
    pub label: Option<&'info str>,
    pub size: u64,
    pub ty: BufferType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(wgpu::Id<wgpu::Buffer>);

#[derive(Debug, Clone)]
pub struct Buffer {
    buffer: Arc<wgpu::Buffer>,
}

impl Buffer {
    pub fn id(&self) -> BufferId {
        BufferId(self.buffer.global_id())
    }
}

impl Deref for Buffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferType {
    Index,
    Uniform,
    Vertex,
}

pub struct SamplerInfo<'info> {
    pub label: Option<&'info str>,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
}

impl<'info> Default for SamplerInfo<'info> {
    fn default() -> Self {
        Self {
            label: Some("default"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerId(wgpu::Id<wgpu::Sampler>);

#[derive(Debug, Clone)]
pub struct Sampler {
    sampler: Arc<wgpu::Sampler>,
}

impl Sampler {
    pub fn id(&self) -> SamplerId {
        SamplerId(self.sampler.global_id())
    }
}

impl Deref for Sampler {
    type Target = wgpu::Sampler;

    fn deref(&self) -> &Self::Target {
        &self.sampler
    }
}

impl PartialEq for Sampler {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
}

impl From<AddressMode> for wgpu::AddressMode {
    fn from(mode: AddressMode) -> Self {
        match mode {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            AddressMode::Repeat => wgpu::AddressMode::Repeat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Linear,
    Nearest,
}

impl From<FilterMode> for wgpu::FilterMode {
    fn from(mode: FilterMode) -> Self {
        match mode {
            FilterMode::Linear => wgpu::FilterMode::Linear,
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}

#[derive(Clone)]
pub struct DrawTarget {
    color_target: TextureView,
    label: Option<String>,
}

impl DrawTarget {
    pub fn new(color_target: &Texture, label: Option<&str>) -> Self {
        let color_target = color_target.create_view(&TextureViewInfo {
            label: Some("color target"),
        });

        Self {
            color_target,
            label: label.map(|s| s.to_string()),
        }
    }

    pub fn color_target(&self) -> &TextureView {
        &self.color_target
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl From<&WindowTarget> for DrawTarget {
    fn from(window_target: &WindowTarget) -> Self {
        window_target.draw_target.clone()
    }
}

impl TryFrom<&mut WindowSurface> for DrawTarget {
    type Error = AgeError;

    fn try_from(surface: &mut WindowSurface) -> Result<Self, Self::Error> {
        Ok(DrawTarget {
            color_target: surface.acquire()?,
            label: Some("window surface".to_string()),
        })
    }
}

pub(crate) struct WindowTarget {
    color_target: Texture,
    draw_target: DrawTarget,
    sampler: Sampler,
    bgl: BindGroupLayout,
    bg: BindGroup,
    pl: PipelineLayout,
    shader: Shader,
    pipeline: RenderPipeline,
}

impl WindowTarget {
    pub(crate) fn new(width: u32, height: u32, device: &RenderDevice) -> Self {
        let color_target = device.create_texture(&TextureInfo {
            label: Some("window target"),
            width,
            height,
            format: TextureFormat::Rgba8Unorm, // todo: make this rgba unorm or srgb?
            renderable: true,
            sample_count: 1,
        });
        let sampler = device.create_sampler(&SamplerInfo::default());
        let bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("window target"),
            entries: &[
                BindingType::Sampler,
                BindingType::Texture {
                    sample_count: color_target.sample_count(),
                },
            ],
        });
        let shader = device.create_shader(&ShaderInfo {
            label: Some("fullscreen"),
            src: include_str!("shaders/fullscreen.wgsl"),
        });
        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("fullscreen"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("fullscreen"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            // This is the format of the window surface, not the draw target.
            format: TextureFormat::Bgra8Unorm,
            buffers: &[],
        });

        let (draw_target, bg) =
            Self::create_configurable_resources(&color_target, &bgl, &sampler, device);

        WindowTarget {
            color_target,
            draw_target,
            sampler,
            bgl,
            bg,
            pl,
            shader,
            pipeline,
        }
    }

    pub(crate) fn reconfigure(&mut self, surface: &WindowSurface, device: &RenderDevice) {
        // Handle surface format change.
        if surface.format() != self.pipeline.format() {
            self.pipeline = device.create_render_pipeline(&RenderPipelineInfo {
                label: Some("fullscreen"),
                layout: &self.pl,
                shader: &self.shader,
                vs_main: "vs_main",
                fs_main: "fs_main",
                format: surface.format(),
                buffers: &[],
            });
        }

        // Handle window size change.
        if surface.size() != self.color_target.size() {
            let (width, height) = surface.size();
            let format = self.color_target.format();

            self.color_target = device.create_texture(&TextureInfo {
                label: Some("window target"),
                width,
                height,
                format,
                renderable: true,
                sample_count: 1,
            });

            let (draw_target, bg) = Self::create_configurable_resources(
                &self.color_target,
                &self.bgl,
                &self.sampler,
                device,
            );

            self.draw_target = draw_target;
            self.bg = bg;
        }
    }

    fn create_configurable_resources(
        color_target: &Texture,
        bgl: &BindGroupLayout,
        sampler: &Sampler,
        device: &RenderDevice,
    ) -> (DrawTarget, BindGroup) {
        let draw_target = DrawTarget::new(color_target, Some("window target"));
        let bg = device.create_bind_group(&BindGroupInfo {
            label: Some("window target"),
            layout: bgl,
            entries: &[
                Binding::Sampler { sampler },
                Binding::Texture {
                    texture_view: draw_target.color_target(),
                },
            ],
        });

        (draw_target, bg)
    }

    pub(crate) fn draw(&self, surface: &mut WindowSurface, device: &mut RenderDevice) -> AgeResult {
        let mut bind_groups = [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS];
        bind_groups[0] = Some(self.bg.clone());

        device.push_draw_command(DrawCommand {
            clear_color: Some(Color::RED),
            target: surface.try_into()?,
            bind_groups,
            pipeline: self.pipeline.clone(),
            push_constant_data: None,
            vertex_buffers: [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS],
            vertices: 0..3,
            indexed_draw: None,
        });

        Ok(())
    }
}

pub struct PipelineLayoutInfo<'info> {
    pub label: Option<&'info str>,
    pub bind_group_layouts: &'info [&'info BindGroupLayout],
    pub push_constant_ranges: &'info [&'info Range<u32>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineLayoutId(wgpu::Id<wgpu::PipelineLayout>);

#[derive(Debug, Clone)]
pub struct PipelineLayout {
    layout: Arc<wgpu::PipelineLayout>,
}

impl PipelineLayout {
    pub fn id(&self) -> PipelineLayoutId {
        PipelineLayoutId(self.layout.global_id())
    }
}

impl Deref for PipelineLayout {
    type Target = wgpu::PipelineLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl PartialEq for PipelineLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub struct RenderPipelineInfo<'info> {
    pub label: Option<&'info str>,
    pub layout: &'info PipelineLayout,
    pub shader: &'info Shader,
    pub vs_main: &'info str,
    pub fs_main: &'info str,
    pub format: TextureFormat,
    pub buffers: &'info [VertexBufferLayout],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPipelineId(wgpu::Id<wgpu::RenderPipeline>);

#[derive(Debug, Clone)]
pub struct RenderPipeline {
    pipeline: Arc<wgpu::RenderPipeline>,
    format: TextureFormat,
}

impl RenderPipeline {
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn id(&self) -> RenderPipelineId {
        RenderPipelineId(self.pipeline.global_id())
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl PartialEq for RenderPipeline {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub struct ShaderInfo<'info> {
    pub label: Option<&'info str>,
    pub src: &'info str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderId(wgpu::Id<wgpu::ShaderModule>);

#[derive(Debug, Clone)]
pub struct Shader {
    shader: Arc<wgpu::ShaderModule>,
}

impl Shader {
    pub fn id(&self) -> ShaderId {
        ShaderId(self.shader.global_id())
    }
}

impl Deref for Shader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.shader
    }
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Rgba8Unorm,
    Rgba8UnormSrgb,
}

impl TextureFormat {
    pub fn bytes_per_pixel(&self) -> u32 {
        Into::<wgpu::TextureFormat>::into(*self)
            .block_copy_size(Some(TextureAspect::All))
            .unwrap_or(0)
    }
}

impl From<TextureFormat> for wgpu::TextureFormat {
    fn from(format: TextureFormat) -> Self {
        match format {
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

impl TryFrom<wgpu::TextureFormat> for TextureFormat {
    type Error = AgeError;

    fn try_from(format: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match format {
            wgpu::TextureFormat::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
            wgpu::TextureFormat::Bgra8UnormSrgb => Ok(TextureFormat::Bgra8UnormSrgb),
            wgpu::TextureFormat::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
            wgpu::TextureFormat::Rgba8UnormSrgb => Ok(TextureFormat::Rgba8UnormSrgb),
            _ => Err("unsupported texture format".into()),
        }
    }
}

#[derive(Default)]
pub struct TextureViewInfo<'info> {
    pub label: Option<&'info str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureViewId(wgpu::Id<wgpu::TextureView>);

#[derive(Debug, Clone)]
pub struct TextureView {
    view: Arc<wgpu::TextureView>,
}

impl TextureView {
    pub fn id(&self) -> TextureViewId {
        TextureViewId(self.view.global_id())
    }
}

impl Deref for TextureView {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl PartialEq for TextureView {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub struct TextureInfo<'info> {
    pub label: Option<&'info str>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub renderable: bool,
    pub sample_count: u32,
}

impl<'info> Default for TextureInfo<'info> {
    fn default() -> Self {
        Self {
            label: None,
            width: 1,
            height: 1,
            format: TextureFormat::Bgra8Unorm,
            renderable: false,
            sample_count: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(wgpu::Id<wgpu::Texture>);

#[derive(Clone)]
pub struct Texture {
    texture: Arc<wgpu::Texture>,
    sample_count: u32,
    format: TextureFormat,
    is_renderable: bool,
    label: Option<String>,
}

impl Texture {
    pub fn create_view(&self, info: &TextureViewInfo) -> TextureView {
        let view = self.texture.create_view(&TextureViewDescriptor {
            label: info.label,
            ..Default::default()
        });

        TextureView {
            view: Arc::new(view),
        }
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn id(&self) -> TextureId {
        TextureId(self.texture.global_id())
    }

    pub fn is_render_texture(&self) -> bool {
        self.is_renderable
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn size(&self) -> (u32, u32) {
        let size = self.texture.size();
        (size.width, size.height)
    }
}

impl Deref for Texture {
    type Target = wgpu::Texture;

    fn deref(&self) -> &Self::Target {
        &self.texture
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        // We don't need to include other fields because they are inherent in the texture.
        self.texture.global_id() == other.texture.global_id()
    }
}

pub(crate) struct WindowSurface {
    surface: Option<Surface<'static>>,
    config: Option<SurfaceConfiguration>,
    surface_texture: Option<SurfaceTexture>,
    format: TextureFormat,
    width: u32,
    height: u32,
    vsync: bool,
}

impl WindowSurface {
    pub(crate) fn new() -> Self {
        Self {
            surface: None,
            config: None,
            surface_texture: None,
            format: TextureFormat::Bgra8UnormSrgb,
            width: 0,
            height: 0,
            vsync: true,
        }
    }

    pub(crate) fn acquire(&mut self) -> AgeResult<TextureView> {
        let Some(surface) = self.surface.as_ref() else {
            return Err("window surface is not resumed".into());
        };

        if self.surface_texture.is_none() {
            // todo: handle the errors that can be recovered from.
            let surface_texture = surface.get_current_texture()?;
            self.surface_texture = Some(surface_texture);
        }

        // Unwrap cannot fail because we just ensured there is a surface texture set.
        let view =
            self.surface_texture
                .as_ref()
                .unwrap()
                .texture
                .create_view(&TextureViewDescriptor {
                    label: Some("window surface"),
                    ..Default::default()
                });

        Ok(TextureView {
            view: Arc::new(view),
        })
    }

    pub(crate) fn format(&self) -> TextureFormat {
        self.format
    }

    pub(crate) fn present(&mut self) {
        if let Some(surface_texture) = self.surface_texture.take() {
            surface_texture.present();
        }
    }

    pub(crate) fn reconfigure(
        &mut self,
        device: &RenderDevice,
        width: u32,
        height: u32,
        vsync: bool,
    ) -> AgeResult {
        let Some(surface) = self.surface.as_ref() else {
            return Err("window surface is not resumed".into());
        };

        let mut config = match self.config.take() {
            Some(config) => config,
            None => match surface.get_default_config(&device.adapter, width, height) {
                Some(config) => config,
                None => return Err("window surface configuration is not supported".into()),
            },
        };

        let present_mode = if vsync {
            PresentMode::Fifo
        } else {
            PresentMode::Immediate
        };

        // todo: pick best format and add/remove srgb from views as required.
        config.format = TextureFormat::Bgra8UnormSrgb.into();
        config.present_mode = present_mode;

        surface.configure(&device.device, &config);

        self.format = config.format.try_into()?;
        self.width = config.width;
        self.height = config.height;
        self.vsync = vsync;
        self.config = Some(config);

        Ok(())
    }

    pub(crate) fn resume(&mut self, device: &RenderDevice, window: Arc<Window>) -> AgeResult {
        let surface = device.instance.create_surface(window.clone())?;
        self.surface = Some(surface);

        let (width, height) = window.inner_size().into();
        self.reconfigure(device, width, height, self.vsync)
    }

    pub(crate) fn suspend(&mut self) {
        self.surface = None;
        self.config = None;
        self.surface_texture = None;
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(crate) fn vsync(&self) -> bool {
        self.vsync
    }
}

struct CommandBuffer {
    commands: Vec<DrawCommand>,
}

impl CommandBuffer {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.commands.clear();
    }

    fn push(&mut self, draw: DrawCommand) {
        self.commands.push(draw);
    }
}

pub struct DrawCommand {
    pub clear_color: Option<Color>,
    pub target: DrawTarget,
    pub bind_groups: [Option<BindGroup>; RenderDevice::MAX_BIND_GROUPS],
    pub pipeline: RenderPipeline,
    pub push_constant_data: Option<Vec<u8>>,
    pub vertex_buffers: [Option<Buffer>; RenderDevice::MAX_VERTEX_BUFFERS],
    pub vertices: Range<u32>,
    pub indexed_draw: Option<IndexedDraw>,
}

pub struct IndexedDraw {
    pub buffer: Buffer,
    pub format: IndexFormat,
    pub indices: Range<u32>,
}

impl From<CreateSurfaceError> for AgeError {
    fn from(err: CreateSurfaceError) -> Self {
        AgeError::new("failed to create window surface").with_source(err)
    }
}

impl From<SurfaceError> for AgeError {
    fn from(err: SurfaceError) -> Self {
        AgeError::new("failed to acquire window surface texture").with_source(err)
    }
}

#[derive(Debug, Clone)]
pub struct VertexBufferLayout {
    pub stride: u64,
    pub ty: VertexType,
    pub formats: &'static [VertexFormat],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexType {
    Vertex,
}

impl From<VertexType> for wgpu::VertexStepMode {
    fn from(ty: VertexType) -> Self {
        match ty {
            VertexType::Vertex => wgpu::VertexStepMode::Vertex,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    Float32x2,
}

impl VertexFormat {
    pub fn size(&self) -> u64 {
        Into::<wgpu::VertexFormat>::into(self).size()
    }
}

impl From<&VertexFormat> for wgpu::VertexFormat {
    fn from(format: &VertexFormat) -> Self {
        match *format {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}

impl From<&IndexFormat> for wgpu::IndexFormat {
    fn from(format: &IndexFormat) -> Self {
        match *format {
            IndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
            IndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_u8(r, g, b, 255)
    }

    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        let a = a as f32 / 255.0;

        Self { r, g, b, a }
    }

    pub const fn to_array_f32(self) -> [f32; 4] {
        let r = self.r;
        let g = self.g;
        let b = self.b;
        let a = self.a;

        [r, g, b, a]
    }

    pub fn to_array_u8(self) -> [u8; 4] {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let a = (self.a * 255.0) as u8;

        [r, g, b, a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}

pub fn align_to<T>(value: T, alignment: T) -> T
where
    T: Add<Output = T> + Copy + Default + PartialEq<T> + Rem<Output = T> + Sub<Output = T>,
{
    wgpu::util::align_to(value, alignment)
}
