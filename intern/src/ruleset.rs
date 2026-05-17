use intern_core::rules::{self, Rule};

use crate::config::Config;

/// Rules that do not run unless opted in - their default threshold is too
/// deck-specific to be a useful blanket check. A default-off rule runs only when its
/// `[rules.X]` table sets `enabled = true`, or it is named in `only` / `--rules`.
const DEFAULT_OFF: &[&str] = &["SLIDE_COUNT"];

fn is_default_off(id: &str) -> bool {
    DEFAULT_OFF.contains(&id)
}

/// The outcome of resolving which rules to run: the active rule set plus any
/// non-fatal warnings (e.g. a rule both whitelisted and disabled).
pub struct Selection {
    pub rules: Vec<Box<dyn Rule>>,
    pub warnings: Vec<String>,
}

/// Builds the active rule set from the config file plus CLI overrides.
///
/// Precedence: disabling always wins. A rule is off if its `[rules.X]` table sets
/// `enabled = false` or it appears in `disable` (config or `--disable`); that beats
/// the `only` whitelist and an explicit `enabled = true`. Such conflicts are
/// collected as warnings rather than failing. Every referenced rule id is validated
/// so a typo fails loudly.
pub fn select(
    cfg: &Config,
    cli_rules: Option<Vec<String>>,
    cli_disable: Option<Vec<String>>,
) -> miette::Result<Selection> {
    let mut all = rules::all_rules(&cfg.limits());
    let known: Vec<&str> = all.iter().map(|r| r.id()).collect();

    let mut disable: Vec<String> = cli_disable.unwrap_or_default();
    if let Some(from_cfg) = &cfg.disable {
        disable.extend(from_cfg.iter().cloned());
    }
    let per_rule_off = rules_flagged(cfg, false);
    let per_rule_on = rules_flagged(cfg, true);

    // CLI `--rules` replaces the config `only` whitelist.
    let whitelist = cli_rules.or_else(|| cfg.only.clone());

    validate_ids(&known, cfg, &disable, whitelist.as_deref())?;

    let is_off = |id: &str| disable.iter().any(|d| d == id) || per_rule_off.iter().any(|d| d == id);

    let mut warnings = Vec::new();
    for id in &per_rule_on {
        if disable.iter().any(|d| d == id) {
            warnings.push(format!(
                "rule '{id}' is enabled in [rules.{id}] but also listed in `disable`; it will not run"
            ));
        }
    }
    for id in whitelist.iter().flatten() {
        if is_off(id) {
            warnings.push(format!(
                "rule '{id}' is whitelisted but also disabled; it will not run"
            ));
        }
    }

    all.retain(|r| {
        let id = r.id();
        if is_off(id) {
            return false;
        }
        if let Some(only) = &whitelist {
            return only.iter().any(|w| w == id);
        }
        // No whitelist: every rule runs except a default-off rule not explicitly enabled.
        !is_default_off(id) || per_rule_on.iter().any(|e| e == id)
    });

    Ok(Selection {
        rules: all,
        warnings,
    })
}

/// Collects the ids of rules whose `[rules.X]` table sets `enabled` to `flag`.
fn rules_flagged(cfg: &Config, flag: bool) -> Vec<String> {
    cfg.rules
        .iter()
        .flatten()
        .filter(|(_, table)| table.enabled == Some(flag))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Fails if any referenced rule id - in a `[rules.X]` table, `disable`, or the
/// whitelist - is not a real rule.
fn validate_ids(
    known: &[&str],
    cfg: &Config,
    disable: &[String],
    whitelist: Option<&[String]>,
) -> miette::Result<()> {
    let from_tables = cfg.rules.iter().flatten().map(|(id, _)| id.as_str());
    let from_disable = disable.iter().map(String::as_str);
    let from_whitelist = whitelist.into_iter().flatten().map(String::as_str);
    for id in from_tables.chain(from_disable).chain(from_whitelist) {
        if !known.contains(&id) {
            miette::bail!("unknown rule id '{id}'");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuleTable;

    fn config(rules: Vec<(&str, RuleTable)>, disable: &[&str], only: Option<&[&str]>) -> Config {
        Config {
            rules: (!rules.is_empty()).then(|| {
                rules
                    .into_iter()
                    .map(|(id, table)| (id.to_string(), table))
                    .collect()
            }),
            disable: (!disable.is_empty()).then(|| disable.iter().map(|s| s.to_string()).collect()),
            only: only.map(|ids| ids.iter().map(|s| s.to_string()).collect()),
            ..Config::default()
        }
    }

    fn off() -> RuleTable {
        RuleTable {
            enabled: Some(false),
            ..RuleTable::default()
        }
    }

    fn on() -> RuleTable {
        RuleTable {
            enabled: Some(true),
            ..RuleTable::default()
        }
    }

    #[test]
    fn unknown_id_in_disable_is_rejected() {
        assert!(select(&config(vec![], &["NOPE"], None), None, None).is_err());
    }

    #[test]
    fn unknown_id_in_only_is_rejected() {
        assert!(select(&config(vec![], &[], Some(&["TYPO"])), None, None).is_err());
    }

    #[test]
    fn unknown_id_as_rule_table_is_rejected() {
        let cfg = config(vec![("NOSUCH", RuleTable::default())], &[], None);
        assert!(select(&cfg, None, None).is_err());
    }

    #[test]
    fn only_keeps_just_the_whitelisted_rules() {
        let sel = select(&config(vec![], &[], Some(&["TITLE_Y"])), None, None).unwrap();
        assert_eq!(sel.rules.len(), 1);
        assert_eq!(sel.rules[0].id(), "TITLE_Y");
        assert!(sel.warnings.is_empty());
    }

    #[test]
    fn per_rule_enabled_false_disables_the_rule() {
        let cfg = config(vec![("TITLE_Y", off())], &[], None);
        let sel = select(&cfg, None, None).unwrap();
        assert!(sel.rules.iter().all(|r| r.id() != "TITLE_Y"));
    }

    #[test]
    fn global_disable_drops_the_rule() {
        let full = select(&Config::default(), None, None).unwrap().rules.len();
        let sel = select(&config(vec![], &["TITLE_Y"], None), None, None).unwrap();
        assert_eq!(sel.rules.len() + 1, full);
    }

    #[test]
    fn whitelisted_but_disabled_warns_and_excludes() {
        let cfg = config(vec![], &["TITLE_Y"], Some(&["TITLE_Y", "TITLE_X_WIDTH"]));
        let sel = select(&cfg, None, None).unwrap();
        assert!(sel.rules.iter().all(|r| r.id() != "TITLE_Y"));
        assert_eq!(sel.warnings.len(), 1);
        assert!(sel.warnings[0].contains("TITLE_Y"));
    }

    #[test]
    fn enabled_true_but_globally_disabled_warns() {
        let cfg = config(vec![("TITLE_Y", on())], &["TITLE_Y"], None);
        let sel = select(&cfg, None, None).unwrap();
        assert!(sel.rules.iter().all(|r| r.id() != "TITLE_Y"));
        assert_eq!(sel.warnings.len(), 1);
    }

    #[test]
    fn cli_rules_replace_config_only() {
        let cfg = config(vec![], &[], Some(&["TITLE_Y"]));
        let sel = select(&cfg, Some(vec!["TITLE_X_WIDTH".into()]), None).unwrap();
        assert_eq!(sel.rules.len(), 1);
        assert_eq!(sel.rules[0].id(), "TITLE_X_WIDTH");
    }

    #[test]
    fn default_off_rule_is_excluded_without_opt_in() {
        let sel = select(&Config::default(), None, None).unwrap();
        assert!(sel.rules.iter().all(|r| r.id() != "SLIDE_COUNT"));
    }

    #[test]
    fn default_off_rule_runs_when_explicitly_enabled() {
        let cfg = config(vec![("SLIDE_COUNT", on())], &[], None);
        let sel = select(&cfg, None, None).unwrap();
        assert!(sel.rules.iter().any(|r| r.id() == "SLIDE_COUNT"));
    }

    #[test]
    fn default_off_rule_stays_off_with_options_but_no_enable() {
        // A table that only sets options does not switch a default-off rule on.
        let cfg = config(
            vec![(
                "SLIDE_COUNT",
                RuleTable {
                    max_slides: Some(40),
                    ..RuleTable::default()
                },
            )],
            &[],
            None,
        );
        let sel = select(&cfg, None, None).unwrap();
        assert!(sel.rules.iter().all(|r| r.id() != "SLIDE_COUNT"));
    }

    #[test]
    fn default_off_rule_runs_when_whitelisted() {
        let sel = select(&Config::default(), Some(vec!["SLIDE_COUNT".into()]), None).unwrap();
        assert_eq!(sel.rules.len(), 1);
        assert_eq!(sel.rules[0].id(), "SLIDE_COUNT");
    }
}
