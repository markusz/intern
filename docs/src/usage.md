# Command-line usage

`intern` checks and fixes presentations. `check` reports violations, `fix` repairs
the ones it can. `check` is the default action, so these two are equivalent:

```sh
intern deck.pptx
intern check deck.pptx
```

Every command accepts multiple files and directories - a directory is expanded to
the `.pptx` files directly inside it:

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

To suppress a rule for one element without affecting the rest of the slide, add
the element id (the `<p:cNvPr id="...">` attribute - shown as **Id** in the table
output) in parentheses:

```text
intern: disable(42) EMPTY_ELEMENT
```

Omit the rule list to suppress every rule for that element:

```text
intern: disable(42)
```

Both syntaxes can appear on the same line as a slide-level directive:

```text
intern: disable TITLE_Y
intern: disable(42) EMPTY_ELEMENT
```

Multiple lines in the same slide's speaker notes are all processed independently.

### Finding IDs

Every finding carries a stable 8-character hex identifier (`FID`) derived from
the rule id, slide number, and element id. The id is shown in the **FID** column
of the table output, inline in text output (`[RULE_ID:fid]`), and in the
`finding_id` field of JSON output. The id is stable across runs as long as the
element is not moved to a different slide or replaced with a different shape.

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
