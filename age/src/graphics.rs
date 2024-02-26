use crate::{engine::EngineComponent, Color, Engine};

pub(crate) struct GraphicsComponent {}

impl GraphicsComponent {
    pub(crate) fn init() -> Self {
        Self {}
    }
}

impl EngineComponent for GraphicsComponent {
    fn on_resume(&mut self, engine: &mut Engine) {
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
}

impl Graphics for Engine {
    fn clear(&mut self, color: Color) {
        let _gfx = self.get_component_mut::<GraphicsComponent>();
    }
}
