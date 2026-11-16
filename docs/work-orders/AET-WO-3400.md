# AET-WO-3400 — Sentinel-01 as a bench board: every test feature, the flight controller's subsystems, and a job after bring-up, scoped on two layers

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3400-bench-board`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass on the measurements.
**Phase:** the first fabricated board (roadmap §3, Sentinel-01 first). **Cadence: spike, then STOP for the architect's and Scotty's ruling.**
- The spike writes nothing outside `spike/3400/` and its FEEDBACK.
- The builds it scopes become their own work orders.

**Grounds:**
- **Scotty, 2026-11-16:**
  - Sentinel-01 is a full instrumented test board, carrying the flight controller's subsystems, on two copper layers.
  - "Add all the test features. That's the biggest payoff for aethelgard as a whole by buying and benching Sentinel-01."
  - Sentinel-01 need not be a flight controller. It should have a use after bring-up that makes it worth its cost.
- **Roadmap §3:**
  - "Custom boards are bought only when a gate needs one, and the first is meant to be used" (Scotty, 2026-11-09).
  - Sentinel-01 is reversed to first (2026-11-16) because it is cheap. It falsifies the extractor, the router, the support gate, programming and SPI bring-up on our own copper before a four-layer flight controller does.
- **Roadmap §6: capability enters through the specification, never as a board's branch.**
  - A test feature that the specification cannot express is an engine change with its own work order. It is never a hand edit to Sentinel-01's files.
- **Sentinel-01 on `main` today** (`examples/sentinel-01-minimal.json`, specification 12):
  - two layers;
  - BME280 and TMP117 on one I2C bus with 2.2 kΩ pull-ups;
  - a W25Q64JV flash on one SPI bus;
  - a USART console;
  - no `power`, no `motor_outputs`, no USB.
- **The first scar** (northstar §2): SPI1 `MODF` and the H7's I2C clock-synchronisation threshold. Both were found on the Nucleo, not on our copper.
- **Held until Sentinel-01's scar:** `4400b`, `4400c`, `4500`, `4220b` (roadmap §5). `4220b` is held because its only consumer was Sentinel-02. If Sentinel-01 carries the ICM-42688-P and BMP581, it has a second consumer, and the spike says so.

> **What this is:** a measured list of what Sentinel-01 should carry. For each candidate it gives what it would tell us, what the specification and the engine must gain to express it, what it costs, and whether the board still routes on two layers with it. The architect and Scotty then pick, and the picks become build work orders.
>
> It is NOT:
> - a change to any crate, to the specification's version, or to `examples/sentinel-01-minimal.json`;
> - firmware for the board's later job;
> - an order.

## 0. Amendment (Scotty, 2026-11-16 22:30 UTC, while the spike runs)

**The board's job is J1: the witness board.**
- Sentinel-01 is an all-purpose instrumented board, built to cut future bench time. It measures other boards and itself.
- It can behave as whatever a reading needs: a sensor, a bus controller, a radio receiver, or an ESC.
- It still carries the Sentinel-02 checks wanted before Sentinel-02's design is committed.
- J2 stays a cheap candidate, and J3 a note.

**New S1 rows, the witness's hardware.** Measure each like the others.

| # | Candidate | What to settle |
|---|---|---|
| W1 | A probe header | Channel count, input protection, level translation for 1.8, 3.3 and 5 V targets, a ground per channel or per pair |
| W2 | Logic-analyzer capture: timer-paced DMA from a GPIO port's input register | The sample rate for 8 and 16 channels (measured on the Nucleo if you can, else cited and labelled), and the buffer depth in SRAM |
| W3 | The host link | USB full speed (on chip, 12 Mbit/s), USB high speed through an external ULPI PHY, or Ethernet RMII (on-chip MAC, external PHY, 100 Mbit/s). Each with its pins, parts, layer cost and streaming rate |
| W4 | Protocol capture by peripheral | SPI and USART receive-only, timer input capture for PWM and DShot. Also why I2C is sniffed through the analyzer path: the I2C peripheral cannot listen without taking part |
| W5 | Stimulus | Driving a bus as its controller or as a device: an I2C target, an SPI peripheral, a CRSF source, a DShot reader |
| W6 | Host software | Speak a logic-analyzer protocol that sigrok's PulseView already reads (SUMP/OLS, for example), so its protocol decoders work from the first day. Say which one, cited |

**S2 measures four layers too.** Run the same variants at `copper_layers: 4` (S-P-P-S, as `4100` built). Scotty ruled two layers for the original probe. Whether the loaded board goes to four is now his decision, made with numbers.

**S4, the cost (new).** Price three configurations:
- **(a) the lean probe:** today's Sentinel-01 as `main` emits it, at two layers;
- **(b) the loaded board at two layers:** the largest variant that passes;
- **(c) the loaded board at four layers.**

For each one, record:
- the board outline and area;
- fabrication for 5 and for 10 boards at two named fabricators, from their published price pages, dated;
- assembly: setup, per-part-type fees, and through-hole work;
- the BOM per board, each line with its source and date;
- the engine work orders needed before it can be ordered, each with a size;
- a calendar estimate to the order.

**Sourcing rules:**
- A price page Furnace cannot reach goes on a list for Scotty, who fetches it.
- Never estimate a price without a source.
- Label every figure measured, read at source, or inferred.

**S3's decision table adds a recommendation** among three paths, with the reason in what each teaches before Sentinel-02 and when:
- (a) now, then the loaded board as a separate **Sentinel-03**;
- (b);
- (c).

## 1. Spike

Probes under `spike/3400/`. **Predictions go into FEEDBACK before any run.**

### S1. The candidates

Measure every row below. Add any you find that has a clear value. For each row, record:
- **what it tells us:** which twin prediction, rule or gate it tests on silicon, cited (an `RUL-`, a §2.1 row or a work order);
- **whether the specification can express it today**, and if not, the smallest generic addition (§3's rule);
- **its parts:** each as a `parts.json` or `power_parts.json` row that exists or would be needed; the KiCad 10.0.7 symbol and footprint and their licence (`RUL-11`); and whether it is in production;
- **its cost:** BOM, with its source, or "unknown". Also its board area and its pins after the solve;
- **its class:** *must* (information Sentinel-02 needs before it is ordered), *extra* (clear value, not needed), or *defer*.

**Test features (Scotty: all of them):**

| # | Candidate | Why it is a candidate |
|---|---|---|
| T1 | Selectable I2C pull-ups, from 1 kΩ to 10 kΩ, by solder jumper or DNP position | Rise time against `RUL-19`'s prediction on our copper; `propose`'s pull-up change measured, not computed |
| T2 | A bus-capacitance coupon: known capacitance switched onto I2C | The extractor's bound and `RiseTimeTooSlow` measured against a known load |
| T3 | A current-sense shunt per rail, with Kelvin test points, or a sense amplifier on the bus | The power tree's static loads (`4300d`, `4300d3`) against measured current |
| T4 | Test points on every bus line and rail, with ground points for a probe | Every bench row can be scoped |
| T5 | Fault-injection jumpers: open a bus line, pull it weakly to ground, or remove a device | The twin's fault classes (`2301`, chaos) against the same fault on silicon |
| T6 | An adjustable MCU rail: a jumper that feeds VDD from a bench supply and bypasses the regulator | Brown-out and BOR thresholds measured (`2301`'s droop model, `bor_level`, 1208's MCU range) |
| T7 | Loopbacks: a USART TX to RX; a motor output to a timer input capture; the MCO clock to a test point | The board witnesses its own timing on silicon, the bench row the twin's virtual-time witness (`4300a`, `4300b`) is compared against |
| T8 | Programming by every path: SWD header, USART boot, USB DFU; a BOOT0 jumper and a reset button | `3301`'s programming on our copper, and the path a user takes |

**The flight controller's subsystems (Scotty: "the dev flight controller needs"):**

| # | Candidate | From |
|---|---|---|
| F1 | The power tree: buck from a battery, LDO, the hot-plug damping | `4300d`, `4300d3`; the power fixture |
| F2 | The flight IMU and barometer (ICM-42688-P, BMP581) on SPI | `4220`; their board path is `4220b` (held) |
| F3 | DShot or PWM motor outputs to a header | `4300a` |
| F4 | A serial RC input (CRSF) | `4300b` |
| F5 | USB | `4300e1`, if it is on `main`; otherwise say what is missing |
| F6 | The SPI flash, kept | already on Sentinel-01 |

**A job after bring-up (Scotty: worth its cost):**

| # | Candidate | What it adds |
|---|---|---|
| J1 | **The witness fixture.** Sentinel-01 drives and measures the next board's buses: it generates CRSF frames, reads DShot by input capture, and stands in for a sensor | The bench rows for Sentinel-02 and every later board, run by a board instead of by hand. Its firmware is ours (`RUL-12`): Rust, either the engine's own generated stack or a hand-written application on it |
| J2 | **A DroneCAN node:** an FDCAN transceiver and a connector, running ArduPilot's AP_Periph | It can sit on Sentinel-02's CAN bus as a sensor node; AP_Periph builds from the ArduPilot tree the image already pins, through the same hwdef path `pincheck` judges |
| J3 | Klipper, grblHAL or Zephyr | Note only whether the solved pinout would permit each. Their published support names the H743 and H723. The H753 is the H743 with crypto, so an H743 build is expected to run, unverified: say so as a prediction, not a fact |

### S2. Two layers, measured

- Build specification variants under `spike/3400/`, all at `copper_layers: 2`. Start from today's Sentinel-01, then add the *must* rows, then the *must* and *extra* rows, then everything.
- For each variant, run `validate`, `generate`, `emit`, `check erc` and `check drc --spec` from `main`'s CLI, and record each exit and finding.
- Where a variant fails for want of a specification field, record the field as S1 asks and **leave the row out of the variant**. Never hand-edit an emitted file.
- Find the largest variant that routes and passes. Record its board outline and the router's own numbers.
- **The renaming check** (`1307`'s name-dependence): the largest passing variant still passes under the 20 fixed renamings.
- **TI's SNVSAN3F** asks for an inner ground plane for the buck (`4300d2` D4). At two layers this is named as not met. Say what the bench can measure instead, because that comparison against Sentinel-02's four layers is itself a result.

### S3. The decision table

FEEDBACK §7 holds:
- **A three-tier recommendation:** which rows to carry; which engine work orders that implies, each with its size and what it unblocks; and whether `4220b` is unheld for Sentinel-01.
- **The later job:** J1, J2, both, or neither, with what each needs on the board now so that it is not a respin later (a connector, a transceiver, free pins after the solve).
- **The cost:** BOM and fabrication, with sources, for the recommended variant.

**STOP** there.

## 2. Acceptance

- Every row in S1 has every field, or names why it cannot.
- Every variant's commands and exits are recorded, so they can be rerun from `spike/3400/` on `main`.
- The renaming check is recorded for the largest passing variant.
- No file outside `spike/3400/` and `docs/work-orders/AET-WO-3400.FEEDBACK.md` changes. The CLAUDE.md gates pass.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-11`, `RUL-12`, `RUL-19`; roadmap §6 (capability through the specification).
- **Fence:** `spike/3400/` and FEEDBACK. No crate, no workflow, no version number of any kind.
- **Running beside you:**
  - `AET-WO-2302` is in the CLI and chaos.
  - `AET-WO-1208` is in `ir`'s validation and `power_parts.json`.
  - `4403`'s follow-up is in `container/`.

  You read `main`; you touch none of theirs.
- One PR, left unmerged, as the record. Write `~/.cache/aet-3400/pr-title-body.md`.

## 5. Adversarial pass, targeted

On the measurements only:
- **A variant that passes because a constraint was dropped.** Diff each variant's `net_constraints` and rules against today's Sentinel-01, and name every relaxation.
- **A *must* row whose "why" does not cite a prediction it would test.** Demote it.
- **A cost with no source.**

## 7. Flag-and-stop conditions

- **Today's Sentinel-01 does not pass S2's commands on `main`.**
- *Amended 2026-11-16 (§0):* a loaded board that does not route on two layers no longer stops the spike. It is reported beside the four-layer result, and S4 prices both.
- **A candidate needs a credential or a purchase to measure.**
