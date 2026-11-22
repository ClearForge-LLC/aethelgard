# AET-WO-1218 — predictions, committed before any measurement

Base: main 2ccd2450 (specification 26, solved 17). Baseline binary = `cargo build --release -p aethelgard-cli` at that head.

## P1 — the baseline (34 committed specifications, by `spike/1211d/measure/sweep.sh`)
- P1.1 No committed specification has both `pin_devices` and `test_points` (counted from `git ls-files`: 2 with pin devices, 4 with test points, 0 with both).
- P1.2 Running the sweep twice with the base binary is byte-identical (`same.sh`, 0 moved).

## P2 — the scratch boards, today (base binary)
Scratch boards are D2 (`crates/aethelgard-ir/tests/fixtures/d2-drv8833.json`) and D1 (`d1-drv8876.json`) plus one test point.
- P2.1 D2 + `TP1` on `GND`: `validate` refuses `TestPointReferenceTaken` naming `TP1` against `` `TP1` `` (it collides with itself). `solve` and `emit` refuse the same.
- P2.2 The same for D1 + `TP1`, and for a test point on a pin device's own net (`DRV_NFAULT`), and on `BOOT0`.
- P2.3 D2 + a test point spelled like a pin-device reference (`U2`), or like D1's passive (exact and by KiCad key), is refused `TestPointReferenceTaken` today. That is the right refusal, so it must not move.

## P3 — the scratch boards, after (1)
The fix is in bench step 15a only: "every other reference" is built by construction (the same chain without the test points), not by position and not by a name filter. `names::references`' order and eda's copy of it do not change.
- P3.1 D2 + `TP1` (`GND`), D1 + `TP1`, D2 + a test point on `DRV_NFAULT`, and D2 + a test point on `BOOT0` each validate, solve and emit (exit 0).
- P3.2 Emitting each twice is byte-identical.
- P3.3 Each scratch board's emitted references are the bare D-board's references plus the test point's. On `BOOT0` there is also the bench's BOOT0 pull-down, numbered after every other part, so no other reference moves.
- P3.4 P2.3's collisions are still refused `TestPointReferenceTaken` by step 15a. The kind does not move to step 20's `EmitterReference`.
- P3.5 The null sweep: the after binary against the base over 34 specifications, before the bump, has 0 moved.

## P4 — (2), (3), (4)
- P4.1 (2): a solved topology with `mcu` renamed (to `STM32H743ZITx`, a part with no row, and to `stm32h753zitx`, a case variant) passes `check pinout`'s topology validation today, and refuses at the ELF or later. After, `check pinout` refuses it before the ELF is read, exit 2, with a new kind.
- P4.2 (3): the power tree's sense reservoir is judged against VDDA's minimum today (`power.rs` `sense()`). A capacitor rated between VDDA's minimum and maximum validates today and is refused after. Whether a held capacitor row reaches that band is open; if none does, a seam test is built and said so.
- P4.3 R4: `NoInstance`'s reason for a bus whose only pins are analog pads names the pad as withheld today in no wording. After, it names it.
- P4.4 R5: a pin device's reservoir part `NOPE`, or a resistor row, validates today when its source is under RAIN, and `emit` refuses it. After, `validate` refuses it. `power.rs` `sense()` has the same `continue`; whether the power tree's own part lookup already catches it is open (probe).

## P5 — the bump
(1) admits content that specification 26 refused, so a bump is due (`1215` D4): specification 27. After the bump, the null sweep moves only the version lines and the sha lines that follow from them. `fab/` sha lines move, so a dispatch is queued.
