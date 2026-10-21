# aethelgard-cli

The `aethelgard` binary. It is the contract every caller uses (`docs/architecture.md` `RUL-10`): exactly one JSON object on one line of stdout, and the exit codes below. Diagnostics go to stderr. `AET-WO-1500` inherits these codes; changing one is a breaking change.

## Exit codes

| Code | Meaning | `error.kind` values |
|---|---|---|
| `0` | `validate`: the specification is valid. A tool subcommand: the tool ran and its verdict is pass | none: stdout is the success report or the verdict |
| `1` | A tool subcommand: the tool ran and its verdict is fail (violations found, the proof failed, the crate did not compile, the harness does not exist, the platform did not load). The input is at fault; the tool worked | none: stdout is the verdict |
| `2` | Invalid specification: well-formed JSON of the wrong shape (unknown, missing or duplicate field, wrong type including deep nesting where a value was expected, negative or overflowing number), or a value `validate()` refuses | `SchemaViolation`, or an `IrValidationError` variant name such as `ZeroNominalVoltage` |
| `3` | Unreadable input. `validate`: the file or stdin cannot be read, is empty, is more than 16 MiB, or is not one JSON document (syntax error, trailing data, a byte-order mark). A tool subcommand: an input path that does not exist, cannot be read, or is not a regular file (not a directory, for `--crate`); a crate holding a symlink, a special file or a non-UTF-8 name; a Renode built-in path outside Renode's own tree | `UnreadableInput`, `InputTooLarge`, `InvalidJson` |
| `4` | The tool cannot be used: it is not on `PATH`, is not executable, will not report its version, or lacks an environment variable it needs; or the sandbox is unavailable. Never a verdict (`INV-06`) | `ToolMissing` (`id`: the tool), `ToolUnusable` (`id`: the tool, or the missing variable), `SandboxUnavailable` (`id`: `landlock`) |
| `5` | The tool ran but its output cannot be trusted as a verdict: killed by a signal, an exit code outside the ones measured for it, a missing, unparseable or oversized report or output, or two witnesses that disagree. Never read as pass or fail | `ToolError` (`id`: the tool) |
| `64` | Usage error: unknown subcommand or flag, missing subcommand or required flag | `UsageError` |
| `70` | Internal error: the report could not be written to stdout (for example, the reader closed the pipe). Nothing is printed on stdout; the reason goes to stderr | none |

A stdout closed by the caller (`>&-`) is not an exit-`70` case: the Rust runtime reopens a closed standard descriptor on `/dev/null` before `main` runs, so it behaves exactly like `>/dev/null`, and the exit code still reports the verdict.

`--help` and `--version` exit `0` and are the only output that is not JSON. `message` is for humans and its wording is not part of the contract; `ok`, `kind`, `id` and the verdict fields are.

## `validate`

```
aethelgard validate [--file <path>]
```

Reads one `TopologyGraph` from stdin, or from `--file` when given (stdin is then not read; `-` is a file name, not stdin), and runs `TopologyGraph::validate()`. The specification and each domain and net must be JSON objects.

On success, exit `0`:

```json
{"ok":true,"project_name":"sentinel-01","voltage_domains":1,"net_constraints":2}
```

On refusal, exit `2` or `3`:

```json
{"ok":false,"error":{"kind":"ZeroNominalVoltage","id":"VDD_3V3","message":"voltage domain `VDD_3V3` has nominal_millivolts 0"}}
```

`id` is the trimmed offending domain id or net name, or `null` when the refusal has no id to name (an empty id, a whole-document error).

Every key is required, including `impedance_ohms`, which is `null` when unconstrained. See `examples/` at the repository root.

## The tool subcommands

```
aethelgard check drc --board <file.kicad_pcb>
aethelgard check erc --schematic <file.kicad_sch>
aethelgard prove --crate <dir> --harness <name>
aethelgard simulate --platform <file.repl | @path-inside-renode>
```

Each runs one external tool on one input and prints one verdict. They are **Linux-only**, because the tools and the sandbox are. The published tools image (`container/IMAGE`) has all four tools; see [Running it](#running-it).

### What every call does, in order

1. **Checks the input path** (exit `3` if refused). `--board`, `--schematic` and a `--platform` file must be regular files after following symlinks; `--crate` must be a directory. A `--platform` starting with `@` names a file inside Renode's own tree (the directory holding the real `renode` binary), as Renode's own `@` syntax does.
2. **Finds the tool** by searching the caller's `PATH` for `kicad-cli`, `renode`, or `cargo` and `cargo-kani` (exit `4`, `ToolMissing`, naming the first one absent). `prove` also needs `KANI_HOME` and `RUSTUP_HOME` set to existing directories, and `PATH` (exit `4`, `ToolUnusable`, naming the variable).
3. **Copies the input** into a fresh private temporary directory and hashes the copy. The tool judges the copy, never the original, so the hash in the verdict is of exactly the bytes judged (`INV-03`), and nothing is written beside the input.
4. **Asks the tool its version**, then **runs it**, each time in the sandbox below. A tool that will not report its version is exit `4`, `ToolUnusable`.
5. **Reads two witnesses** (below), prints the verdict, and removes the temporary directory.

### The sandbox

Every tool process runs with:

- an argument array, never a shell, and every input given as an absolute path inside the temporary directory, so no file name can be read as a flag;
- an **empty environment** plus `HOME` and `TMPDIR` set to the temporary directory; `prove` alone also passes `KANI_HOME`, `RUSTUP_HOME` and `PATH` through from the caller, by name;
- stdin from `/dev/null`, and stdout and stderr captured up to 16 MiB each (more is exit `5`);
- **Landlock**: read-only access to the system directories, the tool's own installation and the input copy, read-write access only to its temporary directory, and no other filesystem access. Renode also gets read-only `/proc` and `/dev/urandom`; Kani also gets read-write `/dev/null`. Landlock's filesystem rights through ABI 3 are **required**: if the kernel cannot enforce them, the call refuses with exit `4`, `SandboxUnavailable`, `id` `landlock`. There is no flag or variable that runs a tool unconfined. Where the kernel has Landlock's network rules (ABI 4 and later), TCP bind and connect are denied as well; the verdict's `sandbox` field says which;
- a **reaper**: the CLI starts a copy of itself, in a new process group, which starts the tool in that group and kills the whole group if the CLI dies by any signal (a signal to the CLI's own process group included). The CLI owns no timeout (`RUL-13`): the caller's timeout ends the CLI, and the reaper takes the tools with it. A `SIGKILL` sent to the reaper's own PID is outside this guarantee. The reaper is invoked through an internal first argument that is not a subcommand and is not part of this contract.

### Two witnesses for every verdict

| Check | Pass | Fail | Anything else |
|---|---|---|---|
| `check drc`, `check erc` (`kicad-cli` with `--exit-code-violations --format json`, `RUL-03`) | exit `0` and a report with no finding | exit `5` and a report with at least one finding | `ToolError`: another exit code, a missing or unparseable report, a count that disagrees with the code, a report whose `included_severities` is not exactly `["error","warning"]`, or whose `kicad_version` differs from the version `kicad-cli version` printed |
| `prove` (`cargo kani --harness <name>`) | exit `0`, `VERIFICATION:- SUCCESSFUL`, and `Complete - N successfully verified harnesses, 0 failures` with N ≥ 1 | exit `1` with `VERIFICATION:- FAILED` (`proof_failed`), or `Found N compilation errors` (`compile_error`), or `Failed to match the following harness(es)` (`harness_not_found`) | `ToolError` |
| `simulate` (`renode` loads the platform, then `peripherals`) | a CPU in the listing (an entry followed by a `Slot:` line) and no `There was an error executing command` | `There was an error executing command` and no CPU (`platform_load_error`) | `ToolError`: both, neither, or a nonzero exit. Renode exits `0` even when the load fails (`architecture.md` §2.4), so its exit code is recorded and never trusted alone |

Every KiCad finding is a fail, warnings included (ruled on PR #12): the pass criterion is that KiCad found nothing at its default severities.

### The verdict

```json
{"ok":false,"verdict":"fail","check":"drc",
 "input":{"path":"/work/drc-violation.kicad_pcb","sha256":"<64 hex>"},
 "tool":{"name":"kicad-cli","version":"10.0.7","exit_code":5},
 "sandbox":{"landlock_abi":8,"network_confined":true},
 "included_severities":["error","warning"],
 "findings":[{"kind":"clearance","severity":"error","message":"Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)",
              "where":[{"description":"Track [NET_B] on F.Cu, length 20.0000 mm","x":105.0,"y":110.3,"uuid":"…"}]}]}
```

- `ok` is `true` exactly when the exit code is `0`; `verdict` is `pass` or `fail`; `check` is `drc`, `erc`, `prove` or `simulate`.
- `input.path` is the canonical path of the input as given (for an `@` platform, the file inside Renode's tree); `input.sha256` is the hash of the copy the tool judged.
- `tool.version` is the tool's own version line, exactly as printed; `tool.exit_code` is the exit code of the judging run.
- `findings` is normalized to `{kind, severity, message, where}`. For KiCad, `kind` is the report's `type` (`clearance`, `track_dangling`, `pin_not_connected`, `label_dangling`, …), `severity` is `error` or `warning`, `message` is the report's `description`, and `where` lists the items it names. For `prove` and `simulate`, `kind` is one of the names in the table above, `severity` is `error`, and `where` is empty.
- `check drc` and `check erc` also carry `included_severities` from the report; `prove` carries `harness`; `simulate` carries `machine`: `{"cpus":["cpu"],"peripherals":[{"name":"cpu","type":"CortexM"},…]}`.

A verdict that cannot name its input's hash and its judge's version is never printed; the call refuses instead.

### The crate hash

`prove`'s `input.sha256` covers the crate's file tree. For every regular file beneath the crate directory, except under a top-level `target/` directory, take the line `<sha256 of the file, lowercase hex>  <path relative to the crate, with / separators>\n`. Sort the lines by path, byte order, and hash their concatenation. A crate holding a symlink, a special file or a name that is not UTF-8 is refused with exit `3`. The same tree, minus `target/`, is what is copied and judged.

### Running it

Each tool subcommand runs inside the published image, with the repository mounted (`-v "$PWD:/repo:ro" -w /repo`) and the CLI built into a mounted target directory. For example, from the repository root:

```
IMAGE=$(cat container/IMAGE)
docker run --rm -v "$PWD:/repo" -w /repo -e CARGO_TARGET_DIR=/repo/target "$IMAGE" cargo build --locked -p aethelgard-cli
docker run --rm --network none -v "$PWD:/repo:ro" -w /repo "$IMAGE" target/debug/aethelgard check drc --board container/fixtures/drc-violation.kicad_pcb
docker run --rm --network none -v "$PWD:/repo:ro" -w /repo "$IMAGE" target/debug/aethelgard check erc --schematic container/fixtures/erc-dangling-label.kicad_sch
docker run --rm --network none -v "$PWD:/repo:ro" -w /repo "$IMAGE" target/debug/aethelgard prove --crate container/fixtures/kani-smoke --harness add_in_range
docker run --rm --network none -v "$PWD:/repo:ro" -w /repo "$IMAGE" target/debug/aethelgard simulate --platform @platforms/boards/nucleo_h753zi.repl
```
