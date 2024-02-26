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

impl Gpu {
    fn create_command_encoder(
        &self,
        desc: &wgpu::CommandEncoderDescriptor,
    ) -> wgpu::CommandEncoder {
        self.inner.device.create_command_encoder(desc)
    }

    fn create_surface<'window>(
        &self,
        target: impl Into<wgpu::SurfaceTarget<'window>>,
    ) -> Result<wgpu::Surface, Error> {
        let s = self.inner.instance.create_surface(target)?;

        Ok(s)
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
        if let Err(_) = self.tx.send(RenderMessage::Dispatch(ctx)) {
            panic!("render thread has been stopped");
        }
    }

    pub(crate) fn stop_render_thread(&self) {
        self.thread_stop.store(true, Ordering::Relaxed);
    }
}

pub(crate) struct RenderPass {
    pub(crate) label: Option<String>,
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

enum RenderMessage {
    Dispatch(RenderContext),
}

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
    inner: Arc<Mutex<SurfaceInner<'window>>>,
}

#[derive(Default)]
struct SurfaceInner<'window> {
    s: Option<wgpu::Surface<'window>>,
    config: Option<wgpu::SurfaceConfiguration>,
    frame: Option<wgpu::SurfaceTexture>,
}

impl<'window> Surface<'window> {
    fn acquire(&self) -> Result<wgpu::TextureView, Error> {
        let mut inner = self.inner.lock().expect("failed to acquire lock");
        assert!(inner.s.is_some(), "surface is not resumed");

        inner.frame = Some(inner.s.as_ref().unwrap().get_current_texture()?); // todo: better handling of non-acquired surface texture.

        let view =
            inner
                .frame
                .as_ref()
                .unwrap()
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("backbuffer surface"),
                    ..Default::default()
                });

        Ok(view)
    }

    fn present(&mut self) {
        let mut inner = self.inner.lock().expect("failed to acquire lock");
        if let Some(frame) = inner.frame.take() {
            frame.present();
        }
    }

    fn resume(&mut self, gpu: &Gpu, window: Window) -> Result<(), Error> {
        let s = gpu.create_surface(window)?;

        todo!();

        Ok(())
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a window surface").with_source(value)
    }
}

impl From<wgpu::SurfaceError> for Error {
    fn from(value: wgpu::SurfaceError) -> Self {
        Error::new("failed to acquire surface texture").with_source(value)
    }
}

pub(crate) fn render_thread_main(renderer: Renderer) -> Result<(), Error> {
    while !renderer.thread_stop.load(Ordering::Relaxed) {
        for message in renderer.render_proxy_rx.try_iter() {
            match message {
                RenderMessage::Dispatch(ctx) => render_thread_dispatch(&renderer, ctx)?,
            }
        }
    }

    Ok(())
}

fn render_thread_dispatch(renderer: &Renderer, ctx: RenderContext) -> Result<(), Error> {
    let gpu = renderer.gpu.clone();

    // We need to acquire the texture view from any surfaces before the main render passes so that we are not
    // trying to have exclusive and shared borrows of the vec at the same time. We cannot return a reference to
    // the view from Surface::acquire() because it would need to be behind a Mutex.
    let mut surface_views_lut = Vec::new();
    for pass in ctx.passes.iter() {
        for target in pass.target.color_targets.iter() {
            if let Some(target) = target.as_ref() {
                if let ColorTarget::Backbuffer { surface } = target {
                    surface_views_lut.push(surface.acquire()?);
                }
            }
        }
    }

    let mut encoder = gpu.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render thread"),
    });

    let command_offset = 0;
    let mut next_surface_index = 0;
    for pass in ctx.passes {
        let mut color_attachments = Vec::new();
        for (i, target) in pass.target.color_targets.iter().enumerate() {
            if let Some(target) = target.as_ref() {
                let view = match target {
                    ColorTarget::Backbuffer { .. } => {
                        // We stored the acquired texture views for the surface in the surface views LUT previously.
                        let view = &surface_views_lut[next_surface_index];
                        next_surface_index += 1;
                        view
                    }
                };

                let attachment = wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match pass.clear_colors[i] {
                            Some(color) => wgpu::LoadOp::Clear(color.into()),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                };

                color_attachments.push(Some(attachment));
            }
        }

        let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: pass.label.as_deref(),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for _draw in &ctx.draws[command_offset..pass.commands] {}
    }

    Ok(())
}
