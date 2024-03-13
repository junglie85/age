mod app;
mod error;
mod graphics;
mod os;
mod renderer;

pub use app::{App, AppBuilder, Context};
pub use error::{AgeError, AgeResult};
pub use graphics::{Camera, Vertex};
pub use renderer::{
    align_to, AddressMode, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo, Binding,
    BindingType, Buffer, BufferInfo, BufferType, Color, DrawCommand, DrawTarget, FilterMode,
    IndexFormat, IndexedDraw, PipelineLayout, PipelineLayoutInfo, RenderDevice, RenderPipeline,
    RenderPipelineInfo, Sampler, SamplerInfo, Shader, ShaderInfo, Texture, TextureFormat,
    TextureInfo, TextureView, TextureViewInfo, VertexBufferLayout, VertexFormat, VertexType,
};

pub trait Game {
    fn on_start(&mut self, _ctx: &mut Context) {}

    fn on_tick(&mut self, ctx: &mut Context);

    fn on_stop(&mut self, _ctx: &mut Context) {}

    fn on_exit(&mut self, ctx: &mut Context) {
        ctx.exit();
    }
}
