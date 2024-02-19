use std::{any::TypeId, collections::HashMap};

use crate::{Ctx, Error};

pub trait Plugin {
    fn on_start(&mut self, ctx: &mut Ctx) -> Result<(), Error>;

    fn before_update(&mut self, ctx: &mut Ctx);

    fn after_update(&mut self, ctx: &mut Ctx);

    fn on_stop(&mut self, ctx: &mut Ctx);

    fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait CreatePlugin: Plugin {
    fn new() -> Self;
}

#[derive(Default)]
pub(crate) struct Plugins {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_index: HashMap<TypeId, usize>,
}

impl Plugins {
    pub(crate) fn push<P: CreatePlugin + 'static>(&mut self) {
        self.plugins.push(Box::new(P::new()));
        self.plugin_index
            .insert(TypeId::of::<P>(), self.plugins.len() - 1);
    }

    pub(crate) fn get_plugin<P: Plugin + 'static>(&self) -> &P {
        let index = self.plugin_index[&TypeId::of::<P>()];
        let plugin = self.plugins[index].as_ref();
        plugin
            .as_any()
            .downcast_ref()
            .expect("plugin is not registered")
    }

    pub(crate) fn get_plugin_mut<P: Plugin + 'static>(&mut self) -> &mut P {
        let index = self.plugin_index[&TypeId::of::<P>()];
        let plugin = self.plugins[index].as_mut();
        plugin
            .as_any_mut()
            .downcast_mut()
            .expect("plugin is not registered")
    }

    pub(crate) fn on_start(&mut self, ctx: &mut Ctx) -> Result<(), Error> {
        for plugin in self.plugins.iter_mut() {
            plugin.on_start(ctx)?;
        }

        Ok(())
    }

    pub(crate) fn before_update(&mut self, ctx: &mut Ctx) {
        for plugin in self.plugins.iter_mut() {
            plugin.before_update(ctx);
        }
    }

    pub(crate) fn after_update(&mut self, ctx: &mut Ctx) {
        for plugin in self.plugins.iter_mut() {
            plugin.after_update(ctx);
        }
    }

    pub(crate) fn on_stop(&mut self, ctx: &mut Ctx) {
        for plugin in self.plugins.iter_mut() {
            plugin.on_stop(ctx);
        }
    }
}
