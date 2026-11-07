# aethelgard-chaos

Fault scenarios, the campaign runner, the trace archive, and verdicts
(`docs/architecture.md` §3). So far this crate holds the **trace archive**
(`AET-WO-0400`, `RUL-06`, `INV-04`): a deterministic CBOR format, a local
append-only store a run writes as it goes, and a replay that refuses any trace
it cannot prove complete. Scenarios, the runner and verdicts arrive later.

Only `sha2` and `thiserror` are used. There is no CBOR crate: the encoder and
the strict decoder are `src/cbor.rs`.

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
symlink or a leftover `manifest.cbor.tmp` included, is refused. Replay reads
neither mirror file (`mirror.cbor.tmp` is what a crash in the mirror step
leaves behind; it must not orphan a committed run).

- **`trace.cbor`**, a CBOR sequence (RFC 8742): the header, then one item per
  event, no enclosing array, so it can be appended to.
  - Header: `["aethelgard-trace", 1, run_id, "ns", artifacts]`. `run_id` is 16
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

    Pin, net and scenario names are 1 to 64 bytes of `0x21`–`0x7E` (`RUL-14`).
    `detail` is 0 to 1,024 bytes of `0x20`–`0x7E`. An unknown kind, or a payload
    of the wrong length or type, is refused.
- **`manifest.cbor`**: `["aethelgard-manifest", 2, trace_bytes, trace_sha256,
  event_count, state_hash]`. Written last; it is what commits the run.
- **`mirror.cbor`**, optional: `["aethelgard-mirror", 1, uploaded, reason]`.
  Written after the commit; replay never reads it, so it is not covered by the
  manifest: its contents and size are unchecked.

## Version rule

The trace header and the mirror record carry version 1; the manifest carries
version 2 (`AET-WO-0400a` changed what `state_hash` means, the trace's bytes
not at all). A reader refuses any other version (`UnknownVersion`), judged
before the item's shape. A change to the format, to the event table, or to the
golden constants in `tests/golden.rs` bumps it.

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

A write that leaves the state as it was still has its delta, so it changes
`state_hash`. Timestamps, and a fault's scenario and detail, are not in the
chain; `trace_sha256` covers them.

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
