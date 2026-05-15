mod cli;
mod config;
mod fix;
mod report;
mod ruleset;

use std::process;

use clap::Parser;
use cli::{CheckArgs, Cli, Command, GroupBy, OutputFormat};
use config::Config;
use intern_core::{model, reader, rules};
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let cfg = Config::auto_load(cli.config.as_deref())?;

    match cli.command {
        Command::Check(args) => run_check(args, cfg),
        Command::Fix(args) => fix::run(args, cfg),
    }
}

fn run_check(args: CheckArgs, cfg: Config) -> miette::Result<()> {
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

    let active = ruleset::select(&cfg, args.rules, args.disable)?;

    let path = args
        .file
        .to_str()
        .ok_or_else(|| miette::miette!("invalid file path"))?;

    let mut slides = reader::read_presentation(path).into_diagnostic()?;

    if let Some(n) = args.slide {
        slides.retain(|s| s.index + 1 == n);
    }

    let violations: Vec<rules::Violation> = active
        .iter()
        .flat_map(|r| r.check(&slides, threshold))
        .collect();

    report::print_violations(&violations, group_by, format);

    process::exit(if violations.is_empty() { 0 } else { 1 });
}
