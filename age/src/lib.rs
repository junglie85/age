mod app;
mod error;
mod os;

pub use app::{App, AppBuilder, Context};
pub use error::{AgeError, AgeResult};

pub trait Game {
    fn on_start(&mut self, _ctx: &mut Context) {
        println!("start");
    }

    fn on_update(&mut self, _ctx: &mut Context) {
        println!("update");
    }

    fn on_render(&mut self, _ctx: &mut Context) {
        println!("render");
    }

    fn on_stop(&mut self, _ctx: &mut Context) {
        println!("stop");
    }

    fn on_exit(&mut self, ctx: &mut Context) {
        ctx.exit();
    }
}
