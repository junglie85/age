use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{
    graphics::Graphics,
    os,
    renderer::{
        DrawTarget, PipelineLayoutInfo, RenderDevice, RenderPipeline, RenderPipelineInfo,
        ShaderInfo, TextureFormat, WindowSurface, WindowTarget,
    },
    AgeResult, Game,
};

pub(crate) struct AppConfig {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) title: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            title: "AGE".to_string(),
        }
    }
}

#[derive(Default)]
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        let config = AppConfig {
            width,
            height,
            ..Default::default()
        };

        Self { config }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn build(self) -> AgeResult<App> {
        let el = os::create_event_loop()?;
        let window = os::create_window(&self.config, &el)?;
        let device = RenderDevice::new()?;
        let surface = WindowSurface::new();

        let (width, height) = window.inner_size().into();
        let window_target = WindowTarget::new(width, height, &device);

        let graphics = Graphics::new();

        Ok(App {
            config: self.config,
            el,
            window,
            device,
            surface,
            window_target,
            graphics,
        })
    }
}

pub struct App {
    config: AppConfig,
    el: EventLoop<()>,
    window: Window,
    device: RenderDevice,
    surface: WindowSurface,
    window_target: WindowTarget,
    graphics: Graphics,
}

impl App {
    pub fn new(width: u32, height: u32) -> AgeResult<Self> {
        AppBuilder::new(width, height).build()
    }

    pub fn run(self, mut game: impl Game) -> AgeResult {
        let App {
            config,
            el,
            window,
            device,
            mut surface,
            window_target,
            graphics,
        } = self;

        let window = Arc::new(window);

        // Start - Temporary.
        let shader = device.create_shader(&ShaderInfo {
            label: Some("triangle"),
            src: include_str!("shaders/triangle.wgsl"),
        });
        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("triangle"),
            bind_group_layouts: &[],
        });
        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("triangle"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Rgba8Unorm,
        });
        // End - Temporary.

        let mut ctx = Context {
            config,
            device,
            graphics,
            window_target,
            running: true,
        };

        game.on_start(&mut ctx);
        window.set_visible(true);

        os::run(el, |event, elwt| {
            #[allow(clippy::collapsible_match)]
            match event {
                Event::WindowEvent { window_id, event } if window.id() == window_id => {
                    #[allow(clippy::single_match)]
                    match event {
                        WindowEvent::CloseRequested => game.on_exit(&mut ctx),

                        WindowEvent::RedrawRequested => {
                            game.on_update(&mut ctx);

                            ctx.device.begin_frame();
                            ctx.graphics.set_draw_target(&ctx.window_target);
                            ctx.graphics.set_render_pipeline(&triangle_pipeline); // todo: move this pipeline into graphics.

                            game.on_render(&mut ctx);

                            ctx.device.end_frame(&mut surface, &ctx.window_target)?;

                            window.pre_present_notify();
                            surface.present();
                            window.request_redraw();
                        }

                        _ => {}
                    }
                }

                Event::Resumed => {
                    surface.resume(&ctx.device, window.clone())?;
                    ctx.window_target.reconfigure(&surface, &ctx.device);
                }
                Event::Suspended => surface.suspend(),

                _ => {}
            }

            if !ctx.running {
                elwt.exit();
            }

            Ok(())
        })?;

        game.on_stop(&mut ctx);

        Ok(())
    }
}

pub struct Context {
    #[allow(dead_code)]
    config: AppConfig,
    device: RenderDevice,
    graphics: Graphics,
    window_target: WindowTarget,
    running: bool,
}

impl Context {
    pub fn exit(&mut self) {
        self.running = false;
    }
}

impl Context {
    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.graphics.set_draw_target(target);
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.graphics.set_render_pipeline(pipeline);
    }

    pub fn draw_filled_triangle(&mut self) {
        self.graphics.draw_filled_triangle(&mut self.device);
    }
}
