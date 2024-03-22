use age::{AgeResult, App, CharSet, Color, Context, Game, SpriteFont};
use age_math::v2;

struct Application {
    font: SpriteFont,
}

impl Application {
    fn new(ctx: &Context) -> AgeResult<Self> {
        let device = ctx.render_device();
        let gfx = ctx.graphics();

        let font = gfx.default_font().load_charset(
            32.0,
            CharSet::ASCII,
            gfx.texture_bind_group_layout(),
            gfx.default_sampler(),
            device,
        )?;

        Ok(Self { font })
    }
}

impl Game for Application {
    fn on_start(&mut self, ctx: &mut Context) {
        ctx.set_title("AGE - Input Example");
    }

    fn on_tick(&mut self, ctx: &mut Context) {
        let screen_pos = ctx.screen_position();

        ctx.clear(Color::rgb(1.0, 0.0, 1.0));

        let font_size = 24.0;
        let color = Color::WHITE;
        let position = v2(5.0, 5.0);
        let text = format!("Screen pos: {:.2}, {:.2}", screen_pos.0, screen_pos.1);
        ctx.draw_string(&self.font, &text, font_size, color, position);
    }
}

fn main() {
    let Ok(app) = App::new(1920, 1080) else {
        return;
    };

    let Ok(application) = Application::new(app.context()) else {
        return;
    };

    if let Err(err) = app.run(application) {
        eprintln!("Application exited with error: {err}");
    }
}
