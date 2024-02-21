use crate::{
    renderer::{DrawCommand, DrawTarget, GeometryVertex, Texture},
    Color, Engine,
};

impl Engine {
    pub fn clear(&mut self, color: Color) {
        self.clear_color = Some(color);
        self.needs_render_pass = true;
        self.push_render_pass();
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite) {
        self.push_draw_command(DrawCommand {
            vertices: sprite.vertices().to_vec(),
            indices: sprite.indices().to_vec(),
        });
    }

    pub fn set_draw_target<T: Into<DrawTarget>>(&mut self, target: T) {
        self.draw_target = target.into();
        self.clear_color = None;
        self.needs_render_pass = true;
    }

    fn push_draw_command(&mut self, draw: DrawCommand) {
        if self.needs_render_pass {
            self.push_render_pass();
        }

        self.draws.record(draw);
    }

    fn push_render_pass(&mut self) {
        self.needs_render_pass = false;
        self.draws
            .set_render_pass(self.draw_target.texture_view(), self.clear_color);
    }
}

#[derive(Clone)]
pub struct Sprite {
    width: u32,
    height: u32,
    // texture: Texture,
}

impl Sprite {
    const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
    const VERTICES: [GeometryVertex; 4] = [
        GeometryVertex { pos: [0.0, 0.0] },
        GeometryVertex { pos: [0.0, 0.5] },
        GeometryVertex { pos: [0.5, 0.5] },
        GeometryVertex { pos: [0.5, 0.0] },
    ];

    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height, // texture: Texture {},
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn indices(&self) -> &[u16] {
        &Self::INDICES
    }

    pub(crate) fn _texture(&self) -> &Texture {
        // &self.texture
        todo!()
    }

    pub(crate) fn vertices(&self) -> &[GeometryVertex] {
        &Self::VERTICES
    }

    pub fn width(&self) -> u32 {
        self.width
    }
}
