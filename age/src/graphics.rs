use crate::renderer::Texture;

#[derive(Clone)]
pub struct Sprite {
    // texture: Texture,
}

impl Sprite {
    pub fn new(_width: u32, _height: u32) -> Self {
        Self {
            // texture: Texture {},
        }
    }

    pub(crate) fn _texture(&self) -> &Texture {
        // &self.texture
        todo!()
    }
}
