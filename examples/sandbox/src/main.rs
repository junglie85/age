use std::process::ExitCode;

use age::{Color, Engine, Error, Game, Sprite};

struct Sandbox {
    sprite: Sprite,
}

impl Game for Sandbox {
    fn on_start(_age: &mut Engine) -> Result<Self, Error> {
        let sprite = Sprite::new(100, 200);

        Ok(Self { sprite })
    }

    fn on_update(&mut self, age: &mut Engine) {
        age.clear(Color::RED);
        age.draw_sprite(&self.sprite);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
