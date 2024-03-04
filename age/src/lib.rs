mod app;
mod math;
mod renderer;

use std::{fmt::Display, process::ExitCode};

pub use app::App;
pub use math::*;
pub use renderer::Color;

pub fn run<G: Game>(width: u32, height: u32, px_width: u32, px_height: u32) -> ExitCode {
    match app::run::<G>(width, height, px_width, px_height) {
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

    fn on_window_resized(&mut self, _app: &mut App, _width: u32, _height: u32) {}
}

#[derive(Debug)]
pub struct Error {
    msg: String,
    src: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            msg: msg.into(),
            src: None,
        }
    }

    pub fn with_source<E: std::error::Error + 'static>(self, err: E) -> Self {
        Self {
            src: Some(Box::new(err)),
            ..self
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.msg)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_deref()
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Self::new(msg)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new("an i/o operation failed").with_source(err)
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(err: wgpu::CreateSurfaceError) -> Self {
        Error::new("failed to create window surface").with_source(err)
    }
}

impl From<winit::error::EventLoopError> for Error {
    fn from(err: winit::error::EventLoopError) -> Self {
        Self::new("an event loop error occurred").with_source(err)
    }
}

impl From<winit::error::OsError> for Error {
    fn from(err: winit::error::OsError) -> Self {
        Self::new("an operating system error occurred").with_source(err)
    }
}
