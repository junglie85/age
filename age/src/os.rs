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

#[derive(Debug, Default)]
pub struct Mouse {}

impl Mouse {
    pub fn position(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    pub fn position_delta(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    pub fn button_pressed(&self, button: MouseButton) -> bool {
        false
    }

    pub fn button_released(&self, button: MouseButton) -> bool {
        false
    }

    pub fn button_held(&self, button: MouseButton) -> bool {
        false
    }

    pub(crate) fn on_event(&mut self, event: &winit::event::WindowEvent) {}

    pub(crate) fn flush(&mut self) {}
}

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
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

#[cfg(test)]
mod test {
    use super::*;
    use age_math::v2;
    use winit::{dpi::PhysicalPosition, event::DeviceId};

    #[test]
    fn mouse_defaults() {
        let m = Mouse::default();

        assert_eq!((0.0, 0.0), m.position());
        assert_eq!((0.0, 0.0), m.position_delta());

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));

        assert!(!m.button_pressed(MouseButton::Right));
        assert!(!m.button_released(MouseButton::Right));
        assert!(!m.button_held(MouseButton::Right));

        assert!(!m.button_pressed(MouseButton::Middle));
        assert!(!m.button_released(MouseButton::Middle));
        assert!(!m.button_held(MouseButton::Middle));

        assert!(!m.button_pressed(MouseButton::Back));
        assert!(!m.button_released(MouseButton::Back));
        assert!(!m.button_held(MouseButton::Back));

        assert!(!m.button_pressed(MouseButton::Forward));
        assert!(!m.button_released(MouseButton::Forward));
        assert!(!m.button_held(MouseButton::Forward));
    }

    #[test]
    fn mouse_position_changed() {
        let mut m = Mouse::default();

        m.on_event(&cursor_moved(10.0, 10.0));
        m.flush();

        assert_eq!((10.0, 10.0), m.position());
        assert_eq!((0.0, 0.0), m.position_delta());

        m.on_event(&cursor_moved(15.0, 20.0));
        m.flush();

        assert_eq!((11.0, 11.0), m.position());
        assert_eq!((5.0, 10.0), m.position_delta());
    }

    fn cursor_moved(x: f32, y: f32) -> winit::event::WindowEvent {
        winit::event::WindowEvent::CursorMoved {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            position: PhysicalPosition::new(x as f64, y as f64),
        }
    }
}
