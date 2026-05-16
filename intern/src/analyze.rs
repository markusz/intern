use std::borrow::Cow;
use std::collections::HashMap;

use intern_core::model::{self, SlideData};
use intern_core::reader::{self, SlideExclusion};
use intern_core::rules;
use miette::IntoDiagnostic;

use crate::config::Config;
use crate::ruleset::Selection;

/// Reads a presentation, applies its `intern: disable` exclusions, runs the active
/// rules (each with its own threshold), and tags every violation with the
/// configured severity. Shared by `check` and `fix`.
pub fn check_file(
    path: &str,
    slide: Option<usize>,
    global_px: u32,
    cfg: &Config,
    selection: &Selection,
) -> miette::Result<Vec<rules::Violation>> {
    let mut slides = reader::read_presentation(path).into_diagnostic()?;
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

    let mut violations = Vec::new();
    for rule in &selection.rules {
        let threshold = cfg.rule_threshold_px(rule.id(), global_px) as i64 * model::EMU_PER_PX;
        let severity = cfg.rule_severity(rule.id());
        let view = slides_for_rule(&slides, &exclusions, rule.id());
        for mut violation in rule.check(&view, threshold) {
            violation.severity = severity;
            violations.push(violation);
        }
    }
    Ok(violations)
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

    fn slide(index: usize) -> SlideData {
        SlideData {
            index,
            elements: vec![],
        }
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
