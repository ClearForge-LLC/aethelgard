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
  - Header: `["aethelgard-trace", 2, run_id, "ns", artifacts]`. `run_id` is 16
    bytes. `artifacts` is `[name, sha256]` pairs sorted by name, names unique,
    each hash 32 bytes: what the run judged (`INV-03`). An artifact name is 1 to
    1,024 bytes of printable ASCII, space allowed.
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
    | 5–15 | a campaign record | see "Campaign records" |

    Pin, net and scenario names are 1 to 64 bytes of `0x21`–`0x7E` (`RUL-14`).
    `detail` is 0 to 1,024 bytes of `0x20`–`0x7E`. An unknown kind, or a payload
    of the wrong length or type, is refused.
- **`manifest.cbor`**: `["aethelgard-manifest", 2, trace_bytes, trace_sha256,
  event_count, state_hash]`. Written last; it is what commits the run.
- **`mirror.cbor`**, optional: `["aethelgard-mirror", 1, uploaded, reason]`.
  Written after the commit; replay never reads it, so it is not covered by the
  manifest: its contents and size are unchecked.

## Version rule

The trace header carries version 3 (`AET-WO-2300` added kinds 5 to 14 at
version 2, and `AET-WO-3100` kind 15 and the run rules that join to it at
version 3; each time the golden trace's bytes changed in that one header byte
only). A version-2 trace is refused, not read under version 3's rules. The mirror record
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
| Manifest decodes under the profile | `MalformedManifest` |
| Manifest version is 2 | `UnknownVersion` |
| Manifest's `trace_bytes` is at most 256 MiB | `TraceTooLarge` |
| Trace length equals `trace_bytes` (checked before the file is read) | `Truncated` / `Extended` |
| Trace sha256 equals `trace_sha256` | `HashMismatch` |
| Header and events decode; a header item is at most 1 MiB and an event item at most 4,096 encoded bytes, checked before the item's nested values are built | `NotDeterministicCbor` (with offset), `MalformedHeader`, `UnknownVersion`, `UnknownEventKind`, `MalformedEvent`, `TimestampRegressed` |
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

Kinds 5 to 15 (`src/record.rs`) are the records `aethelgard campaign` archives
for one scenario's run, in the order of "The run rules" below: kind 15 comes
after kind 6, the rest by number. Every field is an unsigned or negative
integer, a byte string, a text string, or an array of them: no float, no map.
A flag is 0 or 1. An optional value is an array of zero or one element. A
`reading` is `[address, mask, expected, observed?]`, each a `u32`. The writer
refuses an event over 4,096 encoded bytes, as the reader does.

| kind | record | payload |
|---|---|---|
| 5 | scenario | `name`, `sha256` (32 bytes, of the file), `expect_pass`, `[finding kind…]` (sorted; empty exactly when `expect_pass`), `[override…]`: `[0, bus, ohms]`, `[1, net, femtofarads]` or `[2, bus, line]` (0 SDA, 1 SCL) |
| 6 | solved | `specification_sha256`, `topology_sha256` (32 bytes each) |
| 7 | `analog.rise_time_measured` | `bus`, `net`, `rise_30_70_ps`, `complete`, `ceiling_ps`, `exceeds` (which must equal `!complete \|\| rise > ceiling`) |
| 8 | `analog.line_stuck_high` | `bus`, `net`, `microvolts_at_release`, `ceiling_ps` |
| 9 | `bridge.fault_state` | `bus`, `fault`, `source` (1–128 bytes), `[cause…]`, `[net…]` (sorted), `firmware_sees` (0–1,024 bytes) |
| 10 | the boot | `image_sha256`, the boot word's `reading`, `instructions_expected`, `instructions_observed?`, `pc?` |
| 11 | a register | `name`, `reading` |
| 12 | an I2C identity read | `reference`, `address`, `register` (each ≤ 255), `expected`, `observed?`, `status?` (text) |
| 13 | an I2C controller witness | `peripheral`, `reference`, `cr2` and `txdr` `reading`s |
| 14 | the verdict | `pass`, `[[kind, severity, count]…]` (sorted, unique by kind and severity; `pass` exactly when every severity is `info`) |
| 15 | a solved I2C bus (`AET-WO-3100`) | `bus`, `peripheral`, `[line…]`: exactly two, SCL then SDA, each `[signal, net, pin, alternate_function, ceiling_ps]` (`signal` is the peripheral's name then `_SCL` or `_SDA`; the two nets and pins differ), `[[reference, address]…]` (sorted, unique by reference) |

Kinds 5 to 9 and 15 are at virtual time 0, decided before the boot
(`Record::before_boot`); 10 to 14 at the boot's end. Kind 15 is written from
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

- Every key is required and no other is allowed, at either level; a duplicate
  key, a positional array or `null` in a key an override does not use is
  refused. `schema_version` is 1. `name` is 1 to 64 bytes of `0x21`–`0x7E`.
- An override is exactly `{bus, pull_up_ohms}` (a `u32`),
  `{net, max_capacitance_femtofarads}` (a `u64`), or `{bus, line_held_low}`
  (`"sda"` or `"scl"`). Two overrides of one thing are refused
  (`DuplicateOverride`); at most 32 overrides and 16 expected kinds, so the
  scenario's record fits the event bound.
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
  `scenario.line_held_low`).

**A run matches its scenario** when its verdict is the one expected and, for
a failure, the set of finding kinds above `info` equals `expect_findings`
exactly: a scenario that fails for another reason, or for more reasons, does
not match (`AET-WO-2300` §5.4).

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
not the one its scenario, specification and firmware give. Then
`campaign::check_run`, which the campaign also applies before it writes a run,
refuses:

- records out of the order: the scenario, the topology, the solved buses
  (kind 15), the analog records (7 and 8), the bus faults (9), the boot (10),
  the registers (11), the identity reads (12), the witnesses (13) and the
  verdict; or a run without exactly one scenario, topology, boot and verdict;
- a broken join (`AET-WO-3100` R2): solved buses not sorted by unique bus; a
  kind 7, 8 or 9 on a bus no kind 15 solves, or on a net that is not one of
  that bus's lines; a kind 7 or 8 whose `ceiling_ps` is not its line's; a
  device on two solved buses; an identity read whose reference is no solved
  device at its address; a witness whose reference is no solved device, or
  whose `peripheral` is not that device's bus's;
- a bus fault naming a net with no measurement past its ceiling (or stuck
  high), a measurement past its ceiling that faulted no bus, a held-low fault
  without its override, or an override without its fault;
- a verdict whose count of `analog_rise_exceeds_ceiling`,
  `analog_line_stuck_high` or `i2c_line_held_low` findings differs from those
  records (so a passing verdict beside a bus fault, too).

So a run cannot replay as passing, or without its analog evidence, unless its
manifest is re-sealed and its records rewritten consistently: which is the
forgery the manifest's integrity (not authenticity) already admits.
