mod app;
mod error;
mod graphics;
mod image;
mod os;
mod renderer;

pub use app::{App, AppBuilder, Context};
pub use error::{AgeError, AgeResult};
pub use graphics::{Camera, Rect, Vertex};
pub use image::Image;
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
}
