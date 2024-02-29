use std::{ops::Deref, sync::Arc};

use winit::{dpi::LogicalSize, event_loop::ControlFlow};

use crate::error::Error;

pub(crate) struct EventLoop {
    el: Option<winit::event_loop::EventLoop<()>>,
}

impl EventLoop {
    pub(crate) fn init() -> Result<Self, Error> {
        let el = Some(winit::event_loop::EventLoopBuilder::new().build()?);
        Ok(Self { el })
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

impl Deref for EventLoop {
    type Target = winit::event_loop::EventLoop<()>;

    fn deref(&self) -> &Self::Target {
        // This is safe because we create the winit event loop on init and don't consume it until run.
        self.el.as_ref().unwrap()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(winit::window::WindowId);

pub struct Window {
    w: Arc<winit::window::Window>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    surface_texture_view: Option<wgpu::TextureView>,
}

impl Window {
    pub(crate) fn init(width: u32, height: u32, el: &EventLoop) -> Result<Window, Error> {
        let size = LogicalSize::new(width, height);
        let w = winit::window::WindowBuilder::new()
            .with_title("AGE")
            .with_inner_size(size)
            .with_visible(false)
            .build(el)?;
        Ok(Window {
            w: Arc::new(w),
            surface_texture: None,
            surface_texture_view: None,
        })
    }

    pub(crate) fn get_handle(&self) -> WindowHandle {
        WindowHandle { w: self.w.clone() }
    }

    pub fn get_id(&self) -> WindowId {
        WindowId(self.w.id())
    }

    pub(crate) fn get_name(&self) -> Option<&str> {
        Some("primary window")
    }

    pub fn get_size(&self) -> (u32, u32) {
        self.w.inner_size().into()
    }

    pub(crate) fn present(&mut self) {
        assert!(
            self.surface_texture.is_some(),
            "surface texture has not been set"
        );

        self.w.pre_present_notify();
        self.surface_texture.take().unwrap().present();
        self.surface_texture_view = None;
        self.w.request_redraw();
    }

    pub(crate) fn set_surface_texture(&mut self, surface_texture: wgpu::SurfaceTexture) {
        self.surface_texture = Some(surface_texture);
    }

    pub(crate) fn get_surface_texture_view(&self) -> &wgpu::TextureView {
        assert!(
            self.surface_texture_view.is_some(),
            "surface texture view has not been set"
        );
        self.surface_texture_view.as_ref().unwrap()
    }

    pub(crate) fn set_surface_texture_view(&mut self, view: wgpu::TextureView) {
        self.surface_texture_view = Some(view);
    }

    pub fn set_visible(&self, visible: bool) {
        self.w.set_visible(visible);
    }
}

pub struct WindowHandle {
    w: Arc<winit::window::Window>,
}

impl raw_window_handle::HasDisplayHandle for WindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.w.display_handle()
    }
}

impl raw_window_handle::HasWindowHandle for WindowHandle {
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
