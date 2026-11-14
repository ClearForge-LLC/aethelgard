# AET-WO-2205 — The archive says what judged it and what it means: header versions, honest record names, attribution's evidence exact

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2205-archive-hygiene`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P2/P3 hardening. **Cadence: build, with one stop (§1.6) for the seeded set's evidence table before its code.**

**Grounds, each from a merged work order or the review:**
- **Review B4 (`RUL-19`).** The archive does not say which engine and tool versions judged it, so a replayed run after a Renode upgrade cannot say what it was judged by. Record kinds still carry "measured" (`analog.rise_time_measured`) for values that are computed from declared ones (`architecture.md` §2.19).
- **Review A10.4, `2301` B4.** The analog deck's 400 kHz window is fixed (`twin/src/analog.rs`), whatever the bus's declared speed, and nothing in the record says so.
- **`3100` F2.** The status-to-phase table, which gates `TXDR` and `NBYTES` as evidence, lived in `aethelgard-evolve` and was copied from the generated `transfer`'s phases. **`2204` removed the `CR2`/`TXDR` evidence from evolve** (`READ_PHASE`, `WROTE_TXDR`, `NBYTES_BITS` and others). Whether any phase table remains is this work order's first question.
- **`3100` F13.** `pc_outside_firmware` is listed as software evidence, but the firmware's executable segments are not archived, so a run failing only on it attributes `I1`, never `S2`. `2204` now reads the segments at their physical address (`cli/src/elf.rs`).
- **Review A7.5.** The seeded set's test allows evidence kinds per rule (its `allowed` table duplicates `Rule::evidence_kinds`; kind 7 is allowed for S1 and S2), not the exact evidence per case.
- **Review A7.6.** No seeded case is indeterminate end to end.
- **`2301` D10 (closed 2026-11-14).** `power::SOURCE` cites DS12117 Rev 7 from a mirror. Rev 10, fetched from st.com, has equal thresholds (Table 16, p. 107, sha256 `607b9ead…c032`).
- **`2204` B4.** The droop's `ObservedReport.witness` carries a whole `WitnessReport`, of which kind 19 archives only `observed`, and its `absent` is always empty.

> **What this is:** the archive and the verdict made honest about what judged them and what their numbers are, and attribution's evidence pinned per case.
>
> It is NOT:
> - a change to any verdict, finding or attribution outcome on the reference or the seeded set, except where §1.6's ruling names one;
> - a change to the boot judge (`2204`) or the droop model (`2301`).

## 1. Scope

1. **The header names its judges.** The archive header carries:
   - the engine's version;
   - the Renode build string it ran (`1.17.0+…git…`, as Renode prints it);
   - the ngspice version;
   - the bridge's identity hash (`2301`).

   Each value is read from the tool, never written as a constant. Replay reports them; it does not require them equal to the replaying host's.
2. **Record kinds renamed from "measured".** `analog.rise_time_measured` and every kind or field naming a computed value as measured become "computed" (`RUL-19` (1), `architecture.md` §2.19). Readers of version 5 are refused by name.
3. **The 400 kHz window labelled.** The analog record states the deck's window and the bus's declared speed beside it. The window itself does not change.
4. **The status-to-phase table (`3100` F2).** Find whether any status-to-phase gating survives `2204`.
   - If none does, close F2 in FEEDBACK with the commit that removed it, and the attribution test that shows a TXIS timeout alone is still never software.
   - If some does, move it to one place both the judge and attribution read.
5. **Executable segments archived (`3100` F13).** Archive the segments the judge used (`elf::Image`'s, at their physical address), so `pc_outside_firmware` becomes software evidence (`S2`). Add a seeded case that shows it.
6. **The seeded set's exact evidence (A7.5, A7.6). STOP here once, before code:**
   - Write in FEEDBACK a table of every seeded case: its rule, and the exact evidence indices and kinds it must attribute with.
   - Propose one indeterminate case end to end (a scenario file, its expected `I*` rule and its evidence).
   - Write `AET-WO-2205 STOPPED: evidence table, ruling needed on PR #<n>`. Build after the ruling: the test asserts exact evidence per case, and `allowed` no longer duplicates `Rule::evidence_kinds`.
7. **`power::SOURCE` cites Rev 10.** Revision 10, January 2026, Table 16 (section 6.3.4, p. 107), `read_from` st.com (fetched by Scotty, 2026-11-14), and Rev 10's sha256. No threshold value changes; a test asserts every value equal to Rev 7's.
8. **`ObservedReport.witness` slimmed (`2204` B4)** to what kind 19 archives. Name the verdict JSON's change in FEEDBACK.
9. **The trace version goes from 5 to 6, once,** for items 1, 2, 3, 5 and 8 together.

## 2. Fence

- `crates/aethelgard-chaos` (header, records, trace version, `power`);
- `crates/aethelgard-twin` (`analog.rs`'s record only);
- `crates/aethelgard-evolve` (evidence and its tests);
- `crates/aethelgard-cli`: where records and the verdict are built (`campaign.rs`, `analog.rs`, `renode.rs`'s report, `verdict.rs`), and `elf.rs`'s segments into the record;
- the seeded set's scenario files, tests and fixtures;
- `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
- this work order's FEEDBACK.
- **No change to the generated firmware or the committed generated files.** No `unsafe` outside `bridge/src/ffi.rs`.
- One PR, left unmerged; **retitle it when it moves from the §1.6 stop to the build.** Push and revoke by the FUR-WO-0301 F7 route.

## 3. Invariants

`INV-04` (replay: every version-6 archive replays byte-identically, and each older version is refused by name), `INV-06`, `INV-07`; `RUL-06`'s CBOR profile is unchanged; `RUL-19`.

## 4. Acceptance

- **Every reference and seeded verdict is unchanged** against `main`'s baseline (binaries built from `main`, the same scenarios), except the fields this work order names. Each changed field is listed in FEEDBACK with the run that shows it.
- **The header's versions are read from the tools,** shown by a test that runs a different Renode build string through a stub.
- **The seeded set asserts exact evidence per case,** with the indeterminate case end to end.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes (`AET-WO-1908` §8).

## 5. Adversarial pass, targeted

1. Make a replay pass with a forged header version, or with segments that do not match the ELF the run booted.
2. Make a seeded case attribute with evidence the exact table does not list.
3. Feed a version-5 archive to every reader. Each must refuse it by name.

## 7. Flag-and-stop conditions

- A verdict or attribution outcome on the reference or the seeded set must change for a reason this work order does not name.
- A tool version cannot be read from the tool, so it would have to be written as a constant.
- `AET-WO-1909` (CI reduction) is touching the same test files at the same time. Merge `origin/main` and keep both; stop if a test both changed cannot be reconciled.
