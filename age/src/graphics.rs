use std::collections::HashMap;

use age_math::{v2, Mat4, Vec2};
use bytemuck::{cast_slice, Pod, Zeroable};

use crate::{
    renderer::{
        self, BindGroup, BindGroupInfo, BindGroupLayout, BindGroupLayoutInfo, Binding, BindingType,
        Buffer, BufferInfo, BufferType, Color, DrawCommand, DrawTarget, IndexFormat, IndexedDraw,
        PipelineLayoutInfo, RenderDevice, RenderPipeline, RenderPipelineInfo, Sampler, ShaderInfo,
        Texture, TextureFormat, VertexBufferLayout, VertexFormat, VertexType,
    },
    AddressMode, AgeResult, FilterMode, Font, Rect, SamplerInfo, SpriteFont, TextureInfo,
    TextureView, TextureViewInfo,
};

pub struct Graphics {
    draw_state: DrawState,
    camera_bgl: BindGroupLayout,
    texture_bgl: BindGroupLayout,
    default_sampler: Sampler,
    default_texture: Texture,
    #[allow(dead_code)]
    default_texture_view: TextureView,
    default_texture_bg: BindGroup,
    default_pipeline: RenderPipeline,
    default_font: Font,
    default_camera: Camera,
    meshes: Meshes,
}

impl Graphics {
    pub const VERTEX_TYPE_FILL: f32 = 1.0;
    pub const VERTEX_TYPE_OUTLINE: f32 = 2.0;
    pub const VERTEX_TYPE_TEXT: f32 = 3.0;

    pub(crate) fn new(size: Vec2, device: &RenderDevice) -> AgeResult<Self> {
        let shader = device.create_shader(&ShaderInfo {
            label: Some("graphics"),
            src: include_str!("shaders/graphics.wgsl"),
        });

        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("graphics camera"),
            entries: &[BindingType::Uniform {
                min_size: std::mem::size_of::<[f32; 16]>() as u64,
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&BindGroupLayoutInfo {
            label: Some("graphics texture"),
            entries: &[
                BindingType::Sampler,
                BindingType::Texture { sample_count: 1 },
            ],
        });

        let pl = device.create_pipeline_layout(&PipelineLayoutInfo {
            label: Some("graphics"),
            bind_group_layouts: &[&camera_bgl, &texture_bgl],
            push_constant_ranges: &[&(0..std::mem::size_of::<PushConstant>() as u32)],
        });

        let default_sampler = device.create_sampler(&SamplerInfo {
            label: Some("graphics default"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
        });

        let default_texture = device.create_texture(&TextureInfo {
            label: Some("graphics default"),
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            renderable: false,
            sample_count: 1,
        });
        device.write_texture(&default_texture, &Color::WHITE.to_array_u8());

        let default_texture_view = default_texture.create_view(&TextureViewInfo {
            label: Some("graphics default"),
        });

        let default_texture_bg = device.create_bind_group(&BindGroupInfo {
            label: Some("graphics texture"),
            layout: &texture_bgl,
            entries: &[
                Binding::Sampler {
                    sampler: &default_sampler,
                },
                Binding::Texture {
                    texture_view: &default_texture_view,
                },
            ],
        });

        let default_pipeline = device.create_render_pipeline(&RenderPipelineInfo {
            label: Some("graphics"),
            layout: &pl,
            shader: &shader,
            vs_main: "vs_main",
            fs_main: "fs_main",
            format: TextureFormat::Rgba8Unorm,
            buffers: &[Vertex::layout()],
        });

        let font_data = include_bytes!("../resources/fonts/RobotoMono-Regular.ttf");
        let default_font = Font::from_bytes(font_data)?;

        let center = size / 2.0;
        let default_camera = Camera::new(center, size);

        let meshes = Meshes::new(device);

        Ok(Self {
            draw_state: DrawState::default(),
            camera_bgl,
            texture_bgl,
            default_sampler,
            default_texture,
            default_texture_view,
            default_texture_bg,
            default_pipeline,
            default_font,
            default_camera,
            meshes,
        })
    }

    pub fn default_font(&self) -> &Font {
        &self.default_font
    }

    pub fn default_pipeline(&self) -> &RenderPipeline {
        &self.default_pipeline
    }

    pub fn default_sampler(&self) -> &Sampler {
        &self.default_sampler
    }

    pub fn default_texture(&self) -> &Texture {
        &self.default_texture
    }

    pub fn texture_bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_bgl
    }

    pub(crate) fn begin_frame(&mut self, target: impl Into<DrawTarget>, device: &RenderDevice) {
        self.draw_state = DrawState::default();
        self.set_draw_target(target);
        self.set_camera(&self.default_camera.clone(), device);
        self.set_render_pipeline(&self.default_pipeline.clone());
    }

    pub fn push_matrix(&mut self, matrix: Mat4) {
        self.push_matrix_ext(matrix, false);
    }

    pub fn push_matrix_ext(&mut self, matrix: Mat4, absolute: bool) {
        self.draw_state.matrix_stack.push(self.draw_state.matrix);
        if absolute {
            self.draw_state.matrix = matrix;
        } else {
            self.draw_state.matrix *= matrix;
        }
    }

    pub fn pop_matrix(&mut self) -> Mat4 {
        let previous = self.draw_state.matrix;
        if let Some(matrix) = self.draw_state.matrix_stack.pop() {
            self.draw_state.matrix = matrix;
        }
        previous
    }

    pub fn set_camera(&mut self, camera: &Camera, device: &RenderDevice) {
        let current_camera = match self.draw_state.cameras.iter().position(|c| c == camera) {
            Some(index) => index,
            None => {
                // todo: we can reuse ubo and bg if we don't completely nuke draw state each frame.
                let ubo = device.create_buffer(&BufferInfo {
                    label: Some("camera"),
                    size: std::mem::size_of::<[f32; 16]>() as u64,
                    ty: BufferType::Uniform,
                });
                device.write_buffer(&ubo, &camera.view_projection_matrix().to_cols_array());
                let bg = device.create_bind_group(&BindGroupInfo {
                    label: Some("camera"),
                    layout: &self.camera_bgl,
                    entries: &[Binding::Uniform { buffer: &ubo }],
                });

                self.draw_state.camera_ubos.push(ubo);
                self.draw_state.camera_bgs.push(bg);

                self.draw_state.cameras.push(camera.clone());
                self.draw_state.cameras.len() - 1
            }
        };

        self.draw_state.current_camera = Some(current_camera);
    }

    pub fn set_draw_target(&mut self, target: impl Into<DrawTarget>) {
        self.draw_state.target = Some(target.into());
        self.draw_state.clear_color = None;
    }

    pub fn set_render_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.draw_state.pipeline = Some(pipeline.clone());
    }

    pub fn clear(&mut self, color: Color, device: &mut RenderDevice) {
        self.draw_state.clear_color = Some(color);

        draw(
            &mut self.draw_state,
            Vec2::ZERO,
            0.0,
            Vec2::ZERO,
            Vec2::ZERO,
            Color::WHITE,
            &self.default_texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ZERO),
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            0,
            device,
        );
    }

    pub fn draw_line(
        &mut self,
        from: Vec2,
        to: Vec2,
        thickness: f32,
        color: Color,
        device: &mut RenderDevice,
    ) {
        let distance = to - from;
        let rotation = distance.y.atan2(distance.x);

        self.draw_filled_rect(
            from,
            rotation,
            v2(distance.length(), thickness),
            Vec2::ZERO,
            color,
            device,
        );
    }

    pub fn draw_line_ext(
        &mut self,
        from: Vec2,
        to: Vec2,
        origin: Vec2,
        thickness: f32,
        color: Color,
        device: &mut RenderDevice,
    ) {
        let distance = to - from;
        let rotation = distance.y.atan2(distance.x);

        self.draw_filled_rect(
            from,
            rotation,
            v2(distance.length(), thickness),
            origin,
            color,
            device,
        );
    }

    pub fn draw_line_from(
        &mut self,
        position: Vec2,
        angle: f32,
        length: f32,
        thickness: f32,
        color: Color,
        device: &mut RenderDevice,
    ) {
        self.draw_filled_rect(
            position,
            angle,
            v2(length, thickness),
            Vec2::ZERO,
            color,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_line_from_ext(
        &mut self,
        position: Vec2,
        angle: f32,
        length: f32,
        thickness: f32,
        color: Color,
        origin: Vec2,
        device: &mut RenderDevice,
    ) {
        self.draw_filled_rect(
            position,
            angle,
            v2(length, thickness),
            origin,
            color,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        thickness: f32,
        color: Color,
        device: &mut RenderDevice,
    ) {
        draw(
            &mut self.draw_state,
            position,
            rotation,
            scale,
            origin,
            color,
            &self.default_texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &self.meshes.rect_outline.vbo,
            [Self::VERTEX_TYPE_OUTLINE, thickness, 0.0, 0.0],
            &self.meshes.rect_outline.ibo,
            self.meshes.rect_outline.indices,
            device,
        );
    }

    pub fn draw_filled_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        color: Color,
        device: &mut RenderDevice,
    ) {
        draw(
            &mut self.draw_state,
            position,
            rotation,
            scale,
            origin,
            color,
            &self.default_texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            self.meshes.rect.indices,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_textured_rect(
        &mut self,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        origin: Vec2,
        texture_bg: &BindGroup,
        device: &mut RenderDevice,
    ) {
        draw(
            &mut self.draw_state,
            position,
            rotation,
            scale,
            origin,
            Color::WHITE,
            texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            self.meshes.rect.indices,
            device,
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
        device: &mut RenderDevice,
    ) {
        draw(
            &mut self.draw_state,
            position,
            rotation,
            scale,
            origin,
            color,
            texture_bg,
            texture_rect,
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            self.meshes.rect.indices,
            device,
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
        device: &mut RenderDevice,
    ) {
        let outline = &self
            .meshes
            .circle_outlines
            .entry(point_count)
            .or_insert_with(|| {
                let (vertices, _) = compute_circle(point_count as usize);
                let (vertices, indices) = compute_outline(&vertices);
                Mesh::new(
                    &vertices,
                    &indices,
                    Some(&format!("circle {} outline", point_count)),
                    device,
                )
            });
        let scale = Vec2::splat(radius);

        draw(
            &mut self.draw_state,
            position + scale, // We add scale here so that default origin is top-left corner of bounding box.
            rotation,
            scale,
            origin,
            color,
            &self.default_texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &outline.vbo,
            [Self::VERTEX_TYPE_OUTLINE, thickness, 0.0, 0.0],
            &outline.ibo,
            outline.indices,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_filled_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        color: Color,
        device: &mut RenderDevice,
    ) {
        let circle = &self.meshes.circles.entry(point_count).or_insert_with(|| {
            let (vertices, indices) = compute_circle(point_count as usize);
            Mesh::new(
                &vertices,
                &indices,
                Some(&format!("circle {}", point_count)),
                device,
            )
        });
        let scale = Vec2::splat(radius);

        draw(
            &mut self.draw_state,
            position + scale, // We add scale here so that default origin is top-left corner of bounding box.
            rotation,
            scale,
            origin,
            color,
            &self.default_texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &circle.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &circle.ibo,
            circle.indices,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_textured_circle(
        &mut self,
        position: Vec2,
        radius: f32,
        point_count: u32,
        rotation: f32,
        origin: Vec2,
        texture_bg: &BindGroup,
        device: &mut RenderDevice,
    ) {
        let circle = &self.meshes.circles.entry(point_count).or_insert_with(|| {
            let (vertices, indices) = compute_circle(point_count as usize);
            Mesh::new(
                &vertices,
                &indices,
                Some(&format!("circle {}", point_count)),
                device,
            )
        });
        let scale = Vec2::splat(radius);

        draw(
            &mut self.draw_state,
            position + scale, // We add scale here so that default origin is top-left corner of bounding box.
            rotation,
            scale,
            origin,
            Color::WHITE,
            texture_bg,
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &circle.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &circle.ibo,
            circle.indices,
            device,
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
        device: &mut RenderDevice,
    ) {
        let circle = &self.meshes.circles.entry(point_count).or_insert_with(|| {
            let (vertices, indices) = compute_circle(point_count as usize);
            Mesh::new(
                &vertices,
                &indices,
                Some(&format!("circle {}", point_count)),
                device,
            )
        });
        let scale = Vec2::splat(radius);

        draw(
            &mut self.draw_state,
            position + scale, // We add scale here so that default origin is top-left corner of bounding box.
            rotation,
            scale,
            origin,
            color,
            texture_bg,
            texture_rect,
            &circle.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &circle.ibo,
            circle.indices,
            device,
        );
    }

    pub fn draw_sprite(
        &mut self,
        sprite: &Sprite,
        position: Vec2,
        rotation: f32,
        device: &mut RenderDevice,
    ) {
        draw(
            &mut self.draw_state,
            position,
            rotation,
            sprite.size(),
            sprite.origin(),
            Color::WHITE,
            sprite.bind_group(),
            Rect::new(Vec2::ZERO, Vec2::ONE),
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            self.meshes.rect.indices,
            device,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite_ext(
        &mut self,
        sprite: &Sprite,
        texture_rect: Rect,
        color: Color,
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        device: &mut RenderDevice,
    ) {
        let scale = sprite.size() * scale * texture_rect.size;
        draw(
            &mut self.draw_state,
            position,
            rotation,
            scale,
            sprite.origin(),
            color,
            sprite.bind_group(),
            texture_rect,
            &self.meshes.rect.vbo,
            [Self::VERTEX_TYPE_FILL, 0.0, 0.0, 0.0],
            &self.meshes.rect.ibo,
            self.meshes.rect.indices,
            device,
        );
    }

    pub fn draw_string(
        &mut self,
        font: &SpriteFont,
        text: &str,
        size: f32,
        color: Color,
        position: Vec2,
        device: &mut RenderDevice,
    ) {
        self.draw_string_ext(font, text, size, color, position, Vec2::ZERO, device)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_string_ext(
        &mut self,
        font: &SpriteFont,
        text: &str,
        size: f32,
        color: Color,
        position: Vec2,
        justify: Vec2,
        device: &mut RenderDevice,
    ) {
        let scale = size / font.size();
        // self.push_matrix(Mat4::trs(pos, 0.0, Vec2f::splat(scale)));

        let ascent = font.ascent();
        let descent = font.descent();

        let mut offset = v2(0.0, ascent + descent);
        if justify != Vec2::ZERO {
            offset -= v2(font.width_of_line(text, 0), font.height_of(text)) * justify;
        }

        let mut last = None;
        for (i, c) in text.chars().enumerate() {
            if c == '\n' {
                offset.x = 0.0;
                let lh = font.line_height();
                offset.y += lh;

                if justify.x != 0.0 {
                    offset.x -= font.width_of_line(text, i + 1) * justify.x;
                }

                last = None;
            } else if let Some(glyph) = font.glyph(c) {
                let mut pos = offset + glyph.offset;
                if let Some(last) = last {
                    pos.x += font.kerning(last, c);
                }

                let texture = font.texture(glyph);
                let texture_size = v2(texture.width() as f32, texture.height() as f32);
                let texture_scale = texture_size * glyph.tex_rect.size * Vec2::splat(scale);
                draw(
                    &mut self.draw_state,
                    position + pos * scale,
                    0.0, // todo: how do we rotate the whole text? draw to a texture and rotate that.
                    texture_scale,
                    Vec2::ZERO,
                    color,
                    font.bind_group(glyph),
                    glyph.tex_rect,
                    &self.meshes.rect.vbo,
                    [Self::VERTEX_TYPE_TEXT, 0.0, 0.0, 0.0],
                    &self.meshes.rect.ibo,
                    self.meshes.rect.indices,
                    device,
                );

                offset.x += glyph.advance;
                last = Some(c);
            }
        }

        // self.pop_matrix();
    }

    pub fn create_sprite(
        &self,
        texture: &Texture,
        sampler: &Sampler,
        device: &RenderDevice,
    ) -> Sprite {
        Sprite::new(texture, sampler, &self.texture_bgl, texture.label(), device)
    }

    pub fn default_camera(&self) -> &Camera {
        &self.default_camera
    }

    pub fn reconfigure(&mut self, width: u32, height: u32, scale_factor: f32) {
        let size = v2(width as f32, height as f32);
        let center = size / 2.0;
        self.default_camera.resize(center, size);
        self.default_camera.set_zoom(1.0 / scale_factor);
        self.default_camera.set_scale_factor(scale_factor);
        println!(
            "graphics reconfigured: {width}, {height}, {:.2}",
            scale_factor
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    draw_state: &mut DrawState,
    position: Vec2,
    rotation: f32,
    scale: Vec2,
    origin: Vec2,
    color: Color,
    texture_bg: &BindGroup,
    texture_rect: Rect,
    vertices: &Buffer,
    info: [f32; 4], // fill, outline, etc.
    indices: &Buffer,
    index_count: usize,
    device: &mut RenderDevice,
) {
    let Some(target) = draw_state.target.as_ref() else {
        panic!("draw target is not set");
    };

    let (camera, camera_bg) = if let Some(index) = draw_state.current_camera {
        (&draw_state.cameras[index], &draw_state.camera_bgs[index])
    } else {
        panic!("camera is not set");
    };

    let Some(pipeline) = draw_state.pipeline.as_ref() else {
        panic!("render pipeline is not set");
    };

    let mut bind_groups = [RenderDevice::EMPTY_BIND_GROUP; RenderDevice::MAX_BIND_GROUPS];
    bind_groups[0] = Some(camera_bg.clone());
    bind_groups[1] = Some(texture_bg.clone());

    let translation = (position - origin).floor();
    let model = draw_state.matrix
        * Mat4::translation(translation)
        * Mat4::translation(origin)
        * Mat4::rotation(rotation)
        * Mat4::translation(-origin)
        * Mat4::scale(scale);
    let push_constant = PushConstant {
        model: model.to_cols_array(),
        color: color.to_array_f32(),
        texture_rect: texture_rect.to_array_f32(),
        info,
    };
    let push_constant_data = Some(cast_slice(&[push_constant]).to_vec());

    let mut vertex_buffers = [RenderDevice::EMPTY_VERTEX_BUFFER; RenderDevice::MAX_VERTEX_BUFFERS];
    vertex_buffers[0] = Some(vertices.clone());

    let indexed_draw = Some(IndexedDraw {
        buffer: indices.clone(),
        format: IndexFormat::Uint16,
        indices: 0..index_count as u32,
    });

    device.push_draw_command(DrawCommand {
        clear_color: draw_state.clear_color.take(),
        target: target.clone(),
        viewport: camera.viewport(),
        scissor: camera.scissor(),
        bind_groups,
        pipeline: pipeline.clone(),
        push_constant_data,
        vertex_buffers,
        vertices: 0..0, // Not needed because we're using indexed draw.
        indexed_draw,
    });
}

#[derive(Default)]
struct DrawState {
    matrix: Mat4,
    matrix_stack: Vec<Mat4>,
    cameras: Vec<Camera>,
    camera_ubos: Vec<Buffer>,
    camera_bgs: Vec<BindGroup>,
    current_camera: Option<usize>,
    clear_color: Option<Color>,
    target: Option<DrawTarget>,
    pipeline: Option<RenderPipeline>,
}

#[derive(Debug, Clone)]
pub struct Camera {
    center: Vec2,
    size: Vec2,
    zoom: f32,
    scale_factor: f32,
    rotation: f32,
    viewport: Rect,
    scissor: Rect,
}

impl Camera {
    pub const WHOLE_VIEW: Rect = Rect::new(Vec2::ZERO, Vec2::ONE);

    pub fn new(center: Vec2, size: Vec2) -> Self {
        Self {
            center,
            size,
            zoom: 1.0,
            scale_factor: 1.0,
            rotation: 0.0,
            viewport: Self::WHOLE_VIEW,
            scissor: Self::WHOLE_VIEW,
        }
    }

    pub fn resize(&mut self, center: Vec2, size: Vec2) {
        self.center = center;
        self.size = size;
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let half_size = self.size / 2.0;
        let top_left = (self.center - half_size) / self.zoom;
        let bottom_right = (self.center + half_size) / self.zoom;
        let proj = Mat4::orthographic(
            top_left.x,
            bottom_right.x,
            bottom_right.y,
            top_left.y,
            100.0,
            0.0,
        );

        let position = top_left;
        let origin = top_left;
        let view = (Mat4::translation(position - origin)
            * Mat4::translation(origin)
            * Mat4::rotation(self.rotation)
            * Mat4::translation(-origin)
            * Mat4::scale(Vec2::ONE))
        .inverse();

        proj * view
    }

    pub fn center(&self) -> Vec2 {
        self.center
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Set the viewport, expressed as a percentage of the overall camera view size.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }

    pub fn scissor(&self) -> Rect {
        self.scissor
    }

    /// Set the scissor rect, expressed as a percentage of the overall camera view size.
    pub fn set_scissor(&mut self, scissor: Rect) {
        self.scissor = scissor;
    }
}

impl PartialEq for Camera {
    fn eq(&self, other: &Self) -> bool {
        self.center == other.center
            && self.size == other.size
            && self.zoom == other.zoom
            && self.rotation == other.rotation
            && self.viewport == other.viewport
            && self.scissor == other.scissor
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstant {
    pub model: [f32; 16],
    pub color: [f32; 4],
    pub texture_rect: [f32; 4],
    pub info: [f32; 4], // [0 => vertex type, 1 => thickness, 2 => unused, 3 => unused]
}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: std::mem::size_of::<Self>() as u64,
            ty: VertexType::Vertex,
            formats: &[
                VertexFormat::Float32x2,
                VertexFormat::Float32x2,
                VertexFormat::Float32x2,
            ],
        }
    }
}

const fn v(position: [f32; 2], normal: [f32; 2], uv: [f32; 2]) -> Vertex {
    Vertex {
        position,
        normal,
        uv,
    }
}

struct Mesh {
    vbo: Buffer,
    ibo: Buffer,
    indices: usize,
}

impl Mesh {
    fn new(
        vertices: &[Vertex],
        indices: &[u16],
        label: Option<&str>,
        device: &RenderDevice,
    ) -> Self {
        let mut aligned_indices = indices.to_vec();
        let current_len = aligned_indices.len();
        let required_len =
            renderer::align_to(current_len, RenderDevice::COPY_BUFFER_ALIGNMENT as usize);
        if required_len != current_len {
            aligned_indices.resize(required_len, 0);
        }

        let vbo = device.create_buffer(&BufferInfo {
            label,
            size: std::mem::size_of::<Vertex>() as u64 * vertices.len() as u64,
            ty: BufferType::Vertex,
        });
        let ibo = device.create_buffer(&BufferInfo {
            label,
            size: std::mem::size_of::<u16>() as u64 * aligned_indices.len() as u64,
            ty: BufferType::Index,
        });

        device.write_buffer(&vbo, vertices);
        device.write_buffer(&ibo, &aligned_indices);

        Self {
            vbo,
            ibo,
            indices: indices.len(),
        }
    }
}

struct Meshes {
    rect: Mesh,
    rect_outline: Mesh,
    circles: HashMap<u32, Mesh>,
    circle_outlines: HashMap<u32, Mesh>,
}

impl Meshes {
    const RECT: [Vertex; 4] = [
        v([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]),
        v([0.0, 1.0], [0.0, 0.0], [0.0, 1.0]),
        v([1.0, 1.0], [0.0, 0.0], [1.0, 1.0]),
        v([1.0, 0.0], [0.0, 0.0], [1.0, 0.0]),
    ];
    const RECT_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

    fn new(device: &RenderDevice) -> Self {
        let rect = Mesh::new(&Self::RECT, &Self::RECT_INDICES, Some("rect"), device);

        let (vertices, indices) = compute_outline(&Self::RECT);
        let rect_outline = Mesh::new(&vertices, &indices, Some("rect outline"), device);

        let mut circles = HashMap::new();
        let (vertices, indices) = compute_circle(30);
        let circle = Mesh::new(&vertices, &indices, Some("circle 30"), device);
        circles.insert(30, circle);

        let mut circle_outlines = HashMap::new();
        let (vertices, indices) = compute_outline(&vertices);
        let circle_outline = Mesh::new(&vertices, &indices, Some("circle 30 outline"), device);
        circle_outlines.insert(30, circle_outline);

        Self {
            rect,
            rect_outline,
            circles,
            circle_outlines,
        }
    }
}

fn compute_circle(point_count: usize) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = vec![v([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]); point_count];
    let mut indices = vec![0_u16; (point_count - 2) * 3];

    for (i, vertex) in vertices.iter_mut().enumerate() {
        let angle = (i as f32 / point_count as f32) * 360.0_f32.to_radians();
        let (sine, cosine) = angle.sin_cos();
        let position = v2(sine, cosine);

        vertex.position = position.to_array();
        // Take the unit circle which is in range -1.0..=1.0 and map into range 0.0..=1.0.
        vertex.uv = ((position + Vec2::ONE) / 2.0).to_array();
    }

    for i in 0..point_count - 2 {
        let offset = i * 3;
        indices[offset] = 0;
        indices[offset + 1] = i as u16 + 1;
        indices[offset + 2] = i as u16 + 2;
    }

    (vertices, indices)
}

fn compute_outline(vertices: &[Vertex]) -> (Vec<Vertex>, Vec<u16>) {
    let point_count = vertices.len();
    let vertex_count = point_count * 2;
    let index_count = point_count * 6;

    // Compute center of the shape, used for pointing the normals outwards.
    let center = geometric_center(vertices);

    let mut outline_vertices = vec![v([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]); vertex_count];
    let mut indices = vec![0_u16; index_count];

    for i in 0..point_count {
        // https://stackoverflow.com/questions/68973103/how-to-create-outline?noredirect=1&lq=1
        let p = if i == 0 { point_count - 1 } else { i - 1 };

        let p1 = v2(vertices[p].position[0], vertices[p].position[1]);
        let p2 = v2(vertices[i].position[0], vertices[i].position[1]);
        let p3 = v2(
            vertices[(i + 1) % point_count].position[0],
            vertices[(i + 1) % point_count].position[1],
        );

        // Compute normals.
        let mut n12 = age_math::normal(p1, p2);
        let mut n23 = age_math::normal(p2, p3);

        // Point outwards.
        // Use dot product of normal and direction of center to current point (center - p2) to decide if inward or outward.
        if n12.dot(center - p2) > 0.0 {
            n12 = -n12;
        }
        if n23.dot(center - p2) > 0.0 {
            n23 = -n23;
        }

        let normal = (n12 + n23).normalize();

        // Construct vertex array such that inside point index % 2 == 0 and outline point % 2 == 1.
        // This allows us to apply a outline thickness weighting to the correct points in the shader.
        outline_vertices[2 * i].position = p2.to_array();
        outline_vertices[2 * i].normal = [0.0; 2];
        outline_vertices[2 * i + 1].position = p2.to_array();
        outline_vertices[2 * i + 1].normal = normal.to_array();

        // Modulo vertex count because the final set of indices needs to wrap back around to the first vertices.
        indices[6 * i] = (2 * i as u16) % vertex_count as u16; // i.e. 0
        indices[6 * i + 1] = ((2 * i as u16) + 1) % vertex_count as u16; // i.e. 1
        indices[6 * i + 2] = ((2 * i as u16) + 2) % vertex_count as u16; // i.e. 2
        indices[6 * i + 3] = ((2 * i as u16) + 2) % vertex_count as u16; // i.e. 2
        indices[6 * i + 4] = ((2 * i as u16) + 1) % vertex_count as u16; // i.e. 1
        indices[6 * i + 5] = ((2 * i as u16) + 3) % vertex_count as u16; // i.e. 3
    }

    (outline_vertices, indices)
}

fn geometric_center(vertices: &[Vertex]) -> Vec2 {
    let point_count = vertices.len();

    // https://stackoverflow.com/questions/34059116/what-is-the-fastest-way-to-find-the-center-of-an-irregular-convex-polygon
    let mut sum_center = Vec2::ZERO;
    let mut sum_weight = 0.0;

    for i in 0..point_count {
        let p = if i == 0 { point_count - 1 } else { i - 1 };

        let p1 = v2(vertices[p].position[0], vertices[p].position[1]);
        let p2 = v2(vertices[i].position[0], vertices[i].position[1]);
        let p3 = v2(
            vertices[(i + 1) % point_count].position[0],
            vertices[(i + 1) % point_count].position[1],
        );

        let weight = (p2 - p3).length() + (p2 - p1).length();
        sum_center += p2 * weight;
        sum_weight += weight;
    }

    sum_center / sum_weight
}

pub struct Sprite {
    texture: Texture,
    #[allow(dead_code)]
    view: TextureView,
    bg: BindGroup,
    origin: Vec2,
}

impl Sprite {
    pub fn new(
        texture: &Texture,
        sampler: &Sampler,
        layout: &BindGroupLayout,
        label: Option<&str>,
        device: &RenderDevice,
    ) -> Self {
        let view = texture.create_view(&TextureViewInfo { label });
        let bg = device.create_bind_group(&BindGroupInfo {
            label,
            layout,
            entries: &[
                Binding::Sampler { sampler },
                Binding::Texture {
                    texture_view: &view,
                },
            ],
        });

        Self {
            texture: texture.clone(),
            view,
            bg,
            origin: Vec2::ZERO,
        }
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bg
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn set_origin(&mut self, origin: Vec2) {
        self.origin = origin;
    }

    pub fn size(&self) -> Vec2 {
        let w = self.texture.width();
        let h = self.texture.height();
        v2(w as f32, h as f32)
    }
}

pub fn map_screen_to_world(position: Vec2, camera: &Camera) -> Vec2 {
    // Remove scale factor.
    let position = position / camera.scale_factor();

    // From https://github.com/SFML/SFML/blob/7ec3760fe8b451ef58b73df199066e6396a060f9/src/SFML/Graphics/RenderTarget.cpp#L306
    // Convert from viewport coordinates to homogeneous coordinates.
    let viewport = camera.viewport();
    let size = camera.size();
    let normalized = v2(-1.0, 1.0)
        + v2(2.0, -2.0) * (position - viewport.position * size) / (viewport.size * size);

    // Then transform by the inverse of the view-projection matrix.
    camera.view_projection_matrix().inverse() * normalized
}

pub fn map_world_to_screen(position: Vec2, camera: &Camera) -> Vec2 {
    // From https://github.com/SFML/SFML/blob/7ec3760fe8b451ef58b73df199066e6396a060f9/src/SFML/Graphics/RenderTarget.cpp#L326
    // Transform the point by the view-projection matrix.
    let normalized = camera.view_projection_matrix() * position;

    // Then convert to viewport coordinates.
    let viewport = camera.viewport();
    let size = camera.size();
    let position = (normalized * v2(1.0, -1.0) + v2(1.0, 1.0)) / v2(2.0, 2.0)
        * (viewport.size * size)
        + (viewport.position * size);

    // Apply scale factor.
    position * camera.scale_factor()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn map_from_screen_space_to_world_space() {
        let screen_pos = v2(200.0, 300.0);
        let camera = Camera::new(Vec2::splat(500.0), Vec2::splat(1000.0));

        assert_eq!(
            v2(200.0, 300.0),
            map_screen_to_world(screen_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }

    #[test]
    fn map_from_screen_space_to_world_space_with_offset() {
        let offset = Vec2::splat(300.0);
        let screen_pos = v2(200.0, 300.0);
        let camera = Camera::new(Vec2::splat(500.0) + offset, Vec2::splat(1000.0));

        assert_eq!(
            v2(500.0, 600.0),
            map_screen_to_world(screen_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }

    #[test]
    fn map_from_screen_space_to_world_space_with_viewport() {
        let offset = Vec2::splat(300.0);
        let screen_pos = v2(200.0, 300.0);
        let mut camera = Camera::new(Vec2::splat(500.0) + offset, Vec2::splat(1000.0));
        camera.set_viewport(Rect::new(v2(0.5, 0.0), v2(0.5, 1.0)));

        assert_eq!(
            v2(-300.0, 600.0),
            map_screen_to_world(screen_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }

    #[test]
    fn map_from_world_space_to_screen_space() {
        let world_pos = v2(200.0, 300.0);
        let camera = Camera::new(Vec2::splat(500.0), Vec2::splat(1000.0));

        assert_eq!(
            v2(200.0, 300.0),
            map_world_to_screen(world_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }

    #[test]
    fn map_from_world_space_to_screen_space_with_offset() {
        let offset = Vec2::splat(300.0);
        let world_pos = v2(200.0, 300.0);
        let camera = Camera::new(Vec2::splat(500.0) + offset, Vec2::splat(1000.0));

        assert_eq!(
            v2(-100.0, 0.0),
            map_world_to_screen(world_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }

    #[test]
    fn map_from_world_space_to_screen_space_with_viewport() {
        let offset = Vec2::splat(300.0);
        let world_pos = v2(200.0, 300.0);
        let mut camera = Camera::new(Vec2::splat(500.0) + offset, Vec2::splat(1000.0));
        camera.set_viewport(Rect::new(v2(0.5, 0.0), v2(0.5, 1.0)));

        assert_eq!(
            v2(450.0, 0.0),
            map_world_to_screen(world_pos, &camera).ceil() // Ceil, because rounding error.
        );
    }
}
