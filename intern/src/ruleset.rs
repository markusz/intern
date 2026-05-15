use intern_core::rules::{self, Rule};

use crate::config::Config;

/// Builds the active rule set from the config file plus CLI overrides. CLI values
/// take precedence over the config. Every referenced rule id is validated against
/// the known set so a typo fails loudly instead of silently disabling every rule
/// (or whitelisting nothing).
pub fn select(
    cfg: &Config,
    cli_rules: Option<Vec<String>>,
    cli_disable: Option<Vec<String>>,
) -> miette::Result<Vec<Box<dyn Rule>>> {
    let mut all = rules::all_rules(&cfg.limits());
    let known: Vec<&str> = all.iter().map(|r| r.id()).collect();

    let mut disable = cli_disable.unwrap_or_default();
    if let Some(from_cfg) = cfg.rules.as_ref().and_then(|r| r.disable.clone()) {
        disable.extend(from_cfg);
    }
    let enable = cli_rules.or_else(|| cfg.rules.as_ref().and_then(|r| r.enable.clone()));

    for id in disable.iter().chain(enable.iter().flatten()) {
        if !known.contains(&id.as_str()) {
            miette::bail!("unknown rule id '{id}'");
        }
    }

    all.retain(|r| {
        if disable.iter().any(|d| d.as_str() == r.id()) {
            return false;
        }
        match &enable {
            Some(only) => only.iter().any(|e| e.as_str() == r.id()),
            None => true,
        }
    });
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_rule_id_in_enable_is_rejected() {
        assert!(select(&Config::default(), Some(vec!["TYPO".into()]), None).is_err());
    }

    #[test]
    fn unknown_rule_id_in_disable_is_rejected() {
        assert!(select(&Config::default(), None, Some(vec!["NOPE".into()])).is_err());
    }

    #[test]
    fn enable_whitelist_keeps_only_named_rules() {
        let active = select(&Config::default(), Some(vec!["TITLE_Y".into()]), None).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id(), "TITLE_Y");
    }

    #[test]
    fn disable_drops_the_named_rule() {
        let full = select(&Config::default(), None, None).unwrap().len();
        let reduced = select(&Config::default(), None, Some(vec!["TITLE_Y".into()]))
            .unwrap()
            .len();
        assert_eq!(reduced + 1, full);
    }
}
