# AET-WO-0400a — FEEDBACK

**Branch:** `wo/aet-0400a-replay-bounds` · **Base:** `bfe22bc` (PR #22) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Half:** build-only; no §7 stop arose.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## What was built

1. `replay.rs`: the chain hashes each event's delta (`[0,a,v]`, `[1,pin,l]`, `[2,line,n]`, `[3,net,uv]`, `[4,n]`), not the whole state. `State::encode` is gone. Manifest version is now 2 (`MANIFEST_VERSION`); header and mirror stay at 1.
2. `archive.rs`: `TraceTooLarge` for a manifest over 256 MiB, checked before the trace is read. `decode_bounded` decodes an item through a window (1 MiB header, 4,096 bytes event), so an item that runs past it is refused before its nested values are built. The writer refuses a header over 1 MiB.
3. `mirror.rs` / `archive.rs`: `mirror.cbor.tmp` is accepted by the reader's directory check (a regular file) and never read; the mirror step removes a stale one first. `mirror_step_observed` lets a test stop the step after its tmp file is written.
4. README: refusals table, delta table, the "not covered by the manifest" line, and the no-code rulings (§1.5).

## Numbers (Furnace)

- **Golden:** trace sha256 `51c6193f…896d` unchanged (191,873 bytes). New `state_hash` **`af2d46bb1b30e472e73fc9109f69213a80f8537e35d8d9d18f08b2bffb7b4939` (was `535db225…adac4`), computed on Furnace; `gates` must reproduce it (see Gates).
- **Linear replay** (debug build, `Replayer` fold, distinct register writes): **20,000: 0.044 s; 200,000: 0.462 s**, ratio about 10.5 against the allowed 15. The test asserts the ratio. Before: 20,000 took 6.5 s.
- **Peak memory, finding 2's crafted event** (65,536 arrays of 255 empty arrays, 16.8 MB): `/usr/bin/time -v` on the test binary, **19,604 kB** peak RSS (was 578 MB), under the 64 MiB bar.

## Findings

| finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|
| A header just under 1 MiB (65,000 children of 15 empty arrays) passes the window and is built before `Header::from_value` refuses it: 38.4 MB peak RSS (subagent, *run*), about 34× the item, at a 32-byte `Value` per 1-byte item. Bounded and transient, once per replay | `archive.rs` `decode_bounded` | note | Accept; lower `MAX_HEADER_BYTES` if P2 shows headers far under 1 MiB | no |
| Replay state grows with distinct keys: 29.7M distinct register writes (267 MB trace, just under the cap) replayed in 5.55 s at 1.41 GiB peak RSS, about 5.8× the trace (subagent, *run*). `Replayed.state.clone()` and the full-trace read are part of it; not split out. Linear in events and memory | `archive.rs` `replay_dir`, `replay.rs` | risk | The 256 MiB cap is `OQ-03`'s; a streaming reader (out of scope here) removes the copies | no |
| A directory at `mirror.cbor.tmp` wedges the mirror step (`remove_file` fails with EISDIR, and the log line is written again) and `replay_dir` refuses with `UnexpectedEntry`. A FIFO or symlink there self-heals. A crash cannot create a directory there; whoever can can rewrite the trace | `mirror.rs` | note | None | no |
| `mirror.cbor` as a directory: replay refuses and the retry returns `MirrorExists` | `mirror.rs` | note | None (strict-reader behaviour) | no |
| A retried mirror step logs its line again, so "one line" holds per attempt | `mirror.rs` | note | None | no |
| `state_hash` still does not see timestamps or a fault's scenario and detail (finding 7's residue); `trace_sha256` covers them. `timestamps_are_covered_by_the_trace_hash_not_the_state_hash` pins that | `replay.rs` | note | None | no |

## Adversarial pass (§5, targeted)

A fresh subagent (Sonnet 5.5) ran it at `7c61298` in its own worktree with uncommitted probes. Its claims below are the *ran* ones I read in its report; I did not re-run them except where noted.

1. **Same `state_hash`, different sequences:** none beyond the known residue. It searched all 14,425 sequences up to length 3 over a 28-event alphabet (boundaries at 23/24, 255/256, `u32::MAX`, i64 extremes, the "a"/"ab" prefix pair, faults) against the sequence with timestamps and fault text stripped: no collision. Reasoned: deltas begin with their kind and the profile is canonical, so the delta sequence is injective. No domain separation between `h0` and the chain; a collision would need a header that begins with a 32-byte hash and a delta (about 2^128).
2. **Past the bounds and still proportional:** the 1 MiB header (finding 1) and replay state (finding 2), both above. Events are bounded to about 128 KB transient (*reasoned*, not run). A depth-8 chain of arrays each claiming 65,536 elements: 4.9 MB RSS (*run*). No super-linear replay.
3. **Mirror step interrupted:** a stale regular tmp (any size, 64 MiB tried) or a crash after the rename: replay stays OK and the retry heals or returns `MirrorExists`. Directory, FIFO, symlink at the tmp: see findings. Power loss before the directory sync not tested (same as AET-WO-0400 #8).

My own mutants, one per new guard, each killed: window bound (3 tests fail), `TraceTooLarge` check, tmp accepted by the reader, tmp cleared by the mirror step, manifest version back to 1, header window, and a delta that drops the interrupt count (`the_chain_is_the_documented_deltas`). A first interrupt mutant of mine was a no-op edit and "survived"; I redid it as a real one.

The adversarial subagent's isolated worktree was created by the harness under `~/aethelgard/.claude/worktrees/`, which the kickoff fences off. I removed that one worktree and its branch (`git worktree remove --force`, `git branch -D`) once its report was in; nothing else there was touched.

I slipped once: a `git checkout -- .` appended to a measurement command would have discarded the uncommitted work. The permission check refused it; nothing was lost, and the work was verified intact before continuing.

## Gates

- Reduction pass (diff): `Reduction pass (diff): −13 net · 1 duplicates removed · 0 dead paths removed · 0 protected kept · 0 findings below` (the two test helpers `header_bytes` and `write`, defined alike in two test files, now live in `tests/common`).
- Local on Furnace: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` green.
- CI on the first push (`70cea2f`+`6af0bb0`, PR #24): run [42411223504](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42411223504), `gates` and `tools` both green **on the first push**. `golden_trace_and_state_hash_are_pinned` passed in `gates`, so `af2d46bb…4939` is reproduced there and the trace sha256 is unchanged. No §7 stop.
- **Token handling:** the Write tool was used once, for the push and PR token files, on an architect ruling since withdrawn (FUR-WO-0301 F7 bars it because it echoes the token into the pane). The operator-sanctioned route is `f=$(gh-write prep)` with a `grep -o` of the token from the session transcript redirected into `"$f"`; the revoke and this fix's push used it. The token was already in the transcript from the mint. It is revoked right after this FEEDBACK push (operator ruling 2026-10-12); the HTTP 204 is reported in the session, not here.
- Protected surfaces: `git diff bfe22bc --name-only` lists only `crates/aethelgard-chaos/**` and this file. No new dependency, no `unsafe`.

## What was deliberately not built (§4 held)

Streaming replay, signing, any change to the trace's bytes or the CBOR profile, anything outside `aethelgard-chaos`.

## Promotions this work order asks for

None. `OQ-03` gains two measured inputs (findings 1 and 2).
