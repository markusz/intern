pub mod columns;
pub mod grid;
pub mod layout;
pub mod text;
pub mod title;

use std::fmt;

/// Returns the median of `values`, or `None` if empty.
/// Sorts a copy, then picks the middle index. For even-length inputs this
/// returns the upper-middle element, which makes alignment thresholds
/// slightly stricter (rounds up rather than down).
pub(crate) fn median(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    let mid = sorted.len() / 2;
    Some(sorted[mid])
}

use crate::model::SlideData;

/// A concrete edit to apply to a PPTX file. All coordinates are in EMU.
#[derive(Debug, Clone)]
pub enum Fix {
    SetX {
        slide_idx: usize,
        element_name: String,
        x: i64,
    },
    SetY {
        slide_idx: usize,
        element_name: String,
        y: i64,
    },
    /// Set X and/or width (cx). `None` means "leave unchanged".
    SetXW {
        slide_idx: usize,
        element_name: String,
        x: Option<i64>,
        w: Option<i64>,
    },
    /// Font size in hundredths of a point (e.g. 4400 = 44pt).
    SetFontSize {
        slide_idx: usize,
        element_name: String,
        size: u32,
    },
    /// Collapse consecutive spaces and trim leading/trailing whitespace in all text runs.
    NormalizeWhitespace {
        slide_idx: usize,
        element_name: String,
    },
}

impl Fix {
    pub fn slide_idx(&self) -> usize {
        match self {
            Fix::SetX { slide_idx, .. }
            | Fix::SetY { slide_idx, .. }
            | Fix::SetXW { slide_idx, .. }
            | Fix::SetFontSize { slide_idx, .. }
            | Fix::NormalizeWhitespace { slide_idx, .. } => *slide_idx,
        }
    }

    pub fn element_name(&self) -> &str {
        match self {
            Fix::SetX { element_name, .. }
            | Fix::SetY { element_name, .. }
            | Fix::SetXW { element_name, .. }
            | Fix::SetFontSize { element_name, .. }
            | Fix::NormalizeWhitespace { element_name, .. } => element_name,
        }
    }
}

impl fmt::Display for Fix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const EPX: f64 = 9525.0;
        let (slide, name) = (self.slide_idx() + 1, self.element_name());
        match self {
            Fix::SetX { x, .. } => write!(
                f,
                "slide {slide} '{name}': set X → {:.1}px",
                *x as f64 / EPX
            ),
            Fix::SetY { y, .. } => write!(
                f,
                "slide {slide} '{name}': set Y → {:.1}px",
                *y as f64 / EPX
            ),
            Fix::SetXW { x, w, .. } => {
                let mut parts: Vec<String> = Vec::new();
                if let Some(x) = x {
                    parts.push(format!("X → {:.1}px", *x as f64 / EPX));
                }
                if let Some(w) = w {
                    parts.push(format!("W → {:.1}px", *w as f64 / EPX));
                }
                write!(f, "slide {slide} '{name}': set {}", parts.join(", "))
            }
            Fix::SetFontSize { size, .. } => {
                write!(
                    f,
                    "slide {slide} '{name}': set font size → {}pt",
                    size / 100
                )
            }
            Fix::NormalizeWhitespace { .. } => {
                write!(f, "slide {slide} '{name}': normalize whitespace")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

/// Structured violation detail. Each variant carries the raw EMU (or point) values
/// so callers can inspect them programmatically; `Display` renders the human-readable form.
#[derive(Debug, Clone)]
pub enum ViolationMessage {
    /// An element's edge deviates from its peers by `diff_emu`.
    EdgeOff { diff_emu: i64 },
    /// Left and right column top edges are misaligned by `diff_emu`.
    ColumnTopMisaligned { diff_emu: i64 },
    /// A gap between two elements is `actual_emu`; the expected value is `expected_emu`.
    GapUneven { actual_emu: i64, expected_emu: i64 },
    /// Title Y position is `actual_emu`; the majority value is `expected_emu`.
    TitleYOff { actual_emu: i64, expected_emu: i64 },
    /// Title X and/or width deviate from peers. `None` means that dimension is within threshold.
    TitlePositionSize {
        x_off_emu: Option<i64>,
        w_off_emu: Option<i64>,
    },
    /// Title font size (in hundredths of a point) differs from the majority.
    TitleFontSize { actual: u32, expected: u32 },
    /// Body/textbox font size (in hundredths of a point) differs from the majority.
    BodyFontSize { actual: u32, expected: u32 },
    /// Body/textbox font family differs from the majority.
    BodyFontFamily { actual: String, expected: String },
    /// Element rect extends outside the slide bounds.
    ElementOverflow,
    /// Body/textbox text color differs from the majority across slides.
    BodyTextColor { actual: String, expected: String },
    /// Paragraph contains two or more consecutive spaces.
    DoubleSpace,
    /// Paragraph has leading or trailing whitespace.
    TrailingSpace,
    /// Bullet paragraphs within an element have inconsistent first-letter capitalization.
    BulletCapitalization { expected_uppercase: bool },
    /// Slide has no title element.
    TitleMissing,
    /// Body or textbox element has no text content.
    EmptyElement,
    /// Paragraph text is ALL CAPS.
    AllCaps,
    /// Bullet ending punctuation is inconsistent with the deck majority.
    BulletPunctuation { expected_punctuation: bool },
    /// Bullet paragraph exceeds the word limit.
    BulletTooLong { word_count: usize, limit: usize },
    /// Title text is duplicated on another slide.
    DuplicateTitle { first_slide: usize },
    /// Title text exceeds the word limit.
    TitleTooLong { word_count: usize, limit: usize },
    /// Title ends with sentence-ending punctuation.
    TitleTrailingPunct { punct: char },
    /// Paragraph contains two consecutive identical words.
    RepeatedWord { word: String },
    /// Deck uses more distinct font families than the limit.
    FontVariety { count: usize, limit: usize },
    /// Deck uses more distinct text colors than the limit.
    ColorVariety { count: usize, limit: usize },
    /// Deck has more slides than the limit.
    SlideCount { count: usize, limit: usize },
    /// Element rect overlaps with another element on the same slide.
    ElementOverlap { other_element: String },
    /// Image aspect ratio differs from the deck majority.
    ImageAspectRatio {
        actual_ratio: f64,
        expected_ratio: f64,
    },
}

const EMU_PER_PX: f64 = 9525.0;

impl fmt::Display for ViolationMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EdgeOff { diff_emu } => {
                write!(
                    f,
                    "edge {:.1}px off from peers",
                    *diff_emu as f64 / EMU_PER_PX
                )
            }
            Self::ColumnTopMisaligned { diff_emu } => write!(
                f,
                "left/right column top edges misaligned by {:.1}px",
                *diff_emu as f64 / EMU_PER_PX
            ),
            Self::GapUneven {
                actual_emu,
                expected_emu,
            } => write!(
                f,
                "gap {:.1}px, expected ~{:.1}px",
                *actual_emu as f64 / EMU_PER_PX,
                *expected_emu as f64 / EMU_PER_PX,
            ),
            Self::TitleYOff {
                actual_emu,
                expected_emu,
            } => {
                let diff = (*actual_emu - *expected_emu) as f64 / EMU_PER_PX;
                if diff > 0.0 {
                    write!(f, "title is {diff:.1}px lower than on most slides")
                } else {
                    write!(f, "title is {:.1}px higher than on most slides", diff.abs())
                }
            }
            Self::TitlePositionSize {
                x_off_emu,
                w_off_emu,
            } => {
                let mut parts: Vec<String> = Vec::new();
                if let Some(d) = x_off_emu {
                    parts.push(format!("X {:.1}px off", *d as f64 / EMU_PER_PX));
                }
                if let Some(d) = w_off_emu {
                    parts.push(format!("width {:.1}px off", *d as f64 / EMU_PER_PX));
                }
                write!(f, "title position/size inconsistent: {}", parts.join(", "))
            }
            Self::TitleFontSize { actual, expected } => write!(
                f,
                "title font size {}pt, expected {}pt",
                actual / 100,
                expected / 100,
            ),
            Self::BodyFontSize { actual, expected } => write!(
                f,
                "body font size {}pt, expected {}pt",
                actual / 100,
                expected / 100,
            ),
            Self::BodyFontFamily { actual, expected } => {
                write!(f, "body font '{actual}', expected '{expected}'")
            }
            Self::ElementOverflow => write!(f, "element extends outside slide bounds"),
            Self::BodyTextColor { actual, expected } => {
                write!(f, "text color #{actual}, most elements use #{expected}")
            }
            Self::DoubleSpace => write!(f, "paragraph contains double spaces"),
            Self::TrailingSpace => write!(f, "paragraph has leading or trailing whitespace"),
            Self::BulletCapitalization { expected_uppercase } => {
                if *expected_uppercase {
                    write!(
                        f,
                        "bullet starts lowercase; most bullets in this deck start uppercase"
                    )
                } else {
                    write!(
                        f,
                        "bullet starts uppercase; most bullets in this deck start lowercase"
                    )
                }
            }
            Self::TitleMissing => write!(f, "slide has no title element"),
            Self::EmptyElement => write!(f, "no text content"),
            Self::AllCaps => write!(f, "text is all caps - use title case or sentence case"),
            Self::BulletPunctuation {
                expected_punctuation,
            } => {
                if *expected_punctuation {
                    write!(
                        f,
                        "bullet ends without punctuation; most bullets in this deck end with punctuation"
                    )
                } else {
                    write!(
                        f,
                        "bullet ends with punctuation; most bullets in this deck end without"
                    )
                }
            }
            Self::BulletTooLong { word_count, limit } => {
                write!(f, "bullet is {word_count} words ({limit}-word limit)")
            }
            Self::DuplicateTitle { first_slide } => {
                write!(f, "same title as slide {first_slide}")
            }
            Self::TitleTooLong { word_count, limit } => {
                write!(f, "title is {word_count} words ({limit}-word limit)")
            }
            Self::TitleTrailingPunct { punct } => {
                write!(f, "title ends with '{punct}' - remove it")
            }
            Self::RepeatedWord { word } => write!(f, "'{word}' appears twice in a row"),
            Self::FontVariety { count, limit } => {
                write!(f, "deck uses {count} font families (max {limit})")
            }
            Self::ColorVariety { count, limit } => {
                write!(f, "deck uses {count} text colors (max {limit})")
            }
            Self::SlideCount { count, limit } => {
                write!(f, "deck has {count} slides (max {limit})")
            }
            Self::ElementOverlap { other_element } => {
                write!(f, "bounding box overlaps with '{other_element}'")
            }
            Self::ImageAspectRatio {
                actual_ratio,
                expected_ratio,
            } => write!(
                f,
                "image aspect ratio {actual_ratio:.2} differs from majority {expected_ratio:.2}"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub rule_id: &'static str,
    pub slide: Option<usize>, // 1-based
    pub element: Option<String>,
    pub message: ViolationMessage,
    pub severity: Severity,
    /// Proposed automatic fix, if one can be determined unambiguously.
    pub fix: Option<Fix>,
}

pub trait Rule {
    fn id(&self) -> &'static str;
    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation>;
}

/// Configurable numeric limits for rules that enforce "must be ≤ N" policies.
/// All fields have opinionated defaults matching common presentation standards.
pub struct Limits {
    /// Maximum words allowed in a slide title (default 10).
    pub title_words: usize,
    /// Maximum words allowed in a single bullet point (default 20).
    pub bullet_words: usize,
    /// Maximum distinct font families across the deck (default 2).
    pub font_families: usize,
    /// Maximum distinct text colors across the deck (default 3).
    pub text_colors: usize,
    /// Maximum number of slides in the deck (default 20).
    pub slide_count: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            title_words: 10,
            bullet_words: 20,
            font_families: 2,
            text_colors: 3,
            slide_count: 20,
        }
    }
}

pub fn all_rules(limits: &Limits) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(title::TitleYRule),
        Box::new(title::TitleXWidthRule),
        Box::new(title::TitleFontSizeRule),
        Box::new(columns::ColumnLeftEdgeRule),
        Box::new(columns::ColumnTopEdgeRule),
        Box::new(columns::ColumnRightLeftEdgeRule),
        Box::new(grid::GridHSpacingRule),
        Box::new(grid::GridVSpacingRule),
        Box::new(grid::GridRowTopRule),
        Box::new(grid::GridColLeftRule),
        Box::new(text::BodyFontSizeRule),
        Box::new(text::BodyFontFamilyRule),
        Box::new(text::BodyTextColorRule),
        Box::new(text::DoubleSpaceRule),
        Box::new(text::TrailingSpaceRule),
        Box::new(text::BulletCapitalizationRule),
        Box::new(text::AllCapsRule),
        Box::new(text::BulletPunctuationRule),
        Box::new(text::BulletLengthRule {
            limit: limits.bullet_words,
        }),
        Box::new(layout::TitlePresentRule),
        Box::new(layout::EmptyElementRule),
        Box::new(layout::ElementOverflowRule),
        Box::new(layout::ElementOverlapRule),
        Box::new(layout::ImageAspectRatioRule),
        Box::new(layout::SlideCountRule {
            limit: limits.slide_count,
        }),
        Box::new(title::DuplicateTitleRule),
        Box::new(title::TitleLengthRule {
            limit: limits.title_words,
        }),
        Box::new(title::TitleTrailingPunctRule),
        Box::new(text::RepeatedWordRule),
        Box::new(text::FontVarietyRule {
            limit: limits.font_families,
        }),
        Box::new(text::ColorVarietyRule {
            limit: limits.text_colors,
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violation_message_edge_off_display() {
        let m = ViolationMessage::EdgeOff { diff_emu: 95_250 };
        assert_eq!(m.to_string(), "edge 10.0px off from peers");
    }

    #[test]
    fn violation_message_column_top_misaligned_display() {
        let m = ViolationMessage::ColumnTopMisaligned { diff_emu: 19_050 };
        assert_eq!(
            m.to_string(),
            "left/right column top edges misaligned by 2.0px"
        );
    }

    #[test]
    fn violation_message_gap_uneven_display() {
        let m = ViolationMessage::GapUneven {
            actual_emu: 190_500,
            expected_emu: 95_250,
        };
        assert_eq!(m.to_string(), "gap 20.0px, expected ~10.0px");
    }

    #[test]
    fn violation_message_title_y_off_display() {
        let m = ViolationMessage::TitleYOff {
            actual_emu: 400_000,
            expected_emu: 274_638,
        };
        let s = m.to_string();
        assert!(s.contains("lower than on most slides"), "{s}");
    }

    #[test]
    fn violation_message_title_position_size_display() {
        let m = ViolationMessage::TitlePositionSize {
            x_off_emu: Some(95_250),
            w_off_emu: None,
        };
        assert!(m.to_string().contains("X 10.0px off"));
    }

    #[test]
    fn violation_message_title_font_size_display() {
        let m = ViolationMessage::TitleFontSize {
            actual: 3600,
            expected: 4400,
        };
        assert_eq!(m.to_string(), "title font size 36pt, expected 44pt");
    }

    #[test]
    fn fix_set_x_display() {
        let f = Fix::SetX {
            slide_idx: 0,
            element_name: "Box".into(),
            x: 95_250,
        };
        assert_eq!(f.to_string(), "slide 1 'Box': set X → 10.0px");
    }

    #[test]
    fn fix_set_y_display() {
        let f = Fix::SetY {
            slide_idx: 1,
            element_name: "Title".into(),
            y: 285_750,
        };
        assert_eq!(f.to_string(), "slide 2 'Title': set Y → 30.0px");
    }

    #[test]
    fn fix_set_xw_display_both() {
        let f = Fix::SetXW {
            slide_idx: 0,
            element_name: "T".into(),
            x: Some(95_250),
            w: Some(190_500),
        };
        let s = f.to_string();
        assert!(s.contains("X → 10.0px"), "{s}");
        assert!(s.contains("W → 20.0px"), "{s}");
    }

    #[test]
    fn fix_set_font_size_display() {
        let f = Fix::SetFontSize {
            slide_idx: 2,
            element_name: "H".into(),
            size: 4400,
        };
        assert_eq!(f.to_string(), "slide 3 'H': set font size → 44pt");
    }

    #[test]
    fn fix_accessors() {
        let f = Fix::SetX {
            slide_idx: 3,
            element_name: "E".into(),
            x: 0,
        };
        assert_eq!(f.slide_idx(), 3);
        assert_eq!(f.element_name(), "E");
    }

    #[test]
    fn fix_normalize_whitespace_display() {
        let f = Fix::NormalizeWhitespace {
            slide_idx: 0,
            element_name: "Body".into(),
        };
        assert!(f.to_string().contains("normalize whitespace"), "{f}");
    }

    #[test]
    fn title_y_off_renders_higher_and_lower() {
        let lower = ViolationMessage::TitleYOff {
            actual_emu: 400_000,
            expected_emu: 100_000,
        };
        assert!(lower.to_string().contains("lower"), "{lower}");
        let higher = ViolationMessage::TitleYOff {
            actual_emu: 100_000,
            expected_emu: 400_000,
        };
        assert!(higher.to_string().contains("higher"), "{higher}");
    }

    #[test]
    fn violation_message_display_covers_every_variant() {
        use ViolationMessage::*;
        assert_eq!(
            BodyFontSize {
                actual: 1800,
                expected: 2400
            }
            .to_string(),
            "body font size 18pt, expected 24pt"
        );
        assert!(
            BodyFontFamily {
                actual: "Arial".into(),
                expected: "Calibri".into()
            }
            .to_string()
            .contains("Arial")
        );
        assert!(
            BodyTextColor {
                actual: "FF0000".into(),
                expected: "000000".into()
            }
            .to_string()
            .contains("FF0000")
        );
        assert_eq!(
            ElementOverflow.to_string(),
            "element extends outside slide bounds"
        );
        assert_eq!(DoubleSpace.to_string(), "paragraph contains double spaces");
        assert!(TrailingSpace.to_string().contains("whitespace"));
        assert!(
            BulletCapitalization {
                expected_uppercase: true
            }
            .to_string()
            .contains("uppercase")
        );
        assert!(
            BulletCapitalization {
                expected_uppercase: false
            }
            .to_string()
            .contains("lowercase")
        );
        assert_eq!(TitleMissing.to_string(), "slide has no title element");
        assert_eq!(EmptyElement.to_string(), "no text content");
        assert!(AllCaps.to_string().contains("all caps"));
        assert!(
            BulletPunctuation {
                expected_punctuation: true
            }
            .to_string()
            .contains("punctuation")
        );
        assert!(
            BulletPunctuation {
                expected_punctuation: false
            }
            .to_string()
            .contains("punctuation")
        );
        assert!(
            BulletTooLong {
                word_count: 25,
                limit: 20
            }
            .to_string()
            .contains("25")
        );
        assert_eq!(
            DuplicateTitle { first_slide: 2 }.to_string(),
            "same title as slide 2"
        );
        assert!(
            TitleTooLong {
                word_count: 12,
                limit: 10
            }
            .to_string()
            .contains("12")
        );
        assert!(TitleTrailingPunct { punct: '!' }.to_string().contains('!'));
        assert!(
            RepeatedWord { word: "the".into() }
                .to_string()
                .contains("the")
        );
        assert!(FontVariety { count: 4, limit: 2 }.to_string().contains('4'));
        assert!(
            ColorVariety { count: 5, limit: 3 }
                .to_string()
                .contains('5')
        );
        assert!(
            SlideCount {
                count: 30,
                limit: 20
            }
            .to_string()
            .contains("30")
        );
        assert!(
            ElementOverlap {
                other_element: "Box 2".into()
            }
            .to_string()
            .contains("Box 2")
        );
        assert!(
            ImageAspectRatio {
                actual_ratio: 1.33,
                expected_ratio: 1.78
            }
            .to_string()
            .contains("1.33")
        );
    }
}
