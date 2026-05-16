mod cli;
mod config;
mod fix;
mod input;
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
        Some(Command::Check(args)) => run_check(args, cfg),
        Some(Command::Fix(args)) => fix::run(args, cfg),
        None => run_check(cli.check, cfg),
    }
}

fn run_check(args: CheckArgs, cfg: Config) -> miette::Result<()> {
    let threshold_px = args.threshold.or(cfg.threshold_px).unwrap_or(2);
    let threshold = threshold_px as i64 * model::EMU_PER_PX;
    let group_by = resolve_group_by(args.group_by, &cfg);
    let format = resolve_format(args.output, &cfg);

    let selection = ruleset::select(&cfg, args.rules, args.disable)?;
    for warning in &selection.warnings {
        eprintln!("warning: {warning}");
    }

    let files = input::collect_pptx(&args.files)?;
    let mut results: Vec<(String, Vec<rules::Violation>)> = Vec::new();
    for file in &files {
        let path = file
            .to_str()
            .ok_or_else(|| miette::miette!("invalid file path '{}'", file.display()))?;
        let mut slides = reader::read_presentation(path).into_diagnostic()?;
        if let Some(n) = args.slide {
            slides.retain(|s| s.index + 1 == n);
        }
        let violations: Vec<rules::Violation> = selection
            .rules
            .iter()
            .flat_map(|r| r.check(&slides, threshold))
            .collect();
        results.push((path.to_string(), violations));
    }

    let clean = results.iter().all(|(_, v)| v.is_empty());
    report::print_results(&results, group_by, format);
    process::exit(if clean { 0 } else { 1 });
}

fn resolve_group_by(cli: Option<GroupBy>, cfg: &Config) -> report::GroupBy {
    match cli {
        Some(GroupBy::Rule) => report::GroupBy::Rule,
        Some(GroupBy::Slide) => report::GroupBy::Slide,
        None => {
            let rule_grouped = cfg
                .output
                .as_ref()
                .and_then(|o| o.group_by.as_deref())
                .map(|s| s == "rule")
                .unwrap_or(false);
            if rule_grouped {
                report::GroupBy::Rule
            } else {
                report::GroupBy::Slide
            }
        }
    }
}

fn resolve_format(cli: Option<OutputFormat>, cfg: &Config) -> report::OutputFormat {
    match cli {
        Some(OutputFormat::Text) => report::OutputFormat::Text,
        Some(OutputFormat::Json) => report::OutputFormat::Json,
        Some(OutputFormat::Table) => report::OutputFormat::Table,
        None => match cfg.output.as_ref().and_then(|o| o.format.as_deref()) {
            Some("text") => report::OutputFormat::Text,
            Some("json") => report::OutputFormat::Json,
            _ => report::OutputFormat::Table,
        },
    }
}
