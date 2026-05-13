use std::collections::HashMap;

use crate::model::{ElementKind, SlideData, SlideElement};
use crate::rules::{Rule, Severity, Violation, ViolationMessage};

pub struct TitleYRule;
pub struct TitleXWidthRule;
pub struct TitleFontSizeRule;

fn titles(slides: &[SlideData]) -> Vec<(usize, &SlideElement)> {
    slides
        .iter()
        .flat_map(|s| {
            s.elements
                .iter()
                .filter(|e| e.kind == ElementKind::Title)
                .map(move |e| (s.index, e))
        })
        .collect()
}

fn median(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut s = values.to_vec();
    s.sort();
    Some(s[s.len() / 2])
}

impl Rule for TitleYRule {
    fn id(&self) -> &'static str {
        "TITLE_Y"
    }

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let ts = titles(slides);
        if ts.len() < 2 {
            return vec![];
        }
        let ys: Vec<i64> = ts.iter().map(|(_, e)| e.rect.y).collect();
        let Some(expected) = median(&ys) else {
            return vec![];
        };
        ts.iter()
            .filter(|(_, e)| (e.rect.y - expected).abs() > threshold)
            .map(|(idx, e)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(e.name.clone()),
                message: ViolationMessage::TitleYOff {
                    actual_emu: e.rect.y,
                    expected_emu: expected,
                },
                severity: Severity::Warning,
            })
            .collect()
    }
}

impl Rule for TitleXWidthRule {
    fn id(&self) -> &'static str {
        "TITLE_X_WIDTH"
    }

    fn check(&self, slides: &[SlideData], threshold: i64) -> Vec<Violation> {
        let ts = titles(slides);
        if ts.len() < 2 {
            return vec![];
        }
        let xs: Vec<i64> = ts.iter().map(|(_, e)| e.rect.x).collect();
        let ws: Vec<i64> = ts.iter().map(|(_, e)| e.rect.w).collect();
        let Some(exp_x) = median(&xs) else {
            return vec![];
        };
        let Some(exp_w) = median(&ws) else {
            return vec![];
        };

        ts.iter()
            .filter_map(|(idx, e)| {
                let dx = (e.rect.x - exp_x).abs();
                let dw = (e.rect.w - exp_w).abs();
                if dx <= threshold && dw <= threshold {
                    return None;
                }
                Some(Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(e.name.clone()),
                    message: ViolationMessage::TitlePositionSize {
                        x_off_emu: (dx > threshold).then_some(dx),
                        w_off_emu: (dw > threshold).then_some(dw),
                    },
                    severity: Severity::Warning,
                })
            })
            .collect()
    }
}

impl Rule for TitleFontSizeRule {
    fn id(&self) -> &'static str {
        "TITLE_FONT_SIZE"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let ts = titles(slides);
        let sized: Vec<(usize, u32)> = ts
            .iter()
            .filter_map(|(idx, e)| e.font_size.map(|sz| (*idx, sz)))
            .collect();

        if sized.len() < 2 {
            return vec![];
        }

        let mut counts: HashMap<u32, usize> = HashMap::new();
        for (_, sz) in &sized {
            *counts.entry(*sz).or_default() += 1;
        }
        let expected = *counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(sz, _)| sz)
            .unwrap();

        sized
            .iter()
            .filter(|(_, sz)| *sz != expected)
            .map(|(idx, sz)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: None,
                message: ViolationMessage::TitleFontSize {
                    actual: *sz,
                    expected,
                },
                severity: Severity::Warning,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Rect, SlideData, SlideElement};
    use crate::rules::ViolationMessage;

    const THRESHOLD: i64 = 19_050; // 2px at 96 DPI

    fn title_slide(slide_idx: usize, y: i64, font_size: Option<u32>) -> SlideData {
        SlideData {
            index: slide_idx,
            elements: vec![SlideElement {
                name: format!("Title {}", slide_idx + 1),
                kind: ElementKind::Title,
                rect: Rect {
                    x: 457_200,
                    y,
                    w: 8_229_600,
                    h: 1_143_000,
                },
                font_size,
            }],
        }
    }

    #[test]
    fn title_y_clean() {
        let slides = vec![title_slide(0, 274_638, None), title_slide(1, 274_638, None)];
        assert!(TitleYRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn title_y_fires_on_outlier() {
        let slides = vec![
            title_slide(0, 274_638, None),
            title_slide(1, 274_638, None),
            title_slide(2, 400_000, None), // ~13px off
        ];
        let v = TitleYRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
    }

    #[test]
    fn title_y_ignores_sub_threshold_diff() {
        let slides = vec![
            title_slide(0, 274_638, None),
            title_slide(1, 274_638 + 9_000, None), // ~0.9px, under 2px threshold
        ];
        assert!(TitleYRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn title_font_size_clean() {
        let slides = vec![
            title_slide(0, 274_638, Some(4_400)),
            title_slide(1, 274_638, Some(4_400)),
        ];
        assert!(TitleFontSizeRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn title_font_size_fires_on_outlier() {
        let slides = vec![
            title_slide(0, 274_638, Some(4_400)), // 44pt majority
            title_slide(1, 274_638, Some(4_400)),
            title_slide(2, 274_638, Some(3_600)), // 36pt outlier
        ];
        let v = TitleFontSizeRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
        assert!(matches!(
            v[0].message,
            ViolationMessage::TitleFontSize {
                actual: 3600,
                expected: 4400
            }
        ));
    }

    #[test]
    fn title_font_size_skips_inherited() {
        // None = inherited from layout master, no false positives
        let slides = vec![title_slide(0, 274_638, None), title_slide(1, 274_638, None)];
        assert!(TitleFontSizeRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn title_x_width_fires_on_shifted_x() {
        let slides = vec![
            SlideData {
                index: 0,
                elements: vec![SlideElement {
                    name: "Title 1".into(),
                    kind: ElementKind::Title,
                    rect: Rect {
                        x: 457_200,
                        y: 274_638,
                        w: 8_229_600,
                        h: 1_143_000,
                    },
                    font_size: None,
                }],
            },
            SlideData {
                index: 1,
                elements: vec![SlideElement {
                    name: "Title 2".into(),
                    kind: ElementKind::Title,
                    rect: Rect {
                        x: 600_000,
                        y: 274_638,
                        w: 8_229_600,
                        h: 1_143_000,
                    },
                    font_size: None,
                }],
            },
        ];
        let v = TitleXWidthRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::TitlePositionSize {
                x_off_emu: Some(_),
                w_off_emu: None
            }
        ));
    }
}
