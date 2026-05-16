# Rules reference

`intern` ships **31 rules** across four categories. Every rule has a stable id you
can pass to `--rules` or `--disable` (or list under `[rules]` in `.intern.toml`).

## Alignment

Geometric checks. All compare positions within a configurable pixel
[threshold](#threshold).

| Rule | What it catches |
|---|---|
| `TITLE_Y` | Title top edge inconsistent across slides |
| `TITLE_X_WIDTH` | Title left edge or width inconsistent across slides |
| `COLUMN_LEFT_EDGE` | Left-column elements have inconsistent left edges |
| `COLUMN_TOP_EDGE` | Left and right columns don't start at the same Y |
| `COLUMN_RIGHT_LEFT_EDGE` | Right-column elements have inconsistent left edges |
| `GRID_H_SPACING` | Horizontal gaps between grid elements are uneven |
| `GRID_V_SPACING` | Vertical gaps between grid elements are uneven |
| `GRID_ROW_TOP` | Elements in the same grid row have misaligned top edges |
| `GRID_COL_LEFT` | Elements in the same grid column have misaligned left edges |
| `ELEMENT_OVERLAP` | Two elements on the same slide have overlapping rects |
| `ELEMENT_OVERFLOW` | Element extends outside the slide bounds |

## Typography

| Rule | What it catches |
|---|---|
| `TITLE_FONT_SIZE` | Title font size differs from the majority |
| `BODY_FONT_SIZE` | Body font size differs from the majority across slides |
| `BODY_FONT_FAMILY` | Body font family differs from the majority across slides |
| `BODY_TEXT_COLOR` | Body text color differs from the majority across slides |
| `FONT_VARIETY` | More distinct font families than the limit |
| `COLOR_VARIETY` | More distinct text colors than the limit |
| `IMAGE_ASPECT_RATIO` | Image aspect ratio differs from the deck majority by >5% |

## Text quality

| Rule | What it catches |
|---|---|
| `DOUBLE_SPACE` | Paragraph contains two or more consecutive spaces |
| `TRAILING_SPACE` | Paragraph has leading or trailing whitespace |
| `ALL_CAPS` | Paragraph text is ALL CAPS |
| `REPEATED_WORD` | Two consecutive identical words ("the the") |
| `BULLET_CAPITALIZATION` | Bullets have inconsistent first-letter capitalization |
| `BULLET_PUNCTUATION` | Bullet ending punctuation is inconsistent across the deck |
| `BULLET_LENGTH` | Bullet exceeds the word limit |

## Structure

| Rule | What it catches |
|---|---|
| `TITLE_PRESENT` | Slide has no title element |
| `TITLE_LENGTH` | Title exceeds the word limit |
| `TITLE_TRAILING_PUNCT` | Title ends with `.` `!` or `?` |
| `DUPLICATE_TITLE` | Title text is duplicated on another slide |
| `EMPTY_ELEMENT` | Body or textbox element has no text content |
| `SLIDE_COUNT` | Deck has more slides than the limit |

## Auto-fixable rules

`intern fix` repairs the rules with an unambiguous correction - the alignment rules
(snap to the peer median), the font-size rules, and `DOUBLE_SPACE` / `TRAILING_SPACE`
(normalise whitespace). The remaining text-quality and structural rules report the
problem but leave the wording to you.

## Threshold

Geometric comparisons use EMU (English Metric Units). The default threshold is
**2 px ≈ 19,050 EMU** at 96 dpi. Override it per run:

```sh
intern check deck.pptx --threshold 5
```

or permanently in `.intern.toml`:

```toml
threshold_px = 5
```

The word- and count-based limits (`TITLE_LENGTH`, `BULLET_LENGTH`, `FONT_VARIETY`,
`COLOR_VARIETY`, `SLIDE_COUNT`) are tuned in each rule's `[rules.<RULE_ID>]` table -
see [Configuration](./configuration.md).
