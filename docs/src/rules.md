# Rules reference

`intern` ships **36 rules** across four categories. Every rule has a stable id you
can pass to `--rules` or `--disable`, or configure under `[rules.<RULE_ID>]` in
`.intern.toml`.

Rules are either **on** (run by default) or **off** (must be opted in via
`enabled = true` in `[rules.<RULE_ID>]` or named in `--rules`). The status is
shown in the tables below.

## Alignment

Geometric checks. All compare positions within a configurable pixel
[threshold](#threshold).

### Margin consistency (cross-slide)

Groups are treated as single units - only the group's bounding box is considered,
not its individual children.

| Rule | Status | What it catches | Default threshold |
|---|---|---|---|
| `LEFT_MARGIN` | on | Slide's leftmost unit is off the typical left margin | 10 px |
| `RIGHT_MARGIN` | on | Slide's rightmost unit right edge is off the typical right margin | 10 px |
| `BOTTOM_MARGIN` | on | Content extends deeper than the typical bottom margin (overflow only) | 10 px |
| `TITLE_MARGIN` | on | Gap between title and nearest content unit differs from the typical gap | 5 px |

### Proximity alignment (per-slide)

| Rule | Status | What it catches | Default threshold |
|---|---|---|---|
| `CLOSE_X` | on | Two units on the same slide have X positions within threshold - likely misaligned | 5 px |
| `CLOSE_Y` | on | Two units on the same slide have Y positions within threshold - likely misaligned | 5 px |

### Other alignment

| Rule | Status | What it catches |
|---|---|---|
| `TITLE_Y` | on | Title top edge inconsistent across slides |
| `TITLE_X_WIDTH` | on | Title left edge or width inconsistent across slides |
| `TEXT_ELEMENT_OVERLAP` | on | Two text-bearing elements on the same slide have overlapping rects |
| `ELEMENT_OVERFLOW` | on | Element extends outside the slide bounds |

## Typography

| Rule | Status | What it catches | Limit |
|---|---|---|---|
| `TITLE_FONT_SIZE` | on | Title font size differs from the majority | - |
| `FONT_SIZE_VARIETY` | on | Too many distinct body font sizes across the deck | 3 sizes |
| `BODY_FONT_FAMILY` | on | Body font family differs from the majority across slides | - |
| `BODY_TEXT_COLOR` | **off** | Body text color differs from the majority across slides | - |
| `FONT_VARIETY` | on | Too many distinct font families across the deck | 4 families |
| `COLOR_VARIETY` | on | Too many distinct text colors across the deck | 6 colors |

> `BODY_TEXT_COLOR` is off by default: color varies intentionally in most decks
> (branded slides, dark backgrounds, highlighted callouts).

## Text quality

| Rule | Status | What it catches | Limit |
|---|---|---|---|
| `DOUBLE_SPACE` | on | Paragraph contains two or more consecutive spaces | - |
| `LEADING_SPACE` | on | Paragraph starts with whitespace | - |
| `ALL_CAPS` | **off** | Paragraph text is ALL CAPS | - |
| `REPEATED_WORD` | on | Two consecutive identical words ("the the") | - |
| `BULLET_CAPITALIZATION` | on | Bullets have inconsistent first-letter capitalization | - |
| `BULLET_PUNCTUATION` | on | Bullet ending punctuation is inconsistent across the deck | - |
| `BULLET_LENGTH` | on | Bullet is too long | 20 words |

> `ALL_CAPS` is off by default: common in corporate decks for KPI labels, callout
> boxes, and section stamps.

## Structure

| Rule | Status | What it catches | Limit |
|---|---|---|---|
| `TITLE_PRESENT` | **off** | Slide has no title element | - |
| `TITLE_LENGTH` | on | Title is too long | 10 words |
| `TITLE_TRAILING_PUNCT` | on | Title ends with `.` `,` `:` or `;` | - |
| `DUPLICATE_TITLE` | on | Title text is duplicated on another slide | - |
| `EMPTY_TEXTBOX` | on | Text box has no text content | - |
| `SLIDE_COUNT` | **off** | Deck has too many slides | 20 slides |

> `TITLE_PRESENT` is off by default: section dividers and full-bleed image slides
> legitimately have no title element.
>
> `SLIDE_COUNT` is off by default: the 20-slide limit is too deck-specific to be
> a useful default.

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
