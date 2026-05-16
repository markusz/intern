# Examples

Practical recipes for common situations. Each one is self-contained - copy it,
swap in your own file name, and run it. If you have never used a terminal before,
start with the first recipe; it walks through every step.

The recipes are grouped into **everyday use** (checking and tidying a single deck)
and **teams & automation** (shared standards, CI, scripting).

---

## Everyday use

### Check a deck before you send it

This is the one command most people need.

1. Open a terminal:
   - **macOS** - press `Cmd+Space`, type `Terminal`, press `Enter`.
   - **Windows** - open `PowerShell` from the Start menu.
2. Move to the folder that holds your presentation. If the file is on your
   Desktop:
   ```sh
   cd Desktop
   ```
3. Run the check:
   ```sh
   intern check quarterly.pptx
   ```

`intern` prints one row per problem it finds:

```text
  Slide  Rule                  Element   Message
  ─────────────────────────────────────────────────────────────────────
  2      TITLE_Y               Title 2   title is 34.2px lower than on most slides
  3      BODY_FONT_SIZE        Body      body font size 18pt, expected 24pt
  4      TITLE_TRAILING_PUNCT  -         title ends with '.' - remove it

3 violation(s)
```

- **Slide** - which slide the problem is on (slide 1 is the first slide).
- **Rule** - the short id of the check that fired; look it up in the
  [Rules reference](./rules.md).
- **Element** - the shape involved, or `-` when the problem is the whole slide.
- **Message** - a plain-language description.

If the deck is clean you'll see `No violations found.` instead.

### Fix the easy problems automatically

Many problems - misaligned boxes, inconsistent font sizes, stray double spaces -
can be corrected for you:

```sh
intern fix quarterly.pptx
```

```text
Applied 5 fix(es) to quarterly.pptx  (original backed up to quarterly.pptx.bak)
```

Your original file is always saved next to it as `quarterly.pptx.bak`, so you can
go back if you don't like a change. Wording problems (a too-long title, ALL CAPS
text) are reported but not rewritten - those need a human. See which rules are
auto-fixable in the [Rules reference](./rules.md).

### Preview the fixes first

To see exactly what `intern fix` would change without touching the file:

```sh
intern fix quarterly.pptx --dry-run
```

```text
5 fix(es) would be applied:
  slide 2 'Title 2': set Y → 274.6px
  slide 3 'Body': set font size → 24pt
  ...
```

### Work on a single slide

When you only care about one slide, pass its number (counting from 1):

```sh
intern check quarterly.pptx --slide 4
```

Note that deck-wide checks - such as "all titles line up" - need every slide to
compare against, so run without `--slide` for the full picture.

### Adjust how strict the alignment check is

Alignment is compared with a tolerance, in pixels (default: `2`). A box off by
less than the tolerance is treated as fine.

```sh
intern check quarterly.pptx --threshold 5   # more forgiving
intern check quarterly.pptx --threshold 1   # pixel-perfect
```

### Switch rules off, or run only some

Turn off checks you don't care about:

```sh
intern check quarterly.pptx --disable ALL_CAPS,SLIDE_COUNT
```

Or run *only* the checks you want:

```sh
intern check quarterly.pptx --rules TITLE_Y,TITLE_X_WIDTH
```

Rule ids are comma-separated and come from the [Rules reference](./rules.md). A
misspelled id is rejected with an error, so a typo never silently does nothing.

---

## Teams & automation

### Share one standard across a team

Put an `.intern.toml` file in your project folder and commit it to version
control. Everyone who runs `intern` in that folder picks up the same settings.

```toml
# .intern.toml - house style for our decks
threshold_px = 3

[rules]
disable = ["IMAGE_ASPECT_RATIO"]

[limits]
TITLE_LENGTH = 8
SLIDE_COUNT  = 30
```

See the [Configuration](./configuration.md) chapter for every available setting.

### Set your personal defaults

For settings you want on *every* deck you personally lint - regardless of project -
put the same kind of file at `~/.config/intern.toml`. A project's `.intern.toml`
takes precedence over it when both exist.

### Block messy decks in CI

`intern check` exits with code `0` when a deck is clean and `1` when it finds
violations - exactly what a CI system needs to pass or fail a build.

GitHub Actions example - save as `.github/workflows/decks.yml`:

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
      - name: Lint the deck
        run: intern check slides/quarterly.pptx
```

To fail the build when a deck *could* be auto-tidied (rather than when it is
outright broken), use `fix --check` - it applies nothing and exits `1` if any fix
is available:

```sh
intern fix slides/quarterly.pptx --check
```

### Use the results in another tool

`--output json` emits machine-readable results:

```sh
intern check quarterly.pptx --output json
```

```json
{
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
```

Combine it with [`jq`](https://jqlang.github.io/jq/) to, for example, list only
the slides with a title-alignment problem:

```sh
intern check quarterly.pptx --output json \
  | jq -r '.violations[] | select(.rule_id == "TITLE_Y") | .slide'
```

### Check every deck in a folder

`intern` checks one file at a time; a shell loop covers a whole directory:

```sh
for deck in decks/*.pptx; do
  echo "=== $deck ==="
  intern check "$deck"
done
```

To stop at the first deck with problems (useful in a script):

```sh
for deck in decks/*.pptx; do
  intern check "$deck" || { echo "Problems in $deck"; exit 1; }
done
```
