use std::process::ExitCode;

pub use color::*;
pub use error::Error;
pub use graphics::Sprite;
use renderer::{CommandBuffer, DrawTarget};

mod app;
mod color;
mod error;
mod graphics;
mod renderer;
mod sys;

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

    // Graphics
    // todo: Encapsulate in a graphics context?
    draw_target: DrawTarget,
    clear_color: Option<Color>,
    needs_render_pass: bool,
    draws: CommandBuffer,
}

impl Engine {
    fn new<T: Into<DrawTarget>>(draw_target: T) -> Self {
        Self {
            exit: false,

            // Graphics.
            draw_target: draw_target.into(),
            clear_color: None,
            needs_render_pass: true,
            draws: CommandBuffer::default(),
        }
    }
}
