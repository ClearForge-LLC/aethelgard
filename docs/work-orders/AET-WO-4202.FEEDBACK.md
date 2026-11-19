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

The fence lets this order edit U2 and its load row only. §7 lists the supply gate failing on a moved board as a stop, and renaming two other rows is outside the fence. **The fixture is not renamed here.** The ruling request is on the bus, with options (a) bring R7→R6 and R8→R7 into the fence, or (b) split, so that the `run_for` fix lands alone.

Red tests on the branch, all on Sentinel-03 (tier 1; everything else in the workspace is green):

| Test | On `ee835bde` | Branch |
|---|---|---|
| `supply_tier1::sentinel_03_on_rule_sized_vias_bounds_every_return` | pass | `SupplySinkUncounted` (R6) |
| `supply_tier1::each_input_the_gate_reads_is_refused_by_name_when_absent` | pass | the same refusal comes first |
| `supply_tier1::a_via_pad_the_worst_hole_does_not_fit_refuses_its_pins_with_the_remedy` | pass | the same refusal comes first |
| `outline_tier1::the_fixture_places_at_the_spikes_outline_with_every_gap_kept` | 85.25 × 80.95 mm | 86.70 × 80.95 mm (inside 100 × 100); placement digest moves |
| `via_ring_tier1::sentinel_03_on_rule_sized_vias_passes_every_bound_judged_by_extract_alone` | rung 7, via cost 320, IMU_SCK 24,945 fF | rung 6, via cost 96, EXT_SDA 78,403 fF bound vs 100,000 declared |

The outline, digest and rung are moves, not regressions: an earlier, cheaper rung now routes every bounded net. They are **held, not re-pinned**, until the ruling, because the specification's bytes may still change.
