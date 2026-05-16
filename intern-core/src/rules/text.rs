use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::model::{ElementKind, SlideData, SlideElement};
use crate::rules::{Fix, Rule, Severity, Violation, ViolationMessage};

pub struct BodyFontSizeRule;
pub struct BodyFontFamilyRule;
pub struct BodyTextColorRule;
pub struct DoubleSpaceRule;
pub struct TrailingSpaceRule;
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

fn body_elements(slides: &[SlideData]) -> Vec<(usize, &SlideElement)> {
    slides
        .iter()
        .flat_map(|s| {
            s.elements
                .iter()
                .filter(|e| matches!(e.kind, ElementKind::Body | ElementKind::TextBox))
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

impl Rule for BodyFontSizeRule {
    fn id(&self) -> &'static str {
        "BODY_FONT_SIZE"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let es = body_elements(slides);
        let sized: Vec<(usize, String, u32)> = es
            .iter()
            .filter_map(|(idx, e)| e.font_size.map(|sz| (*idx, e.name.clone(), sz)))
            .collect();

        if sized.len() < 2 {
            return vec![];
        }

        let sizes: Vec<u32> = sized.iter().map(|(_, _, sz)| *sz).collect();
        // SAFETY: sized.len() >= 2 ensures sizes is non-empty.
        let expected = mode(&sizes).unwrap();

        sized
            .iter()
            .filter(|(_, _, sz)| *sz != expected)
            .map(|(idx, name, sz)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(name.clone()),
                message: ViolationMessage::BodyFontSize {
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

impl Rule for BodyFontFamilyRule {
    fn id(&self) -> &'static str {
        "BODY_FONT_FAMILY"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let es = body_elements(slides);
        let familied: Vec<(usize, String, String)> = es
            .iter()
            .filter_map(|(idx, e)| {
                e.font_family
                    .as_ref()
                    .map(|f| (*idx, e.name.clone(), f.clone()))
            })
            .collect();

        if familied.len() < 2 {
            return vec![];
        }

        let families: Vec<String> = familied.iter().map(|(_, _, f)| f.clone()).collect();
        // SAFETY: familied.len() >= 2 ensures families is non-empty.
        let expected = mode(&families).unwrap();

        familied
            .iter()
            .filter(|(_, _, f)| f != &expected)
            .map(|(idx, name, actual)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(name.clone()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let es = body_elements(slides);
        let colored: Vec<(usize, String, String)> = es
            .iter()
            .filter_map(|(idx, e)| {
                e.text_color
                    .as_ref()
                    .map(|c| (*idx, e.name.clone(), c.clone()))
            })
            .collect();

        if colored.len() < 2 {
            return vec![];
        }

        let colors: Vec<String> = colored.iter().map(|(_, _, c)| c.clone()).collect();
        // SAFETY: colored.len() >= 2 ensures colors is non-empty.
        let expected = mode(&colors).unwrap();

        colored
            .iter()
            .filter(|(_, _, c)| c != &expected)
            .map(|(idx, name, actual)| Violation {
                rule_id: self.id(),
                slide: Some(idx + 1),
                element: Some(name.clone()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if e.paragraphs.iter().any(|p| p.contains("  ")) {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.name.clone()),
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

impl Rule for TrailingSpaceRule {
    fn id(&self) -> &'static str {
        "TRAILING_SPACE"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if e.paragraphs.iter().any(|p| p != p.trim()) {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.name.clone()),
                        message: ViolationMessage::TrailingSpace,
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
) -> impl Iterator<Item = (bool, usize, &str)> {
    e.paragraphs
        .iter()
        .filter_map(move |p| first_alpha(p).map(|c| (c.is_uppercase(), slide_idx, e.name.as_str())))
}

impl Rule for BulletCapitalizationRule {
    fn id(&self) -> &'static str {
        "BULLET_CAPITALIZATION"
    }

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let es = body_elements(slides);

        let all: Vec<(bool, usize, &str)> = es
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
        for (is_upper, idx, name) in &all {
            if *is_upper != majority_upper && seen.insert((*idx, *name)) {
                violations.push(Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(name.to_string()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if !matches!(e.kind, ElementKind::Body | ElementKind::TextBox) {
                    continue;
                }
                let has_all_caps = e.paragraphs.iter().any(|p| {
                    let mut alpha = p.chars().filter(|c| c.is_alphabetic()).peekable();
                    alpha.peek().is_some() && alpha.all(|c| c.is_uppercase())
                });
                if has_all_caps {
                    violations.push(Violation {
                        rule_id: self.id(),
                        slide: Some(slide.index + 1),
                        element: Some(e.name.clone()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let es = body_elements(slides);
        let bullets: Vec<(bool, usize, &str)> = es
            .iter()
            .flat_map(|(idx, e)| {
                e.paragraphs
                    .iter()
                    .map(move |p| (ends_with_punct(p), *idx, e.name.as_str()))
            })
            .collect();

        if bullets.len() < 2 {
            return vec![];
        }

        let punct_count = bullets.iter().filter(|(p, _, _)| *p).count();
        let majority_punct = is_majority(punct_count, bullets.len());

        let mut seen = HashSet::new();
        let mut violations = Vec::new();
        for (has_punct, idx, name) in &bullets {
            if *has_punct != majority_punct && seen.insert((*idx, *name)) {
                violations.push(Violation {
                    rule_id: self.id(),
                    slide: Some(idx + 1),
                    element: Some(name.to_string()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                if !matches!(e.kind, ElementKind::Body | ElementKind::TextBox) {
                    continue;
                }
                for p in &e.paragraphs {
                    let word_count = p.split_whitespace().count();
                    if word_count > self.limit {
                        violations.push(Violation {
                            rule_id: self.id(),
                            slide: Some(slide.index + 1),
                            element: Some(e.name.clone()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
        let mut violations = Vec::new();
        for slide in slides {
            for e in &slide.elements {
                'para: for p in &e.paragraphs {
                    let words: Vec<String> = p.split_whitespace().map(word_key).collect();
                    for w in words.windows(2) {
                        if !w[0].is_empty() && w[0] == w[1] {
                            violations.push(Violation {
                                rule_id: self.id(),
                                slide: Some(slide.index + 1),
                                element: Some(e.name.clone()),
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
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

    fn check(&self, slides: &[SlideData], _threshold: i64) -> Vec<Violation> {
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
    use crate::model::{ElementKind, Rect, SlideData, SlideElement};

    const T: i64 = 19_050;

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
            paragraphs: paragraphs.into_iter().map(str::to_string).collect(),
        }
    }

    fn slide(idx: usize, elements: Vec<SlideElement>) -> SlideData {
        SlideData {
            index: idx,
            elements,
        }
    }

    #[test]
    fn body_font_size_clean() {
        let slides = vec![
            slide(0, vec![body("B1", Some(2_400), None)]),
            slide(1, vec![body("B1", Some(2_400), None)]),
        ];
        assert!(BodyFontSizeRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_font_size_fires_on_outlier() {
        let slides = vec![
            slide(0, vec![body("B1", Some(2_400), None)]),
            slide(1, vec![body("B1", Some(2_400), None)]),
            slide(2, vec![body("B1", Some(3_200), None)]), // outlier
        ];
        let v = BodyFontSizeRule.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].slide, Some(3));
        assert!(matches!(
            v[0].message,
            ViolationMessage::BodyFontSize {
                actual: 3200,
                expected: 2400
            }
        ));
        assert!(v[0].fix.is_some());
    }

    #[test]
    fn body_font_size_skips_elements_without_size() {
        let slides = vec![
            slide(0, vec![body("B1", None, None)]),
            slide(1, vec![body("B1", None, None)]),
        ];
        assert!(BodyFontSizeRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_font_size_ignores_title_elements() {
        let mut e = body("T", Some(4_400), None);
        e.kind = ElementKind::Title;
        let slides = vec![
            slide(0, vec![e.clone(), body("B1", Some(2_400), None)]),
            slide(1, vec![e.clone(), body("B1", Some(2_400), None)]),
        ];
        assert!(BodyFontSizeRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_font_family_clean() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Calibri"))]),
        ];
        assert!(BodyFontFamilyRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_font_family_fires_on_outlier() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Calibri"))]),
            slide(2, vec![body("B1", None, Some("Arial"))]), // outlier
        ];
        let v = BodyFontFamilyRule.check(&slides, T);
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
        assert!(BodyFontFamilyRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_text_color_clean() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("000000"), vec![])]),
        ];
        assert!(BodyTextColorRule.check(&slides, T).is_empty());
    }

    #[test]
    fn body_text_color_fires_on_outlier() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
        ];
        let v = BodyTextColorRule.check(&slides, T);
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
        assert!(DoubleSpaceRule.check(&slides, T).is_empty());
    }

    #[test]
    fn double_space_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Two  spaces"])],
        )];
        let v = DoubleSpaceRule.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].message, ViolationMessage::DoubleSpace));
        assert!(matches!(v[0].fix, Some(Fix::NormalizeWhitespace { .. })));
    }

    #[test]
    fn trailing_space_clean() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Clean text"])],
        )];
        assert!(TrailingSpaceRule.check(&slides, T).is_empty());
    }

    #[test]
    fn trailing_space_fires_on_trailing() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["trailing space "])],
        )];
        let v = TrailingSpaceRule.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].message, ViolationMessage::TrailingSpace));
    }

    #[test]
    fn trailing_space_fires_on_leading() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec![" leading space"])],
        )];
        assert_eq!(TrailingSpaceRule.check(&slides, T).len(), 1);
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
        assert!(BulletCapitalizationRule.check(&slides, T).is_empty());
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
        let v = BulletCapitalizationRule.check(&slides, T);
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
        assert!(BulletCapitalizationRule.check(&slides, T).is_empty());
    }

    #[test]
    fn all_caps_clean() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["Normal text here"])],
        )];
        assert!(AllCapsRule.check(&slides, T).is_empty());
    }

    #[test]
    fn all_caps_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["SHOUTING TEXT"])],
        )];
        let v = AllCapsRule.check(&slides, T);
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
        assert!(AllCapsRule.check(&slides, T).is_empty());
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
        assert!(BulletPunctuationRule.check(&slides, T).is_empty());
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
        let v = BulletPunctuationRule.check(&slides, T);
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
        let v = BulletPunctuationRule.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0].message,
            ViolationMessage::BulletPunctuation {
                expected_punctuation: false
            }
        ));
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
        assert!(BulletLengthRule { limit: 20 }.check(&slides, T).is_empty());
    }

    #[test]
    fn bullet_length_fires_over_limit() {
        let long = "word ".repeat(21).trim().to_string();
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec![long.as_str()])],
        )];
        let v = BulletLengthRule { limit: 20 }.check(&slides, T);
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
        assert_eq!(BulletLengthRule { limit: 20 }.check(&slides, T).len(), 1);
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
        assert!(RepeatedWordRule.check(&slides, T).is_empty());
    }

    #[test]
    fn repeated_word_fires() {
        let slides = vec![slide(
            0,
            vec![body_with("B1", None, None, None, vec!["the the quick fox"])],
        )];
        let v = RepeatedWordRule.check(&slides, T);
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
        assert_eq!(RepeatedWordRule.check(&slides, T).len(), 1);
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
        assert_eq!(RepeatedWordRule.check(&slides, T).len(), 1);
    }

    #[test]
    fn font_variety_clean() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Arial"))]),
        ];
        assert!(FontVarietyRule { limit: 2 }.check(&slides, T).is_empty());
    }

    #[test]
    fn font_variety_fires_over_limit() {
        let slides = vec![
            slide(0, vec![body("B1", None, Some("Calibri"))]),
            slide(1, vec![body("B1", None, Some("Arial"))]),
            slide(2, vec![body("B1", None, Some("Times New Roman"))]),
        ];
        let v = FontVarietyRule { limit: 2 }.check(&slides, T);
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
        assert!(FontVarietyRule { limit: 2 }.check(&slides, T).is_empty());
    }

    #[test]
    fn color_variety_clean() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("0000FF"), vec![])]),
        ];
        assert!(ColorVarietyRule { limit: 3 }.check(&slides, T).is_empty());
    }

    #[test]
    fn color_variety_fires_over_limit() {
        let slides = vec![
            slide(0, vec![body_with("B1", None, None, Some("000000"), vec![])]),
            slide(1, vec![body_with("B1", None, None, Some("FF0000"), vec![])]),
            slide(2, vec![body_with("B1", None, None, Some("0000FF"), vec![])]),
            slide(3, vec![body_with("B1", None, None, Some("00FF00"), vec![])]),
        ];
        let v = ColorVarietyRule { limit: 3 }.check(&slides, T);
        assert_eq!(v.len(), 1);
        assert!(v[0].slide.is_none());
        assert!(matches!(
            v[0].message,
            ViolationMessage::ColorVariety { count: 4, limit: 3 }
        ));
    }
}
