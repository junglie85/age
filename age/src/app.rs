use crate::{
    error::Error,
    render_thread_main,
    sys::{Event, Sys},
    BackbufferInfo, Engine, Game, Gpu, Renderer,
};

pub(crate) fn run<G: Game>() -> Result<(), Error> {
    let width = 1920;
    let height = 1080;
    let sys = Sys::init()?;
    let window = sys.create_window(width, height)?;

    let gpu = Gpu::new()?;
    let renderer = Renderer::new(&gpu);
    let render_proxy = renderer.create_render_proxy();
    let render_thread = std::thread::Builder::new()
        .name("render thread".to_string())
        .spawn(|| {
            if let Err(err) = render_thread_main(renderer) {
                panic!("render thread error: {err}");
            }
        })?;

    let backbuffer = gpu.create_backbuffer(&BackbufferInfo { window: &window });

    let mut age = Engine::new(window, backbuffer, gpu);
    let mut game = G::on_start(&mut age)?;

    sys.run(|event, platform| {
        match event {
            Event::ExitRequested => game.on_exit_requested(&mut age),

            Event::PlatformReady => {
                age.on_resume();
            }

            Event::Update => {
                game.on_update(&mut age);
                age.update(&render_proxy);
            }
        };

        if age.exit {
            platform.exit();
        }

        Ok(())
    })?;

    render_proxy.stop_render_thread();
    render_thread
        .join()
        .map_err(|err| Error::new(format!("{:?}", err)))?;

    Ok(())
}
