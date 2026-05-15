use std::process;

use intern_core::{model, reader, rules, writer};
use miette::IntoDiagnostic;

use crate::cli::FixArgs;
use crate::config::Config;

pub fn run(args: FixArgs, cfg: Config) -> miette::Result<()> {
    let threshold_px = args.threshold.or(cfg.threshold_px).unwrap_or(2);
    let threshold = threshold_px as i64 * model::EMU_PER_PX;

    let path = args
        .file
        .to_str()
        .ok_or_else(|| miette::miette!("invalid file path"))?;

    let mut slides = reader::read_presentation(path).into_diagnostic()?;

    if let Some(n) = args.slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let active = crate::ruleset::select(&cfg, args.rules, args.disable)?;

    let violations: Vec<rules::Violation> = active
        .iter()
        .flat_map(|r| r.check(&slides, threshold))
        .collect();

    let fixes: Vec<rules::Fix> = violations.into_iter().filter_map(|v| v.fix).collect();

    if fixes.is_empty() {
        println!("No fixable violations found.");
        return Ok(());
    }

    if args.dry_run || args.check {
        println!("{} fix(es) would be applied:", fixes.len());
        for fix in &fixes {
            println!("  {fix}");
        }
        if args.check {
            process::exit(1);
        }
        return Ok(());
    }

    let n = fixes.len();
    writer::apply_fixes(path, &fixes).into_diagnostic()?;
    println!("Applied {n} fix(es) to {path}  (original backed up to {path}.bak)");

    Ok(())
}
