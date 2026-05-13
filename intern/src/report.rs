use comfy_table::{
    ColumnConstraint, ContentArrangement, Table, Width, presets::UTF8_FULL_CONDENSED,
};

use intern_core::rules::{Severity, Violation};
use serde::Serialize;

pub enum GroupBy {
    Slide,
    Rule,
}

pub enum OutputFormat {
    Table,
    Text,
    Json,
}

pub fn print_violations(violations: &[Violation], group_by: GroupBy, format: OutputFormat) {
    match format {
        OutputFormat::Table => print_table(violations, group_by),
        OutputFormat::Text => print_text(violations, group_by),
        OutputFormat::Json => print_json(violations),
    }
}

fn print_table(violations: &[Violation], group_by: GroupBy) {
    if violations.is_empty() {
        println!("No violations found.");
        return;
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

    // Cap element and message columns so the table stays readable in a standard terminal.
    // SAFETY: header always has 4 columns, so indices 2 and 3 always exist.
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
            .unwrap_or_else(|| "—".to_string());
        let element = v.element.clone().unwrap_or_else(|| "—".to_string());
        let message = v.message.to_string();
        match group_by {
            GroupBy::Slide => table.add_row(vec![&slide, v.rule_id, &element, &message]),
            GroupBy::Rule => table.add_row(vec![v.rule_id, &slide, &element, &message]),
        };
    }

    println!("{table}");
    println!("{} violation(s)", violations.len());
}

fn print_text(violations: &[Violation], group_by: GroupBy) {
    if violations.is_empty() {
        println!("No violations found.");
        return;
    }
    match group_by {
        GroupBy::Slide => print_text_by_slide(violations),
        GroupBy::Rule => print_text_by_rule(violations),
    }
}

fn print_text_by_slide(violations: &[Violation]) {
    let mut sorted = violations.to_vec();
    sorted.sort_by_key(|v| (v.slide.unwrap_or(0), v.rule_id));

    let mut current: Option<usize> = None;
    for v in &sorted {
        if current != v.slide {
            current = v.slide;
            match v.slide {
                Some(n) => println!("\nSlide {n}:"),
                None => println!("\nPresentation-wide:"),
            }
        }
        if let Some(ref el) = v.element {
            println!("  [{}] {} — {}", v.rule_id, el, v.message);
        } else {
            println!("  [{}] {}", v.rule_id, v.message);
        }
    }

    println!("\n{} violation(s)", violations.len());
}

fn print_text_by_rule(violations: &[Violation]) {
    let mut sorted = violations.to_vec();
    sorted.sort_by_key(|v| (v.rule_id, v.slide.unwrap_or(0)));

    let mut current: Option<&str> = None;
    for v in &sorted {
        if current != Some(v.rule_id) {
            current = Some(v.rule_id);
            println!("\n[{}]:", v.rule_id);
        }
        let loc = v
            .slide
            .map(|n| format!("slide {n}"))
            .unwrap_or_else(|| "presentation".to_string());
        if let Some(ref el) = v.element {
            println!("  {loc} — {el} — {}", v.message);
        } else {
            println!("  {loc} — {}", v.message);
        }
    }

    println!("\n{} violation(s)", violations.len());
}

#[derive(Serialize)]
struct JsonViolation<'a> {
    rule_id: &'a str,
    slide: Option<usize>,
    element: Option<&'a str>,
    message: String,
    severity: &'a str,
}

fn print_json(violations: &[Violation]) {
    let items: Vec<JsonViolation> = violations
        .iter()
        .map(|v| JsonViolation {
            rule_id: v.rule_id,
            slide: v.slide,
            element: v.element.as_deref(),
            message: v.message.to_string(),
            severity: match v.severity {
                Severity::Warning => "warning",
                Severity::Error => "error",
            },
        })
        .collect();

    let output = serde_json::json!({ "violations": items });
    // SAFETY: JsonViolation contains only str/usize/Option primitives — serialization to a String is infallible.
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
