# AET-WO-4300 — Flight I/O: what a flight controller's outputs and links need, measured, then scoped

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300-flight-io`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at each build (`docs/roadmap.md` §6).
**Phase:** P4, its third work order (roadmap §5: `4200`, `4100`, then `4300`). **Cadence: spike (§1), then STOP for the architect's ruling.** The ruling scopes the build, and may split it into lettered work orders (`4300a`, `4300b`, …). The spike runs now, while `4200` and `4100` build. No build starts before `4200` merges, because the build changes the same crates.

**Grounds, each read on `main` at `2967d16`:**
- **The roadmap's intent** (§5 P4): timer outputs able to drive ESCs (PWM and DShot timing), a USB device, UARTs for a receiver and MAVLink, and the power input and regulator parts, each solved and gated.
- **The specification declares only I2C and SPI.** `Specification` has `i2c_buses`, `spi_buses` and their devices. There are no timers, UARTs, USB, ADC or power parts (`crates/aethelgard-ir/src/lib.rs`).
- **Renode's STM32H743 platform declares the peripherals.** `timer1`–`timer17`, `usart1`–`3` and `6`, `uart4`, `5`, `7` and `8`, `lpuart1`, and `usb1` and `usb2` are in `RESERVED_REFERENCES` because the platform names them (`lib.rs` L698–749). Which of them carry behaviour a judge can witness is unmeasured.
- **Carried here by ruling:**
  - **The SCK edge-rate check `Fmax(OSPEEDR, VDD, C)` and the I/O compensation cell** (`4200` D6 and F6, moved by `4100`'s ruling). DS12117 Rev 10 Table 149's speed-`10` and `11` rows assume the cell enabled (note 5), and the generated firmware never enables it. This work order owns pin speeds.
  - **2100 F24: cold-write every `RCC_*ENR`.** Owned by the first work order that solves a peripheral outside the cold registers.
- **ArduPilot is the flight stack** (roadmap §5 P4; `4400` generates its `hwdef.dat`). Whatever this work order solves, `4400` must be able to name.

> **What this is:** a measured map of what Aethelgard can solve, generate, simulate and witness for a flight controller's I/O, a proposal for how to build it, and a stop.
>
> It is NOT:
> - ArduPilot's board definition (`4400`);
> - Sentinel-02 (`4500`);
> - stackups (`4100`);
> - SPI devices (`4200`);
> - writing a flight stack (`NG-01`).

## 1. Spike

Probes live under `spike/4300/`, in any language Furnace or the pinned image runs (roadmap §6; a build deletes `spike/`). **Predictions go into FEEDBACK before any run.** Read Renode 1.17.0 at its pinned commit (renode `ab721d88`, `src/Infrastructure` `066a7f13`; `AET-WO-2204`'s FEEDBACK names the two files the image's binary differs in).

**S1. Timers as ESC outputs.**
- Read `STM32_Timer` (or whichever model the platform binds `timer1`–`timer17` to) at source:
  - whether a capture/compare channel drives a GPIO output (PWM on a pin);
  - whether its period and duty are observable from Renode;
  - what it logs, and at which level.
- **Measure** a probe firmware that sets `TIM1` channel 1 to 50 Hz servo PWM (1–2 ms pulses) and to 400 Hz PWM on a pin. What does the twin show: pin transitions with virtual timestamps, register state only, or nothing?
- **DShot (DShot300 and DShot600):** bit periods of 3.33 and 1.67 µs, produced on real flight controllers by a timer with DMA bursting duty values. Say whether Renode 1.17.0 models the H7's DMA (`DMA1`/`DMA2`, `DMAMUX1`) driving a timer's `CCR` or `DMAR`, and whether the result is observable at bit-time resolution.
  - If it is not, say what *can* be judged: the DMA and timer configuration as registers, statically; or a static timing proof from the clock tree and the prescaler.
- **For each, what a store can forge.** This is `2204`'s question: prefer what Renode records over what the firmware claims.

**S2. UARTs (receiver and MAVLink).**
- Read the platform's USART model:
  - whether a transmitted byte reaches a Renode-side analyzer or a socket;
  - whether a byte can be injected from outside (a receiver's frames);
  - what is logged.
- **Measure:** a probe firmware sends `MAVLINK` heartbeat bytes (or a fixed pattern) on `usart2`, and echoes received bytes. Record what the twin captures, with virtual time, and whether the capture is a witness (`RUL-19` (5)'s rule) or the firmware's claim.
- **Receivers:**
  - **SBUS:** 100 kbaud, 8E2, inverted.
  - **CRSF:** 420 kbaud, 8N1.

  Say whether the model honours baud, parity, stop bits and inversion, or ignores them. A model that ignores framing cannot render a framing fault.

**S3. USB device.**
- Read what the platform binds `usb1` and `usb2` to.
- **Measure** whether a probe firmware can enumerate as a CDC-ACM device (ArduPilot's console and MAVLink port) in Renode 1.17.0, and what is observable.
- If USB is register-only in the twin, say what static checks a USB design needs instead:
  - `D+`/`D−` on the right alternate function;
  - the 48 MHz clock source;
  - the 90 Ω differential pair the stackup (`4100`) now makes computable;
  - VBUS sensing.

**S4. Power input and regulators.**
- Propose the parts the parts table (`4200` D3) needs: a buck from battery voltage, an LDO, and current and voltage sense. Cover their datasheet fields and which gates read them.
- Say whether the existing analog bridge (ngspice, `2301`) can solve a regulator's input range and a brownout margin from the declared parts, or whether that is a later work order.
- **ADC battery monitoring** (ArduPilot's voltage and current inputs): in or out of this work order? Say so with a reason.

**S5. Pin speeds and the compensation cell** (`4200` D6, F6).
- **Read and measure:**
  - whether Renode models `SYSCFG_CCCSR` (`EN`, `READY`), and whether a firmware that enables the cell and waits on `READY` boots in the twin;
  - DS12117 Rev 10 Table 149 at every `OSPEEDR` setting the generator can emit, and what the speed-`10` limits are without the cell, if the datasheet says.
- **Propose:**
  - the static check `Fmax(OSPEEDR, VDD, C) ≥` each line's required edge rate, for SPI's SCK and for DShot;
  - with C from extraction (`4100`).

**S6. 2100 F24 (cold-write every `RCC_*ENR`).** Read the generator. Say whether timers, USARTs and USB fall outside the cold registers, and what F24's fix is now.

**S7. The split.** From S1–S6, propose the build as one work order or several. For each piece give:
- what it solves;
- what the specification gains (the schema version is `4100`'s 2 until it lands; take the next one once);
- what the twin witnesses or what is only static;
- its fault for the campaign, if any;
- its rough size;
- its order relative to `4400`.

A piece whose only judge is the firmware's own claim is named so, and gets no fault.

**Stop** with the table, a recommendation per question, the proposed split, and the decisions needed.

## 2. Build

Scoped by the ruling. Expected shape, for orientation only:
- each solved peripheral appears in the specification, the solver, the board crate's typestates and Kani harness, the generated firmware, the twin's platform and the judge, under `4200`'s parts table and `2204`'s witness rule;
- each witnessed peripheral gets one static fault the campaign renders and attribution explains.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01` to `INV-07`; `RUL-12`, `RUL-14`, `RUL-16`, `RUL-17`, `RUL-19`.
- **Spike fence:**
  - `spike/4300/` and this work order's FEEDBACK only;
  - no change to any crate, fixture, scenario or `ci/` file.
- **Three builders share Furnace:** `4200` (`~/aethelgard-4200`), `4100` (`~/aethelgard-4100`) and `2205` (`~/aethelgard-2205`). Never touch their worktrees. Keep each container at 8 GiB or less, and run one at a time.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance (spike)

- **Every S1–S7 question answered:** measured where it can be run, read at source where it cannot, and each answer labelled as which.
- **Predictions before runs, and the table setting each result beside its prediction.**
- **A proposed split,** each piece with what witnesses it.

## 7. Flag-and-stop conditions

- **Renode 1.17.0 models no timer output, no UART capture and no USB that a store cannot forge.** Flight I/O would then be static-only in the twin, which is a ruling for the roadmap's P4 exit gate.
- **A probe firmware that enables the compensation cell and waits on `READY` hangs in the twin.** Stop: the cell would then need a model change or a ruling.
