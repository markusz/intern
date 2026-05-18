# intern

> Because your real interns have better things to do than align your ppt boxes.

**intern** is a rule-based linter for PowerPoint (`.pptx`) files. Point it at a deck
and it tells you exactly what's wrong - misaligned boxes, inconsistent fonts, sloppy
text, duplicate titles - and can automatically fix alignment, font-size, and
whitespace problems.

Existing tools are proprietary Office add-ins or AI-powered web uploads. **intern** is
the first open-source, rule-based CLI linter for PowerPoint: configurable, scriptable,
and CI-friendly.

```text
$ intern check quarterly.pptx

  Slide  Rule                  Element   Message
  ─────────────────────────────────────────────────────────────────────
  2      TITLE_Y               Title 2   title is 34.2px lower than on most slides
  3      FONT_SIZE_VARIETY     -         4 distinct body font sizes (limit: 3)
  4      TITLE_TRAILING_PUNCT  -         title ends with '.' - remove it
  7      DUPLICATE_TITLE       Title 7   same title as slide 2

4 violation(s) (4 error, 0 warning)
```

## How it works

1. `intern` unzips the `.pptx` and reads every slide's shapes, text, and images.
2. Each [rule](./rules.md) inspects the deck and reports violations.
3. Most violations carry a suggested fix - alignment, font sizes, and whitespace - and `intern fix` applies them in place.

Exit code is `0` when the deck is clean or has only warnings, and `1` when an
error-severity violation is found - so it drops straight into a CI pipeline.

Head to [Installation](./installation.md) to get started.
