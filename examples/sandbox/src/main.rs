use std::process::ExitCode;

use age::{App, Color, Error, Game};

struct Sandbox {}

impl Game for Sandbox {
    fn on_start(_app: &mut App) -> Result<Self, Error> {
        Ok(Self {})
    }

    fn on_update(&mut self, app: &mut App) {
        // app.set_draw_target(app.get_backbuffer());
        // app.clear(Color::RED);
        // app.renderer.get_command_buffer();
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
