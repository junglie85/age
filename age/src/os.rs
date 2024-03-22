use age_math::{v2, Vec2};
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

pub(crate) fn create_mouse() -> Mouse {
    Mouse::new()
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ButtonState {
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
}

#[derive(Debug)]
pub struct Mouse {
    previous_position: Vec2,
    current_position: Vec2,
    position_delta: Vec2,
    current_scroll_delta: Vec2,
    scroll_delta: Vec2,
    previous_button_state: Vec<bool>,
    current_button_state: Vec<bool>,
    button_state: Vec<ButtonState>,
}

impl Mouse {
    fn new() -> Self {
        Self {
            previous_position: Vec2::ZERO,
            current_position: Vec2::ZERO,
            position_delta: Vec2::ZERO,
            current_scroll_delta: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            previous_button_state: Vec::with_capacity(5), // Left, right, middle, forward, back.
            current_button_state: Vec::with_capacity(5),  // Left, right, middle, forward, back.
            button_state: Vec::with_capacity(5),          // Left, right, middle, forward, back.
        }
    }

    pub fn position(&self) -> (f32, f32) {
        (self.current_position.x, self.current_position.y)
    }

    pub fn position_delta(&self) -> (f32, f32) {
        (self.position_delta.x, self.position_delta.y)
    }

    pub fn scroll_delta(&self) -> (f32, f32) {
        (self.scroll_delta.x, self.scroll_delta.y)
    }

    pub fn button(&self, button: MouseButton) -> ButtonState {
        let index = button.as_usize();
        if index < self.button_state.len() {
            self.button_state[index]
        } else {
            ButtonState::default()
        }
    }

    pub(crate) fn on_event(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.current_position = v2(position.x as f32, position.y as f32);
            }

            winit::event::WindowEvent::MouseWheel {
                delta: winit::event::MouseScrollDelta::LineDelta(x, y),
                ..
            } => {
                self.current_scroll_delta.x += x;
                self.current_scroll_delta.y += y;
            }

            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let index = Into::<MouseButton>::into(*button).as_usize();
                if index >= self.button_state.len() {
                    let new_len = index + 1;
                    self.previous_button_state.resize(new_len, false);
                    self.current_button_state.resize(new_len, false);
                    self.button_state.resize(new_len, ButtonState::default());
                }

                self.current_button_state[index] = *state == winit::event::ElementState::Pressed;
            }

            _ => (),
        }
    }

    pub(crate) fn flush(&mut self) {
        self.position_delta = self.current_position - self.previous_position;
        self.previous_position = self.current_position;

        self.scroll_delta = self.current_scroll_delta;
        self.current_scroll_delta = Vec2::ZERO;

        for (i, state) in self.button_state.iter_mut().enumerate() {
            state.pressed = false;
            state.held = false;
            state.released = false;
            if self.current_button_state[i] != self.previous_button_state[i] {
                if self.current_button_state[i] {
                    state.pressed = !state.held;
                } else {
                    state.released = true;
                }
            } else {
                // No change in state, is the button held?
                state.held = self.current_button_state[i];
            }
            self.previous_button_state[i] = self.current_button_state[i];
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    /// Gets the numeric representation of the enum. Cannot cast because Other(u16) variant is non-primitive.
    fn as_usize(&self) -> usize {
        match self {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Other(id) => *id as usize,
        }
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(id) => MouseButton::Other(id),
        }
    }
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
    use winit::dpi::PhysicalPosition;

    #[test]
    fn mouse_defaults() {
        let m = Mouse::new();

        assert_eq!((0.0, 0.0), m.position());
        assert_eq!((0.0, 0.0), m.position_delta());

        assert!(!m.button(MouseButton::Left).pressed);
        assert!(!m.button(MouseButton::Left).released);
        assert!(!m.button(MouseButton::Left).held);

        assert!(!m.button(MouseButton::Right).pressed);
        assert!(!m.button(MouseButton::Right).released);
        assert!(!m.button(MouseButton::Right).held);

        assert!(!m.button(MouseButton::Middle).pressed);
        assert!(!m.button(MouseButton::Middle).released);
        assert!(!m.button(MouseButton::Middle).held);

        assert!(!m.button(MouseButton::Back).pressed);
        assert!(!m.button(MouseButton::Back).released);
        assert!(!m.button(MouseButton::Back).held);

        assert!(!m.button(MouseButton::Forward).pressed);
        assert!(!m.button(MouseButton::Forward).released);
        assert!(!m.button(MouseButton::Forward).held);
    }

    #[test]
    fn mouse_position_changed() {
        let mut m = Mouse::new();

        m.on_event(&cursor_moved(10.0, 10.0));
        m.flush();

        assert_eq!((10.0, 10.0), m.position());
        assert_eq!((10.0, 10.0), m.position_delta());

        m.on_event(&cursor_moved(15.0, 20.0));
        m.flush();

        assert_eq!((15.0, 20.0), m.position());
        assert_eq!((5.0, 10.0), m.position_delta());
    }

    #[test]
    fn mouse_left_button() {
        let mut m = Mouse::new();

        m.on_event(&mouse_input(
            winit::event::ElementState::Pressed,
            winit::event::MouseButton::Left,
        ));
        m.flush();

        assert!(m.button(MouseButton::Left).pressed);
        assert!(!m.button(MouseButton::Left).held);
        assert!(!m.button(MouseButton::Left).released);

        m.on_event(&mouse_input(
            winit::event::ElementState::Pressed,
            winit::event::MouseButton::Left,
        ));
        m.flush();

        assert!(!m.button(MouseButton::Left).pressed);
        assert!(m.button(MouseButton::Left).held);
        assert!(!m.button(MouseButton::Left).released);

        m.on_event(&mouse_input(
            winit::event::ElementState::Released,
            winit::event::MouseButton::Left,
        ));
        m.flush();

        assert!(!m.button(MouseButton::Left).pressed);
        assert!(!m.button(MouseButton::Left).held);
        assert!(m.button(MouseButton::Left).released);

        m.flush();

        assert!(!m.button(MouseButton::Left).pressed);
        assert!(!m.button(MouseButton::Left).held);
        assert!(!m.button(MouseButton::Left).released);
    }

    #[test]
    fn mouse_scroll_delta() {
        let mut m = Mouse::new();

        m.on_event(&mouse_wheel(1.0, 2.0));
        m.flush();

        assert_eq!((1.0, 2.0), m.scroll_delta());

        m.on_event(&mouse_wheel(-1.0, -2.0));
        m.flush();

        assert_eq!((-1.0, -2.0), m.scroll_delta());

        m.on_event(&mouse_wheel(3.0, 4.0));
        m.on_event(&mouse_wheel(5.0, 6.0));
        m.flush();

        assert_eq!((8.0, 10.0), m.scroll_delta());

        m.flush();

        assert_eq!((0.0, 0.0), m.scroll_delta());
    }

    fn cursor_moved(x: f32, y: f32) -> winit::event::WindowEvent {
        winit::event::WindowEvent::CursorMoved {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            position: PhysicalPosition::new(x as f64, y as f64),
        }
    }

    fn mouse_input(
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) -> winit::event::WindowEvent {
        winit::event::WindowEvent::MouseInput {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            state,
            button,
        }
    }

    fn mouse_wheel(x: f32, y: f32) -> winit::event::WindowEvent {
        winit::event::WindowEvent::MouseWheel {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            delta: winit::event::MouseScrollDelta::LineDelta(x, y),
            phase: winit::event::TouchPhase::Moved, // Doesn't really matter whilst touch is unsupported.
        }
    }
}
