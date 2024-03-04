use std::{sync::Arc, time::Duration};

use wgpu::{Surface, SurfaceConfiguration, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::pump_events::{EventLoopExtPumpEvents, PumpStatus},
    window::{Window, WindowBuilder},
};

use crate::{
    renderer::{CommandBuffer, DrawTarget, RenderPass, Renderer},
    Color, Error, Game,
};

pub(crate) fn run<G: Game>(
    width: u32,
    height: u32,
    px_width: u32,
    px_height: u32,
) -> Result<(), Error> {
    let mut el = EventLoop::new()?;

    let logical_size = LogicalSize::new(width * px_width, height * px_height);
    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(logical_size)
            .with_title("AGE")
            .with_visible(false)
            .build(&el)?,
    );

    let renderer = Renderer::new()?;
    let (surface, config) = renderer.create_surface(window.clone())?;

    let mut app = App {
        exit: false,
        renderer,
        window,
        window_surface: surface,
        window_surface_config: Some(config),
        window_surface_texture: None,
    };

    let mut game = G::on_start(&mut app)?;

    app.show_window(true);

    let mut running = true;
    while running {
        let timeout = Some(Duration::ZERO);
        let status = el.pump_events(timeout, |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            #[allow(clippy::collapsible_match)]
            match event {
                Event::WindowEvent { window_id, event } if window_id == app.window.id() =>
                {
                    #[allow(clippy::single_match)]
                    match event {
                        WindowEvent::CloseRequested => game.on_exit_requested(&mut app),

                        _ => {}
                    }
                }

                _ => {}
            }
        });
        if let PumpStatus::Exit(exit_code) = status {
            return Err(Error::new(format!(
                "event loop exited with exit code {exit_code}"
            )));
        }

        // Update & render.
        game.on_update(&mut app);

        if app.exit {
            running = false;
        }
    }

    Ok(())
}

pub struct App {
    exit: bool,
    renderer: Renderer,
    window: Arc<Window>,
    window_surface: Surface<'static>,
    #[allow(dead_code)]
    window_surface_config: Option<SurfaceConfiguration>,
    window_surface_texture: Option<SurfaceTexture>,
}

impl App {
    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn show_window(&self, show: bool) {
        self.window.set_visible(show);
    }

    pub fn update_display(&mut self, buf: CommandBuffer) {
        let Some(surface_texture) = self.window_surface_texture.take() else {
            return;
        };

        self.renderer.submit(buf);
        self.window.pre_present_notify();
        surface_texture.present();
        self.window.request_redraw();
    }

    pub fn window_draw_target(&mut self) -> DrawTarget {
        // todo: deal with swapchin error
        let surface_texture = self.window_surface.get_current_texture().unwrap();
        self.window_surface_texture = Some(surface_texture);
        self.window_surface_texture.as_ref().unwrap().into()
    }

    pub fn begin_render_pass<'pass, F>(
        &'pass self,
        target: DrawTarget,
        clear_color: Option<Color>,
        f: F,
    ) -> CommandBuffer
    where
        F: Fn(&'pass Renderer, &mut RenderPass<'pass>),
    {
        self.renderer.begin_render_pass(target, clear_color, f)
    }
}
