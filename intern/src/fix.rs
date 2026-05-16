use intern_core::{model, reader, rules, writer};
use miette::IntoDiagnostic;

use crate::cli::FixArgs;
use crate::config::Config;
use crate::input;
use crate::ruleset::{self, Selection};

pub fn run(args: FixArgs, cfg: Config) -> miette::Result<()> {
    let global_px = args.threshold.or(cfg.threshold_px).unwrap_or(2);

    let selection = ruleset::select(&cfg, args.rules, args.disable)?;
    for warning in &selection.warnings {
        eprintln!("warning: {warning}");
    }

    let files = input::collect_pptx(&args.files)?;
    for file in &files {
        let path = file
            .to_str()
            .ok_or_else(|| miette::miette!("invalid file path '{}'", file.display()))?;
        fix_one(path, args.slide, args.dry_run, global_px, &cfg, &selection)?;
    }
    Ok(())
}

fn fix_one(
    path: &str,
    slide: Option<usize>,
    dry_run: bool,
    global_px: u32,
    cfg: &Config,
    selection: &Selection,
) -> miette::Result<()> {
    let mut slides = reader::read_presentation(path).into_diagnostic()?;
    let ignored = reader::ignored_slide_indices(path).into_diagnostic()?;
    slides.retain(|s| !ignored.contains(&s.index));
    if let Some(n) = slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let mut fixes: Vec<rules::Fix> = Vec::new();
    for rule in &selection.rules {
        let threshold = cfg.rule_threshold_px(rule.id(), global_px) as i64 * model::EMU_PER_PX;
        fixes.extend(
            rule.check(&slides, threshold)
                .into_iter()
                .filter_map(|v| v.fix),
        );
    }

    if fixes.is_empty() {
        println!("{path}: no fixable violations");
        return Ok(());
    }
    if dry_run {
        println!("{path}: {} fix(es) would be applied:", fixes.len());
        for fix in &fixes {
            println!("  {fix}");
        }
        return Ok(());
    }

    let n = fixes.len();
    writer::apply_fixes(path, &fixes).into_diagnostic()?;
    println!("{path}: applied {n} fix(es) (backup: {path}.bak)");
    Ok(())
}
