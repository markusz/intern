use intern_core::{rules, writer};
use miette::IntoDiagnostic;

use crate::cli::IgnoreArgs;

pub fn run(args: IgnoreArgs) -> miette::Result<()> {
    let path = args
        .file
        .to_str()
        .ok_or_else(|| miette::miette!("invalid file path '{}'", args.file.display()))?;

    validate_rule_id(&args.rule)?;

    let slide_idx = args
        .slide
        .checked_sub(1)
        .ok_or_else(|| miette::miette!("slide number must be 1 or greater"))?;

    writer::append_notes_directive(path, slide_idx, args.element, &args.rule).into_diagnostic()?;

    let directive = match args.element {
        Some(id) => format!("intern: disable({id}) {}", args.rule),
        None => format!("intern: disable {}", args.rule),
    };
    println!(
        "{path}: added \"{directive}\" to slide {} speaker notes (backup: {path}.bak)",
        args.slide
    );
    Ok(())
}

fn validate_rule_id(rule_id: &str) -> miette::Result<()> {
    let known: Vec<&str> = rules::all_rules(&rules::Limits::default())
        .iter()
        .map(|r| r.id())
        .collect();
    if known.contains(&rule_id) {
        Ok(())
    } else {
        miette::bail!("unknown rule id '{rule_id}'")
    }
}

#[cfg(test)]
mod tests {
    use super::validate_rule_id;

    #[test]
    fn known_rule_passes_validation() {
        assert!(validate_rule_id("EMPTY_TEXTBOX").is_ok());
        assert!(validate_rule_id("TITLE_Y").is_ok());
    }

    #[test]
    fn unknown_rule_fails_validation() {
        assert!(validate_rule_id("TYPO_RULE").is_err());
        assert!(validate_rule_id("").is_err());
    }
}
