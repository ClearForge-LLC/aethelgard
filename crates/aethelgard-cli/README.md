# aethelgard-cli

The `aethelgard` binary. It is the contract every caller uses (`docs/architecture.md` `RUL-10`): JSON in, exactly one JSON object on one line of stdout, and the exit codes below. Diagnostics go to stderr. `AET-WO-0300` and `AET-WO-1500` inherit these codes; changing one is a breaking change.

## Exit codes

| Code | Meaning | `error.kind` values |
|---|---|---|
| `0` | The specification is valid | none: stdout is the success report |
| `2` | Invalid specification: well-formed JSON of the wrong shape (unknown, missing or duplicate field, wrong type including deep nesting where a value was expected, negative or overflowing number), or a value `validate()` refuses | `SchemaViolation`, or an `IrValidationError` variant name such as `ZeroNominalVoltage` |
| `3` | Unreadable or non-JSON input: the file or stdin cannot be read, is empty, is more than 16 MiB, is not one JSON document (syntax error, trailing data, a byte-order mark) | `UnreadableInput`, `InputTooLarge`, `InvalidJson` |
| `64` | Usage error: unknown subcommand or flag, missing subcommand | `UsageError` |
| `70` | Internal error: the report could not be written to stdout (for example, the reader closed the pipe). Nothing is printed on stdout; the reason goes to stderr | none |

`--help` and `--version` exit `0` and are the only output that is not JSON.

## `validate`

```
aethelgard validate [--file <path>]
```

Reads one `TopologyGraph` from stdin, or from `--file` when given (stdin is then not read), and runs `TopologyGraph::validate()`.

On success, exit `0`:

```json
{"ok":true,"project_name":"sentinel-01","voltage_domains":1,"net_constraints":2}
```

On refusal, exit `2` or `3`:

```json
{"ok":false,"error":{"kind":"ZeroNominalVoltage","id":"VDD_3V3","message":"voltage domain `VDD_3V3` has nominal_millivolts 0"}}
```

`id` is the trimmed offending domain id or net name, or `null` when the refusal has no id to name (an empty id, a whole-document error). `message` is for humans and its wording is not part of the contract; `kind` and `id` are.

Every key is required, including `impedance_ohms`, which is `null` when unconstrained. See `examples/` at the repository root.
