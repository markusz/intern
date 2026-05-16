# Installation

Pick whichever fits your setup — all three give you the same `intern` binary.

## Homebrew — macOS & Linux

```sh
brew install markusz/intern/intern
```

## Prebuilt binary

Download the archive for your platform from the
[latest release](https://github.com/markusz/intern/releases/latest), extract it, and
move `intern` onto your `PATH`.

| Platform | Archive |
|---|---|
| macOS (Apple Silicon) | `intern-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `intern-x86_64-apple-darwin.tar.gz` |
| Linux (x86-64) | `intern-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86-64) | `intern-x86_64-pc-windows-msvc.zip` |

One-liner for macOS/Linux (swap in the archive from the table):

```sh
curl -L https://github.com/markusz/intern/releases/latest/download/intern-aarch64-apple-darwin.tar.gz | tar xz
sudo mv intern /usr/local/bin/
```

## Build from source

Requires [Rust](https://rustup.rs).

```sh
cargo install --path intern
```

## Verify the install

```sh
intern --help
```
