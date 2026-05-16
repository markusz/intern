# intern

> Because your real interns have better things to do than align your ppt boxes.

**intern** is a rule-based linter for PowerPoint (`.pptx`) files. Point it at a deck
and it tells you exactly what's wrong — misaligned boxes, inconsistent fonts, sloppy
text, duplicate titles — and can fix geometric issues automatically.

Existing tools are proprietary Office add-ins or AI-powered web uploads. **intern** is
the first open-source, rule-based CLI linter for PowerPoint: configurable, scriptable,
and CI-friendly.

```text
$ intern check quarterly.pptx

  Slide  Rule                  Element   Message
  ─────────────────────────────────────────────────────────────────────
  2      TITLE_Y               Title 2   title is 34.2px lower than on most slides
  3      BODY_FONT_SIZE        Body      body font size 18pt, expected 24pt
  4      TITLE_TRAILING_PUNCT  —         title ends with '.' — remove it
  7      DUPLICATE_TITLE       Title 7   same title as slide 2

4 violation(s)
```

## How it works

1. `intern` unzips the `.pptx` and reads every slide's shapes, text, and images.
2. Each [rule](./rules.md) inspects the deck and reports violations.
3. Geometric violations carry a suggested fix; `intern fix` applies them in place.

Exit code is `0` when the deck is clean and `1` when violations are found — so it
drops straight into a CI pipeline.

Head to [Installation](./installation.md) to get started.
