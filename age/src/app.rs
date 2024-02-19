use std::{any::TypeId, collections::HashMap, process::ExitCode};

use crate::{
    error::Error,
    plugin::{CreatePlugin, Plugins},
    sys::{Event, Sys},
    Ctx, Game, Plugin,
};

pub struct App {
    plugins: Plugins,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            plugins: Plugins::default(),
        }
    }

    pub fn run<G: Game>(self) -> ExitCode {
        match run::<G>(self.plugins) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::FAILURE
            }
        }
    }

    pub fn with_plugin<P: CreatePlugin + 'static>(mut self) -> Self {
        self.plugins.push::<P>();
        self
    }
}

fn run<G: Game>(mut plugins: Plugins) -> Result<(), Error> {
    let sys = Sys::init()?;
    let window = sys.create_window(1920, 1080)?;

    // We create temporary plugins then swap it with the real plugins after on_start has been called
    // because we cannot have more than one exclusive borrow on ctx. This means that get_plugin and
    // get_plugin_mut cannot be called during on_start.
    let temp_plugins = Plugins::default();
    let mut ctx = Ctx::new(window, temp_plugins);
    plugins.on_start(&mut ctx)?;
    std::mem::swap(&mut plugins, &mut ctx.plugins);

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
