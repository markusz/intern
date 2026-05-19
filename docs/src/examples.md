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
┌───────┬──────────────────────┬─────────┬────────────────┬──────┬───────────────────────────────────┬─────────────────────────────────────────┐
│ Slide ┆ Rule                 ┆ Type    ┆ Position       ┆ Id   ┆ Text                              ┆ Message                                 │
╞═══════╪══════════════════════╪═════════╪════════════════╪══════╪═══════════════════════════════════╪═════════════════════════════════════════╡
│ 2     ┆ BULLET_LENGTH        ┆ Body    ┆ (40px, 132px)  ┆ 5    ┆ Our goals for the next quarter... ┆ bullet is 26 words (20-word limit)      │
│ 2     ┆ RIGHT_MARGIN         ┆ Body    ┆ (40px, 132px)  ┆ 5    ┆ Our goals for the next quarter... ┆ right edge at 905.4px (typical 927.0px) │
│       ┆                      ┆         ┆                ┆      ┆                                   ┆                                         │
│ 4     ┆ TITLE_TRAILING_PUNCT ┆ Title   ┆ (28px, 15px)   ┆ 12   ┆ Project Status.                   ┆ title ends with '.' - remove it         │
└───────┴──────────────────────┴─────────┴────────────────┴──────┴───────────────────────────────────┴─────────────────────────────────────────┘
3 violation(s) (3 error, 0 warning)
```

- **Slide** - where the problem is (slide 1 is the first slide).
- **Rule** - the check that fired; look it up in the [Rules reference](./rules.md).
- **Type / Position / Id / Text** - the offending element: its kind, top-left corner, shape id, and a short text excerpt. All `-` for a whole-slide problem.
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

### Skip a slide that is intentionally different

Section dividers, the title slide, a deliberately unique layout - exclude a slide
by adding a line to its **speaker notes**:

```text
intern: disable                        # skip every rule on this slide
intern: disable TITLE_Y, DUPLICATE_TITLE  # skip only these rules
```

The slide is dropped before those rules run, so it skews no baselines either.

### Loosen or tighten alignment

```sh
intern check deck.pptx --threshold 5   # more forgiving
intern check deck.pptx --threshold 1   # pixel-perfect
```

Alignment tolerance in pixels (default `2`). Anything off by less is treated as
fine.

### Turn rules on or off

```sh
intern check deck.pptx --disable ALL_CAPS,TITLE_TRAILING_PUNCT  # skip these
intern check deck.pptx --rules TITLE_Y,TITLE_X_WIDTH            # run only these
```

Rule ids come from the [Rules reference](./rules.md); a typo is rejected with an
error. To make settings permanent, use a config file (below).

## Teams & CI

### A shared team standard

Commit an `.intern.toml` to your project - everyone who runs `intern` there picks
it up:

```toml
threshold_px = 3
disable = ["ALL_CAPS"]

[rules.TITLE_LENGTH]
max_words = 8
```

Full reference: [Configuration](./configuration.md).

### Make a rule advisory instead of blocking

Every rule is an *error* by default and fails CI. To keep a rule's findings visible
without failing the build, set its severity to `warning`:

```toml
[rules.ALL_CAPS]
severity = "warning"
```

`intern check` exits non-zero only when an error-severity violation exists.

### Your personal defaults

Settings you want on every deck you lint, regardless of project, go in
`~/.config/intern.toml`. A project's `.intern.toml` wins over it when both exist.

### Gate a CI build

`intern check` exits `0` when clean (or only warnings) and `1` on an error-severity
violation - point it at a folder to gate every deck. GitHub Actions, saved as
`.github/workflows/decks.yml`:

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
          "element_type": "Title",
          "element_position": "(28px, 47px)",
          "element_id": 12,
          "excerpt": "Project Status",
          "message": "title 34.2px lower than most slides",
          "severity": "error"
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
