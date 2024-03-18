use std::ops::Range;

use age_math::{v2, Vec2};
use fontdue::FontSettings;

use crate::{
    packer::{PackerInfo, TexturePacker},
    AgeResult, BindGroup, BindGroupLayout, Binding, Rect, RenderDevice, Sampler, Texture,
    TextureFormat, TextureInfo, TextureViewInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharSet {
    pub from: i32,
    pub to: i32,
}

impl CharSet {
    pub const ASCII: Self = Self::new(0..128);

    pub const fn new(range: Range<i32>) -> Self {
        Self {
            from: range.start,
            to: range.end,
        }
    }
}

pub struct Font {
    font: fontdue::Font,
}

impl Font {
    pub fn from_bytes(data: &[u8]) -> AgeResult<Self> {
        let settings = FontSettings::default();
        let font = fontdue::Font::from_bytes(data, settings)?;

        Ok(Font { font })
    }

    pub fn load_charset(
        &mut self,
        size: f32,
        charset: CharSet,
        layout: &BindGroupLayout,
        sampler: &Sampler,
        device: &RenderDevice,
    ) -> AgeResult<SpriteFont> {
        let (from, to) = match (
            std::char::from_u32(charset.from as u32),
            std::char::from_u32(charset.to as u32),
        ) {
            (Some(from), Some(to)) => (from, to),
            _ => return Err("invalid codepoint".into()), // todo: better error
        };

        let line_metrics = match self.font.horizontal_line_metrics(size) {
            Some(line_metrics) => line_metrics,
            None => return Err("failed to load line metrics".into()),
        };

        let ascent = f32::ceil(line_metrics.ascent);
        let descent = f32::ceil(line_metrics.descent);
        let line_gap = f32::ceil(line_metrics.line_gap);
        let line_height = f32::ceil(line_metrics.new_line_size);

        let max_size = PackerInfo::MAX_SIZE.min(device.limits().max_texture_dimension_2d);
        let mut packer = TexturePacker::new(&PackerInfo {
            max_size,
            bytes_per_entry: 1,
            ..Default::default()
        });

        let mut glyphs = Vec::new();
        let mut kernings = Vec::new();
        let mut textures = Vec::new();
        let mut bgs = Vec::new();

        for c in from..to {
            if !self.font.has_glyph(c) {
                continue;
            }

            let (metrics, bitmap) = self.font.rasterize(c, size);

            glyphs.push(Glyph {
                codepoint: c,
                size,
                advance: f32::ceil(metrics.advance_width),
                offset: v2(
                    f32::floor(metrics.bounds.xmin),
                    f32::floor(-metrics.bounds.height - metrics.bounds.ymin),
                ),
                ..Default::default()
            });

            packer.add(c, metrics.width as u32, metrics.height as u32, &bitmap);
        }

        packer.pack();

        for page in packer.pages() {
            let texture = device.create_texture(&TextureInfo {
                label: Some("font"),
                width: max_size,
                height: max_size,
                format: TextureFormat::R8Unorm,
                ..Default::default()
            });
            device.write_texture(&texture, page.pixels());

            let view = texture.create_view(&TextureViewInfo {
                label: Some("font"),
            });

            let bg = device.create_bind_group(&crate::BindGroupInfo {
                label: Some("font"),
                layout,
                entries: &[
                    Binding::Sampler { sampler },
                    Binding::Texture {
                        texture_view: &view,
                    },
                ],
            });

            textures.push(texture);
            bgs.push(bg);
        }

        for entry in packer.entries() {
            if let Some(index) = glyphs.iter().position(|g| entry.id == g.codepoint) {
                let glyph = &mut glyphs[index];
                glyph.texture = entry.page;

                let texture = &textures[glyph.texture];
                let tex_size = v2(texture.width() as f32, texture.height() as f32);
                glyph.tex_rect = Rect::new(
                    entry.tex_rect.position / tex_size,
                    entry.tex_rect.size / tex_size,
                );
            }
        }

        for a in glyphs.iter().map(|g| g.codepoint) {
            for b in glyphs.iter().map(|g| g.codepoint) {
                if let Some(value) = self.font.horizontal_kern(a, b, size) {
                    kernings.push(Kerning { a, b, value })
                }
            }
        }

        Ok(SpriteFont {
            size,
            ascent,
            descent,
            line_gap,
            line_height,
            glyphs,
            kernings,
            textures,
            bgs,
        })
    }
}

#[derive(Debug, Default)]
pub struct Glyph {
    pub codepoint: char,
    pub size: f32,
    pub advance: f32,
    pub offset: Vec2,
    pub texture: usize,
    pub tex_rect: Rect,
}

#[derive(Debug, Default)]
struct Kerning {
    a: char,
    b: char,
    value: f32,
}

#[derive(Default)]
pub struct SpriteFont {
    size: f32,
    ascent: f32,
    descent: f32,
    line_gap: f32,
    line_height: f32,
    glyphs: Vec<Glyph>,
    kernings: Vec<Kerning>,
    textures: Vec<Texture>,
    bgs: Vec<BindGroup>,
}

impl SpriteFont {
    pub fn advance(&self, codepoint: char) -> f32 {
        let mut result = 0.0;

        if let Some(index) = self.glyphs.iter().position(|g| g.codepoint == codepoint) {
            result = self.glyphs[index].advance;
        }

        result
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn kerning(&self, a: char, b: char) -> f32 {
        let mut result = 0.0;

        if self.kernings.is_empty() {
            return result;
        }

        let mut lower = 0;
        let mut higher = self.kernings.len() - 1;

        while lower <= higher {
            let mid = (higher + lower) / 2;
            let kerning = &self.kernings[mid];
            if kerning.a == a && kerning.b == b {
                result = kerning.value;
                break;
            }

            #[allow(clippy::comparison_chain)]
            if kerning.a == a {
                if kerning.b < b {
                    lower = mid + 1;
                } else {
                    higher = mid - 1;
                }
            } else if kerning.a < a {
                lower = mid + 1;
            } else {
                higher = mid - 1;
            }
        }

        result
    }

    pub fn glyph(&self, codepoint: char) -> Option<&Glyph> {
        if let Some(index) = self.glyphs.iter().position(|g| g.codepoint == codepoint) {
            Some(&self.glyphs[index])
        } else {
            None
        }
    }

    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    pub fn descent(&self) -> f32 {
        self.descent
    }

    pub fn line_gap(&self) -> f32 {
        self.line_gap
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn height(&self) -> f32 {
        self.ascent - self.descent
    }

    pub fn width_of(&self, text: &str) -> f32 {
        let mut width = 0.0;
        let mut line_width = 0.0;
        let mut last = None;

        for c in text.chars() {
            if c == '\n' {
                line_width = 0.0;
            } else {
                line_width += self.advance(c);
                if let Some(last) = last {
                    line_width += self.kerning(last, c);
                }
                if line_width > width {
                    width = line_width;
                }
                last = Some(c);
            }
        }

        width
    }

    pub fn width_of_line(&self, text: &str, start: usize) -> f32 {
        if start >= text.len() {
            return 0.0;
        }

        let mut width = 0.0;
        let mut last = None;

        for c in text[start..].chars() {
            if c == '\n' {
                break;
            }

            width += self.advance(c);
            if let Some(last) = last {
                width += self.kerning(last, c);
            }
            last = Some(c);
        }

        width
    }

    pub fn height_of(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        let mut height = self.line_height;

        for c in text.chars() {
            if c == '\n' {
                height += self.line_height;
            }
        }

        height - self.line_gap
    }

    pub fn texture(&self, glyph: &Glyph) -> &Texture {
        assert!(glyph.size == self.size);
        &self.textures[glyph.texture]
    }

    pub fn bind_group(&self, glyph: &Glyph) -> &BindGroup {
        assert!(glyph.size == self.size);
        &self.bgs[glyph.texture]
    }
}
