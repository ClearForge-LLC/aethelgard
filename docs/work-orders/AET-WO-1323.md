# AET-WO-1323 — The router's backlog: a work bound that refuses by name, the soft hop judged on the hard maps, a keepout held to its footprint, no stub left by a pair's breakout, and the supply gate's double solve

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1323-router-backlog`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, backlog wave 3. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `1322`** (eda, wave 2). W4's placer follows this, in the same crate.

**Grounds** (roadmap §7.1, quoted from `main` at `f42a0c88`; triage W3-ROUTER, rows 874, 909, 910, 960, 999):
1. **`1307` A4** (roadmap L902): "Router wall-time bound (an adversarial 4 × LQFP-144 board took 616 s, bounded only by `RIPUPS`)". `route.rs:87` `RIPUPS` 500 is the only bound (`route.rs:2258`, `:2314`). `1307` FEEDBACK A4: 616 s and 568 MB on a 249 mm board built as `route::Board`, which `emit` cannot produce.
2. **`1310` ruling** (L937): "`U1`'s soft hop in the escape (dropped from `1310`'s build)". `1310` FEEDBACK D3 defines U1: a soft search's flattened hop judged on the hard maps (`unhop`, `route.rs:1157`), "arguably the sounder rule". `BANS` 16 (`route.rs:157`) was built instead because U1 moved d3's bytes.
3. **`1310` A7** (L938): "A keepout anywhere on the board, not only at the edge". **The FEEDBACK says the opposite of the row's wording** (`1310` FEEDBACK A7, §14): the reader *accepts* a footprint keepout placed anywhere, so a zone moved off its footprint makes `check` judge copper by the misplaced zone. Its fix: the reader requires the zone inside its footprint's courtyard or field. The reader is `route/read.rs:256-258`, `keepout` at `:273`.
4. **`1311` F2** (L988): "The router can lay a dangling breakout stub of a fixed pair (`paired(12 mm)` with a part's pads either side); its own judge refuses the result, so it fails closed". The board is `paired` (`route.rs:4645`); the refusal is `connectivity`'s "joined to nothing" (`route.rs:4118`).
5. **`1910` F1** (L1027): "The supply gate solves `s024l_board()` twice (60 and 165 mV), about 250–390 CI s each, in both jobs; only a change inside `extract` could share the solved network". Owner: "a production change, never at the cost of what the gate proves". Tests: `cli/tests/supply_tier1.rs:259`, `:365`; the solve is `supply.rs` over `flow.rs`.

> **What this is:** (1)–(5), each refusal with a test, and each test with a mutant it kills.
> - **(1) is a work bound, not a clock.** A wall-time refusal would make `emit`'s answer depend on the machine. Count the router's work (search expansions, say), refuse past a named bound as a new `EmitError` kind, and size the bound from §1.1 with a stated margin over the heaviest committed board.
> - **(3) is built as the FEEDBACK states it,** not as the row reads. Say so in FEEDBACK, for the docs PR.
> - **(5) shares nothing across processes:** no cache on disk. Find where the time goes, and cut it inside `extract` only if every verdict, number and note stays the same.
>
> It is NOT:
> - placement (W4), the escape plan's lanes, or `BANS`;
> - any change to `check` or `connectivity`'s judgement, which (2) and (4) must still pass;
> - a specification, trace or format bump.

## 1. Build

1. **Measure first,** with predictions committed in `spike/1323/` before the runs:
   - the emit baseline of every committed specification (`examples/`, `crates/*/tests/fixtures/`), as sha256 per file;
   - each of (2) and (4) prototyped: **which boards' emitted bytes move, by name, and whether any `fab/` byte moves** (`examples/sentinel-01.json` is the ordered bench board);
   - (1)'s work count on every committed board and on `1307` A4's adversary;
   - (5)'s time split: emit, network build, solve, bound, at each limit.
2. **Stop with the measurement,** (1)'s bound and its name, and (2)/(4)'s byte movers. If a `fab/` byte would move, do not build that piece: the package queue is serial, and Sentinel-01 is ordered.
3. **Build to the ruling.** Moved goldens move by their generators.
4. **Re-run the baseline:** every byte as ruled.

## 2. Acceptance

- **(1):** the adversary is refused by name under its bound; every committed board routes under it, with the margin stated.
- **(2):** d1 still routes; every board's bytes as ruled.
- **(3):** a keepout outside its footprint is refused by name at read; every vendored footprint's keepout is read as before.
- **(4):** `paired(12 mm)` with `S` either side routes with no stub, or is refused by name before `connectivity`.
- **(5):** `supply_tier1`'s two tests give the same verdict bytes; their time before and after, measured.
- **Mutants, each killed by a named test:** the bound's check removed, and the bound off by one; U1 reverted to bans alone; the keepout's containment check removed; the stub fix reverted.
- **The CLAUDE.md gates pass, and CI is green.** Weight rows move with any test time.

## 3. Fences

- **Fence:** `crates/aethelgard-eda` (`route.rs`, `route/read.rs`, `extract.rs`, `supply.rs`, `flow.rs`, `schematic.rs` for the one new kind) and its tests; `crates/aethelgard-cli/tests/{supply_tier1,stackups_tier1,classes_tier1}.rs` for the asserted kinds and times; `crates/aethelgard-cli/README.md`'s refusal row; `ci/test-weights.tsv` rows; `spike/1323/` and FEEDBACK.
- **Not:** `pcb.rs` (placement), the IR, the generators, the twin, chaos; workflows; `ci/` except weight rows.
- **Running beside you:** the backlog wave in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1323/pr-title-body.md`.

## 5. Adversarial pass, full

- **A bound that cannot fail,** or one a committed board crosses on a slower runner: it counts work, so a timed run must give the same count.
- **A soft hop** that U1 passes and `check` refuses.
- **A keepout on the footprint's edge,** turned 90°, or under a part named like copper (`1311` A16).
- **A pair whose breakout stub survives** at another placement than the test's; sweep placements as `1307` B6 did.
- **A shared solve** whose 60 mV verdict differs from today's in any field.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement, the bound and the byte movers.
- **A `fab/` byte moves.** Stop with the diff; that piece waits for the package queue and a ruling.
- **(5) can be made faster only by changing what the gate proves** (a looser bound, a cell dropped, a note lost). Stop; record it, and do not build it.
- **A keepout in a vendored footprint lies outside its own courtyard and field.** Stop with the footprint: (3)'s rule would refuse a real part.
