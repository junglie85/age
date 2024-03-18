use age::{
    AgeResult, App, BindGroup, BindGroupInfo, Binding, Color, Context, Game, Image, Rect, Sprite, Texture,
    TextureFormat, TextureInfo, TextureView, TextureViewInfo,
};
use age_math::v2;
use glam::Vec2;

struct Sandbox {
    #[allow(dead_code)]
    grid: Texture,
    #[allow(dead_code)]
    grid_view: TextureView,
    grid_bg: BindGroup,
    fighter: Texture,
    #[allow(dead_code)]
    fighter_view: TextureView,
    fighter_bg: BindGroup,
    sprite: Sprite,
}

impl Sandbox {
    fn new(app: &App) -> AgeResult<Self> {
        let grid_data = [Color::RED, Color::GREEN, Color::BLUE, Color::YELLOW]
            .iter()
            .flat_map(|c| c.to_array_u8())
            .collect::<Vec<_>>();
        let grid = app.render_device().create_texture(&TextureInfo {
            label: Some("grid"),
            width: 2,
            height: 2,
            format: TextureFormat::Rgba8Unorm,
            ..Default::default()
        });
        app.render_device().write_texture(&grid, &grid_data);
        let grid_view = grid.create_view(&TextureViewInfo { label: Some("grid") });
        let grid_bg = app.render_device().create_bind_group(&BindGroupInfo {
            label: Some("grid"),
            layout: app.graphics().texture_bind_group_layout(),
            entries: &[
                Binding::Sampler {
                    sampler: app.graphics().default_sampler(),
                },
                Binding::Texture {
                    texture_view: &grid_view,
                },
            ],
        });

        let fighter_data = include_bytes!("space_fighter.png");
        let fighter_img = Image::from_bytes(fighter_data)?;
        let fighter = app.render_device().create_texture(&TextureInfo {
            label: Some("fighter"),
            width: fighter_img.width(),
            height: fighter_img.height(),
            format: TextureFormat::Rgba8UnormSrgb,
            ..Default::default()
        });
        app.render_device().write_texture(&fighter, fighter_img.pixels());
        let fighter_view = fighter.create_view(&TextureViewInfo { label: Some("fighter") });
        let fighter_bg = app.render_device().create_bind_group(&BindGroupInfo {
            label: Some("fighter"),
            layout: app.graphics().texture_bind_group_layout(),
            entries: &[
                Binding::Sampler {
                    sampler: app.graphics().default_sampler(),
                },
                Binding::Texture {
                    texture_view: &fighter_view,
                },
            ],
        });

        let escort_data = include_bytes!("escort.png");
        let escort_img = Image::from_bytes(escort_data)?;
        let escort = app.render_device().create_texture(&TextureInfo {
            label: Some("escort"),
            width: escort_img.width(),
            height: escort_img.height(),
            format: TextureFormat::Rgba8UnormSrgb,
            ..Default::default()
        });
        app.render_device().write_texture(&escort, escort_img.pixels());
        let mut sprite = app
            .graphics()
            .create_sprite(&escort, app.graphics().default_sampler(), app.render_device());
        sprite.set_origin(sprite.size() / 2.0);

        Ok(Self {
            grid,
            grid_view,
            grid_bg,
            fighter,
            fighter_view,
            fighter_bg,
            sprite,
        })
    }
}

impl Game for Sandbox {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context) {
        // ctx.set_draw_target(target);
        // ctx.set_render_pipeline(pipeline);
        ctx.clear(Color::BLUE);

        ctx.draw_box_filled(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::YELLOW);
        ctx.draw_box(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), 10.0, Color::BLACK);
        ctx.draw_box_filled(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::RED);
        ctx.draw_box_textured(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), &self.grid_bg);
        ctx.draw_box_textured(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32), // todo: impl into Vec2
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            &self.fighter_bg,
        );
        ctx.draw_box(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32),
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            2.0,
            Color::BLACK,
        );
        ctx.draw_box_textured_ext(
            v2(700.0, 700.0),
            0.0,
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32),
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            &self.fighter_bg,
            Rect::new(v2(0.5, 0.0), v2(0.5, 0.5)),
            Color::WHITE,
        );

        ctx.draw_line(v2(500.0, 250.0), v2(700.0, 700.0), v2(0.0, 2.5), 5.0, Color::RED);

        ctx.draw_circle_filled(v2(0.0, 400.0), 100.0, 30, 0.0, Vec2::ZERO, Color::YELLOW);
        ctx.draw_circle(v2(0.0, 400.0), 100.0, 30, 0.0, Vec2::ZERO, 10.0, Color::WHITE);

        ctx.draw_circle_filled(v2(400.0, 400.0), 50.0, 3, 0.0, Vec2::ZERO, Color::GREEN);
        let angle = 0.0_f32;
        let (sine, cosine) = angle.sin_cos();
        let position = v2(450.0, 450.0) + v2(50.0 * sine, 50.0 * cosine);
        ctx.draw_line(v2(450.0, 450.0), position, v2(0.0, 2.0), 2.0, Color::RED);
        ctx.draw_circle(v2(400.0, 400.0), 50.0, 3, 0.0, Vec2::ZERO, 5.0, Color::BLACK);

        ctx.draw_circle_textured(v2(0.0, 700.0), 100.0, 30, 0.0, Vec2::ZERO, &self.fighter_bg);
        ctx.draw_circle_textured_ext(
            v2(300.0, 700.0),
            100.0,
            30,
            0.0,
            Vec2::ZERO,
            &self.fighter_bg,
            Rect::new(v2(0.25, 0.25), v2(0.5, 0.5)),
            Color::RED,
        );

        ctx.draw_box_filled(v2(30.0, 500.0), 0.0, v2(100.0, 300.0), Vec2::ZERO, Color::rgba_u8(255, 0, 0, 100));

        ctx.draw_sprite(v2(600.0, 100.0), 0.0, Vec2::ONE, &self.sprite);
        ctx.draw_sprite_ext(
            v2(600.0, 100.0),
            0.0,
            Vec2::ONE,
            &self.sprite,
            Rect::new(v2(0.0, 0.0), v2(1.0, 0.5)),
            Color::GREEN,
        );
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
