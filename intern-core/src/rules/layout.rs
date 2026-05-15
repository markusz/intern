use crate::model::{ElementKind, Rect, SLIDE_HEIGHT_EMU, SLIDE_WIDTH_EMU, SlideData};
use crate::rules::{Rule, Severity, Violation, ViolationMessage};

pub struct TitlePresentRule;
pub struct EmptyElementRule;
pub struct ElementOverflowRule;
pub struct ElementOverlapRule;
pub struct ImageAspectRatioRule;
pub struct SlideCountRule {
    pub limit: usize,
}

impl Rule for TitlePresentRule {
    fn id(&self) -> &'static str {
        "TITLE_PRESENT"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        slides
            .iter()
            .filter(|s| !s.elements.iter().any(|e| e.kind == ElementKind::Title))
            .map(|s| Violation {
                rule_id: self.id(),
                slide: Some(s.index + 1),
                element: None,
                message: ViolationMessage::TitleMissing,
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

impl Rule for EmptyElementRule {
    fn id(&self) -> &'static str {
        "EMPTY_ELEMENT"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if matches!(e.kind, ElementKind::Body | ElementKind::TextBox)
                    && e.paragraphs.is_empty()
                {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.name.clone()),
                        message: ViolationMessage::EmptyElement,
                        severity: Severity::Warning,
                        fix: None,
                    });
                }
            }
        }
        violations
    }
}

impl Rule for ElementOverflowRule {
    fn id(&self) -> &'static str {
        "ELEMENT_OVERFLOW"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                let r = &e.rect;
                if r.x < 0 || r.y < 0 || r.x + r.w > SLIDE_WIDTH_EMU || r.y + r.h > SLIDE_HEIGHT_EMU
                {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.name.clone()),
                        message: ViolationMessage::ElementOverflow,
                        severity: Severity::Warning,
                        fix: None,
                    });
                }
            }
        }
        violations
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

impl Rule for ElementOverlapRule {
    fn id(&self) -> &'static str {
        "ELEMENT_OVERLAP"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let es = &slide.elements;
            for i in 0..es.len() {
                for j in (i + 1)..es.len() {
                    if rects_overlap(&es[i].rect, &es[j].rect) {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(es[i].name.clone()),
                            message: ViolationMessage::ElementOverlap {
                                other_element: es[j].name.clone(),
                            },
                            severity: Severity::Warning,
                            fix: None,
                        });
                    }
                }
            }
        }
        violations
    }
}

const ASPECT_RATIO_TOLERANCE: f64 = 0.05;

impl Rule for ImageAspectRatioRule {
    fn id(&self) -> &'static str {
        "IMAGE_ASPECT_RATIO"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let images: Vec<(usize, &str, f64)> = slides
            .iter()
            .flat_map(|s| {
                s.elements
                    .iter()
                    .filter(|e| e.kind == ElementKind::Image && e.rect.h > 0)
                    .map(move |e| (s.index, e.name.as_str(), e.rect.w as f64 / e.rect.h as f64))
            })
            .collect();

        if images.len() < 2 {
            return vec![];
        }

        let mut ratios: Vec<f64> = images.iter().map(|(_, _, r)| *r).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // SAFETY: images.len() >= 2, so ratios is non-empty.
        let median = ratios[ratios.len() / 2];

        images
            .iter()
            .filter(|(_, _, r)| (*r - median).abs() / median > ASPECT_RATIO_TOLERANCE)
            .map(|(idx, name, r)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(name.to_string()),
                message: ViolationMessage::ImageAspectRatio {
                    actual_ratio: *r,
                    expected_ratio: median,
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

impl Rule for SlideCountRule {
    fn id(&self) -> &'static str {
        "SLIDE_COUNT"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        if slides.len() <= self.limit {
            return vec![];
        }
        vec![Violation {
            rule_id: self.id(),
            slide: None,
            element: None,
            message: ViolationMessage::SlideCount {
                count: slides.len(),
                limit: self.limit,
            },
            severity: Severity::Warning,
            fix: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Rect, SlideData, SlideElement};

    const T: i64 = 19_050;

    fn shape(name: &str, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::TextBox,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    fn title(name: &str) -> SlideElement {
        SlideElement {
            name: name.into(),
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
            paragraphs: vec!["Title".into()],
        }
    }

    fn body(name: &str, paragraphs: Vec<&str>) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::Body,
            rect: Rect {
                x: 457_200,
                y: 1_600_200,
                w: 8_229_600,
                h: 4_525_963,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: paragraphs.into_iter().map(str::to_string).collect(),
        }
    }

    fn slide(elements: Vec<SlideElement>) -> SlideData {
        SlideData { index: 0, elements }
    }

    fn slide_n(idx: usize, elements: Vec<SlideElement>) -> SlideData {
        SlideData {
            index: idx,
            elements,
        }
    }

    fn image(name: &str, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::Image,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    #[test]
    fn title_present_clean() {
        let s = slide(vec![title("T1"), body("B1", vec!["Content"])]);
        assert!(TitlePresentRule.check(&[s], T).is_empty());
    }

    #[test]
    fn title_present_fires_when_missing() {
        let s = slide(vec![body("B1", vec!["Content"])]);
        let v = TitlePresentRule.check(&[s], T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "TITLE_PRESENT");
        assert!(matches!(v[0].message, ViolationMessage::TitleMissing));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn empty_element_clean() {
        let s = slide(vec![body("B1", vec!["Some text"])]);
        assert!(EmptyElementRule.check(&[s], T).is_empty());
    }

    #[test]
    fn empty_element_fires_on_empty_body() {
        let s = slide(vec![body("B1", vec![])]);
        let v = EmptyElementRule.check(&[s], T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "EMPTY_ELEMENT");
        assert!(matches!(v[0].message, ViolationMessage::EmptyElement));
    }

    #[test]
    fn empty_element_skips_title() {
        // Title with no paragraphs should not fire EMPTY_ELEMENT.
        let mut t = title("T1");
        t.paragraphs.clear();
        let s = slide(vec![t]);
        assert!(EmptyElementRule.check(&[s], T).is_empty());
    }

    #[test]
    fn element_overflow_clean() {
        let s = slide(vec![shape("A", 457_200, 457_200, 3_000_000, 1_000_000)]);
        assert!(ElementOverflowRule.check(&[s], T).is_empty());
    }

    #[test]
    fn element_overflow_fires_on_right_overflow() {
        // x + w > SLIDE_WIDTH_EMU (9_144_000)
        let s = slide(vec![shape("A", 8_000_000, 457_200, 2_000_000, 500_000)]);
        let v = ElementOverflowRule.check(&[s], T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "ELEMENT_OVERFLOW");
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn element_overflow_fires_on_bottom_overflow() {
        // y + h > SLIDE_HEIGHT_EMU (6_858_000)
        let s = slide(vec![shape("A", 457_200, 6_000_000, 1_000_000, 1_500_000)]);
        let v = ElementOverflowRule.check(&[s], T);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn element_overflow_fires_on_negative_x() {
        let s = slide(vec![shape("A", -100_000, 457_200, 1_000_000, 500_000)]);
        let v = ElementOverflowRule.check(&[s], T);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn element_overlap_clean() {
        let s = slide(vec![
            shape("A", 0, 0, 1_000_000, 1_000_000),
            shape("B", 1_000_001, 0, 1_000_000, 1_000_000),
        ]);
        assert!(ElementOverlapRule.check(&[s], T).is_empty());
    }

    #[test]
    fn element_overlap_touching_edges_is_clean() {
        // Touching but not overlapping (strict inequality).
        let s = slide(vec![
            shape("A", 0, 0, 1_000_000, 1_000_000),
            shape("B", 1_000_000, 0, 1_000_000, 1_000_000),
        ]);
        assert!(ElementOverlapRule.check(&[s], T).is_empty());
    }

    #[test]
    fn element_overlap_fires() {
        let s = slide(vec![
            shape("A", 0, 0, 1_000_000, 1_000_000),
            shape("B", 500_000, 500_000, 1_000_000, 1_000_000),
        ]);
        let v = ElementOverlapRule.check(&[s], T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].element.as_deref(), Some("A"));
        assert!(matches!(
            &v[0].message,
            ViolationMessage::ElementOverlap { other_element } if other_element == "B"
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn image_aspect_ratio_clean() {
        // Two images with the same 16:9 ratio.
        let slides = vec![
            slide_n(0, vec![image("I1", 0, 0, 1_600_000, 900_000)]),
            slide_n(1, vec![image("I1", 0, 0, 1_600_000, 900_000)]),
        ];
        assert!(ImageAspectRatioRule.check(&slides, T).is_empty());
    }

    #[test]
    fn image_aspect_ratio_fires_on_outlier() {
        // Two 16:9 images, one square (very different ratio).
        let slides = vec![
            slide_n(0, vec![image("I1", 0, 0, 1_600_000, 900_000)]),
            slide_n(1, vec![image("I2", 0, 0, 1_600_000, 900_000)]),
            slide_n(2, vec![image("I3", 0, 0, 900_000, 900_000)]), // square
        ];
        let v = ImageAspectRatioRule.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn image_aspect_ratio_skips_single_image() {
        let slides = vec![slide_n(0, vec![image("I1", 0, 0, 1_600_000, 900_000)])];
        assert!(ImageAspectRatioRule.check(&slides, T).is_empty());
    }

    #[test]
    fn slide_count_clean() {
        let slides: Vec<SlideData> = (0..20).map(|i| slide_n(i, vec![])).collect();
        assert!(SlideCountRule { limit: 20 }.check(&slides, T).is_empty());
    }

    #[test]
    fn slide_count_fires_over_limit() {
        let slides: Vec<SlideData> = (0..21).map(|i| slide_n(i, vec![])).collect();
        let v = SlideCountRule { limit: 20 }.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert!(v[0].slide.is_none());
        assert!(matches!(
            v[0].message,
            ViolationMessage::SlideCount {
                count: 21,
                limit: 20
            }
        ));
    }
}
