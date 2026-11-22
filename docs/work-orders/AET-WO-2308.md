# AET-WO-2308 — `verify`'s backlog: a stand-in carried into every fab file, the campaign entry's tool versions, an empty directory reported, two guards given killing tests, and the ledger's clock-rule class

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2308-verify-backlog`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-22. **Cadence:** a measurement, then a build. **After `1211c`** (#313), which held `verify`.

**Grounds** (roadmap §7.1; each row's FEEDBACK is on `main`, read it first):
1. **`1211f` F1 (G1), first and blocking.** A stand-in part's note ("stand-in, not a design") reaches the KiCad BOM but not the JLCPCB BOM (`fab::bom_rows` drops `_note`), the sheet or `FAB-NOTES.md`. W25Q's stand-in note is dropped the same way. Since `1211c`, a pin-device board can reach `verify --fab-out`. Ruled: this lands before any board with a stand-in writes a fab package.
2. **`3301` L3:** the `verify` campaign gate's entry carries no tool version; Renode's is in `simulate`'s.
3. **`3301` L4:** `--against` compares files, so an extra empty directory is not reported.
4. **`1909` LP2:** `verify_47k_tier2` runs 47k only with `--fab-out`, so nothing shows that the exit code does not depend on the fab mode.
5. **`3300` F16:** the export's ELF and board `BindingMismatch` guards have no killing test. A swap race in `verify`'s scratch would show it, as `1305`'s extract test does.
6. **`4240d` F5:** the ledger classes every field of the bring-up's whole `RCC_CR` and `RCC_CFGR` words (`HSION`, PLL2/PLL3, `CFGR`'s other fields) as `independent-rule` through the twin's `clock_rule`. That rule is `aethelgard_twin`'s, not the IR's, so the class overstates what judged them.

> **What this is:**
> - **(1) A stand-in is visible wherever a person reads the package.** Two acceptable shapes; measure both, choose by what the fab files can carry:
>   - (a) carry the note into the JLCPCB BOM's note column, the sheet and a `FAB-NOTES.md` section listing every stand-in;
>   - (b) refuse `verify --fab-out` by name while any BOM line is a stand-in (`StandInInFabPackage`), unless the specification marks the board a bench record.
>   - Never a stand-in that reaches a fab file with no label.
> - **(2)–(5)** as their rows say, each with a test that can fail.
> - **(6)** the ledger's class for those fields made true: the class names the twin's rule as the judge, or the fields are `ignored-by-model` with the reason. Measure which first.
>
> It is NOT:
> - the power sequence (`1216b`, beside you);
> - a change to any verdict of a committed board beyond the fields these rows move, each named.

## 1. Build

1. **Measure first,** with predictions committed in `spike/2308/`:
   - which committed boards carry a stand-in today, and what each fab file prints for it;
   - every committed package's bytes, as the baseline;
   - for (6), the ledger rows that name `clock_rule`, and what judged each field.
2. **Each row's fix, its test, and a mutant** the test kills.
3. **The packages:** if a committed `fab/` byte moves, it moves only by these rows, and you queue for a dispatch.

## 2. Acceptance

- **No stand-in reaches a fab file unlabelled,** shown on a fixture with a stand-in through `verify --fab-out`.
- **Rows 2–6 each closed** with a killing test.
- **Every committed verdict and package is byte-identical** except what these rows move, each line named.
- **The CLAUDE.md gates pass, and CI is green.** Each new tier-2 test has its weight row.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli`: `verify`, `fab.rs`, the ledger, and their tests;
  - `crates/aethelgard-eda`'s `fab.rs`, for the JLCPCB BOM's note column only;
  - `ci/test-weights.tsv`, weight rows only;
  - `spike/2308/` and FEEDBACK.
- **Not:** the IR, the generator, the twin, chaos; workflows.
- **Running beside you:** `1606` (the bridge, `doctor`, `build`; in the package queue) and `1216b` (an IR validate step, plus D2's not-judged rows in the ledger). `1216b` touches the ledger: merge `main` early and often.
- One PR, left unmerged. Write `~/.cache/aet-2308/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A stand-in that reaches any fab file unlabelled,** by another BOM path, a DNP line or a second package format.
- **A test that passes with its guard deleted.**
- **A ledger class that names a judge which did not judge.**

## 7. Flag-and-stop conditions

- **Neither (1)(a) nor (1)(b) fits the fab formats as held.** Stop with what each format can carry.
- **Any committed verdict moves beyond these rows' fields.** Stop and report the diff.
