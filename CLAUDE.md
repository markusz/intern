# intern — dev rules

## After every Rust change

Run both before considering work done:

```sh
cargo fmt
cargo clippy -- -D warnings
```

## Communication

Concise only. No summaries, no "here's what I did", no sign-offs. Say the thing, stop.

## Code style

- No panicking code unless the error is truly unrecoverable or the invariant is provably safe. In either case add a `// SAFETY:` comment explaining why.
- No AI-slop comments: no `// ────` dividers, no restating what a field name already says, no per-field struct comments. Only comment when the *why* is non-obvious.
- Prefer TDD: write the test first and use it as the feedback loop.
