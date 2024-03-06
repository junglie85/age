use std::sync::Arc;

use wgpu::{
    CommandEncoderDescriptor, CreateSurfaceError, LoadOp, Operations, PresentMode,
    RenderPassColorAttachment, RenderPassDescriptor, StoreOp, Surface, SurfaceError,
    SurfaceTexture, TextureFormat, TextureView, TextureViewDescriptor,
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

    pub(crate) fn end_frame(&self, surface: &mut WindowSurface) -> AgeResult {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("end frame"),
            });

        {
            let view = surface.acquire()?;
            let _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("window surface"),
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
        }

        self.queue.submit([encoder.finish()]);

        Ok(())
    }
}

pub(crate) struct WindowSurface {
    surface: Option<Surface<'static>>,
    surface_texture: Option<SurfaceTexture>,
    vsync: bool,
}

impl WindowSurface {
    pub(crate) fn new() -> Self {
        Self {
            surface: None,
            surface_texture: None,
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

        Ok(view)
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

        config.format = TextureFormat::Bgra8Unorm; // todo - srgb + pick best format.
        config.present_mode = present_mode;

        surface.configure(&device.device, &config);

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
