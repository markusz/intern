use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;

use intern_core::model::{self, Presentation, SlideData, SlideElement};
use intern_core::reader::{self, SlideExclusion};
use intern_core::rules;
use miette::IntoDiagnostic;

use crate::config::Config;
use crate::ruleset::Selection;

/// Maximum length of the offending-element text snippet shown in reports.
const EXCERPT_CHARS: usize = 30;

/// A violation plus display context the rules themselves do not carry. Derefs to
/// the inner `Violation` so reporting code can read its fields directly.
#[derive(Clone)]
pub struct Finding {
    pub violation: rules::Violation,
    /// First `EXCERPT_CHARS` characters of the offending element's text, when it
    /// has any. `None` for geometry-only or text-free elements.
    pub excerpt: Option<String>,
    /// Element kind as a display string: `"TextBox"`, `"Body"`, etc.
    pub element_kind: Option<String>,
    /// Top-left corner in screen pixels: `"(42px, 107px)"`.
    pub element_position: Option<String>,
    /// The `<p:cNvPr>` id - stable within a slide, used for element-level suppression.
    pub element_id: Option<u32>,
    /// Stable 8-char hex identifier for this finding; use with `intern ignore`.
    pub finding_id: String,
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
        .filter(|s| {
            exclusions
                .get(&s.index)
                .is_some_and(|ex| ex.suppresses_slide())
        })
        .count();
    if whole_slide > 0 {
        eprintln!("{path}: skipped {whole_slide} slide(s) marked 'intern: disable'");
    }
    slides.retain(|s| {
        !exclusions
            .get(&s.index)
            .is_some_and(|ex| ex.suppresses_slide())
    });
    if let Some(n) = slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let element_map: HashMap<(usize, u32), &SlideElement> = slides
        .iter()
        .flat_map(|s| s.elements.iter().map(move |e| ((s.index + 1, e.id), e)))
        .collect();

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
            // Skip violations targeting an element suppressed via `intern: disable(id)`.
            if let (Some(slide_no), Some(eid)) = (violation.slide, violation.element)
                && exclusions
                    .get(&(slide_no - 1))
                    .is_some_and(|ex| ex.suppresses_rule_for_element(eid, rule.id()))
            {
                continue;
            }
            let resolved = violation
                .slide
                .zip(violation.element)
                .and_then(|(slide_no, id)| element_map.get(&(slide_no, id)).copied());
            let excerpt = excerpt_text(resolved);
            let element_kind = resolved.map(|e| e.kind.to_string());
            let element_position = resolved.map(element_position_str);
            let element_id = resolved.map(|e| e.id);
            let finding_id = compute_finding_id(rule.id(), violation.slide, violation.element);
            findings.push(Finding {
                violation,
                excerpt,
                element_kind,
                element_position,
                element_id,
                finding_id,
            });
        }
    }
    Ok(findings)
}

/// First `EXCERPT_CHARS` characters of the element's text (whitespace collapsed).
/// Returns `None` when the element is absent or carries no text.
fn excerpt_text(element: Option<&SlideElement>) -> Option<String> {
    let element = element?;
    let text = element
        .paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
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

/// Top-left corner as `"(42px, 107px)"`.
fn element_position_str(e: &SlideElement) -> String {
    let x_px = e.rect.x / model::EMU_PER_PX;
    let y_px = e.rect.y / model::EMU_PER_PX;
    format!("({}px, {}px)", x_px, y_px)
}

/// The slides a rule should see: all of them, unless some slide's `intern: disable`
/// directive names this rule. Borrows when nothing is excluded so the common case
/// allocates nothing.
fn slides_for_rule<'a>(
    slides: &'a [SlideData],
    exclusions: &HashMap<usize, SlideExclusion>,
    rule_id: &str,
) -> Cow<'a, [SlideData]> {
    let excluded = |s: &SlideData| {
        exclusions
            .get(&s.index)
            .is_some_and(|ex| ex.suppresses_rule_for_slide(rule_id))
    };
    if slides.iter().any(&excluded) {
        Cow::Owned(slides.iter().filter(|s| !excluded(s)).cloned().collect())
    } else {
        Cow::Borrowed(slides)
    }
}

/// Stable 8-character hex finding identifier. FNV-1a over (rule_id, slide, element_id)
/// so it is deterministic across runs and platforms.
fn compute_finding_id(rule_id: &str, slide: Option<usize>, element_id: Option<u32>) -> String {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME: u64 = 1099511628211;
    let mut h = OFFSET;
    for b in rule_id
        .as_bytes()
        .iter()
        .copied()
        .chain(std::iter::once(0u8))
    {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    for b in (slide.unwrap_or(0) as u64).to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    for b in (element_id.unwrap_or(0) as u64).to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    format!("{:08x}", h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern_core::model::{ElementKind, Paragraph, ParagraphKind, Rect, SlideElement};

    fn slide(index: usize) -> SlideData {
        SlideData {
            index,
            elements: vec![],
        }
    }

    fn text_element(id: u32, name: &str, paragraphs: Vec<&str>) -> SlideElement {
        SlideElement {
            id,
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
            paragraphs: paragraphs
                .into_iter()
                .map(|s| Paragraph {
                    text: s.to_string(),
                    kind: ParagraphKind::Plain,
                })
                .collect(),
        }
    }

    #[test]
    fn excerpt_text_collapses_element_text() {
        let el = text_element(1, "Textfeld 100", vec!["HELLO  THERE", "WORLD"]);
        assert_eq!(
            excerpt_text(Some(&el)).as_deref(),
            Some("HELLO THERE WORLD")
        );
    }

    #[test]
    fn excerpt_text_truncates_long_text() {
        let long = "x".repeat(120);
        let el = text_element(1, "Body", vec![long.as_str()]);
        let excerpt = excerpt_text(Some(&el)).unwrap();
        assert!(excerpt.ends_with("..."));
        assert_eq!(excerpt.chars().count(), EXCERPT_CHARS + 3);
    }

    #[test]
    fn excerpt_text_none_when_element_absent_or_empty() {
        let empty = text_element(1, "Empty", vec![]);
        assert!(excerpt_text(Some(&empty)).is_none());
        assert!(excerpt_text(None).is_none());
    }

    #[test]
    fn slides_for_rule_excludes_only_the_named_rule() {
        let slides = vec![slide(0), slide(1), slide(2)];
        let mut exclusions = HashMap::new();
        exclusions.insert(
            1,
            SlideExclusion {
                slide: Some(vec!["TITLE_Y".to_string()]),
                ..SlideExclusion::default()
            },
        );

        let for_title_y = slides_for_rule(&slides, &exclusions, "TITLE_Y");
        assert_eq!(for_title_y.len(), 2);
        assert!(for_title_y.iter().all(|s| s.index != 1));

        let for_other = slides_for_rule(&slides, &exclusions, "GRID_ROW_TOP");
        assert_eq!(for_other.len(), 3);
    }

    #[test]
    fn compute_finding_id_is_stable() {
        let a = compute_finding_id("TITLE_Y", Some(3), Some(7));
        let b = compute_finding_id("TITLE_Y", Some(3), Some(7));
        assert_eq!(a, b);
        assert_eq!(a.len(), 8);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn compute_finding_id_differs_by_input() {
        let base = compute_finding_id("TITLE_Y", Some(3), Some(7));
        assert_ne!(base, compute_finding_id("TITLE_X_WIDTH", Some(3), Some(7)));
        assert_ne!(base, compute_finding_id("TITLE_Y", Some(4), Some(7)));
        assert_ne!(base, compute_finding_id("TITLE_Y", Some(3), Some(8)));
        assert_ne!(base, compute_finding_id("TITLE_Y", None, None));
    }
}
