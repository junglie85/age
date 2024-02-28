use std::process::ExitCode;

use age::{
    App, Color, CommandBuffer, Error, Game, PipelineLayoutDesc, RenderPipeline, RenderPipelineDesc,
    ShaderDesc, TextureFormat,
};

struct Sandbox {
    buf: CommandBuffer,
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

        Ok(Self {
            buf: CommandBuffer::new(),
            pipeline,
        })
    }

    fn on_update(&mut self, app: &mut App) {
        self.buf
            .begin_render_pass(app.get_backbuffer(), Some(Color::RED));

        self.buf.set_render_pipeline(&self.pipeline); // this will come from the sprite's material. could be a default pipeline based on the renderer/pass type?
        self.buf.draw(0..3, 0..1);

        app.gpu.submit(&self.buf);
        self.buf.recall();
    }
}

fn main() -> ExitCode {
    age::run::<Sandbox>()
}
