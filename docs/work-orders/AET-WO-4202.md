# AET-WO-4202 — The BME280 leaves Sentinel-02 and -03: U2's role goes to the BMP581, the twin's BMP581-through-brown-out fault is fixed, and Sentinel-01 stays as ordered

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` **after `AET-WO-4400g` merges** (#238; open at writing, head `46a44a31`). `main` was `56639a89` when written, without `4400g`. Name the post-`4400g` HEAD in the kickoff; verify live HEAD.
**Branch:** `wo/aet-4202-bmp581-u2`.
**Author:** Claude (architect) · **Builder:** strong tier, the `4201` builder reused, with a targeted adversarial pass.
**Phase:** P4 parts, before Sentinel-02 or -03 is ordered. Written 2026-11-19. **Cadence: build.** Re-measure the twin fault before any edit. **Any specification, parts-table or trace schema change is a §7 stop.**

**Grounds:**
- **The part cannot be bought.** The BME280 showed 0 at every authorized US distributor on 2026-11-17, with a 16-week lead (roadmap §7.1 L838).
- **The spike** (`AET-WO-4201`, PR #237, branch `wo/aet-4201-bme280-second-source`; `docs/work-orders/AET-WO-4201.FEEDBACK.md` there) [read]:
  - The BME280 does two jobs. It is the I2C identity witness (`0xD0` = `0x60`). It is an ArduPilot barometer (`BARO BMP280`). **Its humidity is read nowhere** (FEEDBACK §1, L47-56).
  - On Sentinel-02 and -03, U2 is a **second** barometer beside U7's BMP581 at 0x46. Nothing in the repository asks for two (L64-69).
  - **Measured:** with U2 a BMP581 at 0x47, or with U2 removed, Sentinel-02 and -03 pass `validate`, `solve` and `emit` (§4.1, L182-187; `spike/4201/out/dry-runs.txt`). `generate hwdef` writes `BARO BMP581 I2C:0:0x47` beside U7's `…0x46` (L193).
  - **Not measured there:** ERC, DRC, `check support`, `extract`, the supply gate and the outline on Sentinel-03; an ArduPilot build or foreign run with U2 changed (L274, "the ArduPilot ELF was not rebuilt here").
  - **The spike's variants kept U2's `power.loads` row** (`spike/4201/tools/variant.py` edits `i2c_devices` only). Its "U2 removed" Sentinel-03 still declared a 1 mA load for U2, cited to the BME280. `validate` passed it (next bullet).
- **Scotty's choice** (#237, comment 7013295272) [read]:
  - The BMP581, "the only candidate he found in stock."
  - His DigiKey US read, 2026-11-19 ~15:10 UTC: 828-BMP581CT-ND, **53,288 in stock**, US$3.55 q1 (US$3.026 q10), Active, 16-week standard lead.
  - The two-distributor stop of `4201` §7 is waived for this choice, and the waiver is recorded in that comment.
  - The comment also puts the Sentinel-02/-03 choice (U2 a BMP581, or unpopulated) into this order. It puts the twin fault in scope too.
- **Sentinel-01 is ordered** with U2's BME280 LGA-8 2.5 × 2.5 mm pads (`fab/sentinel-01-bench/ORDER.md` L36). A BMP581 (LGA-10 2.0 × 2.0 mm) does not fit them. `AET-WO-3403` owns Sentinel-01's bench: U2 left empty, and a BMP581 breakout on J4 through a bench-only specification (`docs/work-orders/AET-WO-3403.md` L39, L45-57). **This order does not touch Sentinel-01 (§3).**
- **The twin fault** (FEEDBACK §4.2, L204-232; `spike/4201/out/campaign-controls.txt`) [measured there, read here]:
  - On a scratch tree, Sentinel-01 as ordered with U2 a BMP581 at 0x46. The bench campaign is refused on `droop-bor3` alone:
    > `AETHELGARD-INJECT 0 0 1952227 1962227 28a801 770`: its count places it at 20854010 ns in a boot started at 1231740 ns, but it is stamped 20854009 ns
  - Unchanged tree: 2/2 pass. Swap: 2/2 refused on the same line. `serial-rx-open-console` alone passes.
  - **The judge** (`crates/aethelgard-cli/src/renode.rs` on `main`):
    - `hooks_of`, L2535-2571;
    - a scheduled line must be stamped exactly at `start_ns + 10 ns × count` (L2551-2553, `anchor::instant`, `anchor.rs` L265);
    - one stamped earlier is refused, "no core runs ahead of its time" (L2563-2565).
    - On the `4400g` branch the same function sits at L2533.
  - **The judge's boot start is modelled, not read:**
    - it is the last reset's `released_ns + NUDGE_NS` (`renode.rs` L2027-2032);
    - `NUDGE_NS` is 10 (`crates/aethelgard-chaos/src/power.rs` L81-83): "A release takes effect only at the end of the next `RunFor`" (`2301` F5, measured with a BME280 on the bus);
    - the rendering is `machine Reset … cpu IsHalted true` and then `cpu IsHalted false; emulation RunFor 10 ns` (L92-93).
  - **Our BMP581 model carries its own timer** (`models/renode/BMP581.cs`):
    - a 1 GHz `LimitTimer`, one-shot, `autoUpdate` (L48);
    - reset in `Reset()` (L61-69) and re-armed to the next event (L358-366);
    - `Now()` calls `cpu.SyncTime()` from inside a bus access (L256-263).
    - Its sha256 is pinned in `crates/aethelgard-twin/src/model.rs` `OWN` (L61-72).
  - **`4400g` does not touch** `hooks_of`, `anchor.rs` or `NUDGE_NS` [read, `git diff main...46a44a31`]. It does rebuild the timers and USARTs at the solved 64 MHz, and the failing line is a serial injection. **So whether the fault survives `4400g` is unknown.** §1.1 measures it first.
  - **Sentinel-02 on `main` through the same droop** (scenario base retargeted, `spike/4201/scenarios/s02-droop-bor3.json`): no stamp refusal, but `brownout_unrecovered` against an expectation borrowed from Sentinel-01 (FEEDBACK L228). Neither confirmed nor ruled out as the same fault.
- **Every committed specification with a BME280** [measured, `git grep -l -i bme280 main -- '*.json'`, outside `spike/`]. The table's last column is the disposition (D2):

  | Specification | U2 | Beside it | This order |
  |---|---|---|---|
  | `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` (Sentinel-02; CI's ArduPilot board) | BME280 0x76 | U7 BMP581 0x46 | **moves** |
  | `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` (Sentinel-03; power tree, `power.loads` row for U2 at L1221-1226) | BME280 0x76 | U7 BMP581 0x46 | **moves** |
  | `crates/aethelgard-hwdef/tests/fixtures/sentinel-02.topology.json` and `sentinel-02.hwdef.dat` (L26-27) | derived from `sentinel-02-own` | | **regenerated** |
  | `crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json` (`2302`'s motor fixture; `1314` D4) | BME280 0x76 | U7 BMP581 0x46 | stays (D2) |
  | `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` (the power fixture; ICM-20948 stand-in, no U7) | BME280 0x76 | none | stays: in `aethelgard-ir`, fenced (§3) |
  | `examples/sentinel-01.json`, `examples/sentinel-01-minimal.json`, `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json`, hwdef `sentinel-01.*` | BME280 0x76 | | **fenced** (Sentinel-01) |
  | `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer`, `four-layer-power`, `imu-baro-1v8`, `motors-dshot600`, `motors-pwm`, `pair-standin`, `serial-dshot-9600`, `serial-four`, `two-bme` (cli fixtures); hwdef `serial-dshot-9600.*`; pincheck `common/topology.json`; svd-hal `ui/motors-pwm/topology.json`; `examples/invalid-zero-domain.json`; `scenarios/sentinel-01/seeded/cases.json` | BME280 | | stay: engine fixtures. **The BME280 stays a supported part** (row, model, symbol, footprint) |

- **What the engine already has for the BMP581** [read]:
  - the parts row, schema 10 (`crates/aethelgard-ir/data/parts.json` L113): identity `0x01` = `0x50`; the script `0x36 ← 0x40 0x81`, 10 ms settle, data from `0x1D`; ArduPilot driver `BMP581`;
  - the schematic arm, whose SDO is strapped from the address (GND for 0x46, the rail for 0x47), with a second 100 nF (`crates/aethelgard-eda/src/schematic.rs` L1346-1373, L1398-1401);
  - the hwdef `BARO` line per barometer (`crates/aethelgard-hwdef/src/lib.rs` L686, L758);
  - the model (`models/renode/BMP581.cs`).
  - **No parts row, symbol, footprint, model or generator change is needed** (FEEDBACK §4.4, L266-274).
- **A load's `reference` is checked for form and uniqueness only.** Nothing checks that it names a declared part (`crates/aethelgard-ir/src/power.rs` L821-838) [read]. A load row left behind for a removed U2 passes `validate` silently. This order removes the row by hand and tests for it. The rule itself is not built here (§3); it is a finding.
- **Who reads `sentinel-02-own`** [read]:
  - CI's ArduPilot jobs: `generated (sentinel-02)` and `-noimu` (`.github/workflows/ardupilot-image.yml` L256-268, `cmp` at L290), `pinout`, and `foreign` (`ci/foreign-judge.sh` L16).
  - `foreign-judge.sh`'s positive check names U2's identity and `data (reads[0])` (L54-55 on `main`; L83-84 on `4400g`). A mutant's check names U2's identity (L90 on `main`; L141 on `4400g`).
  - Tier-1 and tier-2 tests: `twin_tier2.rs` L3618-3633 (`own_board`) and L3806 (`sentinel-02-own-base`); `models_tier2.rs` L33; `own_models_tier1.rs`; `own_board_tier1.rs`; `ledger_tier1.rs` L269, L292; `hwdef_tier1.rs` L23; `orientation_tier1.rs`; `with_tools.rs` L1618; `aethelgard-ir/tests/orientation.rs` L12 (reads the file; ir is fenced, so the file must still parse there).
- **Who reads `sentinel-03`** [read]: `board_tier2.rs` L1710-1747; `outline_tier1.rs` L19; `supply_tier1.rs` L20; `via_ring_tier1.rs` L21.
  - Sentinel-03's `generate hwdef` was refused at `c3f8ddbb` (`SenseNotEmitted`, `VBAT_SENSE`; `4201` F2). That refusal does not depend on U2.

> **What this is:**
> - U2 on Sentinel-02 and -03 changed by D1: removed (the default) or a BMP581 at 0x47. Every artifact of those two boards is regenerated from the specification, and every moved figure is re-pinned from a measured run;
> - the twin's BMP581-through-brown-out stamp fault re-measured on post-`4400g` `main`, root-caused, fixed where the cause is (the twin, or our model), and guarded by a regression test with a killed mutant;
> - Sentinel-01, its package and every other board left byte-identical.
>
> It is NOT:
> - **any change to Sentinel-01:** its specifications, board, firmware, `.repl`, package, `MANIFEST.json`, `ORDER.md`, `BENCH.md` or scenarios. `3403` owns its bench;
> - a specification, parts-table or trace schema change. `4240v` and `2304b` are bumping the specification schema now;
> - removing the BME280 from the engine. Its row, model, symbol, footprint and every fixture that uses it stay;
> - `4201` F5 (`3303`'s coverage rule counts a bus with no modelled device as covered). That goes to the next coverage order (§7.1 L843);
> - the orphan-load rule in `aethelgard-ir` (Grounds). A finding, not built;
> - a second maker for the barometer (DPS368, `4201` §7.2), or a humidity requirement;
> - Sentinel-02's `brownout_unrecovered` under a borrowed scenario, unless §1.1 shows it is the same fault;
> - an edit to `3304`'s bench sheet (`docs/work-orders/AET-WO-3304.bench.md`). If a twin-column figure there moves, it is a finding.

## Ruled at writing (the architect, 2026-11-19: D1 is R and D2 holds as drafted)

**Sequencing:**
- `4404` (#244) merges before this order's READY. It moves no `hwdef` golden on its own.
- After it lands, merge `main` and re-pin `sentinel-02.hwdef.dat` and the other hwdef fixtures from a measured run.
- `1315` (#243) re-pins the same Sentinel-03 figures. Whichever merges second re-pins from a measured run.

**A carve-out from `1911`'s `ci/` fence:** only the lines of `ci/foreign-judge.sh` that name U2 (main L54–55 and L90). Say so on the bus before you push them.

**The finding that `validate` does not check that a `power.loads` reference names a declared part** (`ir/src/power.rs` L821–838) is recorded, not fixed here. The architect opens a §7.1 row. Remove U2's load row by hand, and test that it is gone.


- **D1. U2 on Sentinel-02 and -03: removed (R), the default.** The alternative is a second BMP581 at 0x47 (K).
  - **Why R:**
    1. Nothing in the repository asks for two barometers. `4210` chose "the IMU and barometer", singular (`4201` FEEDBACK L69, grep measured).
    2. A second BMP581 on the same bus, rail and board area is redundancy without diversity (`4201` §7, L308). A bus lock-up, a rail fault or a lot defect takes both. It covers one die failing, and nothing else.
    3. If barometer redundancy is wanted, it should be a deliberate choice of a second maker (DPS368, `4201` §7.2), in its own order. It should not ride in on a shortage.
    4. It is the smallest change. It takes two unmeasured paths off this order's critical path: ArduPilot with two BMP581s, and the SDO-high strap at 0x47 on real copper.
    5. The boards keep a modelled I2C witness (U7), so `4201` F5's blind spot does not open on them.
    6. One fewer LGA-10 and two fewer 100 nF on Sentinel-03, which must fit 100 × 100 mm.
  - **What R gives up:** ArduPilot's barometer failover between two dies, and an engine path with two parts of one kind on one bus at the SDO-high address. `two-bme.json` already exercises that path for the BME280 at 0x77.
  - **Why K is defensible:** ArduPilot supports two barometers, and K is also measured clean through `emit` and hwdef. Flip to K if the architect wants failover now and accepts the unmeasured ArduPilot run as this order's risk.
  - **Under R, U2 is retired on these boards.** No other part takes the designator in this order.
- **D2. The move set is the two boards that will be ordered:** `sentinel-02-own` and `sentinel-03`, and the hwdef fixtures derived from `sentinel-02-own`.
  - `sentinel-02-shaped` stays as it is. It is `2302`'s motor fixture, and its BME280 does what every other engine fixture's does.
  - `sentinel-02-power` stays as it is. It is in `aethelgard-ir`, fenced here, and it is a power-path fixture with an EOL stand-in IMU that no one orders.
  - Flip: also move `sentinel-02-shaped`. `sentinel-02-power` cannot move in this order.

## 1. Build

0. **Predict, then build.** Before the first run, commit a short prediction table to FEEDBACK, each answer labelled measured, read or inferred (as `4201` did):
   - whether the stamp fault reproduces on post-`4400g` `main`, and at which line;
   - its cause, as a hypothesis (below);
   - which committed hashes and pinned figures move: Sentinel-03's outline and least `extract` slack; its supply-gate figures; the `sentinel-02` hwdef lines; the foreign judge's witness times and window margin; each campaign `state_sequence_hash`;
   - how many tier-2 tests you add.
   - **Hypotheses to test, all inferred, none traced:**
     - **H1.** The model's timer puts a sync point inside the 10 ns nudge. The release then takes effect at a different instant from `released_ns + NUDGE_NS`, and the restarted count's origin moves by 1 ns.
     - **H2.** `cpu.SyncTime()` in `Now()`, called during a transfer, splits the quantum. That moves where the stamp falls relative to the count.
     - **H3.** The model's `Reset()` leaves timer state that re-arms at the reset, which `machine Reset` reaches before the CPU halts.
     - Test H1 first: it predicts the fault for **any** peripheral of ours with its own timer. So run the ICM-42688-P model (also ours, also timed) through the same reset as well.
1. **Re-measure the twin fault, before any edit** (`4201`'s method, `spike/4201/tools/`; scratch trees outside the repository, never committed):
   - Build post-`4400g` `main`'s CLI. Make a scratch copy of `examples/sentinel-01.json` with U2 a BMP581 at 0x46.
   - Run the five bench scenarios through `campaign` twice. Then run the unchanged tree twice as the control. Then run `droop-bor3` alone.
   - Run `s02-droop-bor3` on post-`4400g` `sentinel-02-own`, unchanged. Then again with D1 applied.
   - Use `capped-run --mem 16G`: `prove` is OOM-killed at 8 GiB (`4201` F4).
   - **Root-cause it from evidence:** the Renode log around the release; the instants of the model's timer events; the CPU's count at the release. Write the cause in FEEDBACK with the lines that show it. A cause labelled inferred is not a root cause.
   - **If the fault no longer reproduces,** find which `4400g` change moved it, and show that with a run. If that is shown, write the regression test anyway (step 2) and continue. If it vanished and no one can say why, stop (§7).
2. **Fix it where the cause is, and guard it.**
   - **If the cause applies to any timed peripheral of ours (H1, or the ICM-42688-P shows it too), fix the twin:** the reset's rendering or the judge's model of the release (`chaos/src/power.rs`, `cli/src/renode.rs`). Do not fix the BMP581 model alone. A fix in one model leaves the next model of ours to rediscover it.
   - **If the cause is the model's alone, fix `models/renode/BMP581.cs`,** and move its pin in `aethelgard-twin/src/model.rs` `OWN` with it. Datasheet facts stay cited as they are.
   - **The judge stays exact.** A scheduled line is still stamped at exactly `start_ns + 10 ns × count`. A count ahead of its stamp is still refused. A tolerance, or a `±1 ns`, is not a fix (§7).
   - **The regression test:**
     - it fails on the unfixed tree (show the failing output in FEEDBACK) and passes on the fix;
     - it uses a test-local derived specification, written to a temp directory at test time: Sentinel-01 with U2 a BMP581, and/or a board with the timed model through the reset. It is never a committed Sentinel-01 file;
     - its expectation is the scenario's own `expect_counts`, not the run's output (`INV-07`);
     - add a tier-1 test of the mechanism where one can reach it, so the guard is not only a slow tier-2 run;
     - name the mutant that reverts the fix, and the test that kills it;
     - run the fixed run's trace through replay twice. The state sequences must be identical (`INV-04`).
3. **Apply D1 to the move set.** Edit the specifications only. Every output is regenerated (`INV-01`).
   - **R:**
     - remove U2 from `i2c_devices` in both specifications;
     - remove U2's `power.loads` row from `sentinel-03` (L1221-1226). Search both files for any other mention of U2 and remove it;
     - regenerate `sentinel-02.topology.json` and `sentinel-02.hwdef.dat` with the post-`4400g` CLI. One `BARO` line remains: `BARO BMP581 I2C:0:0x46` (U7).
   - **K:**
     - U2 becomes `BMP581` at 0x47 (71), with `pin_capacitance` as U7 declares it, naming U2;
     - on `sentinel-03`, U2's load row is re-cited as U7's is (`BST-BMP581-DS004-13` Table 4 p. 12, 260 µA max, 1 mA rounded up). The BME280 citation must not survive;
     - the hwdef holds `BARO BMP581 I2C:0:0x47` (U2) and `BARO BMP581 I2C:0:0x46` (U7), in the generator's order.
4. **Every reader of the moved specifications follows:**
   - the tests and goldens in Grounds;
   - `ci/foreign-judge.sh`'s checks that name U2. Under R they drop U2. Under K, U2's step becomes its BMP581 script, as U7's is (`data (script)`);
   - the crate READMEs that state a moved fact (`crates/aethelgard-cli/README.md` L406, if a figure moves).
   - **Change each check to the new truth. Never delete one to make it pass** (§5).
5. **Every moved figure is re-pinned from a measured run, cited to it.** List each in FEEDBACK with before and after:
   - placement, outline and `extract` slack on Sentinel-03;
   - supply-gate figures;
   - foreign witness times and the window's margin;
   - each campaign hash;
   - `TIER2_MIN`;
   - any `ANSWER` line.

## 2. Acceptance

- **Sentinel-01 and its package are byte-identical to post-`4400g` `main`:**
  - run `main`'s binary and this branch's over `examples/sentinel-01.json`, `examples/sentinel-01-minimal.json` and `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json` (`solve`, `emit`, `generate`, `generate hwdef`). Compare unmasked;
  - `aethelgard verify … --against-fab fab/sentinel-01-bench/package`, with the scenarios `ci.yml`'s `bench-package` job names, reports `identical`. Run it once yourself in the image, and cite CI's run;
  - `fab/sentinel-01-bench/package/MANIFEST.json` does not move. **Every Sentinel-01 scenario's campaign `state_sequence_hash` is unchanged.** A twin fix that moves one moves `verdict_sha256` (`1605c`). That is a §7 stop.
- **Every other committed specification is byte-identical in `solve`, `emit`, `generate` and `generate hwdef`.** That covers every one outside the move set, with or without a BME280, and every board without one.
  - Every committed scenario's campaign hash is unchanged, except a scenario based on a moved specification, or one with a BMP581 when the fix is in the model. Each moved hash is named, with its cause.
- **Sentinel-03** (move set) passes, inside its 100 × 100 mm bound and with no finding: `validate`, `generate`, `emit`, ERC, DRC with and without parity, `check support`, `extract` and the supply gate (`board_tier2.rs` L1705, `the_sentinel_03_fixture_passes_every_gate_inside_its_bound`). `generate hwdef` is refused as at baseline (`SenseNotEmitted`, `VBAT_SENSE`) and by that name only, unless post-`4400g` `main` already says otherwise.
- **Sentinel-02** (`sentinel-02-own`):
  - its hwdef is held by `hwdef_tier1` and by CI's `cmp`;
  - unmodified ArduPilot builds it (`generated (sentinel-02)`);
  - `check pinout` exits 0 (`pinout`);
  - **the foreign judge passes inside its window** with every modelled device's identity and data witnessed (`foreign`). The window is never widened.
- **Under R:**
  - U2 appears in no output of either board: schematic, board, BOM, `.repl`, generated firmware, hwdef, verdict;
  - no `power.loads` row names U2.
- **Under K:**
  - U2's SDO is on `VDD_3V3` and U7's on `GND`, read from the copper (`RUL-03`: consumers derive nets from copper);
  - the generated firmware carries U2's BMP581 script (`0x36 ← 0x40 0x81`, 10 ms, data from `0x1D`);
  - the twin witnesses U2's identity (`0x50` at `0x01`) and its data;
  - U2's load row cites the BMP581;
  - the BOM gains U2's two 100 nF.
- **The twin fault:**
  - re-measured first, root-caused from evidence and fixed;
  - the regression test fails on the unfixed tree and passes on the fix;
  - a mutant that reverts the fix is killed;
  - replay is deterministic;
  - the judge's exactness is unchanged (`renode.rs` `hooks_of`: `stamp == at` for a scheduled line).
- **`examples/sentinel-02-bench.json`** (`3304`) still passes `verify`. If its campaign hash or any twin figure moves, it is named.
- **The CLAUDE.md gates pass. CI is green. `TIER2_MIN` is exact.**
- **`1910`'s planner guards hold:** any new test libtest reports over 60 s has a weight row, which `1911` lands (`ci/shard.sh` `heavy_unweighted`, L211-252). No stale row.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-04`, `INV-06`, `INV-07`; `RUL-03`, `RUL-05`, `RUL-12`, `RUL-14`, `RUL-16`, `RUL-20`.
- **Fence (in):**
  - `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` and `sentinel-03.json`: U2 and its load row only;
  - `crates/aethelgard-hwdef/tests/fixtures/sentinel-02.topology.json` and `sentinel-02.hwdef.dat`, regenerated, never hand-edited;
  - the fix's site, named in FEEDBACK. **Either** `models/renode/BMP581.cs` and its pin in `aethelgard-twin/src/model.rs` `OWN`, **or** the reset and anchor path: `cli/src/renode.rs` (`hooks_of`, the boot start), `chaos/src/power.rs` (the release rendering), and `aethelgard-twin` only if the rendering lives there;
  - tests and goldens that read the two moved specifications or pin the fixed path;
  - `ci/foreign-judge.sh`'s checks that name U2. This is a named carve-out from `1911`'s `ci/`: say it on the bus before you push;
  - crate READMEs that state a moved fact; FEEDBACK.
- **Not:**
  - **Sentinel-01, all of it:** `examples/sentinel-01.json`, `examples/sentinel-01-minimal.json`, `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json`, `boards/sentinel-01/**`, `boards/sentinel-01-bench/**`, `fab/sentinel-01-bench/**` (the package, `MANIFEST.json`, `ORDER.md`, `BENCH.md`, `FAB-NOTES.md`), `scenarios/sentinel-01*/**`, hwdef `sentinel-01.*`;
  - **`aethelgard-ir`, every file:** `src/`, `data/parts.json`, `tests/fixtures/sentinel-02-power.json`. That includes the orphan-load rule;
  - `aethelgard-eda` and `aethelgard-hwdef`'s `src/` (no generator change is needed: Grounds);
  - `.github/workflows/`; `ci/` beyond the carve-out (`SHARDS`, `test-weights.tsv`, `shard.sh`, the `TIER2_MIN` line); `container/` (a Renode change is §7); the pincheck fixtures and `container/ardupilot-pinout.sh`;
  - `examples/sentinel-02-bench.json`, `docs/work-orders/AET-WO-3304.bench.md`;
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`. Findings about them go in FEEDBACK.
- **Running beside you:**
  - **`4240v` and `2304b`** are bumping the specification schema. **You take no schema version.** Their bumps edit the `schema_version` line of every fixture, including your two. Whoever merges second merges `main` and re-derives the goldens.
  - **`1911`** owns CI: `ci/`, the workflows, shard weights and `TIER2_MIN`. Say on the bus before you add a tier-2 test or touch `foreign-judge.sh`. The weight row and the `TIER2_MIN` change are coordinated with `1911`, never made alone. Merge `main` before READY.
  - **`1315`** (#243, READY) draws Sentinel-03's hot-plug capacitor and re-pins Sentinel-03's placement, `extract` and supply figures, the same pins you move. If it has merged, its figures are your base. If not, say so on the bus. The second to merge re-pins from a measured run.
  - **`4404`** (#244, open) requires every committed hwdef golden byte-identical, and stops if one moves. You move `sentinel-02.hwdef.dat`. Tell its builder on the bus. If `4404` merges first, merge it and regenerate. If you merge first, `4404` merges `main`, and its stop does not fire for your line (the architect rules it in the kickoff).
  - **`3403`** owns Sentinel-01's bench (U2 empty, a BMP581 on J4 through a bench-only specification). Nothing of yours touches it, and nothing of its touches yours.
- One PR, left unmerged. Write `~/.cache/aet-4202/pr-title-body.md`. On Furnace: no `sudo`, no credentials; push with `gh-write`.

## 5. Adversarial pass, targeted

- **The judge loosened under the name of a fix:** any path where a scheduled line stamped 1 ns before its count is now accepted. Mutate `hooks_of`'s exactness, and confirm a test refuses the mutation.
- **The fix in the wrong place:** the ICM-42688-P model, or a test-only timed peripheral, through the same reset. If it still fails, the twin was not fixed.
- **The fix at one instant only:** move the droop's `start_ns` by a few ns and by a quantum, in a scratch scenario. The stamp must hold at each.
- **Replay:** the regression run's trace replayed twice gives identical state sequences (`INV-04`).
- **Sentinel-01 moving:** any byte of its outputs, its package, its `MANIFEST.json`, or any of its campaign hashes, through the fix, the regenerated goldens or a shared test helper.
- **An orphan U2 (R):** a load row, net, test point, BOM line, `.repl` line, firmware read, hwdef line or foreign check still naming U2. `validate` will not catch a load row (Grounds). Grep the outputs as well as the inputs.
- **A stale citation (K):** U2's load still cited to the BME280, as the spike's variant left it.
- **Two parts at one address (K):** U2's SDO drawn or routed to GND, so both BMP581s answer 0x46 on silicon while the twin places them at their declared addresses. Read it from copper.
- **A check deleted rather than moved:** `foreign-judge.sh` or a tier test that loses its U2 assertion under K, or loses a U7 assertion under R.
- **An expected value read from what is judged (`INV-07`):** a regression expectation, a re-pinned figure or a foreign window taken from the run it judges, rather than from the scenario, the specification or a cited bound.

## 7. Flag-and-stop conditions

- **Any schema change:** a specification field or `SPECIFICATION_SCHEMA_VERSION` (17 after `4400g`; `4240v` and `2304b` take the next ones); `PARTS_SCHEMA_VERSION` (10, `ir/src/parts.rs` L70); a trace record kind or `TRACE_VERSION` (`chaos/src/event.rs`: 13 on `main`, 14 after `4400g`). Stop with what is needed and why. A parallel bump would collide.
- **Any change to `aethelgard-ir`,** `aethelgard-eda` or `aethelgard-hwdef`'s `src/`.
- **Any byte of Sentinel-01 moves,** its `MANIFEST.json` moves, a Sentinel-01 campaign hash moves, or `--against-fab` reports anything but `identical`. If the only correct fix moves one, stop with the fix, the moved hashes and why. The order is ruled again; the package is not re-derived here.
- **The only fix found weakens the judge,** adds a tolerance, or reads the stamp in place of the count.
- **The fix needs a change to Renode** (a patched build, as `1605` did). That is `container/` and an image re-pin.
- **The fault vanished on post-`4400g` `main` and no run shows why.**
- **A gate that passes on `main` fails on a moved board:** Sentinel-03's bound (`OutlineUnreachable`, `OutlineTooSmall`), `extract`'s capacitance, DRC, support or the supply gate. Or the foreign judge's last witness falls outside its window. Stop with the numbers. Never widen a bound or a window, and never edit a fixture to pass.
- **Under K, ArduPilot refuses or mis-probes the second BMP581.** Stop with the log. The architect may flip D1 to R.
- **A protected surface would change** (the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`).
