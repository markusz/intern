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
    rules::{all_rules, Limits},
};

let slides = read_presentation("deck.pptx")?;
let limits = Limits { slide_count: 30, ..Limits::default() };

let violations: Vec<_> = all_rules(&limits)
    .iter()
    .flat_map(|rule| rule.check(&slides, 2 * EMU_PER_PX))
    .collect();

for v in &violations {
    println!("{:?} - {}", v.rule_id, v.message);
}
```

## Key types

- **`reader::read_presentation`** - parses a `.pptx` into `Vec<SlideData>`.
- **`rules::all_rules`** - builds every rule, parameterised by `Limits`.
- **`rules::Rule`** - the trait each rule implements: `check(&slides, threshold)`
  returns a `Vec<Violation>`.
- **`rules::Violation`** - carries the rule id, slide, element, a structured
  `ViolationMessage`, and an optional `Fix`.
- **`writer::apply_fixes`** - applies a slice of `Fix` values to a `.pptx` in place.

Geometry is measured in EMU (English Metric Units); `EMU_PER_PX` converts a pixel
tolerance into the threshold the rules expect.
