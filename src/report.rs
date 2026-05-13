use serde::Serialize;

use crate::rules::{Severity, Violation};

pub enum GroupBy {
    Slide,
    Rule,
}

pub fn print_violations(violations: &[Violation], group_by: GroupBy, json: bool) {
    if json {
        print_json(violations);
        return;
    }
    if violations.is_empty() {
        println!("No violations found.");
        return;
    }
    match group_by {
        GroupBy::Slide => print_by_slide(violations),
        GroupBy::Rule => print_by_rule(violations),
    }
}

fn print_by_slide(violations: &[Violation]) {
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

fn print_by_rule(violations: &[Violation]) {
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
    message: &'a str,
    severity: &'a str,
}

fn print_json(violations: &[Violation]) {
    let items: Vec<JsonViolation> = violations
        .iter()
        .map(|v| JsonViolation {
            rule_id: v.rule_id,
            slide: v.slide,
            element: v.element.as_deref(),
            message: &v.message,
            severity: match v.severity {
                Severity::Warning => "warning",
                Severity::Error => "error",
            },
        })
        .collect();

    let output = serde_json::json!({ "violations": items });
    // SAFETY: JsonViolation contains only str/usize/Option primitives; serializing to a String has no I/O path and cannot fail.
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
