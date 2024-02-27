use std::process::ExitCode;

use age::{App, Color, Error, Game, Graphics, Renderer};

struct Sandbox {}

impl Game for Sandbox {
    fn on_start(_app: &mut App) -> Result<Self, Error> {
        Ok(Self {})
    }

    fn on_update(&mut self, app: &mut App) {
        app.set_draw_target(app.get_backbuffer());
        app.clear(Color::RED);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
