# AET-WO-1208 — `validate` judges the MCU's own supply, and refuses the rest of what `solve` refuses statically

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1208-validate-mcu`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog, written 2026-11-16 from `AET-WO-1207`'s FEEDBACK (PR #178). **Cadence: build, with one measured stop (§1.1).**

**Grounds (`1207` FEEDBACK §8):**
- **F2, a real defect on `main`.** The MCU's own supply is not judged.
  - An MCU rail of 3.8 V or 5.0 V, with one TMP117 (1.8 to 5.5 V) and no SPI bus, passes `validate` and generates a board.
  - DS12117 operates the STM32H753 at 1.62 to 3.6 V.
  - The only bound on that rail today is the speed-table rule, and it fires only beside an SPI bus.
  - Measured by `1207` in `~/.cache/aet-1207/reach4/`.
- **F1.** `solve` still refuses these alone, although each is decided by the specification and the fixed reset clock:
  - `BusTooFast`, `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit` (validation's own variants, which `1206` §1.5 kept at `solve`);
  - `MotorTiming`, `SerialBaudOutOfRange`, `SerialBaudTolerance` and `SpiSpeedBelowKernelRange`.
- **F3.** A library caller of `schematic::emit`, passing an unvalidated specification, still gets a shorted board when a serial or motor net is named for a domain or another spelled net (`VDD_3V3`, `I2C_SDA`, `MOTOR4`). The emitter checks its own nets against the spelled ones, but never the spelled ones against each other.

> **What this is:** the MCU's rail judged against the MCU row's cited operating range, by name, at `validate`. The rest of `solve`'s static refusals move to `validate` by `solve`'s bytes, as `1206` and `1207` did. The emitter stops trusting an unvalidated caller.
>
> It is NOT:
> - a change to any emitted byte for a specification that passes today;
> - a per-device SPI mode (`1207` F5, its own row).

## 1. Build

1. **Measure first (the stop).**
   - Run every candidate rule over every example, fixture and scenario.
   - **STOP** with the list if any rule refuses a specification that `generate` accepts today.
   - For F2, read DS12117's operating-conditions table at source and cite it by table and page. If `power_parts.json`'s MCU row lacks the range, add it as cited data.
2. **The MCU's supply (F2).**
   - `validate` refuses an MCU domain outside the row's operating range, by a new named refusal that cites the row.
   - Judge the domain's declared range: its minimum and its maximum against the MCU's limits, as `4230` moved the pull-up floor to the domain maximum.
3. **The static refusals (F1).**
   - Each moves to `validate` with `solve`'s kind, `id`, `claimants` and message.
   - The bytes are pinned by a tier-1 test per variant.
   - Delete `solve`'s copy where no library caller can reach it without `validate`.
   - Keep the order `solve` used, and record any multi-fault first-refusal change, as `1207` §5 did.
4. **The emitter (F3).** Either:
   - **(a)** `schematic::emit` calls `Specification::validate` first; or
   - **(b)** it checks the spelled names against each other.

   Measure the cost of (a) on the largest fixture, choose, and say why.

## 2. Acceptance

- Every new refusal has a tier-1 test showing the old behaviour first (`validate` passes, while `generate` passes or `solve` refuses).
- F2's test is the measured 5.0 V specification.
- No existing example, fixture or scenario changes its verdict.
- Every emitted file is byte-identical.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`; `RUL-16`.
- **Fence:**
  - `aethelgard-ir` (`lib.rs` validation, `solve.rs`, `power_parts.rs` and `power_parts.json`'s MCU row);
  - `aethelgard-eda` `schematic.rs` (the F3 change only);
  - tests;
  - FEEDBACK.
- `AET-WO-2302` is running in the CLI and chaos; you do not touch the CLI.
- **Versions:**
  - A `power_parts.json` schema change takes the next number.
  - No specification field is needed. If one is, say so at the stop.
- One PR, left unmerged. Write `~/.cache/aet-1208/pr-title-body.md`.

## 5. Adversarial pass, targeted

- An MCU domain whose minimum is in range and whose maximum is out of range, and the reverse.
- A domain exactly at 1.62 V and at 3.6 V.
- An MCU rail driven through a regulator whose output is in range but whose declared tolerance is not.
- A specification with two of F1's faults, to check the first-refusal order.

## 7. Flag-and-stop conditions

- **§1.1 finds an existing specification a new rule would refuse.**
- **DS12117's range cannot be read at source.**
- **Keeping the JSON byte-identical would need two error types to stay apart.**
