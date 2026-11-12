# aethelgard-cli

The `aethelgard` binary. It is the contract every caller uses (`docs/architecture.md` `RUL-10`): exactly one JSON object on one line of stdout, and the exit codes below. The one exception is `campaign`, which prints one line per scenario before its summary line (`AET-WO-2300`). Diagnostics go to stderr. `AET-WO-1500` inherits these codes; changing one is a breaking change.

The repository `README.md`'s `## Quickstart` runs `emit`, `check erc`, `check drc` and `solve` from the published engine image.

## Exit codes

| Code | Meaning | `error.kind` values |
|---|---|---|
| `0` | `validate`: the specification is valid. `solve`: it was solved, and stdout is the `SolvedTopology`. A tool subcommand: the tool ran and its verdict is pass. `campaign`: every scenario's verdict matched its expectation. `replay`: the run replayed to its committed hash. `attribute`: the run is attributed `electrical`, `software` or `none`. `propose`: a proposal was written. `generate`: the tree was written. `verify`: every gate passed | none: stdout is the success report, the `SolvedTopology`, or the verdict |
| `1` | A tool subcommand: the tool ran and its verdict is fail (violations found, the proof failed, a declared cover was never reached, the crate did not compile, the harness does not exist, the platform did not load). The input is at fault; the tool worked. `campaign`: every scenario ran and was archived, and at least one verdict did not match its expectation. `attribute`: the run is `indeterminate`. `propose`: nothing is proposed, and `proposal.json` says why. `build`: the crate did not build (`build_failed`). `verify`: a gate's verdict is fail, or `--against` differs from the generated tree | none: stdout is the verdict, the attribution, or `propose`'s line |
| `2` | Invalid specification: well-formed JSON of the wrong shape (unknown, missing or duplicate field, wrong type including deep nesting where a value was expected, negative or overflowing number), or a value `validate()` refuses. `solve` also: a part with no pin table, or a specification the solver cannot realize. `emit` also: what `solve` refuses, and a part with no vendored model, an address the part cannot take, a reference used twice, a net, rail or reference the emitter needs for itself, a net the solver did not place, a pin the MCU's symbol lacks, or a board the router cannot complete. `check drc --spec`: the specification refused as `validate` refuses it. `simulate --firmware`: the specification refused as `solve` refuses it, or a solved topology the twin cannot lower to a platform | `SchemaViolation`, or an `IrValidationError` variant name such as `InvalidIdentifier`, `NoVoltageDomain`, `UnknownDomain`, `DuplicateI2cAddress` or `ZeroNominalVoltage`; `InvalidIdentifier` also carries `error.path`. `solve` also: `UnknownPart`, or a `SolveError` kind (below), which carries `error.claimants`. `emit` also: `UnknownPart`, `UnsupportedAddress`, `DuplicateReference` (also two references KiCad takes for one, differing only in case or in a leading zero: `u2` and `U2`, `U01` and `U1`), `ReservedName` (also a net or rail starting `unconnected-`), `UnsolvedNet`, `UnknownPin`, `Unroutable` (`id` the reference, net or name; for `Unroutable`, the net the router could not complete, empty when the board as a whole is refused). `simulate --firmware` also: `UnsupportedPeripheral`, `NotInRenode`, `InvalidPin`, `AddressMismatch`. `campaign` (below): a scenario's `SchemaViolation`, `UnsupportedSchemaVersion`, `ExpectContradiction`, `DuplicateOverride`, `UnknownBus`, `UnknownNet`, `DuplicateScenario`, and what `solve` refuses of the overridden specification |
| `3` | Unreadable input. `validate`: the file or stdin cannot be read, is empty, is more than 16 MiB, or is not one JSON document (syntax error, trailing data, a byte-order mark). A tool subcommand: an input path that does not exist, cannot be read, or is not a regular file (not a directory, for `--crate`); a crate holding a symlink, a special file or a non-UTF-8 name; a Renode built-in path outside Renode's own tree; `InputTooLarge`: a copied file, or a crate's files together, of more than 16 MiB (16,777,216 bytes), or a crate of more than 4,096 files (counted after `target/` is skipped). The byte cap is enforced while copying, never by truncating. `check drc`: a board that is not one KiCad S-expression, or one that carries a rule of its own, refused before any tool runs: a local `(clearance …)` on a pad, a footprint or a zone, or a net class defined in the board (`AET-WO-1300` D4). `check drc --schematic`: a schematic that is not one flat KiCad schematic, or a schematic or board carrying what would pass parity unmatched (below). `emit`: an `--out` that cannot be created, or a file there that is a symlink or not a regular file. `simulate --firmware`, before Renode is looked for: a `--platform` that is not byte for byte the platform `aethelgard-twin` emits for `--spec`, or an `@` path; a firmware that is not ELF32 little-endian `EM_ARM` `ET_EXEC` with a sound program header table; a `PT_LOAD` whose physical or virtual range is outside the part's flash or RAM, two `PT_LOAD`s whose memory overlaps, or file bytes over the boot word at either address (below) | `UnreadableInput`, `InputTooLarge`, `InvalidJson`; `campaign`: `BaseOutsideRoot`, `RunExists`, `UnwritableOutput`, `ArchiveWriteFailed`; `campaign` also `FirmwareChanged`; `replay`, `attribute` and `propose`: every `aethelgard-chaos` archive refusal by its own name (`HashMismatch`, `ReplayDiverged`, `UnexpectedEntry`, …), `RunIdMismatch` and `NotACampaignRun`; `attribute` also `Undefined` or `MalformedSource`, a register fact the run names that the ST header lacks; `propose` also what `campaign` refuses of the scenario's file or base, `ScenarioMismatch`, `BaseChanged`, `TopologyChanged` and `UnwritableOutput`; `MalformedBoard`, `LocalClearance`, `BoardNetClass` and `BoardWaiver` (`id` naming the carrier, such as `pad 1 of footprint R7`); `MalformedSchematic`, `HierarchicalSchematic`, `ReservedNetName`, `BoardOnlyFootprint`, `NoConnectPadOnNet`, `SharedUnconnectedNet`; `UnwritableOutput`; `PlatformNotEmitted`, `MalformedFirmware` (overlapping memory included), `FirmwareOutsideMemory`, `FirmwareLoadsBootWord`; `build`: `CrateHasDependency`, `NotABoardCrate`, `UnwritableOutput`; `verify`: `ScenarioBaseMismatch` |
| `4` | The tool cannot be used: it is not on `PATH`, is not executable, will not report its version, or lacks an environment variable it needs; for `simulate --firmware`, a Renode other than the 1.17.0 the twin's platform is pinned to; the sandbox is unavailable; or there is no private place for scratch (below). Never a verdict (`INV-06`) | `ToolMissing` (`id`: the tool), `ToolUnusable` (`id`: the tool, or the missing variable), `ScratchUnsafe` (`id`: the directory that breaks the rule, or `HOME`), `SandboxUnavailable` (`id`: `landlock`, `seccomp`, or `descriptors`); `simulate --firmware`: `BridgeUnavailable` (`id`: `aethelgard-bridge`) when the bridge is not beside the `aethelgard` binary |
| `5` | The tool ran but its output cannot be trusted as a verdict: killed by a signal, an exit code outside the ones measured for it, a missing, unparseable or oversized report or output, or two witnesses that disagree. When `prove` or `simulate` cannot recognise its tool's outcome, the message ends `; the tool said: <line>`, every `error:` line (Kani, cargo) or the first `[ERROR]` line (Renode), as printable ASCII, cut to 400 bytes for cargo or 240 for Renode; for cargo it is every `error:` line joined by `|`. Never read as pass or fail | `ToolError` (`id`: the tool); `verify`: `BindingMismatch` (`id`: the gate), a gate that judged bytes other than the generated ones |
| `6` | `propose` only: the run's failure is a generator defect, and `generator-report.json` was written beside `proposal.json`. MCP reports it as a successful call, as it does `0` and `1` | none: stdout is `propose`'s line |
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

## `generate`

```
aethelgard generate --spec <path> --out <dir>
```

Reads, validates and solves exactly as `solve` does, with the same refusals, then writes everything the specification produces under `<dir>`, which must be empty or absent (`AET-WO-3301` §1.1):

- `kicad/`: the schematic, bill of materials and board, exactly as `emit` writes them;
- `board/`: the board crate `<project_name>-board` (`aethelgard_svd_hal::board::generate_with`, reading each I2C device the twin has a model for), its firmware crate in `board/firmware/`, and the Renode platform `board/<project_name>.repl` (`aethelgard_twin::platform::emit`).

`board/` is what `boards/<project_name>/` holds: from `examples/sentinel-01-minimal.json`, `generate` reproduces `boards/sentinel-01/**` and `emit`'s three files byte for byte. One function, `aethelgard_cli::generate::tree`, composes the generators; the regeneration test of `boards/` calls it too, so the two cannot drift. Nothing else is written and no process is started. Every directory and file is created through a descriptor of its parent, never through a symlink and never over anything that exists: a `<dir>` that is not empty, is a file or is a symlink is refused, exit `3`, `UnwritableOutput`. It prints one line, exit `0`:

```json
{"ok":true,"project_name":"sentinel-01","specification_sha256":"<64 hex>","topology_sha256":"<64 hex>","files":{"board/Cargo.lock":"<64 hex>",…,"kicad/sentinel-01.kicad_sch":"<64 hex>"}}
```

`topology_sha256` is the sha256 of the topology as `solve` prints it, without the newline: the hash `campaign` records.

## The tool subcommands

```
aethelgard check drc --board <file.kicad_pcb> [--spec <specification.json>] [--schematic <file.kicad_sch>]
aethelgard check erc --schematic <file.kicad_sch>
aethelgard prove --crate <dir> --harness <name> [--spec <specification.json>]
aethelgard build --crate <board-dir> --out <file.elf>
aethelgard simulate --platform <file.repl | @path-inside-renode>
aethelgard simulate --platform <emitted.repl> --firmware <elf> --spec <specification.json> --for <time>
```

Each runs one external tool on one input and prints one verdict. They are **Linux-only**, because the tools and the sandbox are. The published tools image (`container/IMAGE`) has all four tools; see [Running it](#running-it).

### What every call does, in order

1. **Checks the input path** (exit `3` if refused). It must be UTF-8, so the verdict can report it exactly. `--board`, `--schematic` and a `--platform` file must be regular files after following symlinks; `--crate` must be a directory. A `--platform` starting with `@` names a file inside Renode's own tree (the directory holding the real `renode` binary), as Renode's own `@` syntax does.
   `--harness` must be a Rust path: letters, digits, `_` and `::` (exit `64` otherwise).
2. **Finds the tool** by searching the caller's `PATH` (every entry must be absolute: a relative or empty one is exit `4`, `ToolUnusable`, naming the tool) for `kicad-cli`, `renode`, or `cargo` and `cargo-kani` (exit `4`, `ToolMissing`, naming the first one absent). `prove` also needs `KANI_HOME` and `RUSTUP_HOME` set to existing directories, and `PATH` (exit `4`, `ToolUnusable`, naming the variable).
3. **Copies the input** into a fresh private temporary directory (see [Where scratch lives](#where-scratch-lives)) and hashes the copy. The tool judges the copy, never the original, so the hash in the verdict is of exactly the bytes given (`INV-03`), and nothing is written beside the input. `check drc` and `check erc` then **prepare the copy** (below), and they do steps 3 and the preparation before step 2, so an input refused for what it carries is refused whatever is installed.
4. **Asks the tool its version**, then **runs it**, each time in the sandbox below. A tool that will not report its version is exit `4`, `ToolUnusable`.
5. **Reads two witnesses** (below), prints the verdict, and removes the temporary directory.

### Where scratch lives

Every call's temporary directory is made in one base (`AET-WO-1601` §1.4): `TMPDIR` when it is set, which must already exist; otherwise `/var/lib/aethelgard` when the CLI runs as root and `$HOME/.cache/aethelgard` for anyone else, made if missing. Cargo and rustup read `.cargo/config.toml`, `rust-toolchain.toml` and `Cargo.toml` from every ancestor of a tool's working directory, so a file another user could create above scratch could be read as configuration (`AET-WO-3301` F13). The base and each of its ancestors must therefore be a directory owned by root or by the CLI's own user and writable by no group and no other user; as root, each must also be searchable by others, so the tool uid can reach its own directory. Its real path must be spelled in `[A-Za-z0-9/._-]` alone, so no tool's command language (the Renode monitor's `@path`) reads a space or a `;` in it (`AET-WO-2100` F25).

A base that breaks any of this is refused before any tool runs, with exit `4`, `ScratchUnsafe`, naming the directory at fault; with neither `TMPDIR` nor an absolute `HOME` (as a user), the `id` is `HOME`. Nothing else is tried: `/tmp` (mode 1777) is refused, never fallen back to. A directory your umask made group-writable is refused too: set `TMPDIR` to one made with `mkdir -m 700`.

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

**With `--spec`**, the rules are the engine's floor until a specification rules field exists (AET-WO-4100), and the verdict says `"rules":"engine-floor"` and carries the file's `specification_sha256`. The specification declares no spacing or fabrication field, so today that is the engine's floor: 0.2 mm clearance and board minimum, and a 0.2 mm minimum track width (`aethelgard_eda::rules::FLOOR_NM`).

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

**`check erc` refuses a hierarchical schematic** (exit `3`, before KiCad runs; `AET-WO-1601` §1.5): a `(sheet …)` anywhere makes KiCad read the file the sheet names, which the gate has neither copied nor hashed (`HierarchicalSchematic`). The copy is read by the same strict reader as a parity schematic, so a schematic it cannot read is refused too (`MalformedSchematic`): a sheet could hide where the two readers disagree. `emit` never writes a sheet.

### The sandbox

Every tool process runs with:

- an argument array, never a shell, and every input given as an absolute path inside the temporary directory, so no file name can be read as a flag;
- **a separate uid when the CLI runs as root** (`AET-WO-1601` §1.2): the trampoline clears the supplementary groups and becomes uid `65534` (`nobody`) and gid `65533`, before Landlock and the filter, then reads every id back from the kernel; a switch that fails refuses the call with exit `4`, `SandboxUnavailable`, `id` `uid`, never a run as root. The input copy and every file the CLI writes beside it belong to root and are read-only (files `0444`, directories `0555`); the tool's directory belongs to root with group `65533`, mode `1770`, so the tool can create files in it but cannot rename or remove what root put there; for `prove`, the crate's root directory is the same, because cargo writes the `Cargo.lock` a crate without one needs there; the call's directory above it is `0710`, root and group `65533`, so no other user can even pass through. Gid `65533` is no account's group, unlike `65534`, which `sync` and `_apt` have in the image. As any other user, the tool runs under the caller's uid, as before. Uid `65534` is measured, not chosen: the analog bridge's libngspice crashes under a uid with no passwd entry, and the image lists no other unprivileged one but its `kicad` login user. A uid switch, a group change or a hand-over that fails refuses the call (exit `4`, `SandboxUnavailable`, `id` `uid`);
- **`RLIMIT_FSIZE` at 64 MiB**, soft and hard, so no tool can write a larger file into its RAM-backed scratch (`AET-WO-1601` §1.6, review A12.1). The largest file any gate writes on the reference is Kani's 2.2 MB goto binary; Renode's .NET would truncate a 2 TiB double-mapping memfd under W^X, so the CLI also sets `DOTNET_EnableWriteXorExecute=0` for Renode. A tool cut off at the cap ends as exit `5`, `ToolError`: killed by `SIGXFSZ` itself, or, when a child of it was (rustc under cargo), by the file it leaves at exactly the cap, whatever its own exit code says;
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
 "sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true,"uid_boundary":false},
 "included_severities":["error","warning"],
 "findings":[{"kind":"clearance","severity":"error","message":"Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)",
              "where":[{"description":"Track [NET_B] on F.Cu, length 20.0000 mm","x":105.0,"y":110.3,"uuid":"…"}]}]}
```

- `ok` is `true` exactly when the exit code is `0`; `verdict` is `pass` or `fail`; `check` is `drc`, `erc`, `prove` or `simulate`.
- `input.path` is the canonical path of the input as given (for an `@` platform, the file inside Renode's tree); `input.sha256` is the hash of the input's bytes as copied. For `check drc` the tool then judges that copy with the gate's setup block written over the board's, a function of those bytes alone (above).
- `tool.version` is the tool's own version line, exactly as printed; `tool.exit_code` is the exit code of the judging run.
- `sandbox.uid_boundary` is `true` only when the tool ran under the separate uid, switched to and read back by the trampoline because the CLI ran as root (`RUL-19`: a verdict claims only what it shows). It says who the tool ran as, and nothing more. **`false` leaves this open:** a process running as the caller's user can change what the tool reads between the gate's hash and the tool's read: a source file, a `.cargo/config.toml` in the tool's `HOME`, an ELF (`AET-WO-3301` F1). **`true` closes that to every other user, with four exceptions:**
  - root;
  - a process running as uid `65534` outside the sandbox. It can plant files in the tool's `HOME` and in `prove`'s crate root, and rewrite what the tool writes, but not change its input copy. Under Docker only root can start one;
  - **under rootless podman**, the host user who owns the user namespace (`forge` on Furnace). It can reach the tools through `/proc` and write their scratch, so there `true` excludes no process of that user;
  - **anyone who can write the toolchain.** In the pinned tools image, Kani's kit under `/opt/kani/kani-0.68.0` belongs to uid 1001, which the image's first `useradd` would get. Check a host or image with `find /opt /usr -xdev ! -uid 0`.
- `findings` is normalized to `{kind, severity, message, where}`. For KiCad, `kind` is the report's `type` (`clearance`, `track_dangling`, `pin_not_connected`, `label_dangling`, …), `severity` is `error` or `warning`, `message` is the report's `description`, and `where` lists the items it names. For `prove` and `simulate`, `kind` is one of the names in the table above, `severity` is `error`, and `where` is empty.
- `check drc` and `check erc` also carry `included_severities` from the report; `check drc` carries `rules`, with `--spec` `specification_sha256`, and with `--schematic` `schematic` (above); `prove` carries `harness` (the name), `harness_provenance` and `cover_count` (below); `build` carries `elf`, `{"path","sha256"}` of the ELF written; `simulate` carries `machine`: `{"cpus":["cpu"],"peripherals":[{"name":"cpu","type":"CortexM"},…]}`, and with `--firmware` also `boot` (below).

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
        "unobservable":["GPIOB.OTYPER","I2C1.TIMINGR"],
        "i2c":[{"reference":"U2","address":"0x76","register":"0xd0","expected":"0x60","observed":"0x60","status":"ok"}]}
```

**The I2C identity reads** (`AET-WO-2202`). The specification's I2C devices are placed on the peripheral their bus was solved to (`SolvedTopology::i2c_devices`, `AET-WO-2203`). For each device whose part has a Renode 1.17.0 model (today the BME280, `I2C.BME280`) the emitted platform declares the model at its bus and address, and the reference firmware reads the part's identity register once, through the board crate's bounded `transfer`, before it writes the boot word.
- **The record.** Each read leaves two words after the boot word, in device order: a status (`0x22020000` OK, or the wait that expired: `…01` TXIS, `…02` TC, `…03` RXNE, `…04` STOPF), then the bytes read, first byte most significant. Before the run the judge writes `0xFFFFFFFF` to every record word, as it does the boot word, so a record the firmware never wrote reads as that fill.
- **The expectation** is the part table in `aethelgard-ir` (`parts::identity`: BME280 register `0xD0`, `0x60`; TMP117 register `0x0F`, `0x0117`), never anything the firmware wrote.
- **Findings:** `i2c_identity_mismatch` (status OK, another value), `i2c_timeout` (naming the wait), `i2c_record_invalid` (a record no transfer writes: the fill, or another word). A record is judged only once the boot word is set; before that `boot_word_missing` already says the run failed.
- **`device_unmodelled`** has severity `info`: the device's part has no model in the twin (the TMP117 has none in Renode 1.17.0), so its identity was not read. It is reported on every boot and does not make the verdict `fail`; it also appears in the platform's header comment.

- **The controller witness** (`AET-WO-2202` W1). The record is the firmware's account, so for the last identity read on each bus the judge also reads the controller's `CR2` and `TXDR` after the run, which Renode 1.17.0 stores. The expectation is computed from the solved address, the part table and the header (`SADD` = address << 1, `RD_WRN` = 1, `NBYTES` = the identity's width, `AUTOEND` set, `TXDR` = the identity register; `START` and every other bit are ignored) and is judged only when that record claims the read completed. A difference is `i2c_transaction_unwitnessed`, naming the device and each field. The verdict's `boot.i2c_witness` lists both registers with their addresses, masks, expected and observed values.

**Named residual** (`AET-WO-2202` W1): only the last transaction on each bus is witnessed by controller state. Today each bus has one modelled device; a bus with two needs a per-transaction witness.

**Named residual** (`AET-WO-1900` A11.1, measured): the controller witness guards a mistaken generated firmware, not a hostile one. A hand-written firmware that writes the record and also stores the reference boot's final `CR2` (`0x020104EC`) and `TXDR` (`0xD0`) straight to `I2C1`, calling no `transfer`, is stored by Renode 1.17.0 and judged `pass` (`twin_tier2::a_firmware_that_writes_the_witness_registers_by_hand_passes_today_a_named_residual`). An observable a plain write cannot set is left to `AET-WO-2301`.

Every `observed` (and `mips`, `pc`) is `null` when nothing was read, as after a load error. `firmware.image_sha256` is the sha256, over every `PT_LOAD` with memory in physical-address order, of its physical address, file size and memory size (each a `u32` little-endian) and then its file bytes: the memory image Renode builds. It is the firmware's identity, because the file's own hash also covers symbol names, which Cargo derives from the checkout path.

### The analog pre-pass (`simulate --firmware`)

`AET-WO-2200` (`RUL-17`). Before Renode is looked for, each I2C line of the solved topology is measured, and the measurement decides the platform the boot loads:

1. `aethelgard-twin::analog` writes one netlist per line from the topology's `analog_lines`: the pull-up from the domain's rail, the line's capacitance, and the open-drain driver, released once in a 22.5 µs window (`AET-WO-2101` S2).
2. `aethelgard-bridge`, found **beside the `aethelgard` binary** (`BridgeUnavailable`, exit `4`, if it is not), measures each line's 30 %–70 % rise with libngspice 44.2 in its own process. It runs as a sandboxed tool like Renode, with the same 30 s and 1 GiB bounds, in a directory of its own, so it can write none of the copies Renode reads. Its rise time is its own interpolation of the points ngspice accepted, and it must agree with ngspice's `meas` to 0.01 ns.
3. Each line has one of three outcomes, each a property of the design (ruled on PR #70, F7 (b)):
   - **it rose**, in `rise_30_70_ns`. Past its ceiling (the net constraint's `max_rise_time_nanoseconds`), it fails the verdict with `analog_rise_exceeds_ceiling`;
   - **it was still rising when the window ended** (`complete: false`, with `rise_30_70_ns_lower_bound`, the window left after its 30 % crossing, or after the release if it never got there). That is always past the ceiling, so `analog_rise_exceeds_ceiling` states the bound as a lower bound;
   - **it was at or above 30 % when its driver released it**: an `analog.line_stuck_high` record with `volts_at_release`, and `analog_line_stuck_high`.

   Any of the three faults the line's bus: the platform the boot loads is the emitted one with that bus's devices left out (no declaration, modelled or not) and the decision, naming them, written into its header. Renode is then expected to list every solved peripheral and only the devices left in. That file is checked against the emission for the specification plus the recorded `faults` just before Renode starts (`PlatformNotEmitted`, exit `3`). `--platform` is still the platform emitted for `--spec` without any fault.
4. The boot runs and is judged as above. The firmware still reads each modelled device's identity, so a device left out by the fault times out at TXIS, and `boot.i2c` records it as `i2c_timeout`.

A bridge that exits nonzero, is killed, or answers anything but one of the three outcomes per line, in order, is `ToolError` (exit `5`, `id` `aethelgard-bridge`): no line is ever read as unfaulted. Exit `5` means the bridge itself failed, never the design.

The verdict carries the pass beside `boot`, never inside it: at 47 kΩ the reference gives three separate records, the measured rise (`analog`), the decision (`faults`) and U2's TXIS timeout (`boot.i2c`). The firmware sees a faulted bus only as its devices' absence, which Renode 1.17.0 renders as a TXIS timeout because it models no NACK; `firmware_sees` says so.

```json
"analog":[{"kind":"analog.rise_time_measured","source":"ngspice","bus":"SENSORS","net":"I2C_SCL","complete":true,
           "rise_30_70_ns":3982.300059495949,"ceiling_ns":300,"exceeds":true,"method":"interpolated","ngspice":"44.2"},…],
"faults":[{"kind":"bridge.fault_state","source":"aethelgard","bus":"SENSORS","fault":"absent",
           "from":["analog.rise_time_measured > ceiling_ns"],"nets":["I2C_SCL","I2C_SDA"],"firmware_sees":"…"}]
```

### Harness provenance (`prove`)

Each gate binds only its own input (`RUL-19` (2)): `prove` judges the crate it is handed, and a crate's own harness can pass vacuously. So every `prove` verdict says which it judged (`AET-WO-3301` §1.4):

- `harness_provenance` is `generated` when `--spec` is given and the crate as copied is byte for byte the board crate `generate` writes for that specification: every file, none missing and none extra (a top-level `target/` is not part of a crate). Otherwise, and always without `--spec`, it is `supplied`. The whole crate is compared, not only `src/proofs.rs`, because an unchanged harness over rewired code is not the generated proof.
- `cover_count` is the number of cover properties Kani reported for the harness, `0` when it reported none.
- A `--spec` that `solve` or `generate` refuses is refused as they refuse it, before any tool is looked for.

### Building firmware (`build`)

`build --crate <dir> --out <file>` builds the firmware ELF of a board crate: `<dir>` is the board crate `generate` writes, whose `firmware/` crate depends on it by `path = ".."`. The crate is copied and checked before any tool is looked for: it must hold `firmware/Cargo.toml` (else exit `3`, `NotABoardCrate`), and neither `Cargo.lock` nor `firmware/Cargo.lock` may name a package with a `source`, a registry or git dependency (exit `3`, `CrateHasDependency`, `RUL-04`). `--out` must not exist (exit `3`, `UnwritableOutput`).

Cargo then runs in the sandbox, as `prove` runs it: `cargo build --release --offline --locked --target thumbv7em-none-eabihf --config profile.release.strip="symbols" --message-format=json-render-diagnostics` in the copy's `firmware/`, with only `RUSTUP_HOME`, `PATH`, `CARGO_TARGET_DIR` and, when the caller set it, `RUSTUP_TOOLCHAIN` (exit `4` naming `RUSTUP_HOME` or `PATH` if unset). Two witnesses decide: cargo's exit code and its `build-finished` message. Exit `0` with `build-finished` `success: true` and exactly one executable is a pass: the ELF is written to `--out` and named in `elf`. Exit `101` with `success: false` is a fail, `build_failed`, naming cargo's first `error:` line. Anything else, such as a cargo configuration cargo cannot load (exit `101` with no `build-finished`), is `ToolError`. A pass whose units were not all built from manifests inside the copy (a path dependency outside the crate, a `[patch]` or `paths` override), or that read any source outside it as cargo's dep-info beside the ELF lists them (a `#[path]` module, an `include_bytes!`, a `[lib] path`), each path resolved before it is compared, or that ran a build script, is refused, exit `3`, `CrateHasDependency`, and no ELF is written. A linker input named in a supplied crate's own rustflags (`-T`, `-C link-arg`) is not in dep-info and is not refused; it can come only from what the sandbox lets cargo read, the tool installation and the system directories (FEEDBACK F12).

**The generated crate's ELF is the same wherever the crate is.** Cargo hashes a path dependency's absolute path into every symbol name, so an unstripped build differs by path in `.strtab` alone (`AET-WO-3200` F11). `--remap-path-prefix` does not reach those hashes; stripping does, and it leaves every loaded segment, so `simulate`'s `image_sha256`, unchanged (both measured). The strip is on cargo's own command line, and the crate's manifest profile does not override it. A supplied crate can still make its ELF path-dependent, with rustflags in its own `.cargo/config.toml` (they come after the strip) or with `env!("CARGO_MANIFEST_DIR")` in its code.

### The crate hash

`prove`'s and `build`'s `input.sha256` covers the crate's file tree. For every regular file beneath the crate directory, except under a top-level `target/` directory, take the line `<sha256 of the file, lowercase hex>  <path relative to the crate, with / separators>\n`. Sort the lines by path, byte order, and hash their concatenation. A crate holding a symlink, a special file or a name that is not UTF-8 is refused with exit `3`. The same tree, minus `target/`, is what is copied and judged.

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

### Running `aethelgard` behind a uid boundary

As root, as in the published image, the CLI runs every tool under a separate uid on a root-owned copy, and verdicts say `"uid_boundary":true`. Elsewhere it runs tools as its caller, and a process of that user can race any gate (above). A host that wants the boundary without running the CLI as root sets it up this way; nothing here is done by the CLI or by this repository:

1. **A dedicated uid for the CLI and its MCP server.** Run `aethelgard` and `aethelgard mcp` as a user that runs nothing else, with its own home, and `TMPDIR` unset or private to it.
2. **A toolchain that user cannot write, and nor can the client.** The tools, `RUSTUP_HOME`, `KANI_HOME` and Renode belong to root (or to the CLI's uid, never the client's). A toolchain the client can write is a toolchain the client can replace, whatever the sandbox does. `find /opt /usr -xdev ! -uid 0` should print nothing, and in the pinned tools image it does not (Kani's kit belongs to uid 1001).
3. **The MCP client as a different uid.** The client reaches the CLI only through `mcp`'s stdin and stdout, and hands it input files the CLI's uid can read. It can then neither write the CLI's scratch nor read it.

The CLI cannot see this set-up, so its verdicts still say `"uid_boundary":false`: they under-claim, which `RUL-19` prefers to over-claiming. Without all three, the residual above is the client's own identity (`RUL-19` (4)).

## `campaign` and `replay`

`AET-WO-2300`: the P2 gate's fault campaign, its archive, and replay from the archive alone (`INV-04`).

```
aethelgard campaign --scenarios scenarios/sentinel-01/pullup-2k2.json scenarios/sentinel-01/pullup-47k.json \
                    --firmware <elf> --archive <dir>
aethelgard replay --archive <dir>/<run id>
```

**A scenario** is a strict JSON file authored apart from anything generated (`INV-07`): its format and its overrides are `crates/aethelgard-chaos/README.md`'s "Scenarios". Its `base` is the specification it starts from, relative to the scenario file's directory. It must not be absolute, and once every symlink is resolved it must be a regular file **inside the current directory** (`BaseOutsideRoot`, exit `3`), read under the same 16 MiB cap and refusals as `validate`. The file is opened once, without following a symlink or blocking on a FIFO, and where that descriptor really is (`/proc/self/fd`) is what is checked against the current directory and then read, so a path swapped after the check is never read. So run a campaign from the repository root. The overrides change the specification and nothing else; the runner solves the result. It never reads or edits a generated file. The two reference scenarios are `scenarios/sentinel-01/`.

**`campaign`** first reads the firmware's hash and every scenario: it parses, applies, validates and solves each one, and refuses (`DuplicateScenario`) two that would name one run. It creates `--archive` if absent and refuses a run directory that already exists (`RunExists`). Any refusal is printed before anything runs. Then, for each scenario in the order given:

1. It writes the platform `aethelgard-twin` emits for the overridden specification and runs exactly `simulate --firmware`'s path on it, with the same sandbox, bridge, bounds and judge, booting for **100 ms**, the longest `--for` admits, so a timed-out read still reaches the boot word. A held-low line (`line_held_low`) is not sent to the bridge: its bus is faulted without a measurement, with `i2c_line_held_low` and a `bridge.fault_state` whose `source` is `scenario <name>`.
2. It archives every record of the verdict, as integer-only CBOR events, in a new run directory `<archive>/<run id>`: the scenario's sha256 and overrides, the overridden specification's and the solved topology's sha256, each `analog.rise_time_measured` (`rise_30_70_ps`, `complete`) and `analog.line_stuck_high`, each `bridge.fault_state`, the boot's witnesses, every register, each I2C identity read and controller witness, and the verdict. The header's `artifacts` are the `base` file's, the `firmware`'s, the `scenario`'s, the overridden `specification`'s and the `topology`'s hashes. A firmware whose bytes changed between the campaign's read and the boot's copy is refused (`FirmwareChanged`, exit `3`). The run is committed before its line is printed.
3. It prints one line:

```json
{"scenario":"pullup-47k","file":"scenarios/sentinel-01/pullup-47k.json","verdict":"fail","expect":"fail","matched":true,
 "findings":["analog_rise_exceeds_ceiling","i2c_timeout"],"expect_findings":["analog_rise_exceeds_ceiling","i2c_timeout"],
 "run_id":"011cc38d…","run":"<archive>/011cc38d…","state_sequence_hash":"0483991c…","event_count":16,"trace_bytes":1335}
```

`matched` is true when the verdict is the one the scenario expects and, for a failure, its finding kinds above `info` are exactly `expect_findings`: a scenario that fails for another reason, or for more, does not match. The last line is the summary, `{"ok":…,"verdict":…,"check":"campaign","scenarios":n,"matched":m,"virtual_time_ns":100000000,"trace_bytes":…}`, with exit `0` only when every scenario matched, and `1` otherwise. A tool that stops one scenario (exit `4` or `5`) stops the campaign with its refusal; runs already committed stay. A campaign therefore asserts what should fail.

**`run_id`** is a function of the scenario's, the overridden specification's and the firmware's hashes, so the same campaign twice names the same runs, with the same state-sequence hash: into one archive the second is `RunExists`, since the archive is append-only.

**`replay`** reads one run directory through `aethelgard-chaos`'s strict reader and nothing else: no solver, no tool, no scratch directory, no file outside the run directory, no network. It recomputes the state-sequence hash, the chain `crates/aethelgard-chaos/README.md` defines over every event's payload (no timestamp, no wall-clock field), and refuses a run whose hash differs from its manifest's (`ReplayDiverged`), and every other refusal of that reader by its name, with exit `3`. The reader opens the run directory once and reads every file through it, never following a symlink or opening a FIFO. A directory not named by the run id its header holds is `RunIdMismatch` (exit `3`): a run copied under another run's name is not that run. It then judges the records (`NotACampaignRun`, exit `3`): no event but campaign records, a header whose hashes and run id are the ones its records give, the records' order, and that every fault, measurement and verdict agrees with the evidence beside it. It prints one line, exit `0`, and two replays of one run print the same bytes:

```json
{"ok":true,"check":"replay","run_id":"011cc38d…","scenario":"pullup-47k","verdict":"fail","expect":"fail","matched":true,
 "findings":[…],"expect_findings":[…],"artifacts":{"base":"…","firmware":"…","scenario":"…","specification":"…","topology":"…"},
 "event_count":16,"trace_bytes":1335,"state_sequence_hash":"0483991c…"}
```

## `attribute`

`AET-WO-3100`, the P3 gate's first clause: one archived campaign run attributed to the electrical half or the software half, with the rule that decided it and the archive events that prove it (`INV-05`).

```
aethelgard attribute --archive <archive>/<run id>
```

It first applies exactly `replay`'s checks, in `replay`'s order, and refuses whatever `replay` refuses with the same line and exit code. Then `aethelgard_evolve::attribute` reads the run's records and the vendored ST header through `aethelgard-svd-hal`. It reads no other file, starts no process, needs no tool on `PATH`, and two calls on one run print the same bytes. It prints one line, exit `0` for `electrical`, `software` or `none` and `1` for `indeterminate`:

```json
{"ok":true,"check":"attribute","run_id":"…","state_sequence_hash":"…","domain":"software","rule":"S1",
 "evidence":[{"index":3,"kind":7},{"index":4,"kind":7},{"index":14,"kind":13}],
 "symptoms":[{"index":13,"kind":12}],"masked":[],"unexplained":[]}
```

`index` is the event's position in `trace.cbor` (the header is not an event), and `kind` is its kind (`crates/aethelgard-chaos/README.md`). The record holds:
- `symptoms`, the failing events;
- `evidence`, the events that prove the cause;
- `masked`, software evidence on an electrically faulted bus, listed and never used;
- `unexplained`, failing finding kinds that no archived event shows.

**Symptoms.** The verdict's finding kinds above `info` choose which events need explaining. They are never evidence. Each kind selects its events, and the count must equal the finding's, summed over every severity above `info`, or the kind is unexplained. The converse holds too: an event that some kind below would select, but that no finding names, is unexplained as `event <index>`. So rewriting the verdict alone never moves a run's domain; at most it makes the run `I1`.

| finding | its events |
|---|---|
| `analog_rise_exceeds_ceiling` | kind 7 past its ceiling |
| `analog_line_stuck_high` | kind 8 |
| `i2c_line_held_low` | kind 9 caused by `scenario.line_held_low` |
| `i2c_timeout`, `i2c_record_invalid` | kind 12 whose status is a `timeout_…`, or `invalid` |
| `i2c_identity_mismatch` | kind 12 whose status is `ok` and whose value is not the part table's |
| `i2c_transaction_unwitnessed` | kind 13 whose read's status is `ok` and whose `CR2` or `TXDR` differs from its expectation |
| `register_mismatch` | kind 11 that differs from its expectation under its mask |
| `boot_word_missing`, `cpu_halted` | kind 10 without the boot word, or with fewer instructions than expected |
| `locked_peripheral_access` | kind 11 in RCC with an expected enable bit clear, at least one: the same fact, archived (the count is not compared, since the judge reports one finding for any number of accesses) |
| anything else (`renode_error_log`, a reaper bound, `pc_outside_firmware`: executable segments are not archived) | none: unexplained |

**Where a symptom is.**
- Kinds 7, 8 and 9 are on their bus.
- Kinds 12 and 13 are on the bus whose kind 15 lists their device, for that device.
- Kind 11 is on a bus when it is wrong only in fields that bus owns. A bus owns its controller's clock gate, the gates of its pins' ports, and each pin's `MODER` and `AFR` field. Each name and position comes from the ST header, and the solved alternate function comes from kind 15.
- Kind 10, and any kind 11 wrong elsewhere, is on the board.

**Software evidence on a bus.** For a symptom of one device, only that device's own witness counts, since another device's transaction says nothing of this one's. The bus's owned registers count for every symptom on it.
- The controller witness (kind 13) differs from its solved device. The witness must be read at its controller's `CR2` and `TXDR` addresses, from the header.
- `CR2.SADD` differs from the device's address shifted left by one. Both phases of the transfer write it. It counts only when the firmware wrote `CR2`, i.e. `SADD` or `NBYTES` is not 0; a `CR2` at its reset value says nothing of an address.
- `NBYTES` differs from the witness's own expectation. It counts only once the record's status shows the read phase began (`ok`, `timeout_rxne`, `timeout_stopf`): before that, `CR2` holds the write's `NBYTES`.
- `TXDR`'s byte differs from the read's register. It counts only once the write phase ran (`ok`, `timeout_tc`, `timeout_rxne`, `timeout_stopf`): on a TXIS timeout `TXDR` was never written.
- A witness whose own expectation is not the solved address and register is no evidence either way.
- An owned register field is wrong. A pin's `MODER` field must hold alternate-function mode and its `AFR` field kind 15's alternate function; both come from kind 15, never from the reading's own expectation, and a reading whose expectation disagrees is no evidence.

**Software evidence on the board.** The boot word is missing, the core executed fewer instructions than expected, or a register is wrong outside every bus's fields.

**A bus is measured healthy** when no kind 7 past its ceiling, no kind 8 and no kind 9 is on it, each of its two lines has its kind 7 (R2 allows one per line), and the ST header knows its controller and pins. A bus whose port or controller the header lacks is never healthy and owns nothing, so its symptoms are `I2` and no other bus's attribution is refused.

**The rules, closed and ordered.** Each symptom is decided by the first rule that holds for it:

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `E1` | electrical | the symptom's bus has a kind 7 past its ceiling, a kind 8 or a kind 9; its software evidence goes to `masked` | 7, 8, 9 |
| `I2` | indeterminate | the symptom's bus is not measured healthy (a line unmeasured, or a bus the header cannot join) | 15 |
| `S1` | software | the symptom's bus is measured healthy and has software evidence | 7, 11, 13 |
| `I3` | indeterminate | the symptom's bus is measured healthy with no software evidence, and another bus is faulted | 7, 8, 9 |
| `I4` | indeterminate | measured healthy, no software evidence, and no fault anywhere | 7 |

For a symptom on the board, any faulted bus gives `I3`. No bus at all, or any bus not measured healthy, gives `I2`. Then board software evidence gives `S2` (software; evidence kinds 7, 10, 11), and none gives `I4`.

**The run.**
- A passing verdict is `none`, rule `N1`.
- An unexplained finding is `I1`, with no evidence and the kinds listed in `unexplained`.
- Otherwise, if any symptom is indeterminate, the run is indeterminate by the lowest such rule, with that rule's evidence.
- If symptoms are split between electrical and software, it is `I5`, with both halves' evidence: never averaged.
- Otherwise the run takes the one domain and the lowest rule that decided any symptom. Its evidence is that rule's alone, so every evidence kind is one the named rule allows. With `S1` and `S2` both deciding, the run is `S1`, and its `symptoms` still list the board's.

Software is never concluded by eliminating electrical, nor electrical by eliminating software.

**The seeded set** is `scenarios/sentinel-01/seeded/cases.json`. It holds 20 failing cases, 10 in each domain, and two passing ones.
- The electrical cases are scenarios run against the generated firmware.
- The software cases are named exact-once patches of `boards/sentinel-01`, built from a scratch copy by `tests/attribute_seeded.rs`. A patch that matches zero or two times fails that test, and so does a generator change.
- The trap: a wrong device address fails with `i2c_timeout` at TXIS, as a 47 kΩ pull-up does. It is `software` with 3.3 kΩ (measured inside the ceiling), and `electrical` with 47 kΩ, with the witness in `masked`.

## `propose`

`AET-WO-3200`, the P3 gate's second clause: one archived campaign run, attributed, turned into a proposed specification change, a generator bug report, or a stated refusal (`RUL-07`, `RUL-18`). It never applies a change to anything but the copy it writes, never merges, and never edits a gate, a ceiling or a constraint.

```
aethelgard propose --archive <archive>/<run id> --scenario <file> --out <dir>
```

**It checks first,** and refuses on any failure, writing nothing:
- `attribute`'s checks, and with them `replay`'s, with the same line and exit code;
- the scenario file's sha256 and overrides are the run's kind 5 (`ScenarioMismatch`, exit `3`);
- its `base` resolves exactly as `campaign` resolves it, inside the current directory, and its bytes hash to the run's archived `base` (`BaseChanged`, exit `3`);
- the base with the overrides, applied, serialized and solved by `campaign`'s own code, gives the run's kind 6 specification and topology hashes, and the kind 15 records `campaign` writes from that topology are the run's (`TopologyChanged`, exit `3`).

Then it attributes the run (`attribute`), reads each evidence event through `aethelgard_evolve::view`, and hands the rules proposer the solved facts of each bus from the verified topology: the pull-up and rail both lines carry, and the pull-up floor at the bus's speed and rail from `aethelgard_ir::electrical`, the floor `solve` refuses below.

**The rules proposer,** a closed table (`aethelgard_evolve::propose::Rules`). Proposers run in a fixed order and the first that answers wins; today there is one.

| attribution | answer |
|---|---|
| `E1`, every piece of evidence a complete rise past its ceiling (kind 7) or a bus fault caused only by one (kind 9) | a proposal per faulted bus, sized below |
| `E1` with a line held low, a line stuck high, an incomplete rise, a bus fault of any other cause, or a line of the bus without exactly one complete rise | no rule: `no rule; RUL-18 model proposer slot`. A held line is a board fault no value change fixes |
| `S1`, `S2` | a generator report, with no specification change |
| `N1`, `I1` to `I5` | nothing is proposed, naming the rule: attribution precedes mutation (`INV-05`) |

**Sizing a pull-up.** The binding line is the one with the least `ceiling / rise`, which is the slowest line when the ceilings are equal. `R_target = R_now × ceiling × 0.8 / rise`, rounded down; the 0.8 is headroom for a ±5 % resistor tolerance and a capacitance spread the run did not measure. The value is the largest E12 value at or below `R_target`. If it is below `Rp_min`, the floor, nothing is proposed, never a value below it. The predicted rise is the binding line's rise scaled by the new value over the old, rounded up (the rise is linear in R × C, `docs/architecture.md` §2.18). Every rise is computed from declared values (`RUL-19`).

**What any proposer may answer.** A change is `{"bus", "pull_up_ohms"}` and nothing else: the scenario override form. The other two override forms describe a fault, never a fix, and a constraint the gates judge is never a lever (`INV-07`). Whatever proposer answers, a proposal stands only for an `electrical` run, with one change per solved bus at or above its floor, and a generator report only for a `software` run; anything else becomes a refusal naming the proposer.

**What it writes,** into `--out`, which must be absent or an empty directory (`UnwritableOutput`, exit `3`, otherwise; a symlink is refused). The directory is opened once without following a symlink, and each file is created new inside it. Every byte is a function of the run, the scenario's bytes and the base's bytes: never of a path. Two calls give the same files.
- `proposal.json`: the outcome, the proposer, the reason when nothing is proposed, the run id and state hash, the scenario by name and sha256, the base, specification and topology hashes, the attribution (domain, rule, evidence), the changes, the sizing numbers (`R_now`, the binding line, its rise and ceiling, the headroom, `R_target`, `Rp_min`, the E12 value and `predicted_rise_ps`), and the proposed specification's sha256.
- With a proposal, `proposed-spec.json`: the run's effective specification with the changes applied through `campaign`'s own override path, then validated.
- With a generator report, `generator-report.json`: the run, the rule and its evidence, and for each unmet solved expectation its event, the register field (or the instruction count), the solved and observed values, and the generator it belongs to: `firmware` for a controller witness or the boot, `board_crate` for a register.

It prints one line, and exits `0` with a proposal, `6` with a generator report, `1` when nothing is proposed:

```json
{"ok":true,"check":"propose","outcome":"proposal","proposer":"rules","run_id":"…","out":"…",
 "files":{"proposal.json":"<sha256>","proposed-spec.json":"<sha256>"}}
```

It reads the run directory, the scenario and its base, and the vendored pin table and ST header; it starts no process. A proposal is evidence about the specification, not the board, and the gates judge it like any other specification: `tests/attribute_seeded.rs` takes each `proposed-spec.json` through `validate`, `solve`, `emit`, ERC, DRC with schematic parity, the generated board crate's proof, and a campaign whose run `attribute` names `N1`. A proposal committed under `proposals/` re-runs to its own bytes there.

## `verify`

```
aethelgard verify --spec <path> [--scenarios <file>...] [--against <dir>]
```

One verdict that binds a specification to every artifact it produced (`INV-01`, `INV-03`, `RUL-19` (2), `AET-WO-3301` §1.3). Only `verify` binds a pair; every other gate binds only its own input.

1. **Generate in memory.** Read, validate and solve `--spec` as `solve` does, and generate the tree as `generate` does. Every hash the verdict names comes from these bytes.
2. **Check the scenarios.** Each is read as `campaign` reads it, and its `base` must be byte for byte `--spec` (exit `3`, `ScenarioBaseMismatch`).
3. **`--against <dir>`**: compare `<dir>` with the tree byte for byte (`board/target/` is skipped, as a crate's `target/` is; a symlink is never followed and is a difference). Any differing, missing or extra file is a fail, exit `1`, with one finding each (`against_differs`, `against_missing`, `against_extra`, the path first in `message`), and no gate runs. This is `INV-01`'s "regenerate and diff", runnable by a user.
4. **Run every gate on the tree**, written into a guarded scratch directory, each as this binary's own subcommand: `check erc` on the schematic; `check drc --spec --schematic` on the board; `prove --spec` on the board crate, once for each harness of its `src/proofs.rs`; `build`; `simulate --firmware` with the generated platform for `100ms`; and, with `--scenarios`, `campaign` over them with that ELF into a scratch archive.
5. **Bind each gate to the bytes.** The hash each gate reports for what it judged must be the generated one: the schematic, the board, the parity schematic and the specification (ERC, DRC); the crate hash of `board/`, and `harness_provenance` `generated` (`prove`, `build`); the platform, the ELF `build` reported and the specification (`simulate`); and each run id, recomputed from the scenario, `--spec` with its overrides and the ELF (`campaign`). Anything else is exit `5`, `BindingMismatch`, naming the gate: a file swapped in `verify`'s own scratch, or a gate that reports judging other bytes, is never named in a verdict.

**What `verify` does not bind.** Each gate hashes its own copy and then hands the copy's path to its tool, as every gate does. A process of the same user that writes into a gate's private scratch in between (a source file before cargo reads it, a `.cargo/config.toml` in the tool's `HOME`, an ELF before Renode loads it) changes what the tool judges while the gate still reports the generated hash. The adversarial pass made `verify` pass that way (`AET-WO-3301` FEEDBACK F1). `verify` binds a pair against everything but a concurrent process with the user's own identity, which by `RUL-19` (4) can also replace the tools themselves. Each gate entry carries the gate's `uid_boundary` (the campaign's only when every run's is `true`; `null` for a gate that did not run): where it is `true`, that window is closed to every other user but the four exceptions under [The verdict](#the-verdict) (`AET-WO-1601`).

A gate that refuses stops `verify` with the gate's own `kind`, `id` and exit code, its `message` prefixed with the gate's name (`INV-06`). Otherwise it prints one line, exit `0` when every gate passed and `1` when any failed:

```json
{"ok":true,"verdict":"pass","check":"verify","engine_version":"0.1.0","project_name":"sentinel-01",
 "specification_sha256":"<64 hex>","topology_sha256":"<64 hex>","files":{"board/Cargo.lock":"<64 hex>",…},
 "against":"identical","elf_sha256":"<64 hex>","rise_times":"computed from declared values",
 "gates":[{"gate":"erc","verdict":"pass","exit_code":0,"tool_version":"10.0.7","uid_boundary":true,"input_sha256":"<64 hex>","findings":[]},…,
          {"gate":"prove","harness":"proofs::every_valid_sequence","verdict":"pass",…,"harness_provenance":"generated","cover_count":9,"findings":[]},…],
 "campaign":[{"scenario":"pullup-2k2","verdict":"pass","expect":"pass","matched":true,"findings":[],"run_id":"<32 hex>","state_sequence_hash":"<64 hex>"}],
 "findings":[]}
```

- A gate after a failed `build` is `not_run`, with `exit_code` `null`.
- **The campaign gate passes only if every run's own verdict is `pass` and matched.** A scenario that expects a fail can never let `verify` pass: the 47 kΩ scenario's campaign exits `0`, but `verify` fails with its findings (`analog_rise_exceeds_ceiling`, `i2c_timeout`).
- `rise_times` says what the analog records are (`RUL-19` (1)): computed from the declared pull-up and capacitance, never measured on a board.
- The verdict names no path in scratch, so two runs on the same inputs print the same bytes.
- `verify` takes as long as its gates: about 2 minutes 15 seconds for the reference with one scenario, inside the tools image on Furnace, most of it the two proofs.

## `mcp`

`aethelgard mcp` serves the other subcommands as [Model Context Protocol](https://modelcontextprotocol.io) tools on stdin and stdout (`RUL-10`, built on `rmcp` 3.7.0 with its `server` and `transport-io` features only). It takes no arguments, listens on no socket, holds no credential, and exits `0` when stdin closes. Stdout carries protocol messages and nothing else; diagnostics go to stderr. It is not a tool itself and adds no tool behaviour.

Before it starts, it runs the trampoline's inherited-descriptor check. A descriptor above 2 without close-on-exec refuses with exit `4`, `SandboxUnavailable`, `id` `descriptors`; the refusal object goes to stderr, since stdout is reserved for the protocol. A `SIGTERM`, `SIGINT` or `SIGHUP` kills the running calls and exits `143`, `130` or `129`, from the first moment, handshake or not. Closing stdin before `initialize` also exits `0`. `SIGKILL` cannot be handled; each running call's CLI then dies by its parent-death signal, and its reaper and guardian clean up.

### The tool list

Generated at startup from the unbuilt `clap` command tree, never written by hand, so a new subcommand appears with no change to the `mcp` module (`INV-01`):

- one tool per leaf subcommand, named by its path joined with `_`: `validate`, `solve`, `emit`, `generate`, `build`, `verify` (whose `scenarios` is repeatable and optional), `check_drc`, `check_erc`, `prove` (whose optional `spec` gives `harness_provenance`), `simulate` (whose optional `firmware`, `spec` and `for` are the boot), `campaign`, `replay`, `attribute` and `propose`. `help` and `mcp` are never tools;
- the description is the subcommand's about text;
- the input schema is an object with one **string** property per declared argument, keyed by its **long name** (`prove`'s directory is `crate`, not `crate_dir`), `required` exactly when `clap` requires it, and `additionalProperties` false. A repeatable argument (`campaign`'s `scenarios`) is a non-empty **array of strings** instead.

Every argument must be a long-named, value-taking option, set once or repeatable. Anything else (a positional, a boolean flag) makes the tool list refuse to build, which a unit test catches before a client can.

### A call

The server checks the arguments against the schema (declared names only, string values only, or a non-empty array of strings for a repeatable argument, required ones present, no NUL byte). A failure is an MCP invalid-params error (`-32602`) and nothing runs; so is an unknown tool name. Otherwise it runs this binary itself as a child, with stdin from `/dev/null`, and the argument vector is the subcommand path followed by one `--<long>=<value>` token per argument, and per item of a repeatable one. One token per argument means a value starting with `-` can never be read as a flag; the CLI judges it. The internal first arguments are not in the tree, so no call reaches them. The call therefore has exactly the CLI's sandbox, exit codes and JSON.

`validate` reads stdin when `file` is absent, and the child's stdin is empty, so pass `file`: without it the call is refused as empty input (exit `3`).

### The result

For a child that exits and prints exactly one JSON object line:

- `content[0].text` is that line, unchanged, without its newline;
- `structuredContent` (its object keys sorted, so not byte-identical to the text) is `{"exit_code": <n or null>, "verdict": <the child's object>}`. `null` means the child was killed by a signal;
- `isError` is `false` for exit `0` and `1`, because a fail verdict is a successful call, and `true` for every other exit code. A refusal, such as `ZeroNominalVoltage` (exit `2`), is therefore `isError` `true` with the refusal object as the verdict.

`campaign` alone prints one line per scenario before its summary (`LINE_PER_RECORD` in `main.rs`): every line must be a JSON object, `verdict` is the last, and `structuredContent` adds `"records"`, the lines before it. `content[0].text` is then every line.

`isError` is `true` for exit `null` as above. It is also `true`, with no `structuredContent` and a text naming the problem, when stdout is not exactly one JSON object line (for `campaign`, one or more) (a child killed before it printed one, for example), and when it exceeds a cap (the server kills the child, so a legitimate verdict over 1 MiB is reported as the cap): stdout 1 MiB, stderr 64 KiB. Past a cap the server kills the child and keeps draining until the pipes close.

### Cancellation and bounds

A `notifications/cancelled` for a call kills its child; the CLI's reaper then ends the tool's whole process group. Closing stdin lets running calls finish for up to 5 s (`rmcp`'s grace), then cancels them. The server sets no timeout of its own (`RUL-13`); only `simulate --firmware` bounds its tool, with the wall bound and memory cap above. `rmcp` drops a JSON-RPC line over 16 MiB without replying. The number of concurrent calls is not bounded by this server.

### Human check

```
npx @modelcontextprotocol/inspector aethelgard mcp
```

lists the nine tools, and a `validate` call with `file` set to `examples/sentinel-01-minimal.json` returns `isError` false.
