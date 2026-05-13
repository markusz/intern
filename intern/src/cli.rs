use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "intern",
    about = "Because your real interns have better things to do than align your ppt boxes"
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

    /// Output format (default: table)
    #[arg(long)]
    pub output: Option<OutputFormat>,

    /// Analyze only this slide (1-based)
    #[arg(long)]
    pub slide: Option<usize>,

    /// Group violations by slide or rule
    #[arg(long)]
    pub group_by: Option<GroupBy>,

    /// Path to config file (default: .intern.toml)
    #[arg(long)]
    pub config: Option<PathBuf>,
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
