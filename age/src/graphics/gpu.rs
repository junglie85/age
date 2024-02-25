use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::{sys::Window, Color, Error};

#[derive(Clone)]
pub struct Gpu {
    inner: Arc<GpuInner>,
    render_contexts: Arc<Mutex<VecDeque<RenderContext>>>,
}

struct GpuInner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
    const RENDER_CONTEXT_POOL_SIZE: usize = 2;

    pub(crate) fn new() -> Result<Self, Error> {
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

        let mut render_contexts = VecDeque::new();
        for _ in 0..Self::RENDER_CONTEXT_POOL_SIZE {
            render_contexts.push_back(RenderContext::new());
        }

        Ok(Self {
            inner: Arc::new(GpuInner {
                instance,
                adapter,
                device,
                queue,
            }),
            render_contexts: Arc::new(Mutex::new(render_contexts)),
        })
    }

    pub(crate) fn create_backbuffer(&self, info: &BackbufferInfo) -> Backbuffer {
        Backbuffer {
            window: info.window.clone(),
            surface: Surface::default(),
        }
    }

    pub(crate) fn get_render_context(&self) -> RenderContext {
        let mut render_contexts = self.render_contexts.lock().expect("failed to obtain lock");
        match render_contexts.pop_front() {
            Some(ctx) => ctx,
            None => RenderContext::new(), // The pool needs to grow, presumably because we have more threads doing work.
        }
    }

    fn return_render_ctx(&self, ctx: RenderContext) {
        let mut ctx = ctx;
        ctx.clear();

        let mut render_contexts = self.render_contexts.lock().expect("failed to obtain lock");
        render_contexts.push_back(ctx);
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a window surface").with_source(value)
    }
}

pub(crate) struct Renderer {
    gpu: Gpu,
    render_proxy_tx: Sender<RenderMessage>,
    render_proxy_rx: Receiver<RenderMessage>,
    thread_stop: Arc<AtomicBool>,
}

impl Renderer {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            gpu: gpu.clone(),
            render_proxy_tx: tx,
            render_proxy_rx: rx,
            thread_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn create_render_proxy(&self) -> RenderProxy {
        RenderProxy {
            tx: self.render_proxy_tx.clone(),
            thread_stop: self.thread_stop.clone(),
        }
    }
}

pub(crate) struct RenderProxy {
    tx: Sender<RenderMessage>,
    thread_stop: Arc<AtomicBool>,
}

impl RenderProxy {
    pub(crate) fn dispatch(&self, ctx: RenderContext) {
        todo!()
    }

    pub(crate) fn stop_render_thread(&self) {
        self.thread_stop.store(true, Ordering::Relaxed);
    }
}

pub(crate) struct RenderPass {
    pub(crate) target: RenderTarget,
    pub(crate) clear_colors: [Option<Color>; RenderTarget::MAX_COLOR_SLOTS],
    // viewport
    // scissor
    pub(crate) commands: usize,
}

pub(crate) struct DrawCommand {}

pub(crate) struct RenderContext {
    passes: Vec<RenderPass>,
    draws: Vec<DrawCommand>,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            passes: Vec::new(),
            draws: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.passes.clear();
        self.draws.clear();
    }

    pub(crate) fn draw(&mut self, cmd: DrawCommand) {
        assert!(
            !self.passes.is_empty(),
            "called draw without a valid render pass"
        );

        let pass = self.passes.len() - 1;
        self.passes[pass].commands += 1;
    }

    pub(crate) fn set_render_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }
}

enum RenderMessage {}

pub struct BackbufferInfo<'info> {
    pub window: &'info Window,
}

#[derive(Clone)]
pub struct Backbuffer {
    window: Window,
    surface: Surface<'static>,
}

impl Backbuffer {
    pub(crate) fn resume(&mut self, gpu: &Gpu) {
        self.surface.resume(gpu, self.window.clone());
    }

    pub(crate) fn present(&mut self) {
        self.surface.present();
    }
}

#[derive(Clone)]
pub struct RenderTarget {
    color_targets: [Option<ColorTarget>; Self::MAX_COLOR_SLOTS],
}

impl RenderTarget {
    pub(crate) const MAX_COLOR_SLOTS: usize = 4;
}

#[derive(Clone)]
enum ColorTarget {
    Backbuffer { surface: Surface<'static> },
}

impl From<Backbuffer> for RenderTarget {
    fn from(backbuffer: Backbuffer) -> Self {
        let mut color_targets = [None, None, None, None];
        color_targets[0] = Some(ColorTarget::Backbuffer {
            surface: backbuffer.surface.clone(),
        });

        RenderTarget { color_targets }
    }
}

#[derive(Default, Clone)]
struct Surface<'window> {
    inner: Arc<SurfaceInner<'window>>,
}

#[derive(Default)]
struct SurfaceInner<'window> {
    s: Option<wgpu::Surface<'window>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl<'window> Surface<'window> {
    fn present(&mut self) {}

    fn resume(&mut self, gpu: &Gpu, window: Window) {}
}

pub(crate) fn render_thread_main(renderer: Renderer) -> Result<(), Error> {
    while !renderer.thread_stop.load(Ordering::Relaxed) {
        for message in renderer.render_proxy_rx.try_iter() {
            match message {}
        }
    }

    Ok(())
}
