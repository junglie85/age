use std::{borrow::Cow, sync::Arc};

use crate::{sys::Window, Color, Error};

pub(crate) struct Renderer<'window> {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'window>>,
    config: Option<wgpu::SurfaceConfiguration>,
    frame: Option<wgpu::SurfaceTexture>,

    backbuffer_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    backbuffer_pl: wgpu::PipelineLayout,
    backbuffer_pipeline: Arc<wgpu::RenderPipeline>,
}

impl<'window> Renderer<'window> {
    pub(crate) fn new() -> Result<Self, Error> {
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

        let backbuffer_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("window"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let backbuffer_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("window"),
            bind_group_layouts: &[&backbuffer_bgl],
            push_constant_ranges: &[],
        });

        let window_shader_src = include_str!("window.wgsl");
        let window_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("window"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(window_shader_src)),
        });

        let backbuffer_pipeline = Arc::new(device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("window"),
                layout: Some(&backbuffer_pl),
                vertex: wgpu::VertexState {
                    module: &window_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
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
                    module: &window_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm, // todo: get this from the surface
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            },
        ));

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: None,
            config: None,
            frame: None,

            backbuffer_bgl,
            backbuffer_pl,
            backbuffer_pipeline,
        })
    }

    pub(crate) fn attach_to_window(&mut self, window: Window) -> Result<(), Error> {
        let (width, height) = (window.width(), window.height());
        let s = self.instance.create_surface(window)?;
        let mut config = match s.get_default_config(&self.adapter, width, height) {
            Some(config) => config,
            None => return Err("window surface is not supported by the graphics adapter".into()),
        };

        config.format = wgpu::TextureFormat::Bgra8Unorm; // todo: deal with srgb.

        s.configure(&self.device, &config);

        self.surface = Some(s);
        self.config = Some(config);

        Ok(())
    }

    pub(crate) fn create_backbuffer(&self) -> Backbuffer {
        Backbuffer::new(
            &self.device,
            &self.backbuffer_pipeline,
            &self.backbuffer_bgl,
        )
    }

    pub(crate) fn present(&mut self) {
        if let Some(frame) = self.frame.take() {
            frame.present();
        }
    }

    pub(crate) fn submit(&mut self, buf: CommandBuffer, backbuffer: &Backbuffer) {
        // This could all be done on a background thread.

        let mut buf = buf;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("submit"),
            });

        for pass in buf.passes.iter() {
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pass.target.inner.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: match pass.clear_color {
                            Some(color) => wgpu::LoadOp::Clear(color.into()),
                            None => wgpu::LoadOp::Load,
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for _draw in buf.draws.drain(0..pass.draw_count) {}
        }

        if let Some(surface) = self.surface.as_ref() {
            let frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(_) => {
                    // todo: try to recreate
                    panic!("failed to obtain next surface texture");
                }
            };

            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("surface"),
                format: Some(wgpu::TextureFormat::Bgra8Unorm), // todo: handle srgb, other formats
                ..Default::default()
            });

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::BLUE.into()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                rpass.set_pipeline(&self.backbuffer_pipeline);
                rpass.set_bind_group(0, &backbuffer.bg, &[]);
                rpass.draw(0..3, 0..1);
            }

            self.frame = Some(frame);
        }

        self.queue.submit([encoder.finish()]);
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create a window surface").with_source(value)
    }
}

pub struct DrawTarget {
    texture: Texture,
}

impl DrawTarget {
    pub(crate) fn texture(&self) -> &Texture {
        &self.texture
    }
}

pub(crate) struct Backbuffer {
    #[allow(dead_code)]
    pipeline: Arc<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    texture: Texture,
    bg: Arc<wgpu::BindGroup>,
}

impl Backbuffer {
    fn new(
        device: &wgpu::Device,
        pipeline: &Arc<wgpu::RenderPipeline>,
        bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backbuffer"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backbuffer"),
            size: wgpu::Extent3d {
                width: 1920, // todo: get these
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // todo: can we use srgb?
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[], // todo: srgb?
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("backbuffer"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backbuffer"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });

        Self {
            pipeline: pipeline.clone(),
            sampler,
            texture: Texture {
                inner: Arc::new(TextureInner {
                    texture,
                    texture_view,
                }),
            },
            bg: Arc::new(bg),
        }
    }
}

impl From<&Backbuffer> for DrawTarget {
    fn from(backbuffer: &Backbuffer) -> Self {
        DrawTarget {
            texture: backbuffer.texture.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Texture {
    inner: Arc<TextureInner>,
}

struct TextureInner {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

#[derive(Default, Clone)]
pub(crate) struct CommandBuffer {
    passes: Vec<RenderPass>,
    draws: Vec<DrawCommand>,
}

impl CommandBuffer {
    pub(crate) fn clear(&mut self) {
        self.passes.clear();
        self.draws.clear();
    }

    pub(crate) fn set_render_pass(&mut self, target: &Texture, clear_color: Option<Color>) {
        // todo: some accounting if we already have a render pass.
        self.passes.push(RenderPass {
            target: target.clone(),
            clear_color,
            draw_count: 0,
        });
    }
}

#[derive(Clone)]
pub(crate) struct RenderPass {
    pub(crate) target: Texture,
    pub(crate) clear_color: Option<Color>,
    pub(crate) draw_count: usize,
}

#[derive(Clone)]
pub(crate) struct DrawCommand {}
