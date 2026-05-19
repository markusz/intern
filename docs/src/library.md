# Using the library

`intern-core` is the engine without the CLI - use it to build custom tooling,
reporting pipelines, or editor integrations.

```toml
[dependencies]
intern-core = { git = "https://github.com/markusz/intern" }
```

## Checking a presentation

```rust
use intern_core::{
    model::EMU_PER_PX,
    reader::read_presentation,
    rules::{all_rules, Limits, RuleContext},
};

let pres = read_presentation("deck.pptx")?;
let ctx = RuleContext {
    threshold: 2 * EMU_PER_PX,
    slide_width: pres.slide_width,
    slide_height: pres.slide_height,
};
let limits = Limits { slide_count: 30, ..Limits::default() };

let violations: Vec<_> = all_rules(&limits)
    .iter()
    .flat_map(|rule| rule.check(&pres.slides, &ctx))
    .collect();

for v in &violations {
    println!("{:?} - {}", v.rule_id, v.message);
}
```

## Key types

- **`reader::read_presentation`** - parses a `.pptx` into a `Presentation` (its
  `slides` plus the deck's slide dimensions).
- **`rules::all_rules`** - builds every rule, parameterised by `Limits`.
- **`rules::Rule`** - the trait each rule implements: `check(slides, ctx)` takes a
  `&[SlideData]` and a `&RuleContext` and returns a `Vec<Violation>`.
- **`rules::Violation`** - carries the rule id, slide, element, a structured
  `ViolationMessage`, and an optional `Fix`.
- **`writer::apply_fixes`** - applies a slice of `Fix` values to a `.pptx` in place.

Geometry is measured in EMU (English Metric Units); `EMU_PER_PX` converts a pixel
tolerance into the threshold the rules expect.
