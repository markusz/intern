# Rules Reference

Each rule gets a unique ID you can pass to `--rules` or `--disable`. All geometric checks use a configurable pixel tolerance (default: 2 px, set via `--threshold` or `threshold_px` in `.intern.toml`).

---

## Title rules

These rules run across all slides and compare title placeholder geometry.

### `TITLE_Y` - title top-edge drift

The top edge of the title should be the same on every slide.

```
Slide 1          Slide 2          Slide 3
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ ┌─────────┐ │  │  ┌───────┐  │  │ ┌─────────┐ │
│ │  Title  │ │  │  │ Title │  │  │ │  Title  │ │  ← Y inconsistent
│ └─────────┘ │  │  └───────┘  │  │ └─────────┘ │
│             │  │             │  │             │
└─────────────┘  └─────────────┘  └─────────────┘
  Y = 457 200      Y = 457 200      Y = 571 500   ← fires
```

### `TITLE_X_WIDTH` - title left-edge or width drift

The left edge and width of the title should be consistent across slides.

```
Slide 1          Slide 2
┌─────────────┐  ┌─────────────┐
│ ┌─────────┐ │  │   ┌───────┐ │
│ │  Title  │ │  │   │ Title │ │  ← left edge shifted right
│ └─────────┘ │  │   └───────┘ │
└─────────────┘  └─────────────┘
  X = 457 200      X = 685 800     ← fires
```

### `TITLE_FONT_SIZE` - font size outlier

One slide's title uses a font size that differs from the majority of slides.

---

## Two-column layout rules

Detected when shapes are clearly split into a left and right column (each column centroid on its respective half of the slide).

```
┌───────────────────────────────────────┐
│  ┌──────────────┐   ┌──────────────┐  │
│  │   Left col   │   │   Right col  │  │
│  ├──────────────┤   ├──────────────┤  │
│  │   Left col   │   │   Right col  │  │
│  ├──────────────┤   ├──────────────┤  │
│  │   Left col   │   │   Right col  │  │
│  └──────────────┘   └──────────────┘  │
└───────────────────────────────────────┘
```

### `COLUMN_LEFT_EDGE` - left-column X drift

All shapes in the left column should share the same left edge.

```
┌────────────────────────────────────┐
│  ┌──────────────┐                  │
│  │  Left col    │  ← X = 457 200   │
│   ┌──────────────┐                 │
│   │  Left col    │  ← X = 552 450  │  fires: 10 px off
│  ┌──────────────┐                  │
│  │  Left col    │  ← X = 457 200   │
└────────────────────────────────────┘
```

### `COLUMN_RIGHT_LEFT_EDGE` - right-column X drift

Same as above, for the right column.

### `COLUMN_TOP_EDGE` - column top misalignment

The topmost shape in the left column and the topmost shape in the right column should start at the same Y.

```
┌────────────────────────────────────────────┐
│  ┌────────────┐                            │
│  │  Left[0]   │  ← Y = 800 000             │
│              ┌────────────┐                │
│              │  Right[0]  │  ← Y = 950 000 │  fires
└────────────────────────────────────────────┘
```

---

## Grid layout rules

Detected when two-column layout is not applicable and shapes form a ≥2×2 matrix (at least ⅔ of cells filled).

```
┌───────────────────────────────────────┐
│   ┌────────┐  ┌────────┐  ┌────────┐  │
│   │  A1    │  │  A2    │  │  A3    │  │
│   └────────┘  └────────┘  └────────┘  │
│   ┌────────┐  ┌────────┐  ┌────────┐  │
│   │  B1    │  │  B2    │  │  B3    │  │
│   └────────┘  └────────┘  └────────┘  │
└───────────────────────────────────────┘
```

### `GRID_ROW_TOP` - row top-edge misalignment

Every shape in a row should share the same top edge (within threshold).

```
Row 0:  ┌────────┐  ┌────────┐  ┌────────┐
        │  A1    │  │  A2    │   │  A3   │  ← A3 shifted down
        │ Y=800k │  │ Y=800k │   │Y=900k │     fires
        └────────┘  └────────┘  └────────┘
```

### `GRID_COL_LEFT` - column left-edge misalignment

Every shape in a column should share the same left edge.

```
Col 1:  ┌────────┐     ┌────────┐
        │  A2    │      │  B2   │  ← B2 shifted right
        │X=3.7M  │      │X=3.8M │     fires
        └────────┘     └────────┘
```

### `GRID_H_SPACING` - uneven horizontal gaps

The horizontal gap between adjacent columns should be consistent across all rows.

```
Row 0:  [A1]──500k──[A2]   ← gap 500 000 EMU (~52 px)
Row 1:  [B1]──800k──[B2]   ← gap 800 000 EMU (~84 px)   fires
```

### `GRID_V_SPACING` - uneven vertical gaps

The vertical gap between adjacent rows should be consistent across all columns.

```
Col 0:  [A1]           Col 1:  [A2]
         │ 600k gap              │ 600k gap
        [B1]                   [B2]
         │ 600k gap              │ 900k gap   ← fires
        [C1]                   [C2]
```

---

## Threshold

All geometric comparisons use EMU (English Metric Units). The default threshold is **2 px = 19 050 EMU** (at 96 dpi). Override with:

```sh
intern deck.pptx --threshold 5   # 5 px tolerance
```

or in `.intern.toml`:

```toml
threshold_px = 5
```
