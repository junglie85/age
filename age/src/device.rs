use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    ops::{Deref, Range},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use crate::{
    sys::{Window, WindowId},
    Color, Error,
};

#[derive(Clone)]
pub struct RenderDevice {
    inner: Arc<RenderDeviceInner>,
    pool: Arc<Mutex<VecDeque<CommandBuffer>>>,
    backbuffers: Arc<Mutex<HashMap<WindowId, Backbuffer>>>,
    command_buffer: Arc<Mutex<Option<CommandBuffer>>>,
}

struct RenderDeviceInner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl RenderDevice {
    const INITIAL_POOL_SIZE: usize = 2;

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

        let mut pool = VecDeque::new();
        for _ in 0..Self::INITIAL_POOL_SIZE {
            pool.push_back(CommandBuffer::new());
        }

        Ok(Self {
            inner: Arc::new(RenderDeviceInner {
                instance,
                adapter,
                device,
                queue,
            }),
            pool: Arc::new(Mutex::new(pool)),
            backbuffers: Arc::new(Mutex::new(HashMap::new())),
            command_buffer: Arc::new(Mutex::new(None)),
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

    fn get_queue(&self) -> &wgpu::Queue {
        &self.inner.queue
    }

    pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDesc) -> PipelineLayout {
        let layout = self
            .get_device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: desc.label,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        PipelineLayout { layout }
    }

    pub fn create_render_pipelne(&self, desc: &RenderPipelineDesc) -> RenderPipeline {
        let pipeline = self
            .get_device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: desc.label,
                layout: Some(desc.layout),
                vertex: wgpu::VertexState {
                    module: desc.shader,
                    entry_point: desc.vs_main,
                    buffers: &[],
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

    pub(crate) fn prepare_window(&self, window: &mut Window) -> Result<(), Error> {
        let mut backbuffers = self
            .backbuffers
            .lock()
            .expect("failed to acquire lock on backbuffers");

        // create backbuffer if it does not yet exist.
        let backbuffer = backbuffers
            .entry(window.get_id())
            .or_insert_with(Backbuffer::new);

        if backbuffer.surface.is_none() {
            let (width, height) = window.get_size();
            let surface = self.get_instance().create_surface(window.get_handle())?;
            let mut config = match surface.get_default_config(self.get_adapter(), width, height) {
                Some(config) => config,
                None => return Err(Error::new("backbuffer surface is not supported")),
            };

            config.format = wgpu::TextureFormat::Bgra8Unorm; // todo.

            surface.configure(self.get_device(), &config);
            backbuffer.surface = Some(surface);
            backbuffer.config = Some(config);
        }

        // todo: Could handle surface resized here too.

        // create surface texture and surface view.
        let config = backbuffer.config.as_ref().unwrap();
        let surface_texture = backbuffer.surface.as_ref().unwrap().get_current_texture()?;
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

        window.set_surface_texture(surface_texture);
        window.set_surface_texture_view(view);

        Ok(())
    }
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

pub struct Backbuffer {
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl Backbuffer {
    pub(crate) fn new() -> Self {
        Self {
            surface: None,
            config: None,
        }
    }
}

#[derive(Default)]
pub struct CommandBuffer {
    render_pipeline: Option<RenderPipeline>,
    passes: Vec<RenderPass>,
    draws: Vec<DrawCommand>,
}

impl CommandBuffer {
    fn new() -> Self {
        Self {
            render_pipeline: None,
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

    pub fn draw(&mut self, vertices: Range<usize>, instances: Range<usize>) {
        assert!(!self.passes.is_empty(), "no render passes are bound");
        assert!(
            self.render_pipeline.is_some(),
            "no render pipeline is bound"
        );

        let pass = self.passes.len() - 1;
        self.passes[pass].draws.end += 1;

        self.draws.push(DrawCommand {
            pipeline: self.render_pipeline.as_ref().unwrap().clone(),
            vertices: vertices.start as u32..vertices.end as u32,
            instances: instances.start as u32..instances.end as u32,
        });
    }

    fn reset(&mut self) {
        self.render_pipeline = None;
        self.passes.clear();
        self.draws.clear();
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.render_pipeline = Some(pipeline.clone());
    }
}

struct RenderPass {
    target: RenderTarget,
    clear_colors: [Option<Color>; 1],
    draws: Range<usize>,
}

struct DrawCommand {
    pipeline: RenderPipeline,
    vertices: Range<u32>,
    instances: Range<u32>,
}

// todo: draw target can have multiple color attachments. we want to be able to convert the following into a target:
// - backbuffer
// - render texture
// - framebuffer / gbuffer (multiple render_textures), eventually - might take some rework elsewhere.
pub struct RenderTarget {
    color_targets: [ColorTarget; Self::MAX_COLOR_TARGETS],
}

impl RenderTarget {
    const MAX_COLOR_TARGETS: usize = 1;
}

enum ColorTarget {
    Backbuffer(WindowId),
}

impl From<&Window> for RenderTarget {
    fn from(window: &Window) -> Self {
        RenderTarget {
            color_targets: [ColorTarget::Backbuffer(window.get_id())],
        }
    }
}

pub struct PipelineLayoutDesc<'desc> {
    pub label: Option<&'desc str>,
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

enum RenderMessage {
    Enqueue(CommandBuffer),
    Submit,
}

#[derive(Clone)]
pub struct RenderProxy {
    tx: Sender<RenderMessage>,
    ready_semaphore: Arc<AtomicBool>,
    stop_semaphore: Arc<AtomicBool>,
}

impl RenderProxy {
    pub fn enqueue(&self, buf: CommandBuffer) {
        todo!()
    }

    pub fn submit(&self, window: &Window) {
        // ideally we wouldn't need to pass window down, or the device needs to hold onto the window handle (which is really what we need as it has an arc of the winit window inside).
        todo!()
    }

    pub(crate) fn shutdown(&self, thread: JoinHandle<()>) {
        todo!()
    }

    pub(crate) fn sync(&self) {
        todo!()
    }
}

pub(crate) fn start_render_thread(
    device: RenderDevice,
) -> Result<(JoinHandle<()>, RenderProxy), Error> {
    let (tx, rx) = std::sync::mpsc::channel();

    let proxy = RenderProxy {
        tx,
        ready_semaphore: Arc::new(AtomicBool::new(false)),
        stop_semaphore: Arc::new(AtomicBool::new(false)),
    };

    let thread = std::thread::Builder::new()
        .name("render thread".to_string())
        .spawn(|| {
            if let Err(err) = render_thread_main(
                device,
                rx,
                proxy.ready_semaphore.clone(),
                proxy.stop_semaphore.clone(),
            ) {
                eprintln!("{err}");
            }
        })?;

    Ok((thread, proxy))
}

fn render_thread_main(
    device: RenderDevice,
    rx: Receiver<RenderMessage>,
    ready_semaphore: Arc<AtomicBool>,
    stop_semaphore: Arc<AtomicBool>,
) -> Result<(), Error> {
    ready_semaphore.store(true, Ordering::Relaxed);

    while !stop_semaphore.load(Ordering::Relaxed) {
        for message in rx.try_iter() {
            match message {
                RenderMessage::Enqueue(buf) => handle_enqueue(buf, &device),
                RenderMessage::Submit => handle_submit(&device),
            }
        }
    }

    Ok(())
}

fn handle_enqueue(buf: CommandBuffer, device: &RenderDevice) {
    // todo: command buffer doesn't need to be on the device anymore, it can be in the render thread outside the loop.
    let mut command_buffer = device
        .command_buffer
        .lock()
        .expect("failed to acquire lock on command buffer");
    if command_buffer.is_some() {
        panic!("have not processed previous command buffer. there is either a problem with thread synchronisation or multiple buffers per frame needs to be supproted");
    }

    *command_buffer = Some(buf);
}

fn handle_submit(device: &RenderDevice) {
    let Some(buf) = device
        .command_buffer
        .lock()
        .expect("failed to acquire lock on command buffer")
        .take()
    else {
        return;
    };

    let mut encoder = device
        .get_device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("submit"),
        });

    for pass in buf.passes.iter() {
        let mut color_attachments = Vec::with_capacity(pass.target.color_targets.len());
        for (i, color_target) in pass.target.color_targets.iter().enumerate() {
            let view = match color_target {
                ColorTarget::Backbuffer(window_id) => {
                    if window.get_id() == *window_id {
                        window.get_surface_texture_view()
                    } else {
                        panic!("window id does not match");
                    }
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

        for draw in &buf.draws[pass.draws.clone()] {
            rpass.set_pipeline(&draw.pipeline);
            rpass.draw(draw.vertices.clone(), draw.instances.clone())
        }
    }

    device.get_queue().submit([encoder.finish()]);
    device.return_command_buffer(buf);
}
