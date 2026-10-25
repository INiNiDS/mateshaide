use crate::export::ExportError;
use crate::export::bounds::Rect;
use glam::Vec2;
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, RawImage, RawImageData, RawImageFormat, XObjectTransform};

pub const PAGE_WIDTH_MM: f32 = 210.0;
pub const PAGE_HEIGHT_MM: f32 = 297.0;

const PAGE_W: f32 = 794.0;
pub const PAGE_H: f32 = 1123.0;

pub const RASTER_SCALE: f32 = 2.0;
pub const RASTER_W: u32 = (PAGE_W * RASTER_SCALE) as u32;
pub const RASTER_H: u32 = (PAGE_H * RASTER_SCALE) as u32;

pub const MAX_PAGES: usize = 200;

pub fn page_tiles(content: Rect, items: &[Rect]) -> Result<Vec<Rect>, ExportError> {
    let size = content.size();
    let cols = ((size.x / PAGE_W).ceil() as usize).max(1);
    let rows = ((size.y / PAGE_H).ceil() as usize).max(1);

    let page_size = Vec2::new(PAGE_W, PAGE_H);
    let mut tiles = Vec::with_capacity(rows * cols);

    for row in 0..rows {
        for col in 0..cols {
            let top = content.max.y - row as f32 * PAGE_H;
            let min = Vec2::new(content.min.x + col as f32 * PAGE_W, top - PAGE_H);
            let tile = Rect {
                min,
                max: min + page_size,
            };

            if items.iter().any(|item| item.intersects(tile)) {
                tiles.push(tile);
            }
        }
    }
    Ok(tiles)
}

pub struct PdfBuilder {
    doc: PdfDocument,
    pages: Vec<PdfPage>,
}

impl PdfBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            doc: PdfDocument::new("MateshaIDE"),
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, rgba: Vec<u8>) -> Result<(), ExportError> {
        if self.pages.len() >= MAX_PAGES {
            return Err(ExportError::TooManyPages(self.pages.len() + 1));
        }

        let rgb = rgba
            .chunks_exact(4)
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect::<Vec<u8>>();

        let raw = RawImage {
            pixels: RawImageData::U8(rgb),
            width: RASTER_W as usize,
            height: RASTER_H as usize,
            data_format: RawImageFormat::RGB8,
            tag: vec![],
        };
        let id = self.doc.add_image(&raw);

        let dpi = RASTER_W as f32 / (PAGE_WIDTH_MM / 25.4);
        let ops = vec![Op::UseXobject {
            id,
            transform: XObjectTransform {
                dpi: Some(dpi),
                ..Default::default()
            }
        }];

        self.pages
            .push(PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops));
        Ok(())
    }

    pub fn finish(mut self) -> Result<Vec<u8>, ExportError> {
        if self.pages.is_empty() {
            return Err(ExportError::Empty);
        }
        self.doc.pages = self.pages;

        let mut warnings = Vec::new();
        Ok(self.doc.save(&PdfSaveOptions::default(), &mut warnings))
    }
}