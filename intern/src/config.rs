use std::fs;
use std::path::Path;

use miette::{Context, IntoDiagnostic};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub threshold_px: Option<u32>,
    pub rules: Option<RulesConfig>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RulesConfig {
    pub disable: Option<Vec<String>>,
    pub enable: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct OutputConfig {
    pub group_by: Option<String>,
    pub format: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> miette::Result<Self> {
        let display = path.display().to_string();
        let content = fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot read config '{display}'"))?;
        toml::from_str(&content)
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot parse config '{display}'"))
    }

    /// Returns `Ok(default)` when no config file is present (auto-discovery only).
    /// Returns `Err` if an explicit path was given or if the auto-discovered file exists but
    /// cannot be parsed.
    pub fn auto_load(explicit: Option<&Path>) -> miette::Result<Self> {
        match explicit {
            Some(path) => Self::load(path),
            None => {
                let default_path = Path::new(".intern.toml");
                if default_path.exists() {
                    Self::load(default_path)
                } else {
                    Ok(Self::default())
                }
            }
        }
    }
}
