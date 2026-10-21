# AET-WO-0400 — The trace archive: a schema, a local append-only store, and a replay that proves itself

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0400-trace-archive`.
**Author:** Claude (architect) · **Builder:** Claude Code session, tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation) · **Phase exit gate:** `docs/roadmap.md` §5 P0. This work order owns two clauses:
- **Clause 5:** a synthetic 10,000-event trace written to the local archive replays to a state sequence whose hash equals the original's. This is `INV-04`'s falsification, run as a test.
- **The remote-mirror half of clause 6:** with the mirror's credentials unset, the archive step logs one error naming the missing variable, and the run still produces its verdict.

With them, every P0 clause is owned by merged work.

**Grounds:**
- `docs/northstar.md` §4: `INV-04` (every simulated run replays from its own trace), `INV-06` and `INV-03`.
- `docs/architecture.md` §4 `RUL-06`: a Cap'n Proto trace schema, local-first, with the archive of record on disk and R2 only an optional mirror.
- `docs/architecture.md` §3: `aethelgard-chaos` owns the trace archive.
- `docs/architecture.md` §5 and §6: secrets by environment-variable name only, `AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID` and `AETHELGARD_R2_SECRET_ACCESS_KEY`. If they are absent, the mirror is skipped with a logged error.
- `docs/architecture.md` §8 `OQ-03`.
- `docs/architecture.md` §2.1: trace timestamps are at the simulator's own time resolution, never "picoseconds".
- `RUL-12`: Rust, with Python only inside Renode.
- The code seams:
  - `crates/aethelgard-chaos/src/lib.rs`, an empty library today;
  - the workspace dependency rules pinned by `crates/aethelgard-cli/tests/workspace_shape.rs`, which limit the library crates' dependencies and will need a ruled extension.

> **What this is:** the trace schema; a local, append-only archive that a run writes as it goes; and a deterministic replay reader that refuses any trace it cannot prove complete. A synthetic event stream exercises all three. The archive also gets the optional remote mirror's *credential boundary*: absence is detected, named and logged, and the run continues.
>
> It is NOT:
> - the R2 upload itself, which waits for `OQ-03`'s measured volume to justify it (`RUL-06`);
> - real simulator events, which arrive in `AET-WO-2100`;
> - the fault-campaign runner (`AET-WO-2300`);
> - attribution (`AET-WO-3100`);
> - a CLI subcommand.
>
> **Why now:** `INV-04` is the invariant everything after P1 rests on. A campaign verdict is only as good as the trace it can be replayed from, and the format has to exist, and be proven deterministic and tamper-evident, before the first real event is written into it.

**Cadence:** spike first, then STOP for the architect's ruling, then build. One PR for both halves, left unmerged.

**Constraints:**
- Furnace can build and test all of this. Nothing here needs the tools image, unless the spike chooses to put a schema compiler in it.
- No credentials exist anywhere: not on Furnace, not in CI, and not in this work order.

## 0. Spike first — the questions the spike must answer

Record every answer in `docs/work-orders/AET-WO-0400.FEEDBACK.md` under "Spike". Cite the Furnace command or CI run that measured it. Then STOP.

- **S1 How the Cap'n Proto schema becomes Rust.**
  - The `capnp` crate's code generator, `capnpc`, needs the `capnp` schema compiler, a C++ executable. It is in neither the tools image nor Furnace, and the `gates` job builds the workspace without the image.
  - Measure each option on the same small schema, and recommend one:
    - **(a)** Generated Rust checked in, plus a check that regenerates it inside the image and diffs it, which needs `capnproto` pinned in the image;
    - **(b)** `capnp` installed wherever the workspace builds;
    - **(c)** keep the schema but generate without the C++ compiler, if a maintained pure-Rust route exists. Name it and its maintenance state;
    - **(d)** a pure-Rust format instead, which would amend `RUL-06`.
  - Record:
    - the build-time cost of each;
    - each one's new dependencies and licences;
    - whether each keeps `RUL-12`'s rule that no C++ is written here;
    - whether a Python writer could later emit the same format from inside Renode, which is the reason Cap'n Proto was chosen.
- **S2 Determinism of the encoding.**
  - Is the encoded byte stream for the same event sequence identical across two runs, two processes, and Furnace against CI? Check canonical encoding, padding, and any map ordering.
  - If it is not byte-stable, what *is* stable enough to hash? `INV-04` needs a hash that cannot drift for reasons unrelated to the run.
- **S3 What "complete" means on disk.**
  - Measure what a reader sees after the writer is killed, with `SIGKILL`, mid-segment, mid-event and mid-commit.
  - Propose the commit protocol: a manifest written last carrying the content hash, `fsync` of file and directory, or an equivalent. The rule is that a partial trace is detected and refused, never replayed as if whole.
- **S4 The cost per event.** For a synthetic stream of the event kinds P2 will plausibly need, measure encoded bytes per event and write throughput on Furnace. The kinds are a register write, a pin level change, an interrupt, an analog sample and a fault injection.

  This is a plumbing measurement. It is **not** `OQ-03`'s bytes per simulated second, which needs real event rates from `AET-WO-2100`. Record it labelled as exactly what it is.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. A question that cannot be measured is answered when the reason is recorded. The architect rules by a comment on the PR and a new goal.

## 1. Scope — the build half, after the ruling

1. **The schema,** in the format S1's ruling chose.
   - A trace is a header, then events.
     - The header carries the schema version, the run id, and the content hashes of the artifacts the run judged. Those hashes are what lets `INV-03` later say "on those hashes".
     - Each event carries a monotonic timestamp in the simulator's own unit, which is named in the header, plus a typed payload.
   - Unknown event kinds are refused on read, never skipped.
2. **The archive, in `aethelgard-chaos`.** It is append-only, written as the run goes, using S3's commit protocol.
   - A run directory holds the trace and a manifest that is written last.
   - The reader refuses a trace with no manifest, a hash mismatch, a truncation, a non-monotonic timestamp, or an unknown schema version. Each refusal is a typed error naming the problem (`INV-06`).
3. **Replay,** as a deterministic fold of events into a state, plus the hash of the state sequence.
   - The P0 state is synthetic: a register file and pin levels, enough to make the fold non-trivial. P2 swaps in the twin's state.
   - Replaying the same archive twice gives the same state-sequence hash.
4. **The mirror boundary.**
   - After the local commit, the archive step reads the three variable names in §2.
   - If any is unset, it logs exactly one error naming the first missing variable, and records `mirror: {uploaded: false, reason}` in the manifest.
   - If all are set, it records `uploaded: false, reason: "remote mirror not built (RUL-06, OQ-03)"`. It never claims an upload that did not happen.
   - Either way, the run's result is unaffected. No upload code, no S3 client and no network access are in this work order.
5. **Workspace rules.** Extend `workspace_shape.rs`'s allowed library dependencies by exactly what S1 rules, and nothing else.
6. **Documentation.** A `crates/aethelgard-chaos/README.md` covering:
   - the schema and its version rule;
   - the on-disk layout;
   - the commit protocol;
   - the refusals;
   - the mirror boundary.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-04` Every simulated run replays from its own trace.** Prove it as a test, not by assertion: write 10,000 synthetic events, replay twice, and compare state-sequence hashes. Then alter one byte of the archive: replay must refuse, not produce a different hash quietly.
- **`INV-06` Absence halts.** A missing manifest, a truncated segment or an unknown version halts replay with a named error. A missing mirror credential halts the *mirror step only*, logged once and named, and the run continues (`RUL-06`; `architecture.md` §5).
- **`INV-03`.** The trace header carries the hashes of what the run judged.
- **Secrets by name only, and none created.** The three variable names appear in code and docs. No value, no `.env` file, no placeholder value, anywhere.
- **No `unsafe` in any workspace crate.**
- **Protected surfaces diff to empty:**
  - the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `crates/aethelgard-ir`, and `crates/aethelgard-cli`, except `tests/workspace_shape.rs` as §1.5 allows;
  - `container/`, unless S1's ruling adds `capnproto` to the image;
  - `.github/workflows/tools-image.yml`.
- **One PR, left unmerged.** Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. Every §0 answer is recorded in FEEDBACK with its Furnace command or CI run.
2. **The P0 gate's clause 5:** 10,000 synthetic events replay twice to equal state-sequence hashes, in `gates` and on Furnace.
3. **Tampering is refused:** a single-byte change, a truncation at every segment boundary, a missing manifest, and a reordered event each refuse with their own named error.
4. **Kill mid-write:** the reader refuses every partial trace S3 produced. No partial trace replays.
5. **The P0 gate's clause 6, mirror half:** with the three variables unset, exactly one error naming `AETHELGARD_R2_BUCKET` is logged, the manifest says why nothing was uploaded, and the run's result is unchanged. The same holds with each variable unset in turn.
6. CI is green on the first push after the ruling, with `gates` and `tools` both required. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- R2 upload, an S3 client, any network access, any credential or bucket.
- Real Renode or ngspice events, the lock-step bridge, the fault orchestrator.
- A CLI subcommand that records or replays. `AET-WO-2300` decides how a campaign drives the archive.
- Compression, retention, sampling or garbage collection. Sampling touches `INV-04` and is Scotty's ruling under `OQ-03`.
- Python writers inside Renode, except as S1 asks whether one would be possible.

## 5. Adversarial pass — break it before calling it done

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Forge a trace** that replays to a *chosen* state while passing every check: rewrite events and recompute the manifest. Record honestly what the manifest's hash does and does not prove. It proves integrity, not authenticity, and say so if that is the case.
2. **Interrupt every write step,** including `fsync`, the manifest rename and the directory sync, and confirm each partial state is refused.
3. **Feed hostile archives:**
   - an enormous declared length;
   - nesting depth that tries to exhaust the reader;
   - a timestamp overflow;
   - a schema version from the future;
   - two manifests;
   - a symlinked segment.

   Each must refuse, bounded in memory and time.
4. **Leak a secret.** Set the mirror variables to sentinel values and grep the manifest, the logs and the trace for those values. None may appear.
5. **Look for assumptions that hold by luck:**
   - filesystem ordering;
   - `fsync` behaviour on Furnace's filesystem compared with the runner's;
   - locale or endianness in hashing;
   - the synthetic fold being too simple to catch a reordering.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0400.FEEDBACK.md`, same schema as before. It must carry:
- what did not work, and why;
- the gates line;
- what was deliberately not built;
- the measured facts for `architecture.md` §2;
- whether `RUL-06` needs amending (S1);
- the bytes-per-event numbers labelled as plumbing, not `OQ-03`;
- a promote-or-needs-none pass over AET-WO-0300a's FEEDBACK. Its findings were promoted at its close into §2.6, §6 and the roadmap (`AET-WO-1600`). Say so.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A design that replays a trace it cannot prove complete, or produces a different hash for the same trace without refusing.
- Anything that needs a credential, a bucket, a network call, or a change to repository or organization settings.
- A format choice that would require writing C++, or `unsafe` in a workspace crate.
- Sampling, dropping or summarising events. That changes `INV-04`, and it is a ruling, not a build step.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo` or a system package on Furnace.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force (AET-WO-0300a's achieved goal), and dismiss it.

**Step 2, in its own turn, set the goal for the spike half:**

```
/goal AET-WO-0400 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited to its Furnace command or CI run in FEEDBACK, on an open unmerged PR from wo/aet-0400-trace-archive, and the build half has not started.
```

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base and anything that changed since this file was written.

**A rule for the architect, learned twice in AET-WO-0200 and AET-WO-0300a:** any ruling that adds a stop condition mid-run comes with a replacement goal, so a correct stop is never held against a goal it can no longer meet.
