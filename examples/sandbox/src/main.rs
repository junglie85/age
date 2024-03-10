use age::{AgeResult, App, Color, Context, Game};
use age_math::v2;

struct Sandbox {}

impl Sandbox {
    fn new(_app: &App) -> AgeResult<Self> {
        Ok(Self {})
    }
}

impl Game for Sandbox {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context) {
        // ctx.set_draw_target(target);
        // ctx.set_render_pipeline(pipeline);
        let position = v2(200.0, 100.0);
        let origin = v2(200.0, 100.0);
        let rotation = 0.0_f32.to_radians();
        let scale = v2(400.0, 200.0);
        let color = Color::YELLOW;
        ctx.draw_filled_triangle(position, rotation, scale, origin, color);
    }

    fn on_stop(&mut self, _ctx: &mut Context) {}

    fn on_exit(&mut self, ctx: &mut Context) {
        ctx.exit();
    }
}

fn main() {
    let Ok(app) = App::new(1920, 1080) else {
        return;
    };

    let Ok(sandbox) = Sandbox::new(&app) else {
        return;
    };

    if let Err(err) = app.run(sandbox) {
        eprintln!("Sandbox exited with error: {err}");
    }
}
