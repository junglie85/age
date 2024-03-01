use crate::{
    error::Error,
    os::{Event, EventLoop, Window},
    renderer::{start_render_thread, RenderDevice, RenderInterface, RenderProxy},
    Game,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;

    let el = EventLoop::init()?;
    let window = Window::init(width, height, &el)?;
    let device = RenderDevice::init()?;
    let interface = RenderInterface::init();

    let (render_thread, render_proxy) =
        start_render_thread(window.clone(), device.clone(), interface.clone())?;

    let mut app = App {
        window,
        device,
        interface,
        proxy: render_proxy.clone(),
        exit: false,
    };

    let mut game = G::on_start(&mut app)?;

    el.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut app),

            Event::PlatformReady => {
                app.on_platform_ready();
            }

            Event::Update => {
                game.on_update(&mut app);
                app.post_update()?;
            }
        };

        if app.should_exit() {
            platform.exit();
        }

        Ok(())
    })?;

    render_proxy.shutdown(render_thread);

    Ok(())
}

pub struct App {
    pub window: Window,
    pub device: RenderDevice,
    pub interface: RenderInterface,
    pub proxy: RenderProxy,
    exit: bool,
}

impl App {
    pub fn exit(&mut self) {
        self.exit = true;
    }

    fn post_update(&mut self) -> Result<(), Error> {
        self.proxy.sync();
        self.proxy.execute();

        Ok(())
    }

    fn on_platform_ready(&mut self) {
        self.window.set_visible(true);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
