# AET-WO-0400 — The trace archive: deterministic CBOR, a local append-only store, and a replay that proves itself

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0400-trace-archive`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6, spend by tier) in tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation) · **Phase exit gate:** `docs/roadmap.md` §5 P0. This work order owns two clauses:
- **Clause 5:** a synthetic 10,000-event trace written to the local archive replays to a state sequence whose hash equals the original's. This is `INV-04`'s falsification, run as a test.
- **The remote-mirror half of clause 6:** with the mirror's credentials unset, the archive step logs one error naming the missing variable, and the run still produces its verdict.

With them, every P0 clause is owned by merged work.

**Rewritten 2026-10-29 as build-only.** The first version, written 2026-10-21 and held for budget, opened with a spike on how a Cap'n Proto schema becomes Rust. `architecture.md` `RUL-06` now answers that question: deterministic CBOR, with an encoder and a strict decoder written here. Everything the spike would have chosen is fixed below, so the builder makes no format decisions.

**Grounds:**
- `docs/northstar.md` §4: `INV-04` (every simulated run replays from its own trace), `INV-06` and `INV-03`.
- `docs/architecture.md`:
  - §4 `RUL-06` as amended 2026-10-29: the CBOR profile, local-first;
  - §3: `aethelgard-chaos` owns the trace archive;
  - §5 and §6: secrets by environment-variable name only, `AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID` and `AETHELGARD_R2_SECRET_ACCESS_KEY`;
  - §8 `OQ-03`;
  - §2.1: timestamps are at the simulator's own resolution, never "picoseconds".
- `docs/roadmap.md` §6: graduating a stub crate is stated, not implied.
- The code seams:
  - `crates/aethelgard-chaos/src/lib.rs`, a stub that re-exports `aethelgard-ir`;
  - `crates/aethelgard-cli/tests/workspace_shape.rs`, whose stub checks the crate graduates from (§1.6).

> **What this is:** the trace format; a local, append-only archive that a run writes as it goes; and a replay that refuses any trace it cannot prove complete. A seeded synthetic event stream exercises all three. The archive also gets the optional remote mirror's *credential boundary*: absence is detected, named and logged, and the run continues.
>
> It is NOT:
> - the R2 upload, which waits for `OQ-03`'s measured volume (`RUL-06`);
> - real simulator events, which arrive in `AET-WO-2100`;
> - the fault-campaign runner (`AET-WO-2300`);
> - a CLI subcommand.
>
> **Why now:** `INV-04` is the invariant everything after P1 rests on. The format has to exist, and be proven deterministic and tamper-evident, before the first real event is written into it.

**Cadence:** build-only. There is no spike and no STOP for a ruling, except the flag-and-stop conditions in §7. One PR, left unmerged. Nothing here needs the tools image or a credential.

## 1. Scope

### 1.1 The CBOR profile (`RUL-06`)

Write the encoder and the strict decoder in `aethelgard-chaos` itself, with no CBOR crate. The profile is RFC 8949's core deterministic encoding (§4.2.1), restricted further:

- **Admitted:** major type 0 (unsigned integer), 1 (negative integer), 2 (byte string), 3 (text string, which must be valid UTF-8) and 4 (array).
- **Refused:**
  - major types 5 (map), 6 (tag) and 7 (floats and simple values, booleans included). A boolean is the unsigned integer 0 or 1;
  - indefinite lengths (additional information 31);
  - the reserved values 28 to 30;
  - any argument not in its shortest form.
- **Bounds, checked before anything is allocated:**
  - an array has at most 65,536 elements;
  - a string has at most 1 MiB;
  - nesting is at most 8 deep.
- **Each file is consumed exactly.** `manifest.cbor` and `mirror.cbor` are one item each, with nothing after it, and `trace.cbor` ends exactly at the end of its last event. Trailing bytes, padding included, are refused.
- Every refusal is a typed error carrying the byte offset.
- **The defining property:** for every byte string the decoder accepts, encoding the decoded value gives back the same bytes. Test it directly.

### 1.2 The trace format

A run directory holds exactly these entries:

- **`trace.cbor`,** a CBOR sequence (RFC 8742): one header item, then one item per event, with no enclosing array, so that it can be appended to.
  - **Header:** `["aethelgard-trace", 1, run_id, time_unit, artifacts]`.
    - `run_id` is a 16-byte byte string the caller supplies.
    - `time_unit` is the text `"ns"`, the only unit version 1 admits.
    - `artifacts` is an array of `[name, sha256]` pairs, sorted by name, with names unique. Each `sha256` is a 32-byte byte string. These are the hashes of what the run judged (`INV-03`).
  - **Events:** `[kind, timestamp, …payload]`. `timestamp` is an unsigned integer in `time_unit`. Timestamps never decrease; equal timestamps are allowed, and file order then decides. File order is the order the simulator produced, which is causal: a register write and the pin change it causes can share a nanosecond. So events are never re-sorted by any other key. Keeping that order deterministic is the writer's obligation, and the golden test (§1.7) proves it.

    | kind | name | payload |
    |---|---|---|
    | 0 | register write | `address` (at most `u32::MAX`), `value` (at most `u32::MAX`) |
    | 1 | pin level | `pin` (text), `level` (0 or 1) |
    | 2 | interrupt | `line` (at most 1023) |
    | 3 | analog sample | `net` (text), `microvolts` (any `i64`) |
    | 4 | fault injected | `scenario` (text), `detail` (text) |

    Pin, net and scenario names are 1 to 64 bytes of printable ASCII `0x21`–`0x7E`, the set `RUL-14` rules. `detail` is 0 to 1,024 bytes of printable ASCII with space, `0x20`–`0x7E`, so no text in a trace has two Unicode spellings. An unknown kind, or a payload of the wrong length or type, is refused, never skipped.
- **`manifest.cbor`:** `["aethelgard-manifest", 1, trace_bytes, trace_sha256, event_count, state_hash]`. It is written last, and is what makes the run committed.
- **`mirror.cbor`, optional:** `["aethelgard-mirror", 1, uploaded, reason]`. It is written after the commit (§1.5), and replay never reads it.

### 1.3 The commit protocol

**The writer:**
1. Create the run directory. Refuse if it already exists.
2. Create `trace.cbor` with `create_new`, and append the header, then each event as the run produces it, through a buffered writer.
3. To finish, flush, then `sync_all` the trace file.
4. Write `manifest.cbor.tmp` with `create_new`, `sync_all` it, rename it to `manifest.cbor`, then `sync_all` the directory. On Linux, opening the directory as a `File` and calling `sync_all` needs no `unsafe`.

**The reader** checks in this order. Each refusal is its own variant of one error type that names the problem (`INV-06`):

| Check | Refusal |
|---|---|
| The run directory is a real directory and not a symlink. Its entries are exactly `trace.cbor` and `manifest.cbor`, with optionally `mirror.cbor`, and each is a regular file, not a symlink | `UnexpectedEntry` for anything else, including a leftover `manifest.cbor.tmp` |
| `manifest.cbor` is present | `Uncommitted` |
| The manifest decodes under the profile, and its version is 1 | `MalformedManifest` or `UnknownVersion` |
| The trace's length matches `trace_bytes` | `Truncated`, or `Extended` for a trace that is too long |
| The trace's sha256 matches `trace_sha256` | `HashMismatch` |
| The header and events decode | `NotDeterministicCbor` (with its offset), `UnknownVersion`, `UnknownEventKind`, `MalformedEvent` or `TimestampRegressed` |
| The number of events matches `event_count` | `EventCountMismatch` |
| The replayed state hash matches `state_hash` | `ReplayDiverged` |

### 1.4 Replay

Replay is a deterministic fold of the events into a state:
- registers, a `BTreeMap<u32, u32>`;
- pin levels, a `BTreeMap<String, u8>`;
- interrupt counts, a `BTreeMap<u32, u64>`;
- the last sample on each net, a `BTreeMap<String, i64>`;
- the number of faults injected.

The state-sequence hash is a chain:
- `h0` is the sha256 of the header item's bytes;
- for each event `i`, `h_i` is `sha256(h_{i-1} ‖ encode(state after event i))`, where `encode` is the §1.1 profile with every map written as an array of `[key, value]` pairs in key order.

The final `h` is `state_hash`. Hashing the whole state after every event is deliberate: it makes any reordering that changes an intermediate state visible. If P2 measures it as too costly, changing it is a ruling, not a build step.

### 1.5 The mirror boundary

The mirror step runs after the local commit. It never runs before the commit, and never in place of it.

- **The environment is a parameter.** It takes the environment as a lookup the caller passes in, `impl Fn(&str) -> Option<OsString>`. Production passes `std::env::var_os`. Tests pass a closure, because `std::env::set_var` is `unsafe` in edition 2024.
- **It reads the three variable names in order:** `AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID`, `AETHELGARD_R2_SECRET_ACCESS_KEY`.
- **If any is unset:**
  - it writes exactly one line to a log sink the caller passes in (an `impl Write`), naming the first missing variable;
  - it writes `mirror.cbor` with `uploaded` 0 and a reason naming that variable.
- **If all are set,** it writes `uploaded` 0 with the reason `"remote mirror not built (RUL-06, OQ-03)"`. It never claims an upload that did not happen.
- **Either way:**
  - `mirror.cbor` goes through the same temp, `sync_all`, rename and directory-sync steps as the manifest;
  - the commit's result is unchanged;
  - no variable's *value* is ever read into the log, the trace, the manifest or `mirror.cbor`.

### 1.6 Graduating `aethelgard-chaos` from the stub checks

In `crates/aethelgard-cli/tests/workspace_shape.rs`:
- `aethelgard-chaos` leaves `library_crates_declare_nothing_and_only_re_export` and `library_crates_are_only_a_manifest_and_lib_rs`;
- its dependencies leave `LIBRARY_DEPENDENCIES`;
- the other four library crates stay under all three checks, unchanged.

One new test replaces them for `chaos`:
- **Files:** `Cargo.toml`, `README.md`, `src/**/*.rs` and `tests/**/*.rs`, and nothing else.
- **Manifest keys:** within `package`, `dependencies` and `lints`.
- **Dependencies:** exactly `sha2` and `thiserror`, from the workspace table. Drop the stub's `aethelgard-ir` re-export; nothing here uses the IR.

Every member still forbids `unsafe` and has no build script. Those tests are untouched.

### 1.7 The synthetic stream and the golden hashes

- **The generator:** a seeded SplitMix64, written in the crate in about ten lines, with no `rand` dependency. With seed `0x0400` it produces 10,000 events covering all five kinds.
  - Timestamps advance by 0 to 1,000 per event.
  - Register addresses are drawn from 16 values, so writes overwrite each other.
- **The golden test:** it pins two constants checked into the test:
  - the sha256 of the resulting `trace.cbor`;
  - its `state_hash`.

  Because CI's `gates` and Furnace both run it, passing on both is the cross-machine determinism proof. A change to either constant requires bumping the format version, and FEEDBACK says why.

### 1.8 Documentation

Add `crates/aethelgard-chaos/README.md` covering:
- the profile;
- the run-directory layout and each item's shape;
- the version rule (an unknown version is refused, and a format change bumps it);
- the commit protocol;
- the refusals table;
- the mirror boundary.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-04`.** Proven as a test: 10,000 events, replayed twice, equal state hashes. One altered byte makes replay refuse; it never yields a different hash quietly.
- **`INV-06`.** A missing manifest, a truncation or an unknown version halts replay with a named error. A missing mirror credential halts *the mirror step only*: it is logged once and named, and the run continues (`RUL-06`).
- **`INV-03`.** The header carries the hashes of what the run judged.
- **Secrets by name only, and none created.** No value, no `.env` file, and no placeholder value, anywhere.
- **No `unsafe` in any workspace crate.**
- **Protected surfaces diff to empty:**
  - the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `crates/aethelgard-ir`;
  - `crates/aethelgard-cli`, except `tests/workspace_shape.rs` as §1.6 allows;
  - `container/`, `.github/workflows/`.
- **One PR, left unmerged.** Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. **The profile:**
   - for every refused form in §1.1, a test that it is refused at the right offset;
   - the round-trip property over every value the generator emits, and over a hand-built set of edge cases: `0`, `23`, `24`, `255`, `256`, `u64::MAX`, `i64::MIN`, the empty string and the empty array.
2. **The P0 gate's clause 5:** the golden test passes in `gates` and on Furnace, and replaying the archive twice gives the same `state_hash`.
3. **Tampering is refused.** Each of these refuses with its own §1.3 variant:
   - a one-byte change in the trace;
   - a truncation at every event boundary, and at one point inside an event;
   - an appended byte;
   - a missing manifest;
   - two events swapped, with the manifest recomputed for the new bytes. This one must give `ReplayDiverged` or `TimestampRegressed`, never a pass.
4. **Kill mid-write.** The test re-executes its own test binary as the writer, with an environment variable naming the role. It reads progress from the child's stdout, and stops it with `Child::kill` (`SIGKILL`) at four points:
   - mid-event;
   - after the trace's `sync_all`;
   - after the manifest's temporary file is written;
   - after the rename.

   The reader refuses the first three. The fourth is committed and replays.
5. **The P0 gate's clause 6, mirror half:**
   - with all three variables unset, exactly one log line names `AETHELGARD_R2_BUCKET`, and `mirror.cbor` says why nothing was uploaded;
   - with each variable unset in turn, the line names that one;
   - with all set to sentinel values, no sentinel appears in any file or in the log;
   - in every case, the commit's result is unchanged.
6. **The workspace gates pass:** fmt, clippy `-D warnings`, test, and the new §1.6 check. CI is green on the first push, with `gates` and `tools` both required. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- R2 upload, an S3 client, any network access, any credential or bucket.
- Real Renode or ngspice events, the lock-step bridge, the fault orchestrator.
- A CLI subcommand that records or replays.
- Compression, retention, sampling or garbage collection. Sampling touches `INV-04` and is Scotty's ruling under `OQ-03`.
- Authenticity. The manifest proves integrity, that the bytes are the ones committed. It does not prove who committed them. Signing is a later ruling.
- A second writer. The archive has one writer, the Rust orchestrator (`RUL-06`). Renode's hooks will send it events over the bridge.

## 5. Adversarial pass — targeted, at the integrity tier

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Forge a trace** that replays to a chosen state while passing every check: rewrite the events and recompute the manifest. Record plainly that this succeeds, because the manifest proves integrity, not authenticity. Confirm it is the only way through.
2. **Hostile archives.** Each must refuse, bounded in memory and time:
   - a declared length of `u64::MAX`;
   - nesting nine deep;
   - a timestamp of `u64::MAX` followed by one more event;
   - version 2;
   - a second manifest under another name;
   - a symlinked `trace.cbor`;
   - a run directory that is itself a symlink.
3. **Leak a secret:** the sentinel test in §3.5, plus a `grep` of the whole run directory.
4. **Look for assumptions that hold by luck:**
   - directory listing order;
   - `sync_all` on Furnace's filesystem compared with the runner's;
   - the generator never producing equal timestamps;
   - the fold being too simple to see a swap of two writes to different registers.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0400.FEEDBACK.md`, same schema as before: `finding · where · type · recommendation · decision-needed`. It must carry:
- what did not work, and why;
- the gates line with CI run URLs;
- what was deliberately not built;
- encoded bytes per event for each kind, and the write throughput on Furnace for the 10,000-event stream, **labelled as plumbing, not `OQ-03`**;
- a promote-or-needs-none pass over `AET-WO-0300a`'s and, if it has merged, `AET-WO-0300b`'s FEEDBACK.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A design that replays a trace it cannot prove complete, or produces a different hash for the same trace without refusing.
- A golden constant that differs between `gates` and Furnace.
- Anything that needs a credential, a bucket, a network call, a new dependency beyond `sha2` and `thiserror`, or a change to repository settings.
- `unsafe` in a workspace crate.
- Sampling, dropping or summarising events. That changes `INV-04`, and it is a ruling.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo` or a system package on Furnace.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-0400 build: §1 is built and every §3 item passes in gates and on Furnace, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-0400-trace-archive.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base and anything that changed since this file was written.

**A rule for the architect** (`docs/roadmap.md` §6): any ruling that adds a stop condition mid-run comes with a replacement goal.
