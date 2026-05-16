# Configuration

Drop an `.intern.toml` in your working directory to make settings permanent. Every
field is optional, and CLI flags always override the config file.

```toml
threshold_px = 3

[rules]
disable = ["SLIDE_COUNT", "IMAGE_ASPECT_RATIO"]
# enable = ["TITLE_Y", "TITLE_X_WIDTH"]   # if set, ONLY these rules run

[output]
format = "table"      # table | text | json
group_by = "rule"     # slide | rule

[limits]
title_words   = 10    # TITLE_LENGTH  — max words in a slide title
bullet_words  = 20    # BULLET_LENGTH — max words in a single bullet
font_families = 2     # FONT_VARIETY  — max distinct font families
text_colors   = 3     # COLOR_VARIETY — max distinct text colors
slide_count   = 20    # SLIDE_COUNT   — max slides in the deck
```

## Sections

- **`threshold_px`** — alignment tolerance in pixels for every geometric rule.
- **`[rules]`** — `disable` skips rules; `enable`, when present, acts as a
  whitelist so that *only* the listed rules run.
- **`[output]`** — default output `format` and `group_by` for `intern check`.
- **`[limits]`** — numeric thresholds for the "must be ≤ N" rules.

Use a different file with `--config <path>`.
