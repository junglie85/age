use std::process::ExitCode;

pub use color::*;
pub use error::Error;
pub use graphics::Sprite;
use renderer::{CommandBuffer, DrawTarget};

mod app;
mod color;
mod error;
mod graphics;
mod renderer;
mod sys;

pub fn run<G: Game>() -> ExitCode {
    match app::run::<G>() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

pub trait Game<T = Self> {
    fn on_start(age: &mut Engine) -> Result<T, Error>;

    fn on_update(&mut self, age: &mut Engine);

    fn on_exit_requested(&mut self, age: &mut Engine) {
        age.exit();
    }
}

pub struct Engine {
    exit: bool,

    // Graphics
    draw_target: DrawTarget,
    clear_color: Option<Color>,
    needs_render_pass: bool,
    draws: CommandBuffer,
}

impl Engine {
    fn new<T: Into<DrawTarget>>(draw_target: T) -> Self {
        Self {
            exit: false,

            // Graphics.
            draw_target: draw_target.into(),
            clear_color: None,
            needs_render_pass: true,
            draws: CommandBuffer::default(),
        }
    }
}

// ----- Graphics -----
impl Engine {
    pub fn clear(&mut self, color: Color) {
        self.clear_color = Some(color);
        self.needs_render_pass = true;
        self.push_draw_command();
    }

    pub fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T) {
        self.draw_target = target.into();
        self.clear_color = None;
        self.needs_render_pass = true;
    }

    fn push_draw_command(&mut self) {
        if self.needs_render_pass {
            self.needs_render_pass = false;
            self.draws
                .set_render_pass(self.draw_target.texture(), self.clear_color);
        }

        // self.draws.push(DrawCommand {
        //     target: self.draw_target.texture().clone(),
        //     clear_color: self.clear_color,
        // });
    }
}

// ----- Platform -----
impl Engine {
    pub fn exit(&mut self) {
        self.exit = true;
    }
}
