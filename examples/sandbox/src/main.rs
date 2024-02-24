use std::process::ExitCode;

use age::{Color, Engine, Error, Game};

struct Sandbox {}

impl Game for Sandbox {
    fn on_start(_age: &mut Engine) -> Result<Self, Error> {
        Ok(Self {})
    }

    fn on_update(&mut self, age: &mut Engine) {
        age.clear(Color::RED);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
