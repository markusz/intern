pub const SLIDE_WIDTH_EMU: i64 = 9_144_000;
pub const SLIDE_HEIGHT_EMU: i64 = 6_858_000;
pub const EMU_PER_PX: i64 = 9_525; // 96 DPI

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    pub fn right(&self) -> i64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i64 {
        self.y + self.h
    }
    pub fn cx(&self) -> i64 {
        self.x + self.w / 2
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    Title,
    Body,
    TextBox,
    Image,
}

#[derive(Debug, Clone)]
pub struct SlideElement {
    pub name: String,
    pub kind: ElementKind,
    pub rect: Rect,
    /// Hundredths of a point (e.g. 4400 = 44pt)
    pub font_size: Option<u32>,
}

#[derive(Debug)]
pub struct SlideData {
    pub index: usize, // 0-based
    pub elements: Vec<SlideElement>,
}
