# AET-WO-0400a — Harden replay before any trace from outside is read: a linear hash chain, bounded decoding, and a mirror crash that does not orphan a run

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0400a-replay-bounds`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6), in a fresh session on Furnace.
**Phase:** P0, an inserted refinement of `AET-WO-0400` after the P0 gate passed. It must merge before any work order replays a trace this project did not write, and before `AET-WO-2300`'s first campaign.

**Grounds:**
- `docs/work-orders/AET-WO-0400.FEEDBACK.md`, adversarial findings 1, 2, 3 and 6, ruled at review of PR #21. The rulings are below.
- `docs/northstar.md` §4: `INV-04` and `INV-06`.
- `docs/architecture.md` `RUL-06` (the profile).
- The seams in `crates/aethelgard-chaos`:
  - `replay.rs` (`State::encode`, the chain);
  - `event.rs` (decoding events);
  - `archive.rs` (`replay_dir`, `check_entries`);
  - `mirror.rs`;
  - `tests/golden.rs`.

> **What this is:** three fixes the adversarial pass measured, each small.
>
> It is NOT:
> - streaming replay;
> - signing;
> - R2;
> - any change to the CBOR profile or the trace file's bytes.

**Cadence:** build-only, and §7 only. One PR, left unmerged.

## 1. Scope, with the rulings

1. **The state chain hashes each event's change, not the whole state** (finding 1: replay took O(events × state), measured 6.5 s for 20,000 distinct registers).
   - `h0` is unchanged: the sha256 of the header item's bytes.
   - For each event `i`, `h_i = sha256(h_{i-1} ‖ encode(delta_i))`, where `delta_i` is one array in the §1.1 profile:
     - register write: `[0, address, value]`;
     - pin level: `[1, pin, level]`;
     - interrupt: `[2, line, new_count]`;
     - analog sample: `[3, net, microvolts]`, with a negative value as major type 1, as the profile already encodes it;
     - fault injected: `[4, new_fault_count]`.
   - A write that leaves the state as it was still gets its delta, so `state_hash` now sees every event that changes nothing. That also closes finding 7's no-op blind spot for those kinds.
   - Replay is linear in events.
   - **This changes what `state_hash` means.** So the manifest's version becomes **2**, and the reader accepts only 2 (`UnknownVersion` otherwise). That follows `AET-WO-0400` §1.7's own rule that a changed constant bumps the version. The trace file's header version stays 1, because its bytes do not change.
   - Recompute the golden `state_hash` on Furnace and confirm it in `gates`. The golden trace sha256 must not change. If it does, that is a §7 stop.
2. **Decoding is bounded by shape before it is built** (finding 2: 578 MB peak memory from one crafted 16.8 MB event).
   - Before reading the trace, a manifest whose `trace_bytes` exceeds 256 MiB refuses with `TraceTooLarge`. `OQ-03` revisits that number when P2 measures real volume.
   - Inside the trace, an event item longer than 4,096 encoded bytes, or a header item longer than 1 MiB, refuses with `MalformedEvent` or `MalformedHeader`. The check runs before the item's nested values are built: read its head, bound its span, then decode. The largest legal event is under 1,200 bytes.
3. **A crash in the mirror step never orphans a committed run** (finding 3).
   - The reader ignores a `mirror.cbor.tmp`, exactly as it ignores `mirror.cbor`. Neither is read by replay.
   - The mirror step removes a stale `mirror.cbor.tmp` before writing its own.
   - Add a kill test: stop the mirror step after its temporary file is written; the run still replays.
4. **Finding 6 (`mirror.cbor` is never read) is accepted as built.** It is a report, not evidence. The README says so in one line, with the words "not covered by the manifest".
5. **Rulings that need no code, for the README only:**
   - artifact names are 1 to 1,024 bytes of printable ASCII with space, as built;
   - an empty environment variable counts as unset in the mirror step;
   - `MissingTrace`, `MalformedHeader`, `MirrorBeforeCommit`, `MirrorExists` and `TraceTooLarge` are added to the README's refusals table.

## 2. Invariants and fences

- **`INV-04`:** the trace file's bytes do not change, and replaying twice gives one `state_hash`.
- **`INV-06`:** every new bound refuses with a named kind. Nothing is truncated or skipped.
- **Protected surfaces diff to empty:** everything outside `crates/aethelgard-chaos`. No new dependency and no `unsafe`.
- One PR, left unmerged. Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance

1. **The golden test:** the trace sha256 is unchanged, and the new `state_hash` constant is reproduced in `gates`.
2. **Linear replay:** 20,000 and then 200,000 distinct register writes. Record both times in FEEDBACK. The second must be within 15× of the first, which shows linear rather than quadratic growth. Assert the ratio, not a wall-clock time.
3. **No-op events:** two traces that differ only by a no-op write give different `state_hash` values.
4. **Bounds:** a 4,097-byte event, a 1 MiB + 1 header, and a manifest declaring 256 MiB + 1 each refuse with their kind. The 16.8 MB crafted event of finding 2 is refused with peak memory under 64 MiB. Measure it on Furnace with `/usr/bin/time -v` and record it in FEEDBACK. It is not a test assertion.
5. **The mirror crash:** the new kill test, and a hand-planted `mirror.cbor.tmp`, both leave the run replayable.
6. **CI:** `gates` and `tools` are green on the first push.

## 4. Scope fence

Streaming replay; signing; a change to the trace file's bytes or the CBOR profile; anything outside `aethelgard-chaos`.

## 5. Adversarial pass — targeted

1. Find two different event sequences that give the same `state_hash`.
2. Find an input that gets past the shape bounds and still allocates in proportion to its size.
3. Interrupt the mirror step at each stage.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0400a.FEEDBACK.md`, same schema. It carries:
- the two replay timings;
- the peak-memory figure;
- the new golden `state_hash`;
- the gates line.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- The golden trace sha256 changes.
- A `state_hash` that differs between Furnace and `gates`.
- A bound that cannot be checked before allocation without `unsafe` or a new dependency.
- A test that can only go green by being skipped, ignored, or weakened.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-0400a: §1 is built and every §3 item passes in gates and on Furnace, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-0400a-replay-bounds.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
