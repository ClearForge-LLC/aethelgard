# AET-WO-1211e — FEEDBACK: pin devices validated against the MCU and the part (A1, A2, `bulk`)

**Branch:** `wo/aet-1211e-pin-device-validate` from `main` `adb8f61a`. Merged in on 2026-11-22: `main` `ddfa64d6` (4240b, specification 23 / solved 15), `c1e444dd` (1211d, 1214), `7fc81640` (1211b, solved 16), `122378c7` (2307) and `bf497733` (1215, 1319, 4240d). **Builder:** Claude (Opus 5.5 orchestrating; one Sonnet worker read DS12117 Table 8 from the rendered pages, blind to the text extraction; one Opus adversarial reviewer). Each report was checked at source before use. Written 2026-11-20 to 2026-11-22.

**Status: READY** (the package is committed from dispatch 44986875477, §9). **The answer, in five lines:**
- **A1:** a declared pull is judged against what the MCU pin takes in operation (DS12117 Rev 10 Table 13, p. 105), with VDD at `mcu.supply`'s minimum and a pull to `mcu.supply` itself exempt. Both of the WO's choices (Table 10, VDD at its maximum) were corrected by ruling.
- **A2:** the DRV8876 is refused with VREF on GND, with an IPROPI no analog input terminates, or with a trip below its declared full scale.
- **`bulk`:** required where the part leaves its value unstated (the DRV8876's VM). It is specification **24**, after 4240b's 23.
- **No committed specification or fixture is refused.** Measured first, then re-measured on the final rule over 65 files. The null sweep shows 33 files, with no emitted byte moved.
- **Mutants: 44 of 44 killed.** The bench package is committed from dispatch 44986875477, moved by its sha lines alone (§9).

## 0. How it was made

- Predictions were committed before the measurement ran: `76e09a95`.
- The measurement and Table 8, read twice: `f6b63eaf`.
- Seven leans were ruled at 19:30Z, and the adversarial pass's D-A1/D-A2/D-A3 at 19:58Z (2026-11-20). The architect's note of 2026-11-22T01:40Z named specification 24.
- The kit is in `spike/1211e/`:
  - `table8.py` extracts Table 8;
  - `measure.py` takes `--table10` (the first run) or `--table13` (the final rule);
  - `rows.py` writes the data rows;
  - `mutants.py` runs the mutants;
  - `compare.py` judges the null sweep, which runs `spike/1211a/measure/run.sh`.
  - Every output is in `out/`.

## 1. The measurement (§1.1)

Predictions were committed before any run (`76e09a95`, `spike/1211e/PREDICTIONS.md`). Results are in `spike/1211e/RESULTS.md` (`f6b63eaf`). **No committed specification is refused by any rule, so there was no stop.** The final rule was re-measured on the merged tree: 65 files, 132 I2C pulls, 1 crossing CS pull and 4 pin-device pulls, every one of them on `mcu.supply`. Nothing committed is refused (`out/measure-committed-spec24.tsv`). The only row refused is `spike/1211/fixtures/d1-drv8876.json`, the 1211 spike's frozen record (`schema_version` `"22-or-23"`), which no test reads. After main `c1e444dd`, `spike/1211d/adv/` brings 1211d's three adversarial boards (`nfault_to_vmotor`, `d1_vref_gnd`, `d1_vref_low`; `out/measure-committed-c1e444dd.tsv`). They are refused, as the boards these rules were written to refuse. They are spike evidence that no test or CI job reads.

- **A1.** 128 I2C line pulls, 1 crossing CS pull and 4 pin-device pulls. The largest is 3465 mV, against at least 4000 mV for any structure. The 8 hand I2C pins are all FT_f, FT_fa or FT_fh.
- **A2.** The only reference-sets-trip part is D1's DRV8876, with VREF on `VDD_3V3` (3135 mV at least). At 1.9 A with AERR's 5 %, IPROPI sources 1995 µA into R2, which is at most 1555 Ω (1 % tolerance, 100 ppm/K over 65 K, 2 % drift). That is 3.102 V, inside VREF with 33 mV to spare.
- **`bulk`.** D1 is the one fixture the rule refuses as committed; it now declares a stand-in. No specification under `examples/` carries a pin device.
- **Positive control.** The same script over `1211d`'s adversarial specifications (`spike/1211e/out/measure-1211d-adv.tsv`) flags every case the rules exist for: nFAULT pulled to 6.3 V and to 12.6 V, VREF on GND, VREF at 0.5 V (it trips near 305 mA), and D1 with no bulk.
- **DS12117 Table 8 was read twice.** `table8.py` extracted it from the held PDF's text, with each row's LQFP144 column equal to the vendored pin data's position. A Sonnet worker read the rendered pages without seeing that extraction. The two agree on all 117 non-supply rows, structure and page. The text read had missed `FT_h` + `a` (`FT_ha`) wrapping onto two lines until a spot check found it.

## 2. What was built (§1.2–§1.4)

- **The data (§1.2): the MCU row of `data/power-parts.json`, `kind.mcu.io`** (power-parts schema 6). It holds:
  - each of the pin table's 114 I/O pins, by canonical name, with its structure as Table 8 prints it and its page;
  - FT's and TT's input in operation, from **Table 13, p. 105** (= Table 111, p. 209): FT `Min(VDD, VDDA, VDD33USB) + 3.6 V, < 5.5 V` (held as at most 5499 mV), TT `VDD + 0.3 V`;
  - note 3's threshold, above which the internal pulls must be off (VDD + 0.3 V).

  **Why there (ruled 19:30Z, lean 3):** the pin table is built in `aethelgard-svd-hal` from ST's open pin data, which carries no I/O structure. `validate` cannot reach svd-hal, and svd-hal is outside the fence. The MCU row already holds the MCU's other DS12117 facts (VDD, BOR, RAIN), and `validate` already reads it. `spike/1211e/rows.py` writes the rows from the measured TSV, so they are reproducible.
- **The structures:** 102 FT and 12 TT: PA4, PA5, PA7, PB1, PC2, PC3, PC4, PC5, PE7, PE8, PE9 and PG1.
  - **PC2 and PC3 are the `PC2_C` and `PC3_C` pads.** The open pin data types them `I/O`, and svd-hal's canonical name drops the `_C`. Table 8 (p. 67) types them `ANA`, `TT_a`. They are held at TT_a, the stricter limit (F4).
- **A1 (`PullAboveMcuTolerance`, `McuPinStructureUnheld`).** Each input's declared pull resistor is judged on the MCU's side, after the part's side (`PullOverDrive`), with VDD at `mcu.supply`'s minimum.
  - **A pull to `mcu.supply` itself is exempt**: on the same rail VIN cannot exceed VDD (ruled D-A1).
  - Any other pull at or below VDD + 0.3 V, the least any held structure takes, needs no lookup. Above it:
    - an unpinned signal is refused, because the solver may place it on a TT pin; the message says to pin it by hand (lean 2);
    - a pin holding an internal pull from reset is refused (PA13, PA15 and PB4 up, PA14 down; RM0433 Rev 9 p. 542; Table 13 note 3);
    - a hand pin is judged by its held structure, or refused for having none. No structure is assumed.
  - The refusal names Table 13 and the pin's structure, with its page.
  - **The MCU's own pull-up is not judged by A1:** it is on its own supply, so the pin sits at VDD. The call was removed in the reduction, since no mutant there could be killed.
- **A2 (`TripReferenceGrounded`, `TripSenseUnbound`, `TripBelowFullScale`).** It is driven by a row field, `trip: {reference, sense, page}`, on the DRV8876 (parts schema 12; SLVSDS7B p. 13, Eq. 3, and Table 6: every IMODE state chops).
  - VREF on `GND` is refused.
  - IPROPI with no analog input terminating it is refused (ruled D-A2). A specification cannot tie a `current_output` pin anywhere else, and SLVSDS7B (pp. 12–13) admits only R_IPROPI to ground, or a tie to GND with VREF above GND.
  - Otherwise the sense voltage at each declared full scale on IPROPI (the error band's worst edge, R at its highest, computed exactly as the ADC rule computes it, through one helper, `full_scale_uv`) must not exceed the VREF domain's minimum. Equality passes: the part then chops at the full scale, not below it.
- **`bulk` (`BulkUndeclared`, `BulkNotAllowed`, `BulkUnderRated`).** `pin_devices[].bulk` is a required key: `null`, or `{reference, part, source}` (lean 6).
  - A row field `bulk_unstated: {supply, page}` makes it required. It is set on the DRV8876's VM (SLVSDS7B p. 3, p. 10 Table 1 CVM2, §9.1 p. 31).
  - Every other row refuses one. No held row allows an optional bulk, so that arm of the WO was not built.
  - The part is a `capacitor` or `bulk_capacitor` row (ruled D-A3), rated strictly above the supply domain's maximum. Both are in mV, the maximum rounded up.
  - Its reference joins step 8 and step 20, and its source step 2.
  - D1's fixture declares `C2`, `GCM32EC71H106KA03` (10 µF, 50 V), with the source "D1 fixture stand-in, not a design: SLVSDS7B 9.1 p. 31 leaves CVM2 to system testing".
- **Kind names** (§1.3):
  - A1: `PullAboveMcuTolerance`, `McuPinStructureUnheld`;
  - A2: `TripReferenceGrounded`, `TripSenseUnbound`, `TripBelowFullScale`.
- **Specification 24** (§1.4), by `spike/4240a/kit/renumber.py . 22 23 14 14`. The solved topology is unchanged at 14.

## 3. Rulings and corrections

- **The WO said VDD at the domain's maximum. That was wrong, and it was corrected (ruled 19:30Z, lean 1).** Each operating limit is `Min(VDD, VDDA, VDD33USB) + …` (Table 13 p. 105; Table 10 p. 103 as well), which is least at the lowest VDD. A1 takes `mcu.supply`'s minimum. The emitter ties VDDA, VDD33USB and VBAT to `mcu.supply` (`aethelgard-eda` `schematic_support.rs`).
  - The test `ft_takes_its_supply_at_the_minimum` holds the edge. With the MCU at 1.9 V ±5 %, it refuses 5454 mV, which is above 1805 + 3600 and under the 5499 mV that VDD's maximum would give.
- **The WO said "that structure's absolute maximum" (Table 10, p. 103). That was the wrong table, and it was corrected (ruled D-A1, 19:58Z).** P. 103 calls Table 10's values "stress ratings only and the functional operation of the device at these conditions is not implied". A pull is permanent, so it is an operating condition, bounded by Table 13 (p. 105). The adversarial pass found the gap: 3672 mV on a TT pin, a second 3.3 V rail on a TT pin, 6.0 V on an FT pin, and 3876 mV on PB4 all passed Table 10. Each is now refused.
- **The first build's measurement (Table 10) and the final one (Table 13, with the same-rail exemption) both refuse no committed specification.** The files are `out/measure-committed.tsv` and `out/measure-committed-table13.tsv`. Every committed pull is on `mcu.supply`. The positive control flags the same cases under both.
- **I2C bus and crossing CS pulls are not walked (lean 4).** Step 5a puts both on `mcu.supply`, which the same-rail exemption now passes by construction anyway. The evidence is the measurement's 129 rows.
- **A DRV8876 with no `analog` entry on IPROPI:** lean 7 (the GND rule only) was replaced by D-A2 (`TripSenseUnbound`) on the adversarial pass's evidence.
## 4. Mutants (§1.5)

`spike/1211e/mutants.py` makes 44 mutants, one per rule, guard and row check. All **44 are killed**, each by a failing test and none by a build break (`out/mutants-result.json`).

| Group | Count | What the mutants cover |
|---|---|---|
| A1 | 17 | the exemption both ways; the floor and its edge; the unpinned refusal; note 3 and PA14; the structure's limit and its edge; VDD and the pull at the wrong edge; FT's supply term and cap; every structure read as FT; an unheld pin assumed FT; the call itself |
| A2 | 7 | the GND tie; the unbound sense; the trip and its edge; VREF at its maximum; the sense filter; the call |
| `bulk` | 10 | each arm of the match; the rating and its edge; the domain's edge; the row's kind (and the electrolytic); the call; the reference and source checks |
| row parsers | 10 | each refusal of `io`, `trip` and `bulk_unstated` |

- The edges are held to the millivolt or microvolt:
  - 3435/3436 mV for the floor;
  - 5499/5500 mV for FT's cap;
  - a VREF minimum of 3192 mV against exactly 3,192,000 µV;
  - a 16 V rating against exactly 16,000 mV.
- One guard has no mutant. Note 3's threshold (`v > pulls_off`) equals the floor with the held rows, so a mutant removing it cannot be killed. It is labelled `belt-and-suspenders:` at the site.

## 5. Adversarial pass (§5)

An Opus reviewer worked in its own worktree at `e4d0f877`, given the code and §5, not my conclusions. Its report is `~/.cache/aet-1211e/adversarial.md`; its probes are in `adv-probes/`. It found 2 defects, 3 gaps and 7 notes.

- **Defects:**
  - A1 judged a permanent pull by stress ratings. Ruled D-A1: Table 13, built.
  - A2 was skipped with IPROPI unbound. Ruled D-A2: `TripSenseUnbound`, built.
- **Gaps:**
  - The electrolytic bulk row was refused. Ruled D-A3, built.
  - `bulk` judges the rating only (F6).
  - The unpowered MCU (F1, F5).
- **Notes:** F4, F7–F10.
- **The WO's §5 list:**
  - **Paths the rule does not walk:**
    - Connectors: off-board voltage is not modelled, as before.
    - A second device: step 11 lets no net be declared twice.
    - Straps: on device pins, not MCU nets.
    - `power.sense`: bounded by step 16's VREF+ rule.
    - Control wires: F7.
    - I2C and crossing CS pulls: on `mcu.supply` by step 5a, so exempt.
  - **VDD at the wrong edge:** no swap anywhere in the new code. The mutants A1.11, A1.12, A2.5 and B.5 hold each edge.
  - **The unpowered MCU:** F1.
  - **Unit and rounding:** both sides are in mV. The domain maximum rounds up and the minimum down. A very large domain saturates without a panic.
- **No panic is reachable from input.** The one `unreachable!` in `trip()` follows `straps()`, which admits only a tie on a `strap_analog` pin.

## 6. Predictions beside results

| # | Prediction (`76e09a95`) | Result | Holds |
|---|---|---|---|
| 1 | No declared pull exceeds its pin's tolerance | 128 + 1 + 4 pulls, at most 3465 mV, against at least 4000 mV (Table 10); every one is on `mcu.supply` (Table 13, exempt) | **holds** |
| 2 | Every hand pin a pull lands on has a held structure; none is TT | 8 hand I2C pins, all FT_f, FT_fa or FT_fh | **holds** |
| 3 | D1's DRV8876 is the only trip part; its VREF is on a domain and meets the full scale | 3.102 V against 3.135 V, with 33 mV to spare | **holds** |
| 4 | D1 is the one fixture `bulk` refuses; no example carries a pin device | D1 only (it now declares a stand-in) | **holds** |

## 7. Findings, not fixed here

- **F1 · An unpowered MCU beside a powered motor rail: a §7.1 row for a power-sequence WO (ruled 19:30Z, lean 5).** DS12117 states no separate unpowered-pin limit. What it states:
  - Table 10 (p. 103; = Table 108, p. 207): FT inputs at `Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)`. With VDD at 0 that is **4.0 V**.
  - Note 1 (p. 103; = p. 208): "All main power (VDD, VDDA, VDD33USB, VBAT) and ground (VSS, VSSA) pins must always be connected to the external power supply, in the permitted range."
  - §3.5.1 (p. 27) sequences only the supplies: "When VDD is below 1 V, other power supplies (VDDA, VDD33USB, VDD50USB) must remain below VDD + 300 mV."

  No specification declares a power sequence, so A1 judges every pull with the MCU powered. **The bench risk:** a board whose VM rail comes up before 3V3 can hold a pin-device output, pulled to a domain above 4.0 V, on an unpowered MCU pin beyond its rating. Even a pull to 3V3 is harmless only because 3V3 is itself the MCU's supply.

  **Proposed row:** "A pull, divider or driver output from a domain that can be up while `mcu.supply` is below 1 V puts more than 4.0 V on an FT pin (DS12117 Rev 10 Table 10 p. 103 at VDD = 0; note 1). Wanted: a declared power sequence (which domains may lead `mcu.supply`, from what source), and A1 judged at VDD = 0 for every domain that may lead. Owner: a power-sequence work order."
- **F2 · A GND tie for a `current_output` pin (ruled D-A2).** `TripSenseUnbound` refuses a DRV8876 whose IPROPI no analog input terminates. SLVSDS7B p. 13's honest way to turn regulation off (IPROPI on GND, VREF above GND) cannot be declared today, because a strap on IPROPI is `SignalKindMismatch`. **Proposed row:** "A `current_output` pin tied to `GND` (no feedback), admitted with its reference above `GND`, which turns the trip off. Wanted: a strap kind for it, and A2 skipping the trip when it is declared. Owner: the next IR work order on pin devices."
- **F3 · The eda arm that draws `bulk`** (lifting `PassiveValueUnstated{CVM2}` for D1) is the follow-up after `1211d` merges, as the WO says. Until then D1 validates and solves, and eda still refuses to draw it. **Proposed row:** "the eda arm draws `bulk` (lifting `PassiveValueUnstated{CVM2}`). Owner: the follow-up after `1211d`."
- **F4 · PC2 and PC3** are the `PC2_C` and `PC3_C` pads on the LQFP144. They are `I/O` in the open pin data and `ANA` in Table 8, and Table 11 limits a Px_C pin to 1 mA sunk (p. 104). A1 holds them at TT_a. Nothing else in the engine knows they are analog-switch pads; whether the solver should offer them as GPIO is a question for the pin table's owner. **Proposed row:** "`PC2_C`/`PC3_C` in the pin table: analog-switch pads (Table 8 `ANA`, Table 11 1 mA sunk), offered to the solver as GPIO."
- **F5 · Table 11 note 4 (p. 104)** calls any VIN above VDD "positive injection". It adds to F1: a rail that leads `mcu.supply` injects current into the MCU before VDD exists. **Proposed row:** F1's row, with Table 11 note 4 and Table 48's injected-current limits added to its citations.
- **F6 · `bulk` judges the rating alone.** An 18 pF C0G validates as a motor-rail bulk. This is the WO's design: no value comes from any source but the declaration, and `source` must say why. **Proposed row:** "a bulk capacitance floor, where a held document states one."
- **F7 · A power-tree control `wire` can join an MCU net** (adversarial A6). PG wired to D2's nFAULT net validates. It adds no voltage today, because every wirable control is open drain or an input. Separately, a buck's EN wired to a bridge output validates. **Proposed row:** "a control wire's net judged against what else drives it."
- **F8 · Pre-existing, from 1211a: the ADC reservoir's rating is compared with VDDA's minimum, where its maximum belongs** (`crates/aethelgard-ir/src/pin_devices.rs:1190`, in `analog()`). It is harmless with the 50 V parts held. Not fixed here (outside the WO's three rules). **Proposed row:** "the reservoir judged against VDDA's maximum (pre-existing, from 1211a)."
- **F9 · Order.** `analog()` runs before `trip()`, so an ADC refusal hides A2's kinds on the same device. The input is still refused; only the kind reported differs (adversarial A8). **Proposed row:** "A2 judged before the ADC rule, so the trip's kind is named first", taken when a work order next touches step 24's order.
- **F10 · An unknown `mcu.part`** is now refused by a pin device with an off-rail pull, at the path `pin_devices[0].mcu`, which names no real field. `analog()` already did this. With D-A1's exemption, a pull to `mcu.supply` no longer reaches it (adversarial A11). **Proposed row:** "an unknown `mcu.part` refused once, at `mcu.part`, before step 24."
- **F11 · Environment, by inference:** `cargo test --workspace` with `CARGO_TARGET_DIR` outside the repository fails `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named` with `BaseOutsideRoot`. Its `connectors.json` base is under the target dir's `tmp`, outside the working directory. It failed the same way before this work order touched any CLI file (the first workspace run, at `e4d0f877`). CI keeps the target in the repository and is the confirmation. **Proposed row:** "the test's scratch under the repository, not `CARGO_TARGET_DIR`."
- **F12 · `fab/sentinel-01-bench/BENCH-EXT-BMP581.md`** cites `examples/sentinel-01-ext-bmp581.json`'s sha256 at specification 23. It is 3403's bench record, written before the bench and never edited (its own rule), and no test binds it. It is left as written; the specification's bytes moved by their version line alone.
- **F13 · 1211d's eda guard on an undeclared IPROPI** (`PinDeviceNotDrawable`, `aethelgard-eda`) is now behind `validate`'s `TripSenseUnbound`. A specification no longer reaches it through `validate`. 1211d's test `an_undeclared_nfault_is_open_and_an_undeclared_ipropi_is_refused` now asserts `TripSenseUnbound` at validate, which is the same intent at the earlier stage, resolved at the merge of `c1e444dd` as the architect asked. Whether eda keeps its guard, labelled `belt-and-suspenders:`, is eda's owner's call: that is outside this fence. **Proposed row:** "eda's IPROPI guard labelled or removed."
- **F14 · `versions::CHANGES` row 24** (1215, merged as `bf497733`) is written: it requires `pin_devices[].bulk` and lists the rules `PullAboveMcuTolerance`, `McuPinStructureUnheld`, `TripReferenceGrounded`, `TripSenseUnbound` and `TripBelowFullScale`. The bulk kinds are not listed, because they judge only the new key. 1215's literals moved to 24 (`[21, 22, 23, 24]`, `2..=24`, `not 24; `). `spike/1215/in/v24-newer.json` is 1215's spike record, named for a version that was newer when it was written and is now current. Nothing reads it, so it is left as written.

## 8. Gates, fence, reduction pass

- **Gates:**
  - `cargo fmt --all -- --check` passes.
  - `cargo clippy --workspace --all-targets -- -D warnings` passes, at `0d349342`.
  - `cargo test --workspace` (Furnace, target outside the repository, at `0d349342`, after the package and the merge of `bf497733`, 2026-11-22): 157 binaries, 2668 passed and 1 failed: `fab_tier1`, which is F11.
    - Before the merges, two goldens were fixed and re-run green: `validate::refuses_unsupported_schema_version` (its list now holds 23 and 25) and `order_2203`'s digests.
    - Before that merge, two goldens were fixed and re-run green: `validate::refuses_unsupported_schema_version` (its list now holds 23 and 25) and `order_2203`'s digests.
- **The null sweep** (§2): main's binary on main's tree against the branch's on the branch's. 33 files; 0 exit codes moved; `validate`, `solve` and `emit` output equal once each sha256 and run directory is masked; **every emitted file byte-identical**. Taken twice:
  - against `ddfa64d6` (`out/sweep-result.json`): 216 generated files, each moved by one sha line;
  - against `7fc81640`, after 1211b, from `80cd4db6` (`out/sweep-result-7fc81640.json`): 223 generated files, each moved by one sha line.
- **Goldens moved by the bump:**
  - 26 files by their sha256 line alone (`boards/sentinel-01*`, the svd-hal UI boards and topology, the hwdef fixtures). `boards/sentinel-01` and `boards/sentinel-01-bench` each equal, file for file, what the branch's `generate` writes for their specifications (`diff -r` against the null sweep's trees). The svd-hal and hwdef goldens are regenerated and compared by their own tests;
  - `solve_cli`'s two digests (re-taken again on main's solved 16 at the merge of `7fc81640`) and `order_2203`'s three text digests, each shown equal to main's with the old sha put back (measured, and recorded at each constant; `order_2203`'s boot image did not move);
  - the cli README's example line.
- **Fence:** `crates/aethelgard-ir` (rules, data, schema, README, tests); the version literal and the files `renumber.py` touches; the sha-bearing goldens; `spike/1211e/`; this file. Nothing in `aethelgard-eda`, the generator, the twin, chaos, workflows or `ci/` changed.
- **Reduction pass (diff): −37 lines removed** (the 34-line copy of the sweep script, 2 dead lines in `measure.py`, 1 in `table8.py`; the MCU-pull call became a 2-line comment saying why there is none).
  - 3 duplicates removed:
    - the MCU row lookup, now one `mcu_row`;
    - the full-scale computation, now one `full_scale_uv` for the ADC rule and A2;
    - the sweep script, now 1211a's `run.sh` instead of a copy.
  - 3 dead paths removed: A1's call on the MCU's own pull-up, a `sense` loop in `measure.py`, a namespace line in `table8.py`. The outputs are byte-identical after each removal.
  - 1 protected copy kept: note 3's threshold, labelled.
  - 0 findings outside the scope.

## 9. The package

`fab/sentinel-01-bench/package` is the artifact of the architect's write=true dispatch, **run 44986875477** on `80cd4db6`, committed byte for byte (`940d26d7`).
- Only `FAB-NOTES.md` and `MANIFEST.json` moved, and each moved by its sha lines alone:
  - the specification's sha (`81e1f19a…` → `d89d62e7…`);
  - the topology's sha, which embeds it;
  - the verdict's sha;
  - FAB-NOTES' own hash.
- The ELF, every Gerber, the drill, the BOMs and the placement are byte-identical.
- `bench_board_tier1` passes on it.
