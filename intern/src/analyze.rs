use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;

use intern_core::model::{self, Presentation, SlideData};
use intern_core::reader::{self, SlideExclusion};
use intern_core::rules;
use miette::IntoDiagnostic;

use crate::config::Config;
use crate::ruleset::Selection;

/// Maximum length of the offending-element text snippet shown in reports.
const EXCERPT_CHARS: usize = 50;

/// A violation plus display context the rules themselves do not carry - currently
/// a short snippet of the offending element's text. Derefs to the inner
/// `Violation` so reporting code can read its fields directly.
#[derive(Clone)]
pub struct Finding {
    pub violation: rules::Violation,
    /// First `EXCERPT_CHARS` characters of the offending element's text, when it
    /// has any. `None` for geometry-only or text-free elements.
    pub excerpt: Option<String>,
}

impl Deref for Finding {
    type Target = rules::Violation;

    fn deref(&self) -> &rules::Violation {
        &self.violation
    }
}

/// Reads a presentation, applies its `intern: disable` exclusions, runs the active
/// rules (each with its own threshold), and tags every violation with the
/// configured severity. Shared by `check` and `fix`.
pub fn check_file(
    path: &str,
    slide: Option<usize>,
    global_px: u32,
    cfg: &Config,
    selection: &Selection,
) -> miette::Result<Vec<Finding>> {
    let Presentation {
        mut slides,
        slide_width,
        slide_height,
    } = reader::read_presentation(path).into_diagnostic()?;
    let exclusions = reader::slide_exclusions(path).into_diagnostic()?;

    let whole_slide = slides
        .iter()
        .filter(|s| matches!(exclusions.get(&s.index), Some(SlideExclusion::All)))
        .count();
    if whole_slide > 0 {
        eprintln!("{path}: skipped {whole_slide} slide(s) marked 'intern: disable'");
    }
    slides.retain(|s| !matches!(exclusions.get(&s.index), Some(SlideExclusion::All)));
    if let Some(n) = slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let mut findings = Vec::new();
    for rule in &selection.rules {
        let threshold = cfg.rule_threshold_px(rule.id(), global_px) as i64 * model::EMU_PER_PX;
        let ctx = rules::RuleContext {
            threshold,
            slide_width,
            slide_height,
        };
        let severity = cfg.rule_severity(rule.id());
        let view = slides_for_rule(&slides, &exclusions, rule.id());
        for mut violation in rule.check(&view, &ctx) {
            violation.severity = severity;
            let excerpt = excerpt_for(&slides, &violation);
            findings.push(Finding { violation, excerpt });
        }
    }
    Ok(findings)
}

/// A short snippet of a violation's offending element text. Looks the element up
/// by name on its slide; whitespace is collapsed so the snippet stays on one line.
/// Yields `None` when the element is composite, absent, text-free, or - since the
/// lookup is by name - when several elements on the slide share that name.
fn excerpt_for(slides: &[SlideData], v: &rules::Violation) -> Option<String> {
    let slide_no = v.slide?;
    let name = v.element.as_deref()?;
    let slide = slides.iter().find(|s| s.index + 1 == slide_no)?;
    let mut matches = slide.elements.iter().filter(|e| e.name == name);
    let element = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let text = element.paragraphs.join(" ");
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() > EXCERPT_CHARS {
        let snippet: String = collapsed.chars().take(EXCERPT_CHARS).collect();
        Some(format!("{snippet}..."))
    } else {
        Some(collapsed)
    }
}

/// The slides a rule should see: all of them, unless some slide's `intern: disable`
/// directive names this rule. Borrows when nothing is excluded so the common case
/// allocates nothing.
fn slides_for_rule<'a>(
    slides: &'a [SlideData],
    exclusions: &HashMap<usize, SlideExclusion>,
    rule_id: &str,
) -> Cow<'a, [SlideData]> {
    let excluded = |s: &SlideData| match exclusions.get(&s.index) {
        Some(SlideExclusion::Rules(ids)) => ids.iter().any(|id| id == rule_id),
        _ => false,
    };
    if slides.iter().any(&excluded) {
        Cow::Owned(slides.iter().filter(|s| !excluded(s)).cloned().collect())
    } else {
        Cow::Borrowed(slides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern_core::model::{ElementKind, Rect, SlideElement};

    fn slide(index: usize) -> SlideData {
        SlideData {
            index,
            elements: vec![],
        }
    }

    fn text_element(name: &str, paragraphs: Vec<&str>) -> SlideElement {
        SlideElement {
            name: name.into(),
            kind: ElementKind::TextBox,
            rect: Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: paragraphs.into_iter().map(str::to_string).collect(),
        }
    }

    fn violation(slide: Option<usize>, element: Option<&str>) -> rules::Violation {
        rules::Violation {
            rule_id: "ALL_CAPS",
            slide,
            element: element.map(str::to_string),
            message: rules::ViolationMessage::AllCaps,
            severity: rules::Severity::Warning,
            fix: None,
        }
    }

    #[test]
    fn excerpt_for_collapses_element_text() {
        let slides = vec![SlideData {
            index: 0,
            elements: vec![text_element("Textfeld 100", vec!["HELLO  THERE", "WORLD"])],
        }];
        let v = violation(Some(1), Some("Textfeld 100"));
        assert_eq!(
            excerpt_for(&slides, &v).as_deref(),
            Some("HELLO THERE WORLD")
        );
    }

    #[test]
    fn excerpt_for_truncates_long_text() {
        let long = "x".repeat(120);
        let slides = vec![SlideData {
            index: 0,
            elements: vec![text_element("Body", vec![long.as_str()])],
        }];
        let v = violation(Some(1), Some("Body"));
        let excerpt = excerpt_for(&slides, &v).unwrap();
        assert!(excerpt.ends_with("..."));
        assert_eq!(excerpt.chars().count(), EXCERPT_CHARS + 3);
    }

    #[test]
    fn excerpt_for_none_when_element_absent_or_empty() {
        let slides = vec![SlideData {
            index: 0,
            elements: vec![text_element("Empty", vec![])],
        }];
        assert!(excerpt_for(&slides, &violation(Some(1), Some("Empty"))).is_none());
        assert!(excerpt_for(&slides, &violation(Some(1), Some("Missing"))).is_none());
        assert!(excerpt_for(&slides, &violation(None, None)).is_none());
    }

    #[test]
    fn excerpt_for_none_when_name_is_ambiguous() {
        // Several elements share the name, so the by-name lookup is ambiguous.
        let slides = vec![SlideData {
            index: 0,
            elements: vec![
                text_element("Dup", vec!["first"]),
                text_element("Dup", vec!["second"]),
            ],
        }];
        assert!(excerpt_for(&slides, &violation(Some(1), Some("Dup"))).is_none());
    }

    #[test]
    fn slides_for_rule_excludes_only_the_named_rule() {
        let slides = vec![slide(0), slide(1), slide(2)];
        let mut exclusions = HashMap::new();
        exclusions.insert(1, SlideExclusion::Rules(vec!["TITLE_Y".to_string()]));

        // The rule named on slide 1 does not see slide 1...
        let for_title_y = slides_for_rule(&slides, &exclusions, "TITLE_Y");
        assert_eq!(for_title_y.len(), 2);
        assert!(for_title_y.iter().all(|s| s.index != 1));

        // ...but every other rule still sees all three slides.
        let for_other = slides_for_rule(&slides, &exclusions, "GRID_ROW_TOP");
        assert_eq!(for_other.len(), 3);
    }
}
