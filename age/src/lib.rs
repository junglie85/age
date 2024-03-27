mod app;
mod error;
mod font;
mod graphics;
mod image;
mod os;
mod packer;
mod renderer;

pub use app::{App, AppBuilder, Context, KeyboardEvent, MouseEvent};
pub use error::{AgeError, AgeResult};
pub use font::{CharSet, Font, Glyph, SpriteFont};
pub use graphics::{map_screen_to_world, map_world_to_screen, Camera, Sprite, Vertex};
pub use image::Image;
pub use os::{ButtonState, KeyCode, KeyLocation, Keyboard, Mouse, MouseButton, ScanCode};
pub use packer::{Entry, PackerInfo, TexturePacker};
pub use renderer::{
    align_to, AddressMode, BindGroup, BindGroupId, BindGroupInfo, BindGroupLayout,
    BindGroupLayoutId, BindGroupLayoutInfo, Binding, BindingType, Buffer, BufferId, BufferInfo,
    BufferType, Color, DrawCommand, DrawTarget, FilterMode, IndexFormat, IndexedDraw,
    PipelineLayout, PipelineLayoutId, PipelineLayoutInfo, Rect, RenderDevice, RenderPipeline,
    RenderPipelineId, RenderPipelineInfo, Sampler, SamplerId, SamplerInfo, Shader, ShaderId,
    ShaderInfo, Texture, TextureFormat, TextureId, TextureInfo, TextureView, TextureViewId,
    TextureViewInfo, VertexBufferLayout, VertexFormat, VertexType,
};

pub trait Game {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context);

    fn on_stop(&mut self, _ctx: &mut Context) {}

    fn on_exit(&mut self, ctx: &mut Context) {
        ctx.exit();
    }

    fn on_mouse_event(&mut self, _event: MouseEvent, _ctx: &mut Context) {}

    fn on_keyboard_event(&mut self, _event: KeyboardEvent, _ctx: &mut Context) {}

    fn on_text_entered(&mut self, _text: &str, _ctx: &mut Context) {}

    fn on_size_changed(&mut self, _width: u32, _height: u32, _ctx: &mut Context) {}

    fn on_scale_factor_changed(&mut self, _scale_factor: f32, _ctx: &mut Context) {}
}
