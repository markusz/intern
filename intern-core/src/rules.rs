pub mod columns;
pub mod grid;
pub mod title;

use std::fmt;

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
}

impl Fix {
    pub fn slide_idx(&self) -> usize {
        match self {
            Fix::SetX { slide_idx, .. }
            | Fix::SetY { slide_idx, .. }
            | Fix::SetXW { slide_idx, .. }
            | Fix::SetFontSize { slide_idx, .. } => *slide_idx,
        }
    }

    pub fn element_name(&self) -> &str {
        match self {
            Fix::SetX { element_name, .. }
            | Fix::SetY { element_name, .. }
            | Fix::SetXW { element_name, .. }
            | Fix::SetFontSize { element_name, .. } => element_name,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
            } => write!(
                f,
                "title Y {actual_emu} vs expected {expected_emu} ({:.1}px off)",
                (*actual_emu - *expected_emu).abs() as f64 / EMU_PER_PX,
            ),
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

pub fn all_rules() -> Vec<Box<dyn Rule>> {
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
        assert!(s.contains("title Y"), "{s}");
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
}
