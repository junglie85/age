use std::ops::{Index, IndexMut};

use image::ImageError;

use crate::{AgeError, AgeResult};

pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    pub fn from_bytes(data: &[u8]) -> AgeResult<Self> {
        let image = image::load_from_memory(data)?.into_rgba8();
        let (width, height) = image.dimensions();
        let pixels = image.to_vec();

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn from_pixels(width: u32, height: u32, pixels: &[u8]) -> Self {
        Self {
            width,
            height,
            pixels: pixels.to_vec(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl Index<usize> for Image {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.pixels[index]
    }
}

impl IndexMut<usize> for Image {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.pixels[index]
    }
}

impl From<ImageError> for AgeError {
    fn from(err: ImageError) -> Self {
        AgeError::new("an image processing error occurred").with_source(err)
    }
}
