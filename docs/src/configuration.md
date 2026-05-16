# Configuration

Settings can live in a TOML file so you don't have to repeat flags on every run.
Every field is optional, and CLI flags always override whatever the file provides.

## Where intern looks

`intern` loads the **first** file it finds, in this order:

1. the path passed to `--config <file>`
2. `./.intern.toml` in the current directory (project config)
3. `$XDG_CONFIG_HOME/intern.toml`, or `~/.config/intern.toml` (user config)

If none exists, the built-in defaults apply. Files are **not merged** - the
highest-precedence file wins as a whole, and CLI flags then layer on top of it.

A path passed to `--config` must exist, or `intern` exits with an error; the
auto-discovered files are used only when present.

## Example

```toml
threshold_px = 3

[rules]
disable = ["SLIDE_COUNT", "IMAGE_ASPECT_RATIO"]
# enable = ["TITLE_Y", "TITLE_X_WIDTH"]   # if set, ONLY these rules run

[output]
format = "table"      # table | text | json
group_by = "rule"     # slide | rule

[limits]
TITLE_LENGTH  = 10    # max words in a slide title
BULLET_LENGTH = 20    # max words in a single bullet
FONT_VARIETY  = 2     # max distinct font families
COLOR_VARIETY = 3     # max distinct text colors
SLIDE_COUNT   = 20    # max slides in the deck
```

## Sections

- **`threshold_px`** - alignment tolerance in pixels for every geometric rule.
- **`[rules]`** - `disable` skips rules; `enable`, when present, acts as a
  whitelist so that *only* the listed rules run.
- **`[output]`** - default output `format` and `group_by` for `intern check`.
- **`[limits]`** - numeric thresholds for the count-based rules, keyed by rule
  id: `TITLE_LENGTH`, `BULLET_LENGTH`, `FONT_VARIETY`, `COLOR_VARIETY`,
  `SLIDE_COUNT`.
