# intern - dev rules

## After every Rust change

Run both before considering work done:

```sh
cargo fmt
cargo clippy -- -D warnings
```

## Communication

Concise only. No summaries, no "here's what I did", no sign-offs. Say the thing, stop.

Push back when a request has bad ROI - a library limitation that makes something a fragile hack, a feature whose implementation cost outweighs the value, a design that works against the grain of the codebase. Say so plainly and suggest the simpler alternative. Don't be precious about it, but don't silently contort the code either.

## Git

Never run `git commit` or `git push` on your own. The user handles all commits and pushes. You may point out when a good moment to commit has arrived, but never do it unless the user explicitly tells you to in that specific instance - blanket or prior permission does not carry over.

## Docs

`README.md`, `RULES.md`, and the mdBook under `docs/src/` overlap heavily - the same rule tables, examples, and output snippets are duplicated across them. When you change any user-facing doc, grep the others for the same content and update every copy in the same pass. A fix that lands in one file but not its mirror is a regression - sweep, don't patch one spot at a time.

## Code style

- No panicking code unless the error is truly unrecoverable or the invariant is provably safe. In either case add a `// SAFETY:` comment explaining why.
- No AI-slop comments: no `// ────` dividers, no restating what a field name already says, no per-field struct comments. Only comment when the *why* is non-obvious.
- TDD is mandatory for new behaviour, not optional. Write the failing test first, then make it pass. Shipping behaviour with no test is a bug in the process, not just a missing nice-to-have.
- Functions should be ≤60 lines (NASA guideline). Longer is allowed when there is a clear reason, but the default is to split.
- Deeply nested iterators (iter-within-iter, closures-within-closures) are a smell. Extract a named helper instead.
- No magic values unless the meaning is self-evident. Non-obvious constants belong at the top of the file with a one-line comment explaining what they represent and why that value.
- No hidden defaults. `unwrap_or(<value>)`, `unwrap_or_default()`, `unwrap_or_else(...)` are only allowed when a genuine fallback makes sense in context - not as a shortcut to avoid handling `Result` or `Option` properly.
- Code is read by humans. Avoid complex inline math, clever index tricks, and terse expressions that require mental unwrapping. Prefer extra named variables and intermediate results over one-liners that need a comment to explain them. KISS.
- No full-path imports in production code (e.g. `std::collections::HashMap::new()` inline, or `use std::a::b::TheActualStruct` buried in a function body). Bring types into scope with `use` at the top of the file. Exception: test code, or when two crates export the same name and disambiguation is unavoidable.
- No em-dashes anywhere (code, comments, docs, commit messages). Use a plain hyphen `-` instead.
- Prefer enums over booleans for named state. `is_bullet: bool` should be `kind: ParagraphKind` with `Bullet`/`Plain` variants. Enums are self-documenting, exhaustively matched, and easier to extend.
