use std::{any::TypeId, collections::HashMap};

use crate::{
    error::Error,
    renderer::{WgpuGpu, WgpuRenderer},
    sys::{Event, EventLoop, WinitWindow},
    Game, GraphicsContext,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;

    let el = EventLoop::init()?;
    let window = WinitWindow::init(width, height, &el)?;
    let gpu = WgpuGpu::init()?;
    let renderer = WgpuRenderer::init(&gpu);
    // let render_thread = std::thread::Builder::new()
    //     .name("render thread".to_string())
    //     .spawn(|| {
    //         if let Err(err) = render_thread_main(renderer) {
    //             panic!("render thread error: {err}");
    //         }
    //     })?;
    let graphics = GraphicsContext::init();

    let mut app = App::new();
    app.register_component(window);
    app.register_component(gpu);
    app.register_component(renderer);
    app.register_component(graphics);

    let mut game = G::on_start(&mut app)?;

    el.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut app),

            Event::PlatformReady => {
                app.on_resume();
            }

            Event::Update => {
                game.on_update(&mut app);
                app.post_update();
            }
        };

        if app.should_exit() {
            platform.exit();
        }

        Ok(())
    })?;

    // render_proxy.stop_render_thread();
    // render_thread
    //     .join()
    //     .map_err(|err| Error::new(format!("{:?}", err)))?;

    Ok(())
}

pub struct App {
    exit: bool,
    components: Vec<Box<dyn Resource>>,
    components_lut: HashMap<TypeId, usize>,
}

impl App {
    fn new() -> Self {
        Self {
            exit: false,
            components: Vec::new(),
            components_lut: HashMap::new(),
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub(crate) fn get_resource<T: Resource + 'static>(&self) -> &T {
        let index = self.components_lut.get(&TypeId::of::<T>()).unwrap();
        let resource = if let Some(b) = self.components.get(*index) {
            b.as_any().downcast_ref()
        } else {
            None
        };
        resource.expect("resource has not been registered")
    }

    pub(crate) fn get_resource_mut<T: Resource + 'static>(&mut self) -> &mut T {
        let index = self.components_lut.get(&TypeId::of::<T>()).unwrap();
        let resource = if let Some(b) = self.components.get_mut(*index) {
            b.as_any_mut().downcast_mut()
        } else {
            None
        };
        resource.expect("resource has not been registered")
    }

    fn on_resume(&mut self) {
        for i in 0..self.components.len() {
            let mut resource = std::mem::replace(&mut self.components[i], Box::new(TempResource));
            resource.on_resume(self);
            self.components[i] = resource;
        }
        // self.gpu.create_backbuffer(self.window.clone());

        let window = self.get_resource::<WinitWindow>();
        window.set_visible(true);
    }

    fn post_update(&mut self) {
        // let render_ctx = self.gpu.get_render_context();
        // let render_ctx = std::mem::replace(&mut self.render_ctx, render_ctx);

        // render_proxy.dispatch(render_ctx);

        let window = self.get_resource::<WinitWindow>();
        window.pre_present();
        // self.gpu.present(self.backbuffer);
        window.post_present();
    }

    fn register_component<T: Resource + 'static>(&mut self, component: T) {
        let key = TypeId::of::<T>();
        if self.components_lut.contains_key(&key) {
            return;
        }

        let index = self.components.len();
        self.components.push(Box::new(component));
        self.components_lut.insert(key, index);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}

pub trait Resource {
    fn on_resume(&mut self, app: &mut App) {}

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

struct TempResource;

impl Resource for TempResource {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
