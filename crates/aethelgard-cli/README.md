# aethelgard-cli

The `aethelgard` binary. It is the contract every caller uses (`docs/architecture.md` `RUL-10`): exactly one JSON object on one line of stdout, and the exit codes below. Diagnostics go to stderr. `AET-WO-1500` inherits these codes; changing one is a breaking change.

The repository `README.md`'s `## Quickstart` runs `emit`, `check erc`, `check drc` and `solve` from the published engine image.

## Exit codes

| Code | Meaning | `error.kind` values |
|---|---|---|
| `0` | `validate`: the specification is valid. `solve`: it was solved, and stdout is the `SolvedTopology`. A tool subcommand: the tool ran and its verdict is pass | none: stdout is the success report, the `SolvedTopology`, or the verdict |
| `1` | A tool subcommand: the tool ran and its verdict is fail (violations found, the proof failed, a declared cover was never reached, the crate did not compile, the harness does not exist, the platform did not load). The input is at fault; the tool worked | none: stdout is the verdict |
| `2` | Invalid specification: well-formed JSON of the wrong shape (unknown, missing or duplicate field, wrong type including deep nesting where a value was expected, negative or overflowing number), or a value `validate()` refuses. `solve` also: a part with no pin table, or a specification the solver cannot realize. `emit` also: what `solve` refuses, and a part with no vendored model, an address the part cannot take, a reference used twice, a net, rail or reference the emitter needs for itself, a net the solver did not place, a pin the MCU's symbol lacks, or a board the router cannot complete. `check drc --spec`: the specification refused as `validate` refuses it. `simulate --firmware`: the specification refused as `solve` refuses it, or a solved topology the twin cannot lower to a platform | `SchemaViolation`, or an `IrValidationError` variant name such as `InvalidIdentifier`, `NoVoltageDomain`, `UnknownDomain`, `DuplicateI2cAddress` or `ZeroNominalVoltage`; `InvalidIdentifier` also carries `error.path`. `solve` also: `UnknownPart`, or a `SolveError` kind (below), which carries `error.claimants`. `emit` also: `UnknownPart`, `UnsupportedAddress`, `DuplicateReference` (also two references KiCad takes for one, differing only in case or in a leading zero: `u2` and `U2`, `U01` and `U1`), `ReservedName` (also a net or rail starting `unconnected-`), `UnsolvedNet`, `UnknownPin`, `Unroutable` (`id` the reference, net or name; for `Unroutable`, the net the router could not complete, empty when the board as a whole is refused). `simulate --firmware` also: `UnsupportedPeripheral`, `NotInRenode`, `InvalidPin`, `AddressMismatch` |
| `3` | Unreadable input. `validate`: the file or stdin cannot be read, is empty, is more than 16 MiB, or is not one JSON document (syntax error, trailing data, a byte-order mark). A tool subcommand: an input path that does not exist, cannot be read, or is not a regular file (not a directory, for `--crate`); a crate holding a symlink, a special file or a non-UTF-8 name; a Renode built-in path outside Renode's own tree; `InputTooLarge`: a copied file, or a crate's files together, of more than 16 MiB (16,777,216 bytes), or a crate of more than 4,096 files (counted after `target/` is skipped). The byte cap is enforced while copying, never by truncating. `check drc`: a board that is not one KiCad S-expression, or one that carries a rule of its own, refused before any tool runs: a local `(clearance …)` on a pad, a footprint or a zone, or a net class defined in the board (`AET-WO-1300` D4). `check drc --schematic`: a schematic that is not one flat KiCad schematic, or a schematic or board carrying what would pass parity unmatched (below). `emit`: an `--out` that cannot be created, or a file there that is a symlink or not a regular file. `simulate --firmware`, before Renode is looked for: a `--platform` that is not byte for byte the platform `aethelgard-twin` emits for `--spec`, or an `@` path; a firmware that is not ELF32 little-endian `EM_ARM` `ET_EXEC` with a sound program header table; a `PT_LOAD` whose physical or virtual range is outside the part's flash or RAM, two `PT_LOAD`s whose memory overlaps, or file bytes over the boot word at either address (below) | `UnreadableInput`, `InputTooLarge`, `InvalidJson`; `MalformedBoard`, `LocalClearance`, `BoardNetClass` and `BoardWaiver` (`id` naming the carrier, such as `pad 1 of footprint R7`); `MalformedSchematic`, `HierarchicalSchematic`, `ReservedNetName`, `BoardOnlyFootprint`, `NoConnectPadOnNet`, `SharedUnconnectedNet`; `UnwritableOutput`; `PlatformNotEmitted`, `MalformedFirmware` (overlapping memory included), `FirmwareOutsideMemory`, `FirmwareLoadsBootWord` |
| `4` | The tool cannot be used: it is not on `PATH`, is not executable, will not report its version, or lacks an environment variable it needs; for `simulate --firmware`, a Renode other than the 1.17.0 the twin's platform is pinned to; or the sandbox is unavailable. Never a verdict (`INV-06`) | `ToolMissing` (`id`: the tool), `ToolUnusable` (`id`: the tool, or the missing variable), `SandboxUnavailable` (`id`: `landlock`, `seccomp`, or `descriptors`); `simulate --firmware`: `bridge_unavailable` (`id`: `aethelgard-bridge`) when the bridge is not beside the `aethelgard` binary |
| `5` | The tool ran but its output cannot be trusted as a verdict: killed by a signal, an exit code outside the ones measured for it, a missing, unparseable or oversized report or output, or two witnesses that disagree. When `prove` or `simulate` cannot recognise its tool's outcome, the message ends `; the tool said: <line>`, every `error:` line (Kani, cargo) or the first `[ERROR]` line (Renode), as printable ASCII, cut to 400 bytes for cargo or 240 for Renode; for cargo it is every `error:` line joined by `|`. Never read as pass or fail | `ToolError` (`id`: the tool) |
| `64` | Usage error: unknown subcommand or flag, missing subcommand or required flag. `simulate`: `--firmware`, `--spec` and `--for` given other than all together, or a `--for` that is not an integer without a leading zero then `ns`, `us` or `ms`, is above 100 ms, or is not a multiple of 10 ns | `UsageError` |
| `70` | Internal error: the report could not be written to stdout (for example, the reader closed the pipe), the parent-death signal could not be set, or the process that started this one is already gone (see the reaper below); or `solve`'s vendored pin table, or `emit`'s vendored KiCad library, failed its own hash or parse check, which only a corrupted build can cause. Nothing is printed on stdout; the reason goes to stderr | none |

A stdout closed by the caller (`>&-`) is not an exit-`70` case: the Rust runtime reopens a closed standard descriptor on `/dev/null` before `main` runs, so it behaves exactly like `>/dev/null`, and the exit code still reports the verdict.

`--help` and `--version` exit `0` and are the only output that is not JSON. `message` is for humans and its wording is not part of the contract; `ok`, `kind`, `id` and the verdict fields are.

## `validate`

```
aethelgard validate [--file <path>]
```

Reads one `Specification` from stdin, or from `--file` when given (stdin is then not read; `-` is a file name, not stdin), and runs `Specification::validate()`. The specification and everything nested in it must be JSON objects. A `SolvedTopology` document is not a specification and is refused as a `SchemaViolation`.

On success, exit `0`:

```json
{"ok":true,"project_name":"sentinel-01","voltage_domains":1,"net_constraints":2}
```

On refusal, exit `2` or `3`:

```json
{"ok":false,"error":{"kind":"ZeroNominalVoltage","id":"VDD_3V3","message":"voltage domain `VDD_3V3` has nominal_millivolts 0"}}
```

`id` is the offending name exactly as written (nothing is trimmed; whitespace is refused as an invalid identifier), or `null` when the refusal names no single thing: `InvalidIdentifier` (the JSON path of the field, such as `i2c_buses[0].sda.net`, is in the optional `path` field, which exists for that kind only and is absent from every other refusal; `message` repeats it for humans), `UnsupportedSchemaVersion`, `NoVoltageDomain`, a whole-document error.

Every key is required, including `peripheral`, `pin` and `impedance_ohms`, which are `null` when unset. The shape and the refusals, in check order, are in `crates/aethelgard-ir/README.md`. See `examples/` at the repository root.

## `solve`

```
aethelgard solve --file <path>
```

Reads one `Specification` from the file, under the same 16 MiB cap and with the same exit `2` and `3` refusals as `validate`, and validates it. It then loads the pin table for `mcu.part` (exit `2`, `UnknownPart`, `id` the part, for any part but `STM32H753ZITx`), solves (`aethelgard_ir::solve`), and prints the `SolvedTopology` as one JSON line, exit `0`. `specification_sha256` is the sha256 of the file's bytes as read, so reformatting the file changes the hash and nothing else. It writes no file.

```json
{"schema_version":1,"specification_sha256":"acd60081…","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},…},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000}}
```

A specification the solver cannot realize is exit `2`. The refusal envelope adds `claimants`, an array naming who is in conflict:

```json
{"ok":false,"error":{"kind":"PinClaimedTwice","id":"PB9","claimants":["i2c_buses[0].sda","spi_devices[0].cs"],"message":"pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"}}
```

The kinds, what `claimants` names for each, the search order, the reserved pins and the check order are in `crates/aethelgard-ir/README.md`.

## `emit`

```
aethelgard emit --file <path> --out <dir>
```

Reads, validates and solves exactly as `solve` does, with the same refusals. It then emits the design with `aethelgard_eda::schematic::emit` and writes three files into `<dir>`, creating it if needed:

- `<project_name>.kicad_sch`: a KiCad 10 schematic (format `20260306`) whose symbols are embedded from the vendored KiCad 10.0.7 library;
- `<project_name>.bom.csv`: one CSV row per part (`Reference,Value,Footprint,Symbol,Note`, RFC 4180, CRLF), sorted by reference;
- `<project_name>.kicad_pcb`: the placed and routed board (format `20260206`), with every part's vendored footprint, each pad on its schematic net, a rectangular outline, tracks on F.Cu and B.Cu, vias, and an unfilled ground zone on B.Cu that `check drc`'s `--refill-zones` fills. From the reference specification, `check erc` on the schematic and `check drc --spec --schematic` on the board both exit `0` with no finding (P1 gate clause 1, `AET-WO-1302`). A board the router cannot complete is not written: `emit` exits `2` with `Unroutable`, naming the net. `crates/aethelgard-eda/README.md` gives the placement and routing rules.

All three are byte-identical for the same specification. It prints one line, exit `0`:

```json
{"ok":true,"project_name":"sentinel-01","specification_sha256":"<64 hex>","schematic":{"path":"/out/sentinel-01.kicad_sch","sha256":"<64 hex>"},"bill_of_materials":{"path":"/out/sentinel-01.bom.csv","sha256":"<64 hex>"},"board":{"path":"/out/sentinel-01.kicad_pcb","sha256":"<64 hex>"}}
```

- **Parts with a model:**
  - the MCU `STM32H753ZITx`;
  - on I2C, `BME280` (address 118 or 119) and `TMP117` (72 to 75);
  - on SPI, `W25Q64JV`, drawn with KiCad's `W25Q32JVSS` symbol, same SOIC-8 pinout, with the substitution noted in its bill-of-materials row.
- **Refused:** any other part, address or name clash is refused (exit `2`), never approximated.
- **Not overwritten unsafely:** an existing file is replaced only if it is a regular file; a symlink is never followed (exit `3`, `UnwritableOutput`).
- **What the emitter adds:** pull-ups per I2C bus, decoupling, VCAP capacitors, strap resistors, `PWR_FLAG`s and no-connects. `crates/aethelgard-eda/README.md` describes them.
- **Net names:** the schematic names each signal net by a local label, so KiCad names the net `/<net>`. A rail's net is its domain id, and ground is `GND`.

## The tool subcommands

```
aethelgard check drc --board <file.kicad_pcb> [--spec <specification.json>] [--schematic <file.kicad_sch>]
aethelgard check erc --schematic <file.kicad_sch>
aethelgard prove --crate <dir> --harness <name>
aethelgard simulate --platform <file.repl | @path-inside-renode>
aethelgard simulate --platform <emitted.repl> --firmware <elf> --spec <specification.json> --for <time>
```

Each runs one external tool on one input and prints one verdict. They are **Linux-only**, because the tools and the sandbox are. The published tools image (`container/IMAGE`) has all four tools; see [Running it](#running-it).

### What every call does, in order

1. **Checks the input path** (exit `3` if refused). It must be UTF-8, so the verdict can report it exactly. `--board`, `--schematic` and a `--platform` file must be regular files after following symlinks; `--crate` must be a directory. A `--platform` starting with `@` names a file inside Renode's own tree (the directory holding the real `renode` binary), as Renode's own `@` syntax does.
   `--harness` must be a Rust path: letters, digits, `_` and `::` (exit `64` otherwise).
2. **Finds the tool** by searching the caller's `PATH` for `kicad-cli`, `renode`, or `cargo` and `cargo-kani` (exit `4`, `ToolMissing`, naming the first one absent). `prove` also needs `KANI_HOME` and `RUSTUP_HOME` set to existing directories, and `PATH` (exit `4`, `ToolUnusable`, naming the variable).
3. **Copies the input** into a fresh private temporary directory and hashes the copy. The tool judges the copy, never the original, so the hash in the verdict is of exactly the bytes given (`INV-03`), and nothing is written beside the input. `check drc` and `check erc` then **prepare the copy** (below), and they do steps 3 and the preparation before step 2, so an input refused for what it carries is refused whatever is installed.
4. **Asks the tool its version**, then **runs it**, each time in the sandbox below. A tool that will not report its version is exit `4`, `ToolUnusable`.
5. **Reads two witnesses** (below), prints the verdict, and removes the temporary directory.

### The gate's own rules (`check drc`, `check erc`)

No rule is ever read from the input (`RUL-03` as amended, `INV-07`). KiCad keeps a board's rules in the board's own setup block and its project files. A DRC that read them would judge the board by rules the board chose. Before KiCad runs, the gate:

- **Refuses a board that carries a rule of its own** (exit `3`, naming the carrier):
  - a `(clearance …)` anywhere outside its top-level setup block, that is, on a pad, a footprint or a zone (`LocalClearance`). A custom pad's `(options (clearance outline))` is its shape and stays. KiCad judges a locally overridden item only against the board minimum, never by the gate's rules (`AET-WO-1300` S3);
  - a net class defined in the board (`BoardNetClass`);
  - a footprint token that waives a check for itself: `net_tie_pad_groups`, `jumper_pad_groups`, `duplicate_pad_numbers_are_jumpers yes`, `allow_missing_courtyard` or `allow_soldermask_bridges` (`BoardWaiver`). Mask and paste margins are the design's geometry and stay;
  - a board that is not one KiCad S-expression, or that KiCad's reader could read differently from the gate's (`MalformedBoard`):
    - a control or non-ASCII byte outside a string;
    - a raw line break inside a string;
    - a line starting with `#` (KiCad skips it as a comment);
    - a `"` touching an atom;
    - a list whose head is not a lowercase keyword;
    - nesting deeper than 256, or more than 1,000,000 tokens.

    Each of these was measured hiding a board's rules from a naive scan while KiCad honoured them (`AET-WO-1300` adversarial pass).
- **Writes the setup block over the board's**, with no clearance token, so the board minimum has one source. Nothing else in the board changes.
- **Writes `input.kicad_pro`, `input.kicad_dru`, `sym-lib-table` and `fp-lib-table`** beside the copy:
  - `input.kicad_pro` holds the Default net class, the board minimum, and ERC's library checks set to `ignore`;
  - `input.kicad_dru` holds the same minimums as custom rules;
  - `sym-lib-table` and `fp-lib-table` name the vendored KiCad 10.0.7 libraries, which the gate copies into `lib/` beside them.

  The board's own `.kicad_pro` and `.kicad_dru` are never copied. No environment variable is set.

**With `--spec`**, the rules are what the specification implies, and the verdict says `"rules":"specification"` and carries the file's `specification_sha256`. The specification declares no spacing or fabrication field, so today that is the engine's floor: 0.2 mm clearance and board minimum, and a 0.2 mm minimum track width (`aethelgard_eda::rules::FLOOR_NM`).

**Without it**, the same files hold KiCad's own defaults (a 0.2 mm Default net class, no board minimum, no track-width minimum), and the verdict says `"rules":"kicad-default"`.

**With `--schematic`**, KiCad also checks the board against that schematic (`--schematic-parity`), and its `schematic_parity` findings are judged like every other finding. The schematic is copied and hashed as the board is, as `input.kicad_sch` beside the board's copy, where KiCad looks for it; the verdict carries `"schematic":{"path":…,"sha256":…}`. Before any tool runs (exit `3`, `aethelgard_eda::parity`), the gate refuses:
- a schematic that is not one KiCad S-expression `kicad_sch`, read as strictly as a board (`MalformedSchematic`);
- a schematic with a hierarchical `(sheet …)` anywhere, since KiCad would read the file it names, which the gate never copied or hashed (`HierarchicalSchematic`);
- a schematic with a label or value starting `unconnected-`, KiCad's own prefix (`ReservedNetName`).

It also refuses what KiCad 10.0.7's parity check was measured passing on a board that does not match its schematic (`AET-WO-1301` adversarial pass):
- a `board_only` footprint, which parity skips (`BoardOnlyFootprint`);
- a pad whose `pintype` says `no_connect` but which sits on a real net, which parity matches by prefix (`NoConnectPadOnNet`);
- an `unconnected-` net on more than one pad (`SharedUnconnectedNet`).

Without `--schematic`, nothing changes.

DRC also runs with `--refill-zones`: zones are judged filled, and the board is never saved, because KiCad's saved fills are not deterministic. `check erc` writes only `input.kicad_pro`.

### The sandbox

Every tool process runs with:

- an argument array, never a shell, and every input given as an absolute path inside the temporary directory, so no file name can be read as a flag;
- an **empty environment** plus `HOME` and `TMPDIR` set to the temporary directory; `prove` alone also passes `KANI_HOME`, `RUSTUP_HOME` and `PATH` through from the caller, by name;
- stdin from `/dev/null`, and stdout and stderr captured up to 16 MiB each (more is exit `5`);
- **Landlock**: read-only access to the system directories, the tool's own installation and the input copy, read-write access only to its temporary directory, and no other filesystem access. Renode also gets read-only `/proc` and `/dev/urandom`; Kani also gets read-write `/dev/null`. Landlock's filesystem rights through ABI 3 are **required**: if the kernel cannot enforce them, the call refuses with exit `4`, `SandboxUnavailable`, `id` `landlock`. There is no flag or variable that runs a tool unconfined. Where the kernel can, Landlock also denies TCP bind and connect (ABI 4 and later; `sandbox.tcp_confined`) and confines signals and abstract Unix sockets to the tool's own processes (ABI 6 and later; `sandbox.scoped`);
- a **seccomp filter** (AET-WO-0300a), installed after Landlock and before the tool starts, inherited by every process the tool starts, and **required** like Landlock: if it cannot be installed, the call refuses with exit `4`, `SandboxUnavailable`, `id` `seccomp`, and there is no flag or variable that skips it. It refuses `socket()` for every family with `EPERM`, so pathname and abstract Unix sockets, UDP, TCP on any kernel, raw and netlink sockets are all out of reach, and it refuses `io_uring_setup`, `io_uring_enter` and `io_uring_register` with `EPERM`, since `io_uring` can open and connect sockets without the `socket` call. It kills the process on any system call made through another architecture, the x32 ABI and i386 included; on a host that is not x86_64 there is no filter, so every tool call refuses with `id` `seccomp`. `socketpair` is allowed only as an `AF_UNIX` stream or seqpacket pair, which is born connected to itself and cannot be redirected; a datagram pair could `connect` to a datagram socket outside, so it is refused. No tool needs a socket (measured; Renode's .NET runtime would try one for its diagnostic IPC, so the CLI also sets `DOTNET_EnableDiagnostics=0` for Renode). `sandbox.sockets_denied` is always `true` in a verdict. **Not confined at the socket layer: nothing.** Outside this filter's scope, a tool can still read what Landlock lets it read, write its temporary directory, and use non-socket channels Landlock does not mediate, such as System V shared memory with other processes of the same user.;
- **no inherited descriptors**: before it confines itself, the trampoline lists its open descriptors (`/proc/self/fd` and `/proc/self/fdinfo`). Any descriptor above 2 without close-on-exec, which the tool would inherit as a channel no other layer sees, refuses the call with exit `4`, `SandboxUnavailable`, `id` `descriptors`, naming the descriptor numbers, and the tool never runs. An unreadable `/proc` refuses the same way. The tool gets stdin from `/dev/null` and its two output pipes, nothing else;
- a **reaper**: the CLI starts a copy of itself, in a new process group, so a signal to the CLI's own group does not reach it. The reaper starts the tool in a second group and, as a child subreaper, inherits every orphan the tool leaves, a daemon in its own session included. When the tool ends, or the CLI dies by any signal, the reaper kills the tool's group and every remaining descendant. A **guardian**, started before the temporary directory is created, removes that directory if the CLI dies before removing it itself. The CLI passes both its own PID, so a CLI killed before either has looked is still seen to be gone, at any instant from its first line to the tool's `exec`. A CLI started by `mcp` also dies, by `SIGKILL`, when `mcp` dies (`PR_SET_PDEATHSIG`); the reaper and the guardian do not, so they clean up after it. `mcp` tells each CLI its pid through an internal variable; with it set, the CLI arms the signal and then exits `70`, before it starts anything, if that parent is already gone. Linux sends the signal when the parent *thread* ends, so no other CLI arms it, and `mcp` never arms it on itself: a CLI run directly behaves as before, and its caller owns its lifetime, as it owns its timeout (`RUL-13`). The CLI owns no timeout (`RUL-13`): the caller's timeout ends the CLI, and the reaper and guardian clean up after it. A `SIGKILL` sent to the reaper's own PID by something outside the sandbox is outside this guarantee; the tool itself cannot send it when `sandbox.scoped` is true. The reaper, the guardian and the Landlock trampoline are invoked through internal first arguments that are not subcommands and are not part of this contract.

### Two witnesses for every verdict

| Check | Pass | Fail | Anything else |
|---|---|---|---|
| `check drc`, `check erc` (`kicad-cli` with `--exit-code-violations --format json`, `RUL-03`) | exit `0` and a report with no finding | exit `5` and a report with at least one finding | `ToolError`: another exit code, a missing or unparseable report, a count that disagrees with the code, a report whose `included_severities` is not exactly `["error","warning"]`, or whose `kicad_version` differs from the version `kicad-cli version` printed |
| `prove` (`cargo kani --harness <name>`, plus `--exact` when the name is a path) | exit `0`, `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures`, exactly one `Checking harness <path>...` whose path is the name asked for or ends in `::<name>`, and, if Kani prints a cover summary, `** N of N cover properties satisfied` | exit `0` as for a pass, but with a cover summary `** S of N …` where S < N (`cover_unsatisfied`: a `kani::cover!` the harness declares is never reached, so the proof may be vacuous; AET-WO-1400); or exit `1` with `VERIFICATION:- FAILED` (`proof_failed`), or `Found N compilation errors` (`compile_error`), or `Failed to match the following harness(es)` (`harness_not_found`); or, with exit `0` or `1`, checked harnesses none of which has that name (`harness_not_found`: Kani's `--harness` matches substrings) or more than one (`harness_ambiguous`: name it by its path) | `ToolError` |
| `simulate --firmware` (below) | the three boot witnesses agree: the boot word holds the magic, every solved register reads as the topology computes it, and the core executed exactly 100 MIPS × the virtual time; the core ends inside an executable segment of the firmware; a CPU and every solved peripheral listed after the reads; no `There was an error executing command`, no `[ERROR]` line and no locked-peripheral warning | `boot_word_missing`, `register_mismatch` (one per register), `cpu_halted` (fewer instructions than the run holds), `wall_bound_exceeded`, `memory_cap_exceeded` and `cpu_aborted` (killed at a bound, below), `locked_peripheral_access` (the firmware touched a peripheral whose clock was off), `pc_outside_firmware` (the core ended outside the firmware's executable segments), `renode_error_log`; `platform_load_error` or `firmware_load_error` when that command failed and nothing was read | `ToolError`: a nonzero exit; markers missing or repeated; a read that is not a bare value; a load error beside reads; another command's error; no CPU, or a listing without a solved peripheral; more instructions than the run holds; or the boot word set while the core executed fewer |
| `simulate` (`renode` loads the platform, then `peripherals`) | a CPU in the listing (an entry followed by a `Slot:` line), no `There was an error executing command` and no `[ERROR]` log line | `There was an error executing command` and no CPU, or a CPU listed beside any `[ERROR]` line (`platform_load_error`; the finding's message is the first `[ERROR]` line). `[WARNING]` lines do not change the verdict | `ToolError`: both, neither, or a nonzero exit. Renode exits `0` even when the load fails (`architecture.md` §2.4), so its exit code is recorded and never trusted alone |

Every KiCad finding is a fail, warnings included (ruled on PR #12): the pass criterion is that KiCad found nothing at its default severities.

### The verdict

```json
{"ok":false,"verdict":"fail","check":"drc",
 "input":{"path":"/work/drc-violation.kicad_pcb","sha256":"<64 hex>"},
 "tool":{"name":"kicad-cli","version":"10.0.7","exit_code":5},
 "sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true},
 "included_severities":["error","warning"],
 "findings":[{"kind":"clearance","severity":"error","message":"Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)",
              "where":[{"description":"Track [NET_B] on F.Cu, length 20.0000 mm","x":105.0,"y":110.3,"uuid":"…"}]}]}
```

- `ok` is `true` exactly when the exit code is `0`; `verdict` is `pass` or `fail`; `check` is `drc`, `erc`, `prove` or `simulate`.
- `input.path` is the canonical path of the input as given (for an `@` platform, the file inside Renode's tree); `input.sha256` is the hash of the input's bytes as copied. For `check drc` the tool then judges that copy with the gate's setup block written over the board's, a function of those bytes alone (above).
- `tool.version` is the tool's own version line, exactly as printed; `tool.exit_code` is the exit code of the judging run.
- `findings` is normalized to `{kind, severity, message, where}`. For KiCad, `kind` is the report's `type` (`clearance`, `track_dangling`, `pin_not_connected`, `label_dangling`, …), `severity` is `error` or `warning`, `message` is the report's `description`, and `where` lists the items it names. For `prove` and `simulate`, `kind` is one of the names in the table above, `severity` is `error`, and `where` is empty.
- `check drc` and `check erc` also carry `included_severities` from the report; `check drc` carries `rules`, with `--spec` `specification_sha256`, and with `--schematic` `schematic` (above); `prove` carries `harness`; `simulate` carries `machine`: `{"cpus":["cpu"],"peripherals":[{"name":"cpu","type":"CortexM"},…]}`, and with `--firmware` also `boot` (below).

A verdict that cannot name its input's hash and its judge's version is never printed; the call refuses instead.

### Booting firmware (`simulate --firmware`)

`AET-WO-2100`, ruled on PR #56. The platform is the one `aethelgard-twin` emits for the specification, committed for the reference board as `boards/sentinel-01/sentinel-01.repl`; any other platform is refused (above). The reference firmware is generated beside the board crate at `boards/sentinel-01/firmware/` and builds with `cargo build --release --offline --locked` from that directory into the repository's `target/`.

**The boot contract** (`aethelgard_twin::boot`), which the generated runtime follows:
- the **boot word** is the first word of RAM, `D1_DTCMRAM_BASE` (`0x20000000`), and the firmware's fixed point writes `0x2100B007` to it;
- the firmware never sleeps once it is running: the instruction witness needs the core busy for the whole run;
- a firmware may load only into flash, `FLASH_BANK1_BASE` to `FLASH_END` (`0x08000000`–`0x081FFFFF`), or RAM, `D1_DTCMRAM_BASE` and the 128 KB its comment states. Every address comes from ST's pinned header.

**The run.** One Renode call executes:
1. `emulation SetSeed 2100`, so the platform's random initial values are the same in every run;
2. the platform, then `sysbus LoadELF` of the firmware's copy;
3. `0` written to every RCC enable register that gates a solved port or peripheral, silicon's reset value (Renode resets them with every clock on), and to the boot word, so only the firmware's code can set it;
   then `cpu VectorTableOffset` set to `FLASH_BANK1_BASE`, where the part boots, rather than letting Renode guess the table from the ELF;
4. `emulation RunFor` the `--for` time;
5. then, under `logLevel 3`, the reads between two marker lines (the boot word, `cpu ExecutedInstructions`, `cpu PerformanceInMips`, `cpu PC` and each solved register), then `peripherals`.

**The bounds** (`RUL-13` as amended for `simulate --firmware` only, ruled on PR #56, B1): the reaper kills Renode's whole process tree after **30 s** of wall-clock time, or once the tree's resident memory (the sum of its processes' `VmRSS`, read every 100 ms) passes **1 GiB**. The call is then a `fail` that names the bound: `wall_bound_exceeded` or `memory_cap_exceeded`, or `cpu_aborted` if Renode logged `This machine has aborted` first (its `RunFor` then never returns). Nothing is read, and `tool.exit_code` is `-1`. The constants are measured: the reference boot at `--for 100ms` took at most 2.59 s and 404,620 KiB in the tools image, and the ruling asks at least 10× and 2× of those. `RLIMIT_AS` cannot do it, because CoreCLR will not start under any address-space limit. Plain `simulate`, `prove` and `check` stay unbounded, and the caller's timeout ends them.

The reader removes a stray `\r` at either end of a line, matches each marker as a whole line, and takes each read only as a line that is a bare `0x…` value. Anything else between the markers is exit `5`.

**What is judged.** The expected registers come from the solved topology alone (`INV-07`), never from the firmware:
- each solved pin's `MODER` field (alternate function, or output for a `gpio` net);
- its `AFRL`/`AFRH` field for an alternate function;
- each cold RCC register in full, which must hold exactly the solved enable bits;
- and the core's final `PC`, which must lie inside an executable `PT_LOAD` of the firmware.

**The named residual** (ruled on PR #56, B2): the boot word witnesses that the solved configuration happened and that the core kept executing inside the firmware's own code; it does not witness that `main` returned.

`GPIOx.OTYPER` and `I2Cn.TIMINGR` are solved but Renode 1.17.0 stores neither, so the verdict lists them as `unobservable`.

**`boot` in the verdict:**

```json
"boot":{"firmware":{"path":"…","sha256":"<64 hex>","image_sha256":"<64 hex>"},
        "specification_sha256":"<64 hex>","seed":2100,"virtual_time_ns":10000000,"mips":100,
        "instructions":{"expected":1000000,"observed":1000000},"pc":"0x08000490",
        "boot_word":{"address":"0x20000000","expected":"0x2100b007","observed":"0x2100b007"},
        "registers":[{"name":"GPIOA.MODER","address":"0x58020000","mask":"0x0000fc03","expected":"0x0000a801","observed":"0x0000a801"},…],
        "unobservable":["GPIOB.OTYPER","I2C1.TIMINGR"]}
```

Every `observed` (and `mips`, `pc`) is `null` when nothing was read, as after a load error. `firmware.image_sha256` is the sha256, over every `PT_LOAD` with memory in physical-address order, of its physical address, file size and memory size (each a `u32` little-endian) and then its file bytes: the memory image Renode builds. It is the firmware's identity, because the file's own hash also covers symbol names, which Cargo derives from the checkout path.

### The analog pre-pass (`simulate --firmware`)

`AET-WO-2200` (`RUL-17`). Before Renode is looked for, each I2C line of the solved topology is measured, and the measurement decides the platform the boot loads:

1. `aethelgard-twin::analog` writes one netlist per line from the topology's `analog_lines`: the pull-up from the domain's rail, the line's capacitance, and the open-drain driver, released once in a 22.5 µs window (`AET-WO-2101` S2).
2. `aethelgard-bridge`, found **beside the `aethelgard` binary** (`bridge_unavailable`, exit `4`, if it is not), measures each line's 30 %–70 % rise with libngspice 44.2 in its own process. It runs as a sandboxed tool like Renode, with the same 30 s and 1 GiB bounds, in a directory of its own, so it can write none of the copies Renode reads. Its rise time is its own interpolation of the points ngspice accepted, and it must agree with ngspice's `meas` to 0.01 ns.
3. A line whose rise exceeds its ceiling (the net constraint's `max_rise_time_nanoseconds`) fails the verdict with `analog_rise_exceeds_ceiling`, and its bus is faulted: the platform the boot loads is the emitted one with that bus's devices left out and the decision written into its header. That file is checked against the emission for the specification plus the recorded `faults` just before Renode starts (`PlatformNotEmitted`, exit `3`). `--platform` is still the platform emitted for `--spec` without any fault.
4. The boot runs and is judged as above.

A bridge that exits nonzero, is killed, or answers anything but one measurement per line, in order, is `ToolError` (exit `5`, `id` `aethelgard-bridge`): no line is ever read as unfaulted. That includes a line that never reaches 70 % inside the window.

The verdict carries the pass beside `boot`, never inside it. The firmware sees a faulted bus only as its devices' absence, which Renode 1.17.0 renders as a TXIS timeout because it models no NACK; `firmware_sees` says so.

```json
"analog":[{"kind":"analog.rise_time_measured","source":"ngspice","bus":"SENSORS","net":"I2C_SCL",
           "rise_30_70_ns":3982.300059495949,"ceiling_ns":300,"exceeds":true,"method":"interpolated","ngspice":"44.2"},…],
"faults":[{"kind":"bridge.fault_state","source":"aethelgard","bus":"SENSORS","fault":"absent",
           "from":"analog.rise_time_measured > ceiling_ns","nets":["I2C_SCL","I2C_SDA"],"firmware_sees":"…"}]
```

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

## `mcp`

`aethelgard mcp` serves the other subcommands as [Model Context Protocol](https://modelcontextprotocol.io) tools on stdin and stdout (`RUL-10`, built on `rmcp` 3.7.0 with its `server` and `transport-io` features only). It takes no arguments, listens on no socket, holds no credential, and exits `0` when stdin closes. Stdout carries protocol messages and nothing else; diagnostics go to stderr. It is not a tool itself and adds no tool behaviour.

Before it starts, it runs the trampoline's inherited-descriptor check. A descriptor above 2 without close-on-exec refuses with exit `4`, `SandboxUnavailable`, `id` `descriptors`; the refusal object goes to stderr, since stdout is reserved for the protocol. A `SIGTERM`, `SIGINT` or `SIGHUP` kills the running calls and exits `143`, `130` or `129`, from the first moment, handshake or not. Closing stdin before `initialize` also exits `0`. `SIGKILL` cannot be handled; each running call's CLI then dies by its parent-death signal, and its reaper and guardian clean up.

### The tool list

Generated at startup from the unbuilt `clap` command tree, never written by hand, so a new subcommand appears with no change to the `mcp` module (`INV-01`):

- one tool per leaf subcommand, named by its path joined with `_`: `validate`, `solve`, `emit`, `check_drc`, `check_erc`, `prove`, `simulate` (whose optional `firmware`, `spec` and `for` are the boot). `help` and `mcp` are never tools;
- the description is the subcommand's about text;
- the input schema is an object with one **string** property per declared argument, keyed by its **long name** (`prove`'s directory is `crate`, not `crate_dir`), `required` exactly when `clap` requires it, and `additionalProperties` false.

Every argument must be a long-named, value-taking option. Anything else (a positional, a boolean flag) makes the tool list refuse to build, which a unit test catches before a client can.

### A call

The server checks the arguments against the schema (declared names only, string values only, required ones present, no NUL byte). A failure is an MCP invalid-params error (`-32602`) and nothing runs; so is an unknown tool name. Otherwise it runs this binary itself as a child, with stdin from `/dev/null`, and the argument vector is the subcommand path followed by one `--<long>=<value>` token per argument. One token per argument means a value starting with `-` can never be read as a flag; the CLI judges it. The internal first arguments are not in the tree, so no call reaches them. The call therefore has exactly the CLI's sandbox, exit codes and JSON.

`validate` reads stdin when `file` is absent, and the child's stdin is empty, so pass `file`: without it the call is refused as empty input (exit `3`).

### The result

For a child that exits and prints exactly one JSON object line:

- `content[0].text` is that line, unchanged, without its newline;
- `structuredContent` (its object keys sorted, so not byte-identical to the text) is `{"exit_code": <n or null>, "verdict": <the child's object>}`. `null` means the child was killed by a signal;
- `isError` is `false` for exit `0` and `1`, because a fail verdict is a successful call, and `true` for every other exit code. A refusal, such as `ZeroNominalVoltage` (exit `2`), is therefore `isError` `true` with the refusal object as the verdict.

`isError` is `true` for exit `null` as above. It is also `true`, with no `structuredContent` and a text naming the problem, when stdout is not exactly one JSON object line (a child killed before it printed one, for example), and when it exceeds a cap (the server kills the child, so a legitimate verdict over 1 MiB is reported as the cap): stdout 1 MiB, stderr 64 KiB. Past a cap the server kills the child and keeps draining until the pipes close.

### Cancellation and bounds

A `notifications/cancelled` for a call kills its child; the CLI's reaper then ends the tool's whole process group. Closing stdin lets running calls finish for up to 5 s (`rmcp`'s grace), then cancels them. The server sets no timeout of its own (`RUL-13`); only `simulate --firmware` bounds its tool, with the wall bound and memory cap above. `rmcp` drops a JSON-RPC line over 16 MiB without replying. The number of concurrent calls is not bounded by this server.

### Human check

```
npx @modelcontextprotocol/inspector aethelgard mcp
```

lists the seven tools, and a `validate` call with `file` set to `examples/sentinel-01-minimal.json` returns `isError` false.
