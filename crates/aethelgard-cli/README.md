# aethelgard-cli

The `aethelgard` binary. It is the contract every caller uses (`docs/architecture.md` `RUL-10`): exactly one JSON object on one line of stdout, and the exit codes below. The one exception is `campaign`, which prints one line per scenario before its summary line (`AET-WO-2300`). Diagnostics go to stderr. `AET-WO-1500` inherits these codes; changing one is a breaking change.

The repository `README.md`'s `## Quickstart` runs `emit`, `check erc`, `check drc` and `solve` from the published engine image.

## Exit codes

| Code | Meaning | `error.kind` values |
|---|---|---|
| `0` | `validate`: the specification is valid. `solve`: it was solved, and stdout is the `SolvedTopology`. A tool subcommand: the tool ran and its verdict is pass. `campaign`: every scenario's verdict matched its expectation. `replay`: the run replayed to its committed hash. `attribute`: the run is attributed `electrical`, `software` or `none`. `propose`: a proposal was written. `generate`: the tree was written. `verify`: every gate passed, and with `--fab-out` the package was written, or with `--against-fab` it matched. `extract`: every I2C, SCK and MOSI line's bound is at most its declared bound. `check speeds`: every SCK and MOSI is served by `00` or `01`. `doctor`: nothing is missing | none: stdout is the success report, the `SolvedTopology`, or the verdict |
| `1` | A tool subcommand: the tool ran and its verdict is fail (violations found, the proof failed, a declared cover was never reached, the crate did not compile, the harness does not exist, the platform did not load). The input is at fault; the tool worked. `campaign`: every scenario ran and was archived, and at least one verdict did not match its expectation. `attribute`: the run is `indeterminate`. `propose`: nothing is proposed, and `proposal.json` says why. `build`: the crate did not build (`build_failed`). `verify`: a gate's verdict is fail (with `--fab-out`, nothing is written), or `--against` differs from the generated tree, or `--against-fab` from the derived package. `extract`: a line's bound exceeds its declared bound. `doctor`: something a subcommand needs is missing, named in `missing` | none: stdout is the verdict, the attribution, or `propose`'s line |
| `2` | Invalid specification: well-formed JSON of the wrong shape (unknown, missing or duplicate field, wrong type including deep nesting where a value was expected, negative or overflowing number, an enum's object left `{}`), or a value `validate()` refuses. `solve` also: a part with no pin table, or a specification the solver cannot realize. `emit` also: what `solve` refuses, and a part with no vendored model, an address the part cannot take, a reference used twice, a net, rail or reference the emitter needs for itself, a net the solver did not place, a pin the MCU's symbol lacks, or a board the router cannot complete. `check drc --spec`: the specification refused as `validate` refuses it. `simulate --firmware`: the specification refused as `solve` refuses it, or a solved topology the twin cannot lower to a platform. `extract`: the specification refused as `solve` or `emit` refuses it, or a design that puts a passive other than a pull-up on an I2C line (`UnmodelledPart`), or a device whose part cites no maximum pin capacitance on SCK or MOSI (`UnboundedPinCapacitance`). `check speeds`: what `solve` refuses, and a line no selectable speed serves (`RateAbovePinSpeed`, `LoadAboveSpeedTable`, `RiseTimeBelowPinSpeed`, `SupplyOutsideSpeedTable`), which `generate` refuses too | `SchemaViolation`; `VariantMissing`, an enum's object left `{}`, with `error.path` the enum's and its message naming the variants serde expects (`AET-WO-1610` (4); it was `InvalidJson`, exit `3`); or an `IrValidationError` variant name such as `InvalidIdentifier`, `NoVoltageDomain`, `UnknownDomain`, `DuplicateI2cAddress`, `ZeroNominalVoltage` or `IoOutsidePart` (a device whose bus's domain leaves its part's I/O range, `AET-WO-4210` D7), `LifecycleNotAccepted` or `LifecycleStatusMismatch` (`AET-WO-4210` D1); `InvalidIdentifier` also carries `error.path`, and its message ends with the fix: the field's key and the whole shape of its identifier family (a project name, a device reference, or any other name), so a value written to that shape passes every identifier rule (`AET-WO-1608`). Since `AET-WO-1206`, `validate` also refuses what `generate` refused for a name or a part, printing exactly what `generate` printed: `MissingBusConstraint` (with `error.claimants`, as `solve` printed it), `UnknownPart` (a device part with no row in the parts table), `DuplicateReference`, `ReservedName` and `InvalidDeviceName`. Since `AET-WO-1209`, `validate` refuses a `power.loads` reference that names no part the board has (`LoadNamesNoPart`, with `error.path`): none the specification declares, nor the MCU, the reset switch, or an `R<n>`/`C<n>` the emitter numbers. The CLI's `validate` also refuses one the emitter does not draw for that specification (`LoadNamesUndrawnPart`, with `error.path`), and, since `AET-WO-1209b`, one whose part the emitter draws with no pin on the load's rail (`LoadPartOffRail`, with `error.path`, the message naming the nets its pins are on). Since `AET-WO-1209c`, `validate` refuses a load naming a part whose current is already counted, the MCU or a regulator (`LoadNamesCountedPart`, with `error.path`). Since `AET-WO-1210`, on a drawn board (`board.drawn` absent or `true`), `validate` refuses an SPI device on a line `extract` bounds whose part cites no maximum pin capacitance and which declares none (`UnboundedPinCapacitance`, `extract`'s kind, with `error.path`), and the CLI's `validate` refuses a device whose part has a row and no emitter arm (`PartNotDrawable`, with `error.path`). A specification that declares `board.drawn: false`, a twin stand-in, is refused by `emit`, `extract`, `verify --fab-out` and `verify --against-fab`, each by `DrawsNoBoard` with `error.path` `board.drawn`; plain `verify` runs it, its board gates `not_applicable`, and its verdict is at best `not_verified` (exit `7`, `AET-WO-1607`); `generate` writes no `kicad/` for it and says so in its `kicad` field. `solve` also: `UnknownPart`, or a `SolveError` kind (below), which carries `error.claimants`. `emit` also: `UnknownPart`, `UnsupportedAddress`, `DuplicateReference` (also two references KiCad takes for one, differing only in case or in a leading zero: `u2` and `U2`, `U01` and `U1`), `ReservedName` (also a net or rail starting `unconnected-`), `UnsolvedNet`, `UnknownPin`, `Unroutable` (`id` the reference, net or name; for `Unroutable`, the net the router could not complete, empty when the board as a whole is refused), `RouterWorkExceeded` (since `AET-WO-1323`: one routing did more work than the router's bound allows, 350 000 000 search steps and fill-flood nodes, a count and never a clock; `id` the net it was routing). `simulate --firmware` also: `UnsupportedPeripheral`, `NotInRenode`, `InvalidPin`, `AddressMismatch`. `campaign` (below): a scenario's `SchemaViolation`, `UnsupportedSchemaVersion`, `ExpectContradiction`, `DuplicateOverride`, `UnknownBus`, `UnknownNet`, `DuplicateScenario`, `BatterySagRefused`, `SeriesResistanceUndeclared`, `EscUndeclared`, `BatterySagSolvedBelowFloor`, `RailOutOfBandRefused`, and what `solve` refuses of the overridden specification |
| `3` | Unreadable input. `validate`: the file or stdin cannot be read, is empty, is more than 16 MiB, or is not one JSON document (syntax error, trailing data, a byte-order mark). A tool subcommand: an input path that does not exist, cannot be read, or is not a regular file (not a directory, for `--crate`); a crate holding a symlink, a special file or a non-UTF-8 name; a Renode built-in path outside Renode's own tree; `InputTooLarge`: a copied file, or a crate's files together, of more than 16 MiB (16,777,216 bytes), or a crate of more than 4,096 files (counted after `target/` is skipped). The byte cap is enforced while copying, never by truncating. `check drc`: a board that is not one KiCad S-expression, or one that carries a rule of its own, refused before any tool runs: a local `(clearance …)` on a pad, a footprint or a zone, or a net class defined in the board (`AET-WO-1300` D4). `check drc --spec` with `board.max_outline_um`: a board whose outline is not one `Edge.Cuts` `gr_rect` (`BoardUnreadable`), or whose outline is wider or taller than the bound, compared as the placer compares (`OutlineExceedsBound`, `AET-WO-1610` (3)). `check drc --spec` with `radio_modules` (`AET-WO-4411`), judged from the module's radio parts row, never its footprint's zones: the footprint at the module's reference is not the row's or is turned (`RadioFootprintWrong`), its U0TXD or U0RXD pad is not on the port's RX or TX net (`RadioUartMiswired`), or its antenna area with the row's clearance meets the outline (`RadioAntennaOnBoard`: the antenna overhangs the board's edge, ruled D2), or it is not at the top-right corner: its antenna edge off the board's top edge, or its feed side farther from the right edge than the row allows (`RadioNotAtCorner`). `check drc --schematic`: a schematic that is not one flat KiCad schematic, or a schematic or board carrying what would pass parity unmatched (below). `emit`: an `--out` that cannot be created, or a file there that is a symlink or not a regular file. `init`: an `--out` where anything already is (a file, empty or not, a directory, a FIFO, or a symlink, dangling or not), or whose directory does not exist (`UnwritableOutput`); it never overwrites. `simulate --firmware`, before Renode is looked for: a `--platform` that is not byte for byte the platform `aethelgard-twin` emits for `--spec`, or an `@` path; a firmware that is not ELF32 little-endian `EM_ARM` `ET_EXEC` with a sound program header table; a `PT_LOAD` whose physical or virtual range is outside the part's flash or RAM, two `PT_LOAD`s whose memory overlaps, or file bytes over the boot word at either address (below) | `UnreadableInput`, `InputTooLarge`, `InvalidJson`; `campaign`: `BaseOutsideRoot`, `RunExists`, `UnwritableOutput`, `ArchiveWriteFailed`; `campaign` also `FirmwareChanged`; `replay`, `attribute` and `propose`: every `aethelgard-chaos` archive refusal by its own name (`HashMismatch`, `ReplayDiverged`, `UnexpectedEntry`, …), `RunIdMismatch` and `NotACampaignRun`; `attribute` also `Undefined` or `MalformedSource`, a register fact the run names that the ST header lacks; `propose` also what `campaign` refuses of the scenario's file or base, `ScenarioMismatch`, `BaseChanged`, `TopologyChanged` and `UnwritableOutput`; `MalformedBoard`, `LocalClearance`, `BoardNetClass` and `BoardWaiver` (`id` naming the carrier, such as `pad 1 of footprint R7`); `MalformedSchematic`, `HierarchicalSchematic`, `ReservedNetName`, `BoardOnlyFootprint`, `NoConnectPadOnNet`, `SharedUnconnectedNet`; `UnwritableOutput`; `PlatformNotEmitted`, `MalformedFirmware` (overlapping memory included), `FirmwareOutsideMemory`, `FirmwareLoadsBootWord`; `simulate --foreign`: `ForeignPanicSymbolMissing` (the firmware's symbol table names no `AP_HAL::panic`, `AET-WO-1609`); `simulate`, a platform that declares a model of our own (`AET-WO-4220` D6): `ModelUnpinned` (a header line naming a model or sha256 `aethelgard-twin` does not pin, or a device declared as one the header does not name), `ModelPinMismatch` (the CLI's own copy does not hash to its pin) and `ModelLoadFailed` (Renode printed `Errors during compilation or loading:`, which it does with exit 0); `build`: `CrateHasDependency`, `NotABoardCrate`, `UnwritableOutput`; `build` and `prove`: `MalformedManifest`, `CargoFeaturesRefused`, `CrateHasProcMacro`, `CrateHasBuildScript`, `CargoConfigRefused`, `ToolchainFileRefused`, `WorkspaceAboveCrate` (the pre-flight, below); `verify`: `ScenarioBaseMismatch`, and `UnwritableOutput` for a `--fab-out` that is not an empty directory or absent, or that is not the caller's alone; `verify --fab-out` and `--against-fab`: `BottomSidePart`, a part placed on the bottom side, which JLCPCB's formats as ruled cannot express; `extract`, and `check drc --spec` before any tool runs: `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`, `ViaNotThrough`, and `ClassNetAbsent` (a declared class's or pair's net on no pad of the board, `AET-WO-1308`); `extract` also: `BoardUnreadable`, `MalformedBoard`, `UnmodelledCopper`, `BoardTooLarge`, `NetLabelsDisagree`, `ClearanceViolated`, `LineNotOnBoard`, `DeviceNotOnLine`, `UnmodelledPart`, `PartMismatch`, `Overflow` |
| `4` | The tool cannot be used: it is not on `PATH`, is not executable, will not report its version, or lacks an environment variable it needs; for `simulate`, a Renode other than the 1.17.0 the twin's platform is pinned to (`ToolUnusable`), or another build of it than the tools image's, or one that names none (`RenodeBuildNotPinned`, `AET-WO-4400g`); the sandbox is unavailable; or there is no private place for scratch (below). Never a verdict (`INV-06`) | `ToolMissing` (`id`: the tool), `ToolUnusable` (`id`: the tool, or the missing variable), `RenodeBuildNotPinned` (`id`: `renode`), `ScratchUnsafe` (`id`: the directory that breaks the rule, or `HOME`), `SandboxUnavailable` (`id`: `landlock`, `seccomp`, or `descriptors`); `simulate --firmware`: `BridgeUnavailable` (`id`: `aethelgard-bridge`) when the bridge is not beside the `aethelgard` binary |
| `5` | The tool ran but its output cannot be trusted as a verdict: killed by a signal, an exit code outside the ones measured for it, a missing, unparseable or oversized report or output, or two witnesses that disagree. When `prove` or `simulate` cannot recognise its tool's outcome, the message ends `; the tool said: <line>`, every `error:` line (Kani, cargo) or the first `[ERROR]` line (Renode), as printable ASCII, cut to 400 bytes for cargo or 240 for Renode; for cargo it is every `error:` line joined by `|`. Never read as pass or fail | `ToolError` (`id`: the tool); `verify`: `BindingMismatch` (`id`: the gate, or `fab`), a gate that judged bytes other than the generated ones, or a board or ELF the package would carry that is not the one the gates were bound to; `verify --fab-out`: `ToolError` for a Gerber whose clock lines are not as measured or that carries another date, a placement row a quote would shift, a declared zone not poured, or a hole that is not a via |
| `6` | `propose` only: the run's failure is a generator defect, and `generator-report.json` was written beside `proposal.json`. MCP reports it as a successful call, as it does `0` and `1` | none: stdout is `propose`'s line |
| `7` | `verify` only: every gate that could run passed, but the specification declares `board.drawn: false`, so it has no ERC and is never verified (`INV-03`): the verdict is `not_verified`, `reason` `DrawsNoBoard`, never `pass` (`AET-WO-1607`). A gate that failed is still `1`. MCP reports it as a successful call, as it does `0`, `1` and `6` | none: stdout is the verdict |
| `64` | Usage error: unknown subcommand or flag, missing subcommand or required flag. `init`: a `--template` that names no template. `simulate`: `--firmware`, `--spec` and `--for` given other than all together, or a `--for` that is not an integer without a leading zero then `ns`, `us` or `ms`, is above 100 ms, or is not a multiple of 10 ns | `UsageError` |
| `70` | Internal error: the report could not be written to stdout (for example, the reader closed the pipe), the parent-death signal could not be set, or the process that started this one is already gone (see the reaper below); or `solve`'s vendored pin table, or `emit`'s vendored KiCad library, failed its own hash or parse check, which only a corrupted build can cause. Nothing is printed on stdout; the reason goes to stderr | none |

A stdout closed by the caller (`>&-`) is not an exit-`70` case: the Rust runtime reopens a closed standard descriptor on `/dev/null` before `main` runs, so it behaves exactly like `>/dev/null`, and the exit code still reports the verdict.

`--help` and `--version` exit `0` and are the only output that is not JSON. `--version` prints `aethelgard <version> (commit <sha>, stated by the build)` when the build set `AETHELGARD_COMMIT`, as the engine image does, and `aethelgard <version> (commit unknown)` otherwise (`AET-WO-1802`). `message` is for humans and its wording is not part of the contract; `ok`, `kind`, `id`, the optional `error` keys named below (`claimants`, `path`, and `entries` on `verify`'s `UnwitnessedFieldNotAcknowledged`) and the verdict fields are.

## `init`

`init --template <name> --out <file>` writes one specification a person starts from, and which cannot pass until they fill it (`AET-WO-1608`). The template is held in this binary (`templates/`), and its bytes are written verbatim: nothing is filled, defaulted or asked.

| Template | From | Placeholders |
|---|---|---|
| `i2c-spi-bench` | `examples/sentinel-02-bench.json`: a 2-layer STM32H753 board with one I2C sensor (BMP581) and one SPI sensor (ICM-42688-P) | `project_name`; `mcu.core_supply.source`; each device's `pin_capacitance.declared.condition` and `.source`; the four `board.stackup` sources |

**What a placeholder is.** Every value a person must own is left empty (`""`): the project name and every citation. `validate` refuses each by its own kind and `error.path` until it is written: `InvalidIdentifier` for `project_name`, `InvalidSource` for each citation. With all but one filled, `validate` names that one. Every other value is the example's design, unchanged. The parts, pins, bus speeds, pull-up and the board's rules and stackup numbers are each judged by an engine rule, and each measured number sits beside a citation the person writes. A test holds each template to its example field for field.

**It never overwrites.** The file is created with `O_CREAT | O_EXCL | O_NOFOLLOW`. Anything already at `--out` (a file, empty or not, a directory, a FIFO, a symlink, dangling or not) is refused, exit `3`, `UnwritableOutput`, and left as it was; a FIFO is never opened. No directory is created: a missing one is refused the same way. Only the last component is never followed, as for `build --out`. An unknown `--template` is exit `64`, `UsageError`.

It prints one line, exit `0`:

```json
{"ok":true,"template":"i2c-spi-bench","out":"spec.json","placeholders":["project_name","mcu.core_supply.source","i2c_devices[0].pin_capacitance.declared.condition","i2c_devices[0].pin_capacitance.declared.source","spi_devices[0].pin_capacitance.declared.condition","spi_devices[0].pin_capacitance.declared.source","board.stackup.finished_thickness_um.source","board.stackup.dielectrics[0].source","board.stackup.copper_max_um.source","board.stackup.relative_permittivity_max_permille.source"]}
```

A filled template passes `validate` and `solve` (tested); every later gate judges it as it judges the example. Over MCP, `init` is a tool like any other subcommand; it writes only a new file at the path the client names.

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

`UnsupportedSchemaVersion` of an older specification (`AET-WO-1215`) also carries the optional `entries`, absent from every other refusal of `validate` and from a newer version's or version 0's: one row of `aethelgard_ir::versions::CHANGES` per version after the file's own, in order, each `{"version", "work_order", "requires", "optional", "replaced": [{"path", "by"}], "removed", "rules"}`, paths and refusal kinds only, never a value. `message` lists the same rows compactly. `validate`, `solve`, `emit`, `generate`, `extract`, `check speeds`, `verify` and a campaign scenario's base print `entries`; `check drc --spec` and `check support` print the message only, as they print no `claimants`. `2307`'s `UnwitnessedFieldNotAcknowledged` uses the same key with its own element shape.

Every key is required, including `peripheral`, `pin` and `impedance_ohms`, which are `null` when unset. The shape and the refusals, in check order, are in `crates/aethelgard-ir/README.md`. See `examples/` at the repository root.

**Unwitnessed acknowledgements** (`AET-WO-4230` R1). When `acknowledge_unwitnessed` has entries, `validate` then computes the witness ledger as `verify` does (the pin table, `solve`, the generator's field list, the classification) and refuses, exit `2`, `UnwitnessedAcknowledgementStale`, the first entry naming a field the ledger has no row of for that register and class, or a register the ledger lacks (`id` the register, `path` `acknowledge_unwitnessed[i].fields`). The message names each such field by its index (`fields[3]`), never by its text, and ends with the ledger's own fields for that register and class, or says it has none (`AET-WO-1610` (5)): nothing the entry supplies reaches it. A specification with entries for which no ledger exists (it does not solve, or the generator refuses it) is refused the same way at its first entry. `verify` refuses a stale entry too, before anything runs. With `[]`, nothing is computed.

## `solve`

```
aethelgard solve --file <path>
```

Reads one `Specification` from the file, under the same 16 MiB cap and with the same exit `2` and `3` refusals as `validate`, and validates it. It then loads the pin table for `mcu.part` (exit `2`, `UnknownPart`, `id` the part, for any part but `STM32H753ZITx`), solves (`aethelgard_ir::solve`), and prints the `SolvedTopology` as one JSON line, exit `0`. `specification_sha256` is the sha256 of the file's bytes as read, so reformatting the file changes the hash and nothing else. It writes no file.

```json
{"schema_version":20,"specification_sha256":"9422b2e7…","assignments":{"CONSOLE_RX":{"pin":"PA10","peripheral":"USART1","signal":"USART1_RX","alternate_function":7},…},"gpio":{"FLASH_CS":{"pin":"PA0","direction":"output","initial":"high"}},"adc_inputs":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{…},"i2c_devices":{…},"spi_devices":{}}
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
  - on SPI, `W25Q64JVxIQ` (Value `W25Q64JVSSIQ`), drawn with KiCad's `W25Q32JVSS` symbol, same SOIC-8 pinout, with the substitution noted in its bill-of-materials row.
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

### `generate hwdef`

```
aethelgard generate hwdef --topology <solved.json> --out <hwdef.dat>
```

Writes ArduPilot `Copter-4.7.2`'s board definition for a `SolvedTopology`, from the topology and the parts tables alone (`aethelgard-hwdef`, `AET-WO-4400b`, `RUL-16`). Unmodified ArduPilot builds it as an untracked board directory, and `check pinout` judges the ELF it links. The crate's `README.md` gives every line, its source, the constants and the refusals. It starts no process. `<hwdef.dat>` must not exist; it is created, never through a symlink. It prints one line, exit `0`:

```json
{"ok":true,"generator":"hwdef","topology_sha256":"<64 hex>","hwdef_sha256":"<64 hex>","not_emitted":["U3 (TMP117): no ArduPilot line; …"]}
```

`not_emitted` names each fact the topology holds that no line states, and why; the file's header lists the same.

| Exit | When |
|---|---|
| `0` | written |
| `2` | the topology is the wrong shape (`SchemaViolation`), `validate` refuses it, or the generator cannot state it, each by name: `NoSerialPort`, `NoConsole`, `ConsoleTwice`, `RcInputTwice`, `TooManySerialPorts`, `InvertedConsole`, `FramingNotImplied`, `ConsoleBaud`, `SerialBaud`, `UnsupportedSignal`, `LabelCollision`, `LabelTakesAfPrefix`, `DuplicateSpidev`, `SpiBelowArduPilotDivider`, `SpiKernelNotCited`, `NoSystemTimer`, `SenseNotOnAdc1`, `SenseTwice`, `DividerNotExact`, `SwdPinSolved`, `UnknownMcu`, `UnsupportedSeries`, `OscillatorNotCited`, `PinNotEmitted` |
| `3` | the topology cannot be read or is not JSON, or `<hwdef.dat>` exists or cannot be written (`UnwritableOutput`) |

## The tool subcommands

```
aethelgard check drc --board <file.kicad_pcb> [--spec <specification.json>] [--schematic <file.kicad_sch>]
aethelgard check erc --schematic <file.kicad_sch>
aethelgard check support --schematic <file.kicad_sch> --board <file.kicad_pcb> --spec <spec.json>
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
  - with `--spec` declaring coupled pairs (`AET-WO-1308`): the board read as `extract` reads it, then each pair judged from its copper (`route::pairs`): `BoardUnreadable` (including a track or via that is not its net's class's), `NetLabelsDisagree`, `ClearanceViolated`, or `PairNotCoupled`, naming the pair and what breaks it. The verdict then carries `pairs`: for each, its nets, gap, layers, each conductor's length, vias and uncoupled length at each end, the length difference and the bounds it was judged by;
  - **the strict reader's refusals** (`aethelgard_eda::strict`, `AET-WO-1306`), `MalformedBoard` with `id` the path to the node, such as `kicad_pcb/general[1]`. In a node any gate reads, a second child setting a value KiCad 10.0.7 holds once (it keeps the last, and no gate may pick either), whichever spelling sets it: a second `(thickness …)`, a pad's second `(net …)`, a track's `(layer …)` and `(layers …)`, a second `general` section. A footprint field named twice, counting a legacy `(fp_text reference …)` or `(fp_text value …)` as the `Reference` or `Value` property it sets. The same reader serves `check erc`, parity and `extract`, so every gate reads one tree.
- **Writes the setup block over the board's**, with no clearance token, so the board minimum has one source. Nothing else in the board changes.
- **Writes `input.kicad_pro`, `input.kicad_dru`, `sym-lib-table` and `fp-lib-table`** beside the copy:
  - `input.kicad_pro` holds the Default net class, the board minimum, and ERC's library checks set to `ignore`;
  - `input.kicad_dru` holds the same minimums as custom rules, then, for each declared net class and coupled pair (`AET-WO-1308`), a rule keyed on its nets' names: tracks exactly its width (min and max), and for a class that declares a via, vias exactly its pad and drill with at least its ring. A later rule replaces the board minimum on those nets. KiCad 10.0.7 was measured reporting a class track too narrow and one too wide, and a class via too small (FEEDBACK §3);
  - `sym-lib-table` and `fp-lib-table` name the vendored KiCad 10.0.7 libraries, which the gate copies into `lib/` beside them.

  The board's own `.kicad_pro` and `.kicad_dru` are never copied. No environment variable is set.

**With `--spec`**, the rules are the specification's own (`board.rules`, AET-WO-4100): its clearance as every clearance and the board minimum, and its track width as the minimum track width, each at or above the engine floor of 0.2 mm, which `validate` enforces. The verdict says `"rules":"specification"` and carries the file's `specification_sha256`.

**Without it**, the same files hold KiCad's own defaults (a 0.2 mm Default net class, no board minimum, no track-width minimum), and the verdict says `"rules":"kicad-default"`.

**With `--schematic`**, KiCad also checks the board against that schematic (`--schematic-parity`), and its `schematic_parity` findings are judged like every other finding. The schematic is copied and hashed as the board is, as `input.kicad_sch` beside the board's copy, where KiCad looks for it; the verdict carries `"schematic":{"path":…,"sha256":…}`. Before any tool runs (exit `3`, `aethelgard_eda::parity`), the gate refuses:
- a schematic that is not one KiCad S-expression `kicad_sch`, read as strictly as a board (`MalformedSchematic`), by the same strict reader: in a placed symbol or the top-level list, a second child setting a value KiCad holds once (`lib_id`, `at`, `unit`, `paper`, …); a symbol field named twice; and a mandatory field (`Reference`, `Value`, `Footprint`, `Datasheet`, `Description`) spelt otherwise than KiCad's canonical name, since KiCad folds `"VALUE"` into `Value` and the gate compares exactly. `id` is the path to the node;
- a schematic with a hierarchical `(sheet …)` anywhere, since KiCad would read the file it names, which the gate never copied or hashed (`HierarchicalSchematic`);
- a schematic with a label or value starting `unconnected-`, KiCad's own prefix (`ReservedNetName`).

It also refuses what KiCad 10.0.7's parity check was measured passing on a board that does not match its schematic (`AET-WO-1301` adversarial pass):
- a `board_only` footprint, which parity skips (`BoardOnlyFootprint`);
- a pad whose `pintype` says `no_connect` but which sits on a real net, which parity matches by prefix (`NoConnectPadOnNet`);
- an `unconnected-` net on more than one pad (`SharedUnconnectedNet`).

Without `--schematic`, nothing changes.

DRC also runs with `--refill-zones`: zones are judged filled, and the board is never saved, because KiCad's saved fills are not deterministic. `check erc` writes only `input.kicad_pro`.

**`check erc` refuses a hierarchical schematic** (exit `3`, before KiCad runs; `AET-WO-1601` §1.5): a `(sheet …)` anywhere makes KiCad read the file the sheet names, which the gate has neither copied nor hashed (`HierarchicalSchematic`). The copy is read by the same strict reader as a parity schematic, so a schematic it cannot read is refused too (`MalformedSchematic`): a sheet could hide where the two readers disagree. `emit` never writes a sheet.

### The support gate (`check support`, `AET-WO-4300g`)

`check support` judges the MCU's support network from the schematic's netlist as KiCad exports it (`kicad-cli sch export netlist --format kicadsexpr`, in the sandbox). The netlist is read through the strict reader (`MalformedNetlist`), against the MCU's row in `power-parts.json` (`kind.mcu.support`) and the specification, never against the emitter's choices (`RUL-20`). The board is read strictly for one thing, where each pad sits, so each bounded capacitor's distance to its pin can be measured, copper edge to copper edge.

**In order:**
- the MCU, found by the specification's part number;
- every supply-group pin on the declared supply, every ground-group pin on `GND`, and every powered pin on a declared domain;
- each strap's pin on its group's net;
- each capacitor role, one distinct capacitor per pin (or per group): between the right two nets, of the role's value class (±20 %), and within its bound where the row gives one;
- every declared connector pin on the supply, `GND` or a debug net, on that net, pin for pin;
- the programming connector, with whether NRST reaches it;
- the declared source's pin on the supply.

**Findings:** each failure is a finding, so the check fails with exit `1`:
- `support_pin_off_net`;
- `strap_wrong`;
- `support_capacitor_missing`;
- `connector_pin_off_net`;
- `no_programming_path`;
- `supply_source_off_net`.

**The verdict's `support`** carries each role with its part, value, gap and bound, the straps, the programming connector, the source, and every condition the gate cannot check (an unbounded role's placement; dielectric, rating and ESR). `verify` runs it after `check drc` as the gate `support`. The package's `FAB-NOTES.md` repeats its record, and the precondition `mcu_support_network_judged` holds only when it passed.

### The sandbox

Every tool process runs with:

- an argument array, never a shell, and every input given as an absolute path inside the temporary directory, so no file name can be read as a flag;
- **a separate uid when the CLI runs as root** (`AET-WO-1601` §1.2): the trampoline clears the supplementary groups and becomes uid `65534` (`nobody`) and gid `65533`, before Landlock and the filter, then reads every id back from the kernel; a switch, group change or hand-over that fails refuses the call with exit `4`, `SandboxUnavailable`, `id` `uid`, never a run as root. The input copy and every file the CLI writes beside it belong to root and are read-only (files `0444`, directories `0555`); the tool's directory belongs to root with group `65533`, mode `1770`, so the tool can create files in it but cannot rename or remove what root put there; for `prove`, the crate's root directory is the same, because cargo writes the `Cargo.lock` a crate without one needs there; the call's directory above it is `0710`, root and group `65533`, so no other user can even pass through. Gid `65533` is no account's group, unlike `65534`, which `sync` and `_apt` have in the image. As any other user, the tool runs under the caller's uid, as before. Uid `65534` is measured, not chosen: the analog bridge's libngspice crashes under a uid with no passwd entry, and the image lists no other unprivileged one but its `kicad` login user.;
- **`RLIMIT_FSIZE` at 64 MiB**, soft and hard, so no tool can write a larger file into its RAM-backed scratch (`AET-WO-1601` §1.6, review A12.1). The largest file any gate writes on the reference is Kani's 2.2 MB goto binary; Renode's .NET would truncate a 2 TiB double-mapping memfd under W^X, so the CLI also sets `DOTNET_EnableWriteXorExecute=0` for Renode. A tool cut off at the cap ends as exit `5`, `ToolError`: killed by `SIGXFSZ` itself, or, when a child of it was (rustc under cargo), by the file it leaves at exactly the cap, whatever its own exit code says. A tool that ignores the signal and deletes that file is judged on what it reports;
- an **empty environment** plus `HOME` and `TMPDIR` set to the temporary directory; `prove` alone also passes `KANI_HOME`, `RUSTUP_HOME` and `PATH` through from the caller, by name;
- stdin from `/dev/null` (Renode's and the analog bridge's: a pipe the CLI holds open, since Renode's console queues every end of `/dev/null` it reads, without bound; `run::HELD_STDIN`, `AET-WO-4400j` D1. It carries one `quit`, which Renode's monitor reads only when a command fails and leaves it at its prompt, and which ends it there as the end of `/dev/null` did; `run::HELD_STDIN_LINE`), and stdout and stderr captured up to 16 MiB each (more is exit `5`);
- **Landlock**: read-only access to the system directories, the tool's own installation and the input copy, read-write access only to its temporary directory, and no other filesystem access. Renode also gets read-only `/proc` and `/dev/urandom`; Kani also gets read-write `/dev/null`. Landlock's filesystem rights through ABI 3 are **required**: if the kernel cannot enforce them, the call refuses with exit `4`, `SandboxUnavailable`, `id` `landlock`. There is no flag or variable that runs a tool unconfined. Where the kernel can, Landlock also denies TCP bind and connect (ABI 4 and later; `sandbox.tcp_confined`) and confines signals and abstract Unix sockets to the tool's own processes (ABI 6 and later; `sandbox.scoped`);
- a **seccomp filter** (AET-WO-0300a), installed after Landlock and before the tool starts, inherited by every process the tool starts, and **required** like Landlock: if it cannot be installed, the call refuses with exit `4`, `SandboxUnavailable`, `id` `seccomp`, and there is no flag or variable that skips it. It refuses `socket()` for every family with `EPERM`, so pathname and abstract Unix sockets, UDP, TCP on any kernel, raw and netlink sockets are all out of reach, and it refuses `io_uring_setup`, `io_uring_enter` and `io_uring_register` with `EPERM`, since `io_uring` can open and connect sockets without the `socket` call. It kills the process on any system call made through another architecture, the x32 ABI and i386 included; on a host that is not x86_64 there is no filter, so every tool call refuses with `id` `seccomp`. `socketpair` is allowed only as an `AF_UNIX` stream or seqpacket pair, which is born connected to itself and cannot be redirected; a datagram pair could `connect` to a datagram socket outside, so it is refused. No tool needs a socket (measured; Renode's .NET runtime would try one for its diagnostic IPC, so the CLI also sets `DOTNET_EnableDiagnostics=0` for Renode). `sandbox.sockets_denied` is always `true` in a verdict. **Not confined at the socket layer: nothing.** Outside this filter's scope, a tool can still read what Landlock lets it read, write its temporary directory, and use non-socket channels Landlock does not mediate, such as System V shared memory with other processes of the same user.;
- **no inherited descriptors**: before it confines itself, the trampoline lists its open descriptors (`/proc/self/fd` and `/proc/self/fdinfo`). Any descriptor above 2 without close-on-exec, which the tool would inherit as a channel no other layer sees, refuses the call with exit `4`, `SandboxUnavailable`, `id` `descriptors`, naming the descriptor numbers, and the tool never runs. An unreadable `/proc` refuses the same way. The tool gets stdin from `/dev/null` and its two output pipes, nothing else;
- a **reaper**: the CLI starts a copy of itself, in a new process group, so a signal to the CLI's own group does not reach it. The reaper starts the tool in a second group and, as a child subreaper, inherits every orphan the tool leaves, a daemon in its own session included. When the tool ends, or the CLI dies by any signal, the reaper kills the tool's group and every remaining descendant. A **guardian**, started before the temporary directory is created, removes that directory if the CLI dies before removing it itself. The CLI passes both its own PID, so a CLI killed before either has looked is still seen to be gone, at any instant from its first line to the tool's `exec`. A CLI started by `mcp` also dies, by `SIGKILL`, when `mcp` dies (`PR_SET_PDEATHSIG`); the reaper and the guardian do not, so they clean up after it. `mcp` tells each CLI its pid through an internal variable; with it set, the CLI arms the signal and then exits `70`, before it starts anything, if that parent is already gone. Linux sends the signal when the parent *thread* ends, so no other CLI arms it, and `mcp` never arms it on itself: a CLI run directly behaves as before, and its caller owns its lifetime, as it owns its timeout (`RUL-13`). The CLI owns no timeout (`RUL-13`): the caller's timeout ends the CLI, and the reaper and guardian clean up after it. A `SIGKILL` sent to the reaper's own PID by something outside the sandbox is outside this guarantee; the tool itself cannot send it when `sandbox.scoped` is true. The reaper, the guardian and the Landlock trampoline are invoked through internal first arguments that are not subcommands and are not part of this contract.

### Two witnesses for every verdict

| Check | Pass | Fail | Anything else |
|---|---|---|---|
| `check drc`, `check erc` (`kicad-cli` with `--exit-code-violations --format json`, `RUL-03`) | exit `0` and a report with no finding | exit `5` and a report with at least one finding | `ToolError`: another exit code, a missing or unparseable report, a count that disagrees with the code, a report whose `included_severities` is not exactly `["error","warning"]`, or whose `kicad_version` differs from the version `kicad-cli version` printed |
| `prove` (`cargo kani --harness <name>`, plus `--exact` when the name is a path) | exit `0`, `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures`, exactly one `Checking harness <path>...` whose path is the name asked for or ends in `::<name>`, and, if Kani prints a cover summary, `** N of N cover properties satisfied` | exit `0` as for a pass, but with a cover summary `** S of N …` where S < N (`cover_unsatisfied`: a `kani::cover!` the harness declares is never reached, so the proof may be vacuous; AET-WO-1400); or exit `1` with `VERIFICATION:- FAILED` (`proof_failed`), or `Found N compilation errors` (`compile_error`), or `Failed to match the following harness(es)` (`harness_not_found`); or, with exit `0` or `1`, checked harnesses none of which has that name (`harness_not_found`: Kani's `--harness` matches substrings) or more than one (`harness_ambiguous`: name it by its path) | `ToolError` |
| `simulate --firmware` (below) | the three boot witnesses agree: the boot word holds the magic, every solved register reads as the topology computes it, and the core executed exactly 100 MIPS × the virtual time; the core ends inside an executable segment of the firmware; a CPU and every solved peripheral listed after the reads; no `There was an error executing command`, no `[ERROR]` line and no locked-peripheral warning | `boot_word_missing`, `register_mismatch` (one per register), `cpu_halted` (fewer instructions than the run holds, or a line of the CLI's hooks whose instruction count lags its stamp: the core slept, `WFI`), `memory_cap_exceeded` and `cpu_aborted` (killed at a bound, below), `locked_peripheral_access` (the firmware touched a peripheral whose clock was off), `pc_outside_firmware` (the core ended outside the firmware's executable segments), `renode_error_log`; `platform_load_error` or `firmware_load_error` when that command failed and nothing was read | `ToolError`: a nonzero exit; markers missing or repeated; a read that is not a bare value; a load error beside reads; another command's error; no CPU, or a listing without a solved peripheral; more instructions than the run holds; or the boot word set while the core executed fewer |
| `simulate` (`renode` loads the platform, then `peripherals`) | a CPU in the listing (an entry followed by a `Slot:` line), no `There was an error executing command` and no `[ERROR]` log line | `There was an error executing command` and no CPU, or a CPU listed beside any `[ERROR]` line (`platform_load_error`; the finding's message is the first `[ERROR]` line). `[WARNING]` lines do not change the verdict | `ToolError`: both, neither, or a nonzero exit. Renode exits `0` even when the load fails (`architecture.md` §2.4), so its exit code is recorded and never trusted alone |

Every KiCad finding is a fail, warnings included (ruled on PR #12): the pass criterion is that KiCad found nothing at its default severities.

### The verdict

```json
{"ok":false,"verdict":"fail","check":"drc",
 "input":{"path":"/work/drc-violation.kicad_pcb","sha256":"<64 hex>"},
 "tool":{"name":"kicad-cli","version":"10.0.7","exit_code":5},
 "sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true,"uid_boundary":false,"uid_boundary_unmet":"tool_uid"},
 "included_severities":["error","warning"],
 "findings":[{"kind":"clearance","severity":"error","message":"Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)",
              "where":[{"description":"Track [NET_B] on F.Cu, length 20.0000 mm","x":105.0,"y":110.3,"uuid":"…"}]}]}
```

- `ok` is `true` exactly when the exit code is `0`; `verdict` is `pass` or `fail`; `check` is `drc`, `erc`, `prove` or `simulate`.
- `input.path` is the canonical path of the input as given (for an `@` platform, the file inside Renode's tree); `input.sha256` is the hash of the input's bytes as copied. For `check drc` the tool then judges that copy with the gate's setup block written over the board's, a function of those bytes alone (above).
- `tool.version` is the tool's own version line, exactly as printed; `tool.exit_code` is the exit code of the judging run.
- `sandbox.uid_boundary` is `true` only when **all three** hold (`RUL-19`: a verdict claims only what it shows; ruled on PR #96). Otherwise it is `false`, and `sandbox.uid_boundary_unmet` names the first that failed:
  1. `tool_uid`: the tool ran under uid `65534` and gid `65533`, switched to and read back by the trampoline. That happens only when the CLI runs as root.
  2. `user_namespace`: the CLI's root is the initial user namespace's (`/proc/self/uid_map` is `0 0 4294967295`). Under rootless podman, root maps to the user who owns the namespace (`forge` on Furnace), and that user can reach every process in it.
  3. `toolchain`: every path the tool's read grant adds beyond the system directories (`/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/etc`, `/proc`, `/dev`) is owned by root, not writable by others, and writable by its group only when that group is root's (gid 0), all the way down; links are skipped. This is the rule the image check shares (`AET-WO-1602`). Those paths are the tool's directory on `PATH` and its real one, `KANI_HOME`, `RUSTUP_HOME`, Renode's tree and the bridge's directory. Each ancestor must pass the same rule, or be sticky. The check runs once per path per process. In the pinned tools image, Kani's kit belongs to uid 1001, so `prove` says `toolchain`.

  A verdict with several tools (`simulate` with the bridge) takes the first failure among them. **`false` leaves this open:** a process the boundary does not exclude can change what the tool reads between the gate's hash and the tool's read: a source file, a `.cargo/config.toml` in the tool's `HOME`, an ELF (`AET-WO-3301` F1). **`true` still leaves open:** root, and a process holding gid `65533` outside the sandbox, which only root can grant. Such a process cannot change the input copy, but it can rewrite what the tool writes and plant files in the tool's `HOME` and `prove`'s crate root, and so decide the verdict (a planted cargo alias for `kani` was measured to pass a failing proof).
- `findings` is normalized to `{kind, severity, message, where}`. For KiCad, `kind` is the report's `type` (`clearance`, `track_dangling`, `pin_not_connected`, `label_dangling`, …), `severity` is `error` or `warning`, `message` is the report's `description`, and `where` lists the items it names. For `prove` and `simulate`, `kind` is one of the names in the table above, `severity` is `error`, and `where` is empty.
- `check drc` and `check erc` also carry `included_severities` from the report; `check drc` carries `rules`, with `--spec` `specification_sha256`, and with `--schematic` `schematic` (above); `prove` carries `harness` (the name), `harness_provenance` and `cover_count` (below); `build` carries `elf`, `{"path","sha256"}` of the ELF written, and `tool.rustc`, `{"release","commit_hash"}` of the compiler cargo ran (below); `simulate` carries `machine`: `{"cpus":["cpu"],"peripherals":[{"name":"cpu","type":"CortexM"},…]}`, and with `--firmware` also `boot` (below).

- **`ran`** (`AET-WO-1609`): each part of the check that a key, a flag or a condition switches, as `verify`'s table states it (below). `check drc`'s parts are `drc.pairs` (`--spec` with `diff_pairs`, judged per pair), `drc.power` (`--spec` with `power`, judged per loop rule), `drc.orientation` (`--spec` with any `spi_devices[].orientation`, judged per declared IMU seat) and `drc.outline` (`--spec` with `board.max_outline_um`, judged once) (`AET-WO-1610` (3)). Without `--spec`, each is `off: --spec`. A generated `simulate`'s part is `simulate.hse_clock`: `off: mcu.hse`, `judged 1` once the boot word is written, or `not_reached` before it. `extract`'s line carries its four parts the same way. `ran` is absent from every other verdict.

A verdict that cannot name its input's hash and its judge's version is never printed; the call refuses instead.

### Booting firmware (`simulate --firmware`)

`AET-WO-2100`, ruled on PR #56. The platform is the one `aethelgard-twin` emits for the specification, committed for the reference board as `boards/sentinel-01/sentinel-01.repl`; any other platform is refused (above). The reference firmware is generated beside the board crate at `boards/sentinel-01/firmware/` and builds with `cargo build --release --offline --locked` from that directory into the repository's `target/`.

**The boot contract** (`aethelgard_twin::boot`), which the generated runtime follows:
- the **boot word** is the first word of RAM, `D1_DTCMRAM_BASE` (`0x20000000`), and the firmware's fixed point writes `0x2100B007` to it;
- the firmware never sleeps once it is running: the instruction witness needs the core busy for the whole run;
- a firmware may load only into flash, `FLASH_BANK1_BASE` to `FLASH_END` (`0x08000000`–`0x081FFFFF`), or RAM, `D1_DTCMRAM_BASE` and the 128 KB its comment states. Every address comes from ST's pinned header.

**The run.** Every Renode the CLI starts reads `--config <scratch>/renode.config`, which sets one key, `[general]` `use-synchronous-logging = true` (`AET-WO-2204` D4): each log line is written by the thread that logs it, so a line the firmware caused is on the console before `RunFor` returns, never split below line level by the monitor's output and never dropped by a level change. Every other key is stated at Renode's own default, the value it writes when given no file (Renode writes a missing key back, and the file is sealed read-only for the tool uid). One Renode call executes:
1. `emulation SetSeed 2100`, so the platform's random initial values are the same in every run;
2. the platform; then `sysbus LoadELF` of the firmware's copy;
3. `0` written to every RCC enable register that gates a solved port or peripheral, silicon's reset value (Renode resets them with every clock on), and to the boot word, so only the firmware's code can set it;
   then `cpu VectorTableOffset` set to `FLASH_BANK1_BASE`, where the part boots, rather than letting Renode guess the table from the ELF;
4. `logLevel -1 sysbus.<bus>.<device>` for each modelled I2C device the platform declares (the device's own log, not the console's level), the CLI's own watchpoint hooks (below), then `emulation RunFor` the `--for` time, at most 100 ms on every board;
5. then the reads between two marker lines (the boot word, `cpu ExecutedInstructions`, `cpu PerformanceInMips`, `cpu PC`, `ICSR` and each solved register), then `peripherals`. No console level is ever changed.

**The pinned Renode's `RETTOBASE`** (`AET-WO-1605`, ruled on PR #223). Renode 1.17.0's NVIC (`renode-infrastructure` `cd4b002a`, `NVIC.cs` L1079) set `ICSR.RETTOBASE` when at most one *system* exception was active, so a handler nested over an external interrupt's read 1, where the Armv7-M ARM (DDI 0403E.b §B3.2.4, p. B3-657) says 0. ChibiOS's interrupt epilogue builds a thread-mode return frame on that bit, so unmodified ArduPilot hard-faulted on the twin and not on silicon (`AET-WO-4400e2`). The CLI corrected it with a monitor hook (`AET-WO-4400e3`) until the tools image carried upstream's fix (`e7a8584c`): its Renode is 1.17.0 with that commit and upstream's cache-maintenance fix rebuilt in (build `1.17.0+20260907gitf1dd1b4af.aet1605`, `container/Dockerfile`), and the hook is gone. Two tier-2 tests hold it: the image's `renode --version` names that build, and `tests/fixtures/rettobase`, two nested external interrupts, reads 0 in the nested handler and 1 in the one beneath it, with nothing of ours.

**The pinned build, recorded and refused** (`AET-WO-4400g` §1.5, Q4 ruled on PR #229). Every `simulate` verdict records the build `renode --version` named as `tool.build`. Any `simulate` (a plain load, `--firmware`, `--foreign`, and so `campaign` and `verify`) on any build but the tools image's is refused by name, `RenodeBuildNotPinned`, exit `4`, as is one that names none, and a plain load is now held to the pinned version too: a stock 1.17.0 judges nested interrupts wrongly with no warning and hard-faults unmodified ArduPilot (`AET-WO-4400f` §2). `doctor` holds Renode to the same build. The build is typed once, `aethelgard_twin::platform::RENODE_BUILD`, and a tier-1 test holds it equal to `container/smoke.sh`'s line.

**The clock** (`AET-WO-4400g` §1.1). The emitted platform builds every timer, USART, LPUART1 and the DWT at the solved clock's frequency, the reset clock's 64 MHz (RM0433 Rev 9 §8.7.2, Table 57, §8.7.21–§8.7.22), with an update entry each, never at the stock file's 250, 200 and 125 MHz. A frequency set at construction holds a timer's counter and its compare channels alike, and no RCC push undoes it at the timer's reset (`AET-WO-4400f`). The motor judge predicts each edge at the period Renode realizes, ⌈ticks × 10⁹ / F⌉ ns (`aethelgard_chaos::record::realized_ns`): a DShot600 bit of 106 ticks is 1,657 ns at 64 MHz, where silicon's is 1,656.25 ns, which only the silicon check judges. A specification that declares `mcu.foreign_clock` carries a foreign firmware's clock. A generated run drops it and runs at the solved clock, so `generate` writes the generated platform as `<project>.repl` and the foreign one beside it as `<project>.foreign.repl`, which `--foreign` takes (`AET-WO-4412` D1 (a)). For an ELF that names ArduPilot's `AP_HAL::panic`, `--foreign` holds the declaration to ArduPilot's own tree for the declared oscillator (`aethelgard-ir`'s `data/ardupilot-h7-clock.tsv`, cited from the pinned image's `stm32h7_mcuconf.h`) and refuses another by name (`ForeignClockNotArduPilots`, `ForeignClockOscillatorUnsupported`, exit `3`; D1 (b)).

**The bounds** (`RUL-13` as amended for `simulate --firmware` only, ruled on PR #56, B1): the reaper kills Renode's whole process tree after **120 s** of wall-clock time, a hang guard (`AET-WO-1918` (2), ruled (b), superseding `RUL-13`'s 30 s; the judged bound is the boot's virtual time, and the worst boot measured over eight CI runs took 23.6 s), or once the tree's resident memory (the sum of its processes' `VmRSS`, read every 100 ms) passes **1.5 GiB** (1,572,864 KiB). A droop scenario's boot, Renode as the bridge's child, is a boot and takes the same bounds; the bridge's own runs keep 1 GiB (below). A kill at the memory cap is then a `fail` that names it, `memory_cap_exceeded`, and either kill is `cpu_aborted` if Renode logged `This machine has aborted` first (its `RunFor` then never returns). Nothing is read, and `tool.exit_code` is `-1`. A plain kill at the wall hang guard judges nothing about the firmware, so it is refused as a `ToolError` (exit `5`) and writes no verdict and no campaign run; replay refuses a run that names `wall_bound_exceeded`. Tests set a shorter guard with the hidden `simulate --wall-bound-ms`, at most 120,000, which the verdict records as `wall_bound_ms`; `verify`, `campaign` and the MCP tools have no such flag. The highest sample the bound saw is reported back through the reaper's status file, and the verdict records it beside `boot`, as `memory`: `{"peak_rss_kib":…,"bound_kib":1572864,"share_permille":…}` (`AET-WO-2302` D6: no bound change, each board's margin recorded; the Sentinel-02-shaped board took about two thirds of the former 1 GiB). It is a sample, so it differs run to run: it is in no finding, not in `boot` and not in a campaign's archive. The constants are measured: the reference boot at `--for 100ms` took at most 2.59 s and 404,620 KiB in the tools image, and the ruling asks at least 10× and 2× of those. The memory cap is 1.5 GiB by a later ruling (Scotty, 2026-11-23, `AET-WO-2313`): Tviburi measured ArduPilot's full boot at 768,856–1,049,556 KiB across CI runs of nearly the same tree, for a cause not yet known, so a 1 GiB cap passed or failed it by chance. `RLIMIT_AS` cannot do it, because CoreCLR will not start under any address-space limit. **A platform load** (plain `simulate --platform`, and the MCP `simulate` tool, which runs it) **and every `renode --version`** (`simulate`'s, before each load and boot, and `doctor`'s) take the same 120 s hang guard, with no memory cap (`AET-WO-1919` (2)). The slowest load measured took 5.1 s in CI and 7.35 s on a loaded Furnace. A kill there is refused as a `ToolError` (exit `5`) naming the load or the `--version`, never a finding or a verdict. `--wall-bound-ms` shortens a load's guard too, and its verdict records it. Every Renode the CLI starts is now bounded. `prove` and `check` stay unbounded, and the caller's timeout ends them.

The reader removes a stray `\r` at either end of a line, matches each marker as a whole line, and takes each read only as a line that is a bare `0x…` value. Every log line before `BEGIN` is the run's; a whole log line between the markers is the judge's own reads' (a read of a peripheral whose clock is off logs one) and is never attributed to the firmware. Anything else between the markers is exit `5`.

**What is judged.** The expected registers come from the solved topology alone (`INV-07`), never from the firmware:
- each solved pin's `MODER` field (alternate function, or output for a `gpio` net);
- its `AFRL`/`AFRH` field for an alternate function;
- each cold RCC register in full, which must hold exactly the solved enable bits;
- and the core's final `PC`, which must lie inside an executable `PT_LOAD` of the firmware;
- after the boot word, the core in thread mode (`ICSR.VECTACTIVE` 0, else `core_in_exception`) and its `PC` on a `yield; b.n` self-loop read from the ELF's own bytes (else `pc_not_on_self_loop`), `AET-WO-2204` D3. **Named residual:** a pass claims the core parked in thread mode on a self-loop after the boot word, not that the loop is the generator's fixed point; a firmware that writes the boot word and parks on another thread-mode self-loop passes, and an interrupt handler that runs and returns after the boot word is not seen (`AET-WO-2204` adversarial A2). The loop's two halfwords are read back from the machine after the run, since a firmware can rewrite its flash in Renode, and an executable segment whose virtual address is not its physical one is refused.

**The named residual** (ruled on PR #56, B2): the boot word witnesses that the solved configuration happened and that the core kept executing inside the firmware's own code; it does not witness that `main` returned.

**Windows and anchors** (`AET-WO-2302`, ruled (a-i)). Every board boots `MAX_VIRTUAL_NS`, 100 ms, the one window the verdict's `boot.window` names (a foreign boot's says `FOREIGN_MAX_VIRTUAL_NS`); a campaign line names none, since every campaign boot's is this (`AET-WO-2310`, `2302` reduction 2); there is no per-board boot window any more (a DShot board booted 2 ms, refused past it as `BootPastWindow`, under `AET-WO-4300a` D1). Each witness gets the window its own evidence needs, anchored where that evidence starts and bounded by the judge, per boot, in the boot it reads (the last, after a droop's release):
- **The CLI's own hooks** (D3, the first monitor Python the CLI composes, `src/anchor.rs`): a `DoubleWord`, a `Word` and a `Byte` `Write` watchpoint on each motor timer's `CR1` (adversarial pass 1, M2: a CPU's 16-bit or byte store of `CEN` anchors and is stopped as a word store is; each width fires on its own stores only, measured), which log every write with the word before it (a hook runs before the write, measured), so the judge decides each `CEN` rise; one on the boot word; and one on each serial port's `TDR` but the last's. Each logs one line, `[WARNING] sysbus: AETHELGARD-<TAG> <fields>` (`CR1`, `STOP`, `CLEARED`, `BOOTWORD`, `INJECT`, `TDR`, `STALE`), fields as decimal counts and Python's `%x` words, no specification text; the judge reads each whole after its validated stamp, and refuses (exit 5) one in no shape it writes, or whose count is ahead of its stamp (no core runs ahead of its time). A count's instant is the boot's start (0, or the last release plus its nudge) plus 10 ns an instruction. A line whose count lags its stamp is the core's, never the tool's (adversarial pass 1, M1): the core executed fewer instructions than its time held (it slept, `WFI`), so the verdict names `cpu_halted` with that line (exit 5 only if the core still executed the whole run's instructions: the witnesses disagree), and the line's instant is its stamp: an anchor there falls back to it, and a stop's instant is its window's end. An action that falls due while the core lags is stale: it does nothing and fails by its own name (`judge_stop_missing`, a port's frame not injected). A boot that sets its boot word and still executed fewer instructions than the run holds is the run's finding, `cpu_halted`, exit 1, and a campaign runs on to its next scenario (`AET-WO-2310`, which overturns `AET-WO-2100`'s ruled A10 on purpose, `2302` Q1: a firmware could abort a campaign that way); replay requires `cpu_halted` on a booted run exactly when kind 10 counts fewer instructions than the run holds. A peripheral's hook survives `machine Reset`; a hook on RAM is not fired again after it (re-adding it fires both), so the boot word's and the ports' hooks go in once, before the boot the judge reads: before the run, or at a droop's last release.
- **A DShot output's window** runs from its timer's first `CEN` rise in the boot to `DSHOT_WINDOW_NS` (2 ms) after it. At that instant the hook's scheduled action, **the judge's stop**, reads the timer's solved registers back (`STOP`), clears `CEN` and reads `CR1` again (`CLEARED`). It silences nothing: every observer stays at `Noisy` for the whole boot, so an edge after the window (a restart of the timer, a pin driven by hand) is seen and named, never truncated (adversarial pass 1, H1); a stopped timer makes no edge, so a healthy board's log does not grow after its windows. Only an edge Renode models is seen (adversarial pass 2, A2-4, measured): `TIM1.BDTR.MOE` is a tag in Renode 1.17.0, already in the verdict's `unobservable` list, so a firmware that toggles it after the window moves no observer and passes, though on silicon `MOE` gates the outputs; `CCMRx` forced output modes (`OCxM` forced active or inactive) do move the pin, and are named `motor_edge_after_window`. The read-back is judged against the solved values (`judge_stop_mismatch`), and the end-of-run read of that `CR1` then expects `CEN` cleared, the judge's own act. **Named residual** (D2, accepted), on every verdict of a board with a DShot output, in `boot.residuals`: "after its window a firmware reading its timer sees it stopped (CEN cleared by the judge): a twin artifact". The stop is `boot.stops`, and kind 30 in a campaign.
- **A PWM output's window** runs from its timer's anchor to the end of the boot; it is not stopped, and its two-period rule is unchanged (`AET-WO-4300a`).
- **Every action is checked stale first** (F5): a rendered reset restarts the instruction count, and an action an earlier boot scheduled still fires in a later one; it acts only when the count is exactly its anchor's plus its delay's, and otherwise logs `STALE` and does nothing.
- **The bounds, each by name:** edges with no anchor observed (a DMA write of `CR1` fires no hook): `motor_anchor_unobserved`, the witness `unanchored`. **Named residual** (adversarial pass 1, M2): a DShot timer started by DMA is never stopped, so it runs to the boot's end; started late its edges are read and each output is named `motor_anchor_unobserved` (tier 2, a start at about 96.9 ms, inferred from the spin count); started early it runs about 98 ms of DShot and fails closed only by a bound, never as a pass: measured on Furnace its log passes the filter's 8 MiB budget (1 773 188 lines dropped) at 98.5 % of the memory bound when it was 1 GiB, about 657 per mille of today's 1.5 GiB (`renode_log_flood`, each output `motor_output_unwitnessed`), and on a CI shard the reaper's then 30 s wall bound killed it first (34 s, 851 per mille of the then 1 GiB, about 567 of 1.5 GiB); since `AET-WO-1918` (2) such a kill is a `ToolError` under the 120 s guard, so a slow machine refuses the run rather than naming it; a second `CEN` rise in one boot: `motor_anchor_repeated`; the anchor before the boot's last device step's witness, after its boot word, or with no boot word observed: `motor_anchor_out_of_order`; a DShot window ending past the boot (F2's late anchor: `CEN` at 99.07 ms): `motor_window_past_run`; no stop for the anchor in the boot, or one that left `CEN` set: `judge_stop_missing`; an edge before the anchor or after the window: `motor_edge_before_anchor`, `motor_edge_after_window`. The witness itself (rises, period, high time, off-grid, `covers`) is unchanged, judged on the edges inside the window against the window's end. Each motor entry's `twin` carries `anchor_ns`, `end_ns`, `edges_before`, `edges_after` and `cen_rises` (its timer's `CEN` rises in the boot), as kind 26 does.
- **Devices** are judged from reset, as before; the anchors' order puts every device step before the outputs start.

`GPIOx.OTYPER` and `I2Cn.TIMINGR` are solved but Renode 1.17.0 stores neither, so the verdict lists them as `unobservable`; so are `PWR.CSR1.ACTVOSRDY` and `PWR.D3CR.VOSRDY` on a generated run, which the emitted PWR reads 1 whatever happens (below).

**The PWR** (`AET-WO-4240v` §1.6). A generated firmware writes `PWR_CR3`'s supply bits once, waits for `ACTVOSRDY`, writes `PWR_D3CR` = `0x8000` (VOS Scale 2) and waits for `VOSRDY`, before any RCC write. The stock `pwr` is a script that drops every write; the emitted platform of a generated run replaces it with an update entry written from RM0433 Rev 9 §6.8 (`aethelgard_twin::platform::pwr_override`): `PWR_CR3` reads revision V's power-on `0x46` until its first write, stores that write and ignores later ones, `D3CR` stores `VOS` with `VOSRDY` 1, and `CSR1` reads `ACTVOS` as the stored `VOS` with `ACTVOSRDY` 1. A platform with `mcu.foreign_clock` keeps the stock stub. Measured in Renode 1.17.0 (`AET-WO-4240v` FEEDBACK): the entry replaces the script, `machine Reset` re-runs its initialization, and `LogPeripheralAccess` logs its accesses as any model's (`[INFO] pwr: [cpu: …] WriteUInt32 to 0xC (unknown), value 0x42`). Every generated boot logs the RCC's and the PWR's accesses, serial port or not, and once the boot word is set `aethelgard_twin::serial::judge_pwr` holds them to the declared supply and Scale 2 (`aethelgard_ir::vos`'s words, never the generator's): one `CR3` word write with the declared supply bits and every other bit as last read, then one `D3CR` write of `0x8000`, both before any RCC enable write. Any other PWR write, a second write, the reverse order, a wrong supply bit, another `VOS`, a PWR write after an RCC enable, or either never written is one `pwr_access` finding naming each; a log not read whole is `pwr_access` too. A foreign firmware is never given the rule. A boot word of `0x2100B0F1`–`F3` is named in `boot_word_missing` (`boot_supply_not_declared`, `boot_actvosrdy_timeout`, `boot_vosrdy_timeout`). **Named residual**, in every generated verdict's `boot.residuals`: the twin never sees a wait time out; it models no Run* mode; no twin clock depends on VOS, so a firmware left at VOS3 runs the same and only the IR's VOS rule refuses it; and `PWR_CR3`'s lock across a warm reset is modelled only as far as the stored first write (the twin's reset re-runs the script's initialization). A specification whose `mcu.core_supply.mode` is `bypass` is refused by `generate`, `emit` and `simulate` as `CoreSupplyNotGenerated` (exit `2`); `validate` and `solve` accept it. So is one that declares `mcu.hse` (`AET-WO-4240a`), as `HseNotGenerated`, until `AET-WO-4240b` writes the HSE bring-up; its `acknowledge_unwitnessed` is left unjudged, as `bypass`'s is. A topology with a pin device (`AET-WO-1211a`) is generated (`AET-WO-1211b`: its GPIO, its ADC inputs and its bridges' enables), with no Renode platform; `simulate` refuses it as `PinDeviceNotSimulated`, naming the device's reference and part, and one with a motor output whose `pwm.idle` is a high level as `IdleHighNotSimulated`, until `AET-WO-1211c`; `emit` refuses a pin device as the schematic emitter's `PinDeviceNotDrawn` until `AET-WO-1211d`; each exits `2`.

**The HSE** (`AET-WO-4240b` §1.6). On a board whose solved clock has an HSE (`clock.hse`), the emitted platform ends its clock entries with an update of the stock RCC, `rcc:` / `hseFrequency: <clock.hse_hz>` (measured in Renode 1.17.0: taken, `sys_ck` 64 MHz after `SW` on both fixtures; a second full declaration is Renode's error E02); a board with no HSE emits none. Its boot's log keeps the RCC's reads of `CR` and `CFGR` (every other boot's filter is unchanged), and once the boot word is set `aethelgard_twin::hse::judge_clock` holds the RCC's, the flash controller's and the PWR's accesses to the H1 bring-up, every word from the topology, RM0433 and ST's header, never the generator's: after `PWR_D3CR`, `RCC_CR` ← `HSEBYP` (external only), `HSEON`, a 32-bit `CR` read with `HSERDY` 1, `HSECSSON`; `PLLCKSELR`, `PLLCFGR`, `PLL1DIVR` as solved; `PLL1ON`, a `CR` read with `PLL1RDY` 1; `FLASH_ACR` ← `clock.hse.flash_acr` and one read of it; `CFGR` ← `SW` = PLL1, a `CFGR` read with `SWS` = PLL1; every RCC enable write after that read; no kernel select register written. A word missing, repeated, out of order or other than solved, a wait's read missing, `SW` before the `PLL1RDY` read, a select written, an enable before the `SWS` read, or `FLASH_ACR` other than once written and once read is one `clock_access` finding naming each; a log not read whole is `clock_access` too. The per-port RCC rule leaves those words to it. A boot word of `0x2100B0F4`–`F8` is named in `boot_word_missing` (`boot_hse_not_ready`, `boot_pll1_not_locked`, `boot_sw_not_taken`, `boot_hse_css_fault`, `boot_flash_acr_not_taken`). **Named residual**, in an HSE board's `boot.residuals`: `HSERDY` and `PLL1RDY` read their ON bits at once, so no wait times out and the twin under-counts the boot by up to the crystal's start-up plus 150 µs; `SWS` follows `SW` at once; `HSEBYP` and `PLLCFGR` are tags; `HSECSSON` has no clock security system behind it; every kernel select reads 0; `FLASH_ACR` stores with no wait state. Those fields are in `boot.unobservable`.

**`boot` in the verdict:**

```json
"boot":{"firmware":{"path":"…","sha256":"<64 hex>","image_sha256":"<64 hex>"},
        "specification_sha256":"<64 hex>","seed":2100,"virtual_time_ns":10000000,"mips":100,
        "instructions":{"expected":1000000,"observed":1000000},"pc":"0x08000490",
        "boot_word":{"address":"0x20000000","expected":"0x2100b007","observed":"0x2100b007"},
        "registers":[{"name":"GPIOA.MODER","address":"0x58020000","mask":"0x0000fc03","expected":"0x0000a801","observed":"0x0000a801"},…],
        "unobservable":["GPIOB.OTYPER","I2C1.TIMINGR","PWR.CSR1.ACTVOSRDY","PWR.D3CR.VOSRDY"],
        "i2c":[{"reference":"U2","address":"0x76","step":"identity","register":"0xd0","width":1,"expected":"0x60","mask":"0xff","observed":"0x60","status":"ok"}],
        "i2c_witness":[{"peripheral":"I2C1","reference":"U2","step":"identity","device":"i2c1.u2",
                        "expected":["[NOISY] i2c1.u2: Write D0","[NOISY] i2c1.u2: Read 60"],
                        "observed":["[NOISY] i2c1.u2: Write D0","[NOISY] i2c1.u2: Read 60"],"absent":[]}],
        "icsr":"0x00000800",
        "residuals":["PWR (AET-WO-4240v): the twin never sees ACTVOSRDY or VOSRDY low, …"]}
```

**The I2C identity reads** (`AET-WO-2202`). The specification's I2C devices are placed on the peripheral their bus was solved to (`SolvedTopology::i2c_devices`, `AET-WO-2203`). For each device whose part has a row in the part table, the reference firmware reads the part's identity register once, through the board crate's bounded `transfer`, before it writes the boot word (`AET-WO-4241`). A device whose part has a Renode 1.17.0 model (today the BME280, `I2C.BME280`) is declared in the emitted platform at its bus and address; an unmodelled device's read comes after every modelled step.
- **The record.** Each read leaves two words after the boot word, in device order: a status (`0x22020000` OK, or the wait that expired: `…01` TXIS, `…02` TC, `…03` RXNE, `…04` STOPF), then the bytes read, first byte most significant. Before the run the judge writes `0xFFFFFFFF` to every record word, as it does the boot word, so a record the firmware never wrote reads as that fill.
- **The expectation** is the part table in `aethelgard-ir` (`parts::identity`: BME280 register `0xD0`, `0x60`; TMP117 register `0x0F`, `0x0117`), never anything the firmware wrote.
- **Findings:** `i2c_identity_mismatch` (status OK, another value), `i2c_timeout` (naming the wait), `i2c_record_invalid` (a record no transfer writes: the fill, or another word). A script's later steps name a mismatch by their step, `i2c_configure_mismatch` or `i2c_data_mismatch` (`AET-WO-4210` F-B10), and SPI's likewise. A record is judged once the boot word is set. Before that, `boot_word_missing` already says the run failed, and only a timed-out record is named, as `i2c_timeout`: the record is the firmware's own (`AET-WO-3201`, `3100` F15). `identity_read_outside_window` (a record still at the fill when a DShot board's 2 ms boot ended) is no longer given: every board boots 100 ms (`AET-WO-2302`).
- **`identity_not_witnessable`** has severity `info`: the device's part has no model in the twin (the TMP117 has none in Renode 1.17.0), so the firmware's read of its identity is made but cannot be witnessed; its record is the twin's timeout, a twin limit and not a prediction (`AET-WO-4241`). `boot.not_witnessable` gives the count, each read's measured twin cost and the total, and `verify`'s `twin_limits` lists `IDENTITY.<ref>` (silicon-only). It is reported on every boot and does not make the verdict `fail`. A foreign firmware's judge still names an unmodelled device `device_unmodelled`.

- **The witness** (`AET-WO-2204` D1, replacing `AET-WO-2202`'s controller `CR2`/`TXDR`, which a plain store sets). The record is the firmware's account; the witness is what Renode itself logs when the controller runs the read on the device's model: for the BME280, `[NOISY] i2c1.u2: Write D0` then `[NOISY] i2c1.u2: Read 60`. The expected lines come from the part table and the emitter (the device's path is the controller's stock name, then the reference lower-cased), never from the firmware. For every record whose status is OK the judge requires exactly those lines from the device, matched as the whole text after the host timestamp, in order: so each read on a bus is witnessed, not only the last. Each device is judged by its own steps, never across devices, so two devices' transactions may interleave (`AET-WO-4210`, `2204` A4). A difference is `i2c_transaction_unwitnessed` (naming the device and the step, what Renode logged and what was expected); a model with no known witness format for the step is `i2c_witness_unavailable`. A device's `[ERROR]` line is one of its lines too: it fails the step it falls in by name, whatever the record says, as `i2c_step_error` or `spi_step_error` (naming the step and the line), beside the run's `renode_error_log`; on I2C it counts toward no step's expected lines and goes to the step under way (`AET-WO-4210` D-A1, ruled). Past the output filter's 8 MiB budget a line may have been dropped, so every read is unwitnessed, never passed. **Named residual** (adversarial A3): a static pass means the right device returned the table's value during the run, not that the record was derived from that read (a firmware that writes the record first and transfers after passes; the droop's probe does check the order). The verdict's `boot.i2c_witness` lists, per read, `device`, `expected`, `observed` and `absent` (the addresses the controller found no device at, kept for attribution, never judged).

**Scripts** (`AET-WO-4210`, ruled on PR #131). A part whose table entry has a `script` is brought up in three steps, each its own transaction and its own record: the identity read, one configuration write (nothing read back), and one data read. The ICM-20948 wakes (`PWR_MGMT_1` `0x01`) and reads `ACCEL_XOUT_H` to `ACCEL_ZOUT_L`. The ICP-10111 soft-resets (`0x805D`) and reads its temperature (`0x7866`, two bytes). Both are twin stand-ins and `eol` in the table.
- **The records.** Each step's record is a status word, then as many value words as its read needs, at least one: a read of up to four bytes is two words, as every identity read always was. A board without a script keeps its layout and its firmware text byte for byte.
- **The expectation.** An identity's `mask` fixes only the bits the table states (the ICP-10111's bits 5:0). A value with any bit above the read's width fails. A configuration's value is 0. A data read's value is the datasheet's conversion of the stimulus the judge sets on the model before the run (`sysbus.<device> DefaultAccelerationZ 1`, `DefaultTemperature 25`). That is the judge's own act, never the firmware's: 16,384 for 1 g at ±2 g, and `(25 + 45) × 65536 / 175` → `0x6666`.
- **The witness.** Each step's expected lines are the model's, as the twin lists them, with the warnings a correct step makes the model write where it writes them (the ICM-20948's `CLKSEL` tag on the wake), each pinned to Renode's source. Any other `[WARNING]` from the device in the step, or a declared one missing, fails it. The ICP-10111 is read at `Debug` (`logLevel 0`), the only level its transactions log at.
- **Per-read windows on SPI** (D5). The multiplexer is set to `Noisy`, and its `GPIO #k set to False` … `True` lines are the device's chip-select windows: a device's `k`-th window holds its `k`-th step. A line in a later window or in none goes to its last step, so it can only fail a witness. A multiplexer line routing a byte to no single device is attributed to the window it falls in, or, in no window, to every device's identity step. The flash's `Transmission finished in the unexpected state: RecognizeOperation`, which the multiplexer causes on every edge (F11), is skipped only on the line directly after the multiplexer's `Finisihing transmission on device` line for its own address.
- **No interrupt armed** (`2204` A2, ruled D6). After the run, `SysTick.CSR`'s `ENABLE`, `TICKINT` and `COUNTFLAG` (mask `0x10003`) and `NVIC.ISER0` to `ISER4` (the part's 150 lines) must read 0, as `register_mismatch` judges every register. Renode reads `CLKSOURCE` as 1 whatever is written (`NVIC.cs` L1040), so that bit is not judged.

**The serial ports** (`AET-WO-4300b`; `aethelgard_twin::serial`). Renode 1.17.0's `UART.STM32F7_USART` passes a byte to its backend only from a `TDR` write with `TE` and `UE` set, and routes no GPIO to the USART, so the composition witnesses bytes and reads the pins' mux itself. For a board with ports, the boot:
- makes a fresh capture directory in its scratch before any tool runs there (as root, the one directory the tool uid may add to), and puts `sysbus.<usart> CreateFileBackend @<dir>/<usart>.bin true` per port before the first `RunFor`;
- injects each port's **frame** by the CLI's own hooks (`AET-WO-2302` D4, sequential), one byte per scheduled action written with `WriteChar`, paced: the first port's byte `k` one quantum (100 µs) after **this boot's boot word** plus `k` times the port's spacing, one character's wire time at the solved baud rounded up to a whole quantum (86 876 ns at 115 200 8N1, so 100 µs; adversarial pass 4, ADV4-2: Renode queues a byte at its `WriteChar`, so a frame written at one instant let a firmware that hardly waits after the first byte echo all eight). Each later port goes in the same way one quantum after the port before it has echoed its `FRAME_LEN`-th frame byte (F1, measured: a frame injected before a settle, or beside another port's, waits in Renode's queue and comes back in microseconds): a hook on that port's `TDR` logs every write after the boot word (`TDR`) and, once the port's first frame byte went in, counts a write only when it is the frame's next byte, so a pattern sent late or junk never counts (adversarial pass 1, L1). The judge recounts the echo from the logged bytes the same way and names a port any byte of whose frame went in before the port before it had echoed its whole frame (`serial_injection_out_of_order`, L2). A port whose predecessor never echoes its whole frame is never injected, and fails closed by name. A port whose RX line the campaign's scenario holds open (`serial_rx_open`, `AET-WO-2302` D5) gets no frame and no `TDR` hook: the chain skips it, so the port after it goes in one quantum after the last echoed byte of the port before it that is not open (after the boot word when none is). The firmware still polls the open port for its echo while that next frame waits in Renode's queue, so the next port's pacing is not tested in such a run (F1's lost test, never a false pass); its echo is judged as ever. The first byte's action reads the port's RX pin's `MODER` and `AFR` and logs them on its `INJECT` line; each byte is stale-checked as every action is. The twin spaces received characters but renders no overrun: a byte waits in Renode's queue however late the firmware reads it. The frame is the first 8 bytes of `sha256("aethelgard-serial-frame" ‖ firmware sha256 ‖ label)`: no firmware can hold it when it is built. A frame that did not go in once, whole and in order (a boot word written over another word twice), is no injection. Before the boot, only what no boot word can save is refused, from the earliest instant, one quantum after the last boot's start, against `MAX_VIRTUAL_NS`: the instant and the quantum after it (`SerialInjectionPastWindow`, exit 3), or the ports' patterns, paced frames and one echoed character each, summed in the specification's order (`SerialWireTimePastWindow`, exit 3). After the run, the ports' wire time after the boot word, from its quantum past the boot's end, is `serial_wire_past_run`: each injected port's paced frame and one echoed character, summed in the specification's order, without the patterns, since the generated bring-up sends every pattern before it writes the boot word (adversarial pass 2, A2-3: a 9600 8N1 console whose boot word fell at about 79.5 or 87 ms echoed its whole frame inside the run yet was named, and is not; the last boot word that fits there is at 90 058 281 ns), and without a port the scenario holds open, which gets no frame. A port into which no whole frame went (the boot ended first, the port before it never echoed, no boot word observed, the log not read whole, the scenario holds its RX line open) fails its echo by name, once the boot word is set (`serial_echo_missing`, saying why). Each port's `twin` carries `injected_ns` and `injected_bytes`, and `boot.boot_word_ns` the anchor.

The judge reads each capture whole, by descriptor and through no link, and names anything else in the directory `serial_capture_forged`, booted or not. Once the boot word is set, each port's capture must be its **pattern** (the label's bytes, then `00 FE 7F 80 FF`) `k` times, `1 ≤ k ≤` releases + 1, then the frame exactly: a missing pattern is `serial_pattern_missing` (also when the TX pin is not in AF mode on its solved AF at the first `TDR` write of any boot or at the end, or is taken off it once solved, as the interval rule below names: `tx_at_send` is then that mux; a change of its type, speed or pull alone is the tags' finding alone), stray bytes or extra patterns `serial_capture_forged`, and a frame short or absent `serial_echo_missing` (also when it came back with the RX pin off its AF at the injection, or changed in any field once solved, as the interval rule below names: `echo` `not_counted`; adversarial pass 4, ADV4-4). The framing's stored fields (`CR1`, `CR2.STOP`, `CR3`, `BRR`) are in the register image; the tags `M0`, `M1`, `TXINV`, `RXINV`, `ONEBIT`, and `FIFOEN`, which the bring-up sets, are stored nowhere and listed `unobservable`. Renode's access log (`sysbus LogPeripheralAccess` on each port's USART, its pins' GPIO ports and the RCC) is read instead, every boot from reset: in each boot, the four words in force at the write that first sets `UE` must be the bring-up's, tags and `FIFOEN` included, none may change after it, nothing the bring-up never writes may be written (`PRESC`, `GTPR`, a sub-word write, `TDR` before `UE`), and no word but `ISR` and `RDR` may be read (a configuration word reads back without its tags in Renode, so a firmware could write back what it read and differ on silicon); the RCC may be written only in its run-mode enable registers (`AHB3ENR` at 0xD4 to `APB4ENR` at 0xF4), as words, and no write may clear the gate of the port's USART or of its pins' GPIO ports once set (a prescaler, a kernel clock selection or a reset changes or stops the port's clock on silicon); each pin must be push-pull, low speed and pulled neither way (the TX pin at the first `TDR` write, the RX pin at the injection, both at the end), and once a pin's mode, alternate function, type, speed and pull first all hold their solved values in a boot, no later write in that boot may change any of them (Renode emits a byte at its `TDR` write and spaces the frame's bytes no finer than a quantum; silicon shifts each over a character's time); no write to either pin's port may be refused as a locked pin's; and the last boot must set `UE`. A failure is `serial_tag_mismatch`, which counts the USART's writes in the last boot; a log that cannot be read whole (past its budget, or a line of a logged source that is not a whole access) fails it too. The silicon check, `aethelgard_ir::serial::check_silicon` against RM0433's receiver tolerance, is its own finding (`serial_framing_static_mismatch`), from the topology alone. `boot.serial` holds one object per port, in the specification's order (label, role, peripheral, pins, framing, `brr`, `actual_millibaud`, `error_ppm`, `silicon` with its row and `tolerance_source`, `twin` with the pattern, frame, patterns, echo and pins' mux and tags, and `speed`: "serial pin speed not judged: off-board load undeclared"), and is absent for a board without one, whose commands are unchanged.

Every `observed` (and `mips`, `pc`) is `null` when nothing was read, as after a load error. `firmware.image_sha256` is the sha256, over every `PT_LOAD` with memory in physical-address order, of its physical address, file size and memory size (each a `u32` little-endian) and then its file bytes: the memory image Renode builds. It is the firmware's identity, because the file's own hash also covers symbol names, which Cargo derives from the checkout path.

### Foreign firmware (`simulate --foreign`)

`AET-WO-4400e`, ruled on PR #212 (comments 6958822008 and 6965511448), with D4 ruled on PR #215 (comment 6961943794). The foreign judge is a second judge, for a firmware this project did not generate, such as unmodified ArduPilot.

**It is declared, never inferred** (`INV-06`):
- `simulate --firmware F --spec S --foreign TIME` chooses it. `TIME` is the virtual time the boot runs, as `--for`'s, at most `FOREIGN_MAX_VIRTUAL_NS` (5 s), or with `--input` `FOREIGN_INPUT_MAX_VIRTUAL_NS` (12 s: an armed input run, `AET-WO-4400j` D3).
- **Its clock is declared** (`AET-WO-4400g` §1.2, Q1 ruled on PR #229): `S` must declare `mcu.foreign_clock`, the firmware's timer kernel clocks per APB bus and each USART's, each `{value, source}`; without it the run is refused by name, `ForeignClockUndeclared`, exit `3`, never booted at the solved clock, which is the generated firmware's (ArduPilot would run 0.32× under it, measured). The platform builds every timer and USART at the declared clocks; LPUART1 and the DWT, which no declaration sets, keep the stock file's, and the verdict names them.
- That bound is 5 s (`AET-WO-4400g` §1.4): at its declared 200 MHz, unmodified ArduPilot's last witness, the ICM's first FIFO read, is at 4.394 s (it was 3.61 s while the twin ran its timers at 250 MHz). Its RSS margin under concurrency is `AET-WO-4400g`'s F1 measurement; the memory bound, 1.5 GiB since `AET-WO-2313`, is a ruling of its own.
- `--for` and `--foreign` together are refused (exit 64). `--for` keeps its 100 ms bound.
- The verdict's `boot` says `"judge": "foreign"`. A generated boot's verdict names no judge and is byte for byte what it was.

**What is the same:**
- the emitted platform, which must be exactly `S`'s;
- the analog pre-pass;
- the sandbox, the 120 s hang guard and the 1.5 GiB bound;
- the cold RCC writes.

**None of the generated runtime's contract applies:** no boot word, no records in RAM, no serial injection, no motor window. The three hooks a foreign boot adds only log: each SPI controller's `CFG2` writes (D4), the flash controller's erase starts, and each entry to the firmware's panic (below).

**The panic** (`AET-WO-1609`, ruled at §1.2; `spike/1609`): ArduPilot's `AP_HAL::panic` prints and sleeps forever in the calling thread, so a run that panics after its last witness ends parked in thread mode with every witness logged, and passed until this hook.
- The judge finds `_ZN6AP_HAL5panicEPKcz`, a function symbol, in the ELF's symbol table, inside an executable `PT_LOAD`, and sets an execution hook on it that logs a `[WARNING]` of the CPU's own on each entry. No firmware can write that line.
- A run that enters it fails, `foreign_panic`, naming the first entry's virtual time and the last device witness's. Every entry is in `boot.panic.entered_ns`, beside the `symbol` and `address`.
- A firmware whose symbol table does not name it is refused before any boot, `ForeignPanicSymbolMissing`, exit `3` (`INV-06`). Two addresses under the name, or one outside every executable segment, is `MalformedFirmware`.
- **It does not judge** a fault, a hang or a reset that never calls the panic. A thread-mode hang after the last witness passes. A fault is the next paragraph's.

**A fault** (`AET-WO-2314`, `spike/2314`): the judge reads the four fault handlers (HardFault, MemManage, BusFault, UsageFault) from the ELF's vector table at `FLASH_BANK1_BASE`, the address the boot writes to VTOR, and hooks each one. When `ICSR.VECTACTIVE` is the hooked vector, the hook logs a `[WARNING]` of the CPU's own and halts the core, so the handler's own loop never reaches the log budget or the memory cap.
- A run that takes one fails `foreign_fault`, naming the vector, the PC the exception stacked (and that frame's address) and the virtual time. That holds beside a bound too, should one still end the run.
- A table entry that is 0, outside every executable `PT_LOAD`, or not loaded is refused before any boot: `ForeignFaultVectorUnusable`, exit `3`.
- **It does not judge** a handler reached without an exception, or a VTOR moved at run time. Only HardFault was exercised.

**The memory rule:**
- Every `PT_LOAD`, at both its physical and its virtual address, lies inside flash or one of the six RAM regions ST's header states (ITCM, DTCM, AXI SRAM, D2 SRAM, D3 SRAM, backup SRAM).
- Each base is its `#define`, and each size is the one its comment states (`region_size_kib`). Nothing is typed in (`RUL-16`).
- No two segments' file bytes may overlap. Zero-filled segments may share a physical address, as ArduPilot's `.crash_log`, `.bss` and `.heap` do.
- An outside segment is `FirmwareOutsideMemory`, naming every region.

**What witnesses the boot:** only what the firmware cannot write (`INV-07`): the models' own transaction lines and the platform's state.
- **The clock** (`AET-WO-4400g` §1.2, Q2 ruled on PR #229): at the run's end the judge reads back `RCC_CR`, `RCC_CFGR`, `RCC_D1CFGR`, `RCC_D2CFGR`, `RCC_PLLCKSELR` and `RCC_PLL1DIVR`, decodes the timers' kernel clock from them by RM0433 Rev 9 (`aethelgard_ir::SolvedClock::from_rcc`, each field cited), and holds each bus's declaration to it: a declaration they contradict fails by name, `foreign_clock_contradicted`, with both figures; registers that do not decode fail as `foreign_clock_undecodable`. `boot.clock` lists the registers, each timer bus `checked` (declared, decoded, `holds`), the USARTs `declared_only`, the `stock` instances, and the conditions it rests on: `usart_clock_declared_unchecked` (Renode 1.17.0 drops `RCC_D2CCIP2R`, so no register says what the firmware selected), `pll1_fraction_not_read` (Renode tags `PLL1FRACEN` and `FRACN1`) and `stock_frequency_undeclared` (no judge reads the DWT or LPUART1).
- **Booted** (D5): the first identity transaction a declared device's model logs, found in the device's own lines. In addition:
  - the run ends in thread mode (`core_in_exception` otherwise);
  - its final `PC` lies in an executable `PT_LOAD` (`pc_outside_firmware` otherwise).

  **The end is read four times** (`AET-WO-4400j` D4), one quantum apart, the last at the window's end and never past it: a solved register, or the core in an exception, fails only if it fails at every read, each read in the verdict (`registers[].earlier`, `icsr_earlier`); the PC, the clock and the pulls are the last read's. On a port the specification declares as an RC input, `CR2.STOP` and `CR1.PCE`, `PS` and `M0` are not judged at the end, while ArduPilot searches for the protocol (`rc_protocol_search`, info; `boot.rc_search`).

  The console banner is refused. With no identity witnessed, the verdict names `foreign_not_booted`.
- **Each modelled device** (D6 (i)):
  - its identity transaction;
  - then one of its data reads after it: the script's data step, or a read its part lists in `parts.json` `reads` (the BME280's burst from `0xF7`; the ICM-42688-P's FIFO burst at `0xB0`, at least one 16-byte packet);
  - each with the virtual time of its first line in `boot.devices`;
  - a missing one is `{bus}_transaction_unwitnessed`.

  The configure step is not judged. The bytes of a listed read are not judged either: the stock models answer from the configuration the firmware wrote.

**In CI** (`ardupilot-image.yml`, job `foreign`; `ci/foreign-judge.sh`): unmodified ArduPilot built for the generated `sentinel-02-own` board, whose specification declares ArduPilot's clock (timers 200 MHz, USARTs 72 MHz), passes under `--foreign 5000ms`, every witness with its virtual time, the timer check holding on both buses. Each mutation is refused by name:
- the window short of U5's identity (4300 ms), or of its first FIFO read (4370 ms): `spi_transaction_unwitnessed`;
- ArduPilot without `--foreign`, on the board with no declared clock: `FirmwareOutsideMemory`;
- `--foreign` with no declared clock: `ForeignClockUndeclared`; a generated run on the declaring specification takes the generated platform and meets `FirmwareOutsideMemory`, and the foreign platform is `PlatformNotEmitted` for it;
- the timers declared at 250 MHz: not ArduPilot's tree for an ELF that names `AP_HAL::panic`, so `ForeignClockNotArduPilots` before the boot (`AET-WO-4412` D1 (b));
- SPI2's receive DMA line dropped, or U5's model removed, from the platform: `PlatformNotEmitted`;
- ArduPilot rebuilt from the same hwdef without its `IMU` line: `spi_transaction_unwitnessed` (U5's identity).
- `AP_HAL::panic` planted straight after U5's last witness (`AET-WO-1609`): `foreign_panic`, every witness present. It passed before the hook;
- the panic's symbol renamed: `ForeignPanicSymbolMissing`.
- an undefined instruction at the same site (`AET-WO-2314`): `core_in_exception` and `foreign_fault`, HardFault at the planted PC, under the memory cap.
- a declared input (`AET-WO-4400i`): ArduPilot echoes each `PARAM_SET` and refuses every ARM with its own `Arm: System not initialised` inside 5 s; a planted force code: `InputForcesArm`; a frame that is text: `InputNotMavlink`; a parameter it does not have: `foreign_param_rejected`; USART1's receive request dropped: `PlatformNotEmitted`; the run archived and replayed to the same witness.
- **D4:** the SPI model's `Attempted to set peripheral into SPI slave mode` `[ERROR]` is exempt only directly after a `CFG2` write with `MASTER` 0 while `CR1` holds `SPE` 0, and only when that controller's next `CFG2` write sets `MASTER` again before any of its devices logs a line (or nothing follows). That is ChibiOS's `spi_lld_stop` then its next `spi_lld_start`, legal on silicon (RM0433 §50.11.4). A firmware that leaves the controller a slave and keeps using it is not exempt, since Renode's model stays a master where silicon would not clock (adversarial F1, accepted on PR #212, comment 6967529218).
  - The judge reads it from a hook on each SPI controller's writes, which logs only a `CFG2` write, with the value and `CR1` as it stands (D7 (b)); the SPI access log stays off.
  - Every exemption is counted in `boot.spi_errors_exempted`.
  - Any other `[ERROR]` is `renode_error_log`.

**The flash** (ruled on PR #212, comments 6965511448 and 6966155027: never a write inside its own loaded ranges):
- the CLI hashes each loaded flash segment's file bytes from the ELF, on the host, before the run; Renode reads the same range after it. A difference is `flash_overwritten`. The twin never witnesses itself (`INV-07`).
- a hook logs each flash-controller write that starts an erase or an option change: `START` with `SER` (and `SNB`) or `BER` on a bank's `FLASH_CR`, or `MER` on `FLASH_OPTCR`, at `0x18` or its alias `0x118` (RM0433 Rev 9 §4.9.7, p. 211; the header's struct has only the first, and Renode's model decodes only the first). An erase overlapping a loaded flash segment is `flash_overwritten`, even when reprogrammed alike. Every offset, bit and size is ST's header's.
- `OPTSTART` (an option-byte change) is the `info` finding `flash_option_change`. Its effect on protection is not judged.
- `boot.flash`, `boot.erases` and `boot.option_changes` list all three.
- Weakens: writes elsewhere in flash (a parameter store, say) are not judged. A store that changes a word of an own segment and a later store that restores it, with no erase between, passes. Silicon cannot do that: programming only clears bits, and a bit returns to 1 only by erasing its sector (RM0433 Rev 9 §4.3.9, p. 161), which the hook fails.

**The serial options** (`AET-WO-2311`, ruled at §1.2 on PR #363): Renode stores no `USART_CR2` bit but `STOP`, so each declared port's USART has its accesses logged (`LogPeripheralAccess`) and the judge holds the last `CR2` word written while `CR1.UE` was clear, the word silicon keeps (RM0433 Rev 9 §48.8.3, p. 2075), to the specification alone (`aethelgard_twin::serial::options`): SWAP clear; on RC input TXINV clear and RXINV unjudged, as AP_RCProtocol sets it while it searches; on every other port TXINV and RXINV each as `framing.inverted`. A wrong bit is `serial_swap_set`, `serial_txinv_wrong` or `serial_rxinv_wrong`; a `CR2` write with `UE` set that changes a judged bit is `serial_options_ignored`; a write into `CR1` or `CR2` that is not a word at its own offset, or a log not read whole, is `serial_options_unjudgeable`; no `UE`-clear word is `serial_options_unwritten`. Each serial line's `PUPDR` is read back at the run's end and reported, not judged, as the `info` finding `serial_pull`; `boot.serial_options` and `boot.serial_pulls` list both. In CI the four-port board's ELF passes, a SWAP and an RC-port TXINV mutation (each a `var_info` default patched in that ELF) are refused by name, and with its GPS port declared inverted the unmodified ELF is refused and the inverted one passes. Weakens: an RC port's RXINV; and the words are the twin's sequence, since a firmware that reads `CR2` back reads every tag as 0.

**The log** (D7): the 8 MiB budget stands. Renode's own `Unhandled write` warnings are kept once per kind (the line with its written value elided); `boot.warnings` lists each kind with its count and first and last virtual time. At most 1,024 kinds are summarised; a line of a further kind is kept or dropped as any other, against the budget. **Each repeating per-transaction line** (`AET-WO-4400j` D2) is kept the first time and counted after, with its count and first and last virtual time in `boot.summarised`, so that the judge reads the kept log as it would the whole one: an SPI controller's D4 cycle by its three lines (counted into `spi_errors_exempted`; any other cycle kept as written with its next event), each declared device's and multiplexer's lines by exact text once the device's identity and every multi-line read are found, a listed read's single-line pattern with its values elided after its first match, and the ADC's start-while-busy warning; a device's warning never. The kept log is never reordered. At most 4,096 keys, then lines as written (`boot.summary_capped`). `boot.log_kept_bytes` reports what was kept. A generated boot's filter is unchanged.

**Unclocked accesses:** `locked_peripheral_access` is an `info` finding for a foreign firmware; `boot.locked` lists each access with its first time and count. Silicon ignores them.

**The rule table:** `boot.rules` lists each of today's rules as kept, replaced or dropped, with what it judges or weakens. In short:
- **kept:** solved pins' `MODER`/`AFR`; `core_in_exception`; `pc_outside_firmware`; device warnings; `renode_error_log` with D4; the log flood with D7 (a); the bounds; `device_unmodelled`.
- **replaced:**
  - the memory rule;
  - the boot word, by "booted";
  - cold RCC: at least the solved bits;
  - USART `CR1`'s frame bits and `CR2.STOP`;
  - the device steps, by witness lines alone;
  - the chip-select windows, by the first matching run;
  - the 100 ms bound;
  - `locked_peripheral_access`, by an `info` finding;
  - the flash-controller access rule, by the flash rule above.
- **dropped**, each weakening named in the verdict: the instruction count; the self-loop; USART `CR3` and `BRR`; `SysTick.CSR`; `NVIC.ISER`; the configure step; the records; the serial witness; the brown-out record; the motor outputs.

### A declared input (`simulate --foreign --input`, `AET-WO-4400i`)

```
aethelgard simulate --platform <repl> --spec <json> --firmware <elf> --foreign 5000ms --input in.json [--archive <dir>]
aethelgard replay --archive <dir>/<run id>
```

**The input** is a run's file, never the specification's: timed MAVLink 2 frames for one declared serial port.

```json
{"input_version": 1, "port": "CONSOLE",
 "frames": [{"at_ns": 750000000, "mavlink": "fd17000000ffbe17…", "fill": 29}]}
```

- Each frame's first byte goes at `at_ns`, a whole number of the 100 µs quantum; each byte after it one character time later, by the port's solved framing; then `fill` zero bytes.
  - The fill is declared, never added: Renode 1.17.0's USART raises no IDLE, and its DMA neither ends a stream on an `EN` clear nor restarts it at `M0AR`. So a firmware that receives by DMA reads a frame only when its buffer fills (ArduPilot's is 64 bytes).
  - Every input verdict names the gap: `usart_idle_unmodelled` in `boot.unobservable`.
- Each frame must be exactly one unsigned MAVLink 2 frame of a message in the closed table (`aethelgard-twin` `data/mavlink-messages.tsv`, cited to ArduPilot's own `common.xml`), CRC checked.
- Refused by name, exit `2`, before any tool is looked for:
  - `InputNotMavlink`, `InputMessageUnlisted` (an unlisted message, or one only the firmware sends), `InputSigned`;
  - `InputForcesArm`: an ARM with force code 21196 or 2989, as `COMMAND_LONG` or `COMMAND_INT`;
  - `InputSkipsChecks`: `ARMING_SKIPCHK` set non-zero, or `ARMING_REQUIRE` set to 0;
  - `InputTimeOffQuantum`, `InputFramesOverlap` (a frame before the last one's end and one idle character), `InputPastWindow`;
  - `InputPortUndeclared`, `InputMalformed` (the file's JSON, its version or its hex);
  - `RestOrientationUndeclared` (below).
  - An unreadable file is exit `3`.
- The USART of each declared port carries its receive DMA request on a foreign board's platform (RM0433 Rev 9 Table 124).

**The board at rest** (D4): an input run starts from rest, not from the witness stimuli.
- Each IMU reads +1 g on the chip axis its declared orientation puts up (DS-000347 rev 1.9 §4.7). An IMU with no declared orientation is `RestOrientationUndeclared`.
- Each barometer reads the parts table's pressure, moved by its datasheet's intrinsic noise (`aethelgard-twin` `data/rest.tsv`: the BMP581 ±0.08 Pa every 83.3 ms). Its data read is witnessed at each value the rest set.
- `boot.input.rest` lists each, with its citation.

**The witness** (`boot.input.witness`) is read from the bytes the firmware transmitted on the port, never from the input (`INV-07`): every frame whose CRC holds.
- `armed`: the first heartbeat with `MAV_MODE_FLAG_SAFETY_ARMED`, from a source (system and component id) no input frame has: a heartbeat the input carried, echoed back, is never the firmware's.
- `acks`: each `COMMAND_ACK` (command, result, count).
- `params`: each parameter the input set, with the firmware's echo or its `PARAM_ERROR`. A `PARAM_ERROR` is the finding `foreign_param_rejected`.
- `texts`: each `STATUSTEXT`.

**`--archive`** writes the run (trace 20, kinds 38 to 40: the input file verbatim, the stimuli, the transmission) to `<dir>/<run id>`, the id a function of the input's, the specification's and the firmware's hashes. **`replay`** re-judges it from the run directory alone: the input's hash against the header's, its frames refused again, and the witness from the archived transmission. It prints `{"ok":true,"check":"replay","judge":"foreign","input_verdict":…}`: `input_verdict` is the input's witness re-judged (`fail` with a `foreign_param_rejected`), not the run's verdict, whose other findings the trace does not hold. The header names the input, the firmware, the platform and the specification, and the engine and the Renode build, as a campaign's does.

### The analog pre-pass (`simulate --firmware`)

`AET-WO-2200` (`RUL-17`). Before Renode is looked for, each I2C line of the solved topology has its rise computed from the declared values (`RUL-19` (1)), and the result decides the platform the boot loads:

1. `aethelgard-twin::analog` writes one netlist per line from the topology's `analog_lines`: the pull-up from the domain's rail, the line's capacitance, and the open-drain driver, released once in a 22.5 µs window (`AET-WO-2101` S2).
2. `aethelgard-bridge`, found **beside the `aethelgard` binary** (`BridgeUnavailable`, exit `4`, if it is not), measures each line's 30 %–70 % rise with libngspice 44.2 in its own process. Its first line is its identity: its name, version and source digest, which must be the ones this CLI was built with (`BridgeMismatch`, exit `4`), and, once the library has started, the banner the library printed (`ngspice_banner`, verbatim) and the release read from it (`ngspice`), which must be `44.2` (`AET-WO-1606`). The bridge refuses a library whose banner is absent, repeated, unreadable or another release, and names no ngspice it did not read. Every answer names that release. It runs as a sandboxed tool like Renode, bounded at 30 s and 1 GiB (`analog::BRIDGE_BOUNDS`; Renode's cap is 1.5 GiB since `AET-WO-2313`), in a directory of its own, so it can write none of the copies Renode reads. Its rise time is its own interpolation of the points ngspice accepted, and it must agree with ngspice's `meas` to 0.01 ns.
3. Each line has one of three outcomes, each a property of the design (ruled on PR #70, F7 (b)):
   - **it rose**, in `rise_30_70_ns`. Past its ceiling (the net constraint's `max_rise_time_nanoseconds`), it fails the verdict with `analog_rise_exceeds_ceiling`;
   - **it was still rising when the window ended** (`complete: false`, with `rise_30_70_ns_lower_bound`, the window left after its 30 % crossing, or after the release if it never got there). That is always past the ceiling, so `analog_rise_exceeds_ceiling` states the bound as a lower bound;
   - **it was at or above 30 % when its driver released it**: an `analog.line_stuck_high` record with `volts_at_release`, and `analog_line_stuck_high`.

   Any of the three faults the line's bus: the platform the boot loads is the emitted one with that bus's devices left out (no declaration, modelled or not) and the decision, naming them, written into its header. Renode is then expected to list every solved peripheral and only the devices left in. That file is checked against the emission for the specification plus the recorded `faults` just before Renode starts (`PlatformNotEmitted`, exit `3`). `--platform` is still the platform emitted for `--spec` without any fault.
4. The boot runs and is judged as above. The firmware still reads each modelled device's identity, so a device left out by the fault times out at TXIS, and `boot.i2c` records it as `i2c_timeout`.

A bridge that exits nonzero, is killed, or answers anything but one of the three outcomes per line, in order, is `ToolError` (exit `5`, `id` `aethelgard-bridge`): no line is ever read as unfaulted. Exit `5` means the bridge itself failed, never the design.

The verdict carries the pass beside `boot`, never inside it: at 47 kΩ the reference gives three separate records, the rise computed from the declared values (`analog`, `RUL-19` (1)), the decision (`faults`) and U2's TXIS timeout (`boot.i2c`). The firmware sees a faulted bus only as its devices' absence, which Renode 1.17.0 renders as a TXIS timeout because it models no NACK; `firmware_sees` says so.

```json
"analog":[{"kind":"analog.rise_time_computed","source":"ngspice","bus":"SENSORS","net":"I2C_SCL","complete":true,
           "rise_30_70_ns":3982.300059495949,"ceiling_ns":300,"exceeds":true,"method":"interpolated","ngspice":"44.2",
           "deck_hz":400000,"declared_hz":400000},…],
"faults":[{"kind":"bridge.fault_state","source":"aethelgard","bus":"SENSORS","fault":"absent",
           "from":["analog.rise_time_computed > ceiling_ns"],"nets":["I2C_SCL","I2C_SDA"],"firmware_sees":"…"}]
```

### Harness provenance (`prove`)

Each gate binds only its own input (`RUL-19` (2)): `prove` judges the crate it is handed, and a crate's own harness can pass vacuously. So every `prove` verdict says which it judged (`AET-WO-3301` §1.4):

- `harness_provenance` is `generated` when `--spec` is given and the crate as copied is byte for byte the board crate `generate` writes for that specification: every file, none missing and none extra (a top-level `target/` is not part of a crate). Otherwise, and always without `--spec`, it is `supplied`. The whole crate is compared, not only `src/proofs.rs`, because an unchanged harness over rewired code is not the generated proof. `prove --spec` generates the board crate alone, without the schematic or the routed board, which it does not judge (`AET-WO-1919` (1)). So a specification whose crate generates but whose board cannot be drawn or routed (`Unroutable`, say) is refused by `generate` and `verify`, but `prove --spec` still judges its crate, `generated` when it matches.
- `cover_count` is the number of cover properties Kani reported for the harness, `0` when it reported none.
- A `--spec` that `solve` refuses, or whose board crate `generate` cannot write, is refused as they refuse it, before any tool is looked for. A refusal of the schematic or the routed board alone (`Unroutable`, say) is not `prove`'s: it generates the crate alone (above).

### Building firmware (`build`)

`build --crate <dir> --out <file>` builds the firmware ELF of a board crate: `<dir>` is the board crate `generate` writes, whose `firmware/` crate depends on it by `path = ".."`. The crate is copied and checked before any tool is looked for: it must hold `firmware/Cargo.toml` (else exit `3`, `NotABoardCrate`), and neither `Cargo.lock` nor `firmware/Cargo.lock` may name a package with a `source`, a registry or git dependency (exit `3`, `CrateHasDependency`, `RUL-04`). `--out` must not exist (exit `3`, `UnwritableOutput`). Then the pre-flight (`AET-WO-1603` §1.2), which `prove` runs too, before any tool is looked for, refuses by name, exit `3`, with `id` the file (relative to the crate inside it, absolute above it): a manifest that is not TOML (`MalformedManifest`); unstable `cargo-features` (`CargoFeaturesRefused`); a proc-macro crate, by `proc-macro = true` or by `crate-type` (`CrateHasProcMacro`), or a build script under `[package]` or its old name `[project]` (`CrateHasBuildScript`), in any manifest of the crate; a path dependency, `[patch]` or `[replace]` entry resolving outside the crate, or a workspace member that is absolute or climbs with `..` (`CrateHasDependency`); a `package.workspace` outside the crate (`WorkspaceAboveCrate`); a `.cargo/config` or `.cargo/config.toml` anywhere in the crate other than `firmware/.cargo/config.toml` exactly as `generate` writes it (`build.target`, `build.target-dir`, and this target's `rustflags` naming `link.x`, nothing else) (`CargoConfigRefused`); a `rust-toolchain` or `rust-toolchain.toml` anywhere in it (`ToolchainFileRefused`); and, in any directory above the crate's copy up to `/`, any of those files (`CargoConfigRefused`, `ToolchainFileRefused`) or a `Cargo.toml`, which cargo would take as a workspace root (`WorkspaceAboveCrate`). With the default scratch directory under `~/.cache/aethelgard`, your home is one of those directories, so a `~/.cargo/config.toml` of your own is refused: cargo would read it. Set `TMPDIR` to a directory with no such file above it.

Cargo then runs in the sandbox, as `prove` runs it: `cargo build --release --offline --locked --target thumbv7em-none-eabihf --config profile.release.strip="symbols" --message-format=json-render-diagnostics` in the copy's `firmware/`, with only `RUSTUP_HOME`, `PATH`, `CARGO_TARGET_DIR` and, when the caller set it, `RUSTUP_TOOLCHAIN` (exit `4` naming `RUSTUP_HOME` or `PATH` if unset). Two witnesses decide: cargo's exit code and its `build-finished` message. Exit `0` with `build-finished` `success: true` and exactly one executable is a pass: the ELF is written to `--out` and named in `elf`. Exit `101` with `success: false` is a fail, `build_failed`, naming cargo's first `error:` line. Anything else, such as a cargo configuration cargo cannot load (exit `101` with no `build-finished`), is `ToolError`. `tool.version` is cargo's; `tool.rustc` names the compiler cargo ran, pass or `build_failed`, by its `release` and `commit_hash` from the one `-vV` answer cargo recorded in the fresh target directory's `.rustc_info.json` (`AET-WO-1606`). Cargo writes that record when it asks its own compiler, so it names the rustc this run invoked, a caller's `RUSTUP_TOOLCHAIN` included; a second `rustc -vV` is not asked, because a compiler cargo was configured to use can differ from the one that probe resolves (measured, `spike/1606`). No record, or one with no single `-vV` answer naming both, is `ToolError`. The record is what that compiler says of itself. A pass whose units were not all built from manifests inside the copy (a path dependency outside the crate, a `[patch]` or `paths` override), or that read any source outside it as cargo's dep-info beside the ELF lists them (a `#[path]` module, an `include_bytes!`, a `[lib] path`), each path resolved before it is compared, or that ran a build script, is refused, exit `3`, `CrateHasDependency`, and no ELF is written. A linker input named in a supplied crate's own rustflags (`-T`, `-C link-arg`) is not in dep-info and is not refused; it can come only from what the sandbox lets cargo read, the tool installation and the system directories (FEEDBACK F12).

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

As root, as in the published image, the CLI runs every tool under a separate uid on a root-owned copy, and verdicts say `"uid_boundary":true`. Elsewhere it runs tools as its caller. A host that wants the boundary without running the CLI as root sets it up this way; nothing here is done by the CLI or by this repository:

1. **A dedicated uid for the CLI and its MCP server.** Run `aethelgard` and `aethelgard mcp` as a user that runs nothing else, with its own home, and `TMPDIR` unset or private to it.
2. **A toolchain that user cannot write, and nor can the client.** The tools, `RUSTUP_HOME`, `KANI_HOME` and Renode belong to root (or to the CLI's uid, never the client's). A toolchain the client can write is a toolchain the client can replace, whatever the sandbox does. `find /opt /usr -xdev ! -uid 0` should print nothing. In the pinned tools image it does not: Kani's kit under `/opt/kani/kani-0.68.0` belongs to uid 1001, which the image's first `useradd` would get.
3. **The MCP client as a different uid.** The client reaches the CLI only through `mcp`'s stdin and stdout, and hands it input files the CLI's uid can read. It can then neither write the CLI's scratch nor read it.

The CLI cannot see this set-up, so its verdicts still say `"uid_boundary":false`: they under-claim, which `RUL-19` prefers to over-claiming. Without all three, the residual above is the client's own identity (`RUL-19` (4)).

## `check pinout`

```
aethelgard check pinout --topology <solved.json> --elf <firmware.elf> [--hwdef-h <hwdef.h>] [--allow <allow.json>]
```

The pinout check of `aethelgard-pincheck` (`AET-WO-4400a`). It judges what a linked STM32H753 firmware writes to its GPIO registers at reset (`gpio_default_config`, 308 bytes) against a `SolvedTopology`, in three directions: forward, reverse and package. The crate's `README.md` gives the rules. It starts no process and runs no tool, so it has no sandbox and no scratch. The package is the solver's own pin table for `STM32H753ZITx`, the one part whose register layout the check knows; a topology names no part yet.

Every input is read whole under the 16 MiB cap. The verdict carries each input's sha256, `ok`, `decodes` (`elf and hwdef.h agree`, or `elf only`), `identity` (`judged`, or `not judged: no hwdef.h`; `AET-WO-4400b2`), `configured` (each configured pin with `named_by`, a topology net, `allow-list` or null), and `findings`. Each finding is named by `check` (`forward`, `reverse` or `package`), `kind` (`NotAlternate`, `WrongAlternateFunction`, `NotOutput`, `WrongOutputType`, `WrongSpeed`, `WrongPull`, `UnnamedPin`, `AllowedPinDiffers` or `NotInPackage`; and, with `hwdef.h`, the identity judge's kinds under `check` `identity`, listed in `aethelgard-pincheck`'s README, `AET-WO-4400b2`), `pin`, `net` (or null), `expected` and `decoded`.

The `hwdef.h` facts the identity judge reads (SPI devices, PWM groups, I2C clock, barometer and IMU probes) are bound to the firmware only by coming from the same build directory as the ELF: a hand-paired ELF and `hwdef.h` is not cross-checked (`AET-WO-4400b2` D1). Each serial port's protocol, options and baud are read from the ELF's `var_info` (`AET-WO-4405`).

| Exit | When |
|---|---|
| `0` | no finding |
| `1` | at least one finding |
| `2` | the topology is well-formed JSON of the wrong shape (`SchemaViolation`), `validate` refuses it, one of its pins is not `P[A-K]0-15` (`InvalidPin`), it holds no stored speed for a pin (`NoPinSpeed`), or it holds a pin device or a gpio entry that is not a chip select, which the check does not judge (`GpioNotJudged`, `AET-WO-1211a`), or it names an MCU other than the pin table's, exactly (`PartMismatch`, `AET-WO-1218`: the analog pads `validate` reads are the named MCU's) |
| `3` | an input cannot be read or is too large (`UnreadableInput`, `InputTooLarge`), or the topology is not JSON (`InvalidJson`). A malformed ELF by name: `Truncated`, `NotElf`, `WrongClass`, `WrongByteOrder`, `WrongType`, `WrongMachine`, `NoSectionTable`, `NoSymbolTable`, `NoProgramHeaders`, `MalformedElf`, `SymbolMissing`, `SymbolDuplicated`, `SymbolNotObject`, `SymbolSize`, `SymbolOutsideSection`, `SymbolNotLoaded`, `SectionTableDisagrees` when the section table and the `PT_LOAD` put the symbol at different file offsets, `SymbolNotInPlace` when that `PT_LOAD` is flashed at one address and run at another, and `SegmentsOverlap` when another `PT_LOAD` flashes bytes over the symbol. A symbol in a writable section is `SymbolOutsideSection`. With `hwdef.h`, a malformed serial parameter table (`AET-WO-4405`): `MalformedVarInfo`, `VarInfoUnterminated`, `VarInfoNameTooLong`, `UnknownVarInfoRow`, `VarInfoRowTwice`, `VarInfoPartialPort`, and `SymbolNotLoaded` for a row name not flashed. `MalformedHeader` (`id` the macro, or `line N` for the file's structure: an unbalanced conditional, an unclosed comment, a continuation at the end or a backslash followed by blanks). `HwdefConditionalUnsupported` (`id` the macro): one of the 33 macros named inside a conditional other than exactly `#if 0` or `#if 1`, which the reader evaluates, after splicing continuations and removing comments, as the preprocessor does (`AET-WO-1603` §1.4). `MalformedAllowList`, and `AllowNamesTopologyPin`, an allow-list that would hide a topology pin |
| `5` | `DecodesDisagree`: `hwdef.h` and the ELF give different `MODER`, `AFRL` or `AFRH` for a port (`id` such as `GPIOB_AFRL`). Never read as pass or fail |
| `70` | the vendored pin table failed its own check |

## `campaign` and `replay`

`AET-WO-2300`: the P2 gate's fault campaign, its archive, and replay from the archive alone (`INV-04`).

```
aethelgard campaign --scenarios scenarios/sentinel-01/pullup-2k2.json scenarios/sentinel-01/pullup-47k.json \
                    --firmware <elf> --archive <dir>
aethelgard replay --archive <dir>/<run id>
```

**A scenario** is a strict JSON file authored apart from anything generated (`INV-07`), as [Scenarios](#scenarios) below says. Its `base` is the specification it starts from, relative to the scenario file's directory. It must not be absolute, and once every symlink is resolved it must be a regular file **inside the current directory** (`BaseOutsideRoot`, exit `3`), read under the same 16 MiB cap and refusals as `validate`. The file is opened once, without following a symlink or blocking on a FIFO, and where that descriptor really is (`/proc/self/fd`) is what is checked against the current directory and then read, so a path swapped after the check is never read. So run a campaign from the repository root. The overrides change the specification and nothing else; the runner solves the result. It never reads or edits a generated file. The two reference scenarios are `scenarios/sentinel-01/`.

**`campaign`** first reads the firmware's hash and every scenario: it parses, applies, validates and solves each one, and refuses (`DuplicateScenario`) two that would name one run. It creates `--archive` if absent and refuses a run directory that already exists (`RunExists`). Any refusal is printed before anything runs. Then, for each scenario in the order given:

1. It writes the platform `aethelgard-twin` emits for the overridden specification and runs exactly `simulate --firmware`'s path on it, with the same sandbox, bridge, bounds and judge, booting **100 ms** on every board, the longest `--for` admits, so a timed-out read still reaches the boot word (`MAX_VIRTUAL_NS`, the window each line names; each witness's own window is in its records, `AET-WO-2302`). A held-low line (`line_held_low`) is not sent to the bridge: its bus is faulted without a measurement, with `i2c_line_held_low` and a `bridge.fault_state` whose `source` is `scenario <name>`.
2. It archives every record of the verdict, as integer-only CBOR events, in a new run directory `<archive>/<run id>`: the scenario's sha256 and overrides, the overridden specification's and the solved topology's sha256, each `analog.rise_time_computed` (`rise_30_70_ps`, `complete`) and `analog.line_stuck_high`, each `bridge.fault_state`, the boot's witnesses, every register, each I2C identity read and controller witness, and the verdict. The header's `artifacts` are the `base` file's, the `firmware`'s, the `scenario`'s, the overridden `specification`'s and the `topology`'s hashes. Its `tools` name what judged the run, each as the tool reported it: `aethelgard` (this CLI's version), `renode` (the `build:` line of `renode --version`; a boot whose Renode names another build than the pinned one, or none, is refused, `RenodeBuildNotPinned`, exit `4`, `AET-WO-4400g`), and, when the bridge ran, `aethelgard-bridge` (its identity digest) and `ngspice` (the release the bridge read from the library's banner, which its identity and every answer name; `AET-WO-2205` §1.1, `AET-WO-1606`), with the sha256 of the library file the bridge's loader mapped (its identity's `ngspice_library`, the path `dladdr` names, hashed by this CLI; two bridges of one boot that name different bytes stop it); and `rust-toolchain`, `rustc -V`'s version with a digest of its whole active sysroot, run as `build` runs cargo (`AET-WO-2310`, ruled Q-T T1). **That digest is of the host's toolchain, not of the one that built the ELF**: `campaign` judges an ELF it did not build, so it says which toolchain was on the host, never that it made the firmware; binding the two is a later row. On a host with no `rustc` on `PATH` or no `RUSTUP_HOME` the entry is `absent`, with no hash. `replay` reports each hash as `tool_sha256`. Kind 10 archives the executable segments the judge placed the core's `pc` against. A firmware whose bytes changed between the campaign's read and the boot's copy is refused (`FirmwareChanged`, exit `3`). The run is committed before its line is printed.
3. It prints one line:

```json
{"scenario":"pullup-47k","file":"scenarios/sentinel-01/pullup-47k.json","verdict":"fail","expect":"fail","matched":true,
 "findings":["analog_rise_exceeds_ceiling","i2c_timeout"],"expect_findings":["analog_rise_exceeds_ceiling","i2c_timeout"],
 "run_id":"011cc38d…","run":"<archive>/011cc38d…","state_sequence_hash":"0483991c…","event_count":16,"trace_bytes":1335,
 "uid_boundary":false,"uid_boundary_unmet":"tool_uid"}
```

Each run line carries its boot's `uid_boundary` and, when it is `false`, `uid_boundary_unmet`, as a verdict's `sandbox` does (`AET-WO-1601`).

`matched` is true when the verdict is the one the scenario expects and, for a failure, its finding kinds above `info` are exactly `expect_findings`: a scenario that fails for another reason, or for more, does not match. The last line is the summary, `{"ok":…,"verdict":…,"check":"campaign","scenarios":n,"matched":m,"virtual_time_ns":100000000,"trace_bytes":…}`, with exit `0` only when every scenario matched, and `1` otherwise. A tool that stops one scenario (exit `4` or `5`) stops the campaign with its refusal; runs already committed stay. A campaign therefore asserts what should fail.

**`run_id`** is a function of the scenario's, the overridden specification's and the firmware's hashes, so the same campaign twice names the same runs, with the same state-sequence hash: into one archive the second is `RunExists`, since the archive is append-only.

A run directory a foreign input run archived (`simulate --foreign --input --archive`, its records kinds 38 to 40) is re-judged by the foreign judge instead ([A declared input](#a-declared-input-simulate---foreign---input-aet-wo-4400i)); every other run as below.

**`replay`** reads one run directory through `aethelgard-chaos`'s strict reader and nothing else: no solver, no tool, no scratch directory, no file outside the run directory, no network. It recomputes the state-sequence hash, the chain `crates/aethelgard-chaos/README.md` defines over every event's payload (no timestamp, no wall-clock field), and refuses a run whose hash differs from its manifest's (`ReplayDiverged`), and every other refusal of that reader by its name, with exit `3`. The reader opens the run directory once and reads every file through it, never following a symlink or opening a FIFO. A directory not named by the run id its header holds is `RunIdMismatch` (exit `3`): a run copied under another run's name is not that run. It then judges the records (`NotACampaignRun`, exit `3`): no event but campaign records, a header whose hashes and run id are the ones its records give and whose `tools` are the ones that ran (each reported, never compared with this host's), the records' order, and that every fault, measurement and verdict agrees with the evidence beside it. It prints one line, exit `0`, and two replays of one run print the same bytes:

```json
{"ok":true,"check":"replay","run_id":"011cc38d…","scenario":"pullup-47k","verdict":"fail","expect":"fail","matched":true,
 "findings":[…],"expect_findings":[…],"artifacts":{"base":"…","firmware":"…","scenario":"…","specification":"…","topology":"…"},
 "tools":{"aethelgard":"0.3.0","aethelgard-bridge":"91e72fcc…","ngspice":"44.2","renode":"1.17.0+20260907gitf1dd1b4af.aet1605"},
 "event_count":16,"trace_bytes":1335,"state_sequence_hash":"0483991c…"}
```

### Scenarios

The format is `aethelgard_chaos::campaign::Scenario::parse`'s (`crates/aethelgard-chaos/README.md`'s "Scenarios"); a refusal is exit `2` before anything runs.

```json
{"schema_version": 3, "name": "serial-rx-open-console", "base": "../../../examples/sentinel-01-minimal.json",
 "expect": "fail", "expect_findings": ["serial_echo_missing"], "expect_counts": {"serial_echo_missing": 1},
 "overrides": [{"serial_rx_open": "CONSOLE"}]}
```

- **Keys.** `schema_version`, `name` (1 to 64 bytes of `0x21`–`0x7E`), `base`, `expect` (`pass` or `fail`), `expect_findings` and `overrides` are required, and `expect_counts` is optional. No other key is allowed, at either level. A duplicate key, a positional array, or a key an override does not use (even as `null`) is `SchemaViolation`.
- **Versions.** A file may use only what its version admits (`SchemaViolation` otherwise), and any other version is `UnsupportedSchemaVersion`:
  - **1** (`AET-WO-2300`): `pull_up_ohms`, `max_capacitance_femtofarads` and `line_held_low`;
  - **2** (`AET-WO-2301`, `AET-WO-4200`): also `spi_device_absent` and `expect_counts`;
  - **3** (`AET-WO-2302` D5): also `serial_rx_open`;
  - **4** (`AET-WO-2304b`) and **5** (`AET-WO-1211c`): also `rail_out_of_band`, and `pin_held` and `analog_value`;
  - **6** (`AET-WO-2310`): a `droop` and a `battery_sag` are read at version 6 only, each planted `from_boot_word_ns`, signed from the undisturbed boot word; an older file holding one is `PlantedEventNeedsVersion6`, naming what changed. An instant that cannot be placed is `PlantedEventUnplaced`; with no boot word it ends with the undisturbed boot's own end: its findings, the flood, the memory for a bound (`AET-WO-2314`).
- **Expectations.** A failing scenario names the finding kinds it fails with in `expect_findings` (at most 16, each once); a passing one names none (`ExpectContradiction`). A run matches when its verdict is the one expected and its finding kinds above `info` are exactly `expect_findings`. `expect_counts` (an object of finding kinds to counts, at most 16, a kind named twice refused) also requires each named kind's count, at any severity and 0 when absent, to be exactly the one declared (`AET-WO-2301` §2.3); it is archived as kind 20.
- **Overrides**, at most 32, two of one thing (one bus's pull-up, one net, one line, one device, one port, or two droops) refused (`DuplicateOverride`). Each id is 1 to 64 bytes of `0x21`–`0x7E`, and an id the specification or its solved topology lacks is refused, exit `2`, by its own kind:

| override | version | what the runner does | refused |
|---|---|---|---|
| `{"bus", "pull_up_ohms"}` (a `u32`) | 1 | sets the I2C bus's pull-up, then validates the specification | `UnknownBus`; a result the specification's rules refuse, by `aethelgard-ir`'s kind |
| `{"net", "max_capacitance_femtofarads"}` (a `u64`) | 1 | sets the net constraint's capacitance, then validates | `UnknownNet`; as above |
| `{"bus", "line_held_low": "sda" \| "scl"}` | 1 | changes nothing in the specification: the line is faulted without a measurement and not sent to the bridge (`i2c_line_held_low`, the fault's cause `scenario.line_held_low`) | `UnknownBus` |
| `{"droop": {domain, floor_millivolts, from_boot_word_ns, fall_ns, hold_ns, rise_ns, source_milliohms, load_milliamps}}` | 6 | a sag of the MCU's supply, rendered through the supervisor's table at the specification's `mcu.bor_level` (`AET-WO-2301`, `AET-WO-2310`; [Booting firmware](#booting-firmware-simulate---firmware)): each field bounded (`floor_millivolts` 1–5 000, `from_boot_word_ns` a multiple of 10 within ±100 000 000, `hold_ns` 1–100 000 000, `fall_ns` and `rise_ns` 1–10 000 000, `source_milliohms` 1–1 000 000, `load_milliamps` 0–10 000); one per scenario. Its instant is the undisturbed boot word plus the offset: the same prepared boot runs first without the droop, its word places the droop, then the droop's boot runs (kind 32 archives the word); before that boot only the offset is judged, and every clause of the instant after it, so such a refusal follows a boot and may follow earlier scenarios' committed runs. `PlantedEventUnplaced`: the instant at or before reset or past the run, or no word | `DroopNotMcuSupply`; `DroopRefused` (a floor or resting level inside a threshold's band or hysteresis region, a floor held less than 10 R·C, effects past the boot) |
| `{"spi_device_absent": reference}` | 2 | changes nothing in the specification: the platform leaves the SPI device out behind its chip select, which stays wired (`AET-WO-4200` D4) | `UnknownDevice` |
| `{"serial_rx_open": port}` | 3 | changes nothing in the specification: the port's RX line is open on the board, so the composition injects no frame into it and the sequential injection skips it (`AET-WO-2302` D5). Its mechanism is `serial_echo_missing` on that port alone; `attribute` names it `E3` | `UnknownSerialPort` |

Kind 5 archives every override (`[0, bus, ohms]` to `[5, port]`, `crates/aethelgard-chaos/README.md`), and `replay` binds each to the records it causes. The reference's scenarios are `scenarios/sentinel-01/` and its seeded set `scenarios/sentinel-01/seeded/`.

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
- `unexplained`, failing finding kinds that no archived event shows; and, as `{kind}: {what the records lack}`, a window finding whose records cannot decide it (below).

**Symptoms.** The verdict's finding kinds above `info` choose which events need explaining. They are never evidence. Each kind selects its events, and the count must equal the finding's, summed over every severity above `info`, or the kind is unexplained. The converse holds too: an event that some kind below would select, but that no finding names, is unexplained as `event <index>`. So rewriting the verdict alone never moves a run's domain; at most it makes the run `I1`, or `H4` by naming `serial_injection_out_of_order` (which replay cannot bind) in a run nothing else decides: indeterminate either way.

| finding | its events |
|---|---|
| `analog_rise_exceeds_ceiling` | kind 7 past its ceiling |
| `analog_line_stuck_high` | kind 8 |
| `i2c_line_held_low` | kind 9 caused by `scenario.line_held_low` |
| `i2c_timeout`, `i2c_record_invalid` | kind 12 whose status is a `timeout_…`, or `invalid` |
| `i2c_identity_mismatch`, `i2c_configure_mismatch`, `i2c_data_mismatch` | kind 12 of that step whose status is `ok` and whose value is not the part table's (SPI's likewise, kind 21) |
| `i2c_transaction_unwitnessed` | kind 13 whose read's status is `ok` and whose observed lines are not exactly its expected ones |
| `register_mismatch` | kind 11 that differs from its expectation under its mask |
| `boot_word_missing`, `cpu_halted` | kind 10 without the boot word, or with fewer instructions than expected |
| `locked_peripheral_access` | kind 11 in RCC with an expected enable bit clear, at least one: the same fact, archived (the count is not compared, since the judge reports one finding for any number of accesses) |
| `motor_output_unwitnessed` | each kind 26 whose status is `unwitnessed` or `unreadable` (`AET-WO-4300a`), once the boot word is set, as the judge names it |
| `motor_timing_mismatch` | each kind 26 whose status is `mismatch` or `batched`, once the boot word is set |
| `motor_timing_static_mismatch` | each kind 25 whose check does not hold |
| anything else (`renode_error_log` and the `i2c_step_error`/`spi_step_error` beside it, a reaper bound, `pc_outside_firmware`: executable segments are not archived; `cpu_halted`'s lagging hook line, which no record carries) | none: unexplained |

**The twin's window findings** (`AET-WO-2302b`). The anchors', the judge's stop's and the injection chain's findings (`AET-WO-2302`) are decided by kind, each by the rule below, never by where their events are. Replay binds each count to the records (`motor_anchor_out_of_order` to a range, `serial_injection_out_of_order` to none), so each selects the events that show it:

| finding | its events | rule |
|---|---|---|
| `motor_anchor_unobserved` | each kind 26 `unanchored` (edges, no `CEN` rise) | `S6` |
| `motor_anchor_repeated` | each timer's first kind 26 whose `cen_rises` is above 1 | `S7` |
| `motor_anchor_out_of_order` | each timer's first kind 26 anchored after the boot word's instant (one quantum before kind 29's first injection), when the verdict names exactly those | `S8` |
| `motor_window_past_run` | each DShot timer's first anchored kind 26 whose anchor plus `DSHOT_WINDOW_NS` passes `MAX_VIRTUAL_NS` | `H1` |
| `motor_edge_before_anchor` | each kind 26 with `edges_before` | `S9` |
| `motor_edge_after_window` | each kind 26 with `edges_after`, with its timer's kind 30 at its anchor that left `CEN` cleared | `S10` |
| `judge_stop_missing` | each DShot timer's first anchored kind 26 whose window ends inside the run, with no kind 30 at its anchor that left `CEN` cleared | `H2` |
| `judge_stop_mismatch` | each kind 30 that left `CEN` cleared and read a register back off its solved value under its mask | `S11` |
| `serial_wire_past_run` | kind 29 of the first port the chain injects, when its frame went in and the ports' frames and echoes from there end past the run (`campaign::wire_past_run_ports`) | `H3` |
| `serial_injection_out_of_order` | each kind 29 injected after the chain's first | `H4` |

Where the records cannot decide, the kind is unexplained by name with what they lack, never a guess (`INV-06`):
- `motor_anchor_out_of_order` named other than exactly the timers anchored after the boot word's instant: its other cases (an anchor before the last device step's witness, or no boot word seen by its hook) are the boot's log lines, which no record carries, so only the verdict would say them;
- `motor_edge_after_window` with no stop at the output's anchor that left `CEN` cleared (a PWM output's window, or a stop that did not run): the edges may be the timer still running;
- `judge_stop_mismatch` of a register the stop did not read back: an unread register shows no write;
- `serial_wire_past_run` when the first port took no frame: the boot word's instant is in no record;
- `serial_injection_out_of_order` beyond the ports injected after the chain's first.

Every one of them is named only once the boot word is set (replay refuses it before), so a boot without its word never reaches these rules.

**Where a symptom is.**
- Kinds 7, 8 and 9 are on their bus.
- Kinds 12 and 13 are on the bus whose kind 15 lists their device, for that device.
- Kind 11 is on a bus when it is wrong only in fields that bus owns. A bus owns its controller's clock gate, the gates of its pins' ports, and each pin's `MODER` and `AFR` field. Each name and position comes from the ST header, and the solved alternate function comes from kind 15.
- Kind 10, and any kind 11 wrong elsewhere, is on the board.

**Software evidence on a bus.** For a symptom of one device, only that device's own witness counts, since another device's transaction says nothing of this one's. The bus's owned registers count for every symptom on it.
- The device's witness (kind 13) differs from its solved read: the model logged a write of another register or a read of another value (the first of each counts), or the record says the read completed and the model logged nothing.
- Its controller found no device at an address other than the device's (Renode's `Unknown slave at address N.`): a transfer to a wrong address, which leaves the model's log empty as a faulted bus does. On a faulted bus the correct address itself is absent, which is not counted.
- A witness whose own expectation is not the solved register and value, or that could not be read whole, is no evidence either way.
- An owned register field is wrong. A pin's `MODER` field must hold alternate-function mode and its `AFR` field kind 15's alternate function; both come from kind 15, never from the reading's own expectation, and a reading whose expectation disagrees is no evidence.

**Software evidence on the board.** The boot word is missing, the core executed fewer instructions than expected, or a register is wrong outside every bus's fields.

**A bus is computed healthy** (its rises are computed from the declared values, `RUL-19` (1)) when no kind 7 past its ceiling, no kind 8 and no kind 9 is on it, each of its two lines has its kind 7 (R2 allows one per line), and the ST header knows its controller and pins. A bus whose port or controller the header lacks is never healthy and owns nothing, so its symptoms are `I2` and no other bus's attribution is refused.

**The rules, closed and ordered.** Each symptom is decided by the first rule that holds for it:

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `E1` | electrical | the symptom's bus has a kind 7 past its ceiling, a kind 8 or a kind 9; its software evidence goes to `masked` | 7, 8, 9 |
| `I2` | indeterminate | the symptom's bus is not computed healthy (a bus the header cannot join; a line with no computed rise is refused by `replay` since `AET-WO-3201`) | 15 |
| `S1` | software | the symptom's bus is computed healthy and has software evidence | 7, 11, 13 |
| `I3` | indeterminate | the symptom's bus is computed healthy with no software evidence, and another bus is faulted | 7, 8, 9 |
| `I4` | indeterminate | computed healthy, no software evidence, and no fault anywhere | 7 |

For a symptom on an SPI device (`AET-WO-4200`: `spi_timeout`, `spi_record_invalid`, `spi_identity_mismatch` and `spi_transaction_unwitnessed`, from kinds 21 and 22):

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `E2` | electrical | the device's witness (kind 22) shows its controller's multiplexer routing a byte to the device's own address and finding nothing there, and the model logged nothing: a device absent behind a chip select the firmware asserted; its software evidence goes to `masked` | 22 |
| `S3` | software | the multiplexer found no chip select asserted, or several; or the model logged lines other than the read's; or the read (kind 21) claims completion with nothing logged or routed | 21, 22 |
| `I4` | indeterminate | neither | none |

For a symptom on a motor output (`AET-WO-4300a`: `motor_output_unwitnessed`, `motor_timing_mismatch` and `motor_timing_static_mismatch`, from kinds 26 and 25), and for a kind 11 wrong only in fields an output owns (its pin's `MODER` and `AFR` fields, its port's and timer's clock gates, and any register of its timer's block; a timer's register stands for the first of its outputs):

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `S4` | software | a register field the output owns is wrong (kind 11): the firmware's own configuration of the pin or the timer differs from the solved topology. The twin models nothing electrical on a timer's pin, so no rule attributes a motor symptom to the electrical half. `CEN` of a timer's `CR1` once the judge's stop cleared it (kind 30) is the judge's own act, never cited; a reading wrong in it alone is unexplained by name (`register_mismatch: TIMn.CR1: …`, ADV-M3) | 11, 26 |
| `I4` | indeterminate | no owned register is wrong (an output with no edges and every register right, or a static mismatch, which is the topology's) | 25, 26 |

For a symptom on a serial port (`AET-WO-4300b`: `serial_pattern_missing`, `serial_echo_missing`, `serial_capture_forged`, `serial_tag_mismatch` and `serial_framing_static_mismatch`, from kinds 29 and 28), and for a kind 11 wrong only in fields serial ports own (a port's TX and RX pins' `MODER` and `AFR` fields, their ports' and its USART's clock gates, and any register of its USART's block; the first port stands for a register several ports' fields share, and another port's fields are never this port's evidence):

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `E3` | electrical | the scenario holds the port's RX line open (kind 5's `serial_rx_open`, `AET-WO-2302` D5), the boot word is set, and the port shows exactly that: its witness (kind 29) has no frame gone in and no RX mux read at an injection (`replay` binds both to the override) and its echo alone missing, its pattern out on its TX pin; its framing check (kind 28) holds, its writes read whole and judged as the bring-up's; and none of the software evidence below. With nothing injected the RX pin's mux at an injection is not read: its mux is the end's (kind 11) and the writes' (kind 28's tags, never changed once solved). Its evidence is the port's own two records, so the port is the only subject | 28, 29 |
| `S5` | software | the firmware's writes are shown wrong, with the port's witness when it failed: a register field the port owns is wrong (kind 11); or the port's writes, rebuilt from Renode's access log, fail its judgement (kind 28's tags: a word wrong at `UE` or changed after it, a write or read the bring-up never makes, an RCC write but to an enable register or one clearing the port's gates, a pin misconfigured, changed once solved, or locked); or its TX pin was not routed at a boot's first `TDR` write (kind 29's `tx_at_send`). The twin models nothing electrical on a USART's pins: only a declared open RX line (`E3`) attributes a serial symptom to the electrical half | 11, 28, 29 |
| `I6` | indeterminate | no write of the firmware's is shown wrong (a dead echo or pattern with every register right, every write judged as the bring-up's, or an access log that could not be read whole), or the port's static framing check failed, which is the topology's | 28, 29 |

For `S5` and `I6`, a witness or framing check is evidence only when it failed; `E3` cites the open port's holding framing check as the positive evidence that its framing and writes hold. `serial_capture_forged` may count more than the witnesses show: a foreign file in the capture directory names no port.

For the twin's window findings (above), six software rules attribute the firmware's own timing, and four harness rules refuse what is the twin's or the judge's own bound, never the device's: they attribute to neither half (`indeterminate`). No window finding is electrical: in the twin nothing a board declares produces one.

| rule | domain | holds when | evidence kinds |
|---|---|---|---|
| `S6` | software | the output's witness logged edges with no `CEN` rise by any CPU store of its timer's `CR1` in the boot the judge reads: the firmware drove the pin, or started the timer, by a GPIO or DMA write | 26 |
| `S7` | software | the timer's `CEN` rose more than once in that boot: the firmware started it again. Not when `CEN` was set again after the judge's stop cleared it (kind 30) and kind 11's end read of its `CR1` has it set: no record says when each rise fell, so a re-enable after the window, harmless on silicon where `CEN` was still set, cannot be told from a restart, and the kind is unexplained by name (ADV-M3) | 26 |
| `S8` | software | the boot word is set (kind 10) and the timer's anchor (kind 26) is after the boot word's instant, which kind 29's first injection places: the bring-up writes the boot word last | 10, 26, 29 |
| `S9` | software | the witness shows edges before its timer's anchor: the pin driven before the timer started | 26 |
| `S10` | software | the witness shows edges after the window the judge's stop closed, `CEN` cleared (kind 30): the timer restarted or the pin driven after it. Not when kind 11's end read of the timer's `CR1` has `CEN` set again: unexplained by name, as for `S7` | 26, 30 |
| `S11` | software | the judge's stop, which left `CEN` cleared, read a solved register of the timer off its solved value: the firmware wrote the timer inside its window | 30 |
| `H1` | indeterminate | the DShot window's end, its anchor plus `DSHOT_WINDOW_NS`, passes the run's `MAX_VIRTUAL_NS`; the output's `stopped` witness whose last rise reaches the run's end (a `motor_timing_mismatch`, with no register of the output's wrong) is the same bound's | 24, 26 |
| `H2` | indeterminate | the judge's stop at the anchor plus `DSHOT_WINDOW_NS` did not run, or left `CEN` set (kind 30, cited when present) | 24, 26, 30 |
| `H3` | indeterminate | the ports' wire time from the boot word passes `MAX_VIRTUAL_NS`; a port in that sum past the run whose echo alone is missing or cut short (never `not_counted`), its framing and writes holding, its RX pin on its AF at the injection when one was read, and no software evidence on it, is the same bound's (its `serial_echo_missing`) | 27, 29 |
| `H4` | indeterminate | the judge's sequential injection put a port's frame in out of order | 29 |

A harness rule never masks a fault of the device's and is never attributed to one: it names the run only when no other rule decided a symptom, its events stay in `symptoms`, and `I5` never counts it. Its evidence joins another rule's `evidence` only where the record is that rule's own evidence too (a DShot witness cut by the run, `H1`'s, whose `edges_before` are `S9`'s), never because of the harness (ADV-M4). Beside an indeterminate symptom of the device's (a dead output with every register right, `I4`), that rule names the run.

For a symptom on the board, any faulted bus gives `I3`. No bus at all, or any bus not computed healthy, gives `I2`. Then board software evidence gives `S2` (software; evidence kinds 7, 10, 11), and none gives `I4`.

**The run.**
- A passing verdict is `none`, rule `N1`.
- An unexplained finding is `I1`, with no evidence and the kinds listed in `unexplained`.
- Otherwise, if any symptom is indeterminate, the run is indeterminate by the lowest such rule, with that rule's evidence.
- If symptoms are split between electrical and software, it is `I5`, with both halves' evidence: never averaged.
- Otherwise the run takes the one domain and the lowest rule that decided any symptom. Its evidence is that rule's alone, so every evidence kind is one the named rule allows. With `S1` and `S2` both deciding, the run is `S1`, and its `symptoms` still list the board's.
- A harness rule (`H1` to `H4`) is set aside in each step above: only when it alone decided the run's symptoms does the run take the lowest of them, with that rule's evidence.

Software is never concluded by eliminating electrical, nor electrical by eliminating software.

**The seeded set** is `scenarios/sentinel-01/seeded/cases.json` (schema version 3). It holds 26 failing cases (12 electrical, 14 software), two indeterminate cases and two passing ones. Every mutant names the specification whose generated board it patches (`spec`, required since version 3, `AET-WO-4300a` Q4); the motor case, `motor-output-dead` (`AET-WO-4300a`, rule `S4`), patches the PWM fixture's board, which is not committed; the serial case, `serial-rx-dead` (`AET-WO-4300b`, rule `S5`), puts the console's RX pin (PA10) on AF8 in the reference board; its electrical twin, `serial-rx-open-console` (`AET-WO-2302` D5, rule `E3`), holds the console's RX line open on the unmutated board, so a dead RX is separated by its cause. Each case states the exact `[index, kind]` events `attribute` must name as `evidence` and as `masked`, and the test asserts them exactly (`AET-WO-2205` §1.6). Each evidence kind must be one of `aethelgard_evolve::Rule::evidence_kinds` for the case's rule. The indices move when the trace gains records: whoever moves them re-derives them from `attribute` and explains each changed row in their FEEDBACK.
- The electrical cases are scenarios run against the generated firmware.
- The software cases are named exact-once patches of `boards/sentinel-01`, built from a scratch copy by `tests/attribute_seeded.rs`. A patch that matches zero or two times fails that test, and so does a generator change.
- The trap: a wrong device address fails with `i2c_timeout` at TXIS, as a 47 kΩ pull-up does. It is `software` with 3.3 kΩ (computed inside the ceiling), and `electrical` with 47 kΩ, with the witness in `masked`.
- The indeterminate trap, `trap-indeterminate-leave-47k`: a core that leaves the firmware after its boot word (`leave-firmware`, `software S2` at 2.2 kΩ), on the 47 kΩ bus fault (`electrical E1` alone). Two real causes: it is `I3`, evidence exactly the bus fault's, and `propose` refuses to mutate (`INV-05`). The same mutant on `cap-390p-sda-2k2` (one line faulted, one healthy) is the second shape, `trap-indeterminate-leave-390p`: `I3`'s evidence is the faulted line's rise and the fault, never the healthy line's rise (`AET-WO-3201`, `2205` pass 2 M1).
- The window case `identity-late-dshot` (`AET-WO-3201`) is retired (`AET-WO-2302`), and its rule with it (`AET-WO-2302b`): its premise, a DShot board's 2 ms boot shorter than its start-up, no longer exists, since every board boots 100 ms.

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
| `E2`, `E3` | no rule: a device absent behind its chip select, or an open RX line, is a board fault no value change fixes |
| `S1` to `S11` | a generator report, with no specification change. `S11` names the timer register its stop read back (kind 30, by kind 11's name); `S6` to `S10`'s evidence (kinds 26 and 29, kind 10 with its boot word set, and `S10`'s kind 30 with nothing read back off its value) shows no unmet solved expectation, so nothing is reported, saying so |
| `H1` to `H4` | nothing is proposed, naming the bound: the twin's own, never the device's (`INV-05`) |
| `N1`, `I1` to `I6` | nothing is proposed, naming the rule: attribution precedes mutation (`INV-05`) |

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

## `extract`

```
aethelgard extract --board <pcb> --spec <path>
```

An upper bound on each I2C line's capacitance, and each SPI bus's SCK and MOSI (`AET-WO-4300c`), from the routed board's own copper, judged against the line's declared `max_capacitance_femtofarads` (`AET-WO-1305`, `RUL-19` (1)). It runs no tool. RC rise time grows with C. So when a line's bound is at most its declared value, the campaign, which simulated the declared value, judged a rise at least as slow as the board's, as far as capacitance within the bound's scope is concerned. **It is a bound, not an estimate of the typical value.** On the reference, 30 pF of each line's bound is the three pins at UM10204's maximum `Ci` (64 % of SDA's, 52 % of SCL's), an assumed figure, not one read from the board.

1. **Read the specification** as `solve` does, and build its design as `emit` does (exit `2` for what either refuses).
2. **Read the board once, into memory**; its sha256 and the bound both come from those bytes. The reader is strict, so that it never reads a board otherwise than KiCad. Refused with exit `3`, before any estimate:
   - `BoardUnreadable`: not one KiCad S-expression, or what `route::read` refuses (a part rotated other than by a right angle, a non-front-SMD part, a track or via other than its net's class's or the board's);
   - `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`, `ViaNotThrough` (`AET-WO-4100`; `ThicknessDisagrees` was `BoardThickness`): a board whose copper layer table (read by KiCad's ids), zones and planes, `(general (thickness …))` or vias are not the specification's declared board;
   - `ImuOrientationDisagrees` (`AET-WO-4400d`): a declared IMU whose footprint is missing, not on F.Cu, or turned so that the part's cited axes (by pins, read from the board's own pads) do not point as the specification declares, as a footprint turned by hand after placement would;
   - `MalformedBoard`: what the strict reader refuses (`check drc` above), with `id` the path to the node: a repeated child KiCad holds once, or a footprint field named twice (a legacy `fp_text reference` beside the `Reference` property included);
   - `UnmodelledCopper`: an item, a child, a list with no keyword, or a repeated child the bound does not model, or a value count other than its own. This covers an arc; a graphic or text on copper; copper in a footprint other than its pads; a pad without `layers`, naming a layer other than by its canonical name, or shaped other than `rect` or `roundrect`; a padstack, drill or offset; a zone fill or local clearance; and a stackup;
   - `BoardTooLarge`: a coordinate or size beyond 250 mm;
   - `UnmodelledPart`: two footprints with one reference;
   - `NetLabelsDisagree`: net labels that disagree with the copper's connections (`route::connectivity`);
   - `ClearanceViolated`: copper closer than the 0.2 mm clearance to another net's (`route::check`).
3. **The parts on each line are the design's** (each I2C bus's SDA and SCL, then each SPI bus's SCK and MOSI, in document order). The design is the parts `emit` draws from the specification and its solved topology. Each part on a line must be the design's part of that reference, with its footprint, its `Value` and exactly its pads on the line. A part that is not a passive or a connector (the MCU, a declared device) is a pin part. The resistor between the line and the bus's rail is a pull-up. Refused, exit `3`:
   - `LineNotOnBoard`: a line with no pad;
   - `DeviceNotOnLine`: a design part missing from the line;
   - `UnmodelledPart`: a part the design does not put there;
   - `PartMismatch`: another footprint, value or pad set;
   - `Overflow`: a bound too large for integer arithmetic.

   A design that puts any other passive on a line is refused with exit `2`, because its capacitance is not known.
4. **Bound each line**, in integer femtofarads, each step rounded up, as the sum of four terms, and a fifth where a connector sits on the line:
   - **trace:** 280 fF per millimetre of routed length, plus 124 fF for each track's two round ends;
   - **vias:** 1,519 fF each;
   - **pads:** each copper pad on the line, by its box, pins' lands included;
   - **pins:** on an I2C line, each pad of a pin part at UM10204's maximum `Ci`, 10 pF. On SCK and MOSI, each declared device's pad at its part's cited maximum (`parts.json` `pin_capacitance`: the W25Q64JVxIQ's 8 pF), and the MCU's pad, which drives the line, not at all (DS12117 Table 149's C is the external load); it is listed under `drivers`. A device whose part cites no maximum is refused (`UnboundedPinCapacitance`, exit `2`).

   Each copper term is the energy of one trial potential: 1 on the piece, falling linearly to 0 at the 0.2 mm clearance, where every other net's copper must be. With the permittivity at its maximum, that energy is an upper bound on the piece's capacitance (Chew, arXiv:2107.09672v2, §7.1.2). The constants, their derivation and their sources are in `aethelgard_eda::extract`.

```json
{"ok":true,"verdict":"pass","check":"extract","engine_version":"0.3.0",
 "input":{"path":"<as given>","sha256":"<64 hex>"},"specification_sha256":"<64 hex>",
 "capacitance":"upper bound estimated from routed geometry","conditions":["scope: …","…"],
 "lines":[{"net":"I2C_SDA","board_net":"/I2C_SDA","bus":"SENSORS","verdict":"pass",
           "c_bound_femtofarads":50576,"declared_femtofarads":100000,
           "terms":{"trace":12922,"vias":6076,"pads":1578,"pins":30000},
           "length_nm":33750000,"segments":28,"via_count":4,"pad_count":4,
           "pins":["U1.137","U2.3","U3.6"],"pull_ups":["R1"]},…],
 "findings":[]}
```

An SCK or MOSI line also carries `"signal"` and `"drivers"`; an I2C line's object is unchanged. A line that a connector carries (`AET-WO-4300f`, ruled D8) also carries `"terms"."offboard"`, each connector pad's declared off-board capacitance (the mated contact, the cable and the remote pins, as the specification states it), and `"offboard"`, listing each as `<reference>.<pad>: <fF> fF (<source>)`. The connector's pad copper is counted, its pin at no `Ci`, and the conditions then end with the one the declared values add. A line with no connector is unchanged.

A line carrying a test pad (`AET-WO-1312`, the specification's `test_points`) counts the pad's copper, as every pad's, and no `Ci`: no part sits on it. A probe clipped to it is an off-board load the bound does not hold, so the conditions end with one line per such pad, naming it and its line, in the lines' order (`AET-WO-1604` §1.5): "`` `TP1` is a test pad on `I2C_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance ``". A test pad on a net no line bounds, and a line with none, add nothing; no bound changes. `verify` copies these conditions into the package's `FAB-NOTES.md`, as it copies every other.

A line carrying a device whose specification declares its pin capacitance (`AET-WO-4220b`, specification version 13, `pin_capacitance.declared`) also carries `"terms"."declared_pins"`, the declared value summed over that device's pads on the line, and `"declared_pins"`, listing each such pad as `<reference>.<pad>`. Those pads are never in `"pins"`, which lists pins counted at a maximum (cited in the parts table, or UM10204's `Ci` on I2C). When an I2C line carries one, the standing condition on I2C pins names only the pins it covers ("every I2C pin not counted at a declared value at most UM10204's Ci of 10 pF; …"), and so, when an SPI line carries one, does the condition on SCK and MOSI ("each device pin not counted at a declared value at its part's cited maximum …"). The conditions then end with one line per declaring device, in the specification's order (I2C devices, then SPI): its value, its condition and its source, and that the value is declared, not a maximum the datasheet prints, so the bound holds only under that condition (`RUL-19`). A declaration is accepted only for a part whose row cites no maximum (`validate`'s `PinCapacitanceAlreadyCited`). An SPI device with neither is refused (`UnboundedPinCapacitance`). A line with no declared pin is unchanged.

Exit `0` when every line's bound is at most its declared bound; `1` otherwise, with one `capacitance_over_declared_bound` finding per line over its bound, naming the line, the bound, its four terms (and `offboard` and `declared pins` when present) and the declared value. A specification with no I2C bus passes with no lines.

**Scope and conditions** (`conditions` in the verdict). The bound is the bare board's copper plus each pin's `Ci`. These are outside it:
- part bodies: a ceramic resistor body's permittivity is near 10, above the bound's 5.4;
- solder;
- a part's terminations above its pads.

What it assumes of the fabricated board, which the board file does not state:
- laminate and solder mask permittivity at most 5.4, the IPC-4101 FR-4 maximum (specified at 1 MHz; IPC gives none below);
- finished copper at most 70 µm thick, and a finished board from 0.34 mm to 2.0 mm thick;
- a ground-zone fill that keeps the clearance, as the DRC gate's refill judges;
- every I2C pin within UM10204's `Ci`, so no multiplexer or switch on the line;
- other nets static while the line rises.

## `check speeds`

```
aethelgard check speeds --spec <path>
```

Each pin the board crate configures, with the `OSPEEDR` value the topology stores (`pin_speeds`, which `solve` chose and every generator reads, `AET-WO-4400b` D2) and why (`AET-WO-4300c`). A judged line holds only if the stored speed's own row meets its rate and rise time; an out-of-scope line fails unless stored as `00`. No tool runs. The check judges the lines whose edges the MCU drives at the rate its peripheral sets, each SPI bus's SCK and MOSI. Each gets the slowest of `00` and `01` whose DS12117 Table 149 Fmax (note 3) meets the bus's `speed_hz`, and whose tr/tf meets the line's declared rise time. It is judged at:
- the VDD band its supply's minimum selects (nominal less tolerance; 2.7 V exactly is the lower band);
- its declared capacitance, rounded up to 10, 30 or 50 pF.

Speeds `10` and `11` need the I/O compensation cell and are never selected. Every other line is `00`, named with its reason:
- open-drain I2C;
- MISO, an input in master mode;
- a software-driven GPIO output such as a chip select.

The capacitance judged is the declared one, never a bound: this check draws no board. `"capacitance"` says to run `extract` (or `verify`, whose extract gate does) on the routed board, or that it is `UNVERIFIED` when no board can be drawn, naming `emit`'s refusal. `"judged"` counts the lines judged; it is 0 for a specification with no SPI bus.

## `verify`

```
aethelgard verify --spec <path> [--scenarios <file>...] [--against <dir>] [--fab-out <dir> | --against-fab <dir>]
# --fab-out and --against-fab each require --scenarios
```

One verdict that binds a specification to every artifact it produced (`INV-01`, `INV-03`, `RUL-19` (2), `AET-WO-3301` §1.3). Only `verify` binds a pair; every other gate binds only its own input.

1. **Generate in memory.** Read, validate and solve `--spec` as `solve` does, and generate the tree as `generate` does. Every hash the verdict names comes from these bytes.
2. **Check the scenarios.** Each is read as `campaign` reads it, and its `base` must be byte for byte `--spec` (exit `3`, `ScenarioBaseMismatch`).
3. **`--against <dir>`**: compare `<dir>` with the tree byte for byte (`board/target/` is skipped, as a crate's `target/` is; a symlink is never followed and is a difference). Any differing, missing or extra file is a fail, exit `1`, with one finding each (`against_differs`, `against_missing`, `against_extra`, the path first in `message`), and no gate runs. This is `INV-01`'s "regenerate and diff", runnable by a user.
4. **Run every gate on the tree**, written into a guarded scratch directory, each as this binary's own subcommand: `check erc` on the schematic; `check drc --spec --schematic` on the board; `check support` on the schematic, the board and the specification; `extract --spec` on the board; `prove --spec` on the board crate, once for each harness of its `src/proofs.rs`; `build`; `simulate --firmware` with the generated platform for `100ms` (every board, `AET-WO-2302`); and, with `--scenarios`, `campaign` over them with that ELF into a scratch archive.
5. **Bind each gate to the bytes.** The hash each gate reports for what it judged must be the generated one: the schematic, the board, the parity schematic and the specification (ERC, DRC); the schematic, the board and the specification (`check support`); the board and the specification (`extract`); the crate hash of `board/`, and `harness_provenance` `generated` (`prove`, `build`); the platform, the ELF `build` reported and the specification (`simulate`); and each run id, recomputed from the scenario, `--spec` with its overrides and the ELF (`campaign`). Anything else is exit `5`, `BindingMismatch`, naming the gate: a file swapped in `verify`'s own scratch, or a gate that reports judging other bytes, is never named in a verdict.

**What `verify` does not bind.** Each gate hashes its own copy and then hands the copy's path to its tool, as every gate does. A process of the same user that writes into a gate's private scratch in between (a source file before cargo reads it, a `.cargo/config.toml` in the tool's `HOME`, an ELF before Renode loads it) changes what the tool judges while the gate still reports the generated hash. The adversarial pass made `verify` pass that way (`AET-WO-3301` FEEDBACK F1). `verify` binds a pair against everything but a concurrent process with the user's own identity, which by `RUL-19` (4) can also replace the tools themselves. Each gate entry carries the gate's `uid_boundary` and, when it is `false`, its `uid_boundary_unmet`. The campaign's is `true` only when every run's is, and a gate that did not run has `null`. Where it is `true`, that window is closed as [The verdict](#the-verdict) says (`AET-WO-1601`).

A gate that refuses stops `verify` with the gate's own `kind`, `id` and exit code, its `message` prefixed with the gate's name (`INV-06`). Otherwise it prints one line, exit `0` when every gate passed and `1` when any failed:

```json
{"ok":true,"verdict":"pass","check":"verify","engine_version":"0.3.0","project_name":"sentinel-01",
 "specification_sha256":"<64 hex>","topology_sha256":"<64 hex>","files":{"board/Cargo.lock":"<64 hex>",…},
 "against":"identical","elf_sha256":"<64 hex>","rise_times":"computed from declared values",
 "gates":[{"gate":"erc","verdict":"pass","exit_code":0,"tool_version":"10.0.7","uid_boundary":true,"input_sha256":"<64 hex>","findings":[]},…,
          {"gate":"prove","harness":"proofs::every_valid_sequence_i2c1","verdict":"pass",…,"harness_provenance":"generated","cover_count":6,"findings":[]},…],
 "campaign":[{"scenario":"pullup-2k2","verdict":"pass","expect":"pass","matched":true,"findings":[],"run_id":"<32 hex>","state_sequence_hash":"<64 hex>"}],
 "findings":[]}
```

- A gate after a failed `build` is `not_run`, with `exit_code` `null` and its `reason`.
- **`ran`, the "ran or not" table** (`AET-WO-1609`, review 3 B5): after `gates`, before `against_fab` and `fab`. It holds one row per gate `verify` can run, in order: `erc`, `drc`, `support`, `extract`, `prove` (one per harness), `build`, `simulate`, `gpio`, `adc`, `bridge`, `campaign` and `attribute` (one per run it attributed). Each gate's row is followed by the rows its own line states for its parts. Every row has exactly one of:
  - `judged` (the count it judged, `0` included, never omitted);
  - `off` (the specification key, or for a gate no key switches, the flag or condition, that turns it on);
  - `not_reached` (why it could not run).

  The rows come from the gates `verify` ran: `not_applicable` is `off: board.drawn`, and `not_run` is `not_reached` with the gate's own reason. `prove` is always a row: with no Kani harness in the generated crate it is `not_reached: no_harness`, beside the error finding `no_harness` (`AET-WO-1610` (1)), never `off`, since no key or flag turns a harness off. A gate that judged nothing is its `judged 0` row; a droop run that witnessed no device says so at its own `campaign[i].observation_witness`. A gate that is absent is off by what turns it on: `gpio`, `adc` and `bridge` by `pin_devices[].signals`, `.analog` and `.bridges`; `campaign` by `--scenarios`; `attribute` by a seeded fault that fails as expected beside a passing baseline. When `simulate` was not reached, each of those is not reached too. The parts:
  - `extract.lines` (always);
  - `extract.supply` (a non-empty `power.drop_limits`; `[]` validates and judges nothing);
  - `extract.oscillator` (`mcu.hse.crystal`; each net judged by its no-via rule);
  - `extract.pairs` and `drc.pairs` (`diff_pairs`);
  - `drc.power` (`power`);
  - `drc.orientation` (`spi_devices[].orientation`; each declared IMU's seat, `ImuOrientationDisagrees` when it disagrees);
  - `drc.outline` (`board.max_outline_um`; `OutlineExceedsBound` when the board's outline is over it);
  - `drc.radio` (`radio_modules`; each module judged, its row present only where a module is declared, so no other verdict moves, `INV-08`);
  - `simulate.hse_clock` (`mcu.hse`, judged once the boot word is written).

  On the reference, every part but `extract.lines` is off by its key. A gate pushed under a name the table does not know stops `verify` (exit `70`).
- On a specification that declares `board.drawn: false`, `erc`, `drc`, `support` and `extract` are `not_applicable`, with a `reason` and `exit_code` `null`, and none runs. The verdict is never `pass`: with every other gate passed it is `not_verified`, `reason` `DrawsNoBoard`, exit `7`; with one failed it is `fail`, exit `1` (`AET-WO-1607` (5), D2). On a drawn board no gate is `not_applicable`.
- Each gate's `uid_boundary` is its own verdict's `sandbox.uid_boundary` (`AET-WO-1601`). It is `null` for a gate that did not run, and for `extract`, which runs no tool and so has no `sandbox` in its verdict.
- **The campaign gate passes a run that survived or that detected its fault** (`AET-WO-3303` D1, ruled, reversing `AET-WO-3301` §3.2). A run survived when its own verdict is `pass` and matched. A run detected its fault when its scenario seeds one intolerable fault on one target (a pull-up off its declared value, a raised line capacitance or a held line on an I2C bus, an absent SPI device, an open RX line on a serial port), declares only that fault's kinds, fails exactly as declared, and `attribute` on its run (bound by run id and state-sequence hash, reported as an `attribute` gate naming the `scenario`) names the `electrical` half by the fault's rule (`E1` on an I2C bus, `E2` for an absent device, `E3` for an open RX line, whose only kind is `serial_echo_missing`) with nothing unexplained or masked and no subject but the seeded bus and its devices, the absent device and its controller, or the open port (`AET-WO-2302` D5); and the campaign holds a no-fault baseline (every override the specification's own value) that passed. Any other failure fails the gate, naming why: the 47 kΩ scenario alone fails `verify` (no baseline), and beside the baseline it is detected.
- `rise_times` says what the analog records are (`RUL-19` (1)): computed from the declared pull-up and capacitance, never measured on a board.
- The verdict names no path in scratch, so two runs on the same inputs print the same bytes.
- `verify` takes as long as its gates: about 2 minutes 15 seconds for the reference with one scenario, inside the tools image on Furnace, most of it the two proofs.

**What the twin witnesses** (`AET-WO-4230` §1.7, `RUL-20`). After `findings`, before any fab key, the verdict carries:
- `declared_peers`: every serial port whose `peer_deviation_source` is `declared`, with its `peer_deviation_ppm`, in the specification's order (R2); a `cited` port is not named.
- `silicon`: each SPI controller's `check_spi`, each I2C controller's `check_i2c` and each serial port's USART `check_brr` (`port`, `peripheral`, `kernel_hz`, `baud`, `brr`, `residue`, `nearest`, `in_range`, `holds`; R3), and `holds`. One that does not hold is also a `silicon_rule_fails` error finding by name.
- `ledger`: one row per register field the generated firmware writes or depends on at reset, classed `independent-rule`, `observed`, `same-source`, `ignored-by-model` or `silicon-only`, with its judge and note; `counts` by class and family, and `unacknowledged`. Each row of the last three classes carries `acknowledged`, `true` with the acknowledging entry's `reason` when the specification's entry for its register and class names its field, else `false`; nothing is dropped. A motor output's `PUPDR` field is `independent-rule` (`silicon::implied_pupd`: `00`, no internal pull, because the pull-down is the declared resistor `motor_outputs[].pull`, `AET-WO-4300i` D0); a value other than the rule's refuses the run, exit `70`, `LedgerRuleDisagrees`.
- `twin_limits`: what the twin cannot judge outside the register fields (each scripted device's start-up wait, each register a part's model answers 0x00 with a warning), never acknowledged and never blocking a package (the architect's ruling on Q-I).

### The fabrication package (`--fab-out`, `--against-fab`)

`AET-WO-3300`, ruled on PR #103. Both flags require `--scenarios` (ruling D3): `INV-03` counts the fault campaign among the gates of "verified", so a call without it is a usage error, exit `64`, before anything is read. Before any gate each flag refuses, exit `3`: `HarnessSkipped` when the board crate declares a Kani harness the plan does not name, and `CampaignCoverageShort` when the scenarios, as declared, leave a declared bus or rail with no fault (`AET-WO-3303` §1.1, D1 as ruled): every I2C and SPI bus a device uses and the MCU's supply need a tolerated fault, or (a bus) a detectable one beside a baseline; every serial port needs its open RX line (`serial_rx_open`) detected beside a baseline (`AET-WO-2302` D5, which ends the `no fault kind yet: serial` exemption ruled on #155); a rail other than the MCU's supply and the battery no fault kind reaches, so a board that declares one is refused until one does; a bus no device uses is named `no device: nothing to fault` and does not block. The verdict's `fab.preconditions` and the manifest's `preconditions` list `every_harness_proved`, `campaign_faults_every_bus_and_rail` (with `no_device`) and `every_gate_passed`, each `met` or `unmet` with what fails it. The reference's seeded set is `pullup-3k3`, `droop-bor3`, `spi-absent-u4`, the `pullup-2k2` baseline and `serial-rx-open-console`. After those, each flag refuses, exit `2`, `UnwitnessedFieldNotAcknowledged`, a specification that leaves any `same-source`, `ignored-by-model` or `silicon-only` ledger row unacknowledged, a field newly written in an acknowledged register included (`AET-WO-4230` R1): `id` is the first such row and `message` names every one, then the reason rule. This refusal alone adds `error.entries` (`AET-WO-2307`), what `acknowledge_unwitnessed` lacks in its own shape: one `{register, class, fields, reason}` per register and class with a row left unacknowledged, sorted by register, then class, its `fields` sorted. Its `fields` are exactly those rows' fields, and, where the specification holds an entry for that register and class, that entry's fields as well: the entry comes back completed, to replace the held one (pasted beside it with its reason written, it is refused as `UnwitnessedAcknowledgedTwice`). Every `reason` is `""`, a completed entry's included, since a reason written for some fields cannot vouch for fields it never saw (the architect's ruling on `AET-WO-2307` F1). `validate` refuses `""` (`InvalidSource` at `acknowledge_unwitnessed[i].reason`), so a pasted entry cannot pass before a person writes its reason: 1 to 200 bytes of printable ASCII, no space at either end. Every field is a ledger row of that register and class, so the stale check (`UnwitnessedAcknowledgementStale`) passes the result, and with every reason written the check passes. Sentinel-03 needs 43 entries, 6 of them completing a held entry (`spike/2307/`). The reference acknowledges all 168 of its rows in 30 entries. Only when every gate has passed, `verify` exports the package and writes it into `--fab-out <dir>`. The directory must be empty or absent. If it exists, it must be the caller's and writable by no group or other, or another uid could replace the package after the verdict names it. It is checked before any gate runs and again just before the package is written, and `tree::write` checks emptiness once more through the descriptor it writes through (exit `3`, `UnwritableOutput`). Nothing is created before that. The flat package, named after the project:

| File | What it is |
|---|---|
| `<project>-F_Cu.gtl`, `-B_Cu.gbl`, `-F_Paste.gtp`, `-B_Paste.gbp`, `-F_Silkscreen.gto`, `-B_Silkscreen.gbo`, `-F_Mask.gts`, `-B_Mask.gbs`, `-Edge_Cuts.gm1` | `kicad-cli pcb export gerbers --check-zones`, with KiCad's two clock lines removed (below) |
| `<project>.drl` | `kicad-cli pcb export drill --excellon-min-header`, unchanged |
| `<project>-pos.csv` | `kicad-cli pcb export pos --format csv --units mm --side both`, unchanged |
| `<project>.bom.csv` | `emit`'s bill of materials, byte for byte |
| `<project>-jlcpcb-bom.csv`, `<project>-jlcpcb-cpl.csv` | the BOM and the placement in JLCPCB's assembly upload formats (`AET-WO-1604` §1.3, ruled on PR #199), laid out as JLCPCB's sample files lay them out and made from the two files above, adding nothing to them (`INV-06`). The BOM is `Comment,Designator,Footprint,JLCPCB Part #（optional）`, one row per kind of part grouped by value and footprint, in the order of each kind's first reference (`C2` before `C10`): `Comment` is the value, `Designator` every reference of the kind, quoted, and `Footprint` the KiCad footprint's name without its library. **The part number is empty for every part**, because no input names one. The CPL is `Designator,Mid X,Mid Y,Layer,Rotation`, one row per placed part in KiCad's order: `Mid X` and `Mid Y` are KiCad's coordinates with `mm`, `Layer` is `T`, and **`Rotation` is KiCad's, normalised to 0 to 359 degrees, in KiCad's frame with no per-footprint offset**. Lines end CRLF. `FAB-NOTES.md` cites JLCPCB's pages and says to check the placement preview by eye. The two must name exactly the same parts, or the export is exit `5`, `ToolError` |
| `<project>.elf` | the ELF `build` wrote, read back and bound to the `elf_sha256` that `simulate` and every campaign run were bound to |
| `FAB-NOTES.md` | generated by `aethelgard-eda`. It states the specification's declared board (AET-WO-4100): its copper layers and, at 4 layers, each plane's net and every dielectric's minimum with its source; FR-4 and the declared thickness; `extract`'s conditions as `extract` derives them from that declaration (for the reference: copper at most 70 µm, a finished board of 0.34 to 2 mm, permittivity at most 5.4); that every hole is a plated 0.3 mm via; the declared rule minima; the specification's sha256, how the Gerbers were made, and that the package was emitted by a passing `verify` and claims nothing beyond it (`RUL-19`) |
| `MANIFEST.json` | every other file's sha256, the specification's and topology's, each `--scenarios` file's name and sha256 in the order given, and `verdict_sha256` (below). Written last |

**How it is made.**
- The board is copied from `verify`'s scratch into the export's own scratch and hashed. The hash must equal the generated board's, or the call stops with exit `5`, `BindingMismatch`. The copy is prepared as `check drc` prepares it: the specification's setup and rules, the vendored libraries (`RUL-03`).
- Every `kicad-cli` call runs in the sandbox, as every tool does.
- `kicad-cli version` is reported in `fab.tool_version`. It is not compared, because the clock-line rule below pins 10.0.7 in every Gerber.
- **A part on the bottom side refuses the package by name**, exit `3`, `BottomSidePart` (`id` the reference), and nothing is written: JLCPCB's pages state no bottom-side convention (mirrored X, rotation seen from below), and no placement row is written that cannot be filled. No board places one today.
- The export refuses with exit `5`, `ToolError`, and writes nothing, when:
  - a placement row is not three quoted fields with no quote inside, then four plain ones. KiCad does not escape a quote in a reference, value or package, so such a name would shift every later column;
  - a zone the board declares has no region of its net on its copper layer (`%TO.N,<net>*%` then `G36`…`G37`), so the copper plotted would not be the copper DRC judged;
  - the emitter's BOM and KiCad's placement file do not name exactly the same parts, or either is not what its writer writes;
  - a pad of the board is drilled, or the drill file is not metric, does not have exactly one tool (the 0.3 mm via drill), or does not have one hit per via. `FAB-NOTES.md` states that every hole is a plated via, and these checks are what make that true.

**KiCad's clock.** KiCad 10.0.7 writes the time of export into two lines of every Gerber, and no option of its own removes them (FEEDBACK §1.1). These are line 2, `%TF.CreationDate,<YYYY-MM-DDThh:mm:ss±hh:mm>*%`, and the creator line, `G04 Created by KiCad (PCBNEW 10.0.7) date <YYYY-MM-DD hh:mm:ss>*`. The creator line is line 9 of every layer but the outline, where it is line 8: the outline has one header attribute fewer.
- Line 1 must be `%TF.GenerationSoftware,KiCad,Pcbnew,10.0.7*%`. Both clock lines must be exactly there and exactly so.
- No other line may carry `CreationDate`, `Created by`, `dddd-dd-dd` or `dd:dd:dd`. The file must be ASCII with `\n` line ends.
- The two lines are then deleted, never rewritten, and every other byte is kept. Any departure is exit `5`, `ToolError`, and nothing is written.
- KiCad's Gerber job file is not included.

**The verdict.** A `--fab-out` run's verdict ends with a `fab` key, after `findings`:

```json
 …,"findings":[],"fab":{"status":"written","tool_version":"10.0.7","uid_boundary":true,"board_sha256":"<64 hex>",
 "files":{"FAB-NOTES.md":"<64 hex>","MANIFEST.json":"<64 hex>","sentinel-01-B_Cu.gbl":"<64 hex>",…}}}
```

- `fab.files` names every file written, `MANIFEST.json` included, by the sha256 of the bytes written. The package is written before the verdict is printed. A refusal while writing prints no verdict, and its message says the directory holds an incomplete package that no verdict names.
- **`MANIFEST.json`'s `verdict_sha256`** is the sha256 of the line the same call prints without `--fab-out`. That is this verdict without its final `,"fab":{…}`, and with no newline. The verdict cannot name the manifest's hash while the manifest names the verdict's, so the manifest names the verdict without the package.
- **The verdict states each gate's sandbox, so `verdict_sha256` does too.** A run under another uid or user namespace (rootless podman's root against Docker's, say), or with other `--scenarios` or `--against`, prints another verdict. Its `--against-fab` then names `MANIFEST.json` as differing while every other file matches.
- `fab.uid_boundary` is `true` only when all three exports ran under the uid boundary. `fab.uid_boundary_unmet` names the first condition that failed, as each gate's does.
- **A failed gate, or an `--against` that differs, writes nothing.** The verdict's `fab` is `{"status":"not_written","tool_version":null,"uid_boundary":null,"board_sha256":null,"files":{}}`, with exit `1`.

**Radio modules** (`AET-WO-4411`). `verify`'s verdict carries `radio`, one line per declared module naming its firmware as not judged, its radio as not simulated, and its ROM's output on its UART at every boot as unjudged, and its supply as the floor rule judges it: on the MCU's rail, a VDD minimum above the reset's falling minimum; on a rail of its own, held by no monitor (`AET-WO-4310` (2)); the key is absent on a board without one. `verify --fab-out` refuses a package with any powered part (the MCU at VBOR0's minimum, each I2C, SPI and radio device, the supervisor, each translator side, each regulator fed from a rail) above its rail's reset or named: on a rail no monitor holds, or with no supply minimum held (`FabricationNotReady`, every one listed).

**Lifecycles** (`AET-WO-4210` D1, ruled by Scotty). `validate`, and so every subcommand that reads a specification, refuses a board that uses an `eol` or `nrnd` part its `accept_lifecycle` does not acknowledge at that status with a statement (`LifecycleNotAccepted`, `LifecycleStatusMismatch`; an entry twice, with an empty statement, or for a part the board does not use as such is refused too). An acknowledged part is named in a `lifecycle` banner, one line per part with its status, devices and vendor citation: in `validate`'s report, every `simulate --spec`, `verify`, `generate`, `check speeds`, `extract`, `emit`, `check drc --spec`, `check support`, `prove --spec` and `check pinout` verdict (`AET-WO-1607`; `check pinout` reads it from the topology), on `solve`'s stderr (its stdout stays the topology alone), and the header of every file the board crate's generator and the platform's emitter write. A board of `active` parts carries none, so its outputs are unchanged. With either fab flag, a part whose lifecycle the table cites as pending (not held; none today: the W25Q64JV's was until `AET-WO-4211` cited Winbond's catalog export) is named in `fab.lifecycle_unheld`, with the page it must be read from.

**`--against-fab <dir>`** derives the package the same way, after every gate passes, and compares it with `<dir>` file by file:
- `against_fab` is `identical`, or `differs` with exit `1`. In that case `fab.differences` names every file that differs, is missing or is extra (`against_differs`, `against_missing`, `against_extra`, the name first in `message`).
- `fab.status` is `derived`.
- With a failed gate, `against_fab` is `not_compared` and `fab.status` is `not_derived`.
- `--fab-out` and `--against-fab` exclude each other, and each requires `--scenarios` (exit `64`). Either may be combined with `--against`, which is part of the verdict the manifest names.

Two runs on one specification and the same scenarios, at any paths, print the same verdict and write the same bytes: the verdict names no path.

## `doctor`

```
aethelgard doctor
```

Says whether this host can run every subcommand, and what blocks each one it cannot (`AET-WO-1604` §1.1), so an exit `4` is explained without reading `run.rs`. It takes no argument and prints one line:

```json
{"ok":false,"verdict":"fail","check":"doctor","engine_version":"0.3.0","engine_commit":"unknown","scratch":"/home/u/.cache/aethelgard","sandbox":{"landlock_abi":8,"tcp_confined":true,"scoped":true,"sockets_denied":true,"uid_boundary":false,"uid_boundary_unmet":"tool_uid"},"tools":[{"tool":"kicad-cli","path":"/usr/bin/kicad-cli","version":"10.0.7","sandbox":{…}},{"tool":"renode"},…],"missing":[{"kind":"ToolMissing","id":"renode","message":"`renode` is not on PATH","blocks":["simulate","campaign","verify"]}]}
```

- **`engine_commit`**: the commit the build stated (`AETHELGARD_COMMIT`, 40 hex digits), or `unknown`. The builder's claim, never proven from bytes (`AET-WO-1802`). `engine_version` is the package version alone. No verdict or committed byte names the commit.
- **`scratch`**: the directory every call's scratch is made in, when it passes the rule under [Where scratch lives](#where-scratch-lives); `null` otherwise. With no scratch nothing can run, so no probe runs: the sandbox is `null`, and each tool is only looked for.
- **`sandbox`**: what the sandbox achieved when it confined a real process, this executable asked `--version` through the same reaper and trampoline every tool runs under, with the fields of a verdict's `sandbox`. Landlock and the seccomp filter are known only by installing them; a layer that cannot be is `null` here and a `SandboxUnavailable` in `missing`, never assumed.
- **`tools`**: `kicad-cli`, `renode`, `aethelgard-bridge`, `cargo` and `cargo-kani`, each with the `path` it was found at, the `version` its probe printed and the `sandbox` the probe ran in, each absent when it does not apply. Each is found, given its variables and read grants, and asked its version by the functions the subcommand that runs it calls: `kicad-cli version`; `renode --config <file> --version` with Renode's variables and grants; `cargo --version` with `build`'s; `cargo kani --version` with `prove`'s. No tool runs beyond that probe. The bridge is asked `aethelgard-bridge --version`, which starts libngspice and runs no circuit: it must print, alone, the identity this CLI was built against naming the library's banner, and exit `0`; `ngspice_loaded` is then `true` and `ngspice` the release the banner named (`44.2` in the tools image, `AET-WO-1606`). A bridge whose `--version` names no ngspice, exits otherwise or says more is `ToolUnusable` in `missing`.
- **`missing`**: each piece missing, as the refusal a blocked subcommand prints (`kind`, `id`, `message`: `ToolMissing`, `ToolUnusable` for a version off its pin or a missing variable, `BridgeUnavailable`, `BridgeMismatch`, `ScratchUnsafe`, `SandboxUnavailable`), and the subcommands it `blocks`. One kind and id is listed once, with every subcommand it blocks. A `kicad-cli` other than 10.0.7 blocks `check drc`, `check erc`, `check support` and `verify`; a Renode other than 1.17.0 blocks only a boot (`simulate --firmware`, `campaign`, `verify`); the bridge blocks `simulate --firmware` when the specification has an I2C line or a declared droop.

Exit `0` when `missing` is empty; `1` otherwise. `doctor` reports what is missing rather than refusing, so through `mcp` it is a successful call either way.

## `mcp`

`aethelgard mcp` serves the other subcommands as [Model Context Protocol](https://modelcontextprotocol.io) tools on stdin and stdout (`RUL-10`, built on `rmcp` 3.7.0 with its `server` and `transport-io` features only). It takes no arguments, listens on no socket, holds no credential, and exits `0` when stdin closes. Stdout carries protocol messages and nothing else; diagnostics go to stderr. It is not a tool itself and adds no tool behaviour.

Before it starts, it runs the trampoline's inherited-descriptor check. A descriptor above 2 without close-on-exec refuses with exit `4`, `SandboxUnavailable`, `id` `descriptors`; the refusal object goes to stderr, since stdout is reserved for the protocol. A `SIGTERM`, `SIGINT` or `SIGHUP` kills the running calls and exits `143`, `130` or `129`, from the first moment, handshake or not. Closing stdin before `initialize` also exits `0`. `SIGKILL` cannot be handled; each running call's CLI then dies by its parent-death signal, and its reaper and guardian clean up.

### The tool list

Generated at startup from the unbuilt `clap` command tree, never written by hand, so a new subcommand appears with no change to the `mcp` module (`INV-01`):

- one tool per leaf subcommand, and one per command that also runs alone with its own arguments (`generate`, beside `generate hwdef`), named by its path joined with `_`: `init`, `validate`, `solve`, `emit`, `generate`, `generate_hwdef`, `build`, `verify` (whose `scenarios` is repeatable and optional, and whose `fab-out` and `against-fab` are optional), `extract`, `check_drc`, `check_erc`, `check_speeds`, `prove` (whose optional `spec` gives `harness_provenance`), `simulate` (whose optional `firmware`, `spec` and `for` are the boot), `campaign`, `replay`, `attribute`, `propose` and `doctor`. `help` and `mcp` are never tools;
- the description is the subcommand's about text;
- the input schema is an object with one **string** property per declared argument, keyed by its **long name** (`prove`'s directory is `crate`, not `crate_dir`), `required` exactly when `clap` requires it, and `additionalProperties` false. A repeatable argument (`campaign`'s `scenarios`) is a non-empty **array of strings** instead.

Every argument must be a long-named, value-taking option, set once or repeatable. Anything else (a positional, a boolean flag) makes the tool list refuse to build, which a unit test catches before a client can.

**The snapshot is the MCP contract** (`AET-WO-1604` §1.2). `tests/mcp-tools.json` is the tool list exactly as `tools/list` serves it, and `tests/mcp_snapshot_tier1.rs` fails, naming each tool and field that differs (a tool added or removed, the order, and every field by its JSON pointer, such as `tool `extract`: field `/inputSchema/required/2` added ("rev")`), whenever the served list moves. A changed argument, a new subcommand or an edited help line therefore changes the contract visibly. After a deliberate change, one command rewrites the snapshot, and its diff is the change under review:

```
AETHELGARD_BLESS=1 cargo test -p aethelgard-cli --test mcp_snapshot_tier1
```

### A call

The server checks the arguments against the schema (declared names only, string values only, or a non-empty array of strings for a repeatable argument, required ones present, no NUL byte). A failure is an MCP invalid-params error (`-32602`) and nothing runs; so is an unknown tool name. Otherwise it runs this binary itself as a child, with stdin from `/dev/null`, and the argument vector is the subcommand path followed by one `--<long>=<value>` token per argument, and per item of a repeatable one. One token per argument means a value starting with `-` can never be read as a flag; the CLI judges it. The internal first arguments are not in the tree, so no call reaches them. The call therefore has exactly the CLI's sandbox, exit codes and JSON.

`validate` reads stdin when `file` is absent, and the child's stdin is empty, so pass `file`: without it the call is refused as empty input (exit `3`).

### The result

For a child that exits and prints exactly one JSON object line:

- `content[0].text` is that line, unchanged, without its newline;
- `structuredContent` (its object keys sorted, so not byte-identical to the text) is `{"exit_code": <n or null>, "verdict": <the child's object>}`. `null` means the child was killed by a signal;
- `isError` is `false` for exit `0` and `1`, because a fail verdict is a successful call, and for `propose`'s `6` and `verify`'s `7` (`not_verified`, `AET-WO-1607`), which are verdicts too; it is `true` for every other exit code. A refusal, such as `ZeroNominalVoltage` (exit `2`), is therefore `isError` `true` with the refusal object as the verdict.

`campaign` alone prints one line per scenario before its summary (`LINE_PER_RECORD` in `main.rs`): every line must be a JSON object, `verdict` is the last, and `structuredContent` adds `"records"`, the lines before it. `content[0].text` is then every line.

`isError` is `true` for exit `null` as above. It is also `true`, with no `structuredContent` and a text naming the problem, when stdout is not exactly one JSON object line (for `campaign`, one or more) (a child killed before it printed one, for example), and when it exceeds a cap (the server kills the child, so a legitimate verdict over 1 MiB is reported as the cap): stdout 1 MiB, stderr 64 KiB. Past a cap the server kills the child and keeps draining until the pipes close.

### Cancellation and bounds

A `notifications/cancelled` for a call kills its child; the CLI's reaper then ends the tool's whole process group. Closing stdin lets running calls finish for up to 5 s (`rmcp`'s grace), then cancels them. The server sets no timeout of its own (`RUL-13`); only `simulate --firmware` bounds its tool, with the wall bound and memory cap above. `rmcp` drops a JSON-RPC line over 16 MiB without replying. The number of concurrent calls is not bounded by this server.

### Human check

```
npx @modelcontextprotocol/inspector aethelgard mcp
```

lists the 21 tools, and a `validate` call with `file` set to `examples/sentinel-01-minimal.json` returns `isError` false.
