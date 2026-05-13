![intern](logo.png)

> Because your real interns have better things to do than align your ppt boxes.

A linter for PowerPoint presentations that detects alignment and consistency issues.

## Workspace layout

```
intern/
├── intern-core/   # library — rules engine, reader, detector (no CLI deps)
└── intern/        # CLI binary
```

`intern-core` can be used independently if you want to embed the engine in your own tooling.

## Installation

```sh
cargo install --path intern
```

## Usage

```sh
intern <file.pptx> [OPTIONS]
```

### Options

| Flag | Description |
|---|---|
| `--rules RULE_ID,...` | Run only the specified rules |
| `--disable RULE_ID,...` | Skip the specified rules |
| `--threshold <px>` | Alignment tolerance in pixels (default: `2`) |
| `--slide <n>` | Analyze only slide `n` (1-based) |
| `--output table\|text\|json` | Output format (default: `table`) |
| `--group-by slide\|rule` | Group violations by slide (default) or rule |
| `--config <path>` | Config file path (default: `.intern.toml`) |

### Examples

```sh
# Lint a file
intern deck.pptx

# Table output grouped by rule
intern deck.pptx --group-by rule

# JSON output
intern deck.pptx --output json

# Only run title rules with a looser tolerance
intern deck.pptx --rules TITLE_Y,TITLE_FONT_SIZE --threshold 5

# Check a single slide
intern deck.pptx --slide 3
```

## Config file

`.intern.toml` is auto-discovered from the working directory, or pass `--config <path>`.

```toml
threshold_px = 3

[rules]
disable = ["TITLE_Y"]

[output]
group_by = "rule"
format = "table"   # table | text | json
```

## Rules

Full documentation with diagrams: [RULES.md](RULES.md)

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

## Exit codes

- `0` — no violations
- `1` — one or more violations found

## Embedding `intern-core`

```toml
[dependencies]
intern-core = { git = "..." }
```

```rust
use intern_core::{reader::read_presentation, rules::all_rules, model::EMU_PER_PX};

let slides = read_presentation("deck.pptx")?;
let violations: Vec<_> = all_rules()
    .iter()
    .flat_map(|r| r.check(&slides, 2 * EMU_PER_PX))
    .collect();
```

A runnable version of this is in [`intern-core/examples/lint.rs`](intern-core/examples/lint.rs):

```sh
cargo run -p intern-core --example lint -- deck.pptx
```
