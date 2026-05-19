# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/); the project uses
[semantic versioning](https://semver.org/).

## [0.7.2]

### Removed

- The `GRID_*` and `COLUMN_*` rules (seven rules) and the layout detector behind
  them. The detector was unreliable on varied decks and produced more noise than
  signal; the column-alignment concept will return as a detector-free margin rule.

### Added

- Integration tests covering the `intern ignore` round-trip, including notes-slide
  creation from scratch.

### Changed

- `RULES.md` rewritten as a full reference: every rule now documents what it does,
  why it matters, its defaults, and (for spatial rules) an example diagram.

## [0.7.1]

### Fixed

- Internal lint cleanup; no functional changes.

## [0.7.0]

### Added

- `intern ignore` command: writes an `intern: disable` directive into a slide's
  speaker notes. Takes `-s <slide>`, `-r <rule>`, and an optional `-e <element>`;
  creates the notes slide from scratch when the slide has none.

### Changed

- Slide reader rewritten on a custom `quick-xml` parser, replacing ppt-rs's
  `SlideParser`. It walks `<p:grpSp>` groups and applies nested group transforms,
  so grouped shapes get correct slide coordinates.
- Violations are keyed by each element's actual `<p:cNvPr>` id.
- Text-based rules (font, color, bullets) run by default.
- Improved `table` output formatting.

### Fixed

- Numerous rule-accuracy fixes across the alignment and slide rules.
- Distinct findings are no longer collapsed - each violation is reported once.

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
