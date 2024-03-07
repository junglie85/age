use winit::{
    dpi::LogicalSize,
    error::{EventLoopError, OsError},
    event::Event,
    event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{app::AppConfig, AgeError, AgeResult};

pub(crate) fn create_event_loop<T>() -> AgeResult<EventLoop<T>> {
    let el = EventLoopBuilder::with_user_event().build()?;

    Ok(el)
}

pub(crate) fn create_window<T>(config: &AppConfig, el: &EventLoop<T>) -> AgeResult<Window> {
    let size = LogicalSize::new(config.width, config.height);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_title(&config.title)
        .with_visible(false)
        .build(el)?;

    Ok(window)
}

pub(crate) fn run<F, T>(el: EventLoop<T>, mut handler: F) -> AgeResult
where
    F: FnMut(Event<T>, &EventLoopWindowTarget<T>) -> AgeResult,
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
