use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use intern_core::rules::Limits;
use miette::{Context, IntoDiagnostic};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub threshold_px: Option<u32>,
    pub rules: Option<RulesConfig>,
    pub output: Option<OutputConfig>,
    pub limits: Option<LimitsConfig>,
}

/// The `[limits]` section, keyed by rule id (e.g. `TITLE_LENGTH = 10`).
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct LimitsConfig {
    pub title_length: Option<usize>,
    pub bullet_length: Option<usize>,
    pub font_variety: Option<usize>,
    pub color_variety: Option<usize>,
    pub slide_count: Option<usize>,
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

    /// Converts the optional `[limits]` section into a `Limits` value, falling back to defaults.
    pub fn limits(&self) -> Limits {
        let cfg = self.limits.as_ref();
        let defaults = Limits::default();
        Limits {
            title_words: cfg
                .and_then(|l| l.title_length)
                .unwrap_or(defaults.title_words),
            bullet_words: cfg
                .and_then(|l| l.bullet_length)
                .unwrap_or(defaults.bullet_words),
            font_families: cfg
                .and_then(|l| l.font_variety)
                .unwrap_or(defaults.font_families),
            text_colors: cfg
                .and_then(|l| l.color_variety)
                .unwrap_or(defaults.text_colors),
            slide_count: cfg
                .and_then(|l| l.slide_count)
                .unwrap_or(defaults.slide_count),
        }
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
    fn limits_returns_defaults_when_no_section() {
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
    fn limits_overrides_configured_fields() {
        let cfg = Config {
            limits: Some(LimitsConfig {
                title_length: Some(5),
                slide_count: Some(10),
                ..LimitsConfig::default()
            }),
            ..Config::default()
        };
        let l = cfg.limits();
        assert_eq!(l.title_words, 5);
        assert_eq!(l.slide_count, 10);
        // unset fields fall back to defaults
        assert_eq!(l.bullet_words, defaults().bullet_words);
        assert_eq!(l.font_families, defaults().font_families);
        assert_eq!(l.text_colors, defaults().text_colors);
    }

    #[test]
    fn limits_parsed_from_toml() {
        let toml = "[limits]\nTITLE_LENGTH = 6\nBULLET_LENGTH = 15\n";
        let cfg: Config = toml::from_str(toml).unwrap();
        let l = cfg.limits();
        assert_eq!(l.title_words, 6);
        assert_eq!(l.bullet_words, 15);
        assert_eq!(l.font_families, defaults().font_families);
    }

    #[test]
    fn limits_drive_title_length_rule() {
        use intern_core::model::{ElementKind, Rect, SlideData, SlideElement};
        use intern_core::rules::all_rules;

        let make_slide = || SlideData {
            index: 0,
            elements: vec![SlideElement {
                name: "Title 1".into(),
                kind: ElementKind::Title,
                rect: Rect {
                    x: 457_200,
                    y: 274_638,
                    w: 8_229_600,
                    h: 1_143_000,
                },
                font_size: None,
                font_family: None,
                text_color: None,
                // 6 words - should fire when limit=5, be silent when limit=7
                paragraphs: vec!["one two three four five six".into()],
            }],
        };

        let threshold = 19_050;

        let tight = Config {
            limits: Some(LimitsConfig {
                title_length: Some(5),
                ..LimitsConfig::default()
            }),
            ..Config::default()
        };
        let fires: Vec<_> = all_rules(&tight.limits())
            .iter()
            .filter(|r| r.id() == "TITLE_LENGTH")
            .flat_map(|r| r.check(&[make_slide()], threshold))
            .collect();
        assert_eq!(fires.len(), 1, "limit=5 should fire on a 6-word title");

        let loose = Config {
            limits: Some(LimitsConfig {
                title_length: Some(7),
                ..LimitsConfig::default()
            }),
            ..Config::default()
        };
        let silent: Vec<_> = all_rules(&loose.limits())
            .iter()
            .filter(|r| r.id() == "TITLE_LENGTH")
            .flat_map(|r| r.check(&[make_slide()], threshold))
            .collect();
        assert!(
            silent.is_empty(),
            "limit=7 should not fire on a 6-word title"
        );
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
