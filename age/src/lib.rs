use std::process::ExitCode;

pub use app::*;
pub use color::*;
pub use error::Error;
pub use graphics::Camera;
pub use renderer::{
    BindGroup, BindGroupDesc, BindGroupLayout, BindGroupLayoutDesc, BindingResource, BindingType,
    Buffer, BufferDesc, BufferType, PipelineLayoutDesc, RenderDevice, RenderPipeline,
    RenderPipelineDesc, RenderProxy, ShaderDesc, TextureFormat,
};

mod app;
mod color;
mod error;
mod graphics;
pub mod math;
mod os;
mod renderer;
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

    fn on_window_resized(&mut self, _app: &mut App, _width: u32, _height: u32) {}
}
