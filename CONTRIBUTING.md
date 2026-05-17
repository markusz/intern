# Contributing

Thanks for your interest in improving **intern**.

## Project layout

A Cargo workspace with two crates:

- `intern-core` - the linting engine: the PPTX reader, the rules, the fixer.
- `intern` - the command-line interface.

The documentation site lives in `docs/` (an [mdBook](https://rust-lang.github.io/mdBook/)).

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
```

## Before you open a pull request

CI runs these three, and they must pass:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

House rules:

- **Tests are not optional.** New behaviour ships with a test that fails before
  your change and passes after it.
- Keep `cargo clippy -- -D warnings` clean - no `#[allow]` papering over a lint
  without a reason in a comment.
- Run `cargo fmt` before committing.

## Adding a rule

Rules live in `intern-core/src/rules/`. Each implements the `Rule` trait and is
registered in `all_rules` (`intern-core/src/rules.rs`). When you add one, list it
in the README rules table and `docs/src/rules.md` too.

## Reporting bugs and proposing features

Open an issue - there are templates for both. For a bug, a small `.pptx` that
reproduces it is worth a thousand words.
