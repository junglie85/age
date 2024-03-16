use age::{
    AgeResult, App, BindGroup, BindGroupInfo, Binding, Color, Context, Game, Image, Texture, TextureFormat,
    TextureInfo, TextureView, TextureViewInfo,
};
use age_math::v2;

struct Sandbox {
    grid: Texture,
    grid_view: TextureView,
    grid_bg: BindGroup,
    fighter: Texture,
    fighter_view: TextureView,
    fighter_bg: BindGroup,
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
            format: TextureFormat::Rgba8Unorm,
            ..Default::default()
        });
        app.render_device().write_texture(&fighter, fighter_img.pixels());
        let fighter_view = fighter.create_view(&TextureViewInfo { label: Some("fighter") }); // todo: add a default view to texture.
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

        Ok(Self {
            grid,
            grid_view,
            grid_bg,
            fighter,
            fighter_view,
            fighter_bg,
        })
    }
}

impl Game for Sandbox {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context) {
        // ctx.set_draw_target(target);
        // ctx.set_render_pipeline(pipeline);
        ctx.draw_rect(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::YELLOW);
        ctx.draw_rect_outline(v2(200.0, 100.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), 10.0, Color::BLACK);
        ctx.draw_rect(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), Color::RED);
        ctx.draw_rect_textured(v2(300.0, 150.0), 0.0, v2(400.0, 200.0), v2(200.0, 100.0), &self.grid_bg, Color::WHITE);
        ctx.draw_rect_textured(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32), // todo: impl into Vec2
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            &self.fighter_bg,
            Color::WHITE,
        );
        ctx.draw_rect_outline(
            v2(600.0, 600.0),
            30.0_f32.to_radians(),
            v2(self.fighter.size().0 as f32, self.fighter.size().1 as f32), // todo: impl into Vec2
            v2(self.fighter.size().0 as f32 / 2.0, self.fighter.size().1 as f32 / 2.0),
            2.0,
            Color::BLACK,
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
