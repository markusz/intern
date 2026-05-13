use serde::Deserialize;
use std::path::Path;

use pptlint::error::PptlintError;

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
    pub json: Option<bool>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, PptlintError> {
        let display = path.display().to_string();
        let content = std::fs::read_to_string(path).map_err(|e| PptlintError::ConfigRead {
            path: display.clone(),
            source: e,
        })?;
        toml::from_str(&content).map_err(|e| PptlintError::ConfigParse {
            path: display,
            source: e,
        })
    }

    /// Returns `Ok(default)` when no config file is present (auto-discovery only).
    /// Returns `Err` if an explicit path was given and the file is missing or malformed,
    /// or if the auto-discovered file exists but cannot be parsed.
    pub fn auto_load(explicit: Option<&Path>) -> Result<Self, PptlintError> {
        match explicit {
            Some(path) => Self::load(path),
            None => {
                let default_path = Path::new(".pptlint.toml");
                if default_path.exists() {
                    Self::load(default_path)
                } else {
                    Ok(Self::default())
                }
            }
        }
    }
}
