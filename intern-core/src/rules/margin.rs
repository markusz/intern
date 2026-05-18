use std::collections::HashMap;

use crate::model::{EMU_PER_PX, ElementKind, SlideData, SlideElement};
use crate::rules::{Rule, RuleContext, Severity, Violation, ViolationMessage};

use super::median;

// Sub-pixel differences are EMU rounding noise from PowerPoint; ignore them.
const MIN_DIFF_EMU: i64 = EMU_PER_PX;

pub struct LeftMarginRule;
pub struct RightMarginRule;
pub struct BottomMarginRule;
pub struct TitleMarginRule;
pub struct CloseXRule;
pub struct CloseYRule;

// --- shared helpers ----------------------------------------------------------

/// A unit that participates in margin/proximity checks: visible (non-zero area),
/// any kind (Title, Body, TextBox, Autoshape, Image, Group).
fn is_visible(e: &SlideElement) -> bool {
    e.rect.w > 0 && e.rect.h > 0
}

fn unit_desc(e: &SlideElement) -> String {
    let x_px = e.rect.x / EMU_PER_PX;
    let y_px = e.rect.y / EMU_PER_PX;
    format!("{} at ({}px, {}px)", e.kind, x_px, y_px)
}

// --- LEFT_MARGIN -------------------------------------------------------------

impl Rule for LeftMarginRule {
    fn id(&self) -> &'static str {
        "LEFT_MARGIN"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(10)
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let per_slide: Vec<(usize, i64, u32)> = slides
            .iter()
            .filter_map(|s| leftmost_unit(s).map(|e| (s.index, e.rect.x, e.id)))
            .collect();
        if per_slide.len() < 2 {
            return vec![];
        }
        let xs: Vec<i64> = per_slide.iter().map(|(_, x, _)| *x).collect();
        let Some(expected) = median(&xs) else {
            return vec![];
        };
        per_slide
            .iter()
            .filter(|(_, x, _)| (*x - expected).abs() > ctx.threshold)
            .map(|(idx, x, eid)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*eid),
                message: ViolationMessage::LeftMarginOff {
                    actual_emu: *x,
                    expected_emu: expected,
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

fn leftmost_unit(slide: &SlideData) -> Option<&SlideElement> {
    slide
        .units
        .iter()
        .filter(|e| is_visible(e))
        .min_by_key(|e| e.rect.x)
}

// --- RIGHT_MARGIN ------------------------------------------------------------

impl Rule for RightMarginRule {
    fn id(&self) -> &'static str {
        "RIGHT_MARGIN"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(10)
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let per_slide: Vec<(usize, i64, u32)> = slides
            .iter()
            .filter_map(|s| rightmost_unit(s).map(|e| (s.index, e.rect.right(), e.id)))
            .collect();
        if per_slide.len() < 2 {
            return vec![];
        }
        let rights: Vec<i64> = per_slide.iter().map(|(_, r, _)| *r).collect();
        let Some(expected) = median(&rights) else {
            return vec![];
        };
        per_slide
            .iter()
            .filter(|(_, r, _)| (*r - expected).abs() > ctx.threshold)
            .map(|(idx, r, eid)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*eid),
                message: ViolationMessage::RightMarginOff {
                    actual_emu: *r,
                    expected_emu: expected,
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

fn rightmost_unit(slide: &SlideData) -> Option<&SlideElement> {
    slide
        .units
        .iter()
        .filter(|e| is_visible(e))
        .max_by_key(|e| e.rect.right())
}

// --- BOTTOM_MARGIN -----------------------------------------------------------

impl Rule for BottomMarginRule {
    fn id(&self) -> &'static str {
        "BOTTOM_MARGIN"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(10)
    }

    // Only flags overflow (deeper than typical), not underflow.
    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let per_slide: Vec<(usize, i64, u32)> = slides
            .iter()
            .filter_map(|s| deepest_unit(s).map(|e| (s.index, e.rect.bottom(), e.id)))
            .collect();
        if per_slide.len() < 2 {
            return vec![];
        }
        let bottoms: Vec<i64> = per_slide.iter().map(|(_, b, _)| *b).collect();
        let Some(expected) = median(&bottoms) else {
            return vec![];
        };
        per_slide
            .iter()
            .filter(|(_, b, _)| *b - expected > ctx.threshold)
            .map(|(idx, b, eid)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*eid),
                message: ViolationMessage::BottomMarginOff {
                    actual_emu: *b,
                    expected_emu: expected,
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

fn deepest_unit(slide: &SlideData) -> Option<&SlideElement> {
    slide
        .units
        .iter()
        .filter(|e| is_visible(e))
        .max_by_key(|e| e.rect.bottom())
}

// --- TITLE_MARGIN ------------------------------------------------------------

impl Rule for TitleMarginRule {
    fn id(&self) -> &'static str {
        "TITLE_MARGIN"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(5)
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let per_slide: Vec<(usize, i64, u32)> = slides
            .iter()
            .filter_map(|s| title_gap(s).map(|(gap, eid)| (s.index, gap, eid)))
            .collect();
        if per_slide.len() < 2 {
            return vec![];
        }
        let gaps: Vec<i64> = per_slide.iter().map(|(_, g, _)| *g).collect();
        let Some(expected) = median(&gaps) else {
            return vec![];
        };
        per_slide
            .iter()
            .filter(|(_, g, _)| (*g - expected).abs() > ctx.threshold)
            .map(|(idx, g, eid)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*eid),
                message: ViolationMessage::TitleGapOff {
                    actual_emu: *g,
                    expected_emu: expected,
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

/// Returns `(gap_emu, element_id)` for the nearest unit below the title.
/// `gap_emu` is the distance from the title's bottom edge to that unit's top edge.
fn title_gap(slide: &SlideData) -> Option<(i64, u32)> {
    let title = slide
        .elements
        .iter()
        .find(|e| e.kind == ElementKind::Title)?;
    let title_bottom = title.rect.bottom();
    slide
        .units
        .iter()
        .filter(|u| u.kind != ElementKind::Title && is_visible(u) && u.rect.y >= title_bottom)
        .map(|u| (u.rect.y - title_bottom, u.id))
        .min_by_key(|(gap, _)| *gap)
}

// --- CLOSE_X -----------------------------------------------------------------

impl Rule for CloseXRule {
    fn id(&self) -> &'static str {
        "CLOSE_X"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(5)
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let units: Vec<&SlideElement> = slide.units.iter().filter(|e| is_visible(e)).collect();
            close_edge_violations(
                &units,
                |e| e.rect.x,
                ctx.threshold,
                slide.index,
                self.id(),
                |diff_emu, other| ViolationMessage::CloseX {
                    diff_emu,
                    other_element: other,
                },
                &mut violations,
            );
        }
        violations
    }
}

// --- CLOSE_Y -----------------------------------------------------------------

impl Rule for CloseYRule {
    fn id(&self) -> &'static str {
        "CLOSE_Y"
    }

    fn default_threshold_px(&self) -> Option<u32> {
        Some(5)
    }

    fn check(&self, slides: &[SlideData], ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            let units: Vec<&SlideElement> = slide.units.iter().filter(|e| is_visible(e)).collect();
            close_edge_violations(
                &units,
                |e| e.rect.y,
                ctx.threshold,
                slide.index,
                self.id(),
                |diff_emu, other| ViolationMessage::CloseY {
                    diff_emu,
                    other_element: other,
                },
                &mut violations,
            );
        }
        violations
    }
}

/// Emits at most one violation per element `j`: the largest-diff near-miss
/// partner `i` (i < j) whose edge coordinate differs by [MIN_DIFF_EMU, threshold].
/// Sub-pixel differences are skipped as EMU rounding noise.
fn close_edge_violations(
    units: &[&SlideElement],
    edge: impl Fn(&SlideElement) -> i64,
    threshold: i64,
    slide_index: usize,
    rule_id: &'static str,
    make_msg: impl Fn(i64, String) -> ViolationMessage,
    out: &mut Vec<Violation>,
) {
    // j element id -> (max_diff, i index)
    let mut best: HashMap<u32, (i64, usize)> = HashMap::new();
    for i in 0..units.len() {
        for j in (i + 1)..units.len() {
            let diff = (edge(units[i]) - edge(units[j])).abs();
            if diff < MIN_DIFF_EMU || diff > threshold {
                continue;
            }
            let entry = best.entry(units[j].id).or_insert((0, i));
            if diff > entry.0 {
                *entry = (diff, i);
            }
        }
    }
    // Emit in document order (j index ascending).
    for j in 0..units.len() {
        if let Some(&(diff, i)) = best.get(&units[j].id) {
            out.push(Violation {
                rule_id,
                slide: Some(slide_index + 1),
                element: Some(units[j].id),
                message: make_msg(diff, unit_desc(units[i])),
                severity: Severity::Warning,
                fix: None,
            });
        }
    }
}

// --- tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Paragraph, ParagraphKind, Rect};

    const T10: RuleContext = RuleContext {
        threshold: 10 * 9_525,
        slide_width: 12_192_000,
        slide_height: 6_858_000,
    };
    const T5: RuleContext = RuleContext {
        threshold: 5 * 9_525,
        slide_width: 12_192_000,
        slide_height: 6_858_000,
    };

    fn unit(id: u32, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            id,
            name: format!("El{id}"),
            kind: ElementKind::TextBox,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![Paragraph {
                text: "text".into(),
                kind: ParagraphKind::Plain,
            }],
        }
    }

    fn group_unit(id: u32, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            id,
            name: format!("Group{id}"),
            kind: ElementKind::Group,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    fn title_el(id: u32, y: i64, h: i64) -> SlideElement {
        SlideElement {
            id,
            name: "Title".into(),
            kind: ElementKind::Title,
            rect: Rect {
                x: 457_200,
                y,
                w: 8_229_600,
                h,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![Paragraph {
                text: "Title text".into(),
                kind: ParagraphKind::Plain,
            }],
        }
    }

    fn slide_with_units(index: usize, units: Vec<SlideElement>) -> SlideData {
        SlideData {
            index,
            elements: vec![],
            units,
        }
    }

    fn slide_with_title_and_units(
        index: usize,
        title: SlideElement,
        units: Vec<SlideElement>,
    ) -> SlideData {
        SlideData {
            index,
            elements: vec![title.clone()],
            units,
        }
    }

    // -- LEFT_MARGIN --

    #[test]
    fn left_margin_clean_when_consistent() {
        let x = 457_200i64;
        let slides = vec![
            slide_with_units(0, vec![unit(1, x, 500_000, 1_000_000, 500_000)]),
            slide_with_units(1, vec![unit(2, x, 500_000, 1_000_000, 500_000)]),
            slide_with_units(2, vec![unit(3, x, 500_000, 1_000_000, 500_000)]),
        ];
        assert!(LeftMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn left_margin_fires_on_outlier() {
        let x = 457_200i64;
        let shifted = x + 200_000; // ~21px off
        let slides = vec![
            slide_with_units(0, vec![unit(1, x, 500_000, 1_000_000, 500_000)]),
            slide_with_units(1, vec![unit(2, x, 500_000, 1_000_000, 500_000)]),
            slide_with_units(2, vec![unit(3, shifted, 500_000, 1_000_000, 500_000)]),
        ];
        let v = LeftMarginRule.check(&slides, &T10);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
        assert!(matches!(
            v[0].message,
            ViolationMessage::LeftMarginOff { .. }
        ));
    }

    #[test]
    fn left_margin_clean_with_sub_threshold_deviation() {
        let x = 457_200i64;
        let slides = vec![
            slide_with_units(0, vec![unit(1, x, 500_000, 1_000_000, 500_000)]),
            slide_with_units(1, vec![unit(2, x + 9_000, 500_000, 1_000_000, 500_000)]),
        ];
        assert!(LeftMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn left_margin_skips_zero_area_units() {
        let x = 457_200i64;
        let slides = vec![
            slide_with_units(
                0,
                vec![
                    unit(1, x, 500_000, 1_000_000, 500_000),
                    unit(2, 0, 0, 0, 0), // degenerate, should be ignored
                ],
            ),
            slide_with_units(1, vec![unit(3, x, 500_000, 1_000_000, 500_000)]),
        ];
        assert!(LeftMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn left_margin_counts_groups() {
        // A group that is the leftmost unit should define the margin.
        let x = 457_200i64;
        let slides = vec![
            slide_with_units(0, vec![group_unit(1, x, 500_000, 2_000_000, 1_000_000)]),
            slide_with_units(1, vec![group_unit(2, x, 500_000, 2_000_000, 1_000_000)]),
            slide_with_units(
                2,
                vec![group_unit(3, x + 300_000, 500_000, 2_000_000, 1_000_000)],
            ),
        ];
        let v = LeftMarginRule.check(&slides, &T10);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
    }

    // -- RIGHT_MARGIN --

    #[test]
    fn right_margin_clean_when_consistent() {
        let right = 11_735_000i64; // x + w
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, right - 457_200, 500_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, right - 457_200, 500_000)]),
        ];
        assert!(RightMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn right_margin_fires_on_outlier() {
        let right = 11_735_000i64;
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, right - 457_200, 500_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, right - 457_200, 500_000)]),
            slide_with_units(
                2,
                vec![unit(
                    3,
                    457_200,
                    500_000,
                    right - 457_200 + 300_000,
                    500_000,
                )],
            ),
        ];
        let v = RightMarginRule.check(&slides, &T10);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
    }

    // -- BOTTOM_MARGIN --

    #[test]
    fn bottom_margin_clean_when_consistent() {
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, 1_000_000, 5_000_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, 1_000_000, 5_000_000)]),
        ];
        assert!(BottomMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn bottom_margin_only_flags_overflow_not_underflow() {
        // Slide 2 is shallower - should NOT fire.
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, 1_000_000, 5_000_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, 1_000_000, 5_000_000)]),
            slide_with_units(2, vec![unit(3, 457_200, 500_000, 1_000_000, 3_000_000)]),
        ];
        assert!(BottomMarginRule.check(&slides, &T10).is_empty());
    }

    #[test]
    fn bottom_margin_fires_on_deeper_slide() {
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, 1_000_000, 5_000_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, 1_000_000, 5_000_000)]),
            slide_with_units(
                2,
                vec![unit(3, 457_200, 500_000, 1_000_000, 5_000_000 + 300_000)],
            ),
        ];
        let v = BottomMarginRule.check(&slides, &T10);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
    }

    // -- TITLE_MARGIN --

    #[test]
    fn title_margin_clean_when_consistent() {
        let title_h = 1_143_000i64;
        let title_y = 274_638i64;
        let content_y = title_y + title_h + 200_000;
        let slides = vec![
            slide_with_title_and_units(
                0,
                title_el(1, title_y, title_h),
                vec![
                    title_el(1, title_y, title_h),
                    unit(2, 457_200, content_y, 8_000_000, 3_000_000),
                ],
            ),
            slide_with_title_and_units(
                1,
                title_el(1, title_y, title_h),
                vec![
                    title_el(1, title_y, title_h),
                    unit(2, 457_200, content_y, 8_000_000, 3_000_000),
                ],
            ),
        ];
        assert!(TitleMarginRule.check(&slides, &T5).is_empty());
    }

    #[test]
    fn title_margin_fires_on_outlier_gap() {
        let title_h = 1_143_000i64;
        let title_y = 274_638i64;
        let content_y = title_y + title_h + 200_000;
        let far_y = content_y + 600_000; // ~63px more gap
        let slides = vec![
            slide_with_title_and_units(
                0,
                title_el(1, title_y, title_h),
                vec![
                    title_el(1, title_y, title_h),
                    unit(2, 457_200, content_y, 8_000_000, 3_000_000),
                ],
            ),
            slide_with_title_and_units(
                1,
                title_el(1, title_y, title_h),
                vec![
                    title_el(1, title_y, title_h),
                    unit(2, 457_200, content_y, 8_000_000, 3_000_000),
                ],
            ),
            slide_with_title_and_units(
                2,
                title_el(1, title_y, title_h),
                vec![
                    title_el(1, title_y, title_h),
                    unit(3, 457_200, far_y, 8_000_000, 3_000_000),
                ],
            ),
        ];
        let v = TitleMarginRule.check(&slides, &T5);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
    }

    #[test]
    fn title_margin_skips_slides_without_title() {
        let slides = vec![
            slide_with_units(0, vec![unit(1, 457_200, 500_000, 1_000_000, 500_000)]),
            slide_with_units(1, vec![unit(2, 457_200, 500_000, 1_000_000, 500_000)]),
        ];
        assert!(TitleMarginRule.check(&slides, &T5).is_empty());
    }

    // -- CLOSE_X --

    #[test]
    fn close_x_clean_when_aligned() {
        let x = 457_200i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, x, 500_000, 1_000_000, 500_000),
                unit(2, x, 1_100_000, 1_000_000, 500_000),
            ],
        };
        assert!(CloseXRule.check(&[slide], &T5).is_empty());
    }

    #[test]
    fn close_x_clean_when_far_apart() {
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, 457_200, 500_000, 1_000_000, 500_000),
                unit(2, 6_000_000, 500_000, 1_000_000, 500_000),
            ],
        };
        assert!(CloseXRule.check(&[slide], &T5).is_empty());
    }

    #[test]
    fn close_x_fires_on_near_miss() {
        let x = 457_200i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, x, 500_000, 1_000_000, 500_000),
                unit(2, x + 3 * 9_525, 1_100_000, 1_000_000, 500_000), // 3px off
            ],
        };
        let v = CloseXRule.check(&[slide], &T5);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].element, Some(2));
        assert!(matches!(v[0].message, ViolationMessage::CloseX { .. }));
    }

    #[test]
    fn close_x_one_finding_per_pair_not_both_directions() {
        let x = 457_200i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, x, 500_000, 1_000_000, 500_000),
                unit(2, x + 9_525, 1_100_000, 1_000_000, 500_000), // 1px off
            ],
        };
        // Only one finding: element j=2, not both (1,2) and (2,1).
        assert_eq!(CloseXRule.check(&[slide], &T5).len(), 1);
    }

    // -- CLOSE_Y --

    #[test]
    fn close_y_clean_when_aligned() {
        let y = 500_000i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, 457_200, y, 1_000_000, 500_000),
                unit(2, 6_000_000, y, 1_000_000, 500_000),
            ],
        };
        assert!(CloseYRule.check(&[slide], &T5).is_empty());
    }

    #[test]
    fn close_y_fires_on_near_miss() {
        let y = 500_000i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, 457_200, y, 1_000_000, 500_000),
                unit(2, 6_000_000, y + 2 * 9_525, 1_000_000, 500_000), // 2px off
            ],
        };
        let v = CloseYRule.check(&[slide], &T5);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].element, Some(2));
        assert!(matches!(v[0].message, ViolationMessage::CloseY { .. }));
    }

    #[test]
    fn close_y_ignores_sub_pixel_noise() {
        // 0.5px difference is EMU rounding - should not fire.
        let y = 500_000i64;
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units: vec![
                unit(1, 457_200, y, 1_000_000, 500_000),
                unit(2, 6_000_000, y + 9_525 / 2, 1_000_000, 500_000),
            ],
        };
        assert!(CloseYRule.check(&[slide], &T5).is_empty());
    }

    #[test]
    fn close_y_one_finding_per_element_not_one_per_pair() {
        // Seven images nearly at the same Y (3px offset from a reference row):
        // the naive O(n^2) loop emits n*(n-1)/2 pairs; we want at most n findings,
        // and each flagged element appears only once.
        let y_base = 5_543_100i64; // ~582px
        let y_ref = y_base - 3 * 9_525; // ~579px - reference row
        let mut units = vec![unit(10, 0, y_ref, 500_000, 200_000)];
        for id in 1u32..=6 {
            units.push(unit(id, (id as i64) * 600_000, y_base, 500_000, 200_000));
        }
        let slide = SlideData {
            index: 0,
            elements: vec![],
            units,
        };
        let v = CloseYRule.check(&[slide], &T5);
        // Each of the 6 near-miss elements gets at most one finding.
        assert!(v.len() <= 6, "got {} findings, expected <= 6", v.len());
        // No element id appears twice.
        let mut seen = std::collections::HashSet::new();
        for violation in &v {
            assert!(
                seen.insert(violation.element),
                "duplicate element in CLOSE_Y findings"
            );
        }
    }
}
