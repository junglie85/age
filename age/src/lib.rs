mod app;
mod error;
mod font;
mod graphics;
mod image;
mod os;
mod packer;
mod renderer;

pub use app::{App, AppBuilder, Context, MouseEvent};
pub use error::{AgeError, AgeResult};
pub use font::{CharSet, Font, Glyph, SpriteFont};
pub use graphics::{Camera, Rect, Sprite, Vertex};
pub use image::Image;
pub use os::{ButtonState, Mouse, MouseButton};
pub use packer::{Entry, PackerInfo, TexturePacker};
pub use renderer::{
    align_to, AddressMode, BindGroup, BindGroupId, BindGroupInfo, BindGroupLayout,
    BindGroupLayoutId, BindGroupLayoutInfo, Binding, BindingType, Buffer, BufferId, BufferInfo,
    BufferType, Color, DrawCommand, DrawTarget, FilterMode, IndexFormat, IndexedDraw,
    PipelineLayout, PipelineLayoutId, PipelineLayoutInfo, RenderDevice, RenderPipeline,
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
}
