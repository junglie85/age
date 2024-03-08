mod app;
mod error;
mod graphics;
mod os;
mod renderer;

pub use app::{App, AppBuilder, Context};
pub use error::{AgeError, AgeResult};
pub use graphics::Camera;

pub trait Game {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context);

    fn on_stop(&mut self, _ctx: &mut Context) {}

    fn on_exit(&mut self, ctx: &mut Context) {
        ctx.exit();
    }
}
