use std::process::ExitCode;

pub use error::Error;
pub use graphics::*;
use sys::Window;

mod app;
mod error;
mod graphics;
pub mod math;
mod sys;
pub mod util;

pub fn run<G: Game>() -> ExitCode {
    match app::run::<G>() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

pub trait Game<T = Self> {
    fn on_start(age: &mut Engine) -> Result<T, Error>;

    fn on_update(&mut self, age: &mut Engine);

    fn on_exit_requested(&mut self, age: &mut Engine) {
        age.exit();
    }
}

pub struct Engine {
    exit: bool,
    window: Window,
    backbuffer: Backbuffer,
    gpu: Gpu,
    render_ctx: RenderContext,

    needs_render_pass: bool,
    render_target: RenderTarget,
    clear_colors: [Option<Color>; RenderTarget::MAX_COLOR_SLOTS],
}

impl Engine {
    fn new(window: Window, backbuffer: Backbuffer, gpu: Gpu) -> Self {
        let render_ctx = gpu.get_render_context();
        let render_target = backbuffer.clone().into();
        Self {
            exit: false,
            window,
            backbuffer,
            gpu,
            render_ctx,

            needs_render_pass: true,
            render_target,
            clear_colors: [None; RenderTarget::MAX_COLOR_SLOTS],
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    fn on_resume(&mut self) {
        self.backbuffer.resume(&self.gpu);
        self.window.set_visible(true);
    }

    fn update(&mut self, render_proxy: &RenderProxy) {
        let render_ctx = self.gpu.get_render_context();
        let render_ctx = std::mem::replace(&mut self.render_ctx, render_ctx);

        render_proxy.dispatch(render_ctx);
        self.window.pre_present();
        self.backbuffer.present();
        self.window.post_present();
    }
}

// Graphics
impl Engine {
    pub fn clear(&mut self, slot: usize, color: Color) {
        assert!(slot < self.clear_colors.len());

        self.clear_colors[slot] = Some(color);
        self.needs_render_pass = true;
    }

    pub fn draw(&mut self) {
        if self.needs_render_pass {
            self.needs_render_pass = false;

            self.render_ctx.set_render_pass(RenderPass {
                label: None,
                target: self.render_target.clone(),
                clear_colors: self.clear_colors,
                commands: 0,
            });
        }
    }

    pub fn get_backbuffer(&self) -> Backbuffer {
        self.backbuffer.clone()
    }

    pub fn set_render_target<T: Into<RenderTarget>>(&mut self, target: T) {
        self.render_target = target.into();

        self.clear_colors = [None; RenderTarget::MAX_COLOR_SLOTS];
        self.needs_render_pass = true;
    }
}
