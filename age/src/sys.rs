use winit::{dpi::LogicalSize, event_loop::ControlFlow};

use crate::error::Error;

pub(crate) struct Sys {
    el: Option<winit::event_loop::EventLoop<()>>,
}

impl Sys {
    pub(crate) fn init() -> Result<Self, Error> {
        let el = Some(winit::event_loop::EventLoopBuilder::new().build()?);
        Ok(Self { el })
    }

    pub(crate) fn create_window(&self, width: u32, height: u32) -> Result<Window, Error> {
        let size = LogicalSize::new(width, height);
        let w = winit::window::WindowBuilder::new()
            .with_title("age")
            .with_inner_size(size)
            .build(self.el.as_ref().unwrap())?;
        Ok(Window { _w: w })
    }

    pub(crate) fn run<F>(mut self, mut handler: F) -> Result<(), Error>
    where
        F: FnMut(Event, &mut SysCtx),
    {
        let el = self.el.take().unwrap();
        let mut ctx = SysCtx::default();
        el.run(|e, el| {
            el.set_control_flow(ControlFlow::Poll);

            #[allow(clippy::collapsible_match)]
            let event = match e {
                winit::event::Event::WindowEvent { event: e, .. } => match e {
                    winit::event::WindowEvent::CloseRequested => Some(Event::ExitRequested),

                    _ => None,
                },

                _ => None,
            };

            if let Some(event) = event {
                handler(event, &mut ctx);
                if ctx.exit {
                    el.exit();
                }
            }
        })?;

        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct SysCtx {
    exit: bool,
}

impl SysCtx {
    pub(crate) fn exit(&mut self) {
        self.exit = true;
    }
}

pub(crate) struct Window {
    _w: winit::window::Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    ExitRequested,
}

impl From<winit::error::EventLoopError> for Error {
    fn from(value: winit::error::EventLoopError) -> Self {
        Error::new("failed to create event loop").with_source(value)
    }
}

impl From<winit::error::OsError> for Error {
    fn from(value: winit::error::OsError) -> Self {
        Error::new("failed to complete the requested operation").with_source(value)
    }
}
