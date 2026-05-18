use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::model::{ElementKind, ParagraphKind, SlideData, SlideElement};
use crate::rules::{Fix, Rule, RuleContext, Severity, Violation, ViolationMessage};

pub struct FontSizeVariationRule {
    pub limit: usize,
}
pub struct BodyFontFamilyRule;
pub struct BodyTextColorRule;
pub struct DoubleSpaceRule;
pub struct LeadingSpaceRule;
pub struct BulletCapitalizationRule;
pub struct AllCapsRule;
pub struct BulletPunctuationRule;
pub struct BulletLengthRule {
    pub limit: usize,
}
pub struct RepeatedWordRule;
pub struct FontVarietyRule {
    pub limit: usize,
}
pub struct ColorVarietyRule {
    pub limit: usize,
}

// Text-bearing element kinds: body placeholders, genuine text boxes, and
// autoshapes (which may carry text). Excludes titles and images.
fn is_text_element(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Body | ElementKind::TextBox | ElementKind::Autoshape
    )
}

fn body_elements(slides: &[SlideData]) -> Vec<(usize, &SlideElement)> {
    slides
        .iter()
        .flat_map(|s| {
            s.elements
                .iter()
                .filter(|e| is_text_element(&e.kind))
                .map(move |e| (s.index, e))
        })
        .collect()
}

fn mode<T: Eq + Hash + Clone>(values: &[T]) -> Option<T> {
    let mut counts: HashMap<T, usize> = HashMap::new();
    for v in values {
        *counts.entry(v.clone()).or_default() += 1;
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v)
}

impl Rule for FontSizeVariationRule {
    fn id(&self) -> &'static str {
        "FONT_SIZE_VARIETY"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let sizes: HashSet<u32> = body_elements(slides)
            .into_iter()
            .filter_map(|(_, e)| e.font_size)
            .collect();
        if sizes.len() <= self.limit {
            return vec![];
        }
        vec![Violation {
            rule_id: self.id(),
            slide: None,
            element: None,
            message: ViolationMessage::FontSizeVariation {
                count: sizes.len(),
                limit: self.limit,
            },
            severity: Severity::Warning,
            fix: None,
        }]
    }
}

impl Rule for BodyFontFamilyRule {
    fn id(&self) -> &'static str {
        "BODY_FONT_FAMILY"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let es = body_elements(slides);
        let familied: Vec<(usize, u32, String)> = es
            .iter()
            .filter_map(|(idx, e)| e.font_family.as_ref().map(|f| (*idx, e.id, f.clone())))
            .collect();

        if familied.len() < 2 {
            return vec![];
        }

        let families: Vec<String> = familied.iter().map(|(_, _, f)| f.clone()).collect();
        // SAFETY: familied.len() >= 2 above; mode returns None only for empty input.
        let expected = mode(&families).unwrap_or_else(|| unreachable!());

        familied
            .iter()
            .filter(|(_, _, f)| f != &expected)
            .map(|(idx, id, actual)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*id),
                message: ViolationMessage::BodyFontFamily {
                    actual: actual.clone(),
                    expected: expected.clone(),
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

impl Rule for BodyTextColorRule {
    fn id(&self) -> &'static str {
        "BODY_TEXT_COLOR"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let es = body_elements(slides);
        let colored: Vec<(usize, u32, String)> = es
            .iter()
            .filter_map(|(idx, e)| e.text_color.as_ref().map(|c| (*idx, e.id, c.clone())))
            .collect();

        if colored.len() < 2 {
            return vec![];
        }

        let colors: Vec<String> = colored.iter().map(|(_, _, c)| c.clone()).collect();
        // SAFETY: colored.len() >= 2 above; mode returns None only for empty input.
        let expected = mode(&colors).unwrap_or_else(|| unreachable!());

        colored
            .iter()
            .filter(|(_, _, c)| c != &expected)
            .map(|(idx, id, actual)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(*id),
                message: ViolationMessage::BodyTextColor {
                    actual: actual.clone(),
                    expected: expected.clone(),
                },
                severity: Severity::Warning,
                fix: None,
            })
            .collect()
    }
}

impl Rule for DoubleSpaceRule {
    fn id(&self) -> &'static str {
        "DOUBLE_SPACE"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if e.paragraphs.iter().any(|p| p.text.contains("  ")) {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.id),
                        message: ViolationMessage::DoubleSpace,
                        severity: Severity::Warning,
                        fix: Some(Fix::NormalizeWhitespace {
                            slide_idx: slide.index,
                            element_name: e.name.clone(),
                        }),
                    });
                }
            }
        }
        violations
    }
}

impl Rule for LeadingSpaceRule {
    fn id(&self) -> &'static str {
        "LEADING_SPACE"
    }

    // Only leading whitespace is flagged: it indents visible text. Trailing
    // whitespace is kept in the file by PowerPoint but renders nothing on the
    // slide, so flagging it is just noise.
    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if e.paragraphs
                    .iter()
                    .any(|p| p.text.starts_with(|c: char| c.is_whitespace()))
                {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.id),
                        message: ViolationMessage::LeadingSpace,
                        severity: Severity::Warning,
                        fix: Some(Fix::NormalizeWhitespace {
                            slide_idx: slide.index,
                            element_name: e.name.clone(),
                        }),
                    });
                }
            }
        }
        violations
    }
}

fn is_majority(count: usize, total: usize) -> bool {
    count > total / 2
}

fn first_alpha(s: &str) -> Option<char> {
    s.chars().find(|c| c.is_alphabetic())
}

fn bullet_capitalizations(
    slide_idx: usize,
    e: &SlideElement,
) -> impl Iterator<Item = (bool, usize, u32)> {
    let id = e.id;
    e.paragraphs
        .iter()
        .filter(|p| p.kind == ParagraphKind::Bullet)
        .filter_map(move |p| first_alpha(&p.text).map(|c| (c.is_uppercase(), slide_idx, id)))
}

impl Rule for BulletCapitalizationRule {
    fn id(&self) -> &'static str {
        "BULLET_CAPITALIZATION"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let es = body_elements(slides);

        let all: Vec<(bool, usize, u32)> = es
            .iter()
            .flat_map(|(idx, e)| bullet_capitalizations(*idx, e))
            .collect();

        if all.len() < 2 {
            return vec![];
        }

        let upper_count = all.iter().filter(|(u, _, _)| *u).count();
        let majority_upper = is_majority(upper_count, all.len());

        // One violation per element that has any paragraph deviating from the majority.
        let mut seen = HashSet::new();
        let mut violations = Vec::new();
        for (is_upper, idx, id) in &all {
            if *is_upper != majority_upper && seen.insert((*idx, *id)) {
                violations.push(Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(*id),
                    message: ViolationMessage::BulletCapitalization {
                        expected_uppercase: majority_upper,
                    },
                    severity: Severity::Warning,
                    fix: None,
                });
            }
        }
        violations
    }
}

impl Rule for AllCapsRule {
    fn id(&self) -> &'static str {
        "ALL_CAPS"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if !is_text_element(&e.kind) {
                    continue;
                }
                let has_all_caps = e.paragraphs.iter().any(|p| {
                    let mut alpha = p.text.chars().filter(|c| c.is_alphabetic()).peekable();
                    alpha.peek().is_some() && alpha.all(|c| c.is_uppercase())
                });
                if has_all_caps {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.id),
                        message: ViolationMessage::AllCaps,
                        severity: Severity::Warning,
                        fix: None,
                    });
                }
            }
        }
        violations
    }
}

fn ends_with_punct(s: &str) -> bool {
    matches!(s.trim_end().chars().last(), Some('.' | '!' | '?' | ':'))
}

impl Rule for BulletPunctuationRule {
    fn id(&self) -> &'static str {
        "BULLET_PUNCTUATION"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let es = body_elements(slides);
        let bullets: Vec<(bool, usize, u32)> = es
            .iter()
            .flat_map(|(idx, e)| {
                let id = e.id;
                e.paragraphs
                    .iter()
                    .filter(|p| p.kind == ParagraphKind::Bullet)
                    .map(move |p| (ends_with_punct(&p.text), *idx, id))
            })
            .collect();

        if bullets.len() < 2 {
            return vec![];
        }

        let punct_count = bullets.iter().filter(|(p, _, _)| *p).count();
        let majority_punct = is_majority(punct_count, bullets.len());

        let mut seen = HashSet::new();
        let mut violations = Vec::new();
        for (has_punct, idx, id) in &bullets {
            if *has_punct != majority_punct && seen.insert((*idx, *id)) {
                violations.push(Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(*id),
                    message: ViolationMessage::BulletPunctuation {
                        expected_punctuation: majority_punct,
                    },
                    severity: Severity::Warning,
                    fix: None,
                });
            }
        }
        violations
    }
}

impl Rule for BulletLengthRule {
    fn id(&self) -> &'static str {
        "BULLET_LENGTH"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if !is_text_element(&e.kind) {
                    continue;
                }
                for p in e
                    .paragraphs
                    .iter()
                    .filter(|p| p.kind == ParagraphKind::Bullet)
                {
                    let word_count = p.text.split_whitespace().count();
                    if word_count > self.limit {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(e.id),
                            message: ViolationMessage::BulletTooLong {
                                word_count,
                                limit: self.limit,
                            },
                            severity: Severity::Warning,
                            fix: None,
                        });
                        break;
                    }
                }
            }
        }
        violations
    }
}

fn word_key(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

impl Rule for RepeatedWordRule {
    fn id(&self) -> &'static str {
        "REPEATED_WORD"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                'para: for p in &e.paragraphs {
                    let words: Vec<String> = p.text.split_whitespace().map(word_key).collect();
                    for w in words.windows(2) {
                        if !w[0].is_empty() && w[0] == w[1] {
                            violations.push(Violation {
                                rule_id: self.id(),
                                slide: Some(slide.index + 1),
                                element: Some(e.id),
                                message: ViolationMessage::RepeatedWord { word: w[0].clone() },
                                severity: Severity::Warning,
                                fix: None,
                            });
                            break 'para;
                        }
                    }
                }
            }
        }
        violations
    }
}

impl Rule for FontVarietyRule {
    fn id(&self) -> &'static str {
        "FONT_VARIETY"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let fonts: HashSet<String> = slides
            .iter()
            .flat_map(|s| s.elements.iter())
            .filter_map(|e| e.font_family.clone())
            .collect();
        if fonts.len() <= self.limit {
            return vec![];
        }
        vec![Violation {
            rule_id: self.id(),
            slide: None,
            element: None,
            message: ViolationMessage::FontVariety {
                count: fonts.len(),
                limit: self.limit,
            },
            severity: Severity::Warning,
            fix: None,
        }]
    }
}

impl Rule for ColorVarietyRule {
    fn id(&self) -> &'static str {
        "COLOR_VARIETY"
    }

    fn check(&self, slides: &[SlideData], _ctx: &RuleContext) -> Vec<Violation> {
        let colors: HashSet<String> = slides
            .iter()
            .flat_map(|s| s.elements.iter())
            .filter_map(|e| e.text_color.clone())
            .collect();
        if colors.len() <= self.limit {
            return vec![];
        }
        vec![Violation {
            rule_id: self.id(),
            slide: None,
            element: None,
            message: ViolationMessage::ColorVariety {
                count: colors.len(),
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

    const T: RuleContext = RuleContext {
        threshold: 19_050,
        slide_width: 9_144_000,
        slide_height: 6_858_000,
    };

    fn body(name: &str, font_size: Option<u32>, font_family: Option<&str>) -> SlideElement {
        body_with(name, font_size, font_family, None, vec![])
    }

    fn body_with(
        name: &str,
        font_size: Option<u32>,
        font_family: Option<&str>,
        text_color: Option<&str>,
        paragraphs: Vec<&str>,
    ) -> SlideElement {
        SlideElement {
            id: 1,
            name: name.into(),
            kind: ElementKind::Body,
            rect: Rect {
                x: 100_000,
                y: 100_000,
                w: 3_000_000,
                h: 1_000_000,
            },
            font_size,
            font_family: font_family.map(str::to_string),
            text_color: text_color.map(str::to_string),
            paragraphs: paragraphs
                .into_iter()
                .map(|s| Paragraph {
                    text: s.to_string(),
                    kind: ParagraphKind::Bullet,
                })
                .collect(),
        }
    }

    fn autoshape_with(name: &str, paragraphs: Vec<&str>) -> SlideElement {
        SlideElement {
            id: 2,
            name: name.into(),
            kind: ElementKind::Autoshape,
            rect: Rect {
                x: 100_000,
                y: 100_000,
                w: 3_000_000,
                h: 1_000_000,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: paragraphs
                .into_iter()
                .map(|s| Paragraph {
                    text: s.to_string(),
                    kind: ParagraphKind::Plain,
                })
                .collect(),
        }
    }

    fn slide(idx: usize, elements: Vec<SlideElement>) -> SlideData {
        SlideData {
            index: idx,
            elements,
            units: vec![],
        }
    }

    #[test]
    fn font_size_variety_clean_within_limit() {
        let slides = vec![
            slide(0, vec![body("B1", Some(2_400), None)]),
            slide(1, vec![body("B1", Some(1_800), None)]),
            slide(2, vec![body("B1", Some(2_000), None)]),
        ];
        assert!(
            FontSizeVariationRule { limit: 3 }
                .check(&slides, &T)
                .is_empty()
        );
    }

    #[test]
    fn font_size_variety_fires_over_limit() {
        let slides = vec![
            slide(0, vec![body("B1", Some(2_400), None)]),
            slide(1, vec![body("B1", Some(1_800), None)]),
            slide(2, vec![body("B1", Some(2_000), None)]),
            slide(3, vec![body("B1", Some(1_400), None)]),
        ];
        let v = FontSizeVariationRule { limit: 3 }.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(v[0].slide.is_none());
        assert!(matches!(
            v[0].message,
            ViolationMessage::FontSizeVariation { count: 4, limit: 3 }
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn font_size_variety_skips_elements_without_size() {
        let slides = vec![
            slide(0, vec![body("B1", None, None)]),
            slide(1, vec![body("B1", None, None)]),
            slide(2, vec![body("B1", None, None)]),
            slide(3, vec![body("B1", None, None)]),
        ];
        assert!(
            FontSizeVariationRule { limit: 3 }
                .check(&slides, &T)
                .is_empty()
        );
    }

    #[test]
    fn font_size_variety_ignores_title_elements() {
        let mut title = body("T", Some(4_400), None);
        title.kind = ElementKind::Title;
        let slides = vec![
            slide(0, vec![title.clone(), body("B1", Some(2_400), None)]),
            slide(1, vec![title.clone(), body("B1", Some(1_800), None)]),
            slide(2, vec![title.clone(), body("B1", Some(2_000), None)]),
            slide(3, vec![title.clone(), body("B1", Some(1_400), None)]),
        ];
        // Title's 4400 must not be counted; only 4 body sizes, which fires.
        let v = FontSizeVariationRule { limit: 3 }.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::FontSizeVariation { count: 4, .. }
        ));
    }

    #[test]
    fn font_size_variety_deduplicates_same_size() {
        // Same size on multiple slides counts as one distinct size.
        let slides = vec![
            slide(0, vec![body("B1", Some(2_400), None)]),
            slide(1, vec![body("B1", Some(2_400), None)]),
        ];
        assert!(
            FontSizeVariationRule { limit: 3 }
                .check(&slides, &T)
                .is_empty()
        );
    }

    #[test]
    fn body_font_family_clean() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Calibri"))]),
        ];
        assert!(BodyFontFamilyRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn body_font_family_fires_on_outlier() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Calibri"))]),
            slide(2, vec![body("B1", None, Some("Arial"))]), // outlier
        ];
        let v = BodyFontFamilyRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0].message,
            ViolationMessage::BodyFontFamily { actual, expected }
                if actual == "Arial" && expected == "Calibri"
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn body_font_family_skips_elements_without_family() {
        let slides = vec![
            slide(0, vec![body("B1", None, None)]),
            slide(1, vec![body("B1", None, None)]),
        ];
        assert!(BodyFontFamilyRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn body_text_color_clean() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("000000"), vec![])]),
        ];
        assert!(BodyTextColorRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn body_text_color_fires_on_outlier() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
        ];
        let v = BodyTextColorRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0].message,
            ViolationMessage::BodyTextColor { actual, expected }
                if actual == "FF0000" && expected == "000000"
        ));
    }

    #[test]
    fn double_space_clean() {
        let slides = vec![slide(
            0,
            vec![body_with(
                "B1",
                None,
                None,
                None,
                vec!["No double spaces here"],
            )],
        )];
        assert!(DoubleSpaceRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn double_space_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Two  spaces"])],
        )];
        let v = DoubleSpaceRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].message, ViolationMessage::DoubleSpace));
        assert!(matches!(v[0].fix, Some(Fix::NormalizeWhitespace { .. })));
    }

    #[test]
    fn leading_space_clean() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Clean text"])],
        )];
        assert!(LeadingSpaceRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn leading_space_ignores_trailing_space() {
        // Trailing whitespace is invisible on a slide, so it is not flagged.
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["trailing space "])],
        )];
        assert!(LeadingSpaceRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn leading_space_fires_on_leading() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec![" leading space"])],
        )];
        let v = LeadingSpaceRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].message, ViolationMessage::LeadingSpace));
        assert!(matches!(v[0].fix, Some(Fix::NormalizeWhitespace { .. })));
    }

    #[test]
    fn bullet_capitalization_skips_autoshapes() {
        // "Rechteck 6" is an autoshape label, not a bullet list.
        let slides = vec![slide(
            0,
            vec![
                body_with("B1", None, None, None, vec!["First", "Second"]),
                autoshape_with("Rechteck 6", vec!["umsatz- ziel 2018"]),
            ],
        )];
        assert!(BulletCapitalizationRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_capitalization_clean() {
        let slides = vec![
            slide(
                0,
                vec![body_with("B1", None, None, None, vec!["First", "Second"])],
            ),
            slide(1, vec![body_with("B1", None, None, None, vec!["Third"])]),
        ];
        assert!(BulletCapitalizationRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_capitalization_fires_on_lowercase_outlier() {
        let slides = vec![
            slide(
                0,
                vec![body_with("B1", None, None, None, vec!["First", "Second"])],
            ),
            slide(1, vec![body_with("B1", None, None, None, vec!["Third"])]),
            slide(
                2,
                vec![body_with("B1", None, None, None, vec!["lowercase outlier"])],
            ),
        ];
        let v = BulletCapitalizationRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::BulletCapitalization {
                expected_uppercase: true
            }
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn bullet_capitalization_skips_non_alpha_starts() {
        // Paragraphs starting with numbers/punctuation are ignored.
        let slides = vec![
            slide(
                0,
                vec![body_with(
                    "B1",
                    None,
                    None,
                    None,
                    vec!["1. First", "2. Second"],
                )],
            ),
            slide(1, vec![body_with("B1", None, None, None, vec!["3. Third"])]),
        ];
        // First alpha in "1. First" is 'F' (uppercase) - should be clean.
        assert!(BulletCapitalizationRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn all_caps_clean() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Normal text here"])],
        )];
        assert!(AllCapsRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn all_caps_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["SHOUTING TEXT"])],
        )];
        let v = AllCapsRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].message, ViolationMessage::AllCaps));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn all_caps_skips_pure_punctuation() {
        // No alphabetic chars → not ALL CAPS.
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["123 !@#"])],
        )];
        assert!(AllCapsRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_punctuation_skips_autoshapes() {
        let slides = vec![slide(
            0,
            vec![
                body_with("B1", None, None, None, vec!["First.", "Second.", "Third."]),
                autoshape_with("Box 1", vec!["no trailing punct"]),
            ],
        )];
        assert!(BulletPunctuationRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_punctuation_clean_with_punct() {
        let slides = vec![
            slide(
                0,
                vec![body_with(
                    "B1",
                    None,
                    None,
                    None,
                    vec!["Point one.", "Point two."],
                )],
            ),
            slide(
                1,
                vec![body_with("B1", None, None, None, vec!["Point three."])],
            ),
        ];
        assert!(BulletPunctuationRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_punctuation_fires_on_missing_punct() {
        let slides = vec![
            slide(
                0,
                vec![body_with(
                    "B1",
                    None,
                    None,
                    None,
                    vec!["Point one.", "Point two."],
                )],
            ),
            slide(
                1,
                vec![body_with("B2", None, None, None, vec!["No punct here"])],
            ),
        ];
        let v = BulletPunctuationRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::BulletPunctuation {
                expected_punctuation: true
            }
        ));
    }

    #[test]
    fn bullet_punctuation_fires_on_unexpected_punct() {
        let slides = vec![
            slide(
                0,
                vec![body_with(
                    "B1",
                    None,
                    None,
                    None,
                    vec!["No punct", "Also no punct"],
                )],
            ),
            slide(
                1,
                vec![body_with("B2", None, None, None, vec!["Has punct."])],
            ),
        ];
        let v = BulletPunctuationRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::BulletPunctuation {
                expected_punctuation: false
            }
        ));
    }

    #[test]
    fn bullet_length_skips_autoshapes() {
        let long = "word ".repeat(21).trim().to_string();
        let slides = vec![slide(0, vec![autoshape_with("Box 1", vec![long.as_str()])])];
        assert!(BulletLengthRule { limit: 20 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_length_clean() {
        let slides = vec![slide(
            0,
            vec![body_with(
                "B1",
                None,
                None,
                None,
                vec!["Short bullet point"],
            )],
        )];
        assert!(BulletLengthRule { limit: 20 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn bullet_length_fires_over_limit() {
        let long = "word ".repeat(21).trim().to_string();
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec![long.as_str()])],
        )];
        let v = BulletLengthRule { limit: 20 }.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::BulletTooLong {
                word_count: 21,
                limit: 20
            }
        ));
        assert!(v[0].fix.is_none());
    }

    #[test]
    fn bullet_length_one_violation_per_element() {
        let long = "word ".repeat(21).trim().to_string();
        let slides = vec![slide(
            0,
            vec![body_with(
                "B1",
                None,
                None,
                None,
                vec![long.as_str(), long.as_str()],
            )],
        )];
        // Two long paragraphs in one element → still one violation (break after first).
        assert_eq!(BulletLengthRule { limit: 20 }.check(&slides, &T).len(), 1);
    }

    #[test]
    fn repeated_word_clean() {
        let slides = vec![slide(
            0,
            vec![body_with(
                "B1",
                None,
                None,
                None,
                vec!["The quick brown fox"],
            )],
        )];
        assert!(RepeatedWordRule.check(&slides, &T).is_empty());
    }

    #[test]
    fn repeated_word_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["the the quick fox"])],
        )];
        let v = RepeatedWordRule.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            &v[0].message,
            ViolationMessage::RepeatedWord { word } if word == "the"
        ));
    }

    #[test]
    fn repeated_word_case_insensitive() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["The the quick fox"])],
        )];
        assert_eq!(RepeatedWordRule.check(&slides, &T).len(), 1);
    }

    #[test]
    fn repeated_word_strips_punctuation() {
        let slides = vec![slide(
            0,
            vec![body_with(
                "B1",
                None,
                None,
                None,
                vec!["word, word more text"],
            )],
        )];
        assert_eq!(RepeatedWordRule.check(&slides, &T).len(), 1);
    }

    #[test]
    fn font_variety_clean() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Arial"))]),
        ];
        assert!(FontVarietyRule { limit: 2 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn font_variety_fires_over_limit() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Arial"))]),
            slide(2, vec![body("B1", None, Some("Times New Roman"))]),
        ];
        let v = FontVarietyRule { limit: 2 }.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(v[0].slide.is_none());
        assert!(matches!(
            v[0].message,
            ViolationMessage::FontVariety { count: 3, limit: 2 }
        ));
    }

    #[test]
    fn font_variety_skips_none_families() {
        let slides = vec![
            slide(0, vec![body("B1", None, None)]),
            slide(1, vec![body("B1", None, None)]),
            slide(2, vec![body("B1", None, None)]),
        ];
        assert!(FontVarietyRule { limit: 2 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn color_variety_clean() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("0000FF"), vec![])]),
        ];
        assert!(ColorVarietyRule { limit: 3 }.check(&slides, &T).is_empty());
    }

    #[test]
    fn color_variety_fires_over_limit() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("0000FF"), vec![])]),
            slide(3, vec![body_with("B1", None, None, Some("00FF00"), vec![])]),
        ];
        let v = ColorVarietyRule { limit: 3 }.check(&slides, &T);
        assert_eq!(v.len(), 1);
        assert!(v[0].slide.is_none());
        assert!(matches!(
            v[0].message,
            ViolationMessage::ColorVariety { count: 4, limit: 3 }
        ));
    }
}
