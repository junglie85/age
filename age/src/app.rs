use std::sync::Arc;

use age_math::{v2, Mat4, Vec2};
use winit::{
    event::{ElementState, Event, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};

use crate::{
    graphics::Graphics,
    os::{self, Keyboard, Mouse, MouseButton},
    renderer::{Color, DrawTarget, RenderDevice, RenderPipeline, WindowSurface, WindowTarget},
    AgeResult, BindGroup, Camera, Game, Image, Key, KeyCode, Rect, ScanCode, Sprite, SpriteFont,
    TextureFormat, TextureInfo,
};

pub struct AppConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            title: "AGE".to_string(),
        }
    }
}

#[derive(Default)]
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    pub fn new(width: u32, height: u32) -> Self {
        let config = AppConfig {
            width,
            height,
            ..Default::default()
        };

        Self { config }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn build(self) -> AgeResult<App> {
        let el = os::create_event_loop::<AppEvent>()?;
        let el_proxy = el.create_proxy();
        let window = os::create_window(&self.config, &el)?;
        let mouse = os::create_mouse();
        let keyboard = os::create_keyboard();

        let device = RenderDevice::new()?;
        let surface = WindowSurface::new();

        let window_target = WindowTarget::new(self.config.width, self.config.height, &device);
        let graphics = Graphics::new(
            v2(self.config.width as f32, self.config.height as f32),
            &device,
        )?;

        let ctx = Context {
            config: self.config,
            el_proxy,
            mouse,
            keyboard,
            window: Arc::new(window),
            device,
            graphics,
            window_target,
            running: false,
        };

        Ok(App { el, surface, ctx })
    }
}

pub struct App {
    el: EventLoop<AppEvent>,
    surface: WindowSurface,
    ctx: Context,
}

impl App {
    pub fn new(width: u32, height: u32) -> AgeResult<Self> {
        AppBuilder::new(width, height).build()
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    pub fn run(self, mut game: impl Game) -> AgeResult {
        let App {
            el,
            mut surface,
            mut ctx,
        } = self;

        game.on_start(&mut ctx);

        ctx.running = true;
        ctx.window.set_visible(true);

        os::run(el, |event, elwt| {
            match event {
                Event::WindowEvent { window_id, event } if ctx.window.id() == window_id => {
                    ctx.mouse.on_event(&event);
                    ctx.keyboard.on_event(&event);

                    match event {
                        WindowEvent::CloseRequested => game.on_exit(&mut ctx),

                        WindowEvent::CursorEntered { .. } => {
                            game.on_mouse_event(MouseEvent::CursorEntered, &mut ctx);
                        }
                        WindowEvent::CursorLeft { .. } => {
                            game.on_mouse_event(MouseEvent::CursorExited, &mut ctx);
                        }
                        WindowEvent::CursorMoved { position, .. } => game.on_mouse_event(
                            MouseEvent::Moved {
                                x: position.x as f32,
                                y: position.y as f32,
                            },
                            &mut ctx,
                        ),
                        WindowEvent::MouseInput {
                            button,
                            state: ElementState::Pressed,
                            ..
                        } => {
                            game.on_mouse_event(MouseEvent::ButtonPressed(button.into()), &mut ctx);
                        }
                        WindowEvent::MouseInput {
                            button,
                            state: ElementState::Released,
                            ..
                        } => {
                            game.on_mouse_event(
                                MouseEvent::ButtonReleased(button.into()),
                                &mut ctx,
                            );
                        }
                        WindowEvent::MouseWheel {
                            delta: MouseScrollDelta::LineDelta(x, y),
                            ..
                        } => game.on_mouse_event(
                            MouseEvent::Scrolled {
                                delta_x: x,
                                delta_y: y,
                            },
                            &mut ctx,
                        ),

                        WindowEvent::KeyboardInput { event, .. } => {
                            let key = &event.logical_key;
                            if let (Ok(keycode), Ok(scancode)) = (
                                TryInto::<KeyCode>::try_into(key),
                                event.physical_key.try_into(),
                            ) {
                                if event.state.is_pressed() {
                                    game.on_keyboard_event(
                                        KeyboardEvent::ButtonPressed(keycode, scancode),
                                        &mut ctx,
                                    );
                                } else {
                                    game.on_keyboard_event(
                                        KeyboardEvent::ButtonReleased(keycode, scancode),
                                        &mut ctx,
                                    );
                                }

                                if let Some(text) = event.text {
                                    game.on_text_entered(text.as_str(), &mut ctx);
                                }
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            ctx.device.begin_frame();
                            ctx.graphics.begin_frame(&ctx.window_target, &ctx.device);

                            game.on_tick(&mut ctx);

                            ctx.window_target.draw(&mut surface, &mut ctx.device)?;
                            ctx.device.end_frame();

                            ctx.window.pre_present_notify();
                            surface.present();
                            ctx.window.request_redraw();

                            ctx.mouse.flush();
                            ctx.keyboard.flush();
                        }

                        WindowEvent::Resized(size) => {
                            let (width, height) = size.into();
                            surface.reconfigure(&ctx.device, width, height, surface.vsync())?;
                            ctx.window_target.reconfigure(&surface, &ctx.device);
                            let logical_size = size.to_logical(ctx.window.scale_factor());
                            ctx.graphics.reconfigure(
                                logical_size.width,
                                logical_size.height,
                                ctx.scale_factor(),
                            );

                            game.on_size_changed(logical_size.width, logical_size.height, &mut ctx);
                        }

                        WindowEvent::ScaleFactorChanged { .. } => {
                            let size = ctx.window.inner_size();
                            let (width, height) = size.into();
                            surface.reconfigure(&ctx.device, width, height, surface.vsync())?;
                            ctx.window_target.reconfigure(&surface, &ctx.device);
                            let logical_size = size.to_logical(ctx.window.scale_factor());
                            ctx.graphics.reconfigure(
                                logical_size.width,
                                logical_size.height,
                                ctx.scale_factor(),
                            );

                            game.on_scale_factor_changed(ctx.scale_factor(), &mut ctx);
                        }

                        _ => {}
                    }
                }

                Event::Resumed => {
                    surface.resume(&ctx.device, ctx.window.clone())?;
                    ctx.window_target.reconfigure(&surface, &ctx.device);
                }

                Event::Suspended => surface.suspend(),

                Event::UserEvent(event) => match event {
                    AppEvent::EnableVsync(enabled) => {
                        if surface.vsync() != enabled {
                            let (width, height) = surface.size();
                            surface.reconfigure(&ctx.device, width, height, enabled)?
                        }
                    }
                },

                _ => {}
            }

            if !ctx.running {
                elwt.exit();
            }

            Ok(())
        })?;

        ctx.running = false;

        game.on_stop(&mut ctx);

        Ok(())
    }
}

enum AppEvent {
    EnableVsync(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEvent {
    ButtonPressed(MouseButton),
    ButtonReleased(MouseButton),
    CursorEntered,
    CursorExited,
    Moved { x: f32, y: f32 },
    Scrolled { delta_x: f32, delta_y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyboardEvent {
    ButtonPressed(KeyCode, ScanCode),
    ButtonReleased(KeyCode, ScanCode),
    // TextEntered
}

pub struct Context {
    config: AppConfig,
    el_proxy: EventLoopProxy<AppEvent>,
    mouse: Mouse,
    keyboard: Keyboard,
    window: Arc<Window>,
    device: RenderDevice,
    graphics: Graphics,
    window_target: WindowTarget,
    running: bool,
}

impl Context {
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn graphics(&self) -> &Graphics {
        &self.graphics
    }

    pub fn mouse(&self) -> &Mouse {
        &self.mouse
    }

    pub fn keyboard(&self) -> &Keyboard {
        &self.keyboard
    }

    pub fn render_device(&self) -> &RenderDevice {
        &self.device
    }

    pub fn window_target(&self) -> DrawTarget {
        Into::<DrawTarget>::into(&self.window_target)
    }

    pub fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    pub fn window_size(&self) -> (u32, u32) {
        self.window.inner_size().into()
    }

    pub fn exit(&mut self) {
        self.running = false;
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    pub fn set_vsync(&self, enabled: bool) {
        if self
            .el_proxy
            .send_event(AppEvent::EnableVsync(enabled))
            .is_err()
        {
            eprintln!("attempted to send message on closed event loop");
        }
    }
}

impl Context {
    pub fn key_pressed(&self, key: impl Into<Key>) -> bool {
        self.keyboard.key_pressed(key)
    }

    pub fn key_held(&self, key: impl Into<Key>) -> bool {
        self.keyboard.key_held(key)
    }

    pub fn key_released(&self, key: impl Into<Key>) -> bool {
        self.keyboard.key_released(key)
    }

    pub fn mouse_position(&self) -> Vec2 {
        self.mouse.position()
    }

    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse.button_pressed(button)
    }

    pub fn mouse_button_released(&self, button: MouseButton) -> bool {
        self.mouse.button_released(button)
    }

    pub fn mouse_button_held(&self, button: MouseButton) -> bool {
        self.mouse.button_held(button)
    }
}

impl Context {
    pub fn create_sprite_from_image(&self, image: &Image, label: Option<&str>) -> Sprite {
        let texture = self.device.create_texture(&TextureInfo {
            label,
            width: image.width(),
            height: image.height(),
            format: TextureFormat::Rgba8UnormSrgb,
            ..Default::default()
        });
        self.device.write_texture(&texture, image.pixels());

        self.graphics
            .create_sprite(&texture, self.graphics.default_sampler(), &self.device)
    }

    pub fn default_camera(&self) -> &Camera {
        self.graphics.default_camera()
    }

    pub fn push_matrix(&mut self, matrix: Mat4) {
        self.graphics.push_matrix(matrix);
    }

    pub fn push_matrix_ext(&mut self, matrix: Mat4, absolute: bool) {
        self.graphics.push_matrix_ext(matrix, absolute);
    }

    pub fn pop_matrix(&mut self) -> Mat4 {
        self.graphics.pop_matrix()
    }

    pub fn set_camera(&mut self, camera: &Camera) {
        self.graphics.set_camera(camera, &self.device);
    }

    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.graphics.set_draw_target(target);
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.graphics.set_render_pipeline(pipeline);
    }

    pub fn clear(&mut self, color: Color) {
        self.graphics.clear(color, &mut self.device);
    }

    pub fn draw_line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color) {
        self.graphics
            .draw_line(from, to, thickness, color, &mut self.device);
    }

    pub fn draw_line_ext(
        &mut self,
        from: Vec2,
        to: Vec2,
        origin: Vec2,
        thickness: f32,
        color: Color,
    ) {
        self.graphics
            .draw_line_ext(from, to, origin, thickness, color, &mut self.device);
    }

    pub fn draw_line_from(
        &mut self,
        position: Vec2,
        angle: f32,
        length: f32,
        thickness: f32,
        color: Color,
    ) {
        self.graphics
            .draw_line_from(position, angle, length, thickness, color, &mut self.device);
    }

    pub fn draw_line_from_ext(
        &mut self,
        position: Vec2,
        angle: f32,
        length: f32,
        thickness: f32,
        color: Color,
        origin: Vec2,
    ) {
        self.graphics.draw_line_from_ext(
            position,
            angle,
            length,
            thickness,
            color,
            origin,
            &mut self.device,
        );
    }

    pub fn draw_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        thickness: f32,
        color: Color,
    ) {
        self.graphics.draw_rect(
            position,
            rotation,
            scale,
            origin,
            thickness,
            color,
            &mut self.device,
        );
    }

    pub fn draw_filled_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        color: Color,
    ) {
        self.graphics
            .draw_filled_rect(position, rotation, scale, origin, color, &mut self.device);
    }

    pub fn draw_textured_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        texture_bg: &BindGroup,
    ) {
        self.graphics.draw_textured_rect(
            position,
            rotation,
            scale,
            origin,
            texture_bg,
            &mut self.device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_textured_rect_ext(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        texture_bg: &BindGroup,
        texture_rect: Rect,
        color: Color,
    ) {
        self.graphics.draw_textured_rect_ext(
            position,
            rotation,
            scale,
            origin,
            texture_bg,
            texture_rect,
            color,
            &mut self.device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        thickness: f32,
        color: Color,
    ) {
        self.graphics.draw_circle(
            position,
            radius,
            point_count,
            rotation,
            origin,
            thickness,
            color,
            &mut self.device,
        );
    }

    pub fn draw_filled_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        color: Color,
    ) {
        self.graphics.draw_filled_circle(
            position,
            radius,
            point_count,
            rotation,
            origin,
            color,
            &mut self.device,
        );
    }

    pub fn draw_textured_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        texture_bg: &BindGroup,
    ) {
        self.graphics.draw_textured_circle(
            position,
            radius,
            point_count,
            rotation,
            origin,
            texture_bg,
            &mut self.device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_textured_circle_ext(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        texture_bg: &BindGroup,
        texture_rect: Rect,
        color: Color,
    ) {
        self.graphics.draw_textured_circle_ext(
            position,
            radius,
            point_count,
            rotation,
            origin,
            texture_bg,
            texture_rect,
            color,
            &mut self.device,
        );
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite, position: Vec2, rotation: f32) {
        self.graphics
            .draw_sprite(sprite, position, rotation, &mut self.device);
    }

    pub fn draw_sprite_ext(
        &mut self,
        sprite: &Sprite,
        texture_rect: Rect,
        color: Color,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
    ) {
        self.graphics.draw_sprite_ext(
            sprite,
            texture_rect,
            color,
            position,
            rotation,
            scale,
            &mut self.device,
        );
    }

    pub fn draw_string(
        &mut self,
        font: &SpriteFont,
        text: &str,
        size: f32,
        color: Color,
        position: Vec2,
    ) {
        self.graphics
            .draw_string(font, text, size, color, position, &mut self.device);
    }

    pub fn draw_string_ext(
        &mut self,
        font: &SpriteFont,
        text: &str,
        size: f32,
        color: Color,
        position: Vec2,
        justify: Vec2,
    ) {
        self.graphics
            .draw_string_ext(font, text, size, color, position, justify, &mut self.device);
    }
}
