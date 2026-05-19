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

┌───────┬──────────────────────┬─────────┬────────────────┬──────┬───────────────────────────────────┬─────────────────────────────────────────┐
│ Slide ┆ Rule                 ┆ Type    ┆ Position       ┆ Id   ┆ Text                              ┆ Message                                 │
╞═══════╪══════════════════════╪═════════╪════════════════╪══════╪═══════════════════════════════════╪═════════════════════════════════════════╡
│ -     ┆ FONT_SIZE_VARIETY    ┆ -       ┆ -              ┆ -    ┆ -                                 ┆ 8 distinct body font sizes (limit: 3)   │
│       ┆                      ┆         ┆                ┆      ┆                                   ┆                                         │
│ 2     ┆ BULLET_LENGTH        ┆ Body    ┆ (40px, 132px)  ┆ 5    ┆ Our goals for the next quarter... ┆ bullet is 26 words (20-word limit)      │
│ 2     ┆ RIGHT_MARGIN         ┆ Body    ┆ (40px, 132px)  ┆ 5    ┆ Our goals for the next quarter... ┆ right edge at 905.4px (typical 927.0px) │
│       ┆                      ┆         ┆                ┆      ┆                                   ┆                                         │
│ 4     ┆ TITLE_TRAILING_PUNCT ┆ Title   ┆ (28px, 15px)   ┆ 12   ┆ Project Status.                   ┆ title ends with '.' - remove it         │
│       ┆                      ┆         ┆                ┆      ┆                                   ┆                                         │
│ 8     ┆ DUPLICATE_TITLE      ┆ Title   ┆ (28px, 15px)   ┆ 44   ┆ Overview                          ┆ same title as slide 6                   │
└───────┴──────────────────────┴─────────┴────────────────┴──────┴───────────────────────────────────┴─────────────────────────────────────────┘
5 violation(s) (5 error, 0 warning)
```

## How it works

1. `intern` unzips the `.pptx` and reads every slide's shapes, text, and images.
2. Each [rule](./rules.md) inspects the deck and reports violations.
3. Most violations carry a suggested fix - alignment, font sizes, and whitespace - and `intern fix` applies them in place.

Exit code is `0` when the deck is clean or has only warnings, and `1` when an
error-severity violation is found - so it drops straight into a CI pipeline.

Head to [Installation](./installation.md) to get started.
