use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use intern_core::rules::Limits;
use miette::{Context, IntoDiagnostic};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub threshold_px: Option<u32>,
    pub disable: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
    pub output: Option<OutputConfig>,
    pub rules: Option<HashMap<String, RuleTable>>,
}

/// A `[rules.<RULE_ID>]` table: the rule's on/off switch plus its own settings.
/// A rule with no table runs enabled with default settings.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RuleTable {
    pub enabled: Option<bool>,
    pub max_words: Option<usize>,
    pub max_families: Option<usize>,
    pub max_colors: Option<usize>,
    pub max_slides: Option<usize>,
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

    /// Reads the count-based limits out of the `[rules.*]` tables, falling back to
    /// built-in defaults for any rule without a configured value.
    pub fn limits(&self) -> Limits {
        let d = Limits::default();
        Limits {
            title_words: self.rule_limit("TITLE_LENGTH", |t| t.max_words, d.title_words),
            bullet_words: self.rule_limit("BULLET_LENGTH", |t| t.max_words, d.bullet_words),
            font_families: self.rule_limit("FONT_VARIETY", |t| t.max_families, d.font_families),
            text_colors: self.rule_limit("COLOR_VARIETY", |t| t.max_colors, d.text_colors),
            slide_count: self.rule_limit("SLIDE_COUNT", |t| t.max_slides, d.slide_count),
        }
    }

    fn rule_limit(
        &self,
        id: &str,
        pick: impl Fn(&RuleTable) -> Option<usize>,
        default: usize,
    ) -> usize {
        self.rules
            .as_ref()
            .and_then(|tables| tables.get(id))
            .and_then(pick)
            .unwrap_or(default)
    }

    /// Loads the highest-precedence config file that exists, or built-in defaults
    /// when none is found. An explicit `--config` path that cannot be read is an
    /// error; the auto-discovered files are used only when present.
    pub fn auto_load(explicit: Option<&Path>) -> miette::Result<Self> {
        match resolve_path(explicit) {
            Some(path) => Self::load(&path),
            None => Ok(Self::default()),
        }
    }
}

const PROJECT_CONFIG: &str = ".intern.toml";

/// Picks the config file to load, highest precedence first: an explicit `--config`
/// path, then a project-local `.intern.toml`, then the user config file. Returns
/// `None` when no file is present so the caller falls back to built-in defaults.
/// Files are never merged - the first one found wins as a whole.
fn resolve_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    let project = PathBuf::from(PROJECT_CONFIG);
    if project.is_file() {
        return Some(project);
    }
    user_config_file(
        env::var("XDG_CONFIG_HOME").ok().as_deref(),
        env::var("HOME").ok().as_deref(),
    )
    .filter(|path| path.is_file())
}

/// Resolves the user-level config path: `$XDG_CONFIG_HOME/intern.toml`, or
/// `$HOME/.config/intern.toml` when `XDG_CONFIG_HOME` is unset or empty.
fn user_config_file(xdg_config_home: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if let Some(xdg) = xdg_config_home.filter(|dir| !dir.is_empty()) {
        return Some(Path::new(xdg).join("intern.toml"));
    }
    home.filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(".config").join("intern.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use intern_core::rules::Limits;

    fn defaults() -> Limits {
        Limits::default()
    }

    #[test]
    fn limits_returns_defaults_when_no_rules() {
        let cfg = Config::default();
        let l = cfg.limits();
        let d = defaults();
        assert_eq!(l.title_words, d.title_words);
        assert_eq!(l.bullet_words, d.bullet_words);
        assert_eq!(l.font_families, d.font_families);
        assert_eq!(l.text_colors, d.text_colors);
        assert_eq!(l.slide_count, d.slide_count);
    }

    #[test]
    fn limits_read_from_rule_tables() {
        let mut rules = HashMap::new();
        rules.insert(
            "TITLE_LENGTH".to_string(),
            RuleTable {
                max_words: Some(5),
                ..RuleTable::default()
            },
        );
        rules.insert(
            "SLIDE_COUNT".to_string(),
            RuleTable {
                max_slides: Some(10),
                ..RuleTable::default()
            },
        );
        let cfg = Config {
            rules: Some(rules),
            ..Config::default()
        };
        let l = cfg.limits();
        assert_eq!(l.title_words, 5);
        assert_eq!(l.slide_count, 10);
        // a rule with no table keeps the default
        assert_eq!(l.bullet_words, defaults().bullet_words);
    }

    #[test]
    fn limits_parsed_from_toml() {
        let toml = "[rules.TITLE_LENGTH]\nmax_words = 6\n\n[rules.BULLET_LENGTH]\nmax_words = 15\n";
        let cfg: Config = toml::from_str(toml).unwrap();
        let l = cfg.limits();
        assert_eq!(l.title_words, 6);
        assert_eq!(l.bullet_words, 15);
        assert_eq!(l.font_families, defaults().font_families);
    }

    #[test]
    fn outdated_top_level_section_is_rejected() {
        // deny_unknown_fields: a pre-0.3 [limits] section fails loudly.
        assert!(toml::from_str::<Config>("[limits]\nTITLE_LENGTH = 6\n").is_err());
    }

    #[test]
    fn explicit_config_path_always_wins() {
        let picked = resolve_path(Some(Path::new("/tmp/custom.toml")));
        assert_eq!(picked, Some(PathBuf::from("/tmp/custom.toml")));
    }

    #[test]
    fn user_config_prefers_xdg_config_home() {
        let path = user_config_file(Some("/cfg"), Some("/home/me")).unwrap();
        assert_eq!(path, PathBuf::from("/cfg/intern.toml"));
    }

    #[test]
    fn user_config_falls_back_to_home_when_xdg_unset() {
        let path = user_config_file(None, Some("/home/me")).unwrap();
        assert_eq!(path, PathBuf::from("/home/me/.config/intern.toml"));
    }

    #[test]
    fn user_config_ignores_empty_xdg() {
        let path = user_config_file(Some(""), Some("/home/me")).unwrap();
        assert_eq!(path, PathBuf::from("/home/me/.config/intern.toml"));
    }

    #[test]
    fn user_config_is_none_without_home() {
        assert!(user_config_file(None, None).is_none());
    }
}
