use std::process::ExitCode;

use age::{Color, Engine, Error, Game};

struct Sandbox {}

impl Game for Sandbox {
    fn on_start(_age: &mut Engine) -> Result<Self, Error> {
        Ok(Self {})
    }

    fn on_update(&mut self, age: &mut Engine) {
        age.set_render_target(age.get_backbuffer());
        age.clear(0, Color::RED);
        age.draw();
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
