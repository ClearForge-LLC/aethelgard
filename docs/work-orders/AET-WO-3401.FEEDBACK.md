# AET-WO-3401 — FEEDBACK

Builder: Claude (strong tier, the `3400` builder), worktree `~/aethelgard-3400`, branch `wo/aet-3401-sentinel-01`, cut from `origin/main` `36e428b`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #187) and is not committed here.

Labels: **[measured]**, **[read at source]**, **[inferred]**.

## 1. Predictions

### 1.0 Out of order, stated

**§1.1–§1.2's gate run on the two EXT-port candidates (§2) happened before this file was written.** The kickoff's cadence asks for predictions first, so that run is not scored as a prediction. Its outcome was largely known in advance from `3400`'s V5 (the EXT bus) and probe `p-nrst` (NRST on J2), both measured on `main` the day before.

### 1.1 Predictions for what is still to be measured (committed before running it)

- **P1. A second board directory's tier-1 regeneration check** adds 4–8 s of CI time. It duplicates `generate_tier1`'s `generate_reproduces_…` (3.308 s in `ci/test-weights.tsv`) and svd-hal's `the_committed_board_crate_is_exactly_the_generated_one` (1.985 s).
- **P2. A byte-for-byte check of the board's fabrication package in CI** needs one `verify --fab-out` (or `--against-fab`) with the seeded set, because `verify` is the only path to the package (CLI README §The fabrication package).
  - On CI that is about 500–600 s. The reference's measured verifies took 510–579 s (`ci/colocate.tsv`, runs 43889629997 and 43895706619).
  - On Furnace in the pinned image it is about 250–350 s. The CI-to-Furnace ratio is 1.7 (`ci/colocate.tsv`).
  - **That exceeds the 3-minute limit, so §7's first stop fires** for a per-PR package check, unless the check can avoid a full verify.
- **P3.** `verify` of `examples/sentinel-01.json` with copies of the reference's four seeded scenarios, rebased on it, **is refused for coverage** (`CampaignCoverageShort`). The EXT bus has no fault scenario. The board then needs scenarios of its own, under `scenarios/`, which is outside this work order's fence.
- **P4.** With a scenario covering EXT, the package is byte-identical across two `--fab-out` runs, as `3300` made it.
- **P5. The §5 adversarial load.** EXT with a declared off-board load near its budget (75 pF per line, against the declared 100 pF) passes `extract`; at 85 pF `extract` exits 1, naming EXT_SDA/EXT_SCL.
