use crate::{
    error::Error,
    renderer::{Gpu, Renderer},
    sys::{Event, EventLoop, Window},
    Game,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;

    let el = EventLoop::init()?;
    let window = Window::init(width, height, &el)?;
    let gpu = Gpu::init()?;
    let renderer = Renderer::init(&gpu);
    // let render_thread = std::thread::Builder::new()
    //     .name("render thread".to_string())
    //     .spawn(|| {
    //         if let Err(err) = render_thread_main(renderer) {
    //             panic!("render thread error: {err}");
    //         }
    //     })?;

    let mut app = App {
        window,
        gpu,
        renderer,
        exit: false,
    };

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
    pub window: Window,
    pub gpu: Gpu,
    pub renderer: Renderer,
    exit: bool,
}

impl App {
    pub fn exit(&mut self) {
        self.exit = true;
    }

    fn on_resume(&mut self) {
        // self.gpu.create_backbuffer(self.window.clone());
        self.window.set_visible(true);
    }

    fn post_update(&mut self) {
        // let render_ctx = self.gpu.get_render_context();
        // let render_ctx = std::mem::replace(&mut self.render_ctx, render_ctx);

        // render_proxy.dispatch(render_ctx);

        self.window.pre_present();
        // self.gpu.present(self.backbuffer);
        self.window.post_present();
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
