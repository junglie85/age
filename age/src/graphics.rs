use crate::renderer::{
    DrawCommand, DrawTarget, PipelineLayoutInfo, RenderDevice, RenderPipeline, RenderPipelineInfo,
    ShaderInfo, TextureFormat,
};

pub struct Graphics {
    draw_state: DrawState,
    triangle_pipeline: RenderPipeline,
}

impl Graphics {
    pub(crate) fn new(device: &RenderDevice) -> Self {
        let shader = device.create_shader(&ShaderInfo {
            label: Some("triangle"),
            src: include_str!("shaders/triangle.wgsl"),
        });
        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("triangle"),
            bind_group_layouts: &[],
        });
        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("triangle"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Rgba8Unorm,
        });

        Self {
            draw_state: DrawState::default(),
            triangle_pipeline,
        }
    }

    pub(crate) fn begin_frame(&mut self, target: impl Into<DrawTarget>) {
        self.set_draw_target(target);
        self.set_render_pipeline(&self.triangle_pipeline.clone());
    }

    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.draw_state.target = Some(target.into());
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.draw_state.pipeline = Some(pipeline.clone());
    }

    pub fn draw_filled_triangle(&mut self, device: &mut RenderDevice) {
        let Some(target) = self.draw_state.target.as_ref() else {
            panic!("draw target is not set");
        };

        let Some(pipeline) = self.draw_state.pipeline.as_ref() else {
            panic!("render pipeline is not set");
        };

        device.push_draw_command(DrawCommand {
            target: target.clone(),
            // todo: this is pretty ugly, can we Default DrawCommand?
            bind_groups: [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS],
            pipeline: pipeline.clone(),
            vertices: 0..3,
        })
    }
}

#[derive(Default)]
struct DrawState {
    target: Option<DrawTarget>,
    pipeline: Option<RenderPipeline>,
}
