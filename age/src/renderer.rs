use std::sync::Arc;

use crate::{sys::Window, App, Error};

#[derive(Clone)]
pub struct Gpu {
    inner: Arc<GpuInner>,
}

struct GpuInner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Gpu {
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
            inner: Arc::new(GpuInner {
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

    fn create_surface(&self, window: Window) -> Result<wgpu::Surface<'_>, Error> {
        let surface = self.get_instance().create_surface(window)?;

        Ok(surface)
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a surface for the window").with_source(value)
    }
}

pub(crate) struct Renderer {
    gpu: Gpu,
}

impl Renderer {
    pub(crate) fn init(gpu: &Gpu) -> Self {
        Self { gpu: gpu.clone() }
    }

    fn get_backbuffer(&self) -> Backbuffer {
        // Backbuffer needs to be clone because we want to do app.set_target(app.get_backbuffer()) but set target needs exclusive borrow.
        todo!()
    }
}

#[derive(Clone)]
pub struct Backbuffer {}

// todo: draw target can have multiple color attachments. we want to be able to convert the following into a target:
// - backbuffer
// - render texture
// - framebuffer / gbuffer (multiple render_textures), eventually - might take some rework elsewhere.
pub struct RenderTarget {
    color_targets: [(); Self::MAX_COLOR_TARGETS],
}

impl RenderTarget {
    const MAX_COLOR_TARGETS: usize = 4;
}

impl From<Backbuffer> for RenderTarget {
    fn from(backbuffer: Backbuffer) -> Self {
        todo!()
    }
}
