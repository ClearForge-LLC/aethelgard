# AET-WO-1312 — Bench access in the specification: test points on any declared net, BOOT0 on a connector, and a reset switch

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1312-bench-access`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P1 board capability, for Sentinel-01 (roadmap §3). Written 2026-11-17 from `AET-WO-3400`'s spike (PR #184, §7.2 E1 and E2, ruled in comment 6932857426). **Cadence: build, with one measured stop (§1.1).**

**Grounds (`3400` FEEDBACK):**
- **T4, measured by probe:** a connector pin is the only way to reach a net today. Sentinel-01's bus lines and rails have no probe point, so its bench rows (rise time against `extract`, the BOR sweep, SPI read-back) need hand-probing 0402 pads.
- **T8, measured by probe:**
  - NRST on a connector pin already works (probe `p-nrst`, every gate passes).
  - BOOT0 on a connector pin is refused (`ConnectorNetUnknown`, probe `t8-boot0-on-swd`). BOOT0 is not one of the emitter's own nets, as NRST is (`names::own_nets`).
  - There is no reset switch.
- **Scotty, 2026-11-17:** Sentinel-01 is the lean probe (V0) plus the cheap test systems that help Sentinel-02 ("V0+", `AET-WO-3401`). This work order is the engine half of it.

> **What this is:** two small, generic specification capabilities, through the specification and never as a board's branch (roadmap §6):
> - **`test_points`:** a pad on any declared net, domain or `GND`, drawn, placed, routed and judged by parity like any pin;
> - **BOOT0 joins the emitter's own nets**, so a connector pin may carry it, and an optional reset push-button joins the MCU's support parts.
>
> It is NOT:
> - jumpers or alternate states (`3400` E3, its own spike);
> - current sense (E4);
> - capture roles (E5);
> - any change to an existing board's emitted bytes.

## 1. Build

1. **Measure first (the stop).**
   - Run every example, fixture and scenario through the new code, with none of the new fields set.
   - **STOP** if any emitted byte changes.
   - Measure the router on Sentinel-01 with a test point on every net it declares (both I2C lines, the four SPI lines, flash CS, console TX/RX, `VDD_3V3`, `GND`, NRST, BOOT0). Report the outline, track, vias and rip-ups against today's board, at two and at four layers.
2. **`test_points`** (specification 13):
   - `test_points: [{ "net": <declared net, domain or GND>, "reference": <TPn> }]`. Empty or absent is today's board.
   - Refuse, by name:
     - a net no part of the specification declares;
     - a duplicate reference, or one that collides with any emitted reference;
     - the same net listed twice.
   - Footprint: `TestPoint:TestPoint_Pad_D1.0mm`; symbol: `Connector:TestPoint`. Both are from the vendored KiCad 10.0.7 libraries, with their licence (`RUL-11`).
   - The placer puts test points where the router can reach them, and parity judges them like any pad.
   - `extract` counts a test pad's capacitance on its net. Declare its bound with its source, or refuse to bound a net that carries one; say which, and why.
3. **BOOT0 and reset:**
   - `BOOT0` joins `names::own_nets`, so a connector pin may carry it. The strap the emitter draws today stays: the pin pulls BOOT0 to its default through the strap, and the board boots from flash with nothing attached.
   - `mcu.reset_switch: true` adds a push-button from NRST to `GND` (`Switch:SW_Push`, a footprint from the vendored library whose maker part is in production, cited). Default: false.
   - Judge the strap's value against the boot pin's datasheet input threshold (DS12117, cited). The connector must be able to drive BOOT0 high through it.
4. **Versions:** the specification takes 13, with a refusal by name for 12-only readers as the earlier bumps did. No other version moves.

## 2. Acceptance

- **Every new refusal has a tier-1 test.**
- **A tier-2 test:** Sentinel-01 with a test point on every net from §1.1, BOOT0 and NRST on its SWD connector, and a reset switch. It passes `validate`, `generate`, `emit`, `check erc` and `check drc --spec` with no finding, parity included, at two and at four layers.
- **Every existing emitted file is byte-identical.**
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`, `INV-07`; `RUL-03`, `RUL-11`, `RUL-14`, `RUL-16`.
- **Fence:**
  - `aethelgard-ir`: the specification type, validation and `names.rs`;
  - `aethelgard-eda`: emit, place, route, parity and extract, for test pads only;
  - `aethelgard-kicad-lib`: vendoring the two footprints and symbols, with their sha256 pins;
  - tests;
  - FEEDBACK.
- **Running beside you:**
  - `AET-WO-2302` is in the CLI and chaos.
  - The release work order (`AET-WO-1801`) bumps every version at its own merge. Merge `main` as it lands.
- One PR, left unmerged. Write `~/.cache/aet-1312/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A test point that makes the board pass a gate it should fail:** a pad that bridges two nets, or one that hides a net from parity.
- **BOOT0 floating or tied the wrong way** with the connector unplugged.
- **A test point on a net whose name is spelled like an emitter net**, reaching `emit` by the library path (`1208` F3).

## 7. Flag-and-stop conditions

- **§1.1 moves an existing byte.**
- **The test-point board does not route at two layers.**
- **BOOT0's strap cannot be judged at source.**
