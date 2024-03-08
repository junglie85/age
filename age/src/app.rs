use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};

use crate::{
    graphics::Graphics,
    os,
    renderer::{DrawTarget, RenderDevice, RenderPipeline, WindowSurface, WindowTarget},
    AgeResult, Camera, Game,
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
        let el = os::create_event_loop::<AppEvent>()?;
        let el_proxy = el.create_proxy();
        let window = os::create_window(&self.config, &el)?;
        let device = RenderDevice::new()?;
        let surface = WindowSurface::new();

        let (width, height) = window.inner_size().into();
        let window_target = WindowTarget::new(width, height, &device);
        let graphics = Graphics::new(
            0.0,
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            &device,
        );

        Ok(App {
            config: self.config,
            el,
            el_proxy,
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
    el: EventLoop<AppEvent>,
    el_proxy: EventLoopProxy<AppEvent>,
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
            el_proxy,
            window,
            device,
            mut surface,
            window_target,
            graphics,
        } = self;

        let window = Arc::new(window);

        let mut ctx = Context {
            config,
            el_proxy,
            device,
            graphics,
            window_target,
            running: true,
        };

        game.on_start(&mut ctx);
        window.set_visible(true);

        os::run(el, |event, elwt| {
            match event {
                Event::WindowEvent { window_id, event } if window.id() == window_id => {
                    match event {
                        WindowEvent::CloseRequested => game.on_exit(&mut ctx),

                        WindowEvent::RedrawRequested => {
                            ctx.device.begin_frame();
                            ctx.graphics.begin_frame(&ctx.window_target);

                            game.on_tick(&mut ctx);

                            ctx.window_target.draw(&mut surface, &mut ctx.device)?;
                            ctx.device.end_frame();

                            window.pre_present_notify();
                            surface.present();
                            window.request_redraw();
                        }

                        WindowEvent::Resized(size) => {
                            let (width, height) = size.into();
                            surface.reconfigure(&ctx.device, width, height, surface.vsync())?;
                            ctx.window_target.reconfigure(&surface, &ctx.device);
                        }

                        WindowEvent::ScaleFactorChanged { .. } => {
                            todo!("handle scale factor change")
                        }

                        _ => {}
                    }
                }

                Event::Resumed => {
                    surface.resume(&ctx.device, window.clone())?;
                    ctx.window_target.reconfigure(&surface, &ctx.device);
                }

                Event::Suspended => surface.suspend(),

                Event::UserEvent(event) => match event {
                    AppEvent::EnableVsync(enabled) => {
                        if surface.vsync() != enabled {
                            let (width, height) = surface.size();
                            surface.reconfigure(&ctx.device, width, height, enabled)?
                        }
                    }
                },

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

enum AppEvent {
    EnableVsync(bool),
}

pub struct Context {
    #[allow(dead_code)]
    config: AppConfig,
    el_proxy: EventLoopProxy<AppEvent>,
    device: RenderDevice,
    graphics: Graphics,
    window_target: WindowTarget,
    running: bool,
}

impl Context {
    pub fn exit(&mut self) {
        self.running = false;
    }

    pub fn set_vsync(&self, enabled: bool) {
        if self
            .el_proxy
            .send_event(AppEvent::EnableVsync(enabled))
            .is_err()
        {
            eprintln!("attempted to send message on closed event loop");
        }
    }
}

impl Context {
    pub fn create_camera(&self, left: f32, right: f32, bottom: f32, top: f32) -> Camera {
        self.graphics
            .create_camera(left, right, bottom, top, &self.device)
    }

    pub fn default_camera(&self) -> &Camera {
        self.graphics.default_camera()
    }

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
