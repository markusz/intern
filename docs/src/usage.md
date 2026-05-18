# Command-line usage

`intern` has three subcommands: `check` reports violations, `fix` repairs the ones
it can, and `ignore` writes a suppression directive into a slide's speaker notes.
`check` is the default action, so these two are equivalent:

```sh
intern deck.pptx
intern check deck.pptx
```

Every `check` and `fix` command accepts multiple files and directories - a directory
is expanded to the `.pptx` files directly inside it:

```sh
intern check slides/ extra.pptx
```

No configuration is required to get started.

## `intern check`

Reads each presentation and prints its violations. Exits `0` when every deck is
clean or has only warnings, and `1` when an error-severity violation is found (see
[severity](./configuration.md#per-rule-tables)).

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Analyze only slide `n` (1-based) |
| `--output table\|text\|json` | `table` | Output format |
| `--group-by slide\|rule` | `slide` | Group violations |
| `--config <path>` | auto | Load settings from a specific file ([Configuration](./configuration.md)) |

An unknown rule id passed to `--rules` or `--disable` is rejected with an error
rather than silently ignored.

## `intern fix`

Applies the suggested fix for every fixable violation, writing each file in place.
The original is backed up next to it as `<file>.bak`.

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Fix only slide `n` (1-based) |
| `--dry-run` | off | Print what would change without writing |

Not every rule is auto-fixable. Alignment, font-size, and whitespace rules carry a
concrete fix; the remaining text-quality and structural rules report the problem but
leave the change to you. See the [rules reference](./rules.md).

## `intern ignore`

Writes a suppression directive into a slide's speaker notes without opening
PowerPoint. Backs the original up to `<file>.bak` before writing.

```sh
intern ignore <file> -s <slide> -r <rule> [-e <element>]
```

| Flag | Description |
|---|---|
| `-s, --slide <n>` | Slide number (1-based, matches the **Slide** column in `check` output) |
| `-r, --rule <ID>` | Rule ID to suppress (e.g. `EMPTY_TEXTBOX`) |
| `-e, --element <id>` | Element id (**Id** column) - omit to suppress the rule for the whole slide |

Suppress `EMPTY_TEXTBOX` for element 42 on slide 3:

```sh
intern ignore deck.pptx -s 3 -e 42 -r EMPTY_TEXTBOX
# appends: intern: disable(42) EMPTY_TEXTBOX
```

Suppress `TITLE_Y` for the whole of slide 1 (a title slide with a non-standard
layout):

```sh
intern ignore deck.pptx -s 1 -r TITLE_Y
# appends: intern: disable TITLE_Y
```

The rule ID is validated; an unknown ID is rejected with an error. If the slide
has no speaker-notes part, `intern ignore` creates one automatically.


## Skipping checks

### Skip an entire slide

To exclude a slide from **every** check - a title slide, a section divider, a
deliberately different layout - add this line to its **speaker notes**:

```text
intern: disable
```

To exclude it from only **specific** rules, list their ids:

```text
intern: disable TITLE_Y, GRID_ROW_TOP
```

Either way the slide is dropped before those rules run, so it affects neither the
report nor their baselines (such as the median title position the other slides are
compared against).

### Skip a single element

The easiest way is `intern ignore deck.pptx -s <slide> -r <rule> -e <element>` -
it writes the directive automatically. To add it by hand, put the element id
(shown as **Id** in the table output) in parentheses:

```text
intern: disable(42) EMPTY_TEXTBOX
```

Omit the rule list to suppress every rule for that element:

```text
intern: disable(42)
```

Both syntaxes can appear on the same line as a slide-level directive:

```text
intern: disable TITLE_Y
intern: disable(42) EMPTY_TEXTBOX
```

Multiple lines in the same slide's speaker notes are all processed independently.

## Use in CI

`intern check` exits `0` when every deck is clean and `1` when it finds an
error-severity violation - wire that exit code straight into a pipeline. Point it at
a directory to gate a whole folder of decks:

```sh
intern check slides/
```

JSON output is available for further processing:

```sh
intern check deck.pptx --output json > violations.json
```
