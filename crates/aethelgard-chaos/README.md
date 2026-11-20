# aethelgard-chaos

Fault scenarios, the campaign runner, the trace archive, and verdicts
(`docs/architecture.md` §3). This crate holds the **trace archive**
(`AET-WO-0400`, `RUL-06`, `INV-04`): a deterministic CBOR format, a local
append-only store a run writes as it goes, and a replay that refuses any trace
it cannot prove complete. Since `AET-WO-2300` it also holds the **scenario
model** and the **campaign's records and run rules** (below); the loop that
boots each scenario is `aethelgard campaign`, in the CLI, because it runs
`simulate --firmware`'s own path.

`sha2`, `thiserror`, `rustix` (for the reader's descriptor-relative opens),
and for scenarios `serde` and `serde_json` are used. Nothing here reads a
`Specification` (`RUL-16`): the CLI applies a scenario's overrides. There is no CBOR crate: the encoder and the strict
decoder are `src/cbor.rs`.

## The profile

RFC 8949 §4.2.1 core deterministic encoding, restricted further.

- Admitted: major types 0 (unsigned), 1 (negative), 2 (byte string), 3 (text
  string, valid UTF-8) and 4 (array).
- Refused: maps (5), tags (6), floats and simple values (7, booleans included;
  a boolean is the unsigned integer 0 or 1), indefinite lengths, additional
  information 28 to 31 where they are not a length, and any argument not in its
  shortest form.
- Bounds, checked before anything is allocated: an array has at most 65,536
  elements, a string at most 1 MiB, nesting at most 8 arrays deep (the outermost
  array is depth 1).
- Each file is consumed exactly: trailing bytes, padding included, are refused.
- Every refusal is a `CborError` carrying the byte offset of the item refused
  (of the first extra byte, for trailing bytes).
- The defining property: every byte string the decoder accepts is the encoding
  of the value it returns. `tests/cbor.rs` tries it on mutated and random bytes.

## The run directory

A run directory holds exactly `trace.cbor` and `manifest.cbor`, and optionally
`mirror.cbor` and `mirror.cbor.tmp`. Each is a regular file. Anything else, a
symlink or a leftover `manifest.cbor.tmp` included, is refused. The reader
opens the directory once (`O_DIRECTORY | O_NOFOLLOW`), lists it and opens each
file through that descriptor (`O_NOFOLLOW | O_NONBLOCK`), and checks and reads
the open file: a name swapped for a symlink or a FIFO after the listing is
refused, never followed or waited on (`AET-WO-2300`). Replay reads
neither mirror file (`mirror.cbor.tmp` is what a crash in the mirror step
leaves behind; it must not orphan a committed run).

- **`trace.cbor`**, a CBOR sequence (RFC 8742): the header, then one item per
  event, no enclosing array, so it can be appended to.
  - Header: `["aethelgard-trace", 13, run_id, "ns", artifacts, tools]`. `run_id`
    is 16 bytes. `artifacts` is `[name, sha256]` pairs sorted by name, names
    unique, each hash 32 bytes: what the run judged (`INV-03`). An artifact name
    is 1 to 1,024 bytes of printable ASCII, space allowed. `tools` is `[name,
    version]` pairs sorted by name, names unique: what judged the run, each
    version as the tool reported it (`AET-WO-2205` §1.1). A tool name is 1 to 64
    printable ASCII bytes with no space, a version 1 to 256 with spaces allowed.
    Replay reports them and never compares them with the replaying host's.
  - Event: `[kind, timestamp, …payload]`. Timestamps are unsigned, in
    nanoseconds, and never decrease; equal timestamps are allowed and file order
    decides. Events are never re-sorted.

    | kind | name | payload |
    |---|---|---|
    | 0 | register write | `address` ≤ `u32::MAX`, `value` ≤ `u32::MAX` |
    | 1 | pin level | `pin` (text), `level` (0 or 1) |
    | 2 | interrupt | `line` ≤ 1023 |
    | 3 | analog sample | `net` (text), `microvolts` (any `i64`) |
    | 4 | fault injected | `scenario` (text), `detail` (text) |
    | 5–30 | a campaign record | see "Campaign records" |

    Pin, net and scenario names are 1 to 64 bytes of `0x21`–`0x7E` (`RUL-14`).
    `detail` is 0 to 1,024 bytes of `0x20`–`0x7E`. An unknown kind, or a payload
    of the wrong length or type, is refused.
- **`manifest.cbor`**: `["aethelgard-manifest", 2, trace_bytes, trace_sha256,
  event_count, state_hash]`. Written last; it is what commits the run.
- **`mirror.cbor`**, optional: `["aethelgard-mirror", 1, uploaded, reason]`.
  Written after the commit; replay never reads it, so it is not covered by the
  manifest: its contents and size are unchecked.

## Version rule

The trace header carries version 15. `AET-WO-2300` added kinds 5 to 14 at
version 2, `AET-WO-3100` kind 15 and the run rules that join to it at version
3, `AET-WO-2301` kinds 16 to 20, a declared supply droop, at version 4, and
`AET-WO-2204` kind 13's device log at version 5; each time the golden trace's
bytes changed in that one header byte only. `AET-WO-2205` added the header's
`tools`, the window on kinds 7 and 8, kind 10's executable segments and kind
19's slimmed witness at version 6; the golden trace's header gained its sixth
item, an empty `tools`. Versions 7 to 13 added the SPI records (`AET-WO-4200`, kinds 21 to 23),
the motor records (`AET-WO-4300a`, kinds 24 to 26), each device step's own
records (`AET-WO-4210`, `AET-WO-4220`), the serial records (`AET-WO-4300b`,
kinds 27 to 29), kind 10's brown-out word and flash accesses (`AET-WO-4230`),
and the anchored windows (`AET-WO-2302`: kind 26's anchor, the edges outside
its window and its timer's `CEN` rises, kind 29's injection instant, kind 30).
Version 14 (`AET-WO-4400g`) changed what kind 26's period and high time mean:
the ns Renode realizes at kind 25's `kernel_hz`, the frequency the twin now
builds every timer at (`record::realized_ns`, the one definition the twin
also calls), no longer 4 ns cycles of a fixed 250 MHz; the golden trace's
bytes changed in the version byte only. Version 15 (`AET-WO-2304b`) added
kind 31, a battery sag's declared chain (its sag, the pack's open voltage, the
healthy path R, each battery-fed sink's power and floors, the capacitance on
`VBAT`, the static operating point, the deck's solved minimum and the binding
floor), re-judged in replay by `battery::check_battery_sag` from its own
fields, and the overrides' tags 6 (`battery_sag`) and 7 (`rail_out_of_band`);
the golden trace's bytes changed in the version byte only. An older trace is
refused by name (`event::TRACE_VERSION`), never read under version 15's rules. The mirror record
carries version 1, and the manifest version 2 (`AET-WO-0400a` changed what
`state_hash` means, the trace's bytes not at all). A reader refuses any other
version (`UnknownVersion`), judged before the item's shape. A change to the
format, to the event table, or to the golden constants in `tests/golden.rs`
bumps it.

## Replay and the state hash

Replay folds events into registers (`BTreeMap<u32, u32>`), pin levels, interrupt
counts, the last sample on each net, and a fault count. The state hash is a
chain over each event's change, so replay is linear in events:
`h0 = sha256(header item bytes)`, `h_i = sha256(h_{i-1} ‖ encode(delta_i))`,
and the final `h` is `state_hash`. `delta_i` is one array in the profile:

| event | delta |
|---|---|
| register write | `[0, address, value]` |
| pin level | `[1, pin, level]` |
| interrupt | `[2, line, new_count]` |
| analog sample | `[3, net, microvolts]` (negative: major type 1) |
| fault injected | `[4, new_fault_count]` |
| campaign record | `[kind, …payload]`: the whole record, every field |

A write that leaves the state as it was still has its delta, so it changes
`state_hash`. Timestamps, and a fault's scenario and detail, are not in the
chain; `trace_sha256` covers them. The fold keeps a campaign run's records, in
order, as `State::records`.

`state_hash` is the **state-sequence hash** `aethelgard campaign` prints and
`aethelgard replay` recomputes (`AET-WO-2300` §1.3). It holds no wall-clock
value: timestamps are virtual nanoseconds and stay out of the chain anyway,
and a campaign's `run_id`, the one header field that could vary, is a
function of what the run judged (below).

## Commit protocol

Writer (`TraceWriter`): create the run directory (refused if it exists); create
`trace.cbor` with `create_new` and append the header and each event through a
buffered writer; flush and `sync_all` the trace; write `manifest.cbor.tmp` with
`create_new`, `sync_all` it, rename it to `manifest.cbor`, `sync_all` the
directory. The writer refuses an event or header its own reader would refuse,
and a timestamp that goes backwards.

Reader (`replay_dir`) checks in this order, each refusal its own
`ArchiveError` variant:

| Check | Refusal |
|---|---|
| Run directory is a real directory; entries are only the three names, regular files | `UnexpectedEntry` |
| `manifest.cbor` present | `Uncommitted` |
| `trace.cbor` present | `MissingTrace` |
| `manifest.cbor` is at most 1 MiB (read through `take`) | `ManifestTooLarge` |
| Manifest decodes under the profile | `MalformedManifest` |
| Manifest version is 2 | `UnknownVersion` |
| Manifest's `trace_bytes` is at most 256 MiB | `TraceTooLarge` |
| Trace length equals `trace_bytes` (checked before the file is read) | `Truncated` / `Extended` |
| Trace sha256 equals `trace_sha256` | `HashMismatch` |
| Header and events decode; a header item is at most 1 MiB and an event item at most 4,096 encoded bytes, checked before the item's nested values are built | `NotDeterministicCbor` (with offset), `MalformedHeader`, `UnknownVersion`, `UnknownEventKind`, `MalformedEvent`, `TimestampRegressed` |
| (while folding) at most 65,536 records | `TooManyRecords` |
| (while folding) no second scenario, topology, boot or verdict record | `DuplicateRecord` |
| Event count equals `event_count` | `EventCountMismatch` |
| Replayed state hash equals `state_hash` | `ReplayDiverged` |

`MirrorBeforeCommit` and `MirrorExists` are the mirror step's refusals, not
the reader's. An empty environment variable counts as unset there.

The manifest proves integrity (these are the committed bytes), not
authenticity: whoever can rewrite the events can recompute the manifest.

## The mirror boundary

`mirror_step(dir, env, log)` runs after the commit, never before or instead of
it. `env` is a lookup the caller passes (`|n| std::env::var_os(n)` in
production). It reads `AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID`
and `AETHELGARD_R2_SECRET_ACCESS_KEY`, in that order. If one is unset or empty
it writes exactly one line naming the first such variable to `log` and writes
`mirror.cbor` with `uploaded` 0 and that reason. If all are set it writes
`uploaded` 0 with the reason "remote mirror not built (RUL-06, OQ-03)": nothing
is uploaded, and it never says otherwise. No variable's value is read into a
log, a trace, a manifest or `mirror.cbor`, and the commit's result is unchanged.

The record goes through the same temporary file, `sync_all`, rename and
directory sync as the manifest. The step removes a stale `mirror.cbor.tmp`
before writing its own, so a crash between the two never orphans the run
(`mirror_step_observed` lets a test stop it there).

## Campaign records

Kinds 5 to 30 (`src/record.rs`) are the records `aethelgard campaign` archives
for one scenario's run, in the order of "The run rules" below. Every field is an unsigned or negative
integer, a byte string, a text string, or an array of them: no float, no map.
A flag is 0 or 1. An optional value is an array of zero or one element. A
`reading` is `[address, mask, expected, observed?]`, each a `u32`. The writer
refuses an event over 4,096 encoded bytes, as the reader does.

| kind | record | payload |
|---|---|---|
| 5 | scenario | `name`, `sha256` (32 bytes, of the file), `expect_pass`, `[finding kind…]` (sorted; empty exactly when `expect_pass`), `[override…]`: `[0, bus, ohms]`, `[1, net, femtofarads]`, `[2, bus, line]` (0 SDA, 1 SCL), `[3, domain, floor_millivolts, start_ns, fall_ns, hold_ns, rise_ns, source_milliohms, load_milliamps, bor_level]` (a droop), `[4, reference]` (an absent SPI device) or `[5, port]` (an open serial RX line, `AET-WO-2302` D5) |
| 6 | solved | `specification_sha256`, `topology_sha256` (32 bytes each) |
| 7 | `analog.rise_time_computed` (from declared values, `RUL-19` (1)) | `bus`, `net`, `rise_30_70_ps`, `complete`, `ceiling_ps`, `exceeds` (which must equal `!complete \|\| rise > ceiling`), `window` `[deck_hz, declared_hz]`: the deck's 400 kHz window beside the bus's declared speed, both above 0 |
| 8 | `analog.line_stuck_high` | `bus`, `net`, `microvolts_at_release`, `ceiling_ps`, `window` as kind 7's |
| 9 | `bridge.fault_state` | `bus`, `fault`, `source` (1–128 bytes), `[cause…]`, `[net…]` (sorted), `firmware_sees` (0–1,024 bytes) |
| 10 | the boot | `image_sha256`, the boot word's `reading`, `instructions_expected`, `instructions_observed?`, `pc?`, `executable`: the firmware's executable segments as `[start, end)` pairs at their physical address, sorted, disjoint, nonempty, at most 16; the boot record's brown-out word's `reading` and the flash controller's accesses the bring-up never makes (`[line…]?`, `AET-WO-4230` D10) |
| 11 | a register | `name`, `reading` |
| 12 | an I2C identity read | `reference`, `address`, `register` (each ≤ 255), `expected`, `observed?`, `status?` (text) |
| 13 | an I2C device witness (`AET-WO-2204`) | `peripheral`, `reference`, `device`, `[expected line…]`, `[observed line…]?`, `[absent address…]` (at most 16 lines, each 1–256 printable bytes) |
| 14 | the verdict | `pass`, `[[kind, severity, count]…]` (sorted, unique by kind and severity; each severity exactly `error` or `info`, refused otherwise on write and on read, `AET-WO-3201`; `pass` exactly when every severity is `info`) |
| 15 | a solved I2C bus (`AET-WO-3100`) | `bus`, `peripheral`, `[line…]`: exactly two, SCL then SDA, each `[signal, net, pin, alternate_function, ceiling_ps]` (`signal` is the peripheral's name then `_SCL` or `_SDA`; the two nets and pins differ), `[[reference, address]…]` (sorted, unique by reference). A `pin` is `P`, a capital letter and 0 to 15 with no leading zero (`record::split_pin`); an `alternate_function` is at most 15 |
| 16 | `analog.rail_declared` (`AET-WO-2301`) | the droop's own values (`domain`, `floor_millivolts`, `start_ns`, `fall_ns`, `hold_ns`, `rise_ns`, `source_milliohms`, `load_milliamps`, `bor_level`), then the domain's `nominal_millivolts` and `tolerance_permille` and the emitter's `decoupling_picofarads`: everything the rail is computed from |
| 17 | `analog.rail_crossing_computed` | `domain`, `level` (`VBOR3_falling`…), `microvolts`, `at_ps`, `falling`: one crossing ngspice computed |
| 18 | `bridge.reset_rendered` | `at_ns`, `released`, `[cause…]` (sorted threshold names), `rendering` (the monitor recipe that stood in for the supervisor: the model's rendering, never a measured reset) |
| 19 | `firmware.observed` | `at_ns`, then the boot word's, the identity record's status and value, and the first identity read's witness lines since the release (`[line…]?`), read at the recovery bound after a release |
| 20 | expected counts | `[[kind, count]…]` (sorted, unique), only when the scenario declares `expect_counts` |
| 21 | an SPI step's read (`AET-WO-4200`, kind 12's analogue) | `reference`, `peripheral`, `select`, `address` (its multiplexer address), `step`, `write` (bytes), `width`, `expected`, `mask`, `observed?`, `status?` |
| 22 | an SPI step's witness (kind 13's analogue) | `peripheral`, `reference`, `device`, `step`, `[expected line…]`, `[observed line…]?`, `[unrouted line…]`: the multiplexer's lines inside the step's chip-select window, attribution's evidence, never a witness |
| 23 | a solved SPI controller (kind 15's analogue) | `peripheral`, `[[reference, address, select]…]` in the specification's order |
| 24 | a solved motor output (`AET-WO-4300a`) | `label`, `timer`, `channel`, `pin`, `alternate_function`, `protocol`, `psc`, `arr`, `ccr`, `pwm?` (`[rate_hz, min_us, max_us]`) |
| 25 | a motor output's static check for silicon | `label`, `kernel_hz`, `period_ps`, `high_ps`, `nominal_period_ps`, `nominal_high_ps`, `holds` |
| 26 | a motor output's twin witness | `label`, `observer`, `period_ns`, `high_ns` (since trace 14, `AET-WO-4400g`, each ⌈`ARR`·(`PSC`+1) or `CCR`·(`PSC`+1) × 10⁹ / kind 25's `kernel_hz`⌉, which replay checks), `grain_ns`, `quantum_ns`, `status`, `rises`, `falls`, `first_ns?`, `last_ns?`, `end_ns`, `span_ns?`, `off_grid`, `batched`; since trace 13 (`AET-WO-2302`) `anchor_ns?` (its timer's first `CEN` rise in the boot; the window ends `DSHOT_WINDOW_NS` after it for DShot, at the boot's end for PWM), `edges_before`, `edges_after` and `cen_rises` (its timer's `CEN` rises: 0 exactly when no anchor; equal across a timer's outputs) |
| 27 | a solved serial port (`AET-WO-4300b`) | `label`, `index`, `peripheral`, `tx`, `tx_af`, `rx`, `rx_af`, `role`, `framing` `[baud, data_bits, parity, stop_bits, inverted]`, `kernel_hz`, `brr`, `actual_millibaud`, `error_ppm` |
| 28 | a port's static framing judgement | `label`; silicon's `kernel_hz`, `brr`, `actual_millibaud`, `error_ppm`, `tolerance?` `[table, m_bits, tolerance_ppm]`, `holds`; the twin's `[tag needed…]` and `tags?` `[writes, [missing…], [unexpected…]]` |
| 29 | a port's twin witness | `label`, `capture?` `[patterns, cut, echoed, stray, len]`, `frame` (8 bytes), `echo`, `rx_at_injection?`, `tx_at_end?`, `tx_at_send?` (each `[mode, af]`); since trace 13 `injected_ns?`, the first frame byte's instant, present exactly when `rx_at_injection` is |
| 30 | a judge's stop (`AET-WO-2302` D2) | `timer`, `anchor_ns`, `at_ns` (`anchor_ns` + `DSHOT_WINDOW_NS`), `[reading…]` (the timer's solved registers read back before `CEN` is cleared, 1 to 32: exactly kind 11's registers of the timer, in its order, each at its address, under its mask, against its expected value, `CR1`'s with `CEN` set where kind 11's is cleared by the stop), `cr1_after?` |

Kinds 5 to 9, 15, 16, 20, 23, 24, 25 and 27 are at virtual time 0, decided
before the boot (`Record::before_boot`); 17 to 19 at their own time
(`Record::timestamp`); the rest at the boot's end. Kind 15 is written from
the solved topology alone (`i2c_devices`, `assignments`, `analog_lines`),
never from the firmware or the specification; `ceiling_ps` is per line because
`analog_lines` holds each net's own rise-time ceiling. The bridge's nanoseconds become picoseconds in the CLI: a measured
rise rounds up, so a rise past its whole-nanosecond ceiling stays past it; a
lower bound rounds down; volts round to the nearest microvolt.

## Scenarios

A scenario is a strict JSON object (`campaign::Scenario::parse`), authored
apart from anything generated (`INV-07`):

```json
{"schema_version": 1, "name": "pullup-47k", "base": "../../examples/sentinel-01-minimal.json",
 "expect": "fail", "expect_findings": ["analog_rise_exceeds_ceiling", "i2c_timeout"],
 "overrides": [{"bus": "SENSORS", "pull_up_ohms": 47000}]}
```

- Every key is required (but `expect_counts`) and no other is allowed, at
  either level; a duplicate key, a positional array or `null` in a key an
  override does not use is refused. `schema_version` is 1 to 4, and a file
  may use only what its version admits (`SchemaViolation` otherwise): version
  2 (`AET-WO-2301`, `AET-WO-4200`) admits `droop`, `spi_device_absent` and
  `expect_counts`; version 3 (`AET-WO-2302` D5) also admits `serial_rx_open`;
  version 4 (`AET-WO-2304b`) also admits `battery_sag` (`{series_milliohms,
  start_ns}`, one per scenario and never beside a droop: `BatterySagRefused`)
  and `rail_out_of_band` (a domain, one per scenario, beside exactly the
  `serial_rx_open` and `spi_device_absent` overrides it renders and nothing
  else: `RailOutOfBandRefused`).
  A version-1 file reads as it always did. `name` is 1 to 64 bytes of
  `0x21`–`0x7E`.
- An override is exactly `{bus, pull_up_ohms}` (a `u32`),
  `{net, max_capacitance_femtofarads}` (a `u64`), or `{bus, line_held_low}`
  (`"sda"` or `"scl"`); in version 2 or 3, `{droop: {domain, floor_millivolts,
  start_ns, fall_ns, hold_ns, rise_ns, source_milliohms, load_milliamps,
  bor_level}}`, every field required and bounded, or `{spi_device_absent:
  reference}`; in version 3, `{serial_rx_open: port}`, a serial port's label.
  Every id is 1 to 64 bytes of `0x21`–`0x7E`. Two overrides of one thing
  (one bus's pull-up, one net, one line, one device, one port), or two
  droops, are refused (`DuplicateOverride`); at most 32 overrides and 16
  expected kinds, so the scenario's record fits the event bound.
- `expect_counts` (version 2 or 3) is an object of finding kinds to counts,
  at most 16, a kind named twice refused.
- `expect` is `pass` or `fail`. A failing scenario names the finding kinds it
  fails with in `expect_findings`; a passing one names none
  (`ExpectContradiction`).
- The runner (`aethelgard campaign`) sets the bus's `pull_up_ohms` or the net constraint's
  `max_capacitance_femtofarads` in the specification, refusing an id it does
  not have (`UnknownBus`, `UnknownNet`), then validates the result: an
  override that leaves the specification invalid is refused with
  `aethelgard-ir`'s own kind. `line_held_low` changes nothing in the
  specification; it returns the held line, which the analog pre-pass faults
  without measuring (its finding is `i2c_line_held_low`, its fault's cause
  `scenario.line_held_low`). `spi_device_absent` and `serial_rx_open` change
  nothing in the specification either: the platform leaves the device out
  behind its chip select, and the composition injects no frame into the port
  (`UnknownDevice`, `UnknownSerialPort` for an id the solved topology lacks).
  Replay binds an open port to its records: it names a solved port (kind 27),
  its witness (kind 29) shows no frame gone in and no RX mux read at an
  injection, and the sequential injection skips it.

**A droop** (`AET-WO-2301`, ruled on PR #107) changes nothing in the
specification. The runner resolves it to the MCU's supply (`DroopNotMcuSupply`
otherwise), that domain's nominal and tolerance and the emitter's decoupling,
and `campaign::check_droop` refuses (`DroopRefused`) a rail floor or resting
level inside an active threshold's band or hysteresis region, a floor held
less than 10 R·C, and a droop whose effects run past the boot. The table and
the rule that turns crossings into resets are `power` (DS12117 Rev 7, Table 26,
provisional).

**A run matches its scenario** when its verdict is the one expected and, for
a failure, the set of finding kinds above `info` equals `expect_findings`
exactly: a scenario that fails for another reason, or for more reasons, does
not match (`AET-WO-2300` §5.4). With `expect_counts`, each named kind's count
(any severity, 0 if absent) must also be exactly the one declared.

**`run_id`** is the first 16 bytes of
`sha256("aethelgard-run\0" ‖ scenario sha256 ‖ specification sha256 ‖ firmware sha256)`,
so the same campaign twice names the same runs, and a second campaign into one
archive is refused as existing: the archive is append-only.

## The run rules

`campaign::check_archived` is what `aethelgard replay` judges an archived
campaign run by, after the strict reader. It refuses a run holding any event
but campaign records, and a header whose `artifacts` are not exactly `base`,
`firmware`, `scenario`, `specification` and `topology`, whose scenario,
specification and topology hashes are not the records', or whose `run_id` is
not the one its scenario, specification and firmware give, or whose `tools`
are not exactly the ones that ran: `aethelgard` and `renode` always,
`aethelgard-bridge` when kinds 7, 8 or 16 exist, and `ngspice` when kinds 7, 8
or 17 exist (`campaign::tools_of`). Then
`campaign::check_run`, which the campaign also applies before it writes a run,
refuses:

- records out of the order: the scenario, its expected counts (20), the
  topology, the solved buses (kind 15), the analog records (7 and 8), the bus
  faults (9), the declared rail (16), the droop's crossings, resets and
  observations (17 to 19) in time order, the boot (10), the registers (11),
  the identity reads (12), the witnesses (13) and the verdict; or a run
  without exactly one scenario, topology, boot and verdict;
- a droop's records that do not follow from it (`AET-WO-2301`): kinds 16 to 19
  without a droop override, or kind 16 not equal to it; a threshold's level
  not crossed exactly once falling and once rising, within 10 R·C of the
  source's ramps, when the declared rail floor is below it, or crossed when it
  is not; resets that are not `power::schedule` of the crossings with their
  fixed renderings; not one observation per release at the recovery bound (a
  boot killed before its reads has none); an observation whose readings are
  not the boot's own; `brownout_reset` (info), `brownout_unrecovered` (error)
  or `rail_below_tolerance` (info) counts that the records do not give;
- a broken join (`AET-WO-3100` R2, extended by its adversarial pass): solved buses not sorted by unique bus; two solved buses sharing a net, a pin or a controller; two devices at one address on one bus; a second kind 7 or 8 for one line, a second kind 12 for one device, or a second kind 13 for one device or one controller; a
  kind 7, 8 or 9 on a bus no kind 15 solves, or on a net that is not one of
  that bus's lines; a kind 7 or 8 whose `ceiling_ps` is not its line's; a
  device on two solved buses; an identity read whose reference is no solved
  device at its address; a witness whose reference is no solved device, or
  whose `peripheral` is not that device's bus's;
- a bus fault naming a net with no measurement past its ceiling (or stuck
  high), a measurement past its ceiling that faulted no bus, a held-low fault
  without its override of that line, or an override without its fault;
- a line of a solved bus not bound by exactly one record: its kind 7, its
  kind 8, or, for a line a scenario holds low (never measured), its held-low
  kind 9 (`AET-WO-3201` B1);
- a verdict whose count of `analog_rise_exceeds_ceiling`,
  `analog_line_stuck_high` or `i2c_line_held_low` findings differs from those
  records (so a passing verdict beside a bus fault, too);
- a verdict that names `pc_outside_firmware` when kind 10's `pc` is inside its
  executable segments, or does not when it is outside them (`AET-WO-2205`
  §1.5). The segments are the judge's; replay cannot bind them to the ELF,
  which is not archived (`RUL-19` (3));
- a verdict whose `boot_word_missing` is not exactly one error when kind 10
  was read without its boot word, and none otherwise; a boot that read
  nothing whose verdict does not name why once, as an error (killed at a
  bound: `cpu_aborted`, `memory_cap_exceeded` or `wall_bound_exceeded`; or not
  loaded: `platform_load_error` or `firmware_load_error`), or a read boot that
  names one (`AET-WO-3201` B2);
- a verdict that judges an identity read's record (a mismatch of any step,
  `*_record_invalid`, `*_transaction_unwitnessed`) of a boot without its boot
  word: the judge names a record before the fixed point only by its timeout
  (`AET-WO-3201`, adversarial F2);
- a verdict whose anchors' or judge's stop's findings (`AET-WO-2302`) the
  records do not give, once the boot word is set, and any before it:
  `motor_anchor_unobserved` once per kind 26 `unanchored`;
  `motor_window_past_run` once per DShot timer (kind 24) whose kind 26 anchor
  plus `DSHOT_WINDOW_NS` passes the boot, and `judge_stop_missing` once per
  one inside it with no kind 30 at that anchor that left `CEN` cleared;
  `judge_stop_mismatch` once per register such a stop read back off its
  solved value, and once more for registers it did not read (adversarial
  pass 1, L3), the read-back itself exactly the timer's registers kind 11
  lists, in its order, each at kind 11's address, under its mask, against its
  expected value, `CR1`'s with `CEN` set where kind 11's, after the stop, has
  it cleared (pass 2, A2-1); `motor_edge_before_anchor` and
  `motor_edge_after_window` once per output, and `motor_anchor_repeated` once
  per timer, from kind 26's `edges_before`, `edges_after` and `cen_rises`;
- a verdict whose `serial_wire_past_run` or `motor_anchor_out_of_order` the
  boot word's instant, as the records place it (one quantum before the first
  port the chain injects, kind 29's `injected_ns`), does not give (pass 2,
  A2-5): `serial_wire_past_run` exactly, once when the ports' frames and
  echoes (kind 27's framings, over the ports not held open) end past the boot
  from there, else never; `motor_anchor_out_of_order` at least once per timer
  anchored after it. Neither before the boot word; `serial_wire_past_run`
  never with no injected port and at most once, `motor_anchor_out_of_order`
  at most once per anchored timer. Unbound, since no record carries what
  decides them: `motor_anchor_out_of_order`'s other cases (the anchor against
  the last device step's witness, or no boot word seen by its hook),
  `serial_wire_past_run` when the first port took no frame, and
  `serial_injection_out_of_order` (the logged `TDR` bytes);
- a motor witness `stopped` whose edges are not continuous on the model's
  sequence (`campaign::witness_continuous`: its last rise `rises - 1`
  periods after its first within its span, its falls as many as its rises
  or one fewer), which the judge never gives (`AET-WO-2302b`, ADV-M1);
- a first injected port (kind 29's `injected_ns`, the first port not held
  open) less than one quantum after the boot's start: the judge injects it
  one quantum after the boot word, which would then precede the run (ADV-S4).

So a run cannot replay as passing, or without its analog evidence, unless its
manifest is re-sealed and its records rewritten consistently: which is the
forgery the manifest's integrity (not authenticity) already admits.
