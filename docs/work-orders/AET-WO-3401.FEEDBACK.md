# AET-WO-3401 — FEEDBACK

Builder: Claude (strong tier, the `3400` builder), worktree `~/aethelgard-3400`, branch `wo/aet-3401-sentinel-01`, cut from `origin/main` `36e428b`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #187) and is not committed here.

Labels: **[measured]**, **[read at source]**, **[inferred]**.

**Status (2026-11-17 ~06:20 UTC): READY for review.**
- **§1** is done and ruled (PR #190: Q1 (i), Q2, Q3).
- **§2** is built (§10–§11):
  - test points on every declared net (one GND: Q7), BOOT0 on J2 and the reset switch;
  - every gate at two layers, with `extract` passing in the order chosen by F1;
  - the package written by CI and committed;
  - `ORDER.md` and `BENCH.md`;
  - `spike/3400` deleted.
- **Open:** Q4–Q7 (§12) and the quotes in `ORDER.md` §5.
- The head is the READY commit. Nothing is pushed after it unless the architect asks. (The earlier STOPPED signal was `4891d231`.)

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

## 6. CI, the first push [measured]

- **Run 43992120410 on `ecd31f4`** (§1, with the tier-2 test and `TIER2_MIN` 161): **green**, 22 checks pass and 3 skip (`publish` ×2 and `quickstart`, as on any PR).
- **Shard times:**

  | Shard | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
  |---|---|---|---|---|---|---|---|---|
  | Time | 24m07s | 26m37s | 22m30s | 20m42s | 31m38s | 23m55s | 27m56s | 25m22s |

  `gates` took 17m20s.
- Shards over 20 minutes are a measurement under roadmap §6, not a refusal. This PR's tests add seconds, so the long shards are the runner's load that hour, not this change [inferred: every shard was over 20 minutes].

## 7. The rulings (architect, PR #190, 2026-11-17 01:50 UTC) and what they changed

- **Q1 (i).**
  - **Per PR:** `bench_board_tier1.rs::the_committed_package_is_bound_to_its_specification_scenarios_and_emit` (tier 1, no tools) checks that the committed package's `MANIFEST.json` names this specification's sha256, `solve`'s topology sha256, and the four scenarios by name and sha256 in order. Every package file is checked against its manifest hash, with no file missing or extra. `emit`'s bill of materials must equal the package's byte for byte.
  - **On `main`'s push and on `workflow_dispatch`:** `ci.yml`'s new `bench-package` job runs `verify --against-fab`.
  - With the dispatch input `write`, the same job writes the package (`--fab-out`) and uploads it as an artifact. **This is how the committed package is made.**
    - The verdict, and so `MANIFEST.json`'s `verdict_sha256`, names each gate's sandbox (CLI README, "The verdict states each gate's sandbox").
    - A package written on Furnace under rootless podman would therefore never match `main`'s Docker run in `MANIFEST.json` alone.
  - For this PR, one temporary commit ran the job in write mode on the pull request itself. `workflow_dispatch` needs the workflow on the default branch first [inferred]. The next commit removes the temporary trigger, and the job's final condition is `github.event_name != 'pull_request'`.
  - 1801 is told, in its inbox, which parts of `ci.yml` this touches.
- **Q2.** `scenarios/sentinel-01-bench/`: `pullup-3k3`, `droop-bor3`, `spi-absent-u4` and `pullup-2k2`. Each is a copy of the reference's scenario (`scenarios/sentinel-01/` and `scenarios/sentinel-01/seeded/`), **differing only in its `base` line** (`diff` shows one line per file; the line was replaced textually, not re-serialised).
- **Q3.** Accepted. The connector keeps `standard: null`, as committed. `ORDER.md` and `BENCH.md` (§2) will carry the sentence the ruling gives.

## 8. The committed package [measured]

- **`fab/sentinel-01-bench/package/`** holds 15 files, 340 KB.
  - It is the artifact `bench-package` wrote in run **43996638859** on `17af6d8`, the temporary commit, in CI's Docker environment.
  - The artifact was downloaded with the read-only token, then committed unchanged. The next commit reverted the temporary trigger.
- **Furnace's package of the same commit compared** (rootless podman, kept out of the tree):
  - every file is byte-identical except `MANIFEST.json`;
  - in `MANIFEST.json` every key is equal except `verdict_sha256`.
  - So the sandbox's line in the verdict is the only environment-dependent byte, as the CLI README states. This is why the package must come from the job.
- **The same run was red only where expected:** `gates` and tools-shard 1 each failed one test, `the_committed_package_is_bound_…` at `bench_board_tier1.rs:84`, the `MANIFEST.json` read of a package not yet committed. Everything else passed.
- **Locally, with the package committed,** `bench_board_tier1.rs` passes (2 of 2).
- **The package is the current specification's.** It must be rewritten when 1801 moves `engine_version`, and when §2 adds bench access:
  1. dispatch `ci.yml` with `write: true` on the branch (once the job is on `main`), or repeat the temporary-commit round trip;
  2. commit the artifact.

## 9. §2: predictions (committed before the §2 gate run)

The §2 specification is built (§10). It validated before this section was written; nothing else has been run on it.
- **S1.** All eight commands pass at two layers with no finding, as 1312's own fixture of this shape did (`sentinel-01-bench.json`, 13 test points).
- **S2. The outline** grows by under 10 % in area over §1's 45.55 × 48.70 mm (1312 FEEDBACK item 4).
- **S3. `extract`:** the I2C bounds rise by up to about 18 pF (1312 measured 43.3 → 61 pF on I2C_SDA), still under 100 pF.
  - **The SPI lines are the risk:** declared at 30 pF, they bound at 16.7/18.0 pF today.
  - I predict SPI_SCK and SPI_MOSI stay under 30 pF (60 %).
  - If either does not, the declared bound is not raised to pass (§5's rule): the finding is recorded and the test pad's placement is the engine's to fix.
- **S4.** `verify --fab-out` passes, and the package grows by the test pads' and the switch's rows in `-pos.csv` and `.bom.csv`.
- **S5. The emitted BOM has value and footprint only for the new parts.**
  - The test pads have no maker part: a bare pad, `TestPoint_Pad_D1.0mm`.
  - The switch has its library footprint, and its maker part is the one 1312 cited.
  - The BOOT0 pull-down is a 10 kΩ 0402.

## 10. §2: bench access, measured

**`main` merged** (`c3a3d711`, from `9a37ae6`). It brings:
- 0.2.0 (#188);
- 4220b and its floor (#192, #193): specification 13, `pin_capacitance`, parts schema 7;
- 1312 (#191): specification 14 with `test_points`, BOOT0 on a connector and `mcu.reset_switch`.

There were two conflicts:
- **`ci.yml`'s `TIER2_MIN`:** main's 162 (4220b and 1312) plus this PR's one, so **163**, with its comment line.
- **`board_tier2.rs`:** both sides appended whole tests. Main's file is taken, and this PR's test is re-appended unchanged.

**The specification (`examples/sentinel-01.json`, version 14):**
- `pin_capacitance: null` on U2–U4, as the reference has;
- `mcu.reset_switch: true`;
- BOOT0 on J2 pin 2 (NRST stays on pin 3);
- **14 test points:** every declared net §2.1 lists (both buses' SDA/SCL, SPI SCK/MOSI/MISO, FLASH_CS, CONSOLE TX/RX, VDD_3V3, NRST, BOOT0), plus one GND.

**One gap: a second GND test point cannot be declared.**
- `validate` refuses it: `TestPointNetRepeated` ("net `GND` carries two test points … one is enough") [measured].
- The other ground points are J1–J4's GND pins. `BENCH.md` says so.

**The gates, at two layers [measured; the merged release CLI; KiCad in the pinned image]:**
- All eight commands exit 0, with no finding in ERC, DRC, parity or support.
- `extract`: I2C_SDA 58.6, I2C_SCL 53.4, EXT_SDA 48.6, EXT_SCL 47.5 pF (declared 100); SPI_SCK 18.6, SPI_MOSI 24.6 pF (declared 30).
- Outline **46.35 × 54.85 mm**: 25.4 cm², +14.6 % over §1's 22.2 cm².

**Finding F1: `extract`'s margin depends on the test points' document order.**
- **The first order** (the list in §2.1's order, TP1 = I2C_SDA … TP14 = GND) failed `extract`: SPI_MOSI was bounded at **30.74 pF against 30**. It had 20.6 mm of track and 8 vias, and every other gate passed.
- Re-ordering only the list moved the bounds [measured, emit and extract on the host]:

  | Order | SPI_SCK | SPI_MOSI | Worst I2C |
  |---|---|---|---|
  | as listed | 18.9 | **30.7 (fail)** | 56.3 |
  | MOSI first | 26.5 | 20.8 | 56.7 |
  | SPI group first | 19.5 | 28.2 | 50.4 |
  | **reversed (adopted)** | **18.6** | **24.6** | 58.6 |

- **The adopted order clears every line with at least 5.4 pF of margin.**
- Document order is a free choice, so adopting it relaxes nothing. No bound was raised, and no listed test point was dropped.
- Two other ways to pass were measured and **not** taken:
  1. SPI's declared 30 pF raised to 35 (passes, but it moves a V0 field that also chooses the pin speed);
  2. no test point on SPI_MOSI (passes, with MOSI at 16.9 pF).
- **For the architect (Q4):** the placer's dependence on `test_points`' order is a robustness gap of the same family as 1307's name-dependence. A board that passes by one ordering and fails by another is decided by an input nobody chose for electrical reasons. The renaming check (1307) does not cover document order.

**Predictions scored (§9):**
- S1 held, once the order was chosen; as first written, `extract` failed.
- **S2 wrong:** the outline grew 14.6 %, not under 10 %.
- **S3 wrong as to SPI:** MOSI failed in the first order.
- S4 and S5: §11.

**`spike/3400/` deleted** (ruled on #192 Q4 and #191 Q3): 527 files, the merged 3400 record. Nothing outside `docs/` named it, and the dated prices `ORDER.md` uses are cited there to `AET-WO-3400.FEEDBACK.md`, which stays.

**`ORDER.md` and `BENCH.md`** are in `fab/sentinel-01-bench/`, beside `package/`, never inside it, because `--against-fab` refuses extra files.
- Each carries the ruling's sentence on EXT verbatim.
- **BENCH:**
  - every rise-time row adds the probe's `C_p` to `extract`'s bound (#191 Q2);
  - the BOOT0 row drives J2 pin 2 through R8, the judged 10 kΩ, with DS12117 Table 49's VIH min of 1.161 V and a margin of 2.14 V at 3.3 V (1.74 V at VDD − 0.4 V);
  - the BOR row's pass lines are Table 115's VBOR3, falling 2.54–2.68 V and rising 2.63–2.78 V (rev V, p. 212; rev Y's Table 16 prints the same; both read at source);
  - the threshold row times SCLH against `TIMINGR` `0x50D32A40` at the solved 64 MHz kernel clock (4.031 µs);
  - USART boot's protocol line is labelled a gap, because AN2606 is unread.
- **ORDER:**
  - one row per BOM line, maker parts checked against the emitted footprints;
  - the generic 0402 passives flagged ⚑, with priced examples;
  - OSH Park at US$19.71 per 3 [computed from the read rate];
  - SGD 54.64? for 33 of 39 rows;
  - the open quotes.
- **Finding F2:** the package's BOM and placement files are KiCad's and the emitter's formats, not a fabricator's upload format (JLCPCB's columns differ, and its rotation offsets are its own). That is engine work if wanted.

**Finding F3: `FAB-NOTES.md` does not list the acknowledged ledger rows** that §2.2 names.
- The generated notes carry the stackup, the holes, the rule minima, the MCU support network, the lifecycle banner, the files, the licence and the brown-out procedure (`BOR_LEV` by read-modify-write of `OPTSR`).
- They do not carry the `acknowledge_unwitnessed` rows: none of "ledger", "witness" or a class name appears in it [measured, on the final package].
- The rows live in the specification (each with its reason) and in `verify`'s ledger; `BENCH.md` B2 reads them from there.
- Writing them into `FAB-NOTES.md` is `aethelgard-eda`'s `fab.rs`, outside this fence, so it is not done here (Q5).

**The final package on Furnace [measured]:** `verify --fab-out` on the final specification passes every gate:
- 8 Kani harnesses, build, simulate, attribute and the campaign;
- coverage met by exemption for EXT (no device) and CONSOLE (no serial fault kind);
- 935 s wall time; engine 0.2.0; 15 files.

The placement file lists SW1 and no test pad, which is right: a bare pad places no part. The committed package is CI's (§8's rule), not this one.

## 11. §2: the package, the adversarial pass, the reduction pass

**The committed package [measured].**
- `fab/sentinel-01-bench/package/` (15 files) is the artifact of `bench-package`'s write run **44025019904** on `f6553b10` (the second temporary commit, reverted by the next).
- It was committed unchanged. Its manifest names this specification, its topology and its verdict (sha256 prefixes `e1e4f6a53ea0eadb`, `5f818dbf1fb82dc5`, `545eceebf58844e0`). The manifest's own sha256 begins `af420f89e9bd5bb2`.
- **Furnace's package of the same tree** is byte-identical except `MANIFEST.json`, and in that file only `verdict_sha256` differs: the sandbox line, as in §8.
- The per-PR binding test passes against it.
- **S4 held:** the placement and BOM files gained SW1 and R8 (BOOT0's pull-down). The test pads place no part.
- **S5 held:** the BOM has value and footprint only for the generic passives; SW1 carries 1312's part, PTS810 SJM 250 SMTR LFS.

**Adversarial pass (work order §5, targeted):**
- **A gate that passes only because the new bus has no device:**
  - in §1, a declared 85 pF off-board load on EXT failed `extract` and 75 pF passed;
  - on the final §2 board, 85 pF fails (113.6/112.5 pF) and 70 pF passes (98.6/97.5).
  - The copper with its test pads now leaves about 71 pF per line before the declared 100 pF. `BENCH.md` states EXT's 20 pF off-board allowance and this budget.
- **A BOM line whose maker part does not match the footprint emitted:** none (`ORDER.md` §2: each package suffix checked against the footprint the emitter wrote). The generic passives have no maker part to mismatch, and they are flagged ⚑.
- **A bench row whose pass line cannot fail:**
  - B1–B8 each name a reading that can fall outside it: attach, register words, `BOR_LEV` bits, `MODF` and `EF 40 17`, the VBOR3 windows, the rise-time upper bounds with `C_p` added, `V_th` within 1.7 ± 0.2 V, and BOOT0's 0.727 V / 1.161 V lines.
  - B6 has an upper bound only: a rise faster than the bound does not falsify `extract`, which is an upper bound itself.
  - B8's bootloader-protocol line is labelled a gap (AN2606 unread), so it is not claimed as a pass line.

**Reduction pass (diff-sized):**
- `bench_board_tier1.rs` keeps one run helper and one hash helper; its two tests share them.
- The svd-hal comparison is one new test reusing `compare`.
- The tier-2 test is one function.
- `ci.yml` gains one job and one trigger input.
- `ORDER.md` and `BENCH.md` repeat only the ruling's EXT sentence, which the ruling asks for in both.
- `spike/3400` is removed (527 files).
- Nothing else is removable without losing a rerun path or a ruled requirement.

## 12. Questions for the architect

- **Q4 (F1).** Should the placer's dependence on `test_points`' document order become a backlog item, beside 1307's name independence? Measured: SPI_MOSI's bound moves between 20.8 and 30.7 pF by order alone.
- **Q5 (F3).** `FAB-NOTES.md` does not list the acknowledged ledger rows §2.2 names. Should a work order in `aethelgard-eda`'s `fab.rs` add them? Until then `BENCH.md` B2 reads them from the specification.
- **Q6 (F2).** Should the package also carry the fabricator's BOM and placement formats? KiCad's and the emitter's are in it today.
- **Q7.** A second GND test point is refused (`TestPointNetRepeated`). Accept J1–J4's GND pins as the other ground points, or should 1312's rule admit a repeated GND?

## 13. Gates at READY [measured]

On the tree of this READY commit (main `f57d9b6` merged):
- `cargo fmt --all -- --check` 0;
- `cargo clippy --workspace --all-targets -- -D warnings` 0;
- `cargo test --workspace` 0, with 2002 passed and 0 failed;
- in the pinned image: `cargo clippy … --features aethelgard-cli/with-tools -- -D warnings` 0, and `board_tier2.rs::the_ordered_board_passes_every_board_gate_with_no_finding` passes on the §2 board (3.24 s);
- `TIER2_MIN` is 163: main's 162 plus this PR's one.

The temporary commit's run (44025019904) was red only on the package-binding test, the expected stale package (the architect's inbox note, 05:38 UTC). CI on this head follows the push.
