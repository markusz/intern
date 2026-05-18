# Rules reference

`intern` ships **36 rules** across four categories. Every rule has a stable id you
can pass to `--rules` or `--disable`, or configure under `[rules.<RULE_ID>]` in
`.intern.toml`.

## Alignment

Geometric checks. All compare positions within a configurable pixel
[threshold](#threshold).

### Margin consistency (cross-slide)

Groups are treated as single units - only the group's bounding box is considered,
not its individual children.

| Rule | What it catches | Default threshold |
|---|---|---|
| `LEFT_MARGIN` | Slide's leftmost unit is off the typical left margin | 10 px |
| `RIGHT_MARGIN` | Slide's rightmost unit right edge is off the typical right margin | 10 px |
| `BOTTOM_MARGIN` | Content extends deeper than the typical bottom margin (overflow only) | 10 px |
| `TITLE_MARGIN` | Gap between title and nearest content unit differs from the typical gap | 5 px |

### Proximity alignment (per-slide)

| Rule | What it catches | Default threshold |
|---|---|---|
| `CLOSE_X` | Two units on the same slide have X positions within threshold - likely misaligned | 5 px |
| `CLOSE_Y` | Two units on the same slide have Y positions within threshold - likely misaligned | 5 px |

### Layout detection (default off)

These rules require a layout detector to identify columns and grids. The detector
is unreliable on varied decks; prefer `CLOSE_X` / `CLOSE_Y` instead.

| Rule | What it catches | Default |
|---|---|---|
| `COLUMN_LEFT_EDGE` | Left-column elements have inconsistent left edges | 2 px |
| `COLUMN_TOP_EDGE` | Left and right columns don't start at the same Y | 2 px |
| `COLUMN_RIGHT_LEFT_EDGE` | Right-column elements have inconsistent left edges | 2 px |
| `GRID_H_SPACING` | Horizontal gaps between grid elements are uneven | 2 px |
| `GRID_V_SPACING` | Vertical gaps between grid elements are uneven | 2 px |
| `GRID_ROW_TOP` | Elements in the same grid row have misaligned top edges | 2 px |
| `GRID_COL_LEFT` | Elements in the same grid column have misaligned left edges | 2 px |

### Other alignment

| Rule | What it catches | Default |
|---|---|---|
| `TITLE_Y` | Title top edge inconsistent across slides | 2 px |
| `TITLE_X_WIDTH` | Title left edge or width inconsistent across slides | 2 px |
| `TEXT_ELEMENT_OVERLAP` | Two text-bearing elements on the same slide have overlapping rects | - |
| `ELEMENT_OVERFLOW` | Element extends outside the slide bounds | - |

## Typography

| Rule | What it catches | Default |
|---|---|---|
| `TITLE_FONT_SIZE` | Title font size differs from the majority | - |
| `FONT_SIZE_VARIETY` | Too many distinct body font sizes across the deck | 3 sizes |
| `BODY_FONT_FAMILY` | Body font family differs from the majority across slides | - |
| `BODY_TEXT_COLOR` | Body text color differs from the majority across slides | - |
| `FONT_VARIETY` | Too many distinct font families across the deck | 4 families |
| `COLOR_VARIETY` | Too many distinct text colors across the deck | 6 colors |

## Text quality

| Rule | What it catches | Default |
|---|---|---|
| `DOUBLE_SPACE` | Paragraph contains two or more consecutive spaces | - |
| `LEADING_SPACE` | Paragraph starts with whitespace | - |
| `ALL_CAPS` | Paragraph text is ALL CAPS | - |
| `REPEATED_WORD` | Two consecutive identical words ("the the") | - |
| `BULLET_CAPITALIZATION` | Bullets have inconsistent first-letter capitalization | - |
| `BULLET_PUNCTUATION` | Bullet ending punctuation is inconsistent across the deck | - |
| `BULLET_LENGTH` | Bullet is too long | 20 words |

## Structure

| Rule | What it catches | Default |
|---|---|---|
| `TITLE_PRESENT` | Slide has no title element | - |
| `TITLE_LENGTH` | Title is too long | 10 words |
| `TITLE_TRAILING_PUNCT` | Title ends with `.` `!` or `?` | - |
| `DUPLICATE_TITLE` | Title text is duplicated on another slide | - |
| `EMPTY_TEXTBOX` | Text box has no text content | - |
| `SLIDE_COUNT` | Deck has too many slides | 20 slides |

> Some rules are **off by default** and run only when switched on with
> `enabled = true` in their `[rules.<RULE_ID>]` table (or named in `--rules`):
> `SLIDE_COUNT` (its 20-slide limit is too deck-specific) and the `GRID_*` and
> `COLUMN_*` rules (the layout detector is unreliable). Every other rule runs by
> default.

## Auto-fixable rules

`intern fix` repairs the rules with an unambiguous correction - the alignment rules
(snap to the peer median), the font-size rules, and `DOUBLE_SPACE` / `LEADING_SPACE`
(normalise whitespace). The remaining text-quality and structural rules report the
problem but leave the wording to you.

## Threshold

Geometric comparisons use EMU (English Metric Units). The default threshold for
most alignment rules is **2 px**. The margin and proximity rules have their own
defaults (10 px for `LEFT_MARGIN`, `RIGHT_MARGIN`, `BOTTOM_MARGIN`; 5 px for
`TITLE_MARGIN`, `CLOSE_X`, `CLOSE_Y`). The global `--threshold` flag affects only
rules that use the 2 px default; per-rule overrides always win:

```sh
intern check deck.pptx --threshold 5
```

```toml
# in .intern.toml - overrides the per-rule default for that rule only
[rules.LEFT_MARGIN]
threshold = 15
```

The word- and count-based limits (`TITLE_LENGTH`, `BULLET_LENGTH`, `FONT_VARIETY`,
`COLOR_VARIETY`, `SLIDE_COUNT`) are tuned in each rule's `[rules.<RULE_ID>]` table -
see [Configuration](./configuration.md).
