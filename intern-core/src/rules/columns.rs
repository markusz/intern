use crate::detector::{SlideLayout, detect};
use crate::model::SlideData;
use crate::rules::{Fix, Rule, RuleContext, Severity, Violation, ViolationMessage};

pub struct ColumnLeftEdgeRule;
pub struct ColumnTopEdgeRule;
pub struct ColumnRightLeftEdgeRule;

use super::median;

impl Rule for ColumnLeftEdgeRule {
    fn id(&self) -> &'static str {
        "COLUMN_LEFT_EDGE"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { left, .. } =
                detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };
            if left.len() < 2 {
                continue;
            }
            // SAFETY: indices from detect() are guaranteed to be within bounds of slide.elements.
            let xs: Vec<i64> = left.iter().map(|&i| slide.elements[i].rect.x).collect();
            let Some(exp) = median(&xs) else { continue };
            for &i in &left {
                let diff = (slide.elements[i].rect.x - exp).abs();
                if diff > threshold {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(slide.elements[i].name.clone()),
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
        violations
    }
}

impl Rule for ColumnTopEdgeRule {
    fn id(&self) -> &'static str {
        "COLUMN_TOP_EDGE"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { left, right } =
                detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };
            // Find the topmost element index in each column so we can name what to fix.
            let left_top_i = left
                .iter()
                .copied()
                .min_by_key(|&i| slide.elements[i].rect.y);
            let right_top_i = right
                .iter()
                .copied()
                .min_by_key(|&i| slide.elements[i].rect.y);
            if let (Some(li), Some(ri)) = (left_top_i, right_top_i) {
                let lt = slide.elements[li].rect.y;
                let rt = slide.elements[ri].rect.y;
                let diff = (lt - rt).abs();
                if diff > threshold {
                    // Snap the lagging column's first element up to match the earlier one.
                    let (target_i, target_y) = if lt > rt { (li, rt) } else { (ri, lt) };
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: None,
                        message: ViolationMessage::ColumnTopMisaligned { diff_emu: diff },
                        severity: Severity::Warning,
                        fix: Some(Fix::SetY {
                            slide_idx: slide.index,
                            element_name: slide.elements[target_i].name.clone(),
                            y: target_y,
                        }),
                    });
                }
            }
        }
        violations
    }
}

impl Rule for ColumnRightLeftEdgeRule {
    fn id(&self) -> &'static str {
        "COLUMN_RIGHT_LEFT_EDGE"
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let threshold = ctx.threshold;
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { right, .. } =
                detect(slide, ctx.slide_width, ctx.slide_height)
            else {
                continue;
            };
            if right.len() < 2 {
                continue;
            }
            let xs: Vec<i64> = right.iter().map(|&i| slide.elements[i].rect.x).collect();
            let Some(exp) = median(&xs) else { continue };
            for &i in &right {
                let diff = (slide.elements[i].rect.x - exp).abs();
                if diff > threshold {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(slide.elements[i].name.clone()),
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
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Rect, SlideElement};
    use crate::rules::Rule;

    const T: RuleContext = RuleContext {
        threshold: 19_050, // 2 px
        slide_width: 9_144_000,
        slide_height: 6_858_000,
    };

    fn shape(name: &str, x: i64, y: i64) -> SlideElement {
        SlideElement {
            name: name.to_owned(),
            kind: ElementKind::TextBox,
            rect: Rect {
                x,
                y,
                w: 1_000_000,
                h: 500_000,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    fn slide(idx: usize, elements: Vec<SlideElement>) -> SlideData {
        SlideData {
            index: idx,
            elements,
        }
    }

    #[test]
    fn column_left_edge_skips_single_element_column() {
        // Only one left-column element → no median comparison possible.
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnLeftEdgeRule.check(&[s], &T);
        assert!(v.is_empty());
    }

    #[test]
    fn column_left_edge_clean_when_aligned() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnLeftEdgeRule.check(&[s], &T);
        assert!(v.is_empty());
    }

    #[test]
    fn column_left_edge_fires_when_misaligned() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200 + 200_000, 2_000_000), // 200_000 EMU off
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnLeftEdgeRule.check(&[s], &T);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "COLUMN_LEFT_EDGE");
        // Fix should snap to the median x (higher of the two, i.e. 457_200+200_000).
        let fix = v[0].fix.as_ref().unwrap();
        assert!(matches!(fix, Fix::SetX { x: 657_200, .. }));
    }

    #[test]
    fn column_left_edge_no_fix_when_sub_threshold() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200 + 1, 2_000_000), // 1 EMU - well within 2 px
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnLeftEdgeRule.check(&[s], &T);
        assert!(v.is_empty());
    }

    #[test]
    fn column_right_left_edge_skips_single_element_column() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_000_000),
            ],
        );
        let v = ColumnRightLeftEdgeRule.check(&[s], &T);
        assert!(v.is_empty());
    }

    #[test]
    fn column_right_left_edge_fires_with_fix() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000 + 200_000, 2_000_000),
            ],
        );
        let v = ColumnRightLeftEdgeRule.check(&[s], &T);
        assert!(!v.is_empty());
        let fix = v[0].fix.as_ref().unwrap();
        // Median of [5_500_000, 5_700_000] is 5_700_000 (higher element at sorted index 1).
        assert!(matches!(fix, Fix::SetX { x: 5_700_000, .. }));
    }

    #[test]
    fn column_top_edge_clean_when_aligned() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_000_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnTopEdgeRule.check(&[s], &T);
        assert!(v.is_empty());
    }

    #[test]
    fn column_top_edge_fires_on_misaligned_top() {
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_000_000 + 200_000), // right column starts lower
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnTopEdgeRule.check(&[s], &T);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "COLUMN_TOP_EDGE");
    }

    #[test]
    fn column_top_edge_fix_targets_lagging_element() {
        // Left top = 1_000_000, right top = 1_200_000 → right is lower, fix snaps it up.
        let s = slide(
            0,
            vec![
                shape("L1", 457_200, 1_000_000),
                shape("L2", 457_200, 2_000_000),
                shape("R1", 5_500_000, 1_200_000),
                shape("R2", 5_500_000, 2_000_000),
            ],
        );
        let v = ColumnTopEdgeRule.check(&[s], &T);
        assert!(!v.is_empty());
        let fix = v[0].fix.as_ref().unwrap();
        assert!(
            matches!(fix, Fix::SetY { element_name, y: 1_000_000, .. } if element_name == "R1")
        );
    }
}
