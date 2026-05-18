use crate::detector::{SlideLayout, detect};
use crate::model::{SlideData, SlideElement};
use crate::rules::{Fix, Rule, RuleContext, Severity, Violation, ViolationMessage};

pub struct GridHSpacingRule;
pub struct GridVSpacingRule;
pub struct GridRowTopRule;
pub struct GridColLeftRule;

use super::median;

fn h_gaps_in_row(row: &[usize], elements: &[SlideElement]) -> Vec<i64> {
    let mut sorted = row.to_vec();
    sorted.sort_by_key(|&i| elements[i].rect.x);
    sorted
        .windows(2)
        .map(|w| elements[w[1]].rect.x - elements[w[0]].rect.right())
        .collect()
}

fn v_gaps_in_col(col: &[usize], elements: &[SlideElement]) -> Vec<i64> {
    let mut sorted = col.to_vec();
    sorted.sort_by_key(|&i| elements[i].rect.y);
    sorted
        .windows(2)
        .map(|w| elements[w[1]].rect.y - elements[w[0]].rect.bottom())
        .collect()
}

impl Rule for GridHSpacingRule {
    fn id(&self) -> &'static str {
        "GRID_H_SPACING"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { rows, .. } = detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };

            // Global expected gap = median across all rows. Using a global value catches
            // cross-row inconsistency that per-row medians miss when rows have only one gap.
            let all_gaps: Vec<i64> = rows
                .iter()
                .filter(|row| row.len() >= 2)
                .flat_map(|row| h_gaps_in_row(row, &slide.elements))
                .collect();

            let Some(exp) = median(&all_gaps) else {
                continue;
            };

            for row in &rows {
                if row.len() < 2 {
                    continue;
                }
                let mut sorted = row.clone();
                sorted.sort_by_key(|&i| slide.elements[i].rect.x);

                for pair in sorted.windows(2) {
                    let sp = slide.elements[pair[1]].rect.x - slide.elements[pair[0]].rect.right();
                    let diff = (sp - exp).abs();
                    if diff > threshold {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(slide.elements[pair[0]].id),
                            message: ViolationMessage::GapUneven {
                                actual_emu: sp,
                                expected_emu: exp,
                            },
                            severity: Severity::Warning,
                            fix: None, // redistributing gaps requires moving multiple shapes
                        });
                    }
                }
            }
        }
        violations
    }
}

impl Rule for GridVSpacingRule {
    fn id(&self) -> &'static str {
        "GRID_V_SPACING"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { cols, .. } = detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };

            let all_gaps: Vec<i64> = cols
                .iter()
                .filter(|col| col.len() >= 2)
                .flat_map(|col| v_gaps_in_col(col, &slide.elements))
                .collect();

            let Some(exp) = median(&all_gaps) else {
                continue;
            };

            for col in &cols {
                if col.len() < 2 {
                    continue;
                }
                let mut sorted = col.clone();
                sorted.sort_by_key(|&i| slide.elements[i].rect.y);

                for pair in sorted.windows(2) {
                    let sp = slide.elements[pair[1]].rect.y - slide.elements[pair[0]].rect.bottom();
                    let diff = (sp - exp).abs();
                    if diff > threshold {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(slide.elements[pair[0]].id),
                            message: ViolationMessage::GapUneven {
                                actual_emu: sp,
                                expected_emu: exp,
                            },
                            severity: Severity::Warning,
                            fix: None, // redistributing gaps requires moving multiple shapes
                        });
                    }
                }
            }
        }
        violations
    }
}

impl Rule for GridRowTopRule {
    fn id(&self) -> &'static str {
        "GRID_ROW_TOP"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { rows, .. } = detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };
            for row in &rows {
                if row.len() < 2 {
                    continue;
                }
                let ys: Vec<i64> = row.iter().map(|&i| slide.elements[i].rect.y).collect();
                let Some(exp) = median(&ys) else { continue };
                for &i in row {
                    let diff = (slide.elements[i].rect.y - exp).abs();
                    if diff > threshold {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(slide.elements[i].id),
                            message: ViolationMessage::EdgeOff { diff_emu: diff },
                            severity: Severity::Warning,
                            fix: Some(Fix::SetY {
                                slide_idx: slide.index,
                                element_name: slide.elements[i].name.clone(),
                                y: exp,
                            }),
                        });
                    }
                }
            }
        }
        violations
    }
}

impl Rule for GridColLeftRule {
    fn id(&self) -> &'static str {
        "GRID_COL_LEFT"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { cols, .. } = detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };
            for col in &cols {
                if col.len() < 2 {
                    continue;
                }
                let xs: Vec<i64> = col.iter().map(|&i| slide.elements[i].rect.x).collect();
                let Some(exp) = median(&xs) else { continue };
                for &i in col {
                    let diff = (slide.elements[i].rect.x - exp).abs();
                    if diff > threshold {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(slide.elements[i].id),
                            message: ViolationMessage::EdgeOff { diff_emu: diff },
                            severity: Severity::Warning,
                            fix: Some(Fix::SetX {
                                slide_idx: slide.index,
                                element_name: slide.elements[i].name.clone(),
                                x: exp,
                            }),
                        });
                    }
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Rect, SlideData, SlideElement};

    const THRESHOLD: RuleContext = RuleContext {
        threshold: 19_050, // 2px
        slide_width: 9_144_000,
        slide_height: 6_858_000,
    };

    fn img(name: &str, x: i64, y: i64) -> SlideElement {
        SlideElement {
            id: 1,
            name: name.into(),
            kind: ElementKind::Image,
            rect: Rect {
                x,
                y,
                w: 1_000_000,
                h: 800_000,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    // 2×2 grid with uniform gap=200_000, top-right shifted by `skew`
    fn grid_with_skew(h_gap: i64, row_skew: i64) -> SlideData {
        let (w, h) = (1_000_000i64, 800_000i64);
        let (x0, y0) = (500_000i64, 1_500_000i64);
        let row_gap = 200_000i64;
        SlideData {
            index: 0,
            elements: vec![
                img("TL", x0, y0),
                img("TR", x0 + w + h_gap, y0 + row_skew),
                img("BL", x0, y0 + h + row_gap),
                img("BR", x0 + w + h_gap, y0 + h + row_gap),
            ],
            units: vec![],
        }
    }

    #[test]
    fn grid_h_spacing_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridHSpacingRule.check(&[s], &THRESHOLD).is_empty());
    }

    #[test]
    fn grid_h_spacing_fires_on_unequal_gaps() {
        // Row 0 gap = 400k, row 1 gap = 200k.
        // Global median = 400k (sorted [200k,400k], idx 1).
        // Row 1's gap (200k) deviates by 200k EMU ≈ 21px → violation.
        let s = SlideData {
            index: 0,
            elements: vec![
                img("TL", 500_000, 1_500_000),
                img("TR", 500_000 + 1_000_000 + 400_000, 1_500_000), // gap=400k
                img("BL", 500_000, 1_500_000 + 800_000 + 200_000),
                img(
                    "BR",
                    500_000 + 1_000_000 + 200_000,
                    1_500_000 + 800_000 + 200_000,
                ), // gap=200k
            ],
            units: vec![],
        };
        let v = GridHSpacingRule.check(&[s], &THRESHOLD);
        assert!(
            !v.is_empty(),
            "expected GRID_H_SPACING violation from cross-row gap mismatch"
        );
    }

    #[test]
    fn grid_row_top_fires_on_vertical_shift() {
        let skew = 50_000i64; // ~5px
        let s = grid_with_skew(200_000, skew);
        let v = GridRowTopRule.check(&[s], &THRESHOLD);
        assert!(
            !v.is_empty(),
            "expected GRID_ROW_TOP violation for skewed top-right"
        );
    }

    #[test]
    fn grid_row_top_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridRowTopRule.check(&[s], &THRESHOLD).is_empty());
    }

    #[test]
    fn grid_row_top_fix_snaps_to_expected_y() {
        let skew = 50_000i64;
        let s = grid_with_skew(200_000, skew);
        let v = GridRowTopRule.check(&[s], &THRESHOLD);
        assert!(!v.is_empty());
        let fix = v[0].fix.as_ref().unwrap();
        // Row 0 has TL at 1_500_000 and TR at 1_550_000; median (index 1) = 1_550_000.
        // TL deviates, fix snaps it to 1_550_000.
        assert!(matches!(fix, Fix::SetY { y: 1_550_000, .. }));
    }

    #[test]
    fn grid_v_spacing_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridVSpacingRule.check(&[s], &THRESHOLD).is_empty());
    }

    #[test]
    fn grid_v_spacing_fires_on_unequal_gaps() {
        // Col 0 vertical gap = 200k, col 1 vertical gap = 400k → mismatch.
        let (w, h) = (1_000_000i64, 800_000i64);
        let col_gap = 200_000i64;
        let s = SlideData {
            index: 0,
            elements: vec![
                img("TL", 500_000, 1_500_000),
                img("TR", 500_000 + w + col_gap, 1_500_000),
                img("BL", 500_000, 1_500_000 + h + 200_000), // gap=200k
                img("BR", 500_000 + w + col_gap, 1_500_000 + h + 400_000), // gap=400k
            ],
            units: vec![],
        };
        let v = GridVSpacingRule.check(&[s], &THRESHOLD);
        assert!(!v.is_empty(), "expected GRID_V_SPACING violation");
    }

    #[test]
    fn grid_col_left_fires_on_misaligned_column() {
        let skew = 50_000i64;
        let (w, h) = (1_000_000i64, 800_000i64);
        let gap = 200_000i64;
        let s = SlideData {
            index: 0,
            elements: vec![
                img("TL", 500_000, 1_500_000),
                img("TR", 500_000 + w + gap, 1_500_000),
                img("BL", 500_000 + skew, 1_500_000 + h + gap), // col 0 bottom shifted right
                img("BR", 500_000 + w + gap, 1_500_000 + h + gap),
            ],
            units: vec![],
        };
        let v = GridColLeftRule.check(&[s], &THRESHOLD);
        assert!(!v.is_empty(), "expected GRID_COL_LEFT violation");
        let fix = v[0].fix.as_ref().unwrap();
        // Median of col 0 xs: [500_000, 550_000], index 1 = 550_000.
        assert!(matches!(fix, Fix::SetX { x: 550_000, .. }));
    }

    #[test]
    fn grid_col_left_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridColLeftRule.check(&[s], &THRESHOLD).is_empty());
    }
}
