use std::{any::TypeId, collections::HashMap};

use crate::{sys::Window, Color};

pub struct Engine {
    exit: bool,
    started: bool,
    key: EngineKey,
    components: Vec<Box<dyn EngineComponent>>,
    components_lut: HashMap<TypeId, usize>,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self {
            started: false,
            exit: false,
            key: EngineKey(()),
            components: Vec::new(),
            components_lut: HashMap::new(),
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub(crate) fn get_component<T: EngineComponent + 'static>(&self) -> &T {
        let index = self.components_lut.get(&TypeId::of::<T>()).unwrap();
        let component = if let Some(b) = self.components.get(*index) {
            b.as_any().downcast_ref()
        } else {
            None
        };
        component.expect("component has not been registered")
    }

    pub(crate) fn get_component_mut<T: EngineComponent + 'static>(&mut self) -> &mut T {
        let index = self.components_lut.get(&TypeId::of::<T>()).unwrap();
        let component = if let Some(b) = self.components.get_mut(*index) {
            b.as_any_mut().downcast_mut()
        } else {
            None
        };
        component.expect("component has not been registered")
    }

    pub(crate) fn on_resume(&mut self, _key: EngineKey) {
        for i in 0..self.components.len() {
            let mut component = std::mem::replace(&mut self.components[i], Box::new(TempComponent));
            component.on_resume(self);
            self.components[i] = component;
        }
        // self.gpu.create_backbuffer(self.window.clone());

        let window = self.get_component::<Window>();
        window.set_visible(true);
    }

    pub(crate) fn on_start(&mut self) -> EngineKey {
        if self.started {
            panic!("engine is already started");
        }

        self.started = true;
        EngineKey(())
    }

    pub(crate) fn post_update(&mut self, key: EngineKey) {
        // let render_ctx = self.gpu.get_render_context();
        // let render_ctx = std::mem::replace(&mut self.render_ctx, render_ctx);

        // render_proxy.dispatch(render_ctx);

        let window = self.get_component::<Window>();
        window.pre_present();
        // self.gpu.present(self.backbuffer);
        window.post_present();
    }

    pub(crate) fn register_component<T: EngineComponent + 'static>(&mut self, component: T) {
        let key = TypeId::of::<T>();
        if self.components_lut.contains_key(&key) {
            return;
        }

        let index = self.components.len();
        self.components.push(Box::new(component));
        self.components_lut.insert(key, index);
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.exit
    }
}

// Struct is passed into app-only callable functions to prevent incorrect calls by other components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EngineKey(());

pub trait EngineComponent {
    fn on_resume(&mut self, engine: &mut Engine) {}

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

struct TempComponent;

impl EngineComponent for TempComponent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
