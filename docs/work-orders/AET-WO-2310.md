# AET-WO-2310 — One trace and scenario schema bump: the droop's `bor_level` gone, `window` removed, events planted from the boot word, a sleeping boot as a run finding, tool content hashes in the header, and the unread-boot literals named

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2310-trace-schema`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, backlog wave 2. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `2309`** (#340).

**Grounds** (roadmap §7.1; read `2302`, `2309`, `4241`, `1606` and `3201` FEEDBACK first):
1. **`4300d` D-B1.** A droop's own `bor_level` is dropped from the scenario format. Today it is refused when it contradicts `mcu.bor_level`.
2. **`2302` reduction finding 2 (#175).** The verdict's and campaign lines' `window` field always says `MAX_VIRTUAL_NS`. Remove it.
3. **`4241` §0a.** An event planted at a fixed count of spins from reset is brittle to any change in boot length: `4241`'s boot shift moved seven pinned instants. Plant events relative to the boot word.
4. **`2302` Q1 (#175).** A core that sleeps and then writes its boot word exits 5. The campaign should record it as that run's finding, not abort.
5. **`1606` D3, F4.** Tool identities are self-reported names. Archive a content hash of libngspice and of the toolchain beside them in the trace header.
6. **`3201` Q4, the `renode.rs` half.** Name the five unread-boot findings' bare literals as constants.
7. **`2309` FEEDBACK §5.** A read boot's witness blanked without its `*_transaction_unwitnessed` finding still replays. Make it a verdict rule.

> **What this is:**
> - **(1)–(7)** under one trace-version bump and one scenario-format version, each with a test and a mutant.
> - Old archives are refused by version with the list of changes, on the `1215` pattern.
>
> It is NOT:
> - a specification bump;
> - a change to the twin's models;
> - any change outside the fence.

## 1. Build

1. **Measure first,** with predictions in `spike/2310/`:
   - every committed scenario and archive replayed on `main`;
   - for (3), every planted instant and where it lands relative to the boot word;
   - for (5), the hashes and where they come from.
2. **Stop with:**
   - the new trace header and scenario shape, and their versions;
   - (3)'s mapping, and whether every committed scenario's events land identically after re-expression.
3. **Build to the ruling.** Move `scenarios/` by their generators where one exists. The bench package moves (its trace header): request a dispatch on the bus. `4300i` may also be in the package queue; the second to dispatch merges `main` and dispatches again.

## 2. Acceptance

- **(1)–(7) closed,** each by a test that fails without it.
- **Every committed scenario** replays to the same verdict, except for the fields the bump removes or adds.
- **The CLAUDE.md gates pass, and CI is green.** The bench package is committed byte for byte from its dispatch.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-chaos`: the `campaign.rs` scenario format, the archive header, replay;
  - `crates/aethelgard-cli`: `src/renode.rs` and `src/campaign.rs`, and their tests;
  - `scenarios/`, and the goldens and version literals the bump moves, by their generators;
  - `fab/` only by the dispatch;
  - `spike/2310/` and FEEDBACK.
- **Not:** the IR, eda, the generators, hwdef, pincheck; workflows; `ci/` except `test-weights.tsv` rows.
- **Running beside you:** `4300i` (IR and generators; it also queues a package), `1607` (CLI `main.rs`, `verify.rs`), `1914` (`ci/`). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-2310/pr-title-body.md`.

## 5. Adversarial pass, full

- **An archive at the old version** that replays without its refusal.
- **A planted event** that lands on a different instruction after re-expression.
- **A tool hash** that does not change when the tool's bytes do.
- **A sleeping boot** that still exits 5 by some path.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the shapes and (3)'s mapping.
- **Any committed scenario's verdict moves** beyond the removed and added fields. Stop with the diff.
