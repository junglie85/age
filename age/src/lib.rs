use std::process::ExitCode;

pub use color::*;
pub use error::Error;
pub use graphics::{Graphics, Sprite};
use renderer::Renderer;

mod app;
mod color;
mod error;
mod gen_vec;
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
    pub renderer: Renderer,
    pub graphics: Graphics,
}

impl Engine {
    fn new(renderer: Renderer, graphics: Graphics) -> Self {
        Self {
            exit: false,
            renderer,
            graphics,
        }
    }
}
