# AET-WO-1211d — results

Predictions: `PREDICTIONS.md` (commit `661ff706`, before any run). Base: `main` `25203c58`.

## N · The null sweep (§1.1)

`measure/sweep.sh` runs `validate`, `solve` and `emit` on every committed specification (31) with one binary. `measure/same.sh` compares two runs byte for byte: every emitted file and exit code raw, and each command's printed JSON with its own output directory's path replaced (`emit` prints where it wrote).

| point | binary | result |
|---|---|---|
| base | `main` `25203c58` | 31 specifications; `emit` exit 0 on 24, 2 on 7 |
| after vendoring (`76e8cafe`) | branch | **identical: 259 files** |
| after the arm (`89b08ec9`) | branch | **identical: 259 files** |
| D4's table and interpreter (spike only, `d4/`) | experiment | **identical: 259 files** |
| READY, `main` `adb8f61a` merged (`fbaaf3b7`), against a base built at `adb8f61a` | branch | **identical: 259 files** |

N1, N2 and N3 hold: no committed specification reaches either arm, and `validate`'s output, which `own_nets` feeds, is unchanged on all 31.

## A · The arms

**D2 drawn** is the committed D2 with `board.drawn` absent and each bridge input on a GPIO output; ruled 18:45Z, built in `pin_devices_drawn_tier1.rs`. D2 as committed is refused `PartLimitUndeclared` inside `solve`, so it cannot be drawn at any level. It is drawn at library level (`draw/`) and judged in the tools image (`kit/img.sh` + `kit/judge.sh`; capped-run 8 GiB, swap 0 read back, the host-built CLI mounted):

| judge | before Q3 (KiCad's `Driver_Motor:DRV8833PWP`) | after Q3 (`Aethelgard_Driver:DRV8833PWP`) |
|---|---|---|
| `validate` | ok | ok |
| `check erc` | fail: 2 `pin_to_pin` warnings (AISEN and BISEN, `bidirectional`, on GND's flag) | **pass**, findings `[]` (re-run at `fbaaf3b7`: all four pass) |
| `check drc --spec --schematic` (parity) | pass | **pass**, findings `[]` |
| `check support` | pass | **pass** |
| `extract` | pass | **pass** |

**D1 drawn**, at library level, is refused `PassiveValueUnstated {U2, DRV8876PWPR, CVM2}` and nothing else (ruled S1; test `d1_is_refused_for_cvm2_alone`). The arm draws CVM2 last, so every other part of D1 is drawn first. **D1-stub** is drawn by `kit/d1-stub.sh` in a scratch copy only, with CVM2's `None` replaced by the literal `UNSTATED-SPIKE-ONLY`, so that the authored DRV8876 symbol and PWP0016A footprint meet the gates:
- the first stub draw was **`Unroutable` (`U2_CPH` blocked by `U2_CPL`)**: CFLY was anchored to both CPH and CPL, adjacent pins 0.65 mm apart. Each required capacitor is now anchored by its first pin end (commit `1a7bcf29`);
- then `check erc` **pass**, `check drc --spec --schematic` **pass**, `check support` **pass**; `extract` refuses `PassiveValueUnstated`, because it re-runs the real emitter (S1 holds on that path too);
- the board's IPROPI chain: MCU pin 13 (PF3, ADC3) on `DRV_IPROPI` with R2 and C1; R3 on `DRV_NFAULT`; nSLEEP on MCU pin 35.

## M · Mutants (`mutants/mutants.py`, `mutants/result.json`)

One per guard this work order adds, each alone in a scratch copy of HEAD, against the named test target. **10 of 11 killed.**

| mutant | killed by |
|---|---|
| W1 a strap left floating (a tie not drawn) | `d2_is_drawn_pin_by_pin…`, `a_strap_missing_from_the_declaration…` and 4 more |
| W2 nFAULT's pull-up missing | `a_declared_nfault_pull_up_is_drawn_between_its_net_and_its_rail` |
| W3 a bridge output to no connector | `a_bridge_output_on_no_connector_is_refused` |
| G1 CVM2 skipped, not refused | `d1_is_refused_for_cvm2_alone` |
| G2 an undeclared current output left open | `an_undeclared_nfault_is_open_and_an_undeclared_ipropi_is_refused` |
| G3 a part with no arm drawn anyway | `the_schematic_emitter_refuses_a_pin_device_by_reference_and_part` |
| G4 `own_nets` without the devices' nets | `the_pin_devices_own_nets_are_listed` and 6 more |
| G5 VINT on a label, not a rail | `d2_is_drawn_pin_by_pin…` (the sheet's power symbol) |
| G7 a row pin left floating (the completeness refusal removed) | `a_strap_missing_from_the_declaration…` (the first run's G7 still returned the refusal: a harness fault, fixed and re-run) |
| S1 the DRV8833 symbol's AISEN back to `bidirectional`, hash re-pinned (by hand) | `the_drv8833_symbol_s_pins_are_its_pin_functions_table` |
| **G6 the pin device's ADC input not on the MCU** | **survives.** Only a DRV8876 has an analog output, and D1 is refused at CVM2 before any test can read its MCU pins. Held today only by the D1-stub board above. It becomes testable when D1 is drawable (S1's IR follow-up) |
