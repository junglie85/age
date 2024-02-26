use std::process::ExitCode;

pub use color::*;
pub use engine::Engine;
pub use error::Error;
pub use graphics::*;

mod app;
mod color;
mod engine;
mod error;
mod graphics;
pub mod math;
mod renderer;
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
