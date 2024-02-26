use crate::{
    error::Error,
    renderer::{Gpu, Renderer},
    sys::{Event, Sys},
    Engine, Game, GraphicsComponent,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;
    let sys = Sys::init()?;
    let window = sys.create_window(width, height)?;

    let gpu = Gpu::init()?;
    let renderer = Renderer::init();
    // let render_thread = std::thread::Builder::new()
    //     .name("render thread".to_string())
    //     .spawn(|| {
    //         if let Err(err) = render_thread_main(renderer) {
    //             panic!("render thread error: {err}");
    //         }
    //     })?;
    let graphics = GraphicsComponent::init();

    let mut age = Engine::new();
    age.register_component(window);
    age.register_component(gpu);
    age.register_component(renderer);
    age.register_component(graphics);
    let key = age.on_start();

    let mut game = G::on_start(&mut age)?;

    sys.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut age),

            Event::PlatformReady => {
                age.on_resume(key);
            }

            Event::Update => {
                game.on_update(&mut age);
                age.post_update(key);
            }
        };

        if age.should_exit() {
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
