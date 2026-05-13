mod cli;
mod config;

use clap::Parser;
use cli::{Cli, GroupBy};
use config::Config;
use miette::IntoDiagnostic;
use pptlint::{model, reader, report, rules};
use rules::Rule;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let cfg = Config::auto_load(cli.config.as_deref()).into_diagnostic()?;

    let threshold_px = cli.threshold.or(cfg.threshold_px).unwrap_or(2);
    let threshold = threshold_px as i64 * model::EMU_PER_PX;

    let json = cli.json || cfg.output.as_ref().and_then(|o| o.json).unwrap_or(false);

    let group_by = match cli.group_by {
        Some(GroupBy::Rule) => report::GroupBy::Rule,
        Some(GroupBy::Slide) | None => {
            let rule_from_cfg = cfg
                .output
                .as_ref()
                .and_then(|o| o.group_by.as_deref())
                .map(|s| s == "rule")
                .unwrap_or(false);
            if rule_from_cfg {
                report::GroupBy::Rule
            } else {
                report::GroupBy::Slide
            }
        }
    };

    let cfg_disable = cfg
        .rules
        .as_ref()
        .and_then(|r| r.disable.as_ref())
        .cloned()
        .unwrap_or_default();
    let cfg_enable = cfg.rules.and_then(|r| r.enable);

    let path = cli
        .file
        .to_str()
        .ok_or_else(|| miette::miette!("invalid file path"))?;

    let mut slides = reader::read_presentation(path).into_diagnostic()?;

    if let Some(n) = cli.slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let mut disabled = cli.disable.unwrap_or_default();
    disabled.extend(cfg_disable);

    let enabled_filter: Option<Vec<String>> = cli.rules.or(cfg_enable);

    let all = rules::all_rules();
    let active: Vec<&dyn Rule> = all
        .iter()
        .map(|r| r.as_ref())
        .filter(|r| {
            if disabled.iter().any(|d| d == r.id()) {
                return false;
            }
            if let Some(ref ef) = enabled_filter {
                return ef.iter().any(|e| e == r.id());
            }
            true
        })
        .collect();

    let violations: Vec<rules::Violation> = active
        .iter()
        .flat_map(|r| r.check(&slides, threshold))
        .collect();

    report::print_violations(&violations, group_by, json);

    std::process::exit(if violations.is_empty() { 0 } else { 1 });
}
