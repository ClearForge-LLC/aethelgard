# AET-WO-1216b — The power sequence judged: every path from a rail the drawn tree does not place after the MCU supply, judged at VDD = 0 against DS12117 Table 10

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1216b-off-domain-paths`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before the P4 baseline's bench. Written 2026-11-22. **Cadence:** a build, to the spike's ruling. **After `1216`** (#317, the record) and `1211f` (#316).

**Grounds:** `1216` FEEDBACK (§1–§4, §7) and its ruling of 2026-11-22T08:15Z, both on `main`. Read `spike/1216/RULES.md`, `judge.py` and `RESULTS.md` first: the walker and the rule already exist in prototype, and the counts they give are this work order's acceptance.

> **What this is:**
> - **One validate step in `aethelgard-ir`:** a walker over the path classes the spike's walk 1 lists (the sense divider, pin-device pulls and current outputs, I2C pulls, crossings, control wires). Each path from a rail that the drawn tree does not place after `mcu.supply` is judged against DS12117 Rev 10 Table 10 at VDD = 0: **4.0 V, BOOT0 9.0 V**. Above it: `OffDomainAboveLimit`, naming the path, the figure, its source and whether that source is a maximum or a recommended-operating figure (D5).
> - **The exemption, read from drawn wiring only (D1):** a rail is judged by A1 alone when its chain passes through `mcu.supply`, or its regulator's EN is wired to a PG on `mcu.supply`'s chain. **The exemption never holds for a rail any connector carries** (F8), and a device with two supplies is judged on each.
> - **Table 10's limits as data** on the MCU row (`power-parts.json` `io`, beside `1211e`'s Table 8 structures), each cited.
> - **Note 3's reading (D4):** VIN up to Table 10's maximum is admitted.
> - **An unbounded path is named, never passed (D2):** in the ledger's not-judged rows and `twin_limits`, never in `validate`'s success output. No committed `validate` byte moves.
> - **D1's fixture: R2 becomes 1.07 kΩ (D3).** At the spike's corrected worst factor it gives 3.33 V at the held 3 mA and 3.88 V at the 3.5 A peak on the typical ratio. Re-check it against the fixture's own tolerance, and say in its source that the peak rests on the typical ratio. `1211e`'s A2 still holds.
> - **Specification 25** and `versions::CHANGES` row 25, naming the rule (`1215` D4).
> - **F7:** re-run the spike's two walks on D1 now that `1211f` draws it, with walk 2's U2 rows classified by the pin row's kind.
>
> It is NOT:
> - a declared sequence (option (a)) or the race refusal (option (b));
> - any dynamic case: power-down, brown-out, hot-plug (named in FEEDBACK as unjudged, as the spike does);
> - a connector drive bound (`1216` D2's row, a connectors work order).

## 1. Build

1. **Measure first,** with predictions committed: the spike's `lean` rule, run on `main`, gives 1/34 (D1). Your walker must give the same paths and the same count before D1's fixture changes, then 0/34 after it.
2. **The walker, the limits, the exemption, the refusal,** in `aethelgard-ir`.
3. **The not-judged rows** where the ledger and `twin_limits` already name what was not judged.
4. **The fixture, the version bump, and the goldens** their generators move.
5. **Tests and mutants:** one per path class, one per exemption guard (downstream only, connector-carried rail, two supplies), one for each limit (4.0 V and BOOT0's 9.0 V), and one showing an unbounded path is never counted as judged.

## 2. Acceptance

- **The walker's paths equal the spike's walk 1** on every committed board, path for path.
- **Before the fixture edit, D1 alone is refused,** by `OffDomainAboveLimit`; after it, none.
- **Every other committed specification validates, solves and emits byte-identically** except the version line. Show it by a null sweep.
- **The CLAUDE.md gates pass, and CI is green.** Each new tier-2 test has its weight row in `ci/test-weights.tsv`. If a `fab/` byte moves, queue for a dispatch: the package queue is serial.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (the walker, limits data, the refusal, versions, README, tests);
  - the ledger's not-judged rows and `twin_limits` reasons, in the CLI, for D2 only;
  - `crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` (R2);
  - the version literals and goldens the bump moves, by their generators;
  - `ci/test-weights.tsv`, weight rows only;
  - `spike/1216b/` and FEEDBACK.
- **Not:** the generator, the twin, chaos, eda; workflows.
- **Running beside you:** `1606` (the bridge, `doctor`, `build`; in the package queue) and `2308` (`verify`'s backlog: fab notes and test guards). `2308` touches the ledger too: merge `main` early.
- One PR, left unmerged. Write `~/.cache/aet-1216b/pr-title-body.md`.

## 5. Adversarial pass, full

- **A path the walker misses** that the spike's walk 2 (the drawn netlist) finds.
- **An exemption granted by a declaration rather than by drawn wiring,** or to a rail a connector carries.
- **An unbounded path reported as judged,** anywhere.
- **A limit read from Table 13 instead of Table 10**, or at VDD other than 0.

## 7. Flag-and-stop conditions

- **The walker's count differs from the spike's 1/34 before the fixture edit.** Stop and report the difference path by path.
- **Any committed specification other than D1 is refused.** Stop.
- **Naming an unbounded path needs a change to `validate`'s success output.** Stop and report: D2 rules it out.
