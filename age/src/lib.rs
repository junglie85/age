use std::process::ExitCode;

pub use app::*;
pub use color::*;
pub use error::Error;
pub use graphics::*;
pub use renderer::*;

mod app;
mod color;
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
    fn on_start(app: &mut App) -> Result<T, Error>;

    fn on_update(&mut self, app: &mut App);

    fn on_exit_requested(&mut self, app: &mut App) {
        app.exit();
    }
}
