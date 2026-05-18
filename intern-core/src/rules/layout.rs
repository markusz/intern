use crate::model::{EMU_PER_PX, ElementKind, Rect, SlideData, SlideElement};
use crate::rules::{Rule, RuleContext, Severity, Violation, ViolationMessage};

pub struct TitlePresentRule;
pub struct EmptyElementRule;
pub struct ElementOverflowRule;
pub struct TextElementOverlapRule;
pub struct SlideCountRule {
    pub limit: usize,
}

impl Rule for TitlePresentRule {
    fn id(&self) -> &'static str {
        "TITLE_PRESENT"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
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

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if matches!(e.kind, ElementKind::Body | ElementKind::TextBox)
                    && e.paragraphs.is_empty()
                {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.id),
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

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                let r = &e.rect;
                if r.x < 0 || r.y < 0 || r.x + r.w > ctx.slide_width || r.y + r.h > ctx.slide_height
                {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.id),
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

fn element_position_display(e: &SlideElement) -> String {
    let x_px = e.rect.x / EMU_PER_PX;
    let y_px = e.rect.y / EMU_PER_PX;
    format!("{} at ({}px, {}px)", e.kind, x_px, y_px)
}

impl Rule for TextElementOverlapRule {
    fn id(&self) -> &'static str {
        "TEXT_ELEMENT_OVERLAP"
    }

    // Only elements that actually carry text are considered: two text blocks on
    // top of each other is a real problem, whereas decorative shapes, images, and
    // backgrounds overlap by design. Text-free elements are skipped on both sides.
    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let texts: Vec<&SlideElement> = slide
                .elements
                .iter()
                .filter(|e| !e.paragraphs.is_empty())
                .collect();
            for i in 0..texts.len() {
                for j in (i + 1)..texts.len() {
                    if rects_overlap(&texts[i].rect, &texts[j].rect) {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(texts[i].id),
                            message: ViolationMessage::ElementOverlap {
                                other_element: element_position_display(texts[j]),
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

impl Rule for SlideCountRule {
    fn id(&self) -> &'static str {
        "SLIDE_COUNT"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
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
    use crate::model::{ElementKind, Paragraph, ParagraphKind, Rect, SlideData, SlideElement};

    // 2 px threshold; 4:3 slide so the overflow-bound tests keep their geometry.
    const T: RuleContext = RuleContext {
        threshold: 19_050,
        slide_width: 9_144_000,
        slide_height: 6_858_000,
    };

    fn shape(name: &str, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            id: 1,
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
            id: 2,
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
            paragraphs: vec![Paragraph {
                text: "Title".into(),
                kind: ParagraphKind::Plain,
            }],
        }
    }

    fn body(name: &str, paragraphs: Vec<&str>) -> SlideElement {
        SlideElement {
            id: 3,
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
            paragraphs: paragraphs
                .into_iter()
                .map(|s| Paragraph {
                    text: s.to_string(),
                    kind: ParagraphKind::Bullet,
                })
                .collect(),
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
            id: 4,
            name: name.into(),
            kind: ElementKind::Image,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    fn text_box(name: &str, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            paragraphs: vec![Paragraph {
                text: "text".into(),
                kind: ParagraphKind::Plain,
            }],
            ..shape(name, x, y, w, h)
        }
    }

    fn autoshape(name: &str) -> SlideElement {
        SlideElement {
            kind: ElementKind::Autoshape,
            ..shape(name, 0, 0, 1_000_000, 1_000_000)
        }
    }

    #[test]
    fn title_present_clean() {
        let s = slide(vec![title("T1"), body("B1", vec!["Content"])]);
        assert!(TitlePresentRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn title_present_fires_when_missing() {
        let s = slide(vec![body("B1", vec!["Content"])]);
        let v = TitlePresentRule.check(&[s], &T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "TITLE_PRESENT");
        assert!(matches!(v[0].message, ViolationMessage::TitleMissing));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn empty_element_clean() {
        let s = slide(vec![body("B1", vec!["Some text"])]);
        assert!(EmptyElementRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn empty_element_fires_on_empty_body() {
        let s = slide(vec![body("B1", vec![])]);
        let v = EmptyElementRule.check(&[s], &T);
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
        assert!(EmptyElementRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn empty_element_skips_autoshape() {
        // An empty decorative autoshape is not a missing-content issue.
        let s = slide(vec![autoshape("Rectangle 1")]);
        assert!(EmptyElementRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn element_overflow_clean() {
        let s = slide(vec![shape("A", 457_200, 457_200, 3_000_000, 1_000_000)]);
        assert!(ElementOverflowRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn element_overflow_fires_on_right_overflow() {
        // x + w (10_000_000) past the 4:3 slide width (9_144_000).
        let s = slide(vec![shape("A", 8_000_000, 457_200, 2_000_000, 500_000)]);
        let v = ElementOverflowRule.check(&[s], &T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "ELEMENT_OVERFLOW");
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn element_overflow_fires_on_bottom_overflow() {
        // y + h (7_500_000) past the 4:3 slide height (6_858_000).
        let s = slide(vec![shape("A", 457_200, 6_000_000, 1_000_000, 1_500_000)]);
        let v = ElementOverflowRule.check(&[s], &T);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn element_overflow_fires_on_negative_x() {
        let s = slide(vec![shape("A", -100_000, 457_200, 1_000_000, 500_000)]);
        let v = ElementOverflowRule.check(&[s], &T);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn element_overflow_respects_actual_slide_width() {
        // An element well within a 16:9 widescreen deck does not overflow, even
        // though its right edge is past the old hardcoded 4:3 width.
        let wide = RuleContext {
            threshold: 19_050,
            slide_width: 12_192_000,
            slide_height: 6_858_000,
        };
        let s = slide(vec![shape("Banner", 0, 0, 12_192_000, 500_000)]);
        assert!(ElementOverflowRule.check(&[s], &wide).is_empty());
    }

    #[test]
    fn text_element_overlap_clean() {
        let s = slide(vec![
            text_box("A", 0, 0, 1_000_000, 1_000_000),
            text_box("B", 1_000_001, 0, 1_000_000, 1_000_000),
        ]);
        assert!(TextElementOverlapRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn text_element_overlap_touching_edges_is_clean() {
        // Touching but not overlapping (strict inequality).
        let s = slide(vec![
            text_box("A", 0, 0, 1_000_000, 1_000_000),
            text_box("B", 1_000_000, 0, 1_000_000, 1_000_000),
        ]);
        assert!(TextElementOverlapRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn text_element_overlap_fires() {
        let s = slide(vec![
            text_box("A", 0, 0, 1_000_000, 1_000_000),
            text_box("B", 500_000, 500_000, 1_000_000, 1_000_000),
        ]);
        let v = TextElementOverlapRule.check(&[s], &T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].element, Some(1));
        assert!(matches!(
            &v[0].message,
            ViolationMessage::ElementOverlap { other_element } if other_element == "TextBox at (52px, 52px)"
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn text_element_overlap_ignores_images() {
        // Overlapping images carry no text, so they are not flagged.
        let s = slide(vec![
            image("I1", 0, 0, 1_000_000, 1_000_000),
            image("I2", 500_000, 500_000, 1_000_000, 1_000_000),
        ]);
        assert!(TextElementOverlapRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn text_element_overlap_ignores_text_free_shapes() {
        // `shape` has no paragraphs: a text-free shape under a text box is fine.
        let s = slide(vec![
            shape("Decoration", 0, 0, 1_000_000, 1_000_000),
            text_box("Caption", 500_000, 500_000, 1_000_000, 1_000_000),
        ]);
        assert!(TextElementOverlapRule.check(&[s], &T).is_empty());
    }

    #[test]
    fn slide_count_clean() {
        let slides: Vec<SlideData> = (0..20).map(|i| slide_n(i, vec![])).collect();
        assert!(SlideCountRule { limit: 20 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn slide_count_fires_over_limit() {
        let slides: Vec<SlideData> = (0..21).map(|i| slide_n(i, vec![])).collect();
        let v = SlideCountRule { limit: 20 }.check(&slides, &T);
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
