use age::{
    AgeResult, App, BindGroup, BindGroupInfo, Binding, Color, Context, Game, Texture, TextureFormat, TextureInfo,
    TextureView, TextureViewInfo,
};
use age_math::v2;

struct Sandbox {
    grid: Texture,
    grid_view: TextureView,
    grid_bg: BindGroup,
}

impl Sandbox {
    fn new(app: &App) -> AgeResult<Self> {
        let grid_data = [Color::RED, Color::GREEN, Color::BLUE, Color::YELLOW]
            .iter()
            .map(|c| c.to_array_u8())
            .flatten()
            .collect::<Vec<_>>();
        let grid = app.render_device().create_texture(&TextureInfo {
            label: Some("grid"),
            width: 2,
            height: 2,
            format: TextureFormat::Rgba8Unorm,
            ..Default::default()
        });
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
        app.render_device().write_texture(&grid, &grid_data);

        Ok(Self {
            grid,
            grid_view,
            grid_bg,
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
