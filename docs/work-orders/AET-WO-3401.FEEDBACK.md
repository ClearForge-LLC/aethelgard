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

## 2. §1.1–§1.2: `examples/sentinel-01.json`, measured

**What it adds to `sentinel-01-minimal.json`** (every V0 field kept, `bor_level` and the BOR acknowledgement among them):
- **NRST on J2 pin 3,** for connect-under-reset;
- **the EXT bus:** I2C, 10 kΩ, 100 kHz, declared at 100 pF and 1000 ns per line, with no device on it;
- **J4,** an `SM04B-GHS-TB` in DS-009 I2C's pinout (1 supply, 2 SCL, 3 SDA, 4 GND), each bus pin declaring 20 pF off-board (the fixtures' JST-lead condition);
- **`project_name` `sentinel-01-bench`;**
- **the EXT bus's 24 ledger rows acknowledged** (D2).

**D1, the EXT port's supply, decided by measurement.**
- **Candidate A**, `standard: "DS-009 I2C"` with a `VDD_5V` domain entering on J3 pin 2:
  - it passes all eight commands;
  - **but `verify --fab-out` refuses it: `CampaignCoverageShort`, "the campaign covers no fault on rail `VDD_5V` (not the MCU's supply: no fault kind droops it)"** [measured]. No fabrication package can be made for a lean board with a second, unregulated rail.
- **Candidate B, chosen:**
  - `standard: null` with the same pinout, and pin 1 on `VDD_3V3`;
  - a DS-009 peripheral expecting 5 V on pin 1 must be powered separately;
  - so it is "the DS-009 I2C pinout", not a DS-009 port, and `ORDER.md` will say so.

**D2, acknowledgements.**
- `verify --fab-out` first refused the package for 24 unacknowledged rows (`UnwitnessedFieldNotAcknowledged`):
  - I2C2's CR1/CR2/ICR;
  - `RCC.APB1LENR.I2C2EN`;
  - GPIOB's MODER/OSPEEDR/AFRH for PB10/PB11.
- Each is acknowledged in the entry for its register and class: V0's own GPIOB entries are extended, and I2C2's mirror V0's I2C1 entries.
- **Every touched reason was then rewritten to be true of both buses** (after review): PB10/PB11 at 100 kHz and 10 kΩ, and I2C2 initialised but never used for a transfer (`let _i2c2` in the generated firmware). So CR2 stays 0 and ICR is written by no transfer. No gate reads a reason, but `FAB-NOTES.md` prints them. The board directory was re-blessed, because every generated file carries the specification's hash.
- `RCC.APB1LENR.I2C2EN`'s class is `same-source`. `validate` refused `ignored-by-model` and `silicon-only` as stale, so the ledger fixed it [measured].

**The gates, rerun on the committed file after the reasons were rewritten** [measured; identical results to the scratch candidate's run; host release CLI from `36e428b`; KiCad steps in the pinned image `598501ab` under `~/bin/capped-run`, swap 0 read back]:**

| Command | Exit | Findings |
|---|---|---|
| `validate` | 0 | — |
| `generate` | 0 | — |
| `emit` | 0 | — |
| `check erc` | 0 | none |
| `check drc --spec` | 0 | none |
| `check drc --spec --schematic` (parity) | 0 | none |
| `check support` | 0 | — |
| `extract` | 0 | I2C_SDA 43.3, I2C_SCL 46.9, EXT_SDA 40.9, EXT_SCL 39.4 pF (declared 100); SPI_SCK 16.7, SPI_MOSI 18.0 pF (declared 30) |
| `verify --fab-out` (the reference's four seeded scenarios rebased on this board; pinned image, debug build) | 0 | every gate passes, among them 8 Kani harnesses, build, simulate, attribute and the campaign. Package written: 9 Gerbers, `.drl`, `-pos.csv`, `.bom.csv`, `.elf`, `FAB-NOTES.md`, `MANIFEST.json` |

- **Outline: 45.55 × 48.70 mm** (V0: 44.10 × 48.50), two layers.
- **`verify`'s coverage precondition** names "I2C bus `EXT` (no device: nothing to fault)" and "serial port `CONSOLE` (no fault kind yet: serial)" as met by exemption. **P3 was wrong:** the refusal I predicted for EXT did not come. Its only coverage refusal was candidate A's `VDD_5V`.

**§5 adversarial, a gate that passes only because the bus has no device [measured]:**
- With a declared off-board load of 75 pF per EXT line, `extract` passes at 95.9 / 94.4 pF against 100.
- At 85 pF, `extract` exits 1, failing EXT_SDA (105.9 pF) and EXT_SCL (104.4 pF).
- The bound counts what plugs in, so a load beyond the budget fails. **P5 held.**

## 3. §1.3: where the package lives, and its CI cost

**Decision: a directory of its own, `boards/sentinel-01-bench/`** (`AETHELGARD_BLESS`-regenerated, as the reference is). The reference's `sentinel-01-minimal.json` and `boards/sentinel-01/` stay byte-identical, so P1's fixture and the quickstart do not move. The name follows `project_name`: the board crate is `sentinel-01-bench-board`. "Bench" names what V0+ adds to the reference, bench access; it is still the board Scotty orders as Sentinel-01.

**What is committed now:**
- `boards/sentinel-01-bench/` (the board crate, its firmware and its Renode platform), generated by `generate`;
- `aethelgard-svd-hal/tests/board.rs::the_committed_bench_board_crate_is_exactly_the_generated_one`, the same comparison the reference's board gets;
- `workspace_shape.rs`'s board list, now both boards (the test names `board.rs` as the list's owner);
- `bench_board_tier1.rs`, `validate` on the example. `generate` does not check acknowledgements; `validate` and `verify` do.
- **`board_tier2.rs::the_ordered_board_passes_every_board_gate_with_no_finding`** (added after review):
  - `emit`, ERC, DRC by the specification's rules with parity, `check support` and `extract`, with no finding, on `examples/sentinel-01.json`;
  - so CI judges the ordered board per pull request, not only Furnace;
  - **2.71 s** in the pinned image [measured; libtest's `finished in`, debug, offline].
  - It raises the tier-2 count by one, so **`TIER2_MIN` in `ci.yml` moves 160 → 161**, with its comment line. That is one line outside the stated fence, needed to keep it exact (§3 Acceptance).

**Measured CI cost of the board directory and its gates: seconds.**
- The tier-1 tests take 1.5–2.1 s and 0.3–0.5 s on Furnace.
- The tier-2 test takes 2.71 s in the image.

P1 held.

**Measured cost of a byte-for-byte package check: one full `verify`.** `verify --fab-out` is the only path to the package (CLI README).
- **On Furnace:** **1013 s** wall time in the pinned image (debug build, as CI's tier-2 runs it) [measured].
- **On CI:** the reference's comparable verifies took 510–579 s (`ci/colocate.tsv`, runs 43889629997 and 43895706619) [read at source].
- **P2's Furnace estimate (250–350 s) was wrong by a factor of three.** CI's figure is the one that matters.

**§7 STOP: a per-PR byte-for-byte check of the fabrication package adds more than 3 minutes to a shard: about 9–10 minutes on CI.** The package is therefore not committed in this PR. §2's package waits for the ruling below, and for `1312` in any case.

**Options for the ruling (Q1):**

| | What CI checks per PR | Cost per PR | What is given up |
|---|---|---|---|
| **(i) the package's inputs per PR; the package itself on `main` (recommended)** | Tier 1: the board crate (as now), and `emit`'s three files against the committed package's `.bom.csv` and against the hashes in its `MANIFEST.json`. The full `verify --against-fab` runs on `main`'s push, or nightly | seconds | **Per PR, this board's `prove`, `build`, `simulate` and campaign do not run, and neither does its package export.** The reference's own verifies still exercise those code paths on every PR, and the board gates (ERC, DRC with parity, support, extract) run on this board per PR (the tier-2 test above). A Gerber regression from a KiCad or exporter change would show on `main`, not on the PR. The exporter is pinned (KiCad 10.0.7, the clock-line rule), so such a change is an image or exporter PR, which runs the reference's own `--fab-out` tests. **It needs a workflow change, outside this fence** |
| (ii) `verify --against-fab` on every PR | the package, byte for byte | +510–579 s on one shard, about +6 % of CI machine time (8 shards near their 20-minute target, so about 160 machine-minutes; [inferred]); the plan must rebalance | Nothing: it is the WO's literal acceptance. It fails §7's 3-minute line |
| (iii) the bench board replaces the reference in an existing package test (`against_fab_tier2`) | the bench package, not the reference's | ±0 | Moves P1's byte-for-byte fixture off `sentinel-01-minimal.json`, which the WO forbids without the architect's agreement |

**P4 (byte-stability of two `--fab-out` runs)** is recorded in §4 when the second run finishes.

**Scenarios for the package (Q2).**
- `verify` needs scenarios whose `base` is `examples/sentinel-01.json`. Copies of the reference's four seeded scenarios, rebased, were used in scratch (`target/aet-3401-scen/`, gitignored).
- To commit the package they must be committed, under `scenarios/sentinel-01-bench/`, which is outside this work order's fence.
- Each copy differs from the reference's only in `base`.

## 4. P4, byte-stability [measured]

- A second `verify --fab-out` of the same specification and scenarios, run later in a fresh container, wrote a package byte-identical to the first.
  - `diff -r` is empty over all 15 files (9 Gerbers, `.drl`, `-pos.csv`, `.bom.csv`, `.elf`, `FAB-NOTES.md`, `MANIFEST.json`).
  - `MANIFEST.json`'s sha256 begins `732a2161` in both.
- **P4 held**, without a scenario that faults EXT (none is needed; §2). §7's third stop (a package that cannot be made byte-stable) does not fire.
- The second run overlapped the workspace gates on Furnace, so its wall time is not a cost measurement.

## 4.1 Gates [measured]

**Workflows this PR triggers:**
- `ci.yml`;
- `engine-image.yml` (`crates/**`);
- `ardupilot-image.yml` (`crates/**`, `examples/**`).

The two image workflows read only named example files (`sentinel-01-minimal.json`, `invalid-zero-domain.json`), so the new example enters neither [read at source].


- On the tree of this PR (base `36e428b`): `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0; `cargo test --workspace` 0, with 1949 passed and 0 failed. That is `main`'s 1947 plus this PR's two tier-1 tests.
- **`TIER2_MIN` is unchanged:** no tier-2 test is added.
- The new tests are unlisted in `ci/test-weights.tsv` and take `ci/shard.sh`'s default weight. Each measured under 3 s on Furnace, well inside the plan's 10 % balance. `ci/` is outside the fence, so they are not listed.

## 5. Questions

- **Q1.** §1.3's package check: (i), (ii) or (iii) above?
- **Q2.** May this work order add `scenarios/sentinel-01-bench/` (the four seeded scenarios rebased, changed only in `base`) when §2 commits the package?
- **Q3.** D1: the EXT port has DS-009 I2C's pinout with 3.3 V on pin 1, because a `VDD_5V` domain with no regulator blocks the package. Accept, or should a 5 V rail with a fault kind wait for Sentinel-03?
