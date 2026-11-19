# AET-WO-4600 — predictions, committed before any measured run

Written 2026-11-19 ~16:20 UTC on `main` `cd43b207`. The CLI under test is `target/debug/aethelgard`, built inside the pinned tools image from `cd43b207`'s crates (sha256 `f9be51d0…ebede`, bridge `44abce24…10cb`; FEEDBACK §0 says how). Each row is per (specification, step): expected exit, refusal kind or verdict, class (§1.4 (a)–(d)), and the source it rests on, **read at source** unless marked **guess**. Any run not listed here is reported as unpredicted.

## 0. What was seen before this file

These were observed, not predicted. Each is reported as "observed at authoring".

- `validate` on `logger.json` and on `driver.json` (each exit 0) ran while they were being written. That run is §1.1's authoring step. No variant has been validated.
- §1.5's `git grep` ran: its result is a measurement, not a run of the engine. The work order's pathspec `crates/*/src` returns 2 hits (manifests only); `crates/*/src/**` returns 62.
- `aethelgard <subcommand> --help`, to read the argument shapes.

## 1. Exit codes (read at source, `cli/src/main.rs` L47–59)

0 pass · 1 fail (a gate's verdict) · 2 invalid specification · 3 unreadable / refused before a gate (`verify`'s `CampaignCoverageShort` and `UnwitnessedFieldNotAcknowledged` use 3, `verify.rs` L1221–1240) · 4 tool unavailable · 5 tool error · 6 generator report · 64 usage · 70 internal.

## 2. The architect's §1.6 table, confirmed or corrected

| Spec | Step | Architect | Mine | Class | Why |
|---|---|---|---|---|---|
| L | `validate`…`verify` (no fab) | pass, TMP117s unmodelled, listed | **confirm.** `simulate` lists U10–U13 under `unmodelled` (`twin/src/boot.rs` L565–571) | (d) | the twin cannot see any TMP117 |
| L | coverage | `TEMP` covered by a tolerated pull-up fault with no TMP117 answering | **confirm: covered.** `pullup-3k3-temp` is `Tolerable([I2c(TEMP)])` (`verify.rs` L733–740, L831–833); coverage counts a bus "used" by its declared devices (L907–908), never by a modelled one | **(b)**, with (d) beneath it: the twin's limit is (d); a coverage line that says `TEMP` is covered when nothing on it is witnessed is (b) | |
| L | `--fab-out` | written, once every ledger row is acknowledged | **first run refused `UnwitnessedFieldNotAcknowledged`, exit 3**: the spec carries `acknowledge_unwitnessed: []`. Coverage (checked first, L1222) passes. The rows are then written by hand from the refusal's list (stage 2), and the second `--fab-out` writes a package | (a) at stage 1; — at stage 2 | |
| `logger-i2c-only` | `generate` onward | unknown | **pass.** The harnesses do not depend on a bus (`svd-hal/src/board.rs` L3961, L4867, L5364, L5443, L5695: option word, schedule, reset clock, timers, USART) | — | **guess** for the twin and the bridge |
| `logger-spi-only` | `generate` onward | unknown | **pass.** A droop's observation falls back to the first SPI witness on a board with no I2C read (`chaos/src/campaign.rs` L1196–1201, L1228–1240). `generate hwdef` refused or written without a baro (not a gate) | — | `extract` with no I2C line: **guess** pass, judging SCK and MOSI only |
| `logger-1s` | `validate` | refused: the buck needs VIN ≥ 3.8 V, the LDO gives 1.8 V | **confirm, exit 2, kind `BatteryOutsideRegulatorInput`** (`ir/src/power.rs` L1316–1340, D-B9). Everything after it is `not_run` | (a), and (c): no 3.3 V regulator from 1S | |
| `logger-offboard` | authoring | not expressible | **confirm.** A device entry is a placed part. The closest shape (U13 dropped, the cable as offboard capacitance on J5) validates. Every gate passes, and the engine never knows a fourth sensor exists | (c) | `extract` **guess**: passes, 65 pF off-board plus the copper under the 100 pF bound |
| M | `validate` | pass | **observed at authoring: pass** | — | |
| M | `generate` onward | unknown, `no_harness` possible | **pass, no `no_harness`.** Every harness above exists with no bus. `extract` judges nothing: **guess** pass with no line (or a refusal "no line", which would be (b)) | — / (b) if refused | |
| M | `simulate` | pass, H-bridge outputs at 1 µs of 50 µs from boot | **confirm pass.** CCR = `pulse_min_us` (`ir/src/motor.rs` L259, "starts the output at the range's minimum", L66–70) | **(b)** | |
| M | "motor runs at boot" | the motor runs | **corrected: depends on the driver class.** `solve` puts outputs with one key (rate, PSC, ARR) on one timer (`ir/src/solve.rs` L87, L1064). **Guess:** HB_IN1 and HB_IN2 share a timer, edge-aligned and in phase. For an IN/IN bridge (DRV8833, DRV8871), IN1 = IN2 = 1 for 1 µs is brake and 0/0 is coast: no net drive, but a 2 % brake chop, not the coast the board wants. For a PH/EN bridge (DRV8874 in PH/EN mode), EN at 2 % drives the motor at boot | (b) either way | the twin judges neither |
| M | servo outputs | — (not in the table) | **added:** the servos idle at `pulse_min_us` 1000 µs, a servo's end stop, not its 1500 µs centre. Every servo swings to one end at boot, and no gate objects | **(b)** | `motor.rs` L66–70 |
| M | duty resolution | 50 steps at 20 kHz (1 µs ticks) | **confirm:** PSC = kernel/1 MHz − 1 (`motor.rs` L253–259). Every PWM output ticks in whole µs, whatever its rate | (b) | |
| `driver-idle-zero` | `validate` | `PulseRange` (`lib.rs` L3543) | **confirm, exit 2, `PulseRange`** at `ir/src/lib.rs` L3542–3547 (step 13b). Even if allowed, `solve_timing` refuses `ccr == 0` (`motor.rs` L262) | **(b)** | |
| `driver-tree` | `validate` | — | **guess pass.** Loads are not checked for completeness: undeclared device currents are only noted (`power.rs` L3020–3029, "A1") | — | |
| `driver-tree` | `campaign` | `PowerNotSimulated` (`campaign.rs` L382) | **confirm, exit 2,** `cli/src/campaign.rs` L380–389 | (c), owner `2304` | |
| `driver-tree` | coverage | `VDD_5V` and `battery` unmet | **confirm:** `verify --fab-out` exit 3 `CampaignCoverageShort`, unmet `VDD_5V` (not the MCU's supply) and `battery` (`verify.rs` L930–944). The coverage refusal comes before any gate, so `PowerNotSimulated` never shows in `verify` | (c), owner `2304` | |
| M, L | `generate hwdef` | refused or meaningless | **guess:** L refused or written with no baro line (TMP117 `ardupilot` null); M written (console, four PWM outputs). Either way not a gate (D1) | not a gate | |
| all | power input | only an XT30 and a cell pack | **confirm** (`ir/src/connectors.rs` L165; `power.rs` L200–213) | (b) for the power-in choice, (c) for USB or a wall adapter | |
| all | serial roles | flight-ecosystem roles and `console` | **(c), not (b):** `console` is neutral enough for a command port and a readout. Missing is a role for a non-console protocol (Modbus, a host link) | (c) | `serial.rs` L31–37 |

## 3. Added suspicious passes (pre-registered)

1. **M's coverage has no motor target.** `coverage()` walks I2C buses, SPI buses, ports and rails (`verify.rs` L903–944). Prediction: M with `baseline`, `droop-bor3` and `serial-rx-open-cmd` meets coverage, with no fault on any of its four outputs. **(b)**: a board whose purpose is its outputs is "covered" with none of them faulted. No fault kind reaches a timer output (`chaos/src/record.rs` L173–191): (c) as well.
2. **M's droop has no device witness.** On a board with no I2C or SPI read, the droop's observation compares the boot word, status and value only (`chaos/src/campaign.rs` L1215–1220: `has_witness` false and `witness` none). Prediction: `droop-bor3` passes on M with nothing on the board read back after the reset. Class (d).
3. **L's `TEMP` bus (four TMP117s) is covered and witnessed by nothing.** See §2.

## 4. Per-specification tables (exit · verdict or kind · class)

Steps run in the order of `run.sh`. Where my prediction is a guess, it says so.

### L — `logger.json` (stage 1: `acknowledge_unwitnessed: []`)

| Step | Exit | Verdict / kind | Class |
|---|---|---|---|
| validate | 0 | ok (observed at authoring) | — |
| solve | 0 | topology, two I2C peripherals, one SPI, one USART | — |
| emit | 0 | | — |
| erc, drc, support | 0 | pass | — |
| speeds | 0 | pass, LOG SCK/MOSI judged | — |
| extract | 0 | pass, four I2C lines and LOG's SCK/MOSI | — |
| generate, build | 0 | | — |
| prove (each) | 0 | pass | — |
| simulate | 0 | pass; U10–U13 `unmodelled`; U20 and U30 witnessed | (d) for U10–U13 |
| campaign | 0 | every run meets its expectation | — |
| replay, attribute | 0 | | — |
| verify | 0 | pass | — |
| verify-fab (stage 1) | 3 | `UnwitnessedFieldNotAcknowledged`, listing every row | (a) |
| verify-fab (stage 2, acks written) | 0 | pass, package written; manifest sha quoted only | (b) risk: coverage of TEMP (§3.3) |
| hwdef | ≠0 **guess** | refused, or written without TMP117 lines | not a gate |
| pinout | 0 **guess** | pass (generated firmware, own topology) | not a gate |

### M — `driver.json` (stage 1 as L)

| Step | Exit | Verdict / kind | Class |
|---|---|---|---|
| validate | 0 | ok (observed at authoring) | — |
| solve | 0 | four timer channels (two timers), one USART | — |
| emit, erc, drc, support | 0 | pass | — |
| speeds | 0 | pass, nothing in scope | — |
| extract | 0 **guess** | pass, no line judged | — (or (b) if it refuses) |
| generate, build | 0 | | — |
| prove (each) | 0 | pass, harnesses for TIMx and USART present | — |
| simulate | 0 | pass, HB outputs high 1 µs of 50 µs, servos 1000 µs of 20 ms | **(b)** §2 |
| campaign | 0 | three runs meet expectations | — |
| replay, attribute | 0 | | — |
| verify | 0 | pass | — |
| verify-fab stage 1 | 3 | `UnwitnessedFieldNotAcknowledged` (coverage met: §3.1) | (a); (b) for coverage |
| verify-fab stage 2 | 0 | package written | **(b) risk:** outputs unfaulted, idle 2 % |
| hwdef | 0 **guess** | written | not a gate |
| pinout | 0 **guess** | pass | not a gate |

### `logger-i2c-only`

Same as L, minus the LOG rows. No `spi-absent`. **Guess:** every step passes. `verify-fab` exits 3 `UnwitnessedFieldNotAcknowledged`; no stage 2 (variants stop at stage 1).

### `logger-spi-only`

Same as L with two SPI buses and no I2C bus. **Guess:** every step passes. U40 (ICM42688P) and U30 are witnessed. `spi-absent-u40` fails as expected, with `spi_identity_mismatch` and `spi_transaction_unwitnessed` (**guess**: the IMU's absent kinds match the flash's). `verify-fab` exits 3 `UnwitnessedFieldNotAcknowledged`.

### `logger-1s`

`validate` exits 2 with `BatteryOutsideRegulatorInput`; every other step that reads the specification refuses the same way (exit 2). Steps that need their outputs are `not_run`. Class (a) + (c).

### `logger-offboard`

As L with three TMP117s. **Guess:** every step passes; `extract` counts J5's 65 pF on TEMP's lines and stays under 100 pF. **(c):** U13 does not exist to the engine.

### `driver-tree`

| Step | Exit | Verdict / kind | Class |
|---|---|---|---|
| validate | 0 **guess** | ok, power notes; A1 note | — |
| solve, emit, erc, drc, support, speeds, extract | 0 **guess** | pass (1315 not merged) | — |
| generate, build, prove, simulate | 0 **guess** | pass | — |
| campaign | 2 | `PowerNotSimulated` | (c), `2304` |
| replay, attribute | not_run | no archived run | — |
| verify | 0 **guess** | pass (no campaign without scenarios) | — |
| verify-fab | 3 | `CampaignCoverageShort`: `VDD_5V`, `battery` | (c), `2304` |

### `driver-idle-zero`

`validate` exits 2 with `PulseRange`. Every other step that reads the specification refuses the same way. **(b).**

## 5. Resources (guess)

Each `verify` runs 9–18 min. Peak under 8 GiB, except `verify`: 2304 measured 3.40 GiB for Sentinel-03, and 4201 needed 16 GiB. No step reaches 900‰ of its cap.
