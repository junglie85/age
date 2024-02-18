pub use app::App;
pub use error::Error;
use sys::Window;

mod app;
mod error;
mod sys;

pub trait Game<T = Self> {
    fn on_start(ctx: &mut Ctx) -> Result<T, Error>;

    fn on_update(&mut self, ctx: &mut Ctx);
}

pub struct Ctx {
    exit_requested: bool,
    exit: bool,
    _window: Window,
}

impl Ctx {
    fn new(window: Window) -> Self {
        Self {
            exit_requested: false,
            exit: false,
            _window: window,
        }
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }
}
