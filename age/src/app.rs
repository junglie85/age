use crate::{
    device::RenderDevice,
    error::Error,
    sys::{Event, EventLoop, Window},
    Game,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;

    let el = EventLoop::init()?;
    let window = Window::init(width, height, &el)?;
    let device = RenderDevice::init()?;

    let mut app = App {
        window,
        device,
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

    Ok(())
}

pub struct App {
    pub window: Window,
    pub device: RenderDevice,
    exit: bool,
}

impl App {
    pub fn exit(&mut self) {
        self.exit = true;
    }

    fn post_update(&mut self) -> Result<(), Error> {
        // sync
        self.device.prepare_window(&self.window)?;
        self.window.present();

        Ok(())
    }

    fn on_platform_ready(&mut self) {
        self.window.set_visible(true);
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
