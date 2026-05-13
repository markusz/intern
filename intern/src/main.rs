mod cli;
mod config;
mod fix;
mod report;

use clap::Parser;
use cli::{Cli, Command, GroupBy, OutputFormat};
use config::Config;
use intern_core::{model, reader, rules};
use miette::IntoDiagnostic;
use rules::Rule;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let cfg = Config::auto_load(cli.config.as_deref())?;

    match cli.command {
        Command::Lint(args) => run_lint(args, cfg),
        Command::Fix(args) => fix::run(args, cfg),
    }
}

fn run_lint(args: cli::LintArgs, cfg: Config) -> miette::Result<()> {
    let threshold_px = args.threshold.or(cfg.threshold_px).unwrap_or(2);
    let threshold = threshold_px as i64 * model::EMU_PER_PX;

    let group_by = match args.group_by {
        Some(GroupBy::Rule) => report::GroupBy::Rule,
        Some(GroupBy::Slide) | None => {
            let from_cfg = cfg
                .output
                .as_ref()
                .and_then(|o| o.group_by.as_deref())
                .map(|s| s == "rule")
                .unwrap_or(false);
            if from_cfg {
                report::GroupBy::Rule
            } else {
                report::GroupBy::Slide
            }
        }
    };

    let format = match args.output {
        Some(OutputFormat::Text) => report::OutputFormat::Text,
        Some(OutputFormat::Json) => report::OutputFormat::Json,
        Some(OutputFormat::Table) | None => {
            let from_cfg = cfg
                .output
                .as_ref()
                .and_then(|o| o.format.as_deref())
                .unwrap_or("table");
            match from_cfg {
                "text" => report::OutputFormat::Text,
                "json" => report::OutputFormat::Json,
                _ => report::OutputFormat::Table,
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

    let path = args
        .file
        .to_str()
        .ok_or_else(|| miette::miette!("invalid file path"))?;

    let mut slides = reader::read_presentation(path).into_diagnostic()?;

    if let Some(n) = args.slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let mut disabled = args.disable.unwrap_or_default();
    disabled.extend(cfg_disable);
    let enabled_filter = args.rules.or(cfg_enable);

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

    report::print_violations(&violations, group_by, format);

    std::process::exit(if violations.is_empty() { 0 } else { 1 });
}
