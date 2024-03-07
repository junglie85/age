use age::{AgeResult, App, Game};

struct Sandbox {}

impl Sandbox {
    fn new(_app: &App) -> AgeResult<Self> {
        Ok(Self {})
    }
}

impl Game for Sandbox {
    fn on_start(&mut self, _ctx: &mut age::Context) {}

    fn on_update(&mut self, _ctx: &mut age::Context) {}

    fn on_render(&mut self, ctx: &mut age::Context) {
        // ctx.set_draw_target(target);
        // ctx.set_render_pipeline(pipeline);
        ctx.draw_filled_triangle();
    }

    fn on_stop(&mut self, _ctx: &mut age::Context) {}

    fn on_exit(&mut self, ctx: &mut age::Context) {
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
