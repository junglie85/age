use age::{
    map_screen_to_world, map_world_to_screen, AgeResult, App, Camera, CharSet, Color, Context, Game, KeyCode,
    KeyLocation, MouseButton, Rect, ScanCode, SpriteFont,
};
use age_math::{v2, Vec2};

struct Application {
    font: SpriteFont,
    camera: Camera,
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

        let camera = gfx.default_camera();
        let center = camera.center();
        let mut camera = Camera::new(center - v2(100.0, 100.0), camera.size());
        camera.set_viewport(Rect::new(v2(0.5, 0.0), v2(0.5, 1.0)));

        Ok(Self { font, camera })
    }
}

impl Game for Application {
    fn on_start(&mut self, ctx: &mut Context) {
        ctx.set_title("AGE - Input Example");
    }

    fn on_tick(&mut self, ctx: &mut Context) {
        let screen_pos = ctx.mouse_position();
        let world_pos = map_screen_to_world(screen_pos, &self.camera);
        let and_back = map_world_to_screen(world_pos, &self.camera);

        ctx.set_camera(&self.camera);
        ctx.clear(Color::rgb(1.0, 0.0, 1.0));

        ctx.draw_filled_circle(world_pos, 5.0, 30, 0.0, v2(5.0, 5.0), Color::rgb(0.0, 1.0, 1.0));

        let advance = v2(0.0, self.font.line_height());
        let font_size = 24.0;
        let color = Color::WHITE;
        let position = v2(5.0, 5.0);
        let text = format!("Screen pos: {:.2}, {:.2}", screen_pos.x, screen_pos.y);
        ctx.draw_string(&self.font, &text, font_size, color, position);

        let position = position + advance;
        let text = format!("World pos: {:.2}, {:.2}", world_pos.x, world_pos.y);
        ctx.draw_string(&self.font, &text, font_size, color, position);

        let position = position + advance;
        let text = format!("And back: {:.2}, {:.2}", and_back.x, and_back.y);
        ctx.draw_string(&self.font, &text, font_size, color, position);

        let position = position + advance;
        let text = "Mouse captured: no (todo)"; // todo: capture mouse.
        ctx.draw_string(&self.font, text, font_size, color, position);

        let released_color = Color::WHITE;
        let pressed_color = Color::rgb(1.0, 1.0, 0.0);

        let position = position + advance;
        let text = "Mouse button - left";
        let color = if ctx.mouse_button_pressed(MouseButton::Left) || ctx.mouse_button_held(MouseButton::Left) {
            pressed_color
        } else {
            released_color
        };
        ctx.draw_string(&self.font, text, font_size, color, position);

        let position = position + advance;
        let text = "Mouse button - middle";
        let color = if ctx.mouse_button_pressed(MouseButton::Middle) || ctx.mouse_button_held(MouseButton::Middle) {
            pressed_color
        } else {
            released_color
        };
        ctx.draw_string(&self.font, text, font_size, color, position);

        let position = position + advance;
        let text = "Mouse button - right";
        let color = if ctx.mouse_button_pressed(MouseButton::Right) || ctx.mouse_button_held(MouseButton::Right) {
            pressed_color
        } else {
            released_color
        };
        ctx.draw_string(&self.font, text, font_size, color, position);

        let position = position + advance;
        let text = "Mouse button - forward";
        let color = if ctx.mouse_button_pressed(MouseButton::Forward) || ctx.mouse_button_held(MouseButton::Forward) {
            pressed_color
        } else {
            released_color
        };
        ctx.draw_string(&self.font, text, font_size, color, position);

        let position = position + advance;
        let text = "Mouse button - back";
        let color = if ctx.mouse_button_pressed(MouseButton::Back) || ctx.mouse_button_held(MouseButton::Back) {
            pressed_color
        } else {
            released_color
        };
        ctx.draw_string(&self.font, text, font_size, color, position);

        if ctx.key_held((KeyCode::Alt, KeyLocation::Left)) {
            ctx.draw_filled_rect(v2(0.0, 0.0), 0.0, v2(100.0, 100.0), Vec2::ZERO, Color::GREEN);
        }
        if ctx.key_held(ScanCode::AltRight) {
            ctx.draw_filled_rect(v2(100.0, 0.0), 0.0, v2(100.0, 100.0), Vec2::ZERO, Color::RED);
        }

        // todo: lost focus, cancel key/button press.
        // todo: controller/gamepad input.
    }

    fn on_scale_factor_changed(&mut self, scale_factor: f32, _ctx: &mut Context) {
        self.camera.set_scale_factor(scale_factor);
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
