use std::sync::Arc;

use winit::{dpi::LogicalSize, event_loop::ControlFlow};

use crate::{error::Error, Engine};

impl Engine {
    pub fn exit(&mut self) {
        self.exit = true;
    }
}

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
            .with_visible(false)
            .build(self.el.as_ref().unwrap())?;
        Ok(Window { w: Arc::new(w) })
    }

    pub(crate) fn run<F>(mut self, mut handler: F) -> Result<(), Error>
    where
        F: FnMut(Event, &mut Platform) -> Result<(), Error>,
    {
        let el = self.el.take().unwrap();
        let mut platform = Platform::default();
        let mut result = Ok(());
        el.run(|e, el| {
            el.set_control_flow(ControlFlow::Poll);

            #[allow(clippy::collapsible_match)]
            let event = match e {
                winit::event::Event::WindowEvent { event: e, .. } => match e {
                    winit::event::WindowEvent::CloseRequested => Some(Event::ExitRequested),

                    winit::event::WindowEvent::RedrawRequested => Some(Event::Update),

                    _ => None,
                },

                winit::event::Event::Resumed => Some(Event::PlatformReady),

                _ => None,
            };

            if let Some(event) = event {
                result = handler(event, &mut platform);
                if platform.exit || result.is_err() {
                    el.exit();
                }
            }
        })?;

        result
    }
}

#[derive(Default)]
pub(crate) struct Platform {
    exit: bool,
}

impl Platform {
    pub(crate) fn exit(&mut self) {
        self.exit = true;
    }
}

#[derive(Clone)]
pub(crate) struct Window {
    w: Arc<winit::window::Window>,
}

impl Window {
    pub(crate) fn height(&self) -> u32 {
        self.w.inner_size().height
    }

    pub(crate) fn width(&self) -> u32 {
        self.w.inner_size().width
    }

    pub(crate) fn post_present(&self) {
        self.w.request_redraw();
    }

    pub(crate) fn pre_present(&self) {
        self.w.pre_present_notify();
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.w.set_visible(visible);
    }
}

impl raw_window_handle::HasDisplayHandle for Window {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.w.display_handle()
    }
}

impl raw_window_handle::HasWindowHandle for Window {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.w.window_handle()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    ExitRequested,
    PlatformReady,
    Update,
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
