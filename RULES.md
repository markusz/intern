# Rules Reference

Each rule has a stable id you can pass to `--rules`, `--disable`, or configure
under `[rules.<RULE_ID>]` in `.intern.toml`.

Rules are either **on** (run by default) or **off** (must be opted in via
`enabled = true` in `[rules.<RULE_ID>]` or named in `--rules`).

Geometric rules compare positions within a configurable tolerance (the
*threshold*). Most rules use 2 px; the margin and proximity rules have their own
defaults noted below.

---

## Title consistency

These rules compare title placeholder positions and styles across all slides.
Consistent titles are the single strongest signal of a polished deck - they make
the structure scannable and keep the eye anchored while presenting.

### `TITLE_Y`

The top edge of the title should be the same on every slide. Drift between slides
causes the title to jump up and down when flipping through the deck.

- **Default:** on | **Threshold:** 2 px | **Auto-fix:** yes (snaps to median Y)

```
Slide 1          Slide 2          Slide 3
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ ┌─────────┐ │  │ ┌─────────┐ │  │             │
│ │  Title  │ │  │ │  Title  │ │  │  ┌───────┐  │
│ └─────────┘ │  │ └─────────┘ │  │  │ Title │  │  fires: 12 px lower
│             │  │             │  │  └───────┘  │
└─────────────┘  └─────────────┘  └─────────────┘
  Y = 457 200      Y = 457 200      Y = 571 500
```

### `TITLE_X_WIDTH`

The left edge and width of the title should be consistent across slides. An
off-centre or narrow title on one slide breaks the visual grid.

- **Default:** on | **Threshold:** 2 px | **Auto-fix:** yes (snaps X and/or width to median)

```
Slide 1          Slide 2
┌─────────────┐  ┌─────────────┐
│ ┌─────────┐ │  │   ┌───────┐ │
│ │  Title  │ │  │   │ Title │ │  fires: left edge 24 px right
│ └─────────┘ │  │   └───────┘ │
└─────────────┘  └─────────────┘
  X = 457 200      X = 685 800
```

### `TITLE_FONT_SIZE`

All title placeholders should use the same font size. A single slide with a
different title size breaks typographic hierarchy and usually indicates a copy-paste
error or an auto-fit resize.

- **Default:** on | **Threshold:** n/a (exact match against majority) | **Auto-fix:** yes (sets to majority size)

---

## Margins and spacing

These rules compare positions against cross-slide medians. Each slide's layout is
measured independently; the median across all slides defines the expected value.
Groups are treated as a single unit - only the group's bounding box is considered.

### `LEFT_MARGIN`

The leftmost unit on each slide should sit at the same distance from the left edge.
An inconsistent left margin makes slides feel unanchored.

- **Default:** on | **Threshold:** 10 px | **Auto-fix:** no

```
slide 1  │▓░░░░ content ░░░░│
slide 2  │▓░░░░ content ░░░░│
slide 3  │  ▓░░░░ content ░░│  fires: left unit 18 px further right than median
          ↑
       margin
```

### `RIGHT_MARGIN`

The rightmost unit on each slide should end at roughly the same distance from the
right edge. Overlong elements that graze the edge are a common source of this
violation.

- **Default:** on | **Threshold:** 10 px | **Auto-fix:** no

### `BOTTOM_MARGIN`

Content should not extend deeper than the typical bottom margin. This rule only
fires for slides that go too deep, not for slides with less content than usual.

- **Default:** on | **Threshold:** 10 px | **Auto-fix:** no

### `TITLE_MARGIN`

The gap between the bottom of the title and the nearest content unit below it
should be consistent across slides. An unusually large or small gap disrupts the
visual rhythm of the deck.

- **Default:** on | **Threshold:** 5 px | **Auto-fix:** no

### `CLOSE_X`

Two units on the same slide have X positions that are close but not equal. This
almost always means they were meant to be aligned but weren't. True intentional
offsets are typically much larger than the threshold.

- **Default:** on | **Threshold:** 5 px | **Auto-fix:** no

```
┌──────────────────────────────┐
│  ┌──────────┐                │  X = 457 200
│    ┌──────────┐              │  X = 471 450   fires: 3 px off
└──────────────────────────────┘
```

### `CLOSE_Y`

Same as `CLOSE_X` but for vertical position. Two elements whose top edges are a
few pixels apart almost certainly belong to the same horizontal band and should
share a Y value.

- **Default:** on | **Threshold:** 5 px | **Auto-fix:** no

---

## Bounds

### `ELEMENT_OVERFLOW`

An element's rect extends outside the slide boundary. Content outside the slide
area is invisible in presentation mode and will be clipped or cropped on export.

- **Default:** on | **Auto-fix:** no

```
slide edge
     │
     │  ┌───────────────────┐
     │  │ content           │─────────┐  fires: right edge past slide
     │  └───────────────────┘─────────┘
     │
```

### `TEXT_ELEMENT_OVERLAP`

Two text-bearing elements on the same slide have overlapping bounding rects.
Overlapping text boxes almost always indicate a layout error - the content
is unreadable or will collide when rendered.

- **Default:** on | **Auto-fix:** no

```
┌──────────────┐
│  Text A      │
│         ┌───┼──────────┐
└─────────┼───┘           │  fires
          │  Text B       │
          └───────────────┘
```

---

## Typography

These rules compare font properties across the deck. They catch the visual noise
that builds up when slides are assembled from multiple sources or edited over time.

### `FONT_SIZE_VARIETY`

Counts distinct font sizes used in body text across the deck. Too many sizes make
the deck feel undesigned; a small set (typically 2-3) is a sign of intentional
hierarchy.

- **Default:** on | **Limit:** 3 sizes | **Auto-fix:** no

### `BODY_FONT_FAMILY`

Each body element should use the same font family as the majority of the deck.
An outlier font family stands out as a copy-paste artefact.

- **Default:** on | **Threshold:** majority match | **Auto-fix:** no

### `BODY_TEXT_COLOR`

Each body element should use the same text color as the majority of the deck.
Off by default because intentional color variation is common (branded slides,
dark-background sections, highlighted callouts).

- **Default:** **off** | **Threshold:** majority match | **Auto-fix:** no

### `FONT_VARIETY`

The deck uses too many distinct font families in total. A well-designed deck
typically uses one or two: a display face for titles and a text face for body.

- **Default:** on | **Limit:** 4 families | **Auto-fix:** no

### `COLOR_VARIETY`

The deck uses too many distinct text colors. Accumulated color variation from
editing and copy-pasting across decks is one of the most common polish issues.

- **Default:** on | **Limit:** 6 colors | **Auto-fix:** no

---

## Text quality

Rules that catch mechanical writing errors independent of content.

### `DOUBLE_SPACE`

A paragraph contains two or more consecutive spaces. Almost always a typo or
artifact of copy-paste from a word processor.

- **Default:** on | **Auto-fix:** yes (collapses to single space)

### `LEADING_SPACE`

A paragraph starts with one or more spaces. Causes visual indentation that differs
from the placeholder's intended margin.

- **Default:** on | **Auto-fix:** yes (trims leading whitespace)

### `REPEATED_WORD`

The same word appears twice in a row ("the the", "and and"). A reliable indicator
of a copy-paste or editing error; virtually never intentional.

- **Default:** on | **Auto-fix:** no

### `ALL_CAPS`

A paragraph is written in ALL CAPS. Off by default because all-caps is common in
corporate decks for KPI labels, callout boxes, and section stamps. Enable it if
your style guide requires sentence case throughout.

- **Default:** **off** | **Auto-fix:** no

---

## Bullets

Rules for bullet point paragraphs in body placeholders and text boxes.

### `BULLET_CAPITALIZATION`

All bullets within an element should start with the same case - either all
uppercase first letter or all lowercase. Inconsistent capitalization signals
an element assembled from multiple sources.

- **Default:** on | **Auto-fix:** no

### `BULLET_PUNCTUATION`

Bullets across the deck should consistently either end with punctuation or not.
A deck where some bullets end with periods and others don't looks inconsistent.
The majority rule across the whole deck sets the expected style.

- **Default:** on | **Auto-fix:** no

### `BULLET_LENGTH`

A bullet point is too long. Long bullets are a sign that slide content has not
been condensed - walls of text defeat the purpose of a slide.

- **Default:** on | **Limit:** 20 words | **Auto-fix:** no

---

## Structure

Rules about slide and deck structure rather than content.

### `TITLE_PRESENT`

Every slide should have a title placeholder element. Off by default because
section dividers and full-bleed image slides legitimately have no title. Enable it
and use `intern: disable` on slides that intentionally omit a title.

- **Default:** **off** | **Auto-fix:** no

### `DUPLICATE_TITLE`

Two slides share the same title text. Duplicate titles make a deck hard to
navigate and are usually a sign of an unfinished slide that was duplicated and
never updated.

- **Default:** on | **Auto-fix:** no

### `TITLE_LENGTH`

The title is too long. Short titles are scannable at a glance; long titles compete
with the content and often mean the slide is trying to do too much.

- **Default:** on | **Limit:** 10 words | **Auto-fix:** no

### `TITLE_TRAILING_PUNCT`

The title ends with sentence-ending punctuation (`.`, `,`, `:`, `;`). Slide titles
are labels, not sentences - trailing punctuation is a holdover from prose writing.
Question marks and exclamation marks are allowed.

- **Default:** on | **Auto-fix:** no

### `EMPTY_TEXTBOX`

A text box element has no text content. Empty text boxes are invisible placeholders
that clutter the element list and can confuse screen readers.

- **Default:** on | **Auto-fix:** no

### `SLIDE_COUNT`

The deck has more slides than the limit. Off by default because the right limit is
too deck-specific to have a useful global default.

- **Default:** **off** | **Limit:** 20 slides | **Auto-fix:** no

---

## Auto-fix summary

`intern fix` applies unambiguous corrections. Rules not listed here require human
judgement to fix.

| Rule | What the fix does |
|---|---|
| `TITLE_Y` | Snaps title Y to the cross-slide median |
| `TITLE_X_WIDTH` | Snaps title X and/or width to the cross-slide median |
| `TITLE_FONT_SIZE` | Sets title font size to the majority value |
| `DOUBLE_SPACE` | Collapses consecutive spaces to one |
| `LEADING_SPACE` | Trims leading whitespace from paragraphs |

---

## Threshold and limits

Geometric thresholds are in pixels at 96 dpi (1 px = 9 525 EMU). The global
`--threshold` flag sets the fallback for rules that use the 2 px default.
Per-rule overrides always win:

```sh
intern check deck.pptx --threshold 5
```

```toml
[rules.LEFT_MARGIN]
threshold_px = 15
```

Count-based limits are tuned per rule in `.intern.toml`:

```toml
[rules.BULLET_LENGTH]
limit = 15

[rules.FONT_VARIETY]
limit = 2
```
