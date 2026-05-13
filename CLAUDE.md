# intern — dev rules

## After every Rust change

Run both before considering work done:

```sh
cargo fmt
cargo clippy -- -D warnings
```
