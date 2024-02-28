use crate::{
    device::RenderDevice,
    error::Error,
    sys::{Event, EventLoop, Window},
    Backbuffer, Game, RenderTarget,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;

    let el = EventLoop::init()?;
    let window = Window::init(width, height, &el)?;
    let device = RenderDevice::init()?;

    let mut app = App {
        device,
        backbuffer: Backbuffer::new(),
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

    Ok(())
}

pub struct App<'app> {
    pub device: RenderDevice,
    pub backbuffer: Backbuffer<'app>,
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
        window.pre_present();
        self.backbuffer.present();
        window.post_present();
    }

    fn on_platform_ready(&mut self, window: &'app Window) -> Result<(), Error> {
        self.backbuffer.resume(&self.device, window)?;
        window.set_visible(true);

        Ok(())
    }

    fn should_exit(&self) -> bool {
        self.exit
    }
}
