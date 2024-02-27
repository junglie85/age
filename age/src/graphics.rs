use crate::{app::Resource, renderer::Backbuffer, App, Color};

pub(crate) struct GraphicsContext {}

impl GraphicsContext {
    pub(crate) fn init() -> Self {
        Self {}
    }
}

impl Resource for GraphicsContext {
    fn on_resume(&mut self, app: &mut App) {
        println!("graphics on_resume")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait Graphics {
    fn clear(&mut self, color: Color);
    fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T);
}

impl Graphics for App {
    fn clear(&mut self, color: Color) {
        let _gfx = self.get_resource_mut::<GraphicsContext>();
    }

    fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T) {
        let _gfx = self.get_resource_mut::<GraphicsContext>();
    }
}

// todo: draw target can have multiple color attachments. we want to be able to convert the following into a target:
// - backbuffer
// - render texture
// - framebuffer / gbuffer (multiple render_textures), eventually - might take some rework elsewhere.
pub struct DrawTarget {
    color_targets: [(); Self::MAX_COLOR_TARGETS],
}

impl DrawTarget {
    const MAX_COLOR_TARGETS: usize = 4;
}

impl From<Backbuffer> for DrawTarget {
    fn from(backbuffer: Backbuffer) -> Self {
        todo!()
    }
}
