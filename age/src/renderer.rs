use std::sync::Arc;

use crate::{app::Resource, sys::WinitWindow, App, Error};

#[derive(Clone)]
pub struct WgpuGpu {
    inner: Arc<WgpuGpuInner>,
}

struct WgpuGpuInner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WgpuGpu {
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

        Ok(Self {
            inner: Arc::new(WgpuGpuInner {
                instance,
                adapter,
                device,
                queue,
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

    fn get_queue(&self) -> &wgpu::Queue {
        &self.inner.queue
    }

    fn create_surface(&self, window: WinitWindow) -> Result<wgpu::Surface<'_>, Error> {
        let surface = self.get_instance().create_surface(window)?;

        Ok(surface)
    }
}

impl Resource for WgpuGpu {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create window surface").with_source(value)
    }
}

pub(crate) struct WgpuRenderer {
    gpu: WgpuGpu,
}

impl WgpuRenderer {
    pub(crate) fn init(gpu: &WgpuGpu) -> Self {
        Self { gpu: gpu.clone() }
    }
}

impl Resource for WgpuRenderer {
    fn on_resume(&mut self, app: &mut App) {
        let window = app.get_resource::<WinitWindow>();
        // todo: what to do with the surface?
        self.gpu.create_surface(window.clone());
        // todo: let's add backbuffer as a resource in the app.
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait Renderer {
    fn get_backbuffer(&self) -> Backbuffer;
}

impl Renderer for App {
    fn get_backbuffer(&self) -> Backbuffer {
        // Backbuffer needs to be clone because we want to do app.set_target(app.get_backbuffer()) but set target needs exclusive borrow.
        todo!()
    }
}

#[derive(Clone)]
pub struct Backbuffer {}
