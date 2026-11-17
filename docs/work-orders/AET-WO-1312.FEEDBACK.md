# AET-WO-1312 FEEDBACK — bench access: test points, BOOT0 on a connector, a reset switch

Builder: Claude (Opus 5.5), worktree `~/aethelgard-1312`, branch `wo/aet-1312-bench-access`, cut from `main` 36e428b. The work order is read from docs PR #187 (`origin/claude/argon-repo-docs-synthesis-rjrb0d`, 6a5a181). Labels: **[measured]** run here, **[read]** read at source, **[inferred]** reasoned, not run.

## 0. Before building: what was read, and two premise findings

- **P1, BOOT0's strap is a wire, not a resistor [read].** `power-parts.json`'s MCU row straps `BOOT0` to the `VSS` group (RM0433 p. 138, Table 9), and `schematic_support::draw` puts pin 138 on `GND` directly. The work order's "the strap the emitter draws today stays; the pin pulls BOOT0 to its default through the strap" assumes a resistor. Through a wire, a connector cannot drive BOOT0 high (it would short the driver to ground), so §1.3's judgement fails for today's strap.
  - **What I build:** when a connector pin or a test point names `BOOT0`, the emitter draws pin 138 on its own `BOOT0` net with a pull-down resistor to `GND` whose value is judged at source (DS12117). When nothing names `BOOT0`, the wire stays, so no existing board moves. This is a deviation from "the strap stays", taken because it is the only design that satisfies both §1.3 and §2's byte identity. It is not a §7 stop: the strap can be judged at source (below).
  - **DS12117 Rev 10, Table 49, pp. 136–137 [read: the rendered page, `~/.cache/aet-1312/ds/ds-p137.png`; the text dump lost the unit]:** BOOT0 `VIL` max `0.19·VDD + 0.1` V; `VIH` min `0.17·VDD + 0.6` V; the `VPP (BOOT0 alternate function)` leakage, `0 < VIN ≤ VDD`, is at most **15 nA** (the `Ilkg` rows share one unit cell, `nA`). Both revision sections (rev Y p. 136–137, rev V) print the same BOOT0 rows [read].
- **P2, the version bump moves the golden board's provenance lines [read].** `boards/sentinel-01/**` (8 files) and `crates/aethelgard-svd-hal/tests/ui/serial-four/board.rs` carry the sha256 of `examples/sentinel-01-minimal.json`'s bytes. Writing `"schema_version": 13` into that file changes that hash, whatever the code does. So §1.1 is measured in two steps: (1) the new code with the version still 12, against `main`'s binary, on the whole corpus; (2) the bump alone, showing that only the version and the hash lines move.
- **P3, absent fields [read].** Every earlier bump made its new keys required (`INV-06`: "nothing substitutes a permissive default"). This work order says "empty or absent is today's board" and "default: false". I follow the work order's words: `test_points` may be absent and `mcu.reset_switch` may be absent, and neither is serialized when empty or false. **Question for the architect (Q1):** should they be required keys instead, as every earlier bump was? Either is a small change; the required form would add two keys to every specification in the corpus.

## 1. Predictions (committed before any measured run)

1. §1.1 step 1 (new code, version 12, no new field set): **zero bytes move** on every example, fixture, scenario and spike specification, through `validate`, `solve`, `generate` and `emit` [inferred: every new draw is gated on a field or a `BOOT0` mention no corpus file has].
2. §1.1 step 2 (the bump): only `schema_version` lines and the specification-hash provenance lines move [inferred from P2].
3. No corpus specification spells a net or a domain `BOOT0` in any case, so adding it to the reserved names refuses nothing that passes today [inferred; to measure].
4. The router on Sentinel-01 with 13 test points (both I2C lines, the four SPI lines, flash CS, console TX and RX, `VDD_3V3`, `GND`, NRST, BOOT0): **routes at 2 and 4 layers**; the outline grows by under 10 % in area; vias rise by fewer than 20; rip-ups rise [inferred: each pad is one more pin on a net that already routes; the 1.0 mm pads sit beside their anchors].
5. `extract` bounds a test pad's copper as it bounds every pad (the pad's box in its shell, Chew §7.1.2), with no pin capacitance: a bare pad has no part. I predict the I2C lines' bounds rise by under 1 pF each [inferred].
6. The BOOT0 pull-down: **10 kΩ** passes both judgements over the MCU's whole supply range (1.62–3.6 V): unplugged, `15 nA × 10 kΩ = 0.15 mV ≤ VIL max 0.408 V` at 1.62 V; driven to VDD, `VDD ≥ VIH min` at every VDD ≥ 0.73 V, at a strap current of at most 0.36 mA [inferred arithmetic on read values].
7. The tier-2 board passes `validate`, `generate`, `emit`, `check erc` and `check drc --spec` with no finding at 2 and 4 layers [inferred]. The likeliest failure: a test pad placed by the MCU's pins blocks an escape, which would show as an unrouted net at 2 layers (a §7 stop).

## 2. §1.1, measured (the stop did not fire)

**Step 1, the new code at version 12 against `main`'s binary [measured].** Both binaries built from 36e428b and from this branch; every specification file in the corpus (39: `examples/` 2, `crates/*/tests/**` 19, `spike/3400/variants` 17 and its one probe) run through `validate`, `solve` and `generate` (whose `kicad/` is `emit`'s), and every written file's sha256 compared (`~/.cache/aet-1312/v-base`, `v-new`):
- **38 of 39 are byte-identical**: every exit code, every stdout line, every emitted file.
- The 39th is the intended change: `spike/3400/probes/t8-boot0-on-swd.json` (BOOT0 on J2 pin 2) was refused `ConnectorNetUnknown` and now passes `validate`, `solve` and `generate`.
- Prediction 1 held. Prediction 3 held: no corpus file spells a net or domain `BOOT0` (only that probe names it, as a connector pin).
- The whole workspace's tests, the golden board's byte-for-byte test included, are in §4.

**The router on Sentinel-01 with a test point on every net §1.1 lists [measured]** (fixture `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json`: V0 plus 12 test points, BOOT0 on J2 pin 2, NRST on J2 pin 3, the reset switch). "The four SPI lines" are read as SCK, MOSI, MISO and the flash's CS, so the list is 12 distinct nets, not the 13 prediction 4 counted. Figures are read from the emitted board by a scratch reader (`~/.cache/aet-1312/stats`), which also re-routes the read board and gets the same copper; V0 was emitted by `main`'s binary.

| Board | Outline (mm) | Area (mm²) | Parts | Track (mm) | Segments | Vias | Rip-ups |
|---|---|---|---|---|---|---|---|
| V0, 2 layers | 44.10 × 48.50 | 2138.8 | 34 | 597.4 | 549 | 101 | 18 |
| Bench, 2 layers | 46.35 × 52.45 | 2431.1 (+13.7 %) | 48 | 761.9 | 679 | 130 (+29) | 10 |
| V0, 4 layers | 44.10 × 48.50 | 2138.8 | 34 | 413.8 | 393 | 104 | 1 |
| Bench, 4 layers | 46.35 × 52.45 | 2431.1 (+13.7 %) | 48 | 567.5 | 496 | 124 (+20) | 1 |

- **It routes at 2 and at 4 layers.** All eight steps pass on both: `validate`, `generate`, `emit`, `check erc`, `check drc --spec`, `check drc --spec --schematic` (parity), `check support` and `extract`. None has a finding.
- Prediction 4 was wrong in three of four figures. The area grew by 13.7 %, not under 10 %. Vias rose by 29 at 2 layers, not under 20. Rip-ups **fell** at 2 layers (18 to 10) instead of rising. I have no measured explanation for the fall; the placement differs, so the search differs.
- **`extract` on the bench board [measured]:** each test pad adds 897 fF (2L, I2C) to the line's `pads` term, so prediction 5 held for the pad. The line's whole bound rises by up to 17.9 pF (I2C_SDA at 2L: 43.3 to 61.3 pF), from the track and vias routed to the pad. Every line stays under its declared bound (I2C 100 pF, SPI 30 pF) at 2 and 4 layers.

## 3. What was built, and the decisions taken inside the work order

**Specification (`aethelgard-ir`).**
- `test_points: [{net, reference}]` (`bench::TestPoint`) and `mcu.reset_switch` (bool), each optional and not written back when empty or false (P3, Q1).
- Step 15a (`bench::validate`), after the connectors' step 15, refuses by name:
  - `TestPointNetUnknown`: the net is not one a connector pin may name. The test point and the connector read one list, `connectors::nameable`, so the two cannot disagree. That list is a declared signal, a domain, `GND`, `SWDIO`, `SWCLK`, `NRST` or `BOOT0`.
  - `TestPointReferenceTaken`, for either of two collisions:
    - another reference of the specification has the same KiCad key;
    - the reference is one the emitter numbers: `U`, `R`, `C` or `SW` and a number, as `ConnectorReferenceReserved` holds for a connector, with `SW` added for the switch.
  - `TestPointNetRepeated`: one net carries two test points.
- Identifiers are checked at step 2, as `test_points[i].net` and `.reference`.
- **`BOOT0`** joins `DEBUG_NETS`, so a connector pin or test point may name it, and step 11 refuses a declared net or domain spelled like it.
  - It joins `names::own_nets` only where it is drawn, that is when a connector pin or test point carries it.
  - The existing guard `the_nets_validate_reserves_are_the_emitters_own` (every listed net is drawn) caught my first version, which listed it always.
- **The switch** is numbered `SW<n>` as the MCU's `U<n>` is: the first `n` whose `SW<n>` the specification does not spell (`bench::switch_reference`). Step 20 refuses another spelling of it (`sw1`, `SW01`) as `EmitterReference`.
- **The BOOT0 judgement** (`bench::judge_boot0_pull_down`), from DS12117 Rev 10, Tables 49 and 50, read at source:
  - **unplugged:** leakage times R must not exceed VIL max at VDD min;
  - **driven:** a driver at VDD − 0.4 V must reach VIH min at VDD min;
  - **drive current:** VDD max / R must not exceed the 4 mA at which Table 50 states VOH.
  - It is judged over the MCU row's 1.62–3.6 V. The pull-down is 10 kΩ. It passes between 900 Ω and 27.1 MΩ.

**Emitter (`aethelgard-eda`).**
- Bench parts are drawn after every other part (`schematic::bench`), so no other reference moves:
  - BOOT0's 10 kΩ pull-down, anchored by pin 138;
  - the switch, `Switch:SW_Push` on `Button_Switch_SMD:SW_SPST_PTS810`, valued `PTS810 SJM 250 SMTR LFS`. C&K's part is listed Active by digikey.sg, as `3400` FEEDBACK T8 read it.
  - each test point, `Connector:TestPoint` on `TestPoint:TestPoint_Pad_D1.0mm`, drawn on its net as a connector pin is.
- The four files are vendored byte for byte from KiCad's GitLab at tag 10.0.7, and each file's git blob SHA matches the API's [measured]. They are pinned in `library.rs`, and the README rows carry the licence (`RUL-11`).
- **Placement:** test points and the switch are not ICs, so they are never an anchor, and they are placed after every other part except the connectors.
- **Two readers taught one shape each:**
  - the placer reads an `fp_circle` courtyard or silkscreen as its box;
  - `route::read` takes a circular courtyard as the square that holds it. A courtyard only keeps vias out, so the rule is kept wider, never weakened. Arcs are still refused.
- **BOM and parity.** KiCad's `TestPoint` symbol is `in_bom yes`, and its pad footprint is `exclude_from_bom`. Parity found 12 `footprint_symbol_mismatch` warnings on the first tier-2 run [measured]. Each test point's symbol instance is now written `in_bom no`, and test points are left out of `emit`'s BOM, because a bare pad is bought and placed by no one.
- **`extract`:** a test point is `Load::TestPad`. Its pad's copper is bounded as every pad's is (Chew §7.1.2, the existing trial), and it carries no `Ci`, because no part is mounted on it. A probe clipped to it is outside the bound, as any off-board load is. **Answer to §1.2:** the bound is declared, and the net is not refused. `extract`'s footprint reader admits `fp_circle` off copper, as it admits `fp_arc`.
- **Support gate:**
  - When BOOT0 is carried, its strap is judged as BOOT0 on a net of its own, shared with no other MCU pin, with exactly one populated resistor to the strap's ground net, whose value passes the DS12117 judgement. The record line names the resistor and its value.
  - A connector pin on `BOOT0` is judged against the boot pin's net, as NRST is.
  - New findings: `test_point_off_net` (each test point's pad on its declared net) and `reset_switch_wrong` (a switch from the MCU's NRST to `GND` exactly when declared).

**Not changed, and why.**
- `spike/3400/**` stays at version 12. It is the 3400 spike's record, nothing builds or tests it, and builds delete `spike/` (roadmap). The bump makes its specifications refused by version.
- `aethelgard-cli`'s `extract` conditions are not changed (the CLI is `2302`'s). See Q2.
