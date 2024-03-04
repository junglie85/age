use std::process::ExitCode;

use age::{App, Color, Error, Game};

struct Sandbox {}

impl Game for Sandbox {
    fn on_start(_app: &mut App) -> Result<Self, Error> {
        Ok(Self {})
    }

    fn on_update(&mut self, app: &mut App) {
        let window = app.window_draw_target();
        let buf = app.begin_render_pass(window, Some(Color::RED), |renderer, rpass| {
            renderer.draw_filled_rect(rpass);
        });
        app.update_display(buf);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>(1920, 1080, 1, 1)
}

// draw_pixel()
// draw_line()
// draw_circle()
// draw_filled_circle()
// draw_textured_circle()
// draw_rect()
// draw_filled_rect()
// draw_textured_rect()
// draw_sprite()
// draw_sprite_rect()
// draw_text()
