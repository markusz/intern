use crate::detector::{SlideLayout, detect};
use crate::model::SlideData;
use crate::rules::{Rule, Severity, Violation, ViolationMessage};

pub struct ColumnLeftEdgeRule;
pub struct ColumnTopEdgeRule;
pub struct ColumnRightLeftEdgeRule;

fn median(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut s = values.to_vec();
    s.sort();
    Some(s[s.len() / 2])
}

impl Rule for ColumnLeftEdgeRule {
    fn id(&self) -> &'static str {
        "COLUMN_LEFT_EDGE"
    }

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { left, .. } = detect(slide) else {
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

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { left, right } = detect(slide) else {
                continue;
            };
            let left_top = left.iter().map(|&i| slide.elements[i].rect.y).min();
            let right_top = right.iter().map(|&i| slide.elements[i].rect.y).min();
            if let (Some(lt), Some(rt)) = (left_top, right_top) {
                let diff = (lt - rt).abs();
                if diff > threshold {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: None,
                        message: ViolationMessage::ColumnTopMisaligned { diff_emu: diff },
                        severity: Severity::Warning,
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

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let SlideLayout::TwoColumn { right, .. } = detect(slide) else {
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
                    });
                }
            }
        }
        violations
    }
}
