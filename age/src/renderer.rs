use std::{borrow::Cow, ops::Deref, sync::Arc};

use wgpu::{
    BindGroupDescriptor, BindGroupLayoutDescriptor, BindingResource, BlendState, ColorTargetState,
    ColorWrites, CommandEncoderDescriptor, CreateSurfaceError, Extent3d, Face, FragmentState,
    FrontFace, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode,
    PresentMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, Surface, SurfaceError, SurfaceTexture, TextureDescriptor,
    TextureDimension, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension, VertexState,
};
use winit::window::Window;

use crate::{AgeError, AgeResult};

pub struct RenderDevice {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl RenderDevice {
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

        let required_features = wgpu::Features::empty();
        assert!(adapter.features().contains(required_features));

        let required_limits = wgpu::Limits {
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

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub(crate) fn begin_frame(&self) {}

    pub(crate) fn end_frame(
        &mut self,
        surface: &mut WindowSurface,
        window_target: &WindowTarget,
        triangle_pipeline: &RenderPipeline,
    ) -> AgeResult {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("end frame"),
            });

        {
            let view = &window_target.draw_target.color_target;
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("window target"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLUE),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(triangle_pipeline);
            rpass.draw(0..3, 0..1);
        }

        {
            let view = surface.acquire()?;
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("window surface selecta"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::RED),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_bind_group(0, &window_target.bg, &[]);
            rpass.set_pipeline(&window_target.pipeline);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        Ok(())
    }

    pub fn create_bind_group(&self, info: &BindGroupInfo) -> BindGroup {
        let mut entries = Vec::with_capacity(info.entries.len());
        for (i, entry) in info.entries.iter().enumerate() {
            let resource = match *entry {
                Binding::Sampler { sampler } => BindingResource::Sampler(sampler),
                Binding::Texture { texture_view } => BindingResource::TextureView(texture_view),
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

    pub fn create_pipeline_layout(&self, info: &PipelineLayoutInfo) -> PipelineLayout {
        let bgls = info
            .bind_group_layouts
            .iter()
            .map(|bgl| &*bgl.layout)
            .collect::<Vec<_>>();

        let layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: info.label,
                bind_group_layouts: &bgls,
                push_constant_ranges: &[],
            });

        PipelineLayout {
            layout: Arc::new(layout),
        }
    }

    pub fn create_render_pipeline(&self, info: &RenderPipelineInfo) -> RenderPipeline {
        let pipeline = self
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: info.label,
                layout: Some(info.layout),
                vertex: VertexState {
                    module: info.shader,
                    entry_point: info.vs_main,
                    buffers: &[],
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

    pub fn create_render_texture(&self, info: &TextureInfo) -> RenderTexture {
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
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[info.format.into()], // todo: handle srgb / non-srgb
        });

        RenderTexture {
            texture: Arc::new(texture),
            sample_count: info.sample_count,
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
}

pub struct BindGroupLayoutInfo<'info> {
    pub label: Option<&'info str>,
    pub entries: &'info [BindingType],
}

#[derive(Clone)]
pub struct BindGroupLayout {
    layout: Arc<wgpu::BindGroupLayout>,
}

impl Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

pub struct BindGroupInfo<'info> {
    pub label: Option<&'info str>,
    pub layout: &'info BindGroupLayout,
    pub entries: &'info [Binding<'info>],
}

#[derive(Clone)]
pub struct BindGroup {
    bg: Arc<wgpu::BindGroup>,
}

impl Deref for BindGroup {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bg
    }
}

#[derive(Debug, Clone)]
pub enum Binding<'a> {
    Sampler { sampler: &'a Sampler },
    Texture { texture_view: &'a TextureView },
}

#[derive(Debug, Clone)]
pub enum BindingType {
    Sampler,
    Texture { sample_count: u32 },
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

#[derive(Debug, Clone)]
pub struct Sampler {
    sampler: Arc<wgpu::Sampler>,
}

impl Deref for Sampler {
    type Target = wgpu::Sampler;

    fn deref(&self) -> &Self::Target {
        &self.sampler
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

pub struct DrawTarget {
    color_target: TextureView,
}

impl DrawTarget {
    pub fn new(color_target: &RenderTexture) -> Self {
        let view = color_target.texture.create_view(&TextureViewDescriptor {
            label: Some("color target"),
            ..Default::default()
        });

        Self {
            color_target: TextureView {
                view: Arc::new(view),
            },
        }
    }

    pub fn color_target(&self) -> &TextureView {
        &self.color_target
    }
}

pub(crate) struct WindowTarget {
    draw_target: DrawTarget,
    #[allow(dead_code)]
    sampler: Sampler,
    bg: BindGroup,
    pl: PipelineLayout,
    shader: Shader,
    pipeline: RenderPipeline,
}

impl WindowTarget {
    pub(crate) fn new(width: u32, height: u32, device: &RenderDevice) -> Self {
        let color_target = device.create_render_texture(&TextureInfo {
            label: Some("window target"),
            width,
            height,
            // This is the format of the color target, not the window surface.
            format: TextureFormat::Rgba8Unorm, // todo: make this rgba unorm or srgb?
            ..Default::default()
        });
        let draw_target = DrawTarget::new(&color_target);
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
        let bg = device.create_bind_group(&BindGroupInfo {
            label: Some("window target"),
            layout: &bgl,
            entries: &[
                Binding::Sampler { sampler: &sampler },
                Binding::Texture {
                    texture_view: draw_target.color_target(),
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
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("fullscreen"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            // This is the format of the window surface, not the draw target.
            format: TextureFormat::Bgra8Unorm,
        });

        WindowTarget {
            draw_target,
            sampler,
            bg,
            pl,
            shader,
            pipeline,
        }
    }

    pub(crate) fn reconfigure(&mut self, surface: &WindowSurface, device: &RenderDevice) {
        // todo: Handle window resized, so resize draw target.

        // Handle surface format change.
        if surface.format() != self.pipeline.format() {
            self.pipeline = device.create_render_pipeline(&RenderPipelineInfo {
                label: Some("fullscreen"),
                layout: &self.pl,
                shader: &self.shader,
                vs_main: "vs_main",
                fs_main: "fs_main",
                format: surface.format(),
            });
        }
    }
}

pub struct PipelineLayoutInfo<'info> {
    pub label: Option<&'info str>,
    pub bind_group_layouts: &'info [&'info BindGroupLayout],
}

#[derive(Debug, Clone)]
pub struct PipelineLayout {
    layout: Arc<wgpu::PipelineLayout>,
}

impl Deref for PipelineLayout {
    type Target = wgpu::PipelineLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

pub struct RenderPipelineInfo<'info> {
    pub label: Option<&'info str>,
    pub layout: &'info PipelineLayout,
    pub shader: &'info Shader,
    pub vs_main: &'info str,
    pub fs_main: &'info str,
    pub format: TextureFormat,
}

#[derive(Debug, Clone)]
pub struct RenderPipeline {
    pipeline: Arc<wgpu::RenderPipeline>,
    format: TextureFormat,
}

impl RenderPipeline {
    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

pub struct ShaderInfo<'info> {
    pub label: Option<&'info str>,
    pub src: &'info str,
}

#[derive(Debug, Clone)]
pub struct Shader {
    shader: Arc<wgpu::ShaderModule>,
}

impl Deref for Shader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.shader
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Bgra8Unorm,
    Rgba8Unorm,
    // todo: srgb
}

impl From<TextureFormat> for wgpu::TextureFormat {
    fn from(format: TextureFormat) -> Self {
        match format {
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
}

impl TryFrom<wgpu::TextureFormat> for TextureFormat {
    type Error = AgeError;

    fn try_from(format: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match format {
            wgpu::TextureFormat::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
            wgpu::TextureFormat::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
            _ => Err("unsupported texture format".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextureView {
    view: Arc<wgpu::TextureView>,
}

impl Deref for TextureView {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

pub struct TextureInfo<'info> {
    pub label: Option<&'info str>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub sample_count: u32,
}

impl<'info> Default for TextureInfo<'info> {
    fn default() -> Self {
        Self {
            label: None,
            width: 1,
            height: 1,
            format: TextureFormat::Bgra8Unorm,
            sample_count: 1,
        }
    }
}

#[derive(Clone)]
pub struct RenderTexture {
    texture: Arc<wgpu::Texture>,
    sample_count: u32,
    format: TextureFormat,
}

impl RenderTexture {
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }
}

pub(crate) struct WindowSurface {
    surface: Option<Surface<'static>>,
    surface_texture: Option<SurfaceTexture>,
    format: TextureFormat,
    vsync: bool,
}

impl WindowSurface {
    pub(crate) fn new() -> Self {
        Self {
            surface: None,
            surface_texture: None,
            format: TextureFormat::Bgra8Unorm,
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

        let mut config = match surface.get_default_config(&device.adapter, width, height) {
            Some(config) => config,
            None => return Err("window surface configuration is not supported".into()),
        };

        let present_mode = if vsync {
            PresentMode::Fifo
        } else {
            PresentMode::Immediate
        };

        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo - srgb + pick best format.
        config.present_mode = present_mode;

        surface.configure(&device.device, &config);

        self.format = config.format.try_into()?;

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
        self.surface_texture = None;
    }
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
