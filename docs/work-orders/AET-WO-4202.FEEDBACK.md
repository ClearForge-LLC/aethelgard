# AET-WO-4202 FEEDBACK

Base: `main` `ee835bde` (post-`4400g`, #238). D1 ruled R (U2 removed), D2 as drafted.

## 0. Predictions (committed before the first run)

Labels: **measured** (a run shown here or in `4201`), **read** (a file read), **inferred** (neither).

| # | Question | Prediction | Label |
|---|---|---|---|
| P1 | Does the stamp fault reproduce on `ee835bde`? | Yes, on `droop-bor3` alone, on an `AETHELGARD-INJECT` line, 1 ns early. The numbers differ from `4201`'s (`4400g` rebuilt the timers and USARTs at 64 MHz, so the injection's count moves). | inferred |
| P2 | Its cause | H1: the BMP581 model's 1 GHz one-shot timer is a sync point off the 10 ns instruction grid. A sync point that is not a multiple of 10 ns leaves the core's count and virtual time out of step by a fraction of an instruction across the reset, so the restarted count's origin is 1 ns off `released_ns + NUDGE_NS`. | inferred |
| P3 | Does the ICM-42688-P model (also ours, also timed) show it? | Yes, if its timer's events fall off the 10 ns grid in the boot after a release. Sentinel-02 carries both and showed no stamp refusal in `4201` (`s02-droop.txt`), so the phase matters; the fault needs an event off the grid in the window that sets the origin. | inferred |
| P4 | Sentinel-03 outline and least `extract` slack | Move: U2 and its decoupling leave the placement. | inferred |
| P5 | Sentinel-03 supply-gate figures | Move by U2's 1 mA load row. | inferred |
| P6 | `sentinel-02` hwdef lines | The `BARO BMP280 I2C:0:0x76` line goes; `BARO BMP581 I2C:0:0x46` stays. Nothing else in the hwdef moves. | read (`4201` §4.1, generator `hwdef/src/lib.rs` L686) |
| P7 | Foreign judge witness times and window margin | Move: ArduPilot probes one barometer fewer, so later witnesses come earlier. | inferred |
| P8 | Campaign `state_sequence_hash`es | Those of scenarios on `sentinel-02-own` move (D1). If the fix is in the twin's reset path, every droop scenario's hash could move, Sentinel-01's included, which is a §7 stop; a fix that renders the same commands for a BME280-only board leaves Sentinel-01's unchanged. | inferred |
| P9 | Tier-2 tests added | One (the regression run through the reset with a timed model of ours), plus one tier-1 test of the mechanism. | inferred |

**Scored after the runs:**
- **P1:** right on the fault; wrong on the numbers, which were `4201`'s exactly.
- **P2, P3:** wrong. The cause is Renode's parse, not the model (§1.2).
- **P4:** right that the figures move, wrong in direction. The outline shrank on the merged tree (96.50 × 69.35 → 95.80 × 67.60 mm), the ladder went from rung 5 to 7, and a turned-IMU variant went over the bound (§2.4).
- **P5:** right only after ruling (a). U2's 1 mA row went, and the renumbering broke the gate first (§2.2).
- **P6:** right.
- **P7:** half right. The witnesses moved by under 2 ms (U7's identity 0.19 ms earlier, U5's 1.56 ms earlier), and the windows still bracket.
- **P8:** right. No committed scenario's hash moved, Sentinel-01's included. No scenario is based on `sentinel-02-own` (its campaign base copy is test-local).
- **P9:** right. One tier-2 test, plus two tier-1 tests (`run_for`, and `hooks_of` exactness after M2 survived) and one tier-1 pin (load rows).

## 1. The twin fault: re-measured, root-caused, fixed

All runs in the pinned tools image under `capped-run --mem 16G` (memory.swap.max 0 read back), on scratch trees outside the repository (`~/.cache/aet-4202/scratch/`). Kit: `~/.cache/aet-4202/tools/`, outputs `~/.cache/aet-4202/out/`.

### 1.1 Re-measured on `ee835bde`, before any edit [measured]

| Run | Result |
|---|---|
| bench campaign (5 scenarios), Sentinel-01 unchanged, ×2 | pass 5/5, 2/2 |
| the same, U2 a BMP581 at 0x46, ×2 | refused 2/2: `AETHELGARD-INJECT 0 0 1952227 1962227 28a801 770`: placed at 20854010 ns in a boot started at 1231740 ns, stamped 20854009 ns |
| `droop-bor3` alone, U2 a BMP581 | refused, the same line |
| `serial-rx-open-console` alone, U2 a BMP581 | pass |

P1 was right on the fault and wrong on the numbers: they are `4201`'s exactly (the firmware ELF of the BMP581 variant is `6cdad1ce…`, its boot unchanged by `4400g`).

### 1.2 Root cause [measured]

**Renode 1.17.0 runs `emulation RunFor "0.001018380"` for 1,018,379 ns.** Its `TimeInterval.TryParse` (renode-infrastructure `cd4b002a`, the pinned source, `src/Emulator/Main/Time/TimeInterval.cs` L55) reads the decimals as a fraction of a second through a double and truncates:

```
var decimals = ... (ulong)(double.Parse($"0{m.Groups["decimals"].Value}", CultureInfo.InvariantCulture) * TicksPerSecond) ...
```

`float("0.001018380") * 1e9` is 1018379.9999…, so the tick count is 1,018,379. A tick is a nanosecond (`TicksPerNanosecond = 1`, L292).

The droop schedule renders the BOR assert as `RunFor "0.001018380"` from 0. Renode stops 1 ns short, and every later instant is 1 ns early. A probe build (scratch only: the full log kept, a CPU hook on the reset handler `0x08000040`, the model's timer events logged) shows it:

```
[00:00:00.001018379] [INFO] machine-0: Machine paused.                      <- RunFor "0.001018380"
[00:00:00.001018379] [WARNING] i2c1.u2: AET4202 reset t=1018379              <- machine Reset
[00:00:00.001231729] [INFO] machine-0: Machine paused.                      <- the release, 1 ns early
AETHELGARD-RELEASE-0
[00:00:00.001231739] [WARNING] cpu: AET4202HOOK pc=8000040 n=0             <- the boot starts 1 ns before released_ns + NUDGE_NS
[00:00:00.020854009] [WARNING] sysbus: AETHELGARD-INJECT 0 0 1952227 1962227 28a801 770
```

The judge places the boot at `released_ns + NUDGE_NS` = 1231740 ns, so every scheduled line is stamped 1 ns before its count. `hooks_of` refuses it, correctly: the run did not do what the schedule said.

**H1, H2 and H3 are refuted by runs.** The model's timer never fires before the reset (armed at 24340 ns for 6124340 ns). With the timer never enabled (E1) or with `Now()` not calling `cpu.SyncTime()` (E2), the slip is the same, at the same instant. P2 and P3 were wrong.

**Why the BMP581, then [measured, from the two traces, kinds 16, 17 and 18]:** the BMP581's schematic arm draws a second 100 nF (`eda/src/schematic.rs` L1398-1401). `decoupling_picofarads` adds it to `VDD_3V3` (`cli/src/campaign.rs` L306). The effect on the bridge's droop:

| | BME280 | BMP581 |
|---|---|---|
| decoupling on `VDD_3V3` (kind 16) | 8,300,000 pF | 8,400,000 pF |
| `VBOR3_falling` crossing (kind 17) | 1,018,322.818 ns | 1,018,370.136 ns |
| reset, on the 10 ns grid (kind 18) | 1,018,330 ns | 1,018,380 ns |

1,018,330 parses exactly, and 1,018,380 does not.

**Not the model, and not one board [measured]:**

- **Predictive control:** Sentinel-01 unchanged (BME280), with `droop-bor3`'s `start_ns` moved +50 ns in a scratch scenario, so that the assert lands on 1,018,380. On `ee835bde` it is refused the same way (`INJECT 0 0 2813 12813`: placed at 1359880, stamped 1359879).
- **Sentinel-02 with D1 applied** (U2 removed, scratch copy without `mcu.foreign_clock`, which `4400g` refuses on a generated run; below) is refused on `ee835bde` the same way (`INJECT 0 0 3905804 3915804`: placed at 40390680, stamped 40390679).
- About 2.2 % of the 10 ns-aligned durations under 2 ms truncate (4,425 of 200,000, computed as Renode parses).

**Survey of committed scenarios [measured].** Every `RunFor` rendered by the six committed droop scenarios parses exactly on `ee835bde`: `sentinel-01-bench/droop-bor3`, `sentinel-01/droop-bor3`, `sentinel-01/droop-tolerated`, `motors/dshot600-droop`, `four-layer-droop` and `spi-only-droop-bor3`. A fix to the rendering therefore cannot move their emulation.

### 1.3 The fix [measured]

**Site:** `run_for` in `crates/aethelgard-cli/src/renode.rs`, which renders every `RunFor` the CLI composes: droop, static and foreign boots. It adds a tenth decimal of 5: `RunFor "0.0010183805"`. The value is then half a nanosecond above `ns`. The double's error for a fraction below one second is under 1e-6 ns (2^-53 relative, times 1e9), so Renode's truncation lands on `ns` exactly.

- **This is the twin's rendering, not a model's.** It holds for every board and every peripheral.
- **The fence:** `run_for` is in `renode.rs` but is not `hooks_of` or the boot start, which the fence names. The bus said so before the edit.
- **The judge is untouched.** `hooks_of` still requires `stamp == at` for a scheduled line and refuses a count ahead of its stamp.
- **`RELEASE_RENDERING` and `ASSERT_RENDERING` are untouched.** Kind 18 records their prose, so editing it would move every droop trace's hash.
- **`QUANTUM`** (`SetGlobalQuantum "0.000100000"`) parses exactly (100,000) and is left alone.

**With the fix, in real Renode** (the probe build plus the fix):

```
[00:00:00.001018380] [INFO] machine-0: Machine paused.
[00:00:00.001231730] [INFO] machine-0: Machine paused.
AETHELGARD-RELEASE-0
[00:00:00.001231740] [WARNING] cpu: AET4202HOOK pc=8000040 n=0
[00:00:00.020854010] [WARNING] sysbus: AETHELGARD-INJECT 0 0 1952227 1962227 28a801 770
```

| Run, fixed CLI | Result |
|---|---|
| bench campaign, Sentinel-01 unchanged, ×2 | pass 5/5, 2/2. **Every `state_sequence_hash` identical to `ee835bde`'s** (`out/s01-bench-hashes-{main,fix}.txt`) |
| `droop-bor3` +50 ns, Sentinel-01 unchanged | pass (was refused) |
| `droop-bor3`, U2 a BMP581, ×2 | no refusal: `fail`, `brownout_unrecovered`, hash `4f811032…` both times (§1.4) |
| Sentinel-02 (scratch, no `foreign_clock`) through `s02-droop-bor3` | `fail`, `brownout_unrecovered`, hash `44d2eea0…`, **the same on `ee835bde`** |
| the same with U2 removed (D1) | `fail`, `brownout_unrecovered` (was refused by the stamp) |

### 1.4 What the stamp fault hid: `brownout_unrecovered` with a BMP581 [measured; a finding, not fixed here]

With the stamp fixed, Sentinel-01 with U2 a BMP581 fails `droop-bor3` on `brownout_unrecovered`. The probe at release + 100 µs (`RECOVERY_BOUND_NS`) reads:

```
AETHELGARD-PROBE-BEGIN-0
0x00000000      <- the boot word: not yet rewritten
0x22020000
0x00000050      <- U2's identity, read
AETHELGARD-PROBE-END-0
```

The boot word's own anchor (count 1952227) places its write at 20,754,010 ns, which is 19.5 ms after the release. The generated firmware writes the boot word only after every device's script. The BMP581's script settles for 10 ms (parts row, `0x36 ← 0x40 0x81`, 10 ms) and its measurement takes 6.1 ms (the model's timer: armed at 1256080 for 7356080). A BME280 has no script, so its boot word comes about 20 µs after the release (`2301` §2.3).

This is the same `brownout_unrecovered` that `4201` saw on Sentinel-02 (U7 is a BMP581). It is present on Sentinel-02 with or without U2, and its hash is unchanged by the fix. It is not the stamp fault. The WO puts it out of scope, so it is a finding (F1).

### 1.5 The bug's reach, and the proof [measured; ruled 20:40Z: the fence takes `run_for`]

**Reach.** Every `RunFor` the CLI composes goes through `run_for`: a droop's schedule (each assert, release, nudge, probe and the run's end), a static boot's one `RunFor` (`--for`, or a campaign's 100 ms), and a foreign boot's window (`renode/foreign.rs` L710). A truncated `RunFor` runs 1 ns short. Every instant after it is then 1 ns early. Only the judge's model of a droop's boot start (`released_ns + NUDGE_NS`) sees that, through a scheduled line (an injected byte, a DShot stop). A hook line is allowed to come early, and the last `RunFor` of a run only ends it 1 ns early, so neither is seen.

**Committed results that could have been affected: none, checked two ways.**
- Every `RunFor` the six committed droop scenarios render was logged and parsed as Renode parses it (§1.2, `out/survey-runfor.txt`).
- Every static and foreign duration committed in tests, CI and scenarios parses exactly: 10 ms, 100 ms, 120 µs, 1 ms, 4300 ms, 4370 ms and 5000 ms (computed).
- Re-running the committed droop scenarios with main's binary and the branch's gives the same archive hashes (§3).

**Why the rendering is exact.** `run_for(ns)` writes `ns + 0.5` ns as decimals of a second. `double.Parse` rounds the fraction (below 1) correctly, to within 2^-53 of it, and the product by 1e9 (exact in a double) rounds once more. The result is `ns + 0.5` to within 2^-52 × 1e9 < 2.3e-7 ns. It lies strictly between `ns` and `ns + 1`, so the truncation is `ns` and can never reach `ns + 1`. Whole seconds are parsed as integers.

`every_run_for_runs_exactly_its_nanoseconds_in_renode` (tier 1, `renode.rs`) re-implements `TryParse` as the pinned source writes it (cited by file and line). It asserts `parse(render(ns)) == ns`:
- for 1,018,380 by name, after asserting that the nine-digit form runs it as 1,018,379;
- for every ns in 0..=2,000,000;
- on a stride of 9,973 ns up to `FOREIGN_MAX_VIRTUAL_NS` (5 s), the longest `RunFor` any boot renders, and at 5 s itself.

The tier-2 recipe test's step (e) measures the same truncation in Renode itself: `"0.001018380"` advances virtual time by 1,018,379 ns, and `"0.0010183805"` by 1,018,380 ns.

## 2. D1 (R) applied, and the §7 stop on Sentinel-03

### 2.1 The edit [measured]

- `sentinel-02-own.json` and `sentinel-03.json`: U2 is removed from `i2c_devices`, and U2's `power.loads` row (cited to `BST-BME280-DS001-24`) from `sentinel-03`. Neither file mentions `U2` or `BME` any more (grep).
- `sentinel-02.topology.json` and `sentinel-02.hwdef.dat` are regenerated by the branch's CLI, as CI's `generated` job does (`solve`, then `generate hwdef`). The semantic diff of the topology: `i2c_devices.U2` goes, U3's index goes 1 → 0 and U7's 2 → 1, and the specification's hash changes. Nothing else moves. The hwdef loses exactly `BARO BMP280 I2C:0:0x76` and keeps `BARO BMP581 I2C:0:0x46` (U7). P6 was right.
- **Unmodified ArduPilot builds both** U2-less hwdefs (`sentinel-02` and `sentinel-02-noimu`), in the pinned image (`aethelgard-ardupilot@sha256:160f9955…`) with `--network none`, 33 s each (`out/ap-build.log`).

### 2.2 Sentinel-03: the supply gate refuses the moved board [measured] — §7

With U2 removed, `extract`'s supply gate on Sentinel-03, gated as `supply_tier1` gates it, refuses:

```
SupplySinkUncounted: R6 (Device:R) is on `VDD_3V3`, and no `power.loads` entry names it: its current is cited from its datasheet, derived, or declared, and never zeroed
```

The emitter numbers straps and pull-ups in the order it draws them, and U2's SDO strap was R5. Removing it renumbers every later one. From the emitted copper (footprint reference → pad nets), `ee835bde` against the branch:

| Reference | `ee835bde` | branch |
|---|---|---|
| R5 | U2_SDO – GND | U7_SDO – GND |
| R6 | U7_SDO – GND | U4_WP – VDD_3V3 |
| R7 | U4_WP – VDD_3V3 | U4_HOLD – VDD_3V3 |
| R8 | U4_HOLD – VDD_3V3 | BOOT0 – GND |
| R9 | BOOT0 – GND | — |

Sentinel-03's `power.loads` names R7 and R8: the flash's WP and HOLD pull-ups ("no DC path but the flash pin's leakage"). On the moved board R6 (WP) is uncounted, and R8 names BOOT0 to GND. In the BOM, C41 (U2's 100 nF) goes and C42/C43 become C41/C42. No load row names a C.

The fence let this order edit U2 and its load row only, and §7 lists the supply gate failing on a moved board as a stop. The work stopped there and asked for a ruling.

**Ruled (a), 21:05Z.** R7 → R6 and R8 → R7 in Sentinel-03's `power.loads`, on condition that a scratch run passes the supply gate with them. Their sources are unchanged: they describe the same two flash pull-ups.
- **The condition, measured on the merged tree (main `96517013`):** the moved board without the rename is refused (`SupplySinkUncounted`, R6). With it, `extract` passes, gated as `supply_tier1` gates it and ungated (`out/merged.log`).
- **The pin:** `supply_tier1::each_load_row_s_resistor_is_the_part_its_row_describes` reads the emitted copper. Each resistor a load row names connects `VDD_3V3` to the net its row describes: R1 /I2C_SDA, R2 /I2C_SCL, R3 /EXT_SDA, R4 /EXT_SCL, R6 /U4_WP, R7 /U4_HOLD, R20 /U11_FB. The rows must be exactly those, in order. Mutants in §3.
- **The finding** (a load row named by an emitted, sequence-numbered designator breaks silently when an unrelated part leaves) is the architect's §7.1 row. The lasting fix is a specification change and is not built here.


### 2.3 Every moved figure, re-pinned from a measured run [measured, on main `96517013` + branch, then `35d23eae`]

| Figure | Before (main) | After (branch) | Where pinned |
|---|---|---|---|
| Sentinel-03 outline | 96.50 × 69.35 mm (`1315`) | 95.80 × 67.60 mm | `outline_tier1` fixture test; `board_tier2` ANSWER |
| Sentinel-03 placement digest | `9b83d120…` | `bd276856…` | `outline_tier1::FIXTURE_PLACEMENT_SHA256` |
| Size sample 1 | 98.80 × 70.35 mm | 98.80 × 69.85 mm (2, 6, 8 still refused by name) | `outline_tier1` |
| Capacitance ladder | rung 5, via cost 48, nearest I2C_SCL 74,937 fF | rung 7, via cost 320, nearest EXT_SDA 75,697 fF of 100,000 | `via_ring_tier1` |
| Least `extract` slack | 8.6 pF | 8.5 pF | `board_tier2` ANSWER (not pinned) |
| Supply gate | passes | passes after the rename (§2.2) | `supply_tier1` 10/10 |
| `generate hwdef` on Sentinel-03 | `SenseNotEmitted` (VBAT_SENSE) | the same, by that name only | measured |
| `sentinel-02.hwdef.dat` | `BARO BMP280 I2C:0:0x76` + `BARO BMP581 I2C:0:0x46` | `BARO BMP581 I2C:0:0x46` only | `hwdef_tier1`, CI `cmp` |
| Foreign witnesses (`positive`, 3 runs each, all pass) | U7 id 385.832 ms, U7 data 435.812 ms, U5 id 4342.644 ms, U5 FIFO 4393.959 ms; 786–789‰ | U7 id 385.639 ms, U7 data 435.807 ms, U5 id 4341.081 ms, U5 FIFO 4392.421 ms; 763–786‰ | `ci/foreign-judge.sh` (windows unchanged: 4300 ms < U5 id < 4370 ms < FIFO < 5000 ms still holds) |
| Campaign `state_sequence_hash`, the six committed droop scenarios | `71a6ffb3…`, `d58b7d39…`, `e21b21b5…`, `b0f9ff9c…`, `76b42f30…`, `297ab68a…` | identical | measured (`out/dh-{m,b}.txt`) |
| Sentinel-01 bench campaign (5 scenarios) | — | identical hashes (§1.3) | measured |
| `TIER2_MIN` | 190 | 191 (one tier-2 test; carried at `1911`'s word) | `ci.yml` |

Sentinel-02's own-board tier-2 reads are moved to the new truth: `twin_tier2`'s I2C steps lose `U2 identity 0x60` and keep U7's three. Before and after, each of these passes on the branch: `twin_tier2` own board ×4, `models_tier2` 12/12, `board_tier2` own parts ×2, `with_tools` keep-out.

### 2.4 A turned-IMU variant over Sentinel-03's bound [measured] — §7

`outline_tier1::a_turned_imu_is_compacted_whole_with_its_keepouts_inside_the_bound` (`4400d`'s variant: `board.forward` "top", U5 turned each of four ways). With U2 removed, the **forward** turn is refused:

```
OutlineUnreachable: board.max_outline_um is 100.000 x 100.000 mm; the placer's best outline is 103.500 x 67.100 mm, over by 3.500 x 0.000 mm
```

The right, left and aft turns place, and on main `96517013` all four place (`out/turned.txt`). The fixture as declared places at 95.80 × 67.60 mm. §7 lists Sentinel-03's bound failing on a moved board as a stop, so the bound and the test are untouched and a ruling is requested on the bus (22:00Z).

### 2.5 `ci/foreign-judge.sh` on the U2-less ArduPilot [measured] — outside the carve-out

The full script runs with the carve-out edits (U2's checks out, the IMU-less run's I2C witness moved to U7). Unmodified ArduPilot is built from the regenerated hwdef. Every case holds but one:

| Case | Result |
|---|---|
| `positive` | holds (also 3/3 standalone, 763–786‰) |
| `window-before-u5` and `window-before-u5-data` | hold. At 4370 ms the branch also shows `core_in_exception` (exception 66): the cut lands in an interrupt. |
| `undeclared`, `clock-undeclared`, `clock-on-generated-run`, `no-spi2-rx-dma`, `no-u5-model` | hold |
| `no-imu-line` | holds; U7's identity is witnessed at 387.244 ms |
| **`clock-contradicted`** | **fails** its exact check, `kinds == ["foreign_clock_contradicted"] * 2`. The branch adds `register_mismatch` ×2: GPIOB.MODER reads 0xa400a000 against the solved 0xa800a000, and AFRH 0x55000000 against 0x55500000 (PB13 not yet configured at the 5 s cut under the 250 MHz declaration). Main `96517013` with main's ELF gives exactly the two contradictions. |

Without U2, ArduPilot probes one barometer fewer, so the run's cut lands at a different point in its initialisation. The two clock contradictions are unchanged. The check is not a U2 line, so it is outside the carve-out and is left alone. Ruling requested on the bus.

### 2.6 Also measured on the final tree

- **`3403`'s `examples/sentinel-01-ext-bmp581.json` through `droop-bor3`** (its ELF `a60306c9…`): the stamp refusal recorded in `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L35 is gone. The run now fails `brownout_unrecovered` (F1). That file is fenced, so its L35 statement is now stale: a finding for `3403`'s owner, not an edit here.
- **`verify examples/sentinel-02-bench.json`** (`3304`): pass, every gate.
- **`ci/shard.sh plan`:** the guard holds (2,508 test names, 8 shards; `1911`'s 12 shards are not merged yet).
- **Sameness** (§3) covered the 33 specifications committed at `96517013`. `3403`'s new specification arrived after it, and its own run is above.

## 3. Adversarial pass (targeted, §5)

| Attack | Result [measured] |
|---|---|
| **The judge loosened** (M2: `hooks_of` accepts a scheduled line stamped 1 ns early) | Survived every existing test (bins 165, `boot_tier1` 61, `campaign_tier1` 65). Fixed: `a_scheduled_line_is_placed_only_at_its_count_s_exact_instant` (tier 1) now kills it. |
| **The fix reverted** (M1: `run_for` without its tenth decimal) | Killed by the tier-1 test (on 1,018,380 by name) and by the tier-2 regression, which then fails with the pre-fix refusal: `INJECT 0 0 2813 12813 …: placed at 1359880 ns in a boot started at 1231750 ns, but it is stamped 1359879 ns`. That is the regression failing on the unfixed tree. |
| **The fix in the wrong place** | The cause is the rendering. The BME280 board (no timed model of ours) fails the same way when the droop is shifted (§1.2), and the fix holds there. Sentinel-02 (ICM-42688-P + BMP581) with D1 also passes the stamp after the fix. |
| **The fix at one instant only** | The BME280 bench droop shifted +1, +3, +5, +7, +9, +10, +20, +50, +100,000 (a quantum) and +100,050 ns: 10/10 pass on the fix (`out/a3-shifts-fix.txt`). |
| **Replay** | The regression's archive is replayed twice in the test: identical output, and its hash equals the campaign's. |
| **Sentinel-01 moving** | The bench campaign's hashes are identical (§1.3). A/B `verify --fab-out`, main's binary against the branch's, writes byte-identical packages (17 files). `--against-fab` locally differs only in `MANIFEST.json`'s `verdict_sha256`, as it does with main's binary: the sandbox (`uid_boundary` false under rootless podman). CI's `bench-package` is the authority. All 33 committed specifications give byte-identical `solve`, `emit`, `generate` and `generate hwdef` under main's binary and the branch's (`out/sameness.log`). |
| **An orphan U2 (R)** | No `U2` or `BME280` (any case) in either moved specification, nor in any output of either board: schematic, board, BOM, generated crate and `.repl`, hwdef, solve, foreign verdicts. No load row names U2. |
| **A renumbered designator** (ruled (a)) | M-swap (R6↔R7) and M-boot0 (R7→R8, BOOT0's strap) are killed by `each_load_row_s_resistor_is_the_part_its_row_describes`. |
| **A check deleted rather than moved** | `foreign-judge.sh`: U2's identity and data checks go, and every U4, U5 and U7 check stays. The IMU-less run's I2C witness moves from U2 to U7. `twin_tier2` keeps every U7 step. |
| **An expectation read from what is judged (`INV-07`)** | The regression's expectation is the bench scenario's own `expect` and `expect_counts`; only `start_ns` moves. Its precondition pins the reset at 1,018,380 ns, the instant the recipe test's step (e) measures Renode running 1 ns short. It is checked against the trace, not the run's verdict. The re-pins are geometry the run produces, re-pinned only under ruling (a), each with its before. |

## 4. Findings

- **F1. With a BMP581 as the first scripted device, a BOR droop cannot recover within 100 µs** (§1.4). The generated firmware writes the boot word after every device's script, and the BMP581's script settles 10 ms. This affects `3403`'s bench specification with a BMP581 on J4 (told on the bus) and Sentinel-02's own board (U7). Not fixed here.
- **F2. A `power.loads` row naming an emitted, sequence-numbered designator breaks silently when an unrelated part leaves** (§2.2). The architect's §7.1 row. The pin test guards Sentinel-03 only.
- **F3. `validate` does not check that a load's `reference` names a declared part** (the WO's Grounds). Recorded, not built.
- **F4. Post-`4400g` `sentinel-02-own` declares `mcu.foreign_clock`,** so a generated campaign on it is refused (`ForeignClockOnGeneratedRun`). The WO's §1.1 step "run `s02-droop-bor3` on `sentinel-02-own`, unchanged" was run on a scratch copy without the declaration, as `twin_tier2::own_board` does.
- **F5. The regression test deviates from the WO's wording.** It uses Sentinel-01 as committed with the bench droop 50 ns later, not Sentinel-01 with U2 a BMP581: that variant fails its own expectation by F1 once the stamp is fixed. The shifted droop lands the reset on the same lossy instant (1,018,380 ns) on the board as ordered.
- **F6. The placer is not monotone in parts.** Removing U2 grew the turned-IMU variant (forward) from fitting to 103.5 mm wide (§2.4).
- **F7. Locally, the foreign judge's memory reached 1005–1007‰ of its 1 GiB bound once each, on main and on the branch** (`memory_cap_exceeded`). Three like-for-like runs each then measured 763–789‰. This is a Furnace load effect; CI's runs are the evidence (`4400g` F1: 773–800‰).

## 5. Gates and the reduction pass

- **Reduction pass (diff): −12 net · 1 duplicate removed · 0 dead paths removed · 0 protected kept · 1 finding below.**
  - **Removed:** `campaign_tier2`'s own copy of Renode's parse. The tier-1 test owns that model and step (e) measures it in Renode. The regression now pins the reset instant (1,018,380 ns) instead. M1 still kills the regression, which fails before that line.
  - **Finding:** `campaign_tier2.rs` keeps its own `run_for` (the nine-digit form) for the `2301` recipe harness, a copy of the CLI's private function, which a test binary cannot reach. Every value it renders (1 ms, 10 ns, 10 µs, 100 µs) parses exactly, so it is stale but harmless. Left for the gate pass.
