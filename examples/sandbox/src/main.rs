use std::process::ExitCode;

use age::{
    App, Color, Error, Game, PipelineLayoutDesc, RenderPipeline, RenderPipelineDesc, ShaderDesc,
    TextureFormat,
};

struct Sandbox {
    pipeline: RenderPipeline,
}

impl Game for Sandbox {
    fn on_start(app: &mut App) -> Result<Self, Error> {
        let layout = app.gpu.create_pipeline_layout(&PipelineLayoutDesc {
            label: Some("sprite forward"),
        });
        let shader = app.gpu.create_shader(&ShaderDesc {
            label: Some("sprite forward"),
            src: include_str!("sprite.wgsl"),
        });
        let pipeline = app.gpu.create_render_pipelne(&RenderPipelineDesc {
            label: Some("sprite forward"),
            layout: &layout,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Bgra8Unorm,
        });

        Ok(Self { pipeline })
    }

    fn on_update(&mut self, app: &mut App) {
        let mut buf = app.gpu.get_command_buffer();
        buf.begin_render_pass(app.get_backbuffer(), Some(Color::RED));
        buf.set_render_pipeline(&self.pipeline); // this will come from the sprite's material. could be a default pipeline based on the renderer/pass type?
        buf.draw(0..3, 0..1);

        app.gpu.submit(buf);
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
