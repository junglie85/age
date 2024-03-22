use age::{
    AgeResult, App, BindGroup, BindGroupInfo, Binding, Camera, CharSet, Color, Context, Font, Game, Image, Rect,
    Sprite, SpriteFont, Texture, TextureFormat, TextureInfo, TextureView, TextureViewInfo,
};
use age_math::{v2, Mat4, Vec2};

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
    sprite_font: SpriteFont,
    draw_target: Texture,
    #[allow(dead_code)]
    draw_target_view: TextureView,
    draw_target_bg: BindGroup,
    camera: Camera,
}

impl Sandbox {
    fn new(ctx: &Context) -> AgeResult<Self> {
        let grid_data = [Color::RED, Color::GREEN, Color::BLUE, Color::YELLOW]
            .iter()
            .flat_map(|c| c.to_array_u8())
            .collect::<Vec<_>>();
        let grid = ctx.render_device().create_texture(&TextureInfo {
            label: Some("grid"),
            width: 2,
            height: 2,
            format: TextureFormat::Rgba8Unorm,
            ..Default::default()
        });
        ctx.render_device().write_texture(&grid, &grid_data);
        let grid_view = grid.create_view(&TextureViewInfo { label: Some("grid") });
        let grid_bg = ctx.render_device().create_bind_group(&BindGroupInfo {
            label: Some("grid"),
            layout: ctx.graphics().texture_bind_group_layout(),
            entries: &[
                Binding::Sampler {
                    sampler: ctx.graphics().default_sampler(),
                },
                Binding::Texture {
                    texture_view: &grid_view,
                },
            ],
        });

        let fighter_data = include_bytes!("space_fighter.png");
        let fighter_img = Image::from_bytes(fighter_data)?;
        let fighter = ctx.render_device().create_texture(&TextureInfo {
            label: Some("fighter"),
            width: fighter_img.width(),
            height: fighter_img.height(),
            format: TextureFormat::Rgba8UnormSrgb,
            ..Default::default()
        });
        ctx.render_device().write_texture(&fighter, fighter_img.pixels());
        let fighter_view = fighter.create_view(&TextureViewInfo { label: Some("fighter") });
        let fighter_bg = ctx.render_device().create_bind_group(&BindGroupInfo {
            label: Some("fighter"),
            layout: ctx.graphics().texture_bind_group_layout(),
            entries: &[
                Binding::Sampler {
                    sampler: ctx.graphics().default_sampler(),
                },
                Binding::Texture {
                    texture_view: &fighter_view,
                },
            ],
        });

        let escort_data = include_bytes!("escort.png");
        let escort_img = Image::from_bytes(escort_data)?;
        let mut sprite = ctx.create_sprite_from_image(&escort_img, Some("escort"));
        sprite.set_origin(sprite.size() / 2.0);

        let font_data = include_bytes!("OpenSans-Regular.ttf");
        let font = Font::from_bytes(font_data)?;
        let sprite_font = font.load_charset(
            36.0,
            CharSet::ASCII,
            ctx.graphics().texture_bind_group_layout(),
            ctx.graphics().default_sampler(),
            ctx.render_device(),
        )?;

        let draw_target = ctx.render_device().create_texture(&TextureInfo {
            label: Some("draw target"),
            width: ctx.config().width,
            height: ctx.config().height,
            format: TextureFormat::Rgba8Unorm,
            renderable: true,
            ..Default::default()
        });
        let draw_target_view = draw_target.create_view(&TextureViewInfo {
            label: Some("draw target"),
        });
        let draw_target_bg = ctx.render_device().create_bind_group(&BindGroupInfo {
            label: Some("draw target"),
            layout: ctx.graphics().texture_bind_group_layout(),
            entries: &[
                Binding::Sampler {
                    sampler: ctx.graphics().default_sampler(),
                },
                Binding::Texture {
                    texture_view: &draw_target_view,
                },
            ],
        });

        let camera = ctx.create_camera(0.0, ctx.config().width as f32, ctx.config().height as f32, 0.0);

        Ok(Self {
            grid,
            grid_view,
            grid_bg,
            fighter,
            fighter_view,
            fighter_bg,
            sprite,
            sprite_font,
            draw_target,
            draw_target_view,
            draw_target_bg,
            camera,
        })
    }
}

impl Game for Sandbox {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context) {
        ctx.set_draw_target(&self.draw_target);
        ctx.set_camera(&self.camera);
        ctx.clear(Color::BLUE);

        ctx.push_matrix(Mat4::translation(v2(500.0, 100.0)));
        ctx.draw_filled_rect(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::YELLOW);
        ctx.draw_rect(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), 10.0, Color::BLACK);
        ctx.draw_filled_rect(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::RED);
        ctx.draw_textured_rect(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), &self.grid_bg);
        ctx.draw_textured_rect(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32), // todo: impl into Vec2
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            &self.fighter_bg,
        );
        ctx.draw_rect(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32),
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            2.0,
            Color::BLACK,
        );
        ctx.draw_textured_rect_ext(
            v2(700.0, 700.0),
            0.0,
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32),
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            &self.fighter_bg,
            Rect::new(v2(0.5, 0.0), v2(0.5, 0.5)),
            Color::WHITE,
        );

        ctx.draw_line(v2(500.0, 250.0), v2(700.0, 700.0), 5.0, Color::RED);
        ctx.draw_line_from(v2(700.0, 700.0), 0.0, 150.0, 5.0, Color::GREEN);

        ctx.draw_filled_circle(v2(0.0, 400.0), 100.0, 30, 0.0, Vec2::ZERO, Color::YELLOW);
        ctx.draw_circle(v2(0.0, 400.0), 100.0, 30, 0.0, Vec2::ZERO, 10.0, Color::WHITE);

        ctx.draw_filled_circle(v2(400.0, 400.0), 50.0, 3, 0.0, Vec2::ZERO, Color::GREEN);
        let angle = 0.0_f32;
        let (sine, cosine) = angle.sin_cos();
        let position = v2(450.0, 450.0) + v2(50.0 * sine, 50.0 * cosine);
        ctx.draw_line_ext(v2(450.0, 450.0), position, v2(0.0, 2.0), 2.0, Color::RED);
        ctx.draw_circle(v2(400.0, 400.0), 50.0, 3, 0.0, Vec2::ZERO, 5.0, Color::BLACK);

        ctx.draw_textured_circle(v2(0.0, 700.0), 100.0, 30, 0.0, Vec2::ZERO, &self.fighter_bg);
        ctx.draw_textured_circle_ext(
            v2(300.0, 700.0),
            100.0,
            30,
            0.0,
            Vec2::ZERO,
            &self.fighter_bg,
            Rect::new(v2(0.25, 0.25), v2(0.5, 0.5)),
            Color::RED,
        );

        ctx.draw_filled_rect(v2(30.0, 500.0), 0.0, v2(100.0, 300.0), Vec2::ZERO, Color::rgba_u8(255, 0, 0, 100));

        ctx.draw_sprite(&self.sprite, v2(600.0, 100.0), 0.0);
        ctx.draw_sprite_ext(
            &self.sprite,
            Rect::new(v2(0.0, 0.0), v2(1.0, 0.5)),
            Color::GREEN,
            v2(600.0, 100.0),
            0.0,
            Vec2::ONE,
        );

        ctx.draw_string(&self.sprite_font, "Ashley's Game Engine", 36.0, Color::WHITE, v2(800.0, 300.0));
        ctx.draw_string_ext(&self.sprite_font, "Sandbox", 36.0, Color::WHITE, v2(800.0, 340.0), Vec2::ZERO);

        ctx.pop_matrix();

        let (window_width, window_height) = ctx.window_size();

        let w = window_width as f32 / self.draw_target.width() as f32;
        let h = window_height as f32 / self.draw_target.height() as f32;
        let scale = w.min(h);

        let window_center = v2(window_width as f32, window_height as f32) / 2.0;
        let target_center = v2(self.draw_target.width() as f32, self.draw_target.height() as f32) / 2.0;
        let origin = target_center * scale;

        let matrix = Mat4::translation(window_center - origin) * Mat4::scale(Vec2::splat(scale));

        ctx.set_draw_target(ctx.window_target());
        ctx.set_camera(&ctx.graphics().default_camera().clone());
        ctx.clear(Color::GREEN);
        ctx.push_matrix(matrix);
        ctx.draw_textured_rect(
            Vec2::ZERO,
            0.0,
            v2(self.draw_target.width() as f32, self.draw_target.height() as f32),
            Vec2::ZERO,
            &self.draw_target_bg,
        );
        ctx.pop_matrix();
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

    let Ok(sandbox) = Sandbox::new(app.context()) else {
        return;
    };

    if let Err(err) = app.run(sandbox) {
        eprintln!("Sandbox exited with error: {err}");
    }
}
