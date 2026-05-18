![intern](logo.png)

[![CI](https://github.com/markusz/intern/actions/workflows/ci.yml/badge.svg)](https://github.com/markusz/intern/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/markusz/intern/branch/main/graph/badge.svg)](https://codecov.io/gh/markusz/intern)
[![Release](https://img.shields.io/github/v/release/markusz/intern)](https://github.com/markusz/intern/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

> Because your real interns have better things to do than align your ppt boxes.

**intern** is a linter for PowerPoint files. Point it at a `.pptx` and it tells you exactly what's wrong - misaligned boxes, inconsistent fonts, sloppy text, duplicate titles. It can automatically fix alignment, font-size, and whitespace issues.

Existing tools are proprietary Office add-ins or AI-powered web uploads. **intern** is the first open-source, rule-based CLI linter for PowerPoint - configurable, scriptable, and CI-friendly.

**[Read the documentation →](https://markusz.github.io/intern/)**

---

```
$ intern check quarterly.pptx

  Slide  Rule                  Element  Message
  ────────────────────────────────────────────────────────────────────────────
  2      TITLE_Y               Title 2  title is 34.2px lower than on most slides
  3      FONT_SIZE_VARIETY     -        4 distinct body font sizes (limit: 3)
  4      TITLE_TRAILING_PUNCT  -        title ends with '.' - remove it
  5      ELEMENT_OVERFLOW      Shape 3  element extends outside slide bounds
  6      DOUBLE_SPACE          Body     paragraph contains double spaces
  7      DUPLICATE_TITLE       Title 7  same title as slide 2
  9      TEXT_ELEMENT_OVERLAP  Text 4   bounding box overlaps with 'Text 5'

7 violation(s) (7 error, 0 warning)
```

---

## Installation

Pick whichever fits your setup - all three give you the same `intern` binary.

### Homebrew - macOS & Linux

```sh
brew install markusz/intern/intern
```

### Prebuilt binary

Download the archive for your platform from the [latest release](https://github.com/markusz/intern/releases/latest), extract it, and move `intern` onto your `PATH`.

| Platform | Archive |
|---|---|
| macOS (Apple Silicon) | `intern-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `intern-x86_64-apple-darwin.tar.gz` |
| Linux (x86-64) | `intern-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86-64) | `intern-x86_64-pc-windows-msvc.zip` |

One-liner for macOS/Linux (swap in the archive from the table):

```sh
curl -L https://github.com/markusz/intern/releases/latest/download/intern-aarch64-apple-darwin.tar.gz | tar xz
sudo mv intern /usr/local/bin/
```

### Build from source

Requires [Rust](https://rustup.rs).

```sh
cargo install --path intern
```

---

## Usage

```sh
intern deck.pptx          # check (the default action)
intern check slides/      # check every .pptx in a folder
intern fix deck.pptx      # auto-fix violations in place
```

That's it. No configuration required to get started - but for ongoing use, most
teams keep an `.intern.toml` with their preferred thresholds and rule set (see
[Config file](#config-file) below).

### Options

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Analyze only slide `n` |
| `--output table\|text\|json` | `table` | Output format |
| `--group-by slide\|rule` | `slide` | Group violations |
| `--config <path>` | auto | Load settings from a specific config file |

### Use in CI

```sh
intern check deck.pptx --output json > violations.json
```

Exit code is `0` when clean (or only warnings) and `1` when an error-severity violation is found - standard for shell scripting and CI pipelines.

### Config file

Settings can live in a TOML file. `intern` loads the first one it finds:

1. the path passed to `--config <file>`
2. `./.intern.toml` in the current directory (project config)
3. `~/.config/intern.toml` (user config, honours `$XDG_CONFIG_HOME`)

Files are not merged - the highest-precedence file wins as a whole, and CLI flags
override individual settings on top of it.

```toml
threshold_px = 2

disable = ["ALL_CAPS"]                  # turn rules off in bulk
# only  = ["TITLE_Y", "TITLE_X_WIDTH"]  # if set, ONLY these rules run

[output]
format = "table"
group_by = "rule"

[rules.TITLE_LENGTH]
max_words = 8

[rules.ALL_CAPS]
severity = "warning"   # report it, but don't fail CI

[rules.SLIDE_COUNT]
enabled = true         # SLIDE_COUNT is off by default; enable it explicitly
max_slides = 40
```

Each rule is configured in its own `[rules.<RULE_ID>]` table; `disable` and `only`
are blunt top-level lists. See the [documentation](https://markusz.github.io/intern/configuration.html) for the full reference.

### Skipping a slide

Exclude a slide from checks by adding a line to its **speaker notes** - handy for
title slides, section dividers, or a deliberately different layout:

```text
intern: disable                        # skip every rule on this slide
intern: disable TITLE_Y, GRID_ROW_TOP  # skip only these rules
```

The slide is dropped before those rules run, so it skews no baselines (like the
median title position) either.

---

## Rules

36 rules across four categories. Rules marked **off** require `enabled = true` in
`[rules.<RULE_ID>]` or `--rules <ID>` to run. Any on-by-default rule can be
suppressed with `--disable`.

### Alignment

| Rule | Status | What it catches |
|---|---|---|
| `LEFT_MARGIN` | on | Slide's leftmost unit is off the typical left margin |
| `RIGHT_MARGIN` | on | Slide's rightmost unit right edge is off the typical right margin |
| `BOTTOM_MARGIN` | on | Content extends deeper than the typical bottom margin |
| `TITLE_MARGIN` | on | Gap between title and nearest content unit differs from the typical gap |
| `CLOSE_X` | on | Two units have X positions within threshold - likely misaligned |
| `CLOSE_Y` | on | Two units have Y positions within threshold - likely misaligned |
| `TITLE_Y` | on | Title top-edge inconsistent across slides |
| `TITLE_X_WIDTH` | on | Title left-edge or width inconsistent across slides |
| `TEXT_ELEMENT_OVERLAP` | on | Two text-bearing elements on the same slide have overlapping rects |
| `ELEMENT_OVERFLOW` | on | Element extends outside the slide bounds |
| `COLUMN_LEFT_EDGE` | **off** | Left-column elements have inconsistent left edges |
| `COLUMN_TOP_EDGE` | **off** | Left and right columns don't start at the same Y |
| `COLUMN_RIGHT_LEFT_EDGE` | **off** | Right-column elements have inconsistent left edges |
| `GRID_H_SPACING` | **off** | Horizontal gaps between grid elements are uneven |
| `GRID_V_SPACING` | **off** | Vertical gaps between grid elements are uneven |
| `GRID_ROW_TOP` | **off** | Elements in the same grid row have misaligned top edges |
| `GRID_COL_LEFT` | **off** | Elements in the same grid column have misaligned left edges |

### Typography

| Rule | Status | What it catches |
|---|---|---|
| `TITLE_FONT_SIZE` | on | Title font size differs from the majority |
| `FONT_SIZE_VARIETY` | on | Too many distinct body font sizes across the deck |
| `BODY_FONT_FAMILY` | on | Body font family differs from the majority across slides |
| `FONT_VARIETY` | on | Too many distinct font families across the deck |
| `COLOR_VARIETY` | on | Too many distinct text colors across the deck |
| `BODY_TEXT_COLOR` | **off** | Body text color differs from the majority across slides |

### Text quality

| Rule | Status | What it catches |
|---|---|---|
| `DOUBLE_SPACE` | on | Paragraph contains two or more consecutive spaces |
| `LEADING_SPACE` | on | Paragraph starts with whitespace |
| `REPEATED_WORD` | on | Two consecutive identical words ("the the") |
| `BULLET_CAPITALIZATION` | on | Bullets have inconsistent first-letter capitalization |
| `BULLET_PUNCTUATION` | on | Bullet ending punctuation is inconsistent across the deck |
| `BULLET_LENGTH` | on | Bullet is too long |
| `ALL_CAPS` | **off** | Paragraph text is ALL CAPS |

### Structure

| Rule | Status | What it catches |
|---|---|---|
| `TITLE_LENGTH` | on | Title is too long |
| `TITLE_TRAILING_PUNCT` | on | Title ends with `.` `!` or `?` |
| `DUPLICATE_TITLE` | on | Title text is duplicated on another slide |
| `EMPTY_TEXTBOX` | on | Text box has no text content |
| `TITLE_PRESENT` | **off** | Slide has no title element |
| `SLIDE_COUNT` | **off** | Deck has too many slides |

The geometric rules, illustrated with diagrams: [RULES.md](RULES.md)

---

## Embed in your own tooling

`intern-core` is the engine without the CLI - use it to build custom tooling, reporting pipelines, or editor integrations.

```toml
[dependencies]
intern-core = { git = "https://github.com/markusz/intern" }
```

```rust
use intern_core::{reader::read_presentation, rules::{all_rules, Limits}, model::EMU_PER_PX};

let slides = read_presentation("deck.pptx")?;
let limits = Limits { slide_count: 30, ..Limits::default() };
let violations: Vec<_> = all_rules(&limits)
    .iter()
    .flat_map(|r| r.check(&slides, 2 * EMU_PER_PX))
    .collect();
```
