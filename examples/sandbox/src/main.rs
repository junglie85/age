use std::process::ExitCode;

use age::{App, Error, Game};
use age_renderer::{Renderer, RendererCtx};

struct Sandbox;

impl Game for Sandbox {
    fn on_start(_ctx: &mut age::Ctx) -> Result<Self, Error> {
        Ok(Self)
    }

    fn on_update(&mut self, ctx: &mut age::Ctx) {
        ctx.do_thing();
        if ctx.exit_requested() {
            ctx.exit();
        }
    }
}

fn main() -> ExitCode {
    App::new().with_plugin::<Renderer>().run::<Sandbox>()
}
