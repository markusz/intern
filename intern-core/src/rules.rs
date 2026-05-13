pub mod columns;
pub mod grid;
pub mod title;

use std::fmt;

use crate::model::SlideData;

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
