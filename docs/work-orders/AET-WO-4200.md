# AET-WO-4200 — SPI devices end to end: parts as data, a witnessed SPI read in the twin, a static SPI fault

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4200-spi-devices`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P4, its first work order (Scotty's order, 2026-11-12: `2301`, then `4200`, then `4100`). **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**

**Grounds, each read on `main` before this work order:**
- **SPI is declared but never exercised.**
  - The reference specification declares SPI bus `FLASH` (20 MHz, `VDD_3V3`) with `U4`, a `W25Q64JV`, whose chip select is `FLASH_CS` (PA0).
  - `aethelgard-ir` has `SpiBus` and `SpiDevice`, and the solver assigns SPI1 to PA5/6/7 (AF5) and PA0.
  - The twin's platform adds `spi1` with `spi4`'s model, `SPI.STM32H7_SPI` (`twin/src/platform.rs`).
  - The generated firmware configures the pins (`svd-hal/src/board/firmware.rs`, `Kind::Spi`) and makes **no SPI transfer**.
  - Nothing models the flash behind `spi1`, and nothing witnesses an SPI read.
- **Review B7.** Parts are code in several crates: addresses, identity registers, pin capacitance, sink limits, and which Renode model a part has. P4 needs IMUs, a barometer, regulators and a USB PHY. A parts table as data opens this work order.
- **`AET-WO-2204`.** The boot judge accepts a device read only when Renode logs the device model's own transaction lines (`architecture.md` §2.20, `RUL-19` (5)). An SPI read must be witnessed the same way, or named unwitnessable.
- **The roadmap, P4.** Which flight-controller IMUs Renode 1.17.0 models is unverified. A flight controller (Sentinel-02, `AET-WO-4500`) needs at least one IMU and one barometer on SPI.

> **What this is:**
> - SPI made real from the specification to the twin: a part table as data;
> - an SPI identity read the generated firmware makes, and the twin witnesses;
> - one static SPI fault the campaign renders and attribution explains.
>
> It is NOT:
> - stackups (`4100`);
> - flight I/O (`4300`);
> - ArduPilot (`4400`);
> - Sentinel-02 itself (`4500`);
> - DMA, interrupts, or more than one transaction per device. Multiple transactions are `AET-WO-2204` A4's question, ruled with the IMU work.

## 1. Spike

Probes live under `spike/4200/`, in any language Furnace or the pinned image runs (roadmap §6; the build deletes `spike/`). **Predictions go into FEEDBACK before any run.**

**S1. What Renode 1.17.0 models on SPI.** Read at source, at the pinned commit:
- every SPI peripheral model, and every flash, IMU, barometer and other sensor model that attaches to an SPI controller;
- for each model: its part, the commands or registers it answers (an identity or JEDEC read above all), and what it logs and at which level (the `2204` witness needs a line no store can produce).

List the flight-controller parts by name, modelled or not: ICM-42688-P, ICM-20602, MPU-6000, BMI088 (accelerometer and gyro dies), BMP388, DPS310, MS5611, and the W25Q64JV or a JEDEC-compatible flash.

**S2. A transfer through `spi1`.** Measure, in the pinned image:
- whether `SPI.STM32H7_SPI`, as `spi1`, carries a transfer to an attached model, and how chip select reaches it (a GPIO connection in the `.repl`, or the controller's own NSS);
- a probe firmware that reads the W25Q64JV's JEDEC ID (`0x9F` → `EF 40 17`), or a modelled stand-in. Record what the controller registers and the model's log show, and what a store-only forgery shows (the `2204` question for SPI).

**S3. The parts table as data (B7).** Propose its shape and home, without building it:
- **Fields:**
  - bus kind;
  - I2C address options, or SPI mode (CPOL and CPHA) and maximum clock;
  - the identity register and its expected value;
  - pin capacitance and sink limits where the electrical gates use them;
  - supply range;
  - Renode model, or "unmodelled";
  - the datasheet and its revision;
  - the witness format.
- **Constraints:**
  - `INV-01` puts shared types in `aethelgard-ir`; `RUL-12` says Rust.
  - Say whether the data is a Rust table, or a pinned data file read by Rust with a refusal for malformed entries (`INV-06`).
  - List every place a part is code today (`grep` the crates), and what migrates.

**S4. The SPI witness.** For the part S2 reads, give the exact whole lines the model logs, at which level, and whether a forgery can produce them. If no SPI model logs a transaction, say what Renode does record that a store cannot (`2204`'s rule: prefer what Renode stores over what the firmware claims), or that SPI reads are unwitnessable in 1.17.0 (a §7 condition).

**S5. A static SPI fault.** Propose the first SPI fault the campaign renders. Candidates:
- chip select never asserted;
- MISO stuck at 0 or 1;
- the wrong clock phase;
- the device absent;
- SCK above the part's maximum.

For each:
- how the twin renders it with stock models;
- what the trace records;
- how attribution would name it (electrical or software).

Measure the one you recommend.

**S6. The analog side.** SPI lines are push-pull. Say whether any analog check (SCK edge rate against the trace capacitance and the pin's `OSPEEDR` drive) is worth a gate in this work order, or belongs with the stackup (`4100`). Reason from DS12117 Rev 10, which is held on Furnace (`~/.cache/aet-2301/ds/`).

**Stop** with the table, a recommendation per question, and the decisions needed.

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **The parts table as data.** Every existing part migrates, and a test pins each part's values to its datasheet citation.
2. **The generated firmware makes an SPI identity read** (the W25Q64JV's JEDEC ID, or the ruled part), and records it as the I2C read does.
3. **The twin models the device** with a stock model, wires its chip select, and **the boot judge witnesses the read** under `2204`'s rule: whole lines, the value matched, a forgery failing.
4. **One static SPI fault** in the campaign, with its trace records, its replay, and an attribution rule. A seeded case for each domain it can be (electrical or software).
5. **The board crate's typestates and Kani harness cover SPI.** A misconfigured SPI pin fails to compile, or fails the proof.
6. **One flight-controller IMU or barometer the spike finds modelled**, declared in a test-fixture specification and read end to end, unless the ruling defers it to `4500`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02` (the generated board crate stays `no_std`), `INV-03`, `INV-04`, `INV-05`, `INV-06`, `INV-07`; `RUL-12`, `RUL-14` (SPI references), `RUL-16`, `RUL-19`.
- **Fence (at the build, refined by the ruling):**
  - `aethelgard-ir` (the parts table and SPI solving);
  - `aethelgard-svd-hal` (the firmware's SPI read, typestates, harness);
  - `aethelgard-twin` (models, platform, boot);
  - the CLI's judge and campaign;
  - `aethelgard-chaos` (the SPI fault's records);
  - `aethelgard-evolve` (its rule);
  - the seeded set, tests and fixtures;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **The reference board's committed generated files change** (the firmware now reads the flash). Name every changed file in FEEDBACK, with the regeneration that shows it. A trace version bump is shared with whatever `2205` and later work leave; take the next number once.
- One PR, left unmerged; retitle it when the spike becomes the build. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The spike table, ruled.**
- **The reference firmware reads U4's identity,** witnessed, in `simulate` and in the campaign. A forged SPI record fails with a named finding.
- **The SPI fault's seeded cases** attribute as their manifests say, and the reference's other verdicts are unchanged.
- **Every part's values are data,** each cited to its datasheet.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes (`AET-WO-1908` §8).

## 5. Adversarial pass, full

1. Forge an SPI read: registers by hand, a read of the wrong device, chip select held for another part, a value written after the transfer.
2. Make a correct SPI read fail: timing, mode, clock, the order of chip select and the transfer.
3. Make the parts table lie: a malformed entry, a duplicate part, a value out of range. Each must be refused by name.
4. Make the SPI fault attribute wrongly: a software fault that looks electrical, and the reverse.

## 7. Flag-and-stop conditions

- Renode 1.17.0 has no SPI model that logs or stores a transaction a store cannot forge (S1, S4). SPI reads would then be unwitnessable without a model of our own, which is a ruling.
- The W25Q64JV and every stand-in are unmodelled, and the read cannot be made against a stock model.
- A change to any I2C verdict on the reference, for a reason other than one this work order names.
