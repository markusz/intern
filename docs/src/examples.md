# Examples

Recipes for common situations - find your task in the quick reference, jump to the
recipe, copy the command. New to the command line? Start with [First run](#first-run).

## Quick reference

| I want to... | Command |
|---|---|
| Check one deck | `intern deck.pptx` |
| Check every deck in a folder | `intern check slides/` |
| Auto-fix what can be fixed | `intern fix deck.pptx` |
| Preview fixes without saving | `intern fix deck.pptx --dry-run` |
| Check a single slide | `intern check deck.pptx --slide 4` |
| Turn a check off | `intern check deck.pptx --disable ALL_CAPS` |
| Run only certain checks | `intern check deck.pptx --rules TITLE_Y` |
| Be stricter or looser on alignment | `intern check deck.pptx --threshold 1` |
| Get machine-readable output | `intern check deck.pptx --output json` |

## First run

Never used a command-line tool? Three steps.

1. **Open a terminal.** macOS: press `Cmd+Space`, type `Terminal`, `Enter`.
   Windows: open `PowerShell` from the Start menu.
2. **Go to your deck's folder.** If it is on your Desktop:
   ```sh
   cd Desktop
   ```
3. **Check it:**
   ```sh
   intern quarterly.pptx
   ```

`intern` prints one row per problem:

```text
  Slide  Rule                  Element   Message
  ─────────────────────────────────────────────────────────────────────
  2      TITLE_Y               Title 2   title is 34.2px lower than on most slides
  4      TITLE_TRAILING_PUNCT  -         title ends with '.' - remove it

2 violation(s)
```

- **Slide** - where the problem is (slide 1 is the first slide).
- **Rule** - the check that fired; look it up in the [Rules reference](./rules.md).
- **Element** - the shape involved, or `-` for a whole-slide problem.
- **Message** - what is wrong, in plain words.

A clean deck prints `No violations found.`

## Everyday tasks

### Check a deck

```sh
intern deck.pptx
```

`check` is the default action, so the subcommand is optional. Pass a folder to
check every `.pptx` inside it:

```sh
intern check slides/
```

### Fix the easy problems

```sh
intern fix deck.pptx
```

Applies every fixable violation and saves a backup as `deck.pptx.bak`. Alignment,
font-size, and whitespace issues are corrected; wording problems are reported for
you to handle. The [Rules reference](./rules.md) marks which is which.

### Preview before fixing

```sh
intern fix deck.pptx --dry-run
```

Lists what `fix` would change without touching the file.

### Focus on one slide

```sh
intern check deck.pptx --slide 4
```

Slides count from 1. Deck-wide checks (like "all titles line up") need every
slide, so drop `--slide` for the full picture.

### Loosen or tighten alignment

```sh
intern check deck.pptx --threshold 5   # more forgiving
intern check deck.pptx --threshold 1   # pixel-perfect
```

Alignment tolerance in pixels (default `2`). Anything off by less is treated as
fine.

### Turn rules on or off

```sh
intern check deck.pptx --disable ALL_CAPS,SLIDE_COUNT   # skip these
intern check deck.pptx --rules TITLE_Y,TITLE_X_WIDTH    # run only these
```

Rule ids come from the [Rules reference](./rules.md); a typo is rejected with an
error. To make settings permanent, use a config file (below).

## Teams & CI

### A shared team standard

Commit an `.intern.toml` to your project - everyone who runs `intern` there picks
it up:

```toml
threshold_px = 3
disable = ["IMAGE_ASPECT_RATIO"]

[rules.TITLE_LENGTH]
max_words = 8
```

Full reference: [Configuration](./configuration.md).

### Your personal defaults

Settings you want on every deck you lint, regardless of project, go in
`~/.config/intern.toml`. A project's `.intern.toml` wins over it when both exist.

### Gate a CI build

`intern check` exits `0` when clean and `1` on violations - point it at a folder
to gate every deck. GitHub Actions, saved as `.github/workflows/decks.yml`:

```yaml
name: decks
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install intern
        run: |
          curl -L https://github.com/markusz/intern/releases/latest/download/intern-x86_64-unknown-linux-gnu.tar.gz | tar xz
          sudo mv intern /usr/local/bin/
      - run: intern check slides/
```

### Feed results to other tools

```sh
intern check deck.pptx --output json
```

JSON output nests violations under each file:

```json
{
  "files": [
    {
      "path": "deck.pptx",
      "violations": [
        {
          "rule_id": "TITLE_Y",
          "slide": 2,
          "element": "Title 2",
          "message": "title is 34.2px lower than on most slides",
          "severity": "warning"
        }
      ]
    }
  ]
}
```

Pipe it through [`jq`](https://jqlang.github.io/jq/) - for example, list every
slide with a title-alignment problem:

```sh
intern check deck.pptx --output json \
  | jq -r '.files[].violations[] | select(.rule_id == "TITLE_Y") | .slide'
```
