use crate::{
    error::Error,
    sys::{Event, Sys},
    Engine, Game, RendererId, Rhi,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;
    let sys = Sys::init()?;
    let window = sys.create_window(width, height)?;

    Rhi::get().init()?;
    // let render_thread = ;
    // let renderer = Renderer::new()?;
    // let render_proxy = renderer.create_render_proxy();
    let render_proxy = Rhi::get().get_render_proxy();
    let mut backbuffer = RendererId::INVALID;

    let mut age = Engine::new();
    let mut game = G::on_start(&mut age)?;

    sys.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut age),

            Event::PlatformReady => {
                backbuffer = render_proxy.create_backbuffer(window.clone());
                window.set_visible(true);
            }

            Event::Update => {
                game.on_update(&mut age);
                window.pre_present();

                window.post_present();
            }
        };

        if age.exit {
            platform.exit();
        }

        Ok(())
    })?;

    Rhi::get().deinit()?;

    Ok(())
}
