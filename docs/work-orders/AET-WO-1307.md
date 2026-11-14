# AET-WO-1307 — Escape planning: a router whose success depends on the board, not on names

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1307-escape-planning`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**

**Grounds, each read on `main` after `AET-WO-4100` (PR #120):**
- **`D3` fails closed at 4 layers.** `crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json` (24 I2C devices and 12 SPI flashes on OSH Park's 4-layer stack) gives `Unroutable` on `VDD_3V3`, blocked by `/CS0_0`, `/S1000_HOLD` and `GND`. The MCU's adjacent supply and ground pads each need their own escape to a plane via, and the router plans none. `D1` (24 I2C devices) routes at 4 layers and fails at 2 (`4100`).
- **The router's order is names.** `route::route` lays the stitched nets first, then sorts the rest by pad count and then by **net name**, and rips up by blocker (`RIPUPS` = 500). Two backlog rows follow from that:
  - **1102 A6:** a net's name alone can make `generate` fail with `Unroutable`.
  - **2204 A5:** the reference names `Peripherals` and `Sysbus_` give `Unroutable`.
- **4100 F7.** `route::read` does not judge a zone's outline, so a 1 mm² zone passes the router's connectivity. KiCad judges the fill, so DRC catches it, but the reader should not need DRC to.

> **What this is:** escape planning for dense pin fields, and a net order that does not depend on names, so that whether a board routes is a fact about the board.
>
> It is NOT:
> - a new router;
> - blind, buried or micro vias, or via-in-pad;
> - 6 layers (backlog);
> - length matching, differential pairs, or impedance (`4300e1`).

## 1. Spike

Probes live under `spike/1307/`. **Predictions go into FEEDBACK before any run.** Everything here is host Rust and KiCad in the pinned image.

**S1. Why `D3` fails.** Instrument the router on `D3` at 4 layers and record:
- which pads' escapes are contested;
- which plane vias have no free site within reach of their pad;
- what the rip-up loop is doing when it gives up.

Name the geometric cause in one sentence, with a figure (a PNG rendered from the board, or the KiCad PDF plot).

**S2. Candidate escape strategies.** For each candidate, measure it on `D3`, `D1`, the reference, the 2 × 5 case and the 4-layer fixture. Record routed or not, time, vias and track length, and whether every result passes the gate's DRC.

Candidates:
- **(a) Escape first.** Before any net is tracked, plan one escape per pad of the dense part (the LQFP): a stub and a via site, outward, alternating rows, with plane nets' vias claimed first.
- **(b) Plane-pad priority.** Each plane-net pad claims its nearest via site before signal escapes. That is today's stitching order, made geometric rather than first-come.
- **(c) Any third strategy the S1 cause suggests.**

**S3. Name independence.** Show the dependence, then propose its removal:
- **Show it.** Rename every net and reference in the reference and in `D1` by a fixed permutation (for example, a hash order), and route each. Count how many permutations of 20 route.
- **Propose an order** that is a function of the board's geometry and connectivity alone, with names as the last tiebreak only where two nets are geometrically identical. Measure the same 20 permutations under it.
- **Guard it.** Propose a test that holds the guarantee.

**S4. Zone outlines (4100 F7).** Propose how `route::read` judges a zone's outline: equal to the board outline, minus the edge clearance, as the emitter writes it. Say whether anything legitimate is refused.

**Stop** with the table, a recommendation per question, and the decisions needed.

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **The ruled escape strategy.** `D3` routes at 4 layers and passes every gate. If it still cannot route, it fails closed naming a geometric cause; that is a ruling, not a pass.
2. **The ruled net order.** The reference, `D1` and `D3` route under every one of 20 fixed renamings, and a test holds it. Close 1102 A6 and 2204 A5.
3. **Zone outlines judged by the reader** (4100 F7).
4. **Sentinel-01 byte-identical,** or every changed committed file named with its cause. A change in the reference's routing is allowed only if the ruling accepts it, because the routed board feeds extraction's bound and the fabrication package.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-12`.
- **Fence:**
  - `aethelgard-eda` (`route.rs`, `pcb.rs` where the escape is written);
  - the CLI's `generate` and `check` tests and fixtures;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **Other builders on Furnace:** `4200` (`~/aethelgard-4200`) and `2205` (`~/aethelgard-2205`). Neither touches `route.rs`. Never touch their worktrees. Merge `origin/main` with a merge commit whenever one lands.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The spike table, ruled.**
- **`D3` routes at 4 layers and passes every gate,** or fails closed by ruling.
- **20 renamings each route** the reference, `D1` and `D3`.
- **Sentinel-01's committed files** are unchanged, or each change is named.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 5. Adversarial pass, full

1. **Make the router depend on names again:** a net or reference chosen to sort first, or to collide.
2. **Make an escape lie:** a stub that leaves a pad but joins nothing, or a via site under another part's courtyard. The gate must refuse each.
3. **Make it slow:** a board built to defeat the escape plan, for example a pin field at minimum pitch. It must fail closed within the rip-up bound, never hang.
4. **A zone that is not the board's outline:** smaller, larger, holed or offset. Each must be refused by name.

## 7. Flag-and-stop conditions

- No candidate routes `D3` without a change to the rules or the stackup.
- The ruled order changes the reference's routed board, and the change moves extraction's bound past a declared value.
