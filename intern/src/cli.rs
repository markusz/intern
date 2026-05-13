use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "intern",
    about = "Because your real interns have better things to do than align your ppt boxes"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to config file (default: .intern.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Lint a presentation and report violations
    Lint(LintArgs),
    /// Fix violations in a presentation (writes in-place, backs up to <file>.bak)
    Fix(FixArgs),
}

#[derive(Args, Debug)]
pub struct LintArgs {
    pub file: PathBuf,

    /// Only run these rule IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub rules: Option<Vec<String>>,

    /// Disable these rule IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub disable: Option<Vec<String>>,

    /// Alignment tolerance in pixels (default: 2)
    #[arg(long)]
    pub threshold: Option<u32>,

    /// Output format (default: table)
    #[arg(long)]
    pub output: Option<OutputFormat>,

    /// Analyze only this slide (1-based)
    #[arg(long)]
    pub slide: Option<usize>,

    /// Group violations by slide or rule
    #[arg(long)]
    pub group_by: Option<GroupBy>,
}

#[derive(Args, Debug)]
pub struct FixArgs {
    pub file: PathBuf,

    /// Only run these rule IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub rules: Option<Vec<String>>,

    /// Disable these rule IDs (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub disable: Option<Vec<String>>,

    /// Alignment tolerance in pixels (default: 2)
    #[arg(long)]
    pub threshold: Option<u32>,

    /// Analyze only this slide (1-based)
    #[arg(long)]
    pub slide: Option<usize>,

    /// Print what would change without writing the file
    #[arg(long)]
    pub dry_run: bool,

    /// Exit 1 if any fixes would be applied (CI gate)
    #[arg(long)]
    pub check: bool,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum GroupBy {
    Slide,
    Rule,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum OutputFormat {
    Table,
    Text,
    Json,
}
