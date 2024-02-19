pub use app::App;
pub use error::Error;
use plugin::Plugins;
pub use plugin::{CreatePlugin, Plugin};
use sys::Window;

mod app;
mod error;
mod plugin;
mod sys;

pub trait Game<T = Self> {
    fn on_start(ctx: &mut Ctx) -> Result<T, Error>;

    fn on_update(&mut self, ctx: &mut Ctx);
}

pub struct Ctx {
    exit_requested: bool,
    exit: bool,
    _window: Window,
    plugins: Plugins,
}

impl Ctx {
    fn new(window: Window, plugins: Plugins) -> Self {
        Self {
            exit_requested: false,
            exit: false,
            _window: window,
            plugins,
        }
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn get_plugin<P: Plugin + 'static>(&self) -> &P {
        self.plugins.get_plugin::<P>()
    }

    pub fn get_plugin_mut<P: Plugin + 'static>(&mut self) -> &mut P {
        self.plugins.get_plugin_mut::<P>()
    }

    pub(crate) fn start_plugins(&mut self) {
        self.plugins.on_start(self);
    }
}
