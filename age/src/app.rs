use std::process::ExitCode;

use crate::{
    error::Error,
    sys::{Event, Sys},
    Ctx, Game,
};

pub struct App;

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    pub fn run<G: Game>(self) -> ExitCode {
        match run::<G>() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::FAILURE
            }
        }
    }
}

fn run<G: Game>() -> Result<(), Error> {
    let sys = Sys::init()?;
    let window = sys.create_window(1920, 1080)?;

    let mut ctx = Ctx::new(window);
    let mut game = G::on_start(&mut ctx)?;

    sys.run(|event, sys| {
        match event {
            Event::ExitRequested => ctx.exit_requested = true,
        };

        game.on_update(&mut ctx);

        if ctx.exit {
            sys.exit();
        } else {
            ctx.exit_requested = false;
        }
    })?;

    Ok(())
}
