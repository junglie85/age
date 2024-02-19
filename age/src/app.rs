use crate::{
    error::Error,
    renderer::Renderer,
    sys::{Event, Sys},
    Engine, Game,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;
    let sys = Sys::init()?;
    let window = sys.create_window(width, height)?;
    let mut renderer = Renderer::new()?;
    let backbuffer = renderer.create_backbuffer(/* width, height */);

    let mut age = Engine::new(&backbuffer);
    let mut game = G::on_start(&mut age)?;

    sys.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut age),

            Event::PlatformReady => {
                renderer.attach_to_window(window.clone())?;
                window.set_visible(true);
            }

            Event::Update => {
                age.set_draw_target(&backbuffer);
                game.on_update(&mut age);
                renderer.submit(age.draws.clone(), &backbuffer);
                window.pre_present();
                renderer.present();
                window.post_present();
                age.draws.clear();
            }
        };

        if age.exit {
            platform.exit();
        }

        Ok(())
    })?;

    Ok(())
}
