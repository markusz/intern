/// Default slide dimensions (standard 16:9 widescreen), used when a deck's
/// `<p:sldSz>` element is missing or unreadable.
pub const DEFAULT_SLIDE_WIDTH_EMU: i64 = 12_192_000;
pub const DEFAULT_SLIDE_HEIGHT_EMU: i64 = 6_858_000;
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
    /// A genuine text box: a `<p:sp>` with `txBox="1"` on its `<p:cNvSpPr>`.
    TextBox,
    /// A non-text-box autoshape (rectangle, callout, ...); may still carry text.
    Autoshape,
    Image,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParagraphKind {
    /// Paragraph carries bullet formatting - either explicitly via `<a:buChar>` /
    /// `<a:buAutoNum>`, or by inheriting from a Body placeholder's layout default.
    Bullet,
    /// Paragraph has no bullet - either suppressed via `<a:buNone>`, or an
    /// autoshape / text box paragraph with no explicit bullet marker.
    Plain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub text: String,
    pub kind: ParagraphKind,
}

impl std::fmt::Display for ElementKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Title => "Title",
            Self::Body => "Body",
            Self::TextBox => "TextBox",
            Self::Autoshape => "Autoshape",
            Self::Image => "Image",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct SlideElement {
    /// Per-slide shape id from `<p:cNvPr id="...">`. Unique within a slide.
    pub id: u32,
    pub name: String,
    pub kind: ElementKind,
    pub rect: Rect,
    /// Hundredths of a point (e.g. 4400 = 44pt)
    pub font_size: Option<u32>,
    /// Font family name (e.g. "Calibri"). None if inherited from theme or not detected.
    pub font_family: Option<String>,
    /// Dominant text color as hex RGB (e.g. "FF0000"). None if not set or inherited.
    pub text_color: Option<String>,
    /// Non-empty paragraphs, in document order.
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone)]
pub struct SlideData {
    pub index: usize, // 0-based
    pub elements: Vec<SlideElement>,
}

/// A parsed presentation: its slides plus the deck's actual slide dimensions
/// (read from `<p:sldSz>`).
#[derive(Debug, Clone)]
pub struct Presentation {
    pub slides: Vec<SlideData>,
    pub slide_width: i64,
    pub slide_height: i64,
}
