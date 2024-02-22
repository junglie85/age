use std::process::ExitCode;

use age::{Color, Engine, Error, Game, Sprite};

struct Sandbox {
    sprite: Sprite,
}

impl Game for Sandbox {
    fn on_start(age: &mut Engine) -> Result<Self, Error> {
        let sprite =
            Sprite::from_image(&mut age.renderer, 100, 200, age.graphics.default_material());

        Ok(Self { sprite })
    }

    fn on_update(&mut self, age: &mut Engine) {
        age.graphics.clear(Color::RED);
        age.graphics.draw_sprite(&self.sprite);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
