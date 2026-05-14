![intern](logo.png)

---

> Because your real interns have better things to do than align your ppt boxes.

**intern** is a linter for PowerPoint files. Point it at a `.pptx` and it tells you exactly what's wrong - misaligned boxes, inconsistent fonts, sloppy text, duplicate titles. It can fix geometric issues automatically.

Existing tools are proprietary Office add-ins or AI-powered web uploads. **intern** is the first open-source, rule-based CLI linter for PowerPoint - configurable, scriptable, and CI-friendly.

---

```
$ intern check quarterly.pptx

  slide  element    rule                   message
  ────────────────────────────────────────────────────────────────
  2      Title 2    TITLE_Y                title Y 34.2px off from peers
  3      Body       BODY_FONT_SIZE         body font size 18pt, expected 24pt
  4      —          TITLE_TRAILING_PUNCT   title ends with '.'
  5      Shape 3    ELEMENT_OVERFLOW       element extends outside slide bounds
  6      Body       DOUBLE_SPACE           paragraph contains double spaces
  7      Title 7    DUPLICATE_TITLE        title text also appears on slide 2
  9      Image 2    IMAGE_ASPECT_RATIO     image aspect ratio 1.33 differs from majority 1.78

7 violations
```

---

## Download

**macOS and Linux — Homebrew**
```sh
brew tap markusz/intern
brew install intern
```

**macOS (Apple Silicon) — manual**
```sh
curl -L https://github.com/markusz/intern/releases/latest/download/intern-aarch64-apple-darwin.tar.gz | tar xz
sudo mv intern /usr/local/bin/
```

**macOS (Intel) — manual**
```sh
curl -L https://github.com/markusz/intern/releases/latest/download/intern-x86_64-apple-darwin.tar.gz | tar xz
sudo mv intern /usr/local/bin/
```

**Linux — manual**
```sh
curl -L https://github.com/markusz/intern/releases/latest/download/intern-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv intern /usr/local/bin/
```

**Windows** — download `intern-x86_64-pc-windows-msvc.zip` from the [releases page](https://github.com/markusz/intern/releases/latest), extract, and add to your PATH.

### Build from source

Requires [Rust](https://rustup.rs).

```sh
cargo install --path intern
```

---

## Usage

```sh
intern check deck.pptx    # check and print violations
intern fix deck.pptx      # auto-fix violations in place
```

That's it. No configuration required to get started.

### Options

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Analyze only slide `n` |
| `--output table\|text\|json` | `table` | Output format |
| `--group-by slide\|rule` | `slide` | Group violations |
| `--config <path>` | `.intern.toml` | Config file path |

### Use in CI

```sh
intern check deck.pptx --output json > violations.json
```

Exit code is `0` if clean, `1` if violations found — standard for shell scripting and CI pipelines.

### Config file

Drop an `.intern.toml` in your working directory to make settings permanent.

```toml
threshold_px = 3

[rules]
disable = ["SLIDE_COUNT", "IMAGE_ASPECT_RATIO"]

[output]
format = "table"
group_by = "rule"
```

---

## Rules

31 rules across four categories. Every rule can be disabled with `--disable`.

### Alignment

| Rule | What it catches |
|---|---|
| `TITLE_Y` | Title top-edge inconsistent across slides |
| `TITLE_X_WIDTH` | Title left-edge or width inconsistent across slides |
| `COLUMN_LEFT_EDGE` | Left-column elements have inconsistent left edges |
| `COLUMN_TOP_EDGE` | Left and right columns don't start at the same Y |
| `COLUMN_RIGHT_LEFT_EDGE` | Right-column elements have inconsistent left edges |
| `GRID_H_SPACING` | Horizontal gaps between grid elements are uneven |
| `GRID_V_SPACING` | Vertical gaps between grid elements are uneven |
| `GRID_ROW_TOP` | Elements in the same grid row have misaligned top edges |
| `GRID_COL_LEFT` | Elements in the same grid column have misaligned left edges |
| `ELEMENT_OVERLAP` | Two elements on the same slide have overlapping rects |
| `ELEMENT_OVERFLOW` | Element extends outside the slide bounds |

### Typography

| Rule | What it catches |
|---|---|
| `TITLE_FONT_SIZE` | Title font size differs from the majority |
| `BODY_FONT_SIZE` | Body font size differs from the majority across slides |
| `BODY_FONT_FAMILY` | Body font family differs from the majority across slides |
| `BODY_TEXT_COLOR` | Body text color differs from the majority across slides |
| `FONT_VARIETY` | More than 2 distinct font families across the deck |
| `COLOR_VARIETY` | More than 3 distinct text colors across the deck |
| `IMAGE_ASPECT_RATIO` | Image aspect ratio differs from the deck majority by >5% |

### Text quality

| Rule | What it catches |
|---|---|
| `DOUBLE_SPACE` | Paragraph contains two or more consecutive spaces |
| `TRAILING_SPACE` | Paragraph has leading or trailing whitespace |
| `ALL_CAPS` | Paragraph text is ALL CAPS |
| `REPEATED_WORD` | Two consecutive identical words ("the the") |
| `BULLET_CAPITALIZATION` | Bullets have inconsistent first-letter capitalization |
| `BULLET_PUNCTUATION` | Bullet ending punctuation is inconsistent across the deck |
| `BULLET_LENGTH` | Bullet exceeds 20 words |

### Structure

| Rule | What it catches |
|---|---|
| `TITLE_PRESENT` | Slide has no title element |
| `TITLE_LENGTH` | Title exceeds 10 words |
| `TITLE_TRAILING_PUNCT` | Title ends with `.` `!` or `?` |
| `DUPLICATE_TITLE` | Title text is duplicated on another slide |
| `EMPTY_ELEMENT` | Body or textbox element has no text content |
| `SLIDE_COUNT` | Deck has more than 20 slides |

Full rule documentation with diagrams: [RULES.md](RULES.md)

---

## Embed in your own tooling

`intern-core` is the engine without the CLI — use it to build custom tooling, reporting pipelines, or editor integrations.

```toml
[dependencies]
intern-core = { git = "https://github.com/markusz/intern" }
```

```rust
use intern_core::{reader::read_presentation, rules::all_rules, model::EMU_PER_PX};

let slides = read_presentation("deck.pptx")?;
let violations: Vec<_> = all_rules()
    .iter()
    .flat_map(|r| r.check(&slides, 2 * EMU_PER_PX))
    .collect();
```
