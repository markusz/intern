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
threshold_px = 2

disable = ["IMAGE_ASPECT_RATIO"]        # turn rules off in bulk
# only  = ["TITLE_Y", "TITLE_X_WIDTH"]  # if set, ONLY these rules run

[output]
format = "table"      # table | text | json
group_by = "rule"     # slide | rule

[rules.TITLE_LENGTH]
max_words = 8

[rules.SLIDE_COUNT]
enabled = false       # turn a single rule off

[rules.BULLET_LENGTH]
max_words = 25
```

## Per-rule tables

Each rule can be configured in its own `[rules.<RULE_ID>]` table. A rule with no
table runs enabled with default settings.

- `enabled = false` turns the rule off.
- The remaining keys are that rule's own settings. The count-based rules take a
  limit:

  | Rule | Key |
  |---|---|
  | `TITLE_LENGTH` | `max_words` |
  | `BULLET_LENGTH` | `max_words` |
  | `FONT_VARIETY` | `max_families` |
  | `COLOR_VARIETY` | `max_colors` |
  | `SLIDE_COUNT` | `max_slides` |

## Blunt controls

- **`disable`** - a top-level list that turns rules off in bulk.
- **`only`** - a top-level whitelist; when present, *only* the listed rules run.
- **Disabling always wins.** If a rule is both whitelisted by `only` and disabled
  (via `disable` or `enabled = false`), it does not run and `intern` prints a
  warning.

## Other settings

- **`threshold_px`** - alignment tolerance in pixels for every geometric rule.
- **`[output]`** - default `format` (`table` | `text` | `json`) and `group_by`
  (`slide` | `rule`) for `intern check`.

CLI flags override the file: `--disable` extends `disable`, and `--rules` replaces
`only`.
