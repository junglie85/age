use crate::{
    error::Error,
    renderer::{Gpu, Renderer},
    sys::{Event, EventLoop, Window},
    Backbuffer, Game, RenderTarget,
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
        gpu,
        backbuffer: Backbuffer::new(),
        renderer,
        exit: false,
    };

    let mut game = G::on_start(&mut app)?;

    el.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut app),

            Event::PlatformReady => {
                app.on_platform_ready(&window)?;
            }

            Event::Update => {
                game.on_update(&mut app);
                app.post_update(&window);
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

pub struct App<'app> {
    pub gpu: Gpu,
    pub backbuffer: Backbuffer<'app>,
    pub renderer: Renderer,
    exit: bool,
}

impl<'app> App<'app> {
    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn get_backbuffer(&mut self) -> RenderTarget {
        (&mut self.backbuffer).into()
    }

    fn post_update(&mut self, window: &Window) {
        // let render_ctx = self.gpu.get_render_context();
        // let render_ctx = std::mem::replace(&mut self.render_ctx, render_ctx);

        // render_proxy.dispatch(render_ctx);
        window.pre_present();
        self.backbuffer.present();
        window.post_present();
    }

    fn on_platform_ready(&mut self, window: &'app Window) -> Result<(), Error> {
        self.backbuffer.resume(&self.gpu, window)?;
        window.set_visible(true);

        Ok(())
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
