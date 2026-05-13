# pptlint

A CLI linter for PowerPoint presentations that detects alignment and consistency issues.

## Installation

```sh
cargo install --path .
```

## Usage

```sh
pptlint <file.pptx> [OPTIONS]
```

### Options

| Flag | Description |
|---|---|
| `--rules RULE_ID,...` | Run only the specified rules |
| `--disable RULE_ID,...` | Skip the specified rules |
| `--threshold <px>` | Alignment tolerance in pixels (default: `2`) |
| `--slide <n>` | Analyze only slide `n` (1-based) |
| `--group-by slide\|rule` | Group output by slide (default) or rule |
| `--json` | Output violations as JSON |
| `--config <path>` | Config file path (default: `.pptlint.toml`) |

### Examples

```sh
# Lint a file
pptlint deck.pptx

# JSON output, grouped by rule
pptlint deck.pptx --json --group-by rule

# Only run title rules with a looser tolerance
pptlint deck.pptx --rules TITLE_Y,TITLE_FONT_SIZE --threshold 5

# Check a single slide
pptlint deck.pptx --slide 3
```

## Config file

`.pptlint.toml` is loaded from the working directory automatically (or pass `--config`).

```toml
threshold_px = 3

[rules]
disable = ["EDGE_SNAP"]

[output]
group_by = "rule"
json = false
```

## Rules

| Rule ID | Description |
|---|---|
| `TITLE_Y` | Title top-edge inconsistent across slides |
| `TITLE_X_WIDTH` | Title left-edge or width inconsistent across slides |
| `TITLE_FONT_SIZE` | Title font size differs from the majority |
| `COLUMN_LEFT_EDGE` | Left-column elements have inconsistent left edges |
| `COLUMN_TOP_EDGE` | Left and right columns don't start at the same Y |
| `COLUMN_RIGHT_LEFT_EDGE` | Right-column elements have inconsistent left edges |
| `GRID_H_SPACING` | Horizontal gaps between grid elements are uneven |
| `GRID_V_SPACING` | Vertical gaps between grid elements are uneven |
| `GRID_ROW_TOP` | Elements in the same grid row have misaligned top edges |
| `GRID_COL_LEFT` | Elements in the same grid column have misaligned left edges |
| `EDGE_SNAP` | Two elements look nearly aligned but aren't (2–12 px off) |

## Exit codes

- `0` — no violations
- `1` — one or more violations found
