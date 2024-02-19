use age::{CreatePlugin, Ctx, Error, Plugin};

pub trait RendererCtx {
    fn do_thing(&self);
}

impl RendererCtx for Ctx {
    fn do_thing(&self) {
        let renderer = self.get_plugin::<Renderer>();
        renderer.msg("called renderer from the ctx");
        // let mut renderer = self.get_plugin_mut::<Renderer>();
    }
}

pub struct Renderer {}

impl Renderer {
    fn msg(&self, m: &str) {
        println!("{}", m)
    }
}

impl Plugin for Renderer {
    fn on_start(&mut self, _ctx: &mut Ctx) -> Result<(), Error> {
        println!("renderer on_start");
        Ok(())
    }

    fn before_update(&mut self, _ctx: &mut Ctx) {
        println!("renderer before_update");
    }

    fn after_update(&mut self, _ctx: &mut Ctx) {
        println!("renderer after_update");
    }

    fn on_stop(&mut self, _ctx: &mut Ctx) {
        println!("renderer on_stop");
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

impl CreatePlugin for Renderer {
    fn new() -> Self {
        Self {}
    }
}
