use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::{os, AgeResult, Game};

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

        Ok(App {
            config: self.config,
            el,
            window,
        })
    }
}

pub struct App {
    config: AppConfig,
    el: EventLoop<()>,
    window: Window,
}

impl App {
    pub fn new(width: u32, height: u32) -> AgeResult<Self> {
        AppBuilder::new(width, height).build()
    }

    pub fn run(self, mut game: impl Game) -> AgeResult {
        let App { config, el, window } = self;
        let mut ctx = Context {
            config,
            running: true,
        };

        game.on_start(&mut ctx);
        window.set_visible(true);

        os::run(el, |event, elwt| {
            #[allow(clippy::collapsible_match)]
            match event {
                Event::WindowEvent { window_id, event } if window.id() == window_id =>
                {
                    #[allow(clippy::single_match)]
                    match event {
                        WindowEvent::CloseRequested => game.on_exit(&mut ctx),

                        WindowEvent::RedrawRequested => {
                            game.on_update(&mut ctx);
                            game.on_render(&mut ctx);
                            window.request_redraw();
                        }

                        _ => {}
                    }
                }

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
    running: bool,
}

impl Context {
    pub fn exit(&mut self) {
        self.running = false;
    }
}
