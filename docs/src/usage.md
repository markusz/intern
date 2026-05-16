# Command-line usage

`intern` has two subcommands: `check` reports violations, `fix` repairs the ones it
can.

```sh
intern check deck.pptx    # check and print violations
intern fix deck.pptx      # auto-fix violations in place
```

No configuration is required to get started.

## `intern check`

Reads a presentation and prints every violation. Exits `0` when clean, `1` when
violations are found.

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Analyze only slide `n` (1-based) |
| `--output table\|text\|json` | `table` | Output format |
| `--group-by slide\|rule` | `slide` | Group violations |
| `--config <path>` | `.intern.toml` | Config file path |

An unknown rule id passed to `--rules` or `--disable` is rejected with an error
rather than silently ignored.

## `intern fix`

Applies the suggested fix for every fixable violation, writing the file in place.
The original is backed up to `<file>.bak`.

| Flag | Default | Description |
|---|---|---|
| `--rules RULE_ID,...` | all | Run only the specified rules |
| `--disable RULE_ID,...` | none | Skip specific rules |
| `--threshold <px>` | `2` | Alignment tolerance in pixels |
| `--slide <n>` | all | Fix only slide `n` (1-based) |
| `--dry-run` | off | Print what would change without writing |
| `--check` | off | Exit `1` if any fix would be applied (CI gate) |

Not every rule is auto-fixable — text-quality and structural rules report a problem
but leave the wording to you. See the [rules reference](./rules.md).

## Use in CI

```sh
intern check deck.pptx --output json > violations.json
```

The JSON output and the `0`/`1` exit code make `intern` straightforward to wire into
a pipeline. To fail a build when a deck *could* be tidied up:

```sh
intern fix deck.pptx --check
```
