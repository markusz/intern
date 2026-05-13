pub mod columns;
pub mod grid;
pub mod title;

use crate::model::SlideData;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub rule_id: &'static str,
    pub slide: Option<usize>, // 1-based
    pub element: Option<String>,
    pub message: String,
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
        Box::new(EdgeSnapRule),
    ]
}

pub struct EdgeSnapRule;

impl Rule for EdgeSnapRule {
    fn id(&self) -> &'static str {
        "EDGE_SNAP"
    }

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        // Fires when edges are off by more than threshold but less than 6x threshold —
        // the "close enough to look intentional but isn't" zone.
        let snap_zone = threshold * 6;
        let mut violations = Vec::new();

        for slide in slides {
            let els = &slide.elements;
            for i in 0..els.len() {
                for j in (i + 1)..els.len() {
                    let a = &els[i];
                    let b = &els[j];

                    use crate::model::ElementKind;
                    if matches!(a.kind, ElementKind::Title | ElementKind::Body)
                        || matches!(b.kind, ElementKind::Title | ElementKind::Body)
                    {
                        continue;
                    }

                    let pairs: [(&str, i64, i64); 4] = [
                        ("left", a.rect.x, b.rect.x),
                        ("right", a.rect.right(), b.rect.right()),
                        ("top", a.rect.y, b.rect.y),
                        ("bottom", a.rect.bottom(), b.rect.bottom()),
                    ];

                    for (edge, ea, eb) in pairs {
                        let diff = (ea - eb).abs();
                        if diff > threshold && diff <= snap_zone {
                            violations.push(Violation {
                                rule_id: self.id(),
                                slide: Some(slide.index + 1),
                                element: Some(format!("{} / {}", a.name, b.name)),
                                message: format!(
                                    "{edge} edges nearly aligned ({:.1}px off): \"{}\" and \"{}\"",
                                    diff as f64 / 9525.0,
                                    a.name,
                                    b.name,
                                ),
                                severity: Severity::Warning,
                            });
                        }
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

    fn shape(name: &str, x: i64, y: i64) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::TextBox,
            rect: Rect {
                x,
                y,
                w: 500_000,
                h: 300_000,
            },
            font_size: None,
        }
    }

    fn slide(elements: Vec<SlideElement>) -> SlideData {
        SlideData { index: 0, elements }
    }

    #[test]
    fn edge_snap_fires_in_near_miss_zone() {
        // 4px off — over 2px threshold, under 6x=12px snap zone
        let diff = 4 * 9_525;
        let s = slide(vec![
            shape("A", 457_200, 500_000),
            shape("B", 457_200 + diff, 500_000),
        ]);
        let v = EdgeSnapRule.check(&[s], THRESHOLD);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "EDGE_SNAP");
    }

    #[test]
    fn edge_snap_silent_when_aligned() {
        let s = slide(vec![
            shape("A", 457_200, 500_000),
            shape("B", 457_200, 800_000),
        ]);
        assert!(EdgeSnapRule.check(&[s], THRESHOLD).is_empty());
    }

    #[test]
    fn edge_snap_silent_when_clearly_different() {
        // 50px off — outside the snap zone
        let diff = 50 * 9_525;
        let s = slide(vec![
            shape("A", 457_200, 500_000),
            shape("B", 457_200 + diff, 500_000),
        ]);
        assert!(EdgeSnapRule.check(&[s], THRESHOLD).is_empty());
    }

    #[test]
    fn edge_snap_silent_when_just_under_threshold() {
        // 1px off — under 2px threshold, acceptable
        let diff = 9_000; // < 9525
        let s = slide(vec![
            shape("A", 457_200, 500_000),
            shape("B", 457_200 + diff, 500_000),
        ]);
        assert!(EdgeSnapRule.check(&[s], THRESHOLD).is_empty());
    }
}
