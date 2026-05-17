use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "intern",
    version,
    about = "Because your real interns have better things to do than align your ppt boxes",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
pub struct Cli {
    /// Load settings from a specific config file
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub check: CheckArgs,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check presentations and report violations (the default action)
    Check(CheckArgs),
    /// Fix violations in place, backing up each file to <file>.bak
    Fix(FixArgs),
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Presentation files or directories (a directory expands to its .pptx files)
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Run only these rule IDs (comma-separated)
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
    /// Presentation files or directories (a directory expands to its .pptx files)
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Run only these rule IDs (comma-separated)
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

    /// Print what would change without writing
    #[arg(long)]
    pub dry_run: bool,
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
