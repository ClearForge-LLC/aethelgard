# AET-WO-4310 §1.1 — results, each prediction scored

Measured 2026-11-27 by the 4310 builder against main `adbd5c66`, after `PREDICTIONS.md` was committed (`141a7082`).

**What was run:**
- main's debug CLI;
- the probe `probe/` (the IR's own tables and `supervisor::reset_floor`);
- three prototype patches on a detached worktree of `adbd5c66` (`proto/*.patch`). Each is gated on an environment variable, so with none set the prototype binary reproduces main's outputs. That was checked on `solve` for all three power-tree boards.

No crate on the branch is changed. Every output named is in `out/`.

## Scores

| # | Prediction | Measured | Score |
|---|---|---|---|
| P2.1 | The outcome turns on which rail reading is ruled | Yes: U30 is a floor under (C), not judged under (A), and named under (B) (`out/supervisor-summary.tsv`) | right |
| P2.2 | (C): U30 3000 > 2870 mV is Sentinel-03's one new floor; `sentinel-03-radio` the same | U30 is a floor under (C), and under all three readings on `sentinel-03-radio` (on `VDD_3V3` there). **Not the only one:** U31.A is a floor too, once its held minimum is read (P2.3) | half right |
| P2.3 | (A): U1, U12 below; U31.A named, its minimum held nowhere | U1 and U12 pass. **U31.A is wrong:** its minimum is held. It is the VCCA band of the timing row the crossing reads (TXU0304 row 1, 3000–3600 mV), which `solve` already carries as `VDD_3V3`'s consumer floor 3000 mV (`out/consumer-floors.txt`). It is the same at-speed minimum the W25Q's `supply_by_read_clock` gives. So U31.A is a **floor, 3000 > 2870, under every reading**. Likewise `sentinel-02-power`'s U13.A (3000 > 2540). The probe's "no minimum" for translators was mine, not the data's | **wrong** |
| P2.4 | The MCU's 1620 mV is a new floor on the eleven BOR-0 boards | **The arithmetic is right; the finding is wrong.** The probe lists it on all eleven, plus three non-drawing specifications (`imu-baro-1v8`, `sentinel-02-bench-hse8`, `invalid-zero-domain`). But DS12117 Table 111's footnote 1 on VDD's 1.62 V minimum (p. 209; the held `ds12117-rev10.txt`, l. 7615) reads: "When RESET is released functionality is guaranteed down to V BOR0 min". The MCU's own floor is therefore VBOR0's minimum, and it is never above its own reset. The power-parts row cites Table 111 without the footnote. A rule that reads the row as written raises 14 false floors | **wrong** |
| P2.5 | The Sentinel-01 family is unchanged | U4 2700 > 2540 stays the only floor, as today (`readiness_4300k.rs` L377); nothing new | right |
| P2.6 | Regulators, passives, connectors and off-board peers are out of scope | No held row gives any of them a minimum operating supply. The pin parts (DRV8876, DRV8833) are named under (B) and (C), with no minimum held. Neither is on a drawn board (`d1-drv8876` is `DrawsNoBoard`; `d2-drv8833` is refused) | right |
| P2.7 | No plain verdict moves if floors reach only readiness | `floors_above_reset` has one non-test caller (`readiness.rs` L142), and `readiness::check` runs only for `FabMode::Write` (`verify.rs` L2435): not `--rebind`, not `--against-fab`. Plain verify names no floor. The module's doc says otherwise ("every other verdict names it, ruled D12"); that is a doc-code gap | right |
| P3a | The loaded floor is built; the source resistance is held nowhere; a §7 stop | Built: `verify.rs` L1625–1646 and `rail_levels`. No regulator row in `power-parts.json` holds an output impedance or a load-regulation figure. The SNVSAN3F PDF (LMR33630) is not in `~/.cache/aet-held/` at all. Every committed droop states 500 mΩ / 100 mA unsourced (`scenarios/*/droop*.json`, `fixtures/*droop*.json`, `bor-recovery/*`). **Stop.** The 325 mV arithmetic is not re-run: it rests on the unsourced 500 mΩ | right (not run past the stop) |
| P3b | −1.906 mV on each battery-fed regulator; no verdict flips; topology bytes move if solve prints it | −1.922 mV on every battery-fed buck of all three power-tree boards; see the table below. No verdict flips. `solve`'s topology moves in three condition texts per board: the input path's drop, 81→83 mV (Sentinel-03), 80→82 (`sentinel-03-radio`, at 1871 mA, up from 1870) and 70→72 (`sentinel-02-power`) | right |
| P3c | Sentinel-03 `VDD_3V3` fails once other returns are counted (≥ 110.7 mV at the √ bound, `VDD_5V` ≥ 25 mV); J4's return named | **Fails, by more:** 213.3 mV against 110, `VDD_5V`'s term 104.1 mV. `VDD_3V3_RF` fails too (221.4 against 144), which I did not predict. J4's return is named | right in direction; the magnitude about 4× under; one domain missed |
| P4.1 | extract cannot read the rung; a ruling | Confirmed (F-c). Of the 27 boards that draw, four run the ladder (`out/base-emit.tsv`) | right |
| P4.3 | One outline line each: 85.00 × 99.75, 46.35 × 54.85 mm | `Edge.Cuts` gives exactly these (`out/outlines.tsv`). They equal the committed Gerbers | right |
| P4.4 | Sentinel-01-bench cannot be re-written after (4) | Read at source (`rebind.rs` `matches`, `manifest_bound_alike`, `notes_bound_alike`): equal line counts, and only the binding hash lines may differ. So (4)'s added lines, and an added scenario from (5)'s fix, both give `RebindMoved`. A write without `--rebind` meets readiness and U4. **Not run**: the write path needs the build | right (from code) |
| P4.5 | "Both strict readers" has no referent | **Wrong.** The plain referent is the board's two readers, `strict::board` (`eda/strict.rs` L708) and `route::read` (`eda/route/read.rs` L841). It fits recording the rung in the board (D6). `--against-fab` does re-derive and diff (`verify.rs` L2797), so a package-level by-name refusal would need a new reader | **wrong** |
| P5.1 | Sentinel-01-bench loses `SENSORS`; Sentinel-01's seeded set keeps it | Measured with `proto/5-coverage.patch`. Of the **full** campaigns, `SENSORS` is lost on `s01-bench`, `sentinel-01-ready` and `four-layer`, each covered today only by `pullup-3k3`. The seeded set keeps it (`held-low-sda` and `held-low-scl`, detected). `four-layer` and `sentinel-01-ready` were not predicted | right, two more |
| P5.2 | `spi-only` `SENSORS` and `logger-4600` `TEMP` are the unmodelled-only buses | `spi-only`'s `SENSORS` becomes "only unmodelled devices". `logger-4600` has no committed campaign. `EXT` stays `no_device` under the prototype (ruled (c) kept) | right |
| P5.3 | Sentinel-03's four outputs are targets with no cover | `MOTOR1`–`MOTOR4` unmet on Sentinel-03, `motors` and `motors-pwm`, the only committed campaigns on boards with motor outputs | right |
| P5.4 | Both packages are `unmet` until scenarios are added | Sentinel-01-bench `SENSORS`; Sentinel-03 `MOTOR1`–`4` | right |

## (2) by board

From `out/supervisor-summary.tsv`, corrected for the translators by `out/consumer-floors.txt`. The probe gave translators no minimum, but each side's held minimum is its row's band. The reset floor is `supervisor::reset_floor`, and "today" is `floors_above_reset`.

| Board | Reset floor | Today | New, rail reading (A) | (B) | (C) |
|---|---|---|---|---|---|
| Sentinel-03 | 2870 (U12 TLV803S) | none | **U31.A floor (3000, held band)** | U31.A floor; U30, U31.B named (no reset on their rail) | **U31.A, U30 floors**; U31.B 3000 against the MCU's reset: floor |
| `sentinel-03-radio` | 2870 | none | **U30 floor** | U30 floor | U30 floor |
| Sentinel-01, `-minimal`, `-ext-bmp581`, `sentinel-01-bench` fixture | 2540 (BOR3) | U4 | none | none | none |
| `sentinel-01-ready` | 2870 | none | none | none | none |
| the 11 BOR-0 boards | 1580 | bus devices (1710–2700) | MCU (false, P2.4) | MCU (false) | MCU (false) |
| `sentinel-02-power` | 2540 | U4 | **U13.A floor (3000)** | U13.A floor; U5, U13.B (1650) named | U13.A floor; U13.B 1650 passes |
| the other BOR-3 boards | 2540 | U4 (or none) | none | none | none |

## (3b) and the battery sag

With (3b) and (5) both on (`out/coverage-old-new.tsv`, `series+new`), `battery-sag-tolerated` still covers the battery on Sentinel-03 and `sentinel-02-power`. The sag is classified from its own chain, not from `headroom`.

## (3b) headroom, measured (`proto/3b-series.patch`)

| Board | Regulator | Load | Margin before, µV | After, µV | Δ |
|---|---|---|---|---|---|
| Sentinel-03 | U10 | 2000 mA | 2,984,774 | 2,982,852 | −1,922 |
| Sentinel-03 | U11 | 750 | 5,022,529 | 5,020,607 | −1,922 |
| Sentinel-03 | U13 | 505 | 5,065,169 | 5,063,247 | −1,922 |
| `sentinel-03-radio` | U10 / U11 | 2000 / 1251 | 2,985,735 / 4,936,293 | 2,983,813 / 4,934,371 | −1,922 |
| `sentinel-02-power` | U10 / U11 | 2000 / 716 | 2,995,344 / 5,039,016 | 2,993,422 / 5,037,094 | −1,922 |

The margin is the available output at the input's minimum, less the output's maximum. The VIN minimum falls by 2 mV: the combined drop rounds up from 81 to 83 mV.

## (3c) other domains' return currents, measured (`proto/3c-returns.patch`, `out/returns-sentinel-03.txt`)

Sentinel-03's routed board is main's emit. Every R is `supply::bound` on the GND copper (400 iterations, the gate's own model):
- R₁₁ is the victim's worst return pin to its regulator's PGND;
- R₂₂ is an aggressor domain's worst consumer GND pad to its regulator's PGND.

Each aggressor is charged at |R₁₂| ≤ √(R₁₁·R₂₂) × its domain load: a passive reciprocal network's resistance matrix is positive semidefinite. The bound is certified as an upper bound only, so the true coupling could be much less (D5).

| Victim (limit) | Own drop | Aggressor | Load | R₁₁ / R₂₂, mΩ | Term, √ bound | Term charged at R₁₁ |
|---|---|---|---|---|---|---|
| `VDD_3V3`, U1 (110 mV) | 85.737 mV | `VDD_5V` (J6, J7, J8 → U10) | 2000 mA | 65.325 / 41.501 | 104.134 mV | 130.650 mV |
| | | `VDD_3V3_RF` (U30, U31 → U13) | 505 mA | 65.325 / 32.895 | 23.409 mV | 32.989 mV |
| | | **total** | | | **213.280 mV: fails by 103.3** | 249.376 mV |
| `VDD_3V3_RF`, U30 (144 mV) | 122.724 mV | `VDD_5V` | 2000 mA | 26.790 / 41.501 | 66.686 mV | 53.580 mV |
| | | `VDD_3V3` (U1, sensors, flash, U12, J2, J3, U31 → U11) | 750 mA | 26.790 / 67.926 | 31.993 mV | 20.092 mV |
| | | **total** | | | **221.403 mV: fails by 77.4** | 196.396 mV |

- **What the bound leaves out, named:**
  - the bucks' input currents returning to J4's GND (XT30, through-hole: `TerminalDrilled`), about 1906 mA at the pack's minimum;
  - U11's own input loop.
- **What the other two boards give:**
  - `sentinel-03-radio` (a 110 mV limit on `VDD_3V3`) was started and stopped unrun, to price (3c)'s cost instead. It is a fixture, not a package.
  - `sentinel-02-power`'s gate cannot run (`TranslatorPinUnbounded`).
  - No other board declares a drop limit.
- **Cost:** `extract` on Sentinel-03 took 2,175 s with the six added bounds (debug build, loaded Furnace, single-threaded).
