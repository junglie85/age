use std::fmt::Debug;

use crate::{Image, Rect};

#[derive(Debug, Clone, Copy)]
pub struct PackerInfo {
    pub row_padding: u32,
    pub spacing: u32,
    pub max_size: u32,
    pub bytes_per_entry: u32,
}

impl PackerInfo {
    pub const MAX_SIZE: u32 = 512;
}

impl Default for PackerInfo {
    fn default() -> Self {
        Self {
            row_padding: 1,
            spacing: 1,
            max_size: Self::MAX_SIZE,
            bytes_per_entry: 4,
        }
    }
}

pub struct TexturePacker<T: Eq + Debug> {
    info: PackerInfo,
    dirty: bool,
    entries: Vec<Entry<T>>,
    buf: Vec<u8>,
    pages: Vec<Image>,
}

impl<T: Eq + Debug> TexturePacker<T> {
    pub fn new(info: &PackerInfo) -> Self {
        let size = info.max_size * info.max_size;
        Self {
            info: *info,
            dirty: false,
            entries: Vec::with_capacity((size / 32) as usize),
            buf: Vec::with_capacity((size * info.bytes_per_entry) as usize),
            pages: Vec::new(),
        }
    }

    pub fn add(&mut self, id: T, width: u32, height: u32, data: &[u8]) {
        if data.is_empty() {
            // Nothing to add.
            return;
        }

        self.dirty = true;

        let entry = Entry {
            id,
            width,
            height,
            bytes_per_pixel: data.len() as u32 / (width * height),
            from: self.buf.len(),
            to: self.buf.len() + data.len(),
            page: 0,
            tex_rect: Rect::default(),
        };

        self.entries.push(entry);
        self.buf.extend_from_slice(data);
    }

    pub fn pack(&mut self) {
        // todo: We can make this simpler by building up the vec of rows, then
        // splitting the vec into pages. Assign row to entry on first pass,
        // and assign the page to the entry on a second pass.

        fn find_suitable_row(
            width: usize,
            height: usize,
            max_size: usize,
            rows: &[Row],
        ) -> Option<usize> {
            for (i, row) in rows.iter().enumerate() {
                if height <= row.height && row.width + width <= max_size {
                    return Some(i);
                }
            }

            None
        }

        fn find_suitable_page(
            last_page: usize,
            row_height: usize,
            rows: &[Row],
            max_size: usize,
        ) -> Option<(usize, usize)> {
            for page in 0..=last_page {
                let top = rows
                    .iter()
                    .filter(|r| r.page == page)
                    .map(|r| r.height)
                    .sum();
                if top + row_height <= max_size {
                    return Some((page, top));
                }
            }

            None
        }

        fn get_or_insert_row<'rows>(
            rows: &'rows mut Vec<Row>,
            width: usize,
            height: usize,
            row_padding: usize,
            max_size: usize,
            last_page: &mut usize,
        ) -> &'rows mut Row {
            match find_suitable_row(width, height, max_size, rows) {
                Some(index) => &mut rows[index],
                None => {
                    let row_height = height + row_padding;
                    let (page, top) =
                        match find_suitable_page(*last_page, row_height, rows, max_size) {
                            Some((page, top)) => (page, top),
                            None => {
                                *last_page += 1;
                                (*last_page, 0)
                            }
                        };

                    rows.push(Row {
                        page,
                        top,
                        width: 0,
                        height: row_height,
                    });

                    let index = rows.len() - 1;
                    &mut rows[index]
                }
            }
        }

        self.pages.clear();

        let mut rows: Vec<Row> = Vec::new();
        let mut last_page = 0;

        for entry in self.entries.iter_mut() {
            let width = entry.width as usize + (self.info.spacing * 2) as usize;
            let height = entry.height as usize + (self.info.spacing * 2) as usize;

            let row = get_or_insert_row(
                &mut rows,
                width,
                height,
                self.info.row_padding as usize,
                self.info.max_size as usize,
                &mut last_page,
            );

            entry.page = row.page;
            entry.tex_rect.position.x = row.width as f32 + self.info.spacing as f32;
            entry.tex_rect.position.y = row.top as f32 + self.info.spacing as f32;
            entry.tex_rect.size.x = entry.width as f32;
            entry.tex_rect.size.y = entry.height as f32;

            row.width += width;

            if entry.page >= self.pages.len() {
                let size = self.info.max_size;
                self.pages.push(Image::from_pixels(
                    size,
                    size,
                    bytemuck::cast_slice(&vec![
                        0u8;
                        (size * size * entry.bytes_per_pixel) as usize
                    ]),
                ));
            }

            let image_width = self.info.max_size as usize * self.info.bytes_per_entry as usize;
            let data_width = entry.width as usize * self.info.bytes_per_entry as usize;
            let image = &mut self.pages[entry.page];
            let data = &self.buf[entry.from..entry.to];
            for y in 0..entry.height as usize {
                let image_advance = (entry.tex_rect.position.y as usize + y) * image_width;
                let data_advance = y * data_width;
                for x in 0..data_width {
                    let to = image_advance + entry.tex_rect.position.x as usize + x;
                    let from = data_advance + x;
                    image[to] = data[from];
                }
            }
        }
    }

    pub fn pages(&self) -> &[Image] {
        &self.pages
    }

    pub fn entries(&self) -> &[Entry<T>] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.dirty = false;
        self.entries.clear();
        self.buf.clear();
        self.pages.clear();
    }
}

#[derive(Debug)]
pub struct Entry<T: Eq + Debug> {
    pub id: T,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    from: usize,
    to: usize,
    pub page: usize,
    pub tex_rect: Rect,
}

#[derive(Debug)]
struct Row {
    page: usize,
    top: usize,
    width: usize,
    height: usize,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pack_single_bitmap() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 4;
        let height = 2;
        let data = vec![1; width * height * info.bytes_per_entry as usize];
        packer.add(1, width as u32, height as u32, &data);

        packer.pack();

        let entries = packer.entries().iter().collect::<Vec<_>>();

        assert_eq!(1, entries.len());

        let entry = entries[0];
        assert_eq!(0, entry.page);
        assert_eq!(info.spacing, entry.tex_rect.position.x as u32);
        assert_eq!(info.spacing, entry.tex_rect.position.y as u32);
        assert_eq!(width as u32, entry.tex_rect.size.x as u32);
        assert_eq!(height as u32, entry.tex_rect.size.y as u32);

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }

    #[test]
    fn pack_two_bitmaps_vertically_on_same_page() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 4;
        let height = 2;
        packer.add(
            1,
            width as u32,
            height as u32,
            &vec![1; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            2,
            width as u32,
            height as u32,
            &vec![2; width * height * info.bytes_per_entry as usize],
        );

        packer.pack();

        let entries = packer.entries().iter().collect::<Vec<_>>();

        assert_eq!(2, entries.len());

        let entry0 = entries[0];
        assert_eq!(0, entry0.page);
        assert_eq!(info.spacing, entry0.tex_rect.position.x as u32);
        assert_eq!(info.spacing, entry0.tex_rect.position.y as u32);
        assert_eq!(width as u32, entry0.tex_rect.size.x as u32);
        assert_eq!(height as u32, entry0.tex_rect.size.y as u32);

        let gap = info.row_padding + info.spacing * 2;
        let entry1 = entries[1];
        assert_eq!(0, entry1.page);
        assert_eq!(info.spacing, entry1.tex_rect.position.x as u32);
        assert_eq!(
            entry0.tex_rect.position.y + entry0.tex_rect.size.y + gap as f32,
            entry1.tex_rect.position.y
        );
        assert_eq!(width as u32, entry1.tex_rect.size.x as u32);
        assert_eq!(
            entry0.tex_rect.position.y + entry0.tex_rect.size.y + gap as f32 + height as f32,
            entry1.tex_rect.position.y + entry1.tex_rect.size.y
        );

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 0, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }

    #[test]
    fn pack_two_bitmaps_horizontally_on_same_page() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 3;
        let height = 2;
        packer.add(
            1,
            width as u32,
            height as u32,
            &vec![1; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            2,
            width as u32,
            height as u32,
            &vec![2; width * height * info.bytes_per_entry as usize],
        );

        packer.pack();

        let entries = packer.entries().iter().collect::<Vec<_>>();

        assert_eq!(2, entries.len());

        let entry0 = entries[0];
        assert_eq!(0, entry0.page);
        assert_eq!(info.spacing, entry0.tex_rect.position.x as u32);
        assert_eq!(info.spacing, entry0.tex_rect.position.y as u32);
        assert_eq!(width as u32, entry0.tex_rect.size.x as u32);
        assert_eq!(height as u32, entry0.tex_rect.size.y as u32);

        let gap = info.spacing * 2;
        let entry1 = entries[1];
        assert_eq!(0, entry1.page);
        assert_eq!(
            entry0.tex_rect.position.x + entry0.tex_rect.size.x + gap as f32,
            entry1.tex_rect.position.x
        );
        assert_eq!(info.spacing, entry1.tex_rect.position.y as u32);
        assert_eq!(
            entry0.tex_rect.position.x + entry0.tex_rect.size.x + gap as f32 + width as f32,
            entry1.tex_rect.position.x + entry1.tex_rect.size.x
        );
        assert_eq!(height as u32, entry1.tex_rect.size.y as u32);

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 0, 0, 2, 2, 2, 0,
            0, 1, 1, 1, 0, 0, 2, 2, 2, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }

    #[test]
    fn pack_four_bitmaps_vertically_and_horizontally_on_same_page() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 3;
        let height = 2;
        packer.add(
            1,
            width as u32,
            height as u32,
            &vec![1; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            2,
            width as u32,
            height as u32,
            &vec![2; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            3,
            width as u32,
            height as u32,
            &vec![3; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            4,
            width as u32,
            height as u32,
            &vec![4; width * height * info.bytes_per_entry as usize],
        );

        packer.pack();

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 0, 0, 2, 2, 2, 0,
            0, 1, 1, 1, 0, 0, 2, 2, 2, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 3, 3, 0, 0, 4, 4, 4, 0,
            0, 3, 3, 3, 0, 0, 4, 4, 4, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }

    #[test]
    fn pack_three_bitmaps_on_row1_row2_row1() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 3;
        let height = 2;
        packer.add(
            1,
            width as u32,
            height as u32,
            &vec![1; width * height * info.bytes_per_entry as usize],
        );
        packer.add(2, 5, 2, &vec![2; 5 * 2 * info.bytes_per_entry as usize]);
        packer.add(
            3,
            width as u32,
            height as u32,
            &vec![3; width * height * info.bytes_per_entry as usize],
        );

        packer.pack();

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 0, 0, 3, 3, 3, 0,
            0, 1, 1, 1, 0, 0, 3, 3, 3, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }

    #[test]
    fn pack_two_bitmaps_on_page1_and_page2() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 5;
        let height = 5;
        packer.add(
            1,
            width as u32,
            height as u32,
            &vec![1; width * height * info.bytes_per_entry as usize],
        );
        packer.add(
            2,
            width as u32,
            height as u32,
            &vec![2; width * height * info.bytes_per_entry as usize],
        );

        packer.pack();

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(2, pages.len());

        let page1 = pages[0];
        let page2 = pages[1];

        #[rustfmt::skip]
        let expected1: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        #[rustfmt::skip]
        let expected2: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual1 = page1.pixels().to_vec();
        let actual2 = page2.pixels().to_vec();

        assert_eq!(expected1.len(), actual1.len());
        assert_eq!(&expected1, &actual1.as_slice());
        assert_eq!(expected2.len(), actual2.len());
        assert_eq!(&expected2, &actual2.as_slice());
    }

    #[test]
    fn pack_three_bitmaps_on_page1_page2_page1() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 1,
        };
        let mut packer = TexturePacker::new(&info);

        packer.add(1, 5, 2, &vec![1; 5 * 2 * info.bytes_per_entry as usize]);
        packer.add(2, 5, 7, &vec![2; 5 * 7 * info.bytes_per_entry as usize]);
        packer.add(3, 5, 2, &vec![3; 5 * 2 * info.bytes_per_entry as usize]);

        packer.pack();

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(2, pages.len());

        let page1 = pages[0];
        let page2 = pages[1];

        #[rustfmt::skip]
        let expected1: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 3, 3, 3, 3, 0, 0, 0, 0,
            0, 3, 3, 3, 3, 3, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        #[rustfmt::skip]
        let expected2: [u8; 100] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 2, 2, 2, 2, 2, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual1 = page1.pixels().to_vec();
        let actual2 = page2.pixels().to_vec();

        assert_eq!(expected1.len(), actual1.len());
        assert_eq!(&expected1, &actual1.as_slice());
        assert_eq!(expected2.len(), actual2.len());
        assert_eq!(&expected2, &actual2.as_slice());
    }

    #[test]
    fn pack_single_bitmap_with_multiple_channels() {
        let info = PackerInfo {
            row_padding: 1,
            spacing: 1,
            max_size: 10,
            bytes_per_entry: 2,
        };
        let mut packer = TexturePacker::new(&info);

        let width = 4;
        let height = 2;
        let data = vec![[1, 2]; width * height]
            .into_iter()
            .flat_map(|i| i.into_iter())
            .collect::<Vec<_>>();
        packer.add(1, width as u32, height as u32, &data);

        packer.pack();

        let entries = packer.entries().iter().collect::<Vec<_>>();

        assert_eq!(1, entries.len());

        let entry = entries[0];
        assert_eq!(0, entry.page);
        assert_eq!(info.spacing, entry.tex_rect.position.x as u32);
        assert_eq!(info.spacing, entry.tex_rect.position.y as u32);
        assert_eq!(width as u32, entry.tex_rect.size.x as u32);
        assert_eq!(height as u32, entry.tex_rect.size.y as u32);

        let pages = packer.pages().iter().collect::<Vec<_>>();

        assert_eq!(1, pages.len());

        let page = pages[0];

        #[rustfmt::skip]
        let expected: [u8; 200] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 2, 1, 2, 1, 2, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 2, 1, 2, 1, 2, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let actual = page.pixels().to_vec();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(&expected, &actual.as_slice());
    }
}
