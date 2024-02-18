use std::process::ExitCode;

use age::{App, Error, Game};

struct Sandbox;

impl Game for Sandbox {
    fn on_start(_ctx: &mut age::Ctx) -> Result<Self, Error> {
        Ok(Self)
    }

    fn on_update(&mut self, ctx: &mut age::Ctx) {
        if ctx.exit_requested() {
            ctx.exit();
        }
    }
}

fn main() -> ExitCode {
    App::new().run::<Sandbox>()
}
