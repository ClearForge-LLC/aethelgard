# AET-WO-4310 — STOP after §1.1, with the table

2026-11-27, the 4310 builder, from main `adbd5c66`.
- **Evidence:**
  - `PREDICTIONS.md` (committed `141a7082` before any probe ran), scored in `RESULTS.md`;
  - `out/` (every table below);
  - `probe/` (the IR's own tables and `reset_floor`);
  - `proto/*.patch` (three prototype patches on a detached worktree of `adbd5c66`, each behind an environment variable).
- **Nothing outside `spike/4310/` is changed.**

**Three stops fire together:**
1. **§1.1, as planned:** the table.
2. **§7, a committed board fails a new rule.**
   - **Sentinel-03:** (2) U30, (3c) `VDD_3V3`, and (5)(iii) its four motor outputs.
   - **Sentinel-01-bench:** (5)(i), I2C `SENSORS`.
   - Each is a finding about the board, with what fixing it takes below. No rule is softened.
3. **§7, a figure the rule needs is held nowhere.**
   - **(3)'s droop source resistance**, for every droop scenario on every board.
     - No regulator row holds an output impedance.
     - SNVSAW1E (LMR33620, held) states load regulation only as typical curves (Fig. 9-19, 9-25).
     - SNVSAN3F (LMR33630, U10 and U11) is not held.
     - Sentinel-01's `VDD_3V3` source is off-board and declared nowhere.
   - **TXU0304's operating minimum** (U31, Sentinel-03) is held only as quoted text inside a citation string ("SCES935A p. 6, 7.3", 1.08–5.5 V). The PDF is not in `~/.cache/aet-held/`.

## The table, by board

Verdict columns: **plain** = `verify` with no fab flag; **write** = `verify --fab-out` (readiness judged); **against** = `verify --against-fab` (the package derived again).

| Board | (2) floor | (3a) droop | (3b) headroom | (3c) returns | (4) package | (5) coverage |
|---|---|---|---|---|---|---|
| **Sentinel-03** (`fab/sentinel-03`) | **U30 3000 > 2870 mV** under (C); named under (B); U31 named (no minimum). write **refused** (`FabricationNotReady` U30); plain and against unchanged | stop (source R) | −1.92 mV of ~2.98–5.07 V; no flip. Topology bytes move (81→83 mV in 5 texts) | **both limited domains fail:** `VDD_3V3` 85.7 own + 104.1 (`VDD_5V`) + 23.4 (`VDD_3V3_RF`) = 213.3 mV against 110 at the √ bound (249.4 naive); `VDD_3V3_RF` 221.4 against 144; J4's battery return named | + rung 7 / step 1 / 1000 ‰ into extract and FAB-NOTES; + "85.00 × 99.75 mm". `FAB-NOTES.md`, `MANIFEST.json` and verdict move; Gerbers, BOM, ELF and CPL do not | **`MOTOR1`–`4` unmet**, so plain is not verified and against and write fail; `bench-package-s03` red until 4 output scenarios |
| **Sentinel-01** (`examples/sentinel-01.json` → `fab/sentinel-01-bench`) | unchanged (U4 2700 > 2540 is today's; the write is `--rebind`, no readiness) | stop (source R) | no power tree: unchanged | no drop limit: unchanged | no ladder: an explicit "not run" if ruled; + "46.35 × 54.85 mm". **The `--rebind` write is refused (`RebindMoved`) once FAB-NOTES gains a line**, and a plain write meets readiness's U4 | **`SENSORS` unmet** (only `pullup-3k3` covered it); `bench-package` red until a detected `SENSORS` fault is added |
| `sentinel-03-radio` (fixture) | **U30 floor under every reading** (on `VDD_3V3` here) | stop | −1.92 mV; topology bytes move | not run (stopped to price (3c)); a 110 mV limit on `VDD_3V3`, the same shape as Sentinel-03 | rung 5 / step 0 | motor outputs, if a test runs its campaign |
| `sentinel-02-power` (ir fixture) | U13 (translator) named | — | −1.92 mV; topology bytes move | the gate cannot run (`TranslatorPinUnbounded`): unchanged | rung 0 | its partial set: unchanged |
| `sentinel-01-ready`, `four-layer` (tier-2 package fixtures) | unchanged | stop (their droops) | — | — | `sentinel-01-ready` rung 7, no step | **`SENSORS` unmet** (only `pullup-3k3`): their package tests fail until a detected `SENSORS` scenario |
| `spi-only` | unchanged | stop (`spi-only-droop-bor3`) | — | — | — | `SENSORS` named "only unmodelled devices" (TMP117) |
| `motors-dshot600`, `motors-pwm` | false MCU floor at BOR0 (motors-pwm), see D2 | stop (`dshot600-droop`) | — | — | — | +`MOTOR1`–`4` unmet (partial sets, not packaged) |
| the remaining INV-08 boards | the 11 BOR-0 boards: a *false* MCU floor unless D2's footnote is built; else unchanged | — | — | — | the extract verdict moves only if "not run" is written explicitly (D6) | no committed campaign: unchanged |

`INV-08`'s 25: under the leans below, every one is byte-identical in board, schematic, BOM, `solve` and `extract` bytes, except where D6 adds the explicit "ladder not run" field.

## The table, by part

| Part | Rule today | What 4310 adds | Boards that move | Held? |
|---|---|---|---|---|
| (2) floors | I2C/SPI on `mcu.supply`, unknown part skipped, readiness only | every powered part; named when it has no minimum | Sentinel-03 (U30, U31), `sentinel-03-radio` (U30), `sentinel-02-power` (U13 named) | all minimums held except TXU0304's (quoted text only) and the DRV88xx pin parts (no drawn board) |
| (3a) droop | loaded floor built (2304b L697); load and R are free scenario numbers | load from the topology, R held | every droop scenario: S01, S01-bench, S03, `motors`, `spi-only`, `bor-recovery` | **R held nowhere: stop** |
| (3b) headroom | fuse + FET counted | + `battery.series_milliohms` × input current | the three power-tree boards: −1.922 mV, topology text | held (spec, sourced) |
| (3c) ground | own domain's return only, a named condition | other domains' returns on shared GND copper, or named | Sentinel-03 (fails), `sentinel-03-radio` | currents held (topology); R measured from copper; the battery's return (J4, through-hole) cannot be bounded: named |
| (4) package | rung and step in `emit` JSON only | extract verdict, FAB-NOTES, outline line, readers | both packages; the four laddered boards' extract | outline: `Edge.Cuts`; rung: must be recorded (F-c) |
| (5)(i) tolerance | any raise covers | only past a declared tolerance; none is declared | S01-bench, `sentinel-01-ready`, `four-layer` lose `SENSORS` | no pull-up tolerance is declared anywhere (bare `Device:R 2k2`) |
| (5)(ii) unmodelled | any device counts | modelled device needed | `spi-only` | TMP117 is the only unmodelled part |
| (5)(iii) motors | no target | each output a target, an output fault kind | Sentinel-03, `motors`, `motors-pwm` | witness exists (`motor_output_unwitnessed`) |

## New findings on committed boards, and what fixing each takes

1. **Sentinel-03, U30's supply floor (2).** U30 needs 3.0 V (v1.7 Table 6-2). Nothing resets it on `VDD_3V3_RF`, and U12 releases the MCU at 2.87 V on another rail.
   - **No supervisor fits on today's rail.** `VDD_3V3_RF`'s minimum at U30's pin is 3,144.7 mV less its 144 mV drop limit, 3,000.7 mV. A supervisor on it must fall at ≥ 3000 mV and release at ≤ 3000.7 mV.
   - **Fixes:**
     - (i) raise `VDD_3V3_RF`'s set point (R30/R31) so a supervisor on U30's EN fits between U30's 3.0 V and the rail's minimum (U30's maximum is 3.6 V). A Sentinel-03 board change: its own work order.
     - (ii) the module's own brown-out reset: the threshold is in the ESP32-C3 chip datasheet, which is not held (the module's v1.7 states none).
     - (iii) keep 4411's ruling (a) by a person's acknowledgement that the rule names and readiness accepts (a new acknowledgement class; the rule is not softened, it names U30).
2. **Sentinel-03, both drop limits with other returns counted (3c).**
   - **Measured on the routed copper:**
     - `VDD_5V`'s 2000 mA (J6, J7, J8 GND → U10 PGND, R₂₂ ≤ 41.5 mΩ) couples into U1's return (R₁₁ ≤ 65.3 mΩ) by at most √(R₁₁R₂₂) × 2 A = 104.1 mV;
     - U1's own drop is 85.7 mV;
     - plus `VDD_3V3_RF`'s 505 mA (U30, U31 → U13 PGND, R₂₂ ≤ 32.9 mΩ) at 23.4 mV;
     - total 213.3 mV against 110 mV (249.4 mV charged naively at R₁₁).
   - **`VDD_3V3_RF` fails too:** 122.7 own + 66.7 (`VDD_5V`) + 32.0 (`VDD_3V3`) = 221.4 mV against 144.
   - **The √ bound is an upper bound.** The true coupling is unmeasured: a certified transfer bound needs a potential solve, which is new numeric code.
   - **Fixes:**
     - (i) layout: a return path for the 5 V ports that does not share U1's (the connectors beside U10, or a GND split with a single tie at U11). A placer or Sentinel-03 work order.
     - (ii) a sharper certified transfer bound, which may pass the board as drawn (unknown until built).
3. **Sentinel-03, motor outputs (5)(iii).** Four targets with no fault. The fix is the new output fault kind (stuck or open), plus four scenarios in `scenarios/sentinel-03/`, witnessed by `motor_output_unwitnessed` / `motor_timing_mismatch`. The package's scenario list and verdict move.
4. **Sentinel-01-bench, I2C `SENSORS` (5)(i).**
   - Its only cover is `pullup-3k3`, a tolerated +1100 Ω on a pull-up that declares no tolerance.
   - **Fix:** add `held-low-sda` on `SENSORS` (the BME280 is modelled; Sentinel-01's seeded set already detects it) to `scenarios/sentinel-01-bench`. The Gerbers do not move, so `ORDERED.sha256` holds.
   - The same is needed in the `sentinel-01-ready` and `four-layer` test fixtures.
5. **Sentinel-01-bench's write path (4).**
   - Its dispatch writes with `--rebind`, which refuses a FAB-NOTES that gained a line.
   - A write without `--rebind` judges readiness, which refuses U4 (2700 > 2540, 4300k (9), today's floor).
   - **Fix:** `rebind`'s reader learns (4)'s lines as derived, not bound, or one ruled write path.

## Facts found that the work order's grounds do not state

- **F-a.** Roadmap L1017 (3303 F5, the droop's loaded floor) is already done by 2304b L697; only its sourcing is open.
- **F-b.** K6 is half done (fuse + FET since 4300k (10)); the battery's own series R is what is left.
- **P2.4, wrong, and why it matters to the build.**
  - DS12117 Table 111 note 1: "When RESET is released functionality is guaranteed down to V BOR0 min".
  - The MCU row's 1620 mV, read without the footnote, makes 14 false floors on BOR-0 specifications.
  - The MCU must be judged by VBOR0's minimum, cited with the note, or left out by name.
- `supervisor.rs` L11–12 says a floor is named in "every other verdict (ruled D12)". No caller does that: only `readiness` under `--fab-out` reads it.
- Roadmap L1276 (4411 S2, split each drop limit into VDD and VSS shares) is tagged slice D but is not in this work order's grounds. **Flagged, not built.**

## CI cost

| Part | Measured or derived | Where | Wall time on `bench-package-s03` |
|---|---|---|---|
| (2), (3b) | IR arithmetic, under 1 ms per board | `solve`, readiness | none |
| (3c) | **measured:** Sentinel-03's `extract` 562 s on main, 2,175 s with the prototype's six added bounds (+1,613 s, 3.9×; debug, single-threaded, Furnace at load 8–9) | `extract`, in 1919's side lane beside prove (the side lane is 8.6 min against prove's 25 min on CI) | **Scaled 3.9× from 1919's CI extract figures:** 4.4 min becomes about 17 min, so the side lane is about 21 min, under prove. **On the slowest runner,** 531 s becomes about 34 min, so the side lane passes the prove lane and `bench-package-s03` lengthens toward its 45 min target. Built, the bounds run in parallel (as the ladder's rungs do) and only on each aggressor's worst pads |
| (4)(a) record in the board | none | `emit` | none |
| (4)(c) re-run the ladder in `extract` | +103.5 s on Furnace (the measured `emit`) | side lane | none, under the same condition |
| (5) | +4 Sentinel-03 scenarios at 6–11 s each (1919: 15 boots 88–168 s on CI); +1 on Sentinel-01-bench | campaign, side lane | Sentinel-03: none. `bench-package` (S01, 45-min limit): +~10 s |
| new tests | about 25 planted boards and scenarios; the (3c) tests need an `extract` each | tier 1 / tier 2 shards | weights measured at build |

`bench-package-s03` was 28.0–33.6 min on 1919's three built heads against the 45 min target. Only (3c) threatens it, through the side lane: it is the one cost that must be built down, not just carried.

## Decisions to rule

- **D1, (2)'s rail reading.**
  - (A) the MCU's supply only. This misses U30 on Sentinel-03, against the work order's "covers the module".
  - **(B) each part against its own rail's reset; a part on a rail with nothing resetting it is named [lean].** This matches what is physical: U12 watches `VDD_3V3` only.
  - (C) every part against the MCU's reset, as `radio::not_judged` compares.
  - Under (B) or (C) Sentinel-03's write is refused until D3.
- **D2, the MCU.** Judge it by VBOR0's minimum with DS12117 Table 111 note 1, as a field on its row **[lean]**, or leave it out by name.
- **D3, Sentinel-03's U30.** Finding 1's (i), (ii) or (iii). **Lean (iii) for this work order**, with (i) as a row for Sentinel-03's next board work order. (ii) needs a held ESP32-C3 chip datasheet.
- **D4, (3a) the source resistance.** Held nowhere. Either:
  - (a) each droop scenario declares its `source_milliohms` and `load_milliamps` with a `source`, as `battery.series_milliohms` does (load checked against the topology's domain load) **[lean]**; or
  - (b) a held figure per regulator, which no held datasheet gives; or
  - (c) (3a) is dropped from this work order, since the loaded floor itself is built.
- **D5, (3c)'s bound.**
  - (a) count each other return at the √ transfer bound. Sentinel-03 fails, finding 2.
  - (b) name each one, uncounted. Sentinel-03 passes with named conditions; that is what the work order's "or names each one it cannot count" allows.
  - (c) build a sharper certified transfer bound (new numeric code), then judge.
  - **Lean (c), falling back to (a)**, never (b) for a return whose copper is readable. The battery's return through J4 (through-hole) is named under all three.
- **D6, (4)'s record and readers.**
  - **Lean:** `emit` writes the rung and the step into the board file as a header property, so the four laddered boards' board bytes move and the Gerbers do not.
  - `extract` reads it and reports `"ladder": "not_run"` explicitly on the others, so every extract verdict of the 25 moves by that one field.
  - The two readers are a new strict FAB-NOTES reader and a MANIFEST reader, used by `--against-fab` to refuse a package lacking the lines by name.
  - `rebind` learns the lines (finding 5).
- **D7, (5).**
  - (i) a pull-up and a capacitance with no declared tolerance need a detected fault, so Sentinel-01-bench gains `held-low-sda`. Or a tolerance field on `i2c_buses[]`, outside this work order.
  - (ii) I2C `EXT` (no device) stays `no_device` (ruled (c)) **[lean]**. Making it blocking fails both packages.
  - (iii) one output-stuck scenario per motor output on Sentinel-03 (four) **[lean]**, or one for all.
- **D8, the order with AET-WO-1802.**
  - 1802 regenerates every package (version bytes, `--rebind`-shaped).
  - (4)'s lines are not `--rebind`-shaped.
  - **Lean:** 1802 merges first, 4310 merges `main`, then the architect's write dispatch.
