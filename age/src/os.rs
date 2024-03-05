use winit::{
    dpi::LogicalSize,
    error::{EventLoopError, OsError},
    event::Event,
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{app::AppConfig, AgeError, AgeResult};

pub(crate) fn create_event_loop() -> AgeResult<EventLoop<()>> {
    let el = EventLoop::new()?;

    Ok(el)
}

pub(crate) fn create_window(config: &AppConfig, el: &EventLoop<()>) -> AgeResult<Window> {
    let size = LogicalSize::new(config.width, config.height);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_title(&config.title)
        .with_visible(false)
        .build(el)?;

    Ok(window)
}

pub(crate) fn run<F>(el: EventLoop<()>, mut handler: F) -> AgeResult
where
    F: FnMut(Event<()>, &EventLoopWindowTarget<()>) -> AgeResult,
{
    let mut result = Ok(());
    el.run(|event, elwt| {
        result = handler(event, elwt);
        if result.is_err() {
            elwt.exit();
        }
    })?;

    result
}

impl From<EventLoopError> for AgeError {
    fn from(err: EventLoopError) -> Self {
        AgeError::new("failed to create event loop").with_source(err)
    }
}

impl From<OsError> for AgeError {
    fn from(err: OsError) -> Self {
        AgeError::new("failed to perform os action").with_source(err)
    }
}
