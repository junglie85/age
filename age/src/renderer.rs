use std::{
    borrow::Cow,
    collections::VecDeque,
    num::NonZeroU64,
    ops::{Add, Deref, Range, Rem, Sub},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use wgpu::PushConstantRange;

use crate::{
    os::{Window, WindowId},
    Color, Error,
};

pub const COPY_BUFFER_ALIGNMENT: usize = wgpu::COPY_BUFFER_ALIGNMENT as usize;

pub fn align_to<T>(value: T, alignment: T) -> T
where
    T: Add<Output = T> + Copy + Default + PartialEq<T> + Rem<Output = T> + Sub<Output = T>,
{
    wgpu::util::align_to(value, alignment)
}

#[derive(Clone)]
pub struct RenderInterface {
    pool: Arc<Mutex<VecDeque<CommandBuffer>>>,
}

impl RenderInterface {
    const INITIAL_POOL_SIZE: usize = 2;

    pub(crate) fn init() -> Self {
        let mut pool = VecDeque::new();
        for _ in 0..Self::INITIAL_POOL_SIZE {
            pool.push_back(CommandBuffer::new());
        }

        Self {
            pool: Arc::new(Mutex::new(pool)),
        }
    }

    pub fn get_command_buffer(&self) -> CommandBuffer {
        match self
            .pool
            .lock()
            .expect("failed to acquire lock on command buffer pool")
            .pop_front()
        {
            Some(buf) => buf,
            None => {
                println!("no buffers in pool, creating new command buffer");
                CommandBuffer::new()
            }
        }
    }

    fn return_command_buffer(&self, mut buf: CommandBuffer) {
        buf.reset();
        self.pool
            .lock()
            .expect("failed to acquire lock on command buffer pool")
            .push_back(buf);
    }
}

#[derive(Clone)]
pub struct RenderDevice {
    inner: Arc<RenderDeviceInner>,
}

struct RenderDeviceInner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    limits: wgpu::Limits,
}

impl RenderDevice {
    pub(crate) fn init() -> Result<Self, Error> {
        let flags = if cfg!(debug_assertions) {
            wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
        } else {
            wgpu::InstanceFlags::empty()
        };

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN, //DX12,
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

        let limits = adapter.limits();
        let required_limits = wgpu::Limits {
            max_push_constant_size: 128,
            ..Default::default()
        };
        let mut in_limits = true;
        required_limits.check_limits_with_fail_fn(&limits, false, |name, wanted, allowed| {
            eprintln!(
                "limit '{}' failed, wanted {} but allowed {}",
                name, wanted, allowed
            );
            in_limits = false;
        });
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
            inner: Arc::new(RenderDeviceInner {
                instance,
                adapter,
                device,
                queue,
                limits,
            }),
        })
    }

    fn get_adapter(&self) -> &wgpu::Adapter {
        &self.inner.adapter
    }

    fn get_device(&self) -> &wgpu::Device {
        &self.inner.device
    }

    fn get_instance(&self) -> &wgpu::Instance {
        &self.inner.instance
    }

    fn get_limits(&self) -> &wgpu::Limits {
        &self.inner.limits
    }

    fn get_queue(&self) -> &wgpu::Queue {
        &self.inner.queue
    }

    pub fn create_bind_group(&self, desc: &BindGroupDesc) -> BindGroup {
        let entries = desc
            .entries
            .iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: match *entry {
                    BindingResource::Buffer(buffer) => {
                        wgpu::BindingResource::Buffer(buffer.buffer.as_entire_buffer_binding())
                    }
                },
            })
            .collect::<Vec<_>>();

        let bg = self
            .get_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label,
                layout: &desc.layout.bgl,
                entries: &entries,
            });

        BindGroup {
            bg: Arc::new(bg),
            label: Arc::new(desc.label.map(|s| s.to_string())),
            layout: desc.layout.clone(),
        }
    }

    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDesc) -> BindGroupLayout {
        let entries = desc
            .entries
            .iter()
            .enumerate()
            .map(|(binding, entry)| wgpu::BindGroupLayoutEntry {
                binding: binding as u32,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: match *entry {
                    BindingType::Storage {
                        read_only,
                        min_size,
                    } => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(min_size as u64),
                    },
                },
                count: None,
            })
            .collect::<Vec<_>>();

        let bgl = self
            .get_device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: desc.label,
                entries: &entries,
            });

        BindGroupLayout { bgl: Arc::new(bgl) }
    }

    pub fn create_buffer(&self, desc: &BufferDesc) -> Buffer {
        let usage = match desc.ty {
            BufferType::Index => wgpu::BufferUsages::INDEX,
            BufferType::Storage => wgpu::BufferUsages::STORAGE,
            BufferType::Vertex => wgpu::BufferUsages::VERTEX,
        };

        let buffer = self.get_device().create_buffer(&wgpu::BufferDescriptor {
            label: desc.label,
            size: desc.size as u64,
            usage: wgpu::BufferUsages::COPY_DST | usage,
            mapped_at_creation: false,
        });

        Buffer {
            buffer: Arc::new(buffer),
            label: Arc::new(desc.label.map(|s| s.to_string())),
            ty: desc.ty,
        }
    }

    pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDesc) -> PipelineLayout {
        let bgl = desc
            .bind_group_layouts
            .iter()
            .map(|bgl| &*bgl.bgl) // Reference to Deref Arc.
            .collect::<Vec<_>>();

        assert!(desc.push_constants_size <= self.get_limits().max_push_constant_size as usize);
        let push_constant_range = if desc.push_constants_size > 0 {
            vec![PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                range: 0..desc.push_constants_size as u32,
            }]
        } else {
            vec![]
        };

        let layout = self
            .get_device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: desc.label,
                bind_group_layouts: &bgl,
                push_constant_ranges: &push_constant_range,
            });

        PipelineLayout { layout }
    }

    pub fn create_render_pipelne(&self, desc: &RenderPipelineDesc) -> RenderPipeline {
        let buffers = desc
            .buffers
            .iter()
            .map(|l| wgpu::VertexBufferLayout {
                array_stride: l.array_stride,
                step_mode: l.step_mode,
                attributes: &l.attributes,
            })
            .collect::<Vec<_>>();

        let pipeline = self
            .get_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(desc.layout),
                vertex: wgpu::VertexState {
                    module: desc.shader,
                    entry_point: desc.vs_main,
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // could create a pipeline per combination of "features" ad formats?
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
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
                    module: desc.shader,
                    entry_point: desc.fs_main,
                    targets: &[Some(wgpu::ColorTargetState {
                        format: desc.format.into(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING), // todo: own blend states
                        write_mask: wgpu::ColorWrites::ALL,
                    })], // todo: more than one target in the pipeline, update targets slice.
                }),
                multiview: None,
            });

        RenderPipeline {
            pipeline: Arc::new(pipeline),
        }
    }

    pub fn create_shader(&self, desc: &ShaderDesc) -> Shader {
        let shader = self
            .get_device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: desc.label,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(desc.src)),
            });

        Shader { shader }
    }

    pub fn write_buffer<T: Copy>(&self, buffer: &Buffer, data: &[T]) {
        let bytes = cast_slice(data);
        let Some(size) = NonZeroU64::new(bytes.len() as u64) else {
            eprintln!("attempted to write 0 bytes to buffer");
            return;
        };

        if let Some(mut w) = self.get_queue().write_buffer_with(&buffer.buffer, 0, size) {
            w.copy_from_slice(bytes);
        } else {
            eprintln!("attempted to write more bytes to buffer than there is capacity");
        }
    }
}

fn cast_slice<T: Copy>(s: &[T]) -> &[u8] {
    let len = std::mem::size_of_val(s);
    let data = s.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(data, len) }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(err: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a surface for the window").with_source(err)
    }
}

impl From<wgpu::SurfaceError> for Error {
    fn from(err: wgpu::SurfaceError) -> Self {
        Error::new("failed to acquire a surface").with_source(err)
    }
}

pub struct WindowSurface {
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    surface_texture: Option<SurfaceTexture>,
    surface_texture_view: Option<wgpu::TextureView>,
}

impl WindowSurface {
    pub(crate) fn new() -> Self {
        Self {
            surface: None,
            config: None,
            surface_texture: None,
            surface_texture_view: None,
        }
    }
}

pub(crate) struct SurfaceTexture(wgpu::SurfaceTexture);

impl SurfaceTexture {
    pub(crate) fn present(self) {
        self.0.present();
    }
}

impl From<wgpu::SurfaceTexture> for SurfaceTexture {
    fn from(texture: wgpu::SurfaceTexture) -> Self {
        Self(texture)
    }
}

#[derive(Default)]
pub struct CommandBuffer {
    render_pipeline: Option<RenderPipeline>,
    bind_groups: [Option<BindGroup>; Self::MAX_BIND_GROUPS],
    vertex_buffers: [Option<Buffer>; Self::MAX_VERTEX_BUFFERS],
    index_buffer: Option<Buffer>,
    index_format: Option<IndexFormat>,
    passes: Vec<RenderPass>,
    draws: Vec<DrawCommand>,
}

impl CommandBuffer {
    const NONE_BIND_GROUP: Option<BindGroup> = None;
    const NONE_VERTEX_BUFFER: Option<Buffer> = None;
    const MAX_BIND_GROUPS: usize = 2;
    const MAX_VERTEX_BUFFERS: usize = 2;

    fn new() -> Self {
        Self {
            render_pipeline: None,
            bind_groups: [Self::NONE_BIND_GROUP; Self::MAX_BIND_GROUPS],
            vertex_buffers: [Self::NONE_VERTEX_BUFFER; Self::MAX_VERTEX_BUFFERS],
            index_buffer: None,
            index_format: None,
            passes: Vec::new(),
            draws: Vec::new(),
        }
    }

    pub fn begin_render_pass<T: Into<RenderTarget>>(
        &mut self,
        target: T,
        clear_color: Option<Color>,
    ) {
        self.passes.push(RenderPass {
            target: target.into(),
            clear_colors: [clear_color],
            draws: self.passes.len()..self.passes.len(),
        });
    }

    pub fn draw_indexed(&mut self, indices: Range<usize>, instances: Range<usize>) {
        assert!(!self.passes.is_empty(), "no render passes are bound");
        assert!(self.index_buffer.is_some(), "no index buffer is bound");
        assert!(
            self.render_pipeline.is_some(),
            "no render pipeline is bound"
        );

        let pass = self.passes.len() - 1;
        self.passes[pass].draws.end += 1;

        self.draws.push(DrawCommand {
            pipeline: self.render_pipeline.as_ref().unwrap().clone(),
            indices: indices.start as u32..indices.end as u32,
            instances: instances.start as u32..instances.end as u32,
            bind_groups: self.bind_groups.clone(),
            vertex_buffers: self.vertex_buffers.clone(),
            index_buffer: self.index_buffer.clone().unwrap(),
            index_format: self.index_format.unwrap(),
        });
    }

    fn reset(&mut self) {
        self.render_pipeline = None;
        self.bind_groups = [Self::NONE_BIND_GROUP; Self::MAX_BIND_GROUPS];
        self.vertex_buffers = [Self::NONE_VERTEX_BUFFER; Self::MAX_VERTEX_BUFFERS];
        self.index_buffer = None;
        self.index_format = None;
        self.passes.clear();
        self.draws.clear();
    }

    pub fn set_bind_group(&mut self, index: usize, bind_group: &BindGroup) {
        assert!(index < Self::MAX_BIND_GROUPS);
        self.bind_groups[index] = Some(bind_group.clone());
    }

    pub fn set_index_buffer(&mut self, buffer: &Buffer, index_format: IndexFormat) {
        self.index_buffer = Some(buffer.clone());
        self.index_format = Some(index_format);
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.render_pipeline = Some(pipeline.clone());
    }

    pub fn set_vertex_buffer(&mut self, slot: usize, buffer: &Buffer) {
        assert!(slot < Self::MAX_VERTEX_BUFFERS);
        self.vertex_buffers[slot] = Some(buffer.clone());
    }
}

struct RenderPass {
    target: RenderTarget,
    clear_colors: [Option<Color>; 1],
    draws: Range<usize>,
}

struct DrawCommand {
    pipeline: RenderPipeline,
    indices: Range<u32>,
    instances: Range<u32>,
    bind_groups: [Option<BindGroup>; CommandBuffer::MAX_BIND_GROUPS],
    vertex_buffers: [Option<Buffer>; CommandBuffer::MAX_VERTEX_BUFFERS],
    index_buffer: Buffer,
    index_format: IndexFormat,
}

// todo: draw target can have multiple color attachments. we want to be able to convert the following into a target:
// - window surface
// - render texture
// - framebuffer / gbuffer (multiple render_textures), eventually - might take some rework elsewhere.
pub struct RenderTarget {
    color_targets: [ColorTarget; Self::MAX_COLOR_TARGETS],
}

impl RenderTarget {
    const MAX_COLOR_TARGETS: usize = 1;
}

enum ColorTarget {
    WindowSurface(WindowId),
}

impl From<&Window> for RenderTarget {
    fn from(window: &Window) -> Self {
        RenderTarget {
            color_targets: [ColorTarget::WindowSurface(window.get_id())],
        }
    }
}

pub struct BindGroupDesc<'desc> {
    pub label: Option<&'desc str>,
    pub layout: &'desc BindGroupLayout,
    pub entries: &'desc [BindingResource<'desc>],
}

#[derive(Clone)]
pub struct BindGroup {
    bg: Arc<wgpu::BindGroup>,
    label: Arc<Option<String>>,
    layout: BindGroupLayout,
}

impl BindGroup {
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

#[derive(Clone, Copy)]
pub enum BindingResource<'a> {
    Buffer(&'a Buffer),
}

pub struct BindGroupLayoutDesc<'desc> {
    pub label: Option<&'desc str>,
    pub entries: &'desc [BindingType],
}

#[derive(Clone)]
pub struct BindGroupLayout {
    bgl: Arc<wgpu::BindGroupLayout>,
}

#[derive(Debug, Clone, Copy)]
pub enum BindingType {
    Storage { read_only: bool, min_size: usize },
}

pub struct BufferDesc<'desc> {
    pub label: Option<&'desc str>,
    pub size: usize,
    pub ty: BufferType,
}

#[derive(Clone)]
pub struct Buffer {
    buffer: Arc<wgpu::Buffer>,
    label: Arc<Option<String>>,
    ty: BufferType,
}

impl Buffer {
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn size(&self) -> usize {
        self.buffer.size() as usize
    }

    pub fn ty(&self) -> BufferType {
        self.ty
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferType {
    Index,
    Storage,
    Vertex,
}

pub struct PipelineLayoutDesc<'desc> {
    pub label: Option<&'desc str>,
    pub bind_group_layouts: &'desc [&'desc BindGroupLayout],
    pub push_constants_size: usize,
}

pub struct PipelineLayout {
    layout: wgpu::PipelineLayout,
}

impl Deref for PipelineLayout {
    type Target = wgpu::PipelineLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

pub struct RenderPipelineDesc<'desc> {
    pub label: Option<&'desc str>,
    pub layout: &'desc PipelineLayout,
    pub shader: &'desc Shader,
    pub vs_main: &'desc str,
    pub fs_main: &'desc str,
    pub format: TextureFormat,
    pub buffers: &'desc [VertexBufferLayout],
}

#[derive(Clone)]
pub struct RenderPipeline {
    pipeline: Arc<wgpu::RenderPipeline>,
}

impl Deref for RenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

pub struct ShaderDesc<'desc> {
    pub label: Option<&'desc str>,
    pub src: &'desc str,
}

pub struct Shader {
    shader: wgpu::ShaderModule,
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
    type Error = Error;

    fn try_from(format: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match format {
            wgpu::TextureFormat::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
            wgpu::TextureFormat::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
            _ => Err(Error::new("unsupported texture format")),
        }
    }
}

pub struct VertexBufferLayoutDesc<'desc> {
    pub stride: usize,
    pub ty: VertexType,
    pub attribute_offset: usize,
    pub attributes: &'desc [VertexFormat],
}

#[derive(Debug, Clone)]
pub struct VertexBufferLayout {
    attributes: Vec<wgpu::VertexAttribute>,
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
}

impl VertexBufferLayout {
    pub fn new(desc: &VertexBufferLayoutDesc) -> Self {
        let mut offset = 0;
        let mut attributes = Vec::with_capacity(desc.attributes.len());
        for (i, format) in desc.attributes.iter().enumerate() {
            attributes.push(wgpu::VertexAttribute {
                format: (*format).into(),
                offset,
                shader_location: (desc.attribute_offset + i) as u32,
            });
            offset += format.size() as u64
        }

        VertexBufferLayout {
            attributes,
            array_stride: desc.stride as u64,
            step_mode: desc.ty.into(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.attributes.len()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VertexFormat {
    Float32x2,
    Uint32,
}

impl VertexFormat {
    pub fn size(&self) -> usize {
        Into::<wgpu::VertexFormat>::into(*self).size() as usize
    }
}

impl From<VertexFormat> for wgpu::VertexFormat {
    fn from(format: VertexFormat) -> Self {
        match format {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Uint32 => wgpu::VertexFormat::Uint32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VertexType {
    Instance,
    Vertex,
}

impl From<VertexType> for wgpu::VertexStepMode {
    fn from(ty: VertexType) -> Self {
        match ty {
            VertexType::Instance => wgpu::VertexStepMode::Instance,
            VertexType::Vertex => wgpu::VertexStepMode::Vertex,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}

impl IndexFormat {
    pub fn size(&self) -> usize {
        match self {
            IndexFormat::Uint16 => std::mem::size_of::<u16>(),
            IndexFormat::Uint32 => std::mem::size_of::<u32>(),
        }
    }
}

impl From<IndexFormat> for wgpu::IndexFormat {
    fn from(format: IndexFormat) -> Self {
        match format {
            IndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
            IndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
        }
    }
}

enum RenderMessage {
    Enqueue(CommandBuffer),
    Flush,
}

#[derive(Clone)]
pub struct RenderProxy {
    tx: Sender<RenderMessage>,
    ready_semaphore: Arc<AtomicBool>,
    stop_semaphore: Arc<AtomicBool>,
}

impl RenderProxy {
    pub fn enqueue(&self, buf: CommandBuffer) {
        self.tx
            .send(RenderMessage::Enqueue(buf))
            .expect("unable to send enqueue message to render thread");
    }

    pub fn flush(&self) {
        self.tx
            .send(RenderMessage::Flush)
            .expect("unable to send flush message to render thread");
    }

    pub(crate) fn shutdown(&self, thread: JoinHandle<()>) {
        self.stop_semaphore.store(true, Ordering::Relaxed);
        thread.join().expect("unable to join render thread");
    }

    pub(crate) fn wait_sync(&self) {
        // wait until render thread sets the `ready_semaphore` to `true`.
        while Err(false)
            == self.ready_semaphore.compare_exchange(
                true,
                false,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
        {
            continue;
        }
    }
}

pub(crate) fn start_render_thread(
    window: Window,
    device: RenderDevice,
    interface: RenderInterface,
) -> Result<(JoinHandle<()>, RenderProxy), Error> {
    let (tx, rx) = std::sync::mpsc::channel();

    let proxy = RenderProxy {
        tx,
        ready_semaphore: Arc::new(AtomicBool::new(false)),
        stop_semaphore: Arc::new(AtomicBool::new(false)),
    };

    let ready_semaphore = proxy.ready_semaphore.clone();
    let stop_semaphore = proxy.stop_semaphore.clone();
    let thread = std::thread::Builder::new()
        .name("render thread".to_string())
        .spawn(|| {
            if let Err(err) = render_thread_main(
                window,
                interface,
                device,
                rx,
                ready_semaphore,
                stop_semaphore,
            ) {
                eprintln!("{err}");
            }
        })?;

    Ok((thread, proxy))
}

fn render_thread_main(
    window: Window,
    interface: RenderInterface,
    device: RenderDevice,
    rx: Receiver<RenderMessage>,
    ready_semaphore: Arc<AtomicBool>,
    stop_semaphore: Arc<AtomicBool>,
) -> Result<(), Error> {
    let mut window_surface = WindowSurface::new();
    let mut submitted_command_buffer: Option<CommandBuffer> = None;

    ready_semaphore.store(true, Ordering::Relaxed);

    while !stop_semaphore.load(Ordering::Relaxed) {
        for message in rx.try_iter() {
            match message {
                RenderMessage::Enqueue(buf) => handle_enqueue(buf, &mut submitted_command_buffer),
                RenderMessage::Flush => handle_flush(
                    &interface,
                    &device,
                    &window,
                    &mut window_surface,
                    &mut submitted_command_buffer,
                    &ready_semaphore,
                )?,
            }
        }
    }

    Ok(())
}

fn handle_enqueue(buf: CommandBuffer, submitted_command_buffer: &mut Option<CommandBuffer>) {
    if submitted_command_buffer.is_some() {
        panic!("have not processed previous command buffer. there is either a problem with thread synchronisation or support for multiple buffers per frame needs to be implemented");
    }

    *submitted_command_buffer = Some(buf);
}

fn handle_flush(
    interface: &RenderInterface,
    device: &RenderDevice,
    window: &Window,
    window_surface: &mut WindowSurface,
    submitted_command_buffer: &mut Option<CommandBuffer>,
    ready_semaphore: &Arc<AtomicBool>,
) -> Result<(), Error> {
    // If nothing is submitted, we might as well return early.
    let Some(buf) = submitted_command_buffer.take() else {
        return Ok(());
    };

    let new_surface = window_surface.surface.is_none();
    if new_surface {
        let (width, height) = window.get_size();
        let surface = device.get_instance().create_surface(window.clone())?;
        let config = match surface.get_default_config(device.get_adapter(), width, height) {
            Some(config) => config,
            None => return Err(Error::new("window surface is not supported")),
        };

        window_surface.surface = Some(surface);
        window_surface.config = Some(config);
    }

    let config = window_surface.config.as_mut().unwrap();
    let (width, height) = window.get_size();
    let surface_resized = config.width != width || config.height != height;

    let window_state_changed = window.has_state_changed();

    if new_surface || surface_resized || window_state_changed {
        let state = window.get_state();

        // If width and height are 0, we get an error. Could just not render because it probably means the
        // window is minimised.
        config.width = if width > 0 { width } else { 1 };
        config.height = if height > 0 { height } else { 1 };
        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo.
        config.present_mode = match state.vsync {
            true => wgpu::PresentMode::Fifo,
            false => wgpu::PresentMode::Immediate,
        };

        let surface = window_surface.surface.as_ref().unwrap();
        surface.configure(device.get_device(), config);
    }

    // create surface texture and surface view.
    let config = window_surface.config.as_ref().unwrap();
    let surface_texture = window_surface
        .surface
        .as_ref()
        .unwrap()
        .get_current_texture()?;
    let view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            label: window.get_name(),
            format: Some(config.format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

    window_surface.surface_texture = Some(surface_texture.into());
    window_surface.surface_texture_view = Some(view);

    // Do rendering.
    let mut encoder = device
        .get_device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("flush"),
        });

    for pass in buf.passes.iter() {
        let mut color_attachments = Vec::with_capacity(pass.target.color_targets.len());
        for (i, color_target) in pass.target.color_targets.iter().enumerate() {
            let view = match color_target {
                ColorTarget::WindowSurface(_window_id) => {
                    window_surface.surface_texture_view.as_ref().unwrap()
                }
            };

            let attachment = Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match pass.clear_colors[i] {
                        Some(c) => wgpu::LoadOp::Clear(c.into()),
                        None => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            });

            color_attachments.push(attachment);
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for DrawCommand {
            pipeline,
            indices,
            instances,
            bind_groups,
            vertex_buffers,
            index_buffer,
            index_format,
        } in &buf.draws[pass.draws.clone()]
        {
            // todo: cache the set values and only reset if they change.
            for (index, bg) in bind_groups.iter().enumerate() {
                if let Some(bg) = bg {
                    rpass.set_bind_group(index as u32, &bg.bg, &[]);
                }
            }
            for (slot, buffer) in vertex_buffers.iter().enumerate() {
                if let Some(buffer) = buffer {
                    rpass.set_vertex_buffer(slot as u32, buffer.buffer.slice(..));
                }
            }
            rpass.set_index_buffer(index_buffer.buffer.slice(..), (*index_format).into());
            rpass.set_pipeline(pipeline);
            rpass.draw_indexed(indices.clone(), 0, instances.clone())
        }
    }

    device.get_queue().submit([encoder.finish()]);
    interface.return_command_buffer(buf);

    if let Some(surface_texture) = window_surface.surface_texture.take() {
        window.present(surface_texture);
    }

    // Signal the main thread that rendering is complete.
    ready_semaphore.store(true, Ordering::Relaxed);

    Ok(())
}
