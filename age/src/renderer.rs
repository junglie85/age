use std::{borrow::Cow, collections::VecDeque, ops::Index, sync::Arc};

use crate::{sys::Window, Color, Error};

#[derive(Default)]
pub(crate) struct Surface<'window> {
    s: Option<wgpu::Surface<'window>>,
    config: Option<wgpu::SurfaceConfiguration>,
    frame: Option<wgpu::SurfaceTexture>,
}

impl<'window> Surface<'window> {
    pub(crate) fn acquire(&mut self) -> wgpu::TextureView {
        assert!(self.s.is_some(), "surface has not been initialised");

        let frame = match self.s.as_ref().unwrap().get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                // todo: try to recreate
                panic!("failed to obtain next surface texture");
            }
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("surface"),
            format: Some(wgpu::TextureFormat::Bgra8Unorm), // todo: handle srgb, other formats
            ..Default::default()
        });

        self.frame = Some(frame);

        view
    }

    pub(crate) fn init(
        &mut self,
        renderer: &Renderer,
        window: &'window Window,
    ) -> Result<(), Error> {
        let (width, height) = (window.width(), window.height());
        let s = renderer.instance.create_surface(window)?;
        let mut config = match s.get_default_config(&renderer.adapter, width, height) {
            Some(config) => config,
            None => return Err("window surface is not supported by the graphics adapter".into()),
        };

        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo: deal with srgb.

        s.configure(&renderer.device, &config);

        self.s = Some(s);
        self.config = Some(config);

        Ok(())
    }

    pub(crate) fn present(&mut self) {
        if let Some(frame) = self.frame.take() {
            frame.present();
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BindGroupId(GenIdx);

impl BindGroupId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for BindGroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BindGroupId").field(&self.0 .0).finish()
    }
}

pub struct BindGroupDesc<'desc> {
    label: Option<&'desc str>,
    layout: BindGroupLayoutId,
    resources: &'desc [BindingResource],
}

pub enum BindingResource {
    Sampler(SamplerId),
    TextureView(TextureViewId),
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutId(GenIdx);

impl BindGroupLayoutId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for BindGroupLayoutId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BindGroupLayoutId")
            .field(&self.0 .0)
            .finish()
    }
}

pub struct BindGroupLayoutDesc<'desc> {
    pub label: Option<&'desc str>,
    pub entries: &'desc [BindingType],
}

pub enum BindingType {
    Sampler,
    Texture { multisampled: bool },
}

impl From<&BindingType> for wgpu::BindingType {
    fn from(ty: &BindingType) -> Self {
        match *ty {
            BindingType::Sampler => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            BindingType::Texture { multisampled } => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PipelineLayoutId(GenIdx);

impl PipelineLayoutId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for PipelineLayoutId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PipelineLayoutId").field(&self.0 .0).finish()
    }
}

pub struct PipelineLayoutDesc<'desc> {
    pub label: Option<&'desc str>,
    pub bind_group_layouts: &'desc [BindGroupLayoutId],
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RenderPipelineId(GenIdx);

impl RenderPipelineId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for RenderPipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderPipelineId").field(&self.0 .0).finish()
    }
}

pub struct RenderPipelineDesc<'desc> {
    label: Option<&'desc str>,
    layout: PipelineLayoutId,
    shader: ShaderId,
    vs_main: &'desc str,
    fs_main: &'desc str,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerId(GenIdx);

impl SamplerId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for SamplerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SamplerId").field(&self.0 .0).finish()
    }
}

pub struct SamplerDesc<'desc> {
    label: Option<&'desc str>,
    address_mode_u: AddressMode,
    address_mode_v: AddressMode,
    mag_filter: FilterMode,
    min_filter: FilterMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
}

impl From<AddressMode> for wgpu::AddressMode {
    fn from(value: AddressMode) -> Self {
        match value {
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
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Linear => wgpu::FilterMode::Linear,
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderId(GenIdx);

impl ShaderId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for ShaderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShaderId").field(&self.0 .0).finish()
    }
}

pub struct ShaderDesc<'desc> {
    pub label: Option<&'desc str>,
    pub source: &'desc str,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(GenIdx);

impl TextureId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for TextureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextureId").field(&self.0 .0).finish()
    }
}

pub struct TextureDesc<'desc> {
    label: Option<&'desc str>,
    width: u32,
    height: u32,
    format: TextureFormat,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureViewId(GenIdx);

impl TextureViewId {
    pub const INVALID: Self = Self(GenIdx::INVALID);
}

impl std::fmt::Debug for TextureViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextureViewId").field(&self.0 .0).finish()
    }
}

pub struct TextureViewDesc<'desc> {
    label: Option<&'desc str>,
    texture: TextureId,
    format: TextureFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Bgra8Unorm,
    Rgba8Unorm,
}

impl From<TextureFormat> for wgpu::TextureFormat {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
}

impl TryFrom<wgpu::TextureFormat> for TextureFormat {
    type Error = Error;

    fn try_from(value: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match value {
            wgpu::TextureFormat::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
            wgpu::TextureFormat::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
            _ => Err(Error::new(format!(
                "texture format {:?} is not supported",
                value
            ))),
        }
    }
}

pub(crate) struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    belt: wgpu::util::StagingBelt,

    backbuffer_bgl: BindGroupLayoutId,
    #[allow(dead_code)]
    backbuffer_pl: PipelineLayoutId,
    #[allow(dead_code)]
    backbuffer_shader: ShaderId,
    backbuffer_pipeline: RenderPipelineId,

    bgs: GenVec<wgpu::BindGroup>,
    bgls: GenVec<wgpu::BindGroupLayout>,
    pls: GenVec<wgpu::PipelineLayout>,
    render_pipelines: GenVec<wgpu::RenderPipeline>,
    samplers: GenVec<wgpu::Sampler>,
    shaders: GenVec<wgpu::ShaderModule>,
    textures: GenVec<wgpu::Texture>,
    texture_views: GenVec<wgpu::TextureView>,
}

impl Renderer {
    pub(crate) fn new() -> Result<Self, Error> {
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

        let required_features = wgpu::Features::empty();
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

        let belt = wgpu::util::StagingBelt::new(1024);

        let mut renderer = Self {
            instance,
            adapter,
            device,
            queue,
            belt,

            backbuffer_bgl: BindGroupLayoutId::INVALID,
            backbuffer_pl: PipelineLayoutId::INVALID,
            backbuffer_shader: ShaderId::INVALID,
            backbuffer_pipeline: RenderPipelineId::INVALID,

            bgs: GenVec::default(),
            bgls: GenVec::default(),
            pls: GenVec::default(),
            render_pipelines: GenVec::default(),
            samplers: GenVec::default(),
            shaders: GenVec::default(),
            textures: GenVec::default(),
            texture_views: GenVec::default(),
        };

        renderer.backbuffer_bgl = renderer.create_bind_group_layout(&BindGroupLayoutDesc {
            label: Some("backbuffer"),
            entries: &[
                BindingType::Sampler,
                BindingType::Texture {
                    multisampled: false,
                },
            ],
        });

        renderer.backbuffer_pl = renderer.create_pipeline_layout(&PipelineLayoutDesc {
            label: Some("backbuffer"),
            bind_group_layouts: &[renderer.backbuffer_bgl],
        });

        renderer.backbuffer_shader = renderer.create_shader(ShaderDesc {
            label: Some("backbuffer"),
            source: include_str!("backbuffer.wgsl"),
        });

        renderer.backbuffer_pipeline = renderer.create_render_pipeline(&RenderPipelineDesc {
            label: Some("backbuffer"),
            layout: renderer.backbuffer_pl,
            shader: renderer.backbuffer_shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
        });

        Ok(renderer)
    }

    pub(crate) fn create_backbuffer(&mut self) -> Backbuffer {
        Backbuffer::new(self, self.backbuffer_pipeline, self.backbuffer_bgl)
    }

    pub fn create_bind_group(&mut self, desc: &BindGroupDesc) -> BindGroupId {
        let layout = &self.bgls[desc.layout.0];
        let entries = desc
            .resources
            .iter()
            .enumerate()
            .map(|(binding, resource)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: match resource {
                    BindingResource::Sampler(id) => {
                        wgpu::BindingResource::Sampler(&self.samplers[id.0])
                    }
                    BindingResource::TextureView(id) => {
                        wgpu::BindingResource::TextureView(&self.texture_views[id.0])
                    }
                },
            })
            .collect::<Vec<_>>();

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: desc.label,
            layout: &layout,
            entries: &entries,
        });

        BindGroupId(self.bgs.add(bg))
    }

    pub fn create_bind_group_layout(&mut self, desc: &BindGroupLayoutDesc) -> BindGroupLayoutId {
        let entries = desc
            .entries
            .iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupLayoutEntry {
                binding: binding as u32,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: entry.into(),
                count: None,
            })
            .collect::<Vec<_>>();

        let bgl = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: desc.label,
                entries: &entries,
            });

        BindGroupLayoutId(self.bgls.add(bgl))
    }

    pub fn create_pipeline_layout(&mut self, desc: &PipelineLayoutDesc) -> PipelineLayoutId {
        let bgls = desc
            .bind_group_layouts
            .iter()
            .map(|bgl| &self.bgls[bgl.0])
            .collect::<Vec<_>>();

        let pl = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: desc.label,
                bind_group_layouts: &bgls,
                push_constant_ranges: &[],
            });

        PipelineLayoutId(self.pls.add(pl))
    }

    pub fn create_render_pipeline(&mut self, desc: &RenderPipelineDesc) -> RenderPipelineId {
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(&self.pls[desc.layout.0]),
                vertex: wgpu::VertexState {
                    module: &self.shaders[desc.shader.0],
                    entry_point: desc.vs_main,
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shaders[desc.shader.0],
                    entry_point: desc.fs_main,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm, // todo: get this from the surface
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            });

        RenderPipelineId(self.render_pipelines.add(pipeline))
    }

    pub fn create_sampler(&mut self, desc: &SamplerDesc) -> SamplerId {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label,
            address_mode_u: desc.address_mode_u.into(),
            address_mode_v: desc.address_mode_v.into(),
            mag_filter: desc.mag_filter.into(),
            min_filter: desc.min_filter.into(),
            ..Default::default()
        });

        SamplerId(self.samplers.add(sampler))
    }

    pub fn create_shader(&mut self, desc: ShaderDesc) -> ShaderId {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: desc.label,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(desc.source)),
            });

        ShaderId(self.shaders.add(shader))
    }

    pub fn create_texture(&mut self, desc: &TextureDesc) -> TextureId {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: desc.label,
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format.into(), // todo: can we use srgb?
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[], // todo: srgb?
        });

        TextureId(self.textures.add(texture))
    }

    pub fn create_texture_view(&mut self, desc: &TextureViewDesc) -> TextureViewId {
        let texture = &self.textures[desc.texture.0];
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: desc.label,
            format: Some(desc.format.into()),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        TextureViewId(self.texture_views.add(view))
    }

    pub(crate) fn submit(
        &mut self,
        buf: CommandBuffer,
        backbuffer: &Backbuffer,
        surface: &mut Surface,
    ) {
        // This could all be done on a background thread.

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("submit"),
            });

        // todo: align indices (u16).
        // todo: resize vbo and ibo
        // todo: write vertices to global vbo
        // todo: write indices to global ibo

        let mut draw_offset = 0;
        for pass in buf.passes.iter() {
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_views[pass.target.0],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match pass.clear_color {
                            Some(color) => wgpu::LoadOp::Clear(color.into()),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for _draw in &buf.draws[draw_offset..pass.draw_count] {}
            draw_offset += pass.draw_count;
        }

        let view = surface.acquire();
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::BLUE.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.render_pipelines[self.backbuffer_pipeline.0]);
            rpass.set_bind_group(0, &self.bgs[backbuffer.bg.0], &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a window surface").with_source(value)
    }
}

pub struct DrawTarget {
    texture_view: TextureViewId,
}

impl DrawTarget {
    pub(crate) fn texture_view(&self) -> TextureViewId {
        self.texture_view
    }
}

pub(crate) struct Backbuffer {
    #[allow(dead_code)]
    pipeline: RenderPipelineId,
    #[allow(dead_code)]
    sampler: SamplerId,
    texture: TextureId,
    texture_view: TextureViewId,
    bg: BindGroupId,
}

impl Backbuffer {
    fn new(renderer: &mut Renderer, pipeline: RenderPipelineId, bgl: BindGroupLayoutId) -> Self {
        let sampler = renderer.create_sampler(&SamplerDesc {
            label: Some("backbuffer"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
        });

        let texture = renderer.create_texture(&TextureDesc {
            label: Some("backbuffer"),
            width: 1920, // todo: pass these in
            height: 1080,
            format: TextureFormat::Rgba8Unorm, // todo: can we use srgb?
        });

        let texture_view = renderer.create_texture_view(&TextureViewDesc {
            label: Some("backbuffer"),
            texture,
            format: TextureFormat::Rgba8Unorm,
        });

        let bg = renderer.create_bind_group(&BindGroupDesc {
            label: Some("backbuffer"),
            layout: bgl,
            resources: &[
                BindingResource::Sampler(sampler),
                BindingResource::TextureView(texture_view),
            ],
        });

        Self {
            pipeline,
            sampler,
            texture,
            texture_view,
            bg,
        }
    }
}

impl From<&Backbuffer> for DrawTarget {
    fn from(backbuffer: &Backbuffer) -> Self {
        DrawTarget {
            texture_view: backbuffer.texture_view,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Texture {
    inner: Arc<TextureInner>,
}

struct TextureInner {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

#[derive(Default, Clone)]
pub(crate) struct CommandBuffer {
    next_pass: usize,
    draws: Vec<DrawCommand>,
    passes: Vec<RenderPass>,
    total_indices: usize,
    total_vertices: usize,
}

impl CommandBuffer {
    pub(crate) fn clear(&mut self) {
        self.next_pass = 0;
        self.draws.clear();
        self.passes.clear();
        self.total_indices = 0;
        self.total_vertices = 0;
    }

    pub(crate) fn record(&mut self, draw: DrawCommand) {
        if self.next_pass == 0 {
            panic!("cannot record draw command without a render pass");
        }

        self.total_indices += draw.indices.len();
        self.total_vertices += draw.vertices.len();
        self.draws.push(draw);
        self.passes[self.next_pass - 1].draw_count += 1;
    }

    pub(crate) fn set_render_pass(&mut self, target: TextureViewId, clear_color: Option<Color>) {
        self.next_pass += 1;
        self.passes.push(RenderPass {
            target,
            clear_color,
            draw_count: 0,
        });
    }
}

#[derive(Clone)]
pub(crate) struct RenderPass {
    pub(crate) target: TextureViewId,
    pub(crate) clear_color: Option<Color>,
    pub(crate) draw_count: usize,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct DrawCommand {
    pub(crate) vertices: Vec<GeometryVertex>,
    pub(crate) indices: Vec<u16>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct GeometryVertex {
    pub(crate) pos: [f32; 2],
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct GenIdx(u32);

impl GenIdx {
    const INVALID: Self = Self::new(0xFFFFFF, 0xFF);

    const fn new(index: usize, gen: u8) -> Self {
        Self(((gen as u32) << 24) | index as u32)
    }

    fn split(&self) -> (usize, u8) {
        let index = (self.0 & 0xFFFFFF) as usize;
        let gen = ((self.0 >> 24) & 0xFF) as u8;
        (index, gen)
    }
}

struct Resource<T> {
    gen: u8,
    item: Option<T>,
}

struct GenVec<T> {
    resources: Vec<Resource<T>>,
    free: VecDeque<usize>,
}

impl<T> Default for GenVec<T> {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            free: VecDeque::new(),
        }
    }
}

impl<T> GenVec<T> {
    fn add(&mut self, resource: T) -> GenIdx {
        let index = if let Some(index) = self.free.pop_front() {
            index
        } else {
            self.resources.push(Resource { gen: 0, item: None });
            self.resources.len() - 1
        };

        self.resources[index].item = Some(resource);

        GenIdx::new(index, self.resources[index].gen)
    }

    fn remove(&mut self, idx: GenIdx) -> Option<T> {
        let (index, gen) = idx.split();
        assert_eq!(
            gen, self.resources[index].gen,
            "resource generation does not match"
        );

        // Recycle generation if we get to u8 max.
        if self.resources[index].gen == 255 {
            self.resources[index].gen = 0;
        } else {
            self.resources[index].gen += 1;
        }

        self.resources[index].item.take()
    }
}

impl<T> Index<GenIdx> for GenVec<T> {
    type Output = T;

    fn index(&self, idx: GenIdx) -> &Self::Output {
        let (index, gen) = idx.split();
        assert_eq!(
            gen, self.resources[index].gen,
            "resource generation does not match"
        );

        self.resources[index].item.as_ref().unwrap()
    }
}
