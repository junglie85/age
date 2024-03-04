use std::{borrow::Cow, mem::MaybeUninit, ptr::addr_of_mut, sync::Arc};

use wgpu::{
    BlendState, ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor, Device,
    Face, FragmentState, FrontFace, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderSource, StoreOp, Surface, SurfaceConfiguration, SurfaceTexture, TextureFormat,
    TextureViewDescriptor, VertexState,
};
use winit::window::Window;

use crate::Error;

pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    pipelines: Pipelines,
}

impl Renderer {
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

        let pipelines = Pipelines::new(&device);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            pipelines,
        })
    }

    pub(crate) fn create_surface<'window>(
        &self,
        window: Arc<Window>,
    ) -> Result<(Surface<'window>, SurfaceConfiguration), Error> {
        let (width, height) = window.inner_size().into();
        let surface = self.instance.create_surface(window)?;
        let mut config = match surface.get_default_config(&self.adapter, width, height) {
            Some(config) => config,
            None => return Err("window surface is not supported by the graphics adapter".into()),
        };

        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo: deal with srgb.

        surface.configure(&self.device, &config);

        Ok((surface, config))
    }

    pub fn begin_render_pass<'pass, F>(
        &'pass self,
        target: DrawTarget,
        clear_color: Option<Color>,
        render_fn: F,
    ) -> CommandBuffer
    where
        F: Fn(&'pass Renderer, &mut RenderPass<'pass>),
    {
        // There is a bunch of shenanigans going on here to convince the borrow checker that encoder lives longer
        // that the render pass. If we don't do this, encoder stays borrowed and we can't call finish on it.
        let mut render_pass_uninit: MaybeUninit<RenderPass> = MaybeUninit::uninit();
        let ptr = render_pass_uninit.as_mut_ptr();

        // Safety: target is owned, so we can safely write it to the target field.
        unsafe {
            addr_of_mut!((*ptr).target).write(target);
        }

        let encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("render pass"),
            });
        // Safety: encoder is owned so we can safely write it to the encoder field.
        unsafe {
            addr_of_mut!((*ptr).encoder).write(encoder);
        }

        // Safety: target and encoder are both owned by the RenderPass struct, which rpass will be added to
        // and will live as long as.
        let (target, encoder) = unsafe {
            (
                &*addr_of_mut!((*ptr).target),
                &mut *addr_of_mut!((*ptr).encoder),
            )
        };
        let rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &target.color_target,
                resolve_target: None,
                ops: Operations {
                    load: match clear_color {
                        Some(color) => LoadOp::Clear(color.into()),
                        None => LoadOp::Load,
                    },
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Safety: rpass has an exclusive reference to encoder, which is owned by the RenderPass struct.
        // By writing rpass to the struct, we have initialised all fields and it is safe to call assume_init.
        let mut render_pass = unsafe {
            addr_of_mut!((*ptr).rpass).write(rpass);
            render_pass_uninit.assume_init()
        };

        render_fn(self, &mut render_pass);

        std::mem::drop(render_pass.rpass);
        CommandBuffer {
            buf: render_pass.encoder.finish(),
        }
    }

    pub(crate) fn submit(&self, buf: CommandBuffer) {
        self.queue.submit([buf.buf]);
    }

    pub fn draw_filled_rect<'pass>(&'pass self, rpass: &mut RenderPass<'pass>) {
        let p = &mut rpass.rpass;
        p.set_pipeline(&self.pipelines.fill);
        p.draw(0..3, 0..1);
    }
}

struct Pipelines {
    fill: RenderPipeline,
}

impl Pipelines {
    fn new(device: &Device) -> Self {
        let fill = Self::create_fill_pipeline(device);

        Self { fill }
    }

    fn create_fill_pipeline(device: &Device) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("fill"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("fill"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/fill.wgsl"))),
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("fill"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
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
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8Unorm, // todo: specialisation - rgba and srgb
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        })
    }
}

pub struct RenderPass<'pass> {
    target: DrawTarget,
    encoder: CommandEncoder,
    rpass: wgpu::RenderPass<'pass>,
}

pub struct CommandBuffer {
    buf: wgpu::CommandBuffer,
}

pub struct DrawTarget {
    color_target: wgpu::TextureView,
}

impl From<&SurfaceTexture> for DrawTarget {
    fn from(surface_texture: &SurfaceTexture) -> Self {
        let color_target = surface_texture.texture.create_view(&TextureViewDescriptor {
            label: Some("window surface"),
            ..Default::default()
        });

        Self { color_target }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_u8(r, g, b, 255)
    }

    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        let a = a as f32 / 255.0;

        Self { r, g, b, a }
    }

    pub const fn to_array_f32(self) -> [f32; 4] {
        let r = self.r;
        let g = self.g;
        let b = self.b;
        let a = self.a;

        [r, g, b, a]
    }

    pub fn to_array_u8(self) -> [u8; 4] {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let a = (self.a * 255.0) as u8;

        [r, g, b, a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}
