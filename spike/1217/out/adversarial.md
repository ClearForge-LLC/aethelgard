# AET-WO-1217 adversarial pass (beb2da1b vs 22a1b3c0)

Binaries: new = `/home/forge/.cache/aet-1217/adv-src/target/debug/aethelgard` (beb2da1b, clean tree); base = `/home/forge/.cache/aet-1217/aethelgard-base` (22a1b3c0).
Probes: `/home/forge/.cache/aet-1217/adv/probes/<name>.json`, written by `adv/bin/probe.py` (one Python edit of a committed file per probe). `adv/bin/both.sh NAME CMD` runs the probe on the new binary and a copy with `schema_version` 25 on the base binary. Output below is `kind @ path-or-id`.

## CONFIRMED

### A1: defect. With the GND tie, "VREF above GND" is checked by the net's name only
- Claim: once `straps.IPROPI` is tied to `GND`, `trip()` refuses VREF only when the strap literally names `GND`. A VREF on a declared domain whose minimum is 0 mV is admitted. That is the configuration SLVSDS7B p. 13 says does not turn regulation off.
- Probe: `probes/a1-vref-low-tie.json` (domain `VREF_LOW` 1 mV ±500‰; `minimum_millivolts` = 1×500/1000 = 0). The control is `probes/a1-vref-low-notie.json`.
- Direct runs: `aethelgard validate --file probes/a1-vref-low-tie.json` gives `{"ok":true,"project_name":"drv8876-ph-en","voltage_domains":3,"net_constraints":0}`, exit 0. `solve` gives a topology, exit 0. Base (v25 copy): `SignalKindMismatch @ pin_devices[0].straps.IPROPI`.
- Control, with no tie and the same domain: `TripBelowFullScale @ pin_devices[0].straps.VREF` on both binaries.
- Why it matters: the tie is what admits this. The early return in `trip()` (`if self.d.straps.contains_key(&t.sense) { … return Ok(()) }`) skips every voltage judgement of the reference. Only `to == GND` stands between it and the F2 condition. The fix is to require `minimum_millivolts(VREF's domain) > 0` (or a cited floor) on the tie path. Reach is low, since it needs a near-zero domain, but the input is admitted and it is wrong.

### A2: gap. The pad guard on a hand-edited topology is bypassed by editing its `mcu` string
- Claim: `SolvedTopology::validate` takes its pads from `solve::analog_pads(&self.mcu)`, which returns an empty list for any part without an MCU row. Nothing in `validate` or in `check pinout` ties `topology.mcu` to a known row (`grep` shows `pinout.rs` and `aethelgard-pincheck` never read `.mcu`). So a topology with `SPI_MISO` on `PC2` passes validation once `mcu` is renamed.
- Probes: `probes/a2-topo-assign-PC2-mcu743.json` (`mcu` "STM32H743ZITx") and `probes/a2-topo-assign-PC2-mculower.json` (`mcu` "stm32h753zitx"). Each is `examples/sentinel-01.json` solved, with `assignments.SPI_MISO.pin` set to `PC2`.
- `aethelgard check pinout --topology probes/a2-topo-assign-PC2-mcu743.json --elf probes/junk.elf` gives `{"kind":"Truncated",…"5 bytes is shorter than an ELF32 header"}`. That means topology validation passed and the run reached the ELF.
- Same topology with the real `mcu` (`probes/a2-topo-assign-PC2.json`): `{"kind":"AnalogPadPin","id":"PC2",…}`.
- `generate hwdef` on the renamed topology is stopped, but by its own `mcu_row` (`UnknownMcu`), not by the pad rule.
- Why it matters: lib.rs documents the new variant as "so a topology edited after `solve` is refused too". `check pinout` judges against the STM32H753 package whatever `mcu` says.
- SUSPECTED: with a real ELF that configures PC2 as SPI2_MISO, `check pinout` would pass end to end. Not run: no such ELF was built.

### A3: gap. F10 refuses content that version 25 admitted, and `CHANGES` row 26 omits it
- Claim: a DRV8833 board (no ADC input, so step 24 never needed the MCU row at base) with an unknown `mcu.part` validated at version 25 and is refused at 26.
  - `CHANGES` row 26 lists only `AnalogPadPin`, yet row 12 sets the precedent of listing an existing kind that a version tightens (`SerialBaudTolerance`, `PullUpBelowSinkFloor`).
  - `spike/1217/RESULTS.md` §3 says "only D1-like probes move", which these probes contradict. Its measurement used committed files only, and `d2` already refuses.
  - The README's step-24 row does document F10. Its version-26 paragraph does not.
- Probe `probes/a3-d2-ok-mcu743.json` (d2 with `part_limits.input_pwm_max_hz` declared, then `mcu.part` STM32H743ZITx). Direct runs:
  - base `validate --file probes/a3-d2-ok-mcu743.v25.json` gives `{"ok":true,"project_name":"drv8833-in-in",…}`, exit 0;
  - base solve gives `UnknownPart` "no pin table for part `STM32H743ZITx`", exit 2;
  - new validate and solve give `PowerPartUnknown @ mcu.part`, exit 2.
  - Control `probes/a3-d2-ok.json`: validate and solve exit 0 on both binaries.
- Why it matters: the refusal follows the ruled narrow F10. What is missing is the record, `VersionChange.rules` ("refusal kinds this version added that judge content a file of the version before could already hold"). The `solve` kind change (`UnknownPart` to `PowerPartUnknown`) is also the effect the global option was rejected for (`solve_cli::an_unknown_part_is_exit_2`), now on every pin-device board.

### A4: gap. F9 turns a malformed ADC entry into `TripSenseUnbound`, whose message is then false
- Claim: when `analog[0].device_pin` is misspelt or names the wrong kind of pin, the trip rule now runs first and reports `TripSenseUnbound` at `.analog`. Its message says "no analog input terminates [IPROPI]: its resistor to ground is not declared", but an entry and its resistor are declared. The precise base refusal is lost, including `UnknownDevicePin`'s list of the row's pins.
- Probes and `both.sh … validate`:
  - `probes/a3-f9-analog-unknownpin.json` (`device_pin` "ipropi"): `base=UnknownDevicePin @ pin_devices[0].analog[0].device_pin`, `new=TripSenseUnbound @ pin_devices[0].analog`.
  - `probes/a3-f9-analog-on-vref.json` (`device_pin` "VREF"): `base=SignalKindMismatch @ pin_devices[0].analog[0]`, `new=TripSenseUnbound @ pin_devices[0].analog`.
  - `probes/a3-f9-analog-on-nfault.json` (`device_pin` "nFAULT"): same move.
- Why it matters: F9's ruling ("a device both refuse is named by its trip", 1211e FEEDBACK F9) covers this only literally. Here the trip refusal is caused by the malformed ADC entry, not by a second, independent fault. The cheap fix is to check each `analog[k].device_pin` (row pin, `current_output` kind) before `trip()`, or to have `trip()`'s unbound arm tell "no entry" apart from "an entry on another pin".

### A6: note. Stale or misleading messages after F2
- `probes/a1-tie-rail.json` (IPROPI tied to `VDD_3V3`) gives `SignalKindMismatch @ pin_devices[0].straps.IPROPI`: "device pin `IPROPI` is a current_output pin; only a strap pin takes a strap". `probes/a1-tie-vmotor.json`, `probes/a1-tie-resistor.json` and `probes/a1-tie-gnd-lower.json` (`"gnd"`) give the same. IPROPI now does take a strap (a tie to `GND`), so the message should say so.

### A7: note. The `NoInstance` message omits the pad exclusion
- Probe `probes/a2-search-spi2-miso.json` (sentinel-01, `spi_buses[0].peripheral` SPI2, cs hand-pinned on PB14):
  - new: `NoInstance`, "SPI2: every pin for `SPI2_MISO` is held by hand (`PB14` by `spi_devices[0].cs`)";
  - base (v25 copy): solved, with `SPI_MISO` on **PC2**. This is F4 working; the base bug is real.
- The message's "every pin" is wrong: PC2 was removed as an analog pad, not held by hand.

### A9: defect, pre-existing and not this WO. No test point is admitted on any board with a pin device
- Claim: `bench.rs` step 15a takes `others = all[..all.len() - n_test_points]` on the assumption that `names::references` lists the test points last. `names.rs:125` chains `pin_devices::references(spec)` after the test points. So the slice keeps the test points and drops pin-device references, and every test point collides with itself.
- Probes and output (validate):
  - `probes/a2-tp-TP1.json` (D1, one test point `TP1` on `DRV_NSLEEP`): `TestPointReferenceTaken` "test point `TP1` takes a reference that `TP1` already holds";
  - `probes/a2-dig-on-testpoint.json` (`TP40`): the same, identical at base;
  - `probes/a2-adc-PC2-on-testpoint.json`: the same.
- Why it matters:
  - The test-point half of attack 2 cannot be run on any pin-device board.
  - The cut also means the last N pin-device references (U2, R3, R2, C1, C2 …) are never checked against a test point's reference.
  - Since 1211a, any pin-device board with a test point is refused.

### A8: note, pre-existing and not this WO. An unneeded reservoir is never looked up
- `probes/a4-reservoir-unknown.json` (reservoir part "NOPE") and `probes/a4-reservoir-is-resistor.json` (part `CRCW0603_F_K`): `validate` is ok on both base and new.
- `emit` then refuses: `PowerNotDrawable` "`NOPE` is not in the power parts table" / "`CRCW0603_F_K` is not a capacitor".
- Why: `analog()` looks the reservoir part up only when R exceeds RAIN. That is an INV-06 gap, the same at base.

## SUSPECTED or argued from code (not reproduced)

### A5: gap, from reading the code. F8 fixed one of the two ADC reservoir ratings
- Claim: `pin_devices.rs` now rates the reservoir against VDDA's maximum. `power.rs` `sense()` (battery-sense reservoir, about line 2196) still compares `cap.rated_millivolts` against `vref_min`, which is `range(mcu.supply).0`, the minimum. It is the same physical capacitor at an ADC pin, judged two ways.
- No probe can reach it: the lowest-rated held capacitor is 10 V (`GRM32ER71A226KE20`), as RESULTS §1 says. This is a code reading.
- Why it matters: it is outside F8's written scope (1211e FEEDBACK F8 names only `pin_devices.rs`), so it belongs on the backlog. Left as is, the two rules diverge for the next capacitor row added below 3.6 V.

### A10: note, from reading the code. Stale emitter message
`crates/aethelgard-eda/src/schematic_arms.rs:1354-1359` still says "IPROPI tied to GND … and a pin device cannot declare that tie". It is now false. The embedded table cannot reach it, because `validate` refuses an unbound trip sense. A copied table with a second `current_output` that is not the trip's sense would reach it (see "second current_output" below).

## Refusal changes on inputs that already refused (attack 3)

Committed specifications: 34 files × {validate, solve} on base (v25 text) and new (v26 text), compared by kind and path. **All 68 are identical** (`adv/sweep/committed.tsv`). The files differ only in their version line (checked by diff).
- 2 refuse at validate and 5 at solve, unchanged.
- 29 solved topologies were compared pin for pin (assignments, gpio, adc_inputs). No pin moved, and none sits on PC2 or PC3, at base or now.
- So the committed sweep shows placement is stable. It exercises almost no refusal-kind change.

Crafted inputs whose kind or path changed (base v25 copy, then new):

| Probe | Base | New | Verdict |
|---|---|---|---|
| a3-mcu-743, a3-mcu-cap (capacitor row as mcu), a3-mcu-lower, a3-mcu-drv (DRV8876 row as mcu); validate and solve | PowerPartUnknown @ pin_devices[0].mcu | PowerPartUnknown @ mcu.part | F10, intent |
| a3-mcu-743-dev-bad (VM on 3V3) | SupplyOutOfRange @ …supplies.VM | PowerPartUnknown @ mcu.part | F10, head of step 24, intent |
| a3-mcu-743-dev-badpart | UnknownPart @ U2 | PowerPartUnknown @ mcu.part | F10, intent |
| a4-two-dev-mcu743, a4-two-dev-mcu743-d1bad (two devices) | PowerPartUnknown @ pin_devices[0].mcu | PowerPartUnknown @ mcu.part (once) | F10, intent |
| a3-d2-mcu743, a3-d2-mcu-cap (d2 as committed) | PartLimitUndeclared @ …input_pwm_max_hz | PowerPartUnknown @ mcu.part | F10, intent |
| **a3-d2-ok-mcu743** | **ok** (validate) / UnknownPart (solve) | PowerPartUnknown @ mcu.part | **A3** |
| a3-f9-vrefgnd-fs (VREF GND, 9000 mA) | PartLimitUndeclared @ analog[0].full_scale | TripReferenceGrounded @ straps.VREF | F9, intent |
| **a3-f9-analog-unknownpin / -on-vref / -on-nfault** | UnknownDevicePin / SignalKindMismatch @ analog[0]… | TripSenseUnbound @ .analog | **A4** |
| a3-tie-vrefgnd-analog | SignalKindMismatch @ straps.IPROPI | TripReferenceGrounded @ straps.VREF | F2, intent |
| a3-tie-plus-unknown-analog | SignalKindMismatch @ straps.IPROPI | UnknownDevicePin @ analog[0].device_pin | F2, intent |
| a1-tie-with-analog | SignalKindMismatch @ straps.IPROPI | DevicePinBoundTwice @ analog[0] | F2, intent |
| a1-tie-noanalog, a1-tie-order (validate and solve) | SignalKindMismatch @ straps.IPROPI | ok | F2, admitted, intent |
| **a1-vref-low-tie** | SignalKindMismatch @ straps.IPROPI | ok | **A1** |
| a2-hand-spi_buses-miso-PC2, a2-hand-spi2-mosi-PC3, a2-hand-cs-PC2, a2-hand-pdsig-PC2 (solve) | ok | AnalogPadPin | F4, intent |
| a2-hand-i2c_buses-sda-PC2 | NoInstance @ SENSORS | AnalogPadPin @ PC2 | F4: hand-pin stage before search (README solve row 3), intent |
| a2-hand-serial_ports-tx-PC2 | NoSerialPeripheral @ CONSOLE | AnalogPadPin @ PC2 | F4, intent |
| a2-hand-motor-PC2 | NoTimerChannel @ DRV_EN | AnalogPadPin @ PC2 | F4, intent |
| a2-adc-PC2-and-sig-PC2 | PinClaimedTwice @ PC2 | AnalogPadPin @ PC2 | F4, ordered before PinClaimedTwice as the README states, intent |

## Second `current_output` (argued from code; `parts.json` holds exactly one `current_output` pin)
With a copied table, a second `current_output` that is not `trip.sense` would go two ways:
- A `GND` tie on it fails the guard `self.row.trip.as_ref().is_some_and(|t| t.sense == *pin)` and is refused as `SignalKindMismatch`.
- Left unbound, it passes `validate`, because `check()` requires only `logic_input` and `enable_input` pins to be bound. `emit` then refuses it with A10's stale "cannot declare that tie" message.

A row whose `trip.sense` is not a `current_output` is refused by the loader (`pin_parts.rs:476`).

## Held
- **VREF on GND with the tie:** `probes/a1-tie-vrefgnd.json` gives `TripReferenceGrounded @ pin_devices[0].straps.VREF` on validate and solve.
- **Tie plus an analog input on IPROPI:** `probes/a1-tie-with-analog.json` gives `DevicePinBoundTwice @ pin_devices[0].analog[0]`.
- **Tie with no analog input:** `probes/a1-tie-noanalog.json` is ok on validate and solve.
- **Straps in a different key order:** `probes/a1-tie-order.json` is ok.
- **A duplicate `IPROPI` key in the raw JSON:** `probes/a1-dupkey.json` gives `SchemaViolation` ("duplicate assignment").
- **A `GND` tie on pins other than the sense:** nFAULT, OUT1, nSLEEP, EN/IN1, VM and VCP (`probes/a1-gndtie-*.json`), and d2's nSLEEP (`probes/a1-d2-tie-nsleep.json`), each give `SignalKindMismatch`, identical to base.
- **Key case:** `probes/a1-tie-ipropi-lower.json` gives `UnknownDevicePin`.
- **Tie combined with a wrong analog entry:**
  - `probes/a1-tie-sense-on-other-dev-pin.json` (an analog entry on nFAULT) gives `SignalKindMismatch @ analog[0]`.
  - `probes/a1-tie-plus-signal.json` (IPROPI as a GPIO signal) gives `SignalKindMismatch @ signals[2]`.
- **Two devices, one tied:** `probes/a4-two-dev-one-tied.json` is ok.
- **Drawn D1 with the tie (`emit`):** `probes/a1-emit-tie.json`, output in `adv/out/a1-emit-tie-emit/`.
  - U2 pad 6 (IPROPI) is on `GND`, and `DRV_IPROPI` and R2 are gone from the schematic, board and BOM.
  - The symbol's IPROPI pin is `passive`, so no pin-type ERC conflict is expected.
- **`generate` with the tie:** `probes/a1-gen-tie.json`, output in `adv/out/a1-gen-tie-gen/`. `diff -rq` against the untied run lists 8 changed files. Beyond the sha lines, each change removes the ADC input: GPIOF, ADC3 and Adc3Common blocks, `PF3<Analog>`, ADC3 bring-up in `main.rs`, and the Kani harness's ADC3 arms. `lib.rs` and the `.repl` differ only by sha line.
- **ADC hand pin on a pad:** `probes/a2-hand-pdadc-PC2.json` and `-PC3.json` solve ok. `generate` gives `DRV_IPROPI` on PC2, ADC3 INP0, `into_analog`.
- **Every digital hand pin on PC2 or PC3 gives `AnalogPadPin`** (`probes/a2-hand-*.json`): I2C sda/scl, SPI sck/mosi/miso (including SPI2 fixed), spi_devices cs, serial tx/rx, motor_outputs, and a pin device's output and input signals.
- **Case and spelling games on a hand pin:** `pc2`, `Pc2`, `PC2_C` and `PC02` give `UnknownPin`; a leading or trailing space gives `InvalidIdentifier` (`probes/a2-case-cs-*.json`).
- **An ADC input on PC2 sharing a net with a digital signal:** serial, motor, GPIO and lower-case nets (`probes/a2-adc-net-*.json`) give `DuplicateNetName`. A battery sense on `SPI_SCK` (`probes/a2-s2-sense-is-spi.json`) gives `DuplicateNetName`.
- **A test point:** blocked by A9 on every pin-device board, so not probed for pads. Test points place no MCU pin.
- **A connector pin on the PC2 ADC net:** `probes/a2-adc-PC2-on-connector.json` validates, solves and emits. No digital path is introduced.
- **A hand-edited topology with the true `mcu`:**
  - `generate hwdef` gives `InvalidTopology` (AnalogPadPin) for both `probes/a2-topo-assign-PC2.json` and `probes/a2-topo-gpio-PC3.json`;
  - `check pinout` gives `AnalogPadPin` for both.
  - The other topology parsers (`analog.rs:1348`, inside `#[cfg(test)] mod tests` at line 1138; `renode.rs:4424`, inside `#[cfg(test)] mod tests` at line 4372) are test-only. `extract` and `speeds` take the topology from `solve`. So the topology-file readers are `generate hwdef` (re-validates, and stops A2 by its own `mcu_row`) and `check pinout` (A2).
- **F8:** `probes/a4-f8-branch-*.json` (R2 1300 Ω) reach the rating branch.
  - The 50 V and 10 V reservoirs pass step 24 and are then refused at step 25 (`OffDomainAboveLimit`).
  - With no reservoir, the result is `AdcSourceImpedance @ …analog[0].transfer`.
  - The `>=` against `maximum_millivolts` (rounded up with `div_ceil`) matches the unit test. The edge is unreachable from input.
- **No panic or odd exit:** validate and solve were run on every probe file in `probes/` (296 runs: new binary on v26 files, base binary on `.v25` copies), and every exit was 0 or 2.

## Not run
ERC and DRC on the tied board, `verify`, `simulate` or a campaign on any probe, and `check pinout` with a real ELF (A2's end-to-end case).
