use std::process::ExitCode;

pub use error::Error;
pub use graphics::*;

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
}

impl Engine {
    fn new() -> Self {
        Self { exit: false }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}

// Graphics
impl Engine {
    pub fn clear(&mut self, color: Color) {}
}
