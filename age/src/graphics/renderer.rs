use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use crate::{sys::Window, Error, RendererId, RendererResource, Rhi};

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    // belt: wgpu::util::StagingBelt,
    tx: Sender<RenderMessage>,
    rx: Receiver<RenderMessage>,
    stop: Arc<AtomicBool>,
}

impl Renderer {
    pub(crate) fn new(stop: Arc<AtomicBool>) -> Result<Self, Error> {
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

        // let belt = wgpu::util::StagingBelt::new(1024);

        let (tx, rx) = std::sync::mpsc::channel();

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            // belt,
            tx,
            rx,
            stop,
        })
    }

    pub fn create_render_proxy(&self) -> RenderProxy {
        RenderProxy {
            ids: Arc::new(Mutex::new(Vec::new())),
            tx: self.tx.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RenderProxy {
    ids: Arc<Mutex<Vec<RendererId>>>,
    tx: Sender<RenderMessage>,
}

impl RenderProxy {
    pub(crate) fn create_backbuffer(&self, window: Window) -> RendererId {
        let renderer_id = Rhi::get().reserve_renderer_id(RendererResource::Backbuffer);

        self.tx
            .send(RenderMessage::CreateBackbuffer {
                renderer_id,
                window,
            })
            .expect("render thread is not running");

        renderer_id
    }
}

enum RenderMessage {
    CreateBackbuffer {
        renderer_id: RendererId,
        window: Window,
    },
}

struct Backbuffer<'window> {
    window: Window,
    surface: wgpu::Surface<'window>,
    config: wgpu::SurfaceConfiguration,
}

impl<'window> Backbuffer<'window> {
    fn new(renderer: &Renderer, window: Window) -> Result<Self, Error> {
        let surface = renderer.instance.create_surface(window.clone())?;
        let mut config =
            match surface.get_default_config(&renderer.adapter, window.width(), window.height()) {
                Some(config) => config,
                None => {
                    return Err("window surface is not supported by the graphics adapter".into())
                }
            };

        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo: deal with srgb.
        surface.configure(&renderer.device, &config);

        Ok(Self {
            window,
            surface,
            config,
        })
    }
}

#[derive(Default)]
struct GpuResources<'window> {
    backbuffers: Vec<Backbuffer<'window>>,
    backbuffers_free: VecDeque<usize>,

    lut: Vec<usize>,
}

pub(crate) fn main_thread(renderer: Renderer) -> Result<(), Error> {
    println!("render thread started");

    let mut resources = GpuResources::default();

    while !renderer.stop.load(Ordering::Relaxed) {
        for message in renderer.rx.try_iter() {
            match message {
                RenderMessage::CreateBackbuffer {
                    renderer_id,
                    window,
                } => {
                    let backbuffer = Backbuffer::new(&renderer, window)?;

                    let index = match resources.backbuffers_free.pop_front() {
                        Some(index) => {
                            resources.backbuffers[index] = backbuffer;
                            index
                        }
                        None => {
                            resources.backbuffers.push(backbuffer);
                            resources.backbuffers.len()
                        }
                    };

                    let (_, id) = renderer_id.split();
                    if resources.lut.len() <= id as usize {
                        resources.lut.resize(id as usize + 1, 0);
                    }
                    resources.lut[id as usize] = index;
                }
            }
        }
    }

    println!("render thread stopped");

    Ok(())
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a window surface").with_source(value)
    }
}
