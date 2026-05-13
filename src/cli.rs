use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "pptlint",
    about = "Lint PowerPoint presentations for alignment issues"
)]
pub struct Cli {
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

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Analyze only this slide (1-based)
    #[arg(long)]
    pub slide: Option<usize>,

    /// Group violations by slide or rule
    #[arg(long)]
    pub group_by: Option<GroupBy>,

    /// Path to config file (default: .pptlint.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum GroupBy {
    Slide,
    Rule,
}
