use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::model::{ElementKind, SlideData, SlideElement};
use crate::rules::{Fix, Rule, Severity, Violation, ViolationMessage};

pub struct TitleYRule;
pub struct TitleXWidthRule;
pub struct TitleFontSizeRule;
pub struct DuplicateTitleRule;
pub struct TitleLengthRule {
    pub limit: usize,
}
pub struct TitleTrailingPunctRule;

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

use super::median;

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
                fix: Some(Fix::SetY {
                    slide_idx: *idx,
                    element_name: e.name.clone(),
                    y: expected,
                }),
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
                    fix: Some(Fix::SetXW {
                        slide_idx: *idx,
                        element_name: e.name.clone(),
                        x: (dx > threshold).then_some(exp_x),
                        w: (dw > threshold).then_some(exp_w),
                    }),
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
        let sized: Vec<(usize, String, u32)> = ts
            .iter()
            .filter_map(|(idx, e)| e.font_size.map(|sz| (*idx, e.name.clone(), sz)))
            .collect();

        if sized.len() < 2 {
            return vec![];
        }

        let mut counts: HashMap<u32, usize> = HashMap::new();
        for (_, _, sz) in &sized {
            *counts.entry(*sz).or_default() += 1;
        }
        // SAFETY: sized.len() >= 2 ensures counts is non-empty, so max_by_key returns Some.
        let expected = *counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(sz, _)| sz)
            .unwrap();

        sized
            .iter()
            .filter(|(_, _, sz)| *sz != expected)
            .map(|(idx, name, sz)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(name.clone()),
                message: ViolationMessage::TitleFontSize {
                    actual: *sz,
                    expected,
                },
                severity: Severity::Warning,
                fix: Some(Fix::SetFontSize {
                    slide_idx: *idx,
                    element_name: name.clone(),
                    size: expected,
                }),
            })
            .collect()
    }
}

fn title_text(e: &crate::model::SlideElement) -> String {
    e.paragraphs.join(" ").trim().to_string()
}

impl Rule for DuplicateTitleRule {
    fn id(&self) -> &'static str {
        "DUPLICATE_TITLE"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let ts = titles(slides);
        let mut first: HashMap<String, usize> = HashMap::new();
        let mut violations = Vec::new();
        for (idx, e) in &ts {
            let text = title_text(e);
            if text.is_empty() {
                continue;
            }
            match first.entry(text) {
                Entry::Vacant(v) => {
                    v.insert(idx + 1);
                }
                Entry::Occupied(o) => {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(idx + 1),
                        element: Some(e.name.clone()),
                        message: ViolationMessage::DuplicateTitle {
                            first_slide: *o.get(),
                        },
                        severity: Severity::Warning,
                        fix: None,
                    });
                }
            }
        }
        violations
    }
}

impl Rule for TitleLengthRule {
    fn id(&self) -> &'static str {
        "TITLE_LENGTH"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        titles(slides)
            .iter()
            .filter_map(|(idx, e)| {
                let text = title_text(e);
                let word_count = text.split_whitespace().count();
                (word_count > self.limit).then(|| Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(e.name.clone()),
                    message: ViolationMessage::TitleTooLong {
                        word_count,
                        limit: self.limit,
                    },
                    severity: Severity::Warning,
                    fix: None,
                })
            })
            .collect()
    }
}

impl Rule for TitleTrailingPunctRule {
    fn id(&self) -> &'static str {
        "TITLE_TRAILING_PUNCT"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        titles(slides)
            .iter()
            .filter_map(|(idx, e)| {
                let text = title_text(e);
                let last = text.chars().last()?;
                matches!(last, '.' | '!' | '?').then(|| Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(e.name.clone()),
                    message: ViolationMessage::TitleTrailingPunct { punct: last },
                    severity: Severity::Warning,
                    fix: None,
                })
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
        titled(slide_idx, y, font_size, "")
    }

    fn titled(slide_idx: usize, y: i64, font_size: Option<u32>, text: &str) -> SlideData {
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
                font_family: None,
                text_color: None,
                paragraphs: if text.is_empty() {
                    vec![]
                } else {
                    vec![text.to_string()]
                },
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
    fn title_x_width_clean() {
        let slides = vec![title_slide(0, 274_638, None), title_slide(1, 274_638, None)];
        assert!(TitleXWidthRule.check(&slides, THRESHOLD).is_empty());
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
                    font_family: None,
                    text_color: None,
                    paragraphs: vec![],
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
                    font_family: None,
                    text_color: None,
                    paragraphs: vec![],
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

    #[test]
    fn duplicate_title_clean() {
        let slides = vec![
            titled(0, 274_638, None, "Intro"),
            titled(1, 274_638, None, "Methods"),
        ];
        assert!(DuplicateTitleRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn duplicate_title_fires_on_repeated_text() {
        let slides = vec![
            titled(0, 274_638, None, "Intro"),
            titled(1, 274_638, None, "Intro"),
        ];
        let v = DuplicateTitleRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(2));
        assert!(matches!(
            v[0].message,
            ViolationMessage::DuplicateTitle { first_slide: 1 }
        ));
    }

    #[test]
    fn duplicate_title_skips_empty_titles() {
        let slides = vec![title_slide(0, 274_638, None), title_slide(1, 274_638, None)];
        assert!(DuplicateTitleRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn duplicate_title_flags_all_extra_occurrences() {
        let slides = vec![
            titled(0, 274_638, None, "Same"),
            titled(1, 274_638, None, "Same"),
            titled(2, 274_638, None, "Same"),
        ];
        let v = DuplicateTitleRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn title_length_clean() {
        let slides = vec![titled(0, 274_638, None, "Short title")];
        assert!(
            TitleLengthRule { limit: 10 }
                .check(&slides, THRESHOLD)
                .is_empty()
        );
    }

    #[test]
    fn title_length_fires_over_limit() {
        let long = "word ".repeat(11).trim().to_string();
        let slides = vec![titled(0, 274_638, None, &long)];
        let v = TitleLengthRule { limit: 10 }.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::TitleTooLong {
                word_count: 11,
                limit: 10
            }
        ));
    }

    #[test]
    fn title_length_at_limit_is_clean() {
        let exact = "word ".repeat(10).trim().to_string();
        let slides = vec![titled(0, 274_638, None, &exact)];
        assert!(
            TitleLengthRule { limit: 10 }
                .check(&slides, THRESHOLD)
                .is_empty()
        );
    }

    #[test]
    fn title_trailing_punct_clean() {
        let slides = vec![titled(0, 274_638, None, "No trailing punct")];
        assert!(TitleTrailingPunctRule.check(&slides, THRESHOLD).is_empty());
    }

    #[test]
    fn title_trailing_punct_fires_on_period() {
        let slides = vec![titled(0, 274_638, None, "Ends with a period.")];
        let v = TitleTrailingPunctRule.check(&slides, THRESHOLD);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::TitleTrailingPunct { punct: '.' }
        ));
    }

    #[test]
    fn title_trailing_punct_fires_on_exclamation() {
        let slides = vec![titled(0, 274_638, None, "Wow!")];
        assert_eq!(TitleTrailingPunctRule.check(&slides, THRESHOLD).len(), 1);
    }

    #[test]
    fn title_trailing_punct_colon_is_ok() {
        // Colons are legitimate in titles ("Results: 2024")
        let slides = vec![titled(0, 274_638, None, "Results: 2024")];
        assert!(TitleTrailingPunctRule.check(&slides, THRESHOLD).is_empty());
    }
}
