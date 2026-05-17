use std::fmt::Write;

use comfy_table::{
    ColumnConstraint, ContentArrangement, Table, Width, presets::UTF8_FULL_CONDENSED,
};
use intern_core::rules::{Severity, Violation};
use serde::Serialize;

#[derive(Clone, Copy)]
pub enum GroupBy {
    Slide,
    Rule,
}

#[derive(Clone, Copy)]
pub enum OutputFormat {
    Table,
    Text,
    Json,
}

/// Prints the per-file check results to stdout.
pub fn print_results(
    results: &[(String, Vec<Violation>)],
    group_by: GroupBy,
    format: OutputFormat,
) {
    println!("{}", render(results, group_by, format));
}

/// Renders the per-file check results into the final output string. Table and text
/// output get a header per file when more than one file was checked; JSON always
/// nests results under `files`.
fn render(results: &[(String, Vec<Violation>)], group_by: GroupBy, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => render_json(results),
        OutputFormat::Table => render_grouped(results, |v| render_table(v, group_by)),
        OutputFormat::Text => render_grouped(results, |v| render_text(v, group_by)),
    }
}

fn render_grouped(
    results: &[(String, Vec<Violation>)],
    render_one: impl Fn(&[Violation]) -> String,
) -> String {
    let multi = results.len() > 1;
    let blocks: Vec<String> = results
        .iter()
        .map(|(path, violations)| {
            let block = render_one(violations);
            if multi {
                format!("\n{path}\n{block}")
            } else {
                block
            }
        })
        .collect();

    let mut out = blocks.join("\n");
    if multi {
        let total: usize = results.iter().map(|(_, v)| v.len()).sum();
        let errors: usize = results.iter().map(|(_, v)| err_warn(v).0).sum();
        let _ = write!(
            out,
            "\n\n{total} violation(s) ({errors} error, {} warning) across {} file(s)",
            total - errors,
            results.len(),
        );
    }
    out
}

fn render_table(violations: &[Violation], group_by: GroupBy) -> String {
    if violations.is_empty() {
        return "No violations found.".to_string();
    }

    let mut sorted = violations.to_vec();
    match group_by {
        GroupBy::Slide => sorted.sort_by_key(|v| (v.slide.unwrap_or(0), v.rule_id)),
        GroupBy::Rule => sorted.sort_by_key(|v| (v.rule_id, v.slide.unwrap_or(0))),
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);
    match group_by {
        GroupBy::Slide => table.set_header(vec!["Slide", "Rule", "Element", "Message"]),
        GroupBy::Rule => table.set_header(vec!["Rule", "Slide", "Element", "Message"]),
    };

    // Cap the element and message columns so the table stays readable.
    // SAFETY: the header always has 4 columns, so indices 2 and 3 exist.
    table
        .column_mut(2)
        .unwrap()
        .set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(36)));
    table
        .column_mut(3)
        .unwrap()
        .set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(56)));

    for v in &sorted {
        let slide = v
            .slide
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let element = v.element.clone().unwrap_or_else(|| "-".to_string());
        let message = v.message.to_string();
        match group_by {
            GroupBy::Slide => table.add_row(vec![&slide, v.rule_id, &element, &message]),
            GroupBy::Rule => table.add_row(vec![v.rule_id, &slide, &element, &message]),
        };
    }

    format!("{table}\n{}", summary(violations))
}

fn render_text(violations: &[Violation], group_by: GroupBy) -> String {
    if violations.is_empty() {
        return "No violations found.".to_string();
    }
    match group_by {
        GroupBy::Slide => render_text_by_slide(violations),
        GroupBy::Rule => render_text_by_rule(violations),
    }
}

fn render_text_by_slide(violations: &[Violation]) -> String {
    let mut sorted = violations.to_vec();
    sorted.sort_by_key(|v| (v.slide.unwrap_or(0), v.rule_id));

    let mut out = String::new();
    let mut current: Option<usize> = None;
    for v in &sorted {
        if current != v.slide {
            current = v.slide;
            match v.slide {
                Some(n) => {
                    let _ = writeln!(out, "\nSlide {n}:");
                }
                None => {
                    let _ = writeln!(out, "\nPresentation-wide:");
                }
            }
        }
        match &v.element {
            Some(el) => {
                let _ = writeln!(out, "  [{}] {} - {}", v.rule_id, el, v.message);
            }
            None => {
                let _ = writeln!(out, "  [{}] {}", v.rule_id, v.message);
            }
        }
    }
    let _ = write!(out, "\n{}", summary(violations));
    out
}

fn render_text_by_rule(violations: &[Violation]) -> String {
    let mut sorted = violations.to_vec();
    sorted.sort_by_key(|v| (v.rule_id, v.slide.unwrap_or(0)));

    let mut out = String::new();
    let mut current: Option<&str> = None;
    for v in &sorted {
        if current != Some(v.rule_id) {
            current = Some(v.rule_id);
            let _ = writeln!(out, "\n[{}]:", v.rule_id);
        }
        let loc = v
            .slide
            .map(|n| format!("slide {n}"))
            .unwrap_or_else(|| "presentation".to_string());
        match &v.element {
            Some(el) => {
                let _ = writeln!(out, "  {loc} - {el} - {}", v.message);
            }
            None => {
                let _ = writeln!(out, "  {loc} - {}", v.message);
            }
        }
    }
    let _ = write!(out, "\n{}", summary(violations));
    out
}

/// Splits violations into (error count, warning count).
fn err_warn(violations: &[Violation]) -> (usize, usize) {
    let errors = violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .count();
    (errors, violations.len() - errors)
}

fn summary(violations: &[Violation]) -> String {
    let (errors, warnings) = err_warn(violations);
    format!(
        "{} violation(s) ({errors} error, {warnings} warning)",
        violations.len()
    )
}

#[derive(Serialize)]
struct JsonViolation<'a> {
    rule_id: &'a str,
    slide: Option<usize>,
    element: Option<&'a str>,
    message: String,
    severity: &'a str,
}

fn json_violation(v: &Violation) -> JsonViolation<'_> {
    JsonViolation {
        rule_id: v.rule_id,
        slide: v.slide,
        element: v.element.as_deref(),
        message: v.message.to_string(),
        severity: match v.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        },
    }
}

fn render_json(results: &[(String, Vec<Violation>)]) -> String {
    let files: Vec<_> = results
        .iter()
        .map(|(path, violations)| {
            let items: Vec<JsonViolation> = violations.iter().map(json_violation).collect();
            serde_json::json!({ "path": path, "violations": items })
        })
        .collect();
    let output = serde_json::json!({ "files": files });
    // SAFETY: the value holds only strings, numbers, and arrays - serialization is infallible.
    serde_json::to_string_pretty(&output).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern_core::rules::ViolationMessage;

    fn violation(rule_id: &'static str, slide: Option<usize>, severity: Severity) -> Violation {
        Violation {
            rule_id,
            slide,
            element: None,
            message: ViolationMessage::TitleMissing,
            severity,
            fix: None,
        }
    }

    fn results(items: Vec<(&str, Vec<Violation>)>) -> Vec<(String, Vec<Violation>)> {
        items
            .into_iter()
            .map(|(path, vs)| (path.to_string(), vs))
            .collect()
    }

    #[test]
    fn summary_splits_errors_and_warnings() {
        let vs = vec![
            violation("A", Some(1), Severity::Error),
            violation("B", Some(2), Severity::Warning),
        ];
        assert_eq!(summary(&vs), "2 violation(s) (1 error, 1 warning)");
    }

    #[test]
    fn table_reports_no_violations_when_clean() {
        let out = render(
            &results(vec![("deck.pptx", vec![])]),
            GroupBy::Slide,
            OutputFormat::Table,
        );
        assert_eq!(out, "No violations found.");
    }

    #[test]
    fn table_lists_the_rule_and_a_summary() {
        let r = results(vec![(
            "deck.pptx",
            vec![violation("TITLE_PRESENT", Some(3), Severity::Error)],
        )]);
        let out = render(&r, GroupBy::Slide, OutputFormat::Table);
        assert!(out.contains("TITLE_PRESENT"), "{out}");
        assert!(out.contains("1 violation(s) (1 error, 0 warning)"), "{out}");
    }

    #[test]
    fn text_groups_by_slide() {
        let r = results(vec![(
            "deck.pptx",
            vec![violation("TITLE_PRESENT", Some(3), Severity::Error)],
        )]);
        let out = render(&r, GroupBy::Slide, OutputFormat::Text);
        assert!(out.contains("Slide 3:"), "{out}");
        assert!(out.contains("[TITLE_PRESENT]"), "{out}");
    }

    #[test]
    fn text_groups_by_rule() {
        let r = results(vec![(
            "deck.pptx",
            vec![violation("TITLE_PRESENT", Some(3), Severity::Error)],
        )]);
        let out = render(&r, GroupBy::Rule, OutputFormat::Text);
        assert!(out.contains("[TITLE_PRESENT]:"), "{out}");
        assert!(out.contains("slide 3"), "{out}");
    }

    #[test]
    fn json_nests_violations_under_files() {
        let r = results(vec![(
            "deck.pptx",
            vec![violation("TITLE_PRESENT", Some(3), Severity::Error)],
        )]);
        let out = render(&r, GroupBy::Slide, OutputFormat::Json);
        assert!(out.contains("\"files\""), "{out}");
        assert!(out.contains("\"path\": \"deck.pptx\""), "{out}");
        assert!(out.contains("\"rule_id\": \"TITLE_PRESENT\""), "{out}");
        assert!(out.contains("\"severity\": \"error\""), "{out}");
    }

    #[test]
    fn multi_file_output_has_headers_and_a_grand_total() {
        let r = results(vec![
            (
                "a.pptx",
                vec![violation("TITLE_PRESENT", Some(1), Severity::Error)],
            ),
            (
                "b.pptx",
                vec![violation("ALL_CAPS", Some(2), Severity::Warning)],
            ),
        ]);
        let out = render(&r, GroupBy::Slide, OutputFormat::Table);
        assert!(out.contains("a.pptx"), "{out}");
        assert!(out.contains("b.pptx"), "{out}");
        assert!(
            out.contains("2 violation(s) (1 error, 1 warning) across 2 file(s)"),
            "{out}"
        );
    }
}
