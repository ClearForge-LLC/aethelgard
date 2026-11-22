# AET-WO-1211c §1.1 — the measurement, and the stop on the motor rail

Run 2026-11-22 on `main` `7fc81640` (main's host CLI, saved aside as `~/.cache/aet-1211c/main-7fc81640-aethelgard` before any crate edit), in the tools image `e5fc7ddb…` under `capped-run` (8G, or 16G for `today.sh`; swap 0, read back), `--network none`, `nice -n 10`. Predictions: `PREDICTIONS.md`, committed `4254de68` before any run. Outputs: `out/`.

## Kit changes after the predictions commit, named

- `compare.py` rewritten: 1211's log has a Renode log record written inside a value line (T3), so records are cut from the byte stream, not per line.
- `fixtures.py`: the VMOTOR droop needed `bor_level` (the parser refused it, `SchemaViolation`); the out-of-band scenario needed a finding (`ExpectContradiction`). Both were my kit's errors, rerun.
- `coverage-d1.sh`: the first edit did not compile (type inference); now `draws_a_board(..).err().filter(|_| false)`.
- **M4's platform:** on the stock platform alone (`d1-stock-plain.resc`, the predictions' form) D1's firmware parks at `BOOT_SUPPLY_NOT_DECLARED` `0x2100B0F1` before any GPIO write (`out/d1-stock-plain.log`): the stock PWR stub never shows the voltage scale ready, which the twin's own PWR model (`platform::pwr_override`, 4240v) does. **This is my kit, not a finding.** M4 was rerun on the twin's emitted platform for 4600's motor driver (`/cache/m/djn.repl`: the same MCU, HSI 64 MHz, TIM1 on PA8/PA9, TIM2 on PA0/PA1, USART1, the PWR model), since D1 has no `.repl` until this work order.
- **M4g, added after M4 and not predicted:** M4 with PA11 held **high** (`d1-held-high.resc`). Reason below (M4, nFAULT).
- **Isolation, added after M3 (the advisor's point):** a detected `serial_rx_open` on `CMD` and `sentinel-03/droop-bor3.json`'s BOR-crossing droop beside the baseline, so VMOTOR is shown alone.
- **P3d, measured:** `rails-sweep.sh`, every committed specification's coverage before any gate.

## Predictions beside results

| # | prediction | result |
|---|---|---|
| P1a | G0–G6, T1–T6, O1–O2, H1–H2, A0–A6 equal 1211's log | **holds:** 112 of 112 value lines identical (`compare.py`). The log records differ only by two `Including script(s)` lines 1211's log has twice |
| P1b | Renode `1.17.0+20260907gitf1dd1b4af.aet1605` | **holds** |
| P2a | D1: validate, solve, generate, build 0; no `.repl`; simulate `PinDeviceNotSimulated` (U2, DRV8876PWPR), exit 2 | **holds.** ELF `05720792…0545`, as 1211b's |
| P2b | D2: `PartLimitUndeclared` U2 at every step | **holds** (seen before the predictions; see their last section) |
| P2c | D2g: validate … build 0; simulate `PinDeviceNotSimulated` (DRV8833PWPR) | **holds.** ELF `3cef8f18…a34e` |
| P2d | verify on D1 refuses before any gate, not drawn | **holds:** `DrawsNoBoard` |
| P3a | D2g `--fab-out`, baseline: `CampaignCoverageShort` on `CMD`, `VDD_3V3`, `VMOTOR` (no rendered consumer) | **holds:** "rail `VMOTOR` (`VMOTOR` has no consumer a judge observes)" |
| P3b | D1 the same | **holds** (with `draws_a_board` skipped) |
| P3c | a VMOTOR droop and a VMOTOR rail out of band refused by name | **holds:** `DroopNotMcuSupply`; `RailOutOfBandRefused` ("not rendered by its consumers' absence alone, or by none") |
| P3d | no committed specification but D1/D2 has a sourceless second domain | **half.** Five committed specifications carry a rail with the same reason: `connectors.json` (`VDD_5V`, connectors only), `sentinel-02-power.json` (`VDD_5V`: a regulator's output and a load), `hse-crystal-16.json`, `hse-external-8.json`, `sentinel-02-shaped.json` (`VDD_1V8`, nothing), `sentinel-02-own.json` (`VDD_5V`, `VDD_1V8`, nothing). **None feeds a pin device's supply pin.** All 28 that reach coverage are already short on other targets (`out/rails.tsv`) |
| P4a | boot word `BOOT_MAGIC` | **holds** on the twin's platform (`0x2100B007`); on the stock platform alone, `0x2100B0F1` (above) |
| P4b | PA1 `BSRR` `0x00020000` before the `MODER` write that makes it an output | **holds:** `BitSet 0x20000`, then `Mode 0x2A0006` (`out/d1-stock.log` L91–93) |
| P4c | PA11 `MODER` 00, `PUPDR` 00 | **holds:** `MODER` `0x002A0006`, `PUPDR` `0x640A0002` |
| P4d | PA8, PA9 read 0 at every instant; zero edges | **holds:** 8 samples 0; the observers log no edge |
| P4e | ADC3 `DR` is **not** 1861 | **missed:** `DR` = `0x745` = **1861**, the 12-bit code at 1.5 V. `CFGR` `0x8` (`RES` 010) converts at 12 bits in Renode's model. Only `RES` 010 is measured |
| P4f | nFAULT held low changes nothing | **holds, and shows nothing:** see below |

## What M4 and M4g show for the twin

- **A held-low input is indistinguishable from an undriven one.** With PA11 held low, `IDR` reads `0x0` throughout; undriven, it would also read 0, because the model applies no pull (G2). M4g held PA11 high: `IDR` bit 11 reads 1 at t = 0, at every sample and at the end, so **a hold survives the firmware's port configuration** (the `MODER`, `PUPDR` and `BSRR` writes leave it). `ODR` reads `0x800` too (G4: one `State` array).
- **So the twin must drive every declared input explicitly**, at the level its declared pull holds when the device does not assert it (D1's nFAULT: R3 to `VDD_3V3`, high; D2's: the MCU's pull-up, high). Otherwise every active-low input reads asserted in every baseline. That level is the twin's assumption about an unmodelled device ("it reports no fault"), so it is named as unwitnessed (G2), not judged.
- **The firmware never reads an input** (no generated step reads PA11). A per-signal gate on a held input therefore reads back the twin's own stimulus through the MCU's `IDR`. It shows that the scenario reached the pin, never that the firmware reacted.
- **ADC3:** the bring-up completes (`ADCAL` is an unhandled tag that reads 0 at once; `ADRDY` at once, A1); `JQDIS` and `SMP5` are unhandled tags (`SMPR` ignored by the model, as 1211 §2.7 (c) classed it); `PCSEL` `0x20`, `SQR1` `0x140`; the one conversion reads `DR` `0x745`.

## The stop: the motor rail (§1.1, §7)

**The measurement.** `VMOTOR` is unmet on D1 and D2g, and with a detected open RX line and a BOR-crossing droop beside the baseline it is **the only unmet target** on both (`out/d1_verify-fab-iso.json`, `out/d2g_verify-fab-iso.json`). So `--fab-out` is blocked on every board with a motor rail by that rail alone. No fault kind reaches it: a droop is refused off the MCU's supply (`DroopNotMcuSupply`), and a rail out of band needs a consumer the campaign renders (`RailOutOfBandRefused`). Its consumers are `U2`'s `VM` pin and a connector pin; the twin models neither.

**The two options, costed from the measurement.**
- **(i) A droop on a sourceless domain.** Nothing on `VMOTOR` is modelled: D1's nFAULT with `IMODE` tied to GND reports overcurrent only (SLVSDS7B Table 6 p. 13), and SLVSAR1E contradicts itself on UVLO (1211 §7). A tolerated droop would cover the rail while the twin observes nothing change. **That is 2306's defect, a gate that judged nothing, counted as coverage.** It could be made to mean something only by rendering the part's response (nFAULT asserted below UVLO), which is a device model this work order excludes.
- **(ii) The domain's exclusion (lean).** A domain is not a `Rail` target when at least one pin device's `supplies` names it and nothing else consumes it but connector pins. It is never `met`, never blocks, and is named in the verdict with its reason, on the precedent of a bus no device uses (`Coverage.no_device`, ruled (c) on 3303): "rail `VMOTOR`: it supplies only pin devices' supply pins (`U2.VM`), and the twin models no pin device (`device_unmodelled`): no fault on it reaches anything the twin observes; judged on the bench".
  - **The predicate is computed from the specification, never declared,** so no board can declare its way out. It catches `VMOTOR` and **none of the five committed rails above** (none feeds a pin device), so no existing blocking rail turns non-blocking.
  - **It adds no key** on a board without such a domain: the precondition's existing `no_device` list carries it (`campaign_faults_every_bus_and_rail`, `verify.rs:203–206`). No precondition name or order moves (`board_tier2.rs:957–965`).
  - It lifts when a device model or 4600 F7's power-input kind exists.

**Lean: (ii).**

## Decisions asked with the stop

- **D1 · The motor rail:** (i) or (ii), above. Lean (ii).
- **D2 · `Target::Bridge`.** 1211 §2.7 (e) writes it "`unmet` (no fault kind reaches it) until an output fault exists" (4600 F3). As written, every board with a bridge is blocked at `--fab-out` whatever D1 rules. Lean: the same treatment as D1's answer (listed with its reason, not blocking), since no fault kind can reach it. If it should block, say so.
- **D3 · Which D2.** D2 as committed is refused `PartLimitUndeclared` at validate, and its fixture is outside my fence. Lean: **D2g**, 1211d's variant (each bridge input a GPIO output, board drawn), built in the tests from the committed fixture so no committed byte moves.
- **D4 · No board reaches `verify` end to end with an ADC input.** D1 is `board.drawn: false` (`DrawsNoBoard`), and drawn it is refused at emit for CVM2 until 1211e. D2g has no ADC. Lean: judge the ADC under `simulate` and `campaign` on D1 (both take an undrawn board), and the GPIO, bridge and coverage under `verify` on D2g. `verify`'s ADC gate and `Target::Analog` are then tested at library level, not end to end.
- **D5 · Detection of a held input needs an attribution rule.** `verify` counts a detected fault only when `attribute` names the electrical half by a rule (`detection()`, `verify.rs:1171`). The rules (E1–E3) live in `aethelgard-evolve` (`Rule`, `decide`), outside this work order's fence; a held nFAULT's run would be attributed indeterminate, so `Target::Gpio` could never be met and its campaign run would fail. Lean: **widen the fence to one electrical rule `E4`** in `aethelgard-evolve` (a declared input at its active level while the scenario holds it there; evidence the new GPIO witness record; subject `device:<reference>`), with its tests. The alternative, counting the detection without attribution, breaks the pattern `INV-05` rests on.
- **D6 · Proceed while D1–D5 are open?** Lean: yes, on what none of them decides: the twin's ADC and GPIO witnesses, the static-level motor witness, the two fault kinds and the refusal, and the version bumps (their own commit). `verify.rs` last, after `main` (2307 edits it).

## Also named

- **The baseline input level** (above) is a twin assumption, named unwitnessed. A declared input with no pull and no resistor is refused at validate (`OpenDrainUnpulled` for open drain); a push-pull device output with no pull has no level the specification states, and the twin refuses it by name rather than pick one.
- **An analog input's baseline value.** The twin must set each declared channel (A2). For a `current_output` transfer on a device whose bridges idle non-driving, the load current is 0, so the value is 0 mV, set explicitly. For any other transfer, no value is stated, and the twin refuses by name.
- **Only `RES` 010 (12 bits) is measured.** The twin's ADC witness is claimed for 12 bits only; another resolution is refused by name until measured.
