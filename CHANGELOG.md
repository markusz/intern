# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/); the project uses
[semantic versioning](https://semver.org/).

## [0.5.0]

### Added

- Per-rule configuration in `[rules.<RULE_ID>]` tables: `enabled`, `severity`,
  `threshold`, and rule-specific limits.
- Per-rule `severity` (`error` / `warning`). `intern check` exits non-zero only
  when an error-severity violation is found; warnings are advisory.
- Per-rule `threshold` override for the alignment rules.
- Slide exclusion via an `intern: disable` speaker-note marker - whole-slide, or
  `intern: disable RULE_ID` for individual rules.
- `check` and `fix` accept multiple files and directories; a bare
  `intern <file>` defaults to `check`.
- An mdBook documentation site under `docs/`.

### Changed

- Config model: the old `[rules]` / `[limits]` sections were replaced by
  `[rules.<RULE_ID>]` tables plus top-level `disable` / `only` lists.
- Config files resolve highest-precedence-first: `--config`, then
  `./.intern.toml`, then `~/.config/intern.toml`.
- Slide order is read from `<p:sldIdLst>`, fixing wrong slide numbers on
  reordered decks.
- JSON output nests violations under a `files` array.
- `SLIDE_COUNT` is off by default - its slide limit is too deck-specific for a
  blanket check; enable it with `[rules.SLIDE_COUNT] enabled = true`.

### Removed

- The `fix --check` flag - CI gates on `check`'s exit code instead.
