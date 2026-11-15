# AET-WO-3303 — "Verified" means verified at fabrication: preconditions, the pair judge's allowlist, a strict number reader, and `main` kept green

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3303-preconditions`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P3 hardening, before `AET-WO-4500`. From the second Mythos 5.2 review (`docs/reviews/2026-11-15-mythos-5-2.md`, A6, A7, A9, A11, A12, A13 D1–D3, A15.1, A16, B6; response `…/2026-11-15-mythos-5-2.response.md`). **Cadence: build, with one measured stop (§1.2).**

**Grounds, on `main` (each checked at source by the architect, 2026-11-15):**
- **A6.** `crates/aethelgard-cli/src/verify.rs:492-504` drops `proofs::every_valid_sequence` on a motor board and reports it at severity `info`. `report.ok` counts only errors, so `--fab-out` writes a package with the P1 schedule proof not run. Sentinel-02 is a motor board. `4300a` Q5 recorded that the harness does not finish in 33.5 minutes, and put its split in `4500`.
- **B6.** `INV-03` asks that the fault campaign ran. One no-fault scenario satisfies `verify` today.
- **A7.** `.github/workflows/quickstart.yml` replaces only the `IMAGE=` line on a `main` run. The README's pinned specification then meets `main`'s binary, and `main`'s engine-image workflow was red from `4100` until the re-pin in PR #150. Every specification bump turns it red again.
- **A9.** `check drc --spec` runs the pair judge with no allowlist (`cli/src/main.rs:786-809`). The reviewer added one `(arc …)` stub to a pair line, and the pair report was unchanged.
- **A11.** `eda/src/sexpr.rs:378-398` accepts `"0.2"`, `+0.2` and `--5` as numbers that KiCad 10.0.7 refuses to load.
- **A15.1.** `crates/aethelgard-pincheck/src/elf.rs:154-162` scans the string table to the next NUL once per symbol, with no bound.
- **Unowned backlog this work order takes** (`roadmap.md` §7.1):
  - FAB-NOTES states every condition the `extract` verdict states (`4300f` F2);
  - a connector named `U1` and a `u64::MAX` off-board budget are refused at `validate` (`4300f` A3, A4);
  - the fabrication package's lifecycle line (`4210` F-B3);
  - fold `fab::describe` into `fab::LAYERS` (`3300` F21).

> **What this is:** `verify --fab-out` and `--against-fab` refuse to write a package unless every precondition `INV-03` implies holds, and the package says which held. The pair judge and the KiCad number reader stop accepting what KiCad refuses. `main`'s quickstart job tests `main`'s own example, so a specification bump no longer turns it red.
>
> It is NOT:
> - the witness ledger and its acknowledgement (`AET-WO-4230`, which adds its row to this list);
> - the support gate (`AET-WO-4300g`).

## 1. Build

1. **Preconditions for a package.** `--fab-out` and `--against-fab` refuse, each by name, when:
   - any harness `generate` produced was not proved (`HarnessSkipped`);
   - the campaign has no seeded fault on a declared bus or a declared rail (`CampaignCoverageShort`), with each uncovered bus or rail named;
   - any other gate's verdict is not `pass`.

   The package's `VERDICT` lists each precondition and its state. `AET-WO-4230` adds the ledger's row to the same list. Whichever merges second rebases onto the other's list.
2. **Split the whole-schedule harness (measured stop).** Split `proofs::every_valid_sequence` by peripheral, so that each part finishes in CI's budget on today's motor fixtures. Measure each part's time in the image.
   - If no split of the same claim finishes, **STOP** with the times. Never weaken the claim to make it finish.
3. **The pair judge's allowlist.** `check drc --spec` runs extraction's copper allowlist before `connectivity`, `check` and `pairs`. The board with the reviewer's arc stub is refused (`UnmodelledCopper`), as `extract` refuses it.
4. **A strict number reader.**
   - A bare numeric atom matches `-?[0-9]+(\.[0-9]{1,6})?`. Anything else is refused by name.
   - `(version …)` is pinned to the value KiCad 10.0.7 writes.
   - A layer row with a fourth element is refused, and a copper layer is recognised by its canonical name, not `ends_with(".Cu")`.
   - The A11 subagent items are verified at source first.
5. **The ELF string compare** reads at most `SYMBOL.len() + 1` bytes.
6. **Tests.**
   - Pin a line's total, off-board term included, in `aethelgard-eda`'s own tests (A12's surviving mutant).
   - Fold the per-section missing-key tests into one walk over every object of a full fixture, so a new section is covered when its fixture is.
7. **The quickstart on `main`.**
   - The `main` run substitutes `main`'s own `examples/sentinel-01-minimal.json` and its sha256 for the README's `SPEC` and checksum lines, as it already does for `IMAGE`.
   - A second step runs the README block exactly as written, against its own pinned image, so the README's pair is still tested.
8. **The backlog this takes**, as listed under Grounds.
9. **Crate README drift** (review D1–D3):
   - `cli/README.md`'s tool count;
   - its solved schema version and hash;
   - `ir/README.md`'s version lines.

   Where it can, a test reads each from the code, so they cannot drift again.
10. **The A16 claims in this fence**, verified at source and fixed where real, or named where not:
    - `route::part`'s circle pad (`w.min(h)/2`);
    - the pair judge's missing minimum coupled length;
    - `via1`/`track_x` reference names;
    - `connectivity`'s cost, with an item cap outside `extract`.

## 2. Acceptance

- **Each precondition refuses on a fixture built to break it,** and the reference package's `VERDICT` lists every precondition as met.
- **The motor fixtures prove every harness part in CI.**
- **The arc-stub board is refused** by `check drc --spec`. Each of the reviewer's three spellings, and `--5`, is refused by `extract`.
- **The quickstart job is green on this PR and passes on `main`** after merge (the architect checks).
- **Every existing board's files are byte-identical.** Verdicts gain only the precondition list.
- **Each new refusal has a tier-1 test.** The CLAUDE.md gates pass. CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-13`, `RUL-19`.
- **Fence:**
  - the CLI: `verify`, `fab`, `main.rs`'s DRC path, `validate`'s two refusals;
  - `aethelgard-eda`: `sexpr.rs`, the route reader's pair path, `fab.rs`, tests;
  - `aethelgard-pincheck`: `elf.rs`;
  - `aethelgard-svd-hal`: the harness split only;
  - `aethelgard-ir`: tests only;
  - `.github/workflows/quickstart.yml`; crate READMEs; `ci/`; FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-3303/pr-title-body.md`.

## 5. Adversarial pass, full

1. **Get a package past a skipped harness:**
   - a harness renamed so the plan misses it;
   - a harness that times out;
   - `--against-fab` instead of `--fab-out`.
2. **Satisfy coverage vacuously:**
   - a fault on a bus no device uses;
   - a "fault" whose parameters equal the no-fault case.
3. **Copper the allowlist misses** on a pair: a `gr_arc`, a zone fill on one line, a via in a pad.
4. **Numbers:** an exponent, a leading `.`, a 7-digit fraction, a Unicode minus.

## 7. Flag-and-stop conditions

- **§1.2: no split of the whole-schedule claim finishes** in CI's budget.
- **Campaign coverage needs a fault kind the chaos crate does not have** for some declared rail or bus. Stop with the list.
