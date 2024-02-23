use crate::{
    error::Error,
    graphics::Graphics,
    renderer::{Renderer, Surface},
    sys::{Event, Sys},
    Engine, Game,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;
    let sys = Sys::init()?;
    let window = sys.create_window(width, height)?;
    let mut renderer = Renderer::new()?;
    let mut surface = Surface::default();
    let backbuffer = renderer.create_backbuffer(/* width, height */);
    let graphics = Graphics::new(&mut renderer);

    let mut age = Engine::new(renderer, graphics);
    let mut game = G::on_start(&mut age)?;

    sys.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut age),

            Event::PlatformReady => {
                surface.init(&age.renderer, &window)?;
                window.set_visible(true);
            }

            Event::Update => {
                age.graphics.set_draw_target(&backbuffer);
                game.on_update(&mut age);
                age.renderer
                    .submit(age.graphics.draws().clone(), &backbuffer, &mut surface);
                window.pre_present();
                surface.present();
                window.post_present();
                age.graphics.draws_mut().clear();
            }
        };

        if age.exit {
            platform.exit();
        }

        Ok(())
    })?;

    Ok(())
}
