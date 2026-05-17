# intern-core

The rule engine behind [**intern**](https://github.com/markusz/intern), a linter
for PowerPoint (`.pptx`) files. Use it directly to build custom tooling, reporting
pipelines, or editor integrations - the CLI is just one consumer of this crate.

```rust
use intern_core::{
    model::EMU_PER_PX,
    reader::read_presentation,
    rules::{all_rules, Limits},
};

let slides = read_presentation("deck.pptx")?;
let violations: Vec<_> = all_rules(&Limits::default())
    .iter()
    .flat_map(|rule| rule.check(&slides, 2 * EMU_PER_PX))
    .collect();
```

See the [documentation](https://markusz.github.io/intern/library.html) for the full
API, and the [intern repository](https://github.com/markusz/intern) for the CLI.

## License

MIT
