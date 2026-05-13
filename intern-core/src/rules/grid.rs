use crate::detector::{SlideLayout, detect};
use crate::model::SlideData;
use crate::rules::{Rule, Severity, Violation, ViolationMessage};

pub struct GridHSpacingRule;
pub struct GridVSpacingRule;
pub struct GridRowTopRule;
pub struct GridColLeftRule;

fn median(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut s = values.to_vec();
    s.sort();
    Some(s[s.len() / 2])
}

impl Rule for GridHSpacingRule {
    fn id(&self) -> &'static str {
        "GRID_H_SPACING"
    }

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { rows, .. } = detect(slide) else {
                continue;
            };

            // Global expected gap = median of all inter-element horizontal gaps across rows.
            // Using a global value catches cross-row inconsistency (e.g. row 0 gap 400px,
            // row 1 gap 200px) which per-row medians would miss when each row has only one gap.
            // SAFETY: all indices in rows/cols come from detect(), which enumerates slide.elements.
            let all_gaps: Vec<i64> = rows
                .iter()
                .filter(|row| row.len() >= 2)
                .flat_map(|row| {
                    let mut sorted = row.clone();
                    sorted.sort_by_key(|&i| slide.elements[i].rect.x);
                    sorted
                        .windows(2)
                        .map(|w| slide.elements[w[1]].rect.x - slide.elements[w[0]].rect.right())
                        .collect::<Vec<_>>()
                })
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
                            element: Some(format!(
                                "{} / {}",
                                slide.elements[pair[0]].name, slide.elements[pair[1]].name
                            )),
                            message: ViolationMessage::GapUneven {
                                actual_emu: sp,
                                expected_emu: exp,
                            },
                            severity: Severity::Warning,
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

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { cols, .. } = detect(slide) else {
                continue;
            };

            // SAFETY: all indices in rows/cols come from detect(), which enumerates slide.elements.
            let all_gaps: Vec<i64> = cols
                .iter()
                .filter(|col| col.len() >= 2)
                .flat_map(|col| {
                    let mut sorted = col.clone();
                    sorted.sort_by_key(|&i| slide.elements[i].rect.y);
                    sorted
                        .windows(2)
                        .map(|w| slide.elements[w[1]].rect.y - slide.elements[w[0]].rect.bottom())
                        .collect::<Vec<_>>()
                })
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
                            element: Some(format!(
                                "{} / {}",
                                slide.elements[pair[0]].name, slide.elements[pair[1]].name
                            )),
                            message: ViolationMessage::GapUneven {
                                actual_emu: sp,
                                expected_emu: exp,
                            },
                            severity: Severity::Warning,
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

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { rows, .. } = detect(slide) else {
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
                            element: Some(slide.elements[i].name.clone()),
                            message: ViolationMessage::EdgeOff { diff_emu: diff },
                            severity: Severity::Warning,
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

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::Grid { cols, .. } = detect(slide) else {
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
                            element: Some(slide.elements[i].name.clone()),
                            message: ViolationMessage::EdgeOff { diff_emu: diff },
                            severity: Severity::Warning,
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

    const THRESHOLD: i64 = 19_050; // 2px

    fn img(name: &str, x: i64, y: i64) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::Image,
            rect: Rect {
                x,
                y,
                w: 1_000_000,
                h: 800_000,
            },
            font_size: None,
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
        }
    }

    #[test]
    fn grid_h_spacing_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridHSpacingRule.check(&[s], THRESHOLD).is_empty());
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
        };
        let v = GridHSpacingRule.check(&[s], THRESHOLD);
        assert!(
            !v.is_empty(),
            "expected GRID_H_SPACING violation from cross-row gap mismatch"
        );
    }

    #[test]
    fn grid_row_top_fires_on_vertical_shift() {
        let skew = 50_000i64; // ~5px
        let s = grid_with_skew(200_000, skew);
        let v = GridRowTopRule.check(&[s], THRESHOLD);
        assert!(
            !v.is_empty(),
            "expected GRID_ROW_TOP violation for skewed top-right"
        );
    }

    #[test]
    fn grid_row_top_clean() {
        let s = grid_with_skew(200_000, 0);
        assert!(GridRowTopRule.check(&[s], THRESHOLD).is_empty());
    }
}
