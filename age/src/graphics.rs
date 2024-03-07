use crate::renderer::{DrawCommand, DrawTarget, RenderDevice, RenderPipeline};

pub struct Graphics {
    state: GraphicState,
}

impl Graphics {
    pub(crate) fn new() -> Self {
        Self {
            state: GraphicState::default(),
        }
    }

    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.state.target = Some(target.into());
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.state.pipeline = Some(pipeline.clone());
    }

    pub fn draw_filled_triangle(&mut self, device: &mut RenderDevice) {
        let Some(target) = self.state.target.as_ref() else {
            panic!("draw target is not set");
        };

        let Some(pipeline) = self.state.pipeline.as_ref() else {
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
struct GraphicState {
    target: Option<DrawTarget>,
    pipeline: Option<RenderPipeline>,
}
