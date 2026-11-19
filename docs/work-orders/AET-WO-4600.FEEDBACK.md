This spike does not satisfy roadmap L455. The clause's board is written at the gate.

# AET-WO-4600 — FEEDBACK: a field logger and a small motor driver, through every gate with no engine change

**Status: STOPPED at §7, twice.** The first stop was lifted by the architect's ruling (bus, 17:08Z): `prove` on a board with an I2C bus at 16G; run the not-run list, predicted first (`spike/4600/PREDICTIONS-2.md`, `a85208c1`). The second stop: `logger-spi-only`'s `prove --harness proofs::every_valid_sequence`, at 8G under that ruling (no I2C bus), passed 900‰ of its scope, and `sh/step.sh`'s watch killed it: `killed-900`, 417.3 s, peak 8.56 GB. The batch then stopped itself. The architect re-keyed the rule (18:06Z: 16G for every unsplit-schedule board), and batch 3 (`PREDICTIONS-3.md`, `988778e9`) ran `driver-tree`, `logger-spi-only-cap` and `logger-offboard` with no further stop.

**The first stop.** A run was killed at its cap. `prove --harness proofs::every_valid_sequence` on the logger ran at the work order's 8 GiB and the kernel's OOM killer killed it: exit 143, 1,031,971 ms, peak = `memory.max` = 8,589,934,592. The user journal records scope `run-p2508243-i174431037` as "Failed with result 'oom-kill'", 8G peak, 16:20:38Z. §7 says "stop with the numbers. A cap is never raised here." The journal shows systemd stopping the scope after the kernel killed a process in it. So the CLI was terminated (143) and printed no verdict: what `prove` reports for an OOM-killed Kani is unmeasured. Every run that was queued at that moment was frozen. The one already running, L's `verify` at its own 16 GiB cap, finished under a host-side watch that would have killed it at 900‰. The architect has the scope of the stop (bus, 2026-11-19 ~16:40Z). Every row not run says so below, with the `run.sh` line that would make it.

**Base:** `main` `cd43b207` (verified live: `origin/main` has since gained `47ec18f6` (1910, CI only, plus `svd-hal`/`sha2` opt-level 2) and `2f8f8381` (2304's record); no crate change that touches a measured row. `4400g` (#238) is open, head `46a44a31`, not merged).
**Branch:** `wo/aet-4600-second-board-spike`. **Predictions:** `spike/4600/PREDICTIONS.md`, committed `e57f7cb8` at 15:59:24Z, 4 s before the first measured run (15:59:28Z). The file says "~16:20 UTC"; that was my estimate, and it is wrong.

## 0. How the runs were made

- **Image:** `ghcr.io/clearforge-llc/aethelgard-tools@sha256:e5fc7ddbe36c050e78763ad1d203ec8d176408a3e534bed5563c424d8d9a1b58` (`container/IMAGE` at the base).
- **CLI:** `cargo build --locked --offline -p aethelgard-cli`, plus the bridge, inside that image, `debug`, as CI's `bench-package` job builds it. `target/debug/aethelgard` sha256 `f9be51d03ea6db4e8dacf2c3c036b32334f5fb618ae237a78a0ecc933e0ebede`, `aethelgard-bridge` `44abce244ad870c48c2cc03c50d9613ec9feea05a304b4612bc17bf2997810cb`.
  - The build was **incremental on 2304's target directory**. In 2304's worktree, `git diff --stat cd43b207 HEAD -- . ':!spike' ':!docs'` is empty, and so is the working tree against HEAD outside those two directories, so that target was built from `cd43b207`'s bytes everywhere outside `spike/` and `docs/`. Cargo recompiled `aethelgard-twin` and `aethelgard-cli` from this worktree.
  - Not a clean build. A clean rebuild is the check if a figure here is ever doubted.
- **Containers:** `spike/4600/sh/img.sh` wraps `~/bin/capped-run`. Each step runs in its own container (`spike/4600/sh/step.sh`), so every row's peak is that step's own scope `memory.peak`.
  - `--network none`, `nice -n 10`, one container at a time (`flock`).
  - The caps are read back inside each scope: 210 readbacks over the three batches, every one `memory.swap.max=0`, in `spike/4600/out/caps.log`.
  - `--mem 8G` for every step but `verify`, which ran at `16G`.
- **Remake:** `spike/4600/run.sh <spec> [steps]`. The variants come from `spike/4600/sh/variants.py`, the scenarios from `spike/4600/sh/scenarios.py`, the stage-2 acknowledgements from `spike/4600/sh/acks.py`.
- **Outputs:** each step's stdout line, its non-empty stderr and its rows are in `spike/4600/out/<spec>/`. Generated trees, ELFs and archives are in `~/.cache/aet-4600/runs/`.
- **ELFs:**
  - M `a21823e87942d49bef42041a4a17e741cc98208f6a8ad613140623239618795a`;
  - L `170258353f30c13048ce06f302c583dbdcd86450149eaf15336ce6418ee02bb8`.
- **Generated trees and archives** (fence: quoted, never committed):

| spec | board crate (`build`'s input sha256) | scenario · run id · `trace.cbor` sha256 |
|---|---|---|
| M | `dd640e559e0aa275d033d63b560a7897b1cd7fdd91bc7377e885f1f7743465b5` | baseline · `b3002ad5…` · `63ea895fded2201134d5e4830f345e88a0ae5c76234e6fa6ebd9e28fb5f38472`; droop-bor3 · `b8e83a06…` · `a2b05ac42f5c3f18c1f1ee3db41c87869f657afd53e2e6e2226b9024007cca98`; serial-rx-open-cmd · `2aed7a1c…` · `cc12ebbad4a5078489a267b62833195d3e62ce9ac63bbc0ea43cff3714c2ddc7` |
| L | `4c6bfe3a613eeed069fa4507ec84be022f6f035687c464e89137fb5b4d2709c3` | baseline · `c4c170f1…` · `1921ba0ab135a535a46c81a8f660658229be9f228e3102aa3816048c1339e1f3`; pullup-3k3-temp · `7a6ee014…` · `60649cf8a1e80fdc4ec094707d6bee17be4e8692b94c7f6b6fe9b0e56b098a75`; pullup-3k3-baro · `cd3d7a0d…` · `e0f0af17264b5f0787a80b28a15ca5c49fc8d2dffbd77fca55515bf4a18b2b75`; spi-absent-u30 · `1846f8d4…` · `ae6c523201e2f8e7ed9d2f30434007b1b13e8277c048b7d018a510bf4f49f5ae`; droop-bor3 · `6166d962…` · `8211655282e8c52c772f59fa53066d36f2534092cba07a930b063786a5a9349d`; serial-rx-open-readout · `e9c94973…` · `5e78ad4b991b48632adbd135287bc3a3077c258446c31fa8877a3ca40420c694` |

- **Specification hashes measured:**
  - M (stage 1, no acknowledgements) `12431afa4768…`;
  - L (stage 1) `d17ecab93094…`.
  - `spike/4600/driver.json` as committed is **stage 2: acknowledgements written, never validated or run** (§7 froze it).

## 1. The two boards, and what changed from the proposal

**L, the field logger** (`spike/4600/logger.json`, project `field-logger`). It is the architect's shape unchanged:
- I2C `TEMP`: four TMP117s, U10–U13, at 0x48–0x4B;
- I2C `BARO`: a BMP581, U20, at 0x46;
- SPI `LOG`: a W25Q64JVxIQ, U30, its `nrnd` acknowledged;
- serial port `READOUT`, role `console`;
- bench 3.3 V on `SM04B-GHS-TB` J3 (`mcu.supply_source`), SWD on `SM06B-SRSS-TB`, two layers.
It differs from Sentinel-01 on purpose: no BME280, two I2C buses, five sensors, one of them modelled.

**M, the motor driver** (`spike/4600/driver.json`, project `bench-motor-driver`). It is the architect's shape unchanged:
- `HB_IN1` and `HB_IN2`: `pwm`, 20 kHz, 1–49 µs;
- `SERVO1` and `SERVO2`: `pwm`, 50 Hz, 1000–2000 µs;
- port `CMD`, role `console`;
- no I2C or SPI bus;
- bench 3.3 V as L; the outputs leave on `SM06B-GHS-TB` J4.

**Variants** (one change each; everything the change forces goes with it, listed in `variants.py`):
- **`logger-offboard`, changed from the proposal.** Written as first sketched (U13 still declared, J5 added), it tests nothing: that is Sentinel-03's J8 path. As built, U13 leaves `i2c_devices` (no construct declares a device off the board) and J5 carries TEMP's lines with 65 pF off-board each. The probe exists only as copper capacitance.
- **`logger-1s-u50`, added after the predictions and not run.** `logger-1s` copied the power fixture's regulator U11 onto a board whose U11 is a TMP117. `logger-1s-u50` renames the regulator, and changes nothing else.

**Authoring cost (measured)**
- **Times.**
  - Session start (first tool call) 15:47:54Z. L was accepted by `validate` at 15:51:21Z, M at 15:51:39Z.
  - Each spec was written in one pass, its skeleton copied from Sentinel-01-minimal by an agent. Time from the authoring marker to acceptance: L 20 s, M 18 s.
  - These are an agent's times, not a person's. They say only that the shape is copyable.
- **Bytes and acknowledgements**, as minified JSON so formatting does not count:

| | as written (stage 1) | acknowledgement entries / fields `--fab-out` demands | bytes with them |
|---|---|---|---|
| Sentinel-01-minimal (reference) | — | 27 entries (in the file) | 11,446 (16,596 as formatted) |
| L | 4,391 (5,607 formatted) | **32 / 188** (from its `verify` ledger; not written: §7) | — |
| M | 3,049 (4,037 formatted) | **41 / 250** (from the refusal) | 12,389: **75 % acknowledgements** |

- **What writing them took, for M:**
  - The refusal (`UnwitnessedFieldNotAcknowledged`) names 250 fields and **no class**. The class of each row is only in `verify`'s JSON ledger (`ledger.rows[].class`), a second run.
  - An entry's `reason` must be 1–200 bytes of printable ASCII. I learned that from a refusal, `InvalidSource` at `acknowledge_unwitnessed[23].reason`, after writing 41 reasons. A `§` also fails it.
  - A driver with no bus needs more acknowledgement entries (41) than Sentinel-01 with three buses (27): every timer channel adds rows.
  - The only practical way to write 250 fields is a script. The reasons in `acks.py` are mine, one per (register kind, class), not copied from the ledger's `judge` column. Copying that column would make the acknowledgement say nothing the artifact did not already say (`INV-07` in spirit). See F12.

## 2. What a real one would need (§1.2)

The datasheet rows were read by a research subagent from TI's PDFs and from distributor pages (`~/.cache/aet-4600/research/datasheets.md`). I spot-checked three document numbers in the extracted PDFs (SBVS320D, SBVS322C, SLVSEU9D). "Unconfirmed" means it was not read from the maker's own document.

| Need | Candidate, datasheet | Beyond a row, the engine needs |
|---|---|---|
| **Driver: a bridge with no bus** | TI DRV8833PWPR, SLVSAR1E (Jul 2015): AIN1/2, BIN1/2, nSLEEP, nFAULT (open drain), AISEN/BISEN are bridge returns for a shunt. TI DRV8876PWPR, SLVSDS7B (Nov 2019): EN/IN1, PH/IN2, nSLEEP, nFAULT, **IPROPI 1000 µA/A** analog current output. DRV8871DDAR, SLVSCY9B (Jul 2016): IN1/IN2/ILIM only, no sleep or fault pin | a **device kind without a bus** (specification construct); **plain GPIO** outputs (sleep, direction) and inputs (fault) as declared signals; an emitter arm in `schematic.rs` plus a vendored symbol and footprint; a model under `models/renode/` if the twin is to see it |
| **Driver: current sense** | DRV8876 IPROPI (above), or TI INA180A1IDBVR, SBOS741H (Jul 2022) | an **ADC input** (the specification has only `power.sense`'s battery divider, `analog_lines` in the topology); a fault kind on it |
| **Driver: motor-current connector** | JST B2B-XH-A, 3 A (catalog copy; JST PDF not read: unconfirmed) | a connector row (`connectors.rs` L67–110 lists four JST parts and one power-in) and its current rating as a field |
| **Driver: fault on the motor rail** | — | a fault kind beyond the six overrides (`chaos/src/record.rs` L173–191) and a rail that is not the MCU's supply (coverage marks it unmet by kind, `verify.rs` L930–938) |
| **Logger: 3.3 V from 1S** | TI TPS63802DLAR buck-boost, SLVSEU9D (Jan 2021): VIN 1.3–5.5 V, 2 A at 3.3 V from VIN ≥ 2.3 V. An LDO is the wrong part: TLV75533P (SBVS320D, Sep 2024) drops 215 mV max at 500 mA, so it drops out below about 3.52 V of a 3.0–4.2 V cell | a **buck-boost kind** in `power-parts.json` and in `power.rs`'s checks (today only `Buck` and `Ldo`, `power.rs` L1006 `Kind`), its emitter (`schematic_power.rs`); a **power input that is not an XT30** (a cell holder or a JST PH lead) |
| **Logger: RTC crystal (LSE)** | Abracon ABS07-32.768KHZ-T, 12.5 pF (distributor; ESR 70 vs 95 kΩ conflicts: unconfirmed); ST AN2867 (revision unconfirmed: st.com not readable) | `4240` covers the HSE only (its stop on #240: H1+VOS2, a 16 MHz crystal). The LSE is not in its scope; it needs its own row, an `mcu.lse_hz` field and an emitter arm. **Owner: none yet** |
| **Logger: removable medium** | Molex 104031-0811 microSD (spec PS-104031-001-001, revision unconfirmed); SD Physical Layer Simplified Specification Part 1 v9.10 (2023-12-01), SPI mode | a socket row with card detect (a GPIO input), and an SD-card device kind on SPI with no JEDEC identity (`parts.json`'s `identity` assumes a register read) and no Renode model in the image (inferred) |
| **Either: a wall or USB supply** | GCT USB4105-GF-A (drawing rev B via Farnell; GCT site 403), CC1/CC2 5.1 kΩ to GND each (secondary sources) | a power input kind other than a battery: `power.battery` is a series pack of `cells` (`power.rs` L200–213), and `mcu.supply_source` is a connector pin with no source declared behind it |

## 3. Predictions beside results

Exit codes: `cli/src/main.rs` L47–59. Classes per §1.4. A miss is marked **MISS**.

### M — `driver.json` (stage 1, spec `12431afa…`)

| Step | Predicted | Measured (exit · verdict/kind · wall · peak) | Where it arises | Class |
|---|---|---|---|---|
| validate | 0 (observed at authoring) | 0 · ok · 0.0 s · 0.03 GB | — | — |
| solve | 0, two timers | 0 · HB_IN1/HB_IN2 on **TIM1 CH1/CH2** (PA8/PA9), servos on TIM2 CH1/CH2, PSC 63, ARR 49 / 19999, CCR 1 / 1000 | `ir/src/solve.rs` L1064 (one key per timer) | — |
| emit · erc · drc · support | 0 pass | 0 · pass · 1.2/1.2/1.7/0.8 s · ≤0.32 GB | — | — |
| speeds | 0, nothing in scope | 0 · pass, **`judged: 0`**; every motor output "out of scope … no connector part declares it yet", though J4 carries them | `cli/src/speeds.rs` L16–17 ("0 for a specification with no SPI bus"); the out-of-scope text `svd-hal/src/board.rs` L900 | (b) note: a pass that judged nothing (F5) |
| extract | 0 guess | 0 · pass, **`lines: []`** | `cli/src/extract.rs` L187 (`bound::lines`: I2C lines and SPI SCK/MOSI only) | (b) note (F5) |
| generate · build | 0 | 0 · 1.8 s · 0.4 s | — | — |
| prove ×8 | 0, no `no_harness` | 0 ×8. The schedule is **split per peripheral** (`every_valid_sequence_tim1`/`_tim2`/`_gpio`), largest 32.1 s, 2.49 GB | `svd-hal/src/board.rs` L4867 | — |
| simulate | 0, HB 1 µs of 50 µs, servos 1000 µs | 0 · pass. `silicon`: HB high 1,000,000 ps of 50,000,000 ps, `holds: true`. Twin: HB period 12,544 ns, high 256 ns; servo 5,119,744 / 256,000 ns, `witnessed`. **HB_IN1 and HB_IN2 first rise at the same instant (27,620 ns)** | `ir/src/motor.rs` L259 (CCR = `pulse_min_us`) | **(b)** F2 |
| campaign | 0, three met | 0 · 3/3 met (baseline pass; droop-bor3 pass; serial-rx-open-cmd fail as expected) · 18.7 s | — | droop: **(d)** F5 (no device witnessed) |
| replay · attribute | 0 | 0 · 0 · on the first archived run in directory order, `2aed7a1c` (`serial-rx-open-cmd`; attribute: electrical, E3) | — | — |
| verify (no fab) | 0 | 0 · pass, 14 gates · **85.5 s · 2.51 GB** of 16 GiB | — | — |
| verify-fab | **3** `UnwitnessedFieldNotAcknowledged`, coverage met | **2** · `UnwitnessedFieldNotAcknowledged`, 250 rows · 1.4 s. **Coverage met: no fault on any motor output** | `cli/src/verify.rs` L1233–1246 (exit `EXIT_INVALID_SPEC`); coverage L903–944 | **MISS on exit code**; (a) for the refusal; **(b)** F3 for coverage |
| verify-fab, stage 2, first set | 0, package | **2 · `InvalidSource`** at `acknowledge_unwitnessed[23].reason` (over 200 bytes); validate refused the same · 0.0 s (`out/driver.stage2-invalidsource/`) | `ir/src/board.rs` L575–585 (`source`: 1–200 printable ASCII) | **MISS; (a)**, my error; F12 |
| verify-fab, stage 2, fixed set (batch 2, spec `172a8be0…`) | 0, package (PREDICTIONS-2) | **0 · pass, package written** · 99.6 s · 2.52 GB. 16 gates, campaign and attribute included; preconditions all `met`, among them **`campaign_faults_every_bus_and_rail: met`** with no output faulted; ledger `unacknowledged: 0`. `MANIFEST.json` sha256 `048a3c4d78327a568335b5676e0e40cd2e81e4143d83207d75f9f39395f73b08` (quoted only; not committed or uploaded) | `cli/src/verify.rs` L1358 | **(b) risk** F3: a package for a driver whose outputs were never faulted |
| generate hwdef | 0 guess | 0 · written; each output a motor line (`PA8 TIM1_CH1 TIM1 PWM(1)`), and as a comment "not emitted … MOT_PWM_TYPE, MOT_PWM_MIN/MAX … wait for the defaults.parm work order" | `hwdef/src/lib.rs` L485 | not a gate (D1) |
| check pinout | 0 guess | **3** · `NoSymbolTable` ("the ELF has no .symtab") | `pincheck/src/elf.rs` L127 | **MISS**; not a gate (D1). The generated firmware is stripped (3301), so `check pinout` cannot judge it at all |

**The H-bridge at boot, measured:** both inputs on one edge-aligned timer, in phase, high for 1 µs of every 50 µs.
- **IN/IN bridge** (DRV8833, DRV8871): IN1 = IN2 = 1 is brake. The motor sees a 2 % brake chop from boot, not coast. The DRV8871 sleeps only with both inputs low, so it never sleeps.
- **PH/EN bridge** (DRV8876 in PH/EN mode): EN at 2 % drives the motor from boot.
- **Servos:** at boot they get 1000 µs, the end stop, not the 1500 µs centre.
- No gate objects to any of the three.

### L — `logger.json` (stage 1, spec `d17ecab9…`)

| Step | Predicted | Measured | Where | Class |
|---|---|---|---|---|
| validate | 0 (observed at authoring) | 0 · ok | — | — |
| solve | 0 | 0 · TEMP on I2C1 (PB6/PB7), BARO on I2C2 (PB10/PB11), LOG on SPI1, CS PA0, READOUT on USART1 | — | — |
| emit · erc · drc · support | 0 | 0 · pass · ≤1.0 s · ≤0.21 GB | — | — |
| speeds | 0 | 0 · pass, `judged: 2` (LOG SCK, MOSI) | — | — |
| extract | 0 | 0 · pass, six lines | — | — |
| generate · build | 0 | 0 · 0 | — | — |
| prove `every_valid_sequence` | 0 | **143 · OOM-killed at the cap · 1,032.0 s · 8.59 GB = `memory.max`** | the harness: unwind 77, unsplit, the same shape as `boards/sentinel-01` and `-bench` | **§7 stop**; F11 |
| prove, the other 7 | 0 | 0 ×7 · ≤15.3 s · ≤1.09 GB | — | — |
| simulate | 0, U10–U13 unmodelled | 0 · pass; four `device_unmodelled` (info): "U1x (TMP117) … has no Renode 1.17.0 model, so its identity was not read"; U20 and U30 witnessed; **boot word at 19,522,270 ns** | `twin/src/boot.rs` L565–571 | **(d)** |
| campaign | 0, all met | **1 · 5/6 met.** `droop-bor3` **fail, `brownout_unrecovered`**, against its expectation (pass) · 50.8 s. The probe at release + 100 µs read the boot word 0 (expected 0x2100B007) and two witness lines against the one expected | `chaos/src/power.rs` L87 `RECOVERY_BOUND_NS = 100_000`; probe `cli/src/renode.rs` L751, L3558–3567; witness `cli/src/analog.rs` L477–480 | **MISS; (b)** F1 (already `2303` F11 on the Sentinel-02 shape) |
| replay · attribute | 0 | 0 · 0 · on the first archived run in directory order, `1846f8d4` (`spi-absent-u30`, a failing run that met its expectation), not the unmatched droop | — | — |
| verify (no fab) | 0 | 0 · pass, 14 gates · **1,010.3 s · 13.06 GB** (760‰ of 16 GiB; host watch `out/logger.stage1/verify-mem.log`) | — | — |
| verify-fab, stage 1 (batch 2) | 2 (PREDICTIONS-2) | **2 · `UnwitnessedFieldNotAcknowledged`, 188 rows** · 2.0 s | `verify.rs` L1233–1246 | (a) |
| verify-fab, stage 2 (32 entries; batch 2) | 1 (PREDICTIONS-2) | **1 · fail; package `not_written`.** Every gate passes but `campaign` (`droop-bor3`); preconditions `campaign_faults_every_bus_and_rail: unmet [rail VDD_3V3]` and `every_gate_passed: unmet` · 988.5 s · 13.02 GB (758‰) | `verify.rs` L1358 | **(b)** F1. **Corrected by the adversarial pass:** a pass-expecting droop below the band that crosses no BOR threshold (e.g. a 3,000 mV floor) still covers `VDD_3V3` (`verify.rs` L783–796, L812–833), so L *can* meet coverage, by never testing a BOR recovery (inferred, not run) | — | (b) F1 |
| hwdef · pinout (batch 2) | 0 · 3 | **0 · written**, U10–U13 "no ArduPilot line; the parts table holds no ardupilot column", BMP581 a `BARO` line, the flash a `SPIDEV dataflash` · **3 · `NoSymbolTable`** | `hwdef/src/lib.rs`; `pincheck/src/elf.rs` L127 | not a gate |

**`TEMP` coverage, measured in the campaign.** `pullup-3k3-temp` passed and is `Tolerable([I2c(TEMP)])` (`verify.rs` L733–740). The bus counts as used because devices are declared on it (`verify.rs` L907–908), not because any is modelled. So `TEMP` is covered with no device on it witnessed. Class **(b)** for the coverage line, (d) beneath it (F4).

### The variants

| Spec | Step | Predicted | Measured | Where | Class |
|---|---|---|---|---|---|
| `driver-idle-zero` | validate (and solve, emit, speeds, generate, verify) | 2 `PulseRange` | **2 · `PulseRange`** "motor output `HB_IN1` has a pulse range of 0 to 49 us: the minimum is at least 1 and below the maximum"; erc, drc, support, extract, build, prove, simulate, hwdef, pinout `not_run` (need emit, generate or solve) | `ir/src/lib.rs` L3542–3547; `motor.rs` L262 would refuse `ccr == 0` next | **(b)** F2 |
| `logger-1s` | validate (and every spec reader) | 2 `BatteryOutsideRegulatorInput` | **2 · `DuplicateReference` `U11`.** My variant was malformed: the fixture's regulator U11 against the logger's TMP117 U11 | `ir/src/lib.rs` L3568–3574 (`references_distinct`) | **MISS; (a)**, my error |
| `logger-1s-u50` | validate (and solve, emit, speeds, generate, verify) | 2 `BatteryOutsideRegulatorInput` | **2 · `BatteryOutsideRegulatorInput`** "`U50`: the battery's 3000 to 4200 mV is outside the input's 3800 to 36000 mV (SNVSAN3F … p. 7)"; the rest `not_run` | `ir/src/power.rs` L1316–1340 | **(a) + (c)** F7 |
| `logger-i2c-only` | every step (batch 2) | pass; campaign 1; fab-out 2 | **as predicted.** All gates 0. `prove:every_valid_sequence` at 16G: **110.7 s, 4.62 GB** (unsplit, no SPI). Campaign 1, 4/5: `droop-bor3` `brownout_unrecovered`. `verify` 0 (152.8 s, 4.77 GB). `verify-fab` 2, 168 rows. hwdef 0 (TMP117s not emitted), pinout 3 | as L | (b) F1; (d) TMP117 |
| `logger-spi-only` | every step (batch 2) | pass, prove at 8G at risk | **Second stop.** `extract` **2 · `UnboundedPinCapacitance`**: U40 (ICM42688P) on `VIB_SCK`, and the parts table cites no maximum (unpredicted: my spec omitted the `pin_capacitance` both Sentinel fixtures declare from DS-000347 v1.9, C_I < 10 pF). `prove:every_valid_sequence` **`killed-900` · 417.3 s · 8.56 GB** of 8G; the other six harnesses pass. `simulate` 0, boot word at 39.03 ms (the ICM's 20 ms settle; 2303 measured 39 ms). Campaign 1, 3/5: `droop-bor3` `brownout_unrecovered`; `spi-absent-u40` failed with `spi_data_mismatch` besides the two expected kinds, so its expectation went unmet (my guess wrong). `verify` **2 · `UnboundedPinCapacitance`** at the extract gate (4.0 s). `verify-fab` 2 (acknowledgements first). hwdef **2 · `ImuOrientationUndeclared`** (predicted); pinout 3 | `eda/src/extract.rs` L1070; `hwdef/src/lib.rs` L149–151 | (a) for extract (my spec), F17 (`validate` accepts it); **stop** at the prove (F11); (b) F1 |
| `logger-spi-only-cap` (batch 3: `logger-spi-only` with U40's `pin_capacitance` declared as Sentinel-03's U5) | every step | extract 0, prove 0, verify 0 | **extract 1 · fail (MISS)**: `capacitance_over_declared_bound`, "`LOG_SCK`: bound 37508 fF (trace 15793, vias 12152, pads 1563, pins 8000) exceeds the declared 30000 fF": the flash's clock routed 48.9 mm through 8 vias; VIB's lines 23.9 and 24.3 pF pass. **prove (16G) 0**: `every_valid_sequence` **390.4 s, 10.65 GB** (650‰). simulate 0. campaign 1, 3/5 (`droop-bor3` unrecovered; `spi-absent-u40` unmet, as predicted). **verify 1** at the extract gate (462.2 s, 10.68 GB). verify-fab 2 (acknowledgements first). hwdef 2 `ImuOrientationUndeclared`, pinout 3 | `eda/src/extract.rs`; placement and routing read no `net_constraints` (only `extract.rs` and `schematic.rs` L956 do) | **(b)** F18; (b) F1 |
| `logger-offboard` | every step (batch 3; PREDICTIONS-3) | extract 1, verify 1, the rest as L | **as predicted.** **extract 1 · fail**: `TEMP_SDA` bound 173,157 fF (trace 32,329, vias 22,785, pads 3,043, pins 50,000, off board 65,000) and `TEMP_SCL` 143,273 fF, against the declared 100,000. My declared cable is too long for the bus; BARO and LOG pass. **prove (16G) 0**: `every_valid_sequence` 834.5 s, 13.00 GB (757‰). simulate 0. campaign 1, 5/6 (`droop-bor3` unrecovered); `pullup-3k3-temp` passes there because the campaign's rise time uses the declared bound, not the extracted one, and `extract` is the gate that catches it. **verify 1** at extract (887.0 s, 13.03 GB). verify-fab 2. hwdef 0, pinout 3 | `eda/src/extract.rs`; `ir/src/lib.rs` L363 (`I2cDevice`, a placed part) | **(a)** for extract; **(c)** F8: the probe's TMP117 still cannot be declared |
| `driver-tree` | every step (batch 3, after the 18:06Z ruling; PREDICTIONS-3) | as predicted but `verify-fab` | **validate … simulate 0** (prove at 8G, split per timer, ≤2.49 GB; the tree's parts drawn). **campaign 2 · `PowerNotSimulated`**; replay/attribute `not_run`. **verify 0** (75.5 s, 2.52 GB). **verify-fab 2 · `PowerNotSimulated`**, not 3 `CampaignCoverageShort` as predicted (**MISS**): `verify` resolves each scenario through `campaign::effective` (`verify.rs` L1178 → `campaign.rs` L372) before coverage (L1223), so a board with a tree never reaches coverage. hwdef 0, pinout 3 | `cli/src/campaign.rs` L372–389; `verify.rs` L1178, L1223 | (c), owner `2304`; MISS on the kind |

**Predictions §5 ("no step reaches 900‰ of its cap") was wrong.** L's schedule proof needs more than 8 GiB, and so does `logger-spi-only`'s. PREDICTIONS-2's misses: `logger-spi-only`'s `extract`/`verify` refusal (unpredicted), and its `spi-absent-u40` kinds. Its prove kill was flagged as the risk. PREDICTIONS-3's misses: `driver-tree`'s `verify-fab` kind (`PowerNotSimulated`, not `CampaignCoverageShort`), and `logger-spi-only-cap`'s `extract` and `verify` (F18). Batch 2's and 3's wall times have a 1 s floor: the watch in `sh/step.sh` polls each second.

## 4. The candidates that reached `verify`

| | M | L |
|---|---|---|
| gates | erc, drc, support, extract, prove ×8, build, simulate: all pass | the same list, all pass |
| campaign gate | none: `verify` without `--scenarios` runs none (`verify.rs` L1660) | none |
| `--fab-out` with acknowledgements (batch 2) | **pass, package written** (99.6 s, 2.52 GB; manifest `048a3c4d…`) | **fail, package not written**: campaign (`droop-bor3`), `VDD_3V3` unmet (988.5 s, 13.02 GB) |
| coverage (fab) | met **with no output faulted** | met before the gates; unmet after them (`VDD_3V3`, F1) |
| ledger rows | 294: same-source 124, ignored-by-model 125, observed 30, independent-rule 14, silicon-only 1 | 342: same-source 107, ignored-by-model 80, observed 63, independent-rule 91, silicon-only 1 |
| rows to acknowledge | 250 (41 entries) | 188 (32 entries) |
| twin limits | none | `WAIT.U20`: the BMP581's 10 ms settle, "only the bench shows the wait suffices" |
| wall / peak | 85.5 s / 2.51 GB | 1,010.3 s / 13.06 GB |
| per gate | `out/driver.stage1/rows.tsv`, `out/driver.stage2/rows.tsv` | `out/logger.stage1/rows.tsv`, `out/logger.stage2/rows.tsv` |

`verify`'s own JSON gives no per-gate wall time or RSS; the standalone rows are the per-gate figures. Neither board reached `--fab-out` with its acknowledgements.

## 5. The §6 board-name check (§1.5)

**The work order's pathspec is vacuous.** `git grep -niI sentinel -- 'crates/*/src' 'bridge/src' models Cargo.toml 'crates/*/Cargo.toml'` returns **2 hits**, both manifests. Git's `crates/*/src` matches no file under `src/`. With `'crates/*/src/**'` it returns **62**. Roadmap §6's "Measured 2026-11-15: outside tests and comments, no crate names Sentinel" may rest on the same pathspec; that is a steering-document question for the architect (F13).

The 62 hits, classified by a read-only subagent (`~/.cache/aet-4600/research/boardname.md`). I spot-checked nine T hits (each after its file's first test module) and `firmware.rs:140`.

| Class | Count | Hits |
|---|---|---|
| test module or test file | 35 | `cli/src/campaign.rs:1722`; `fab.rs:882…1173` (10); `main.rs:2836,2838`; `speeds.rs:216`; `verify.rs:1975–1980,2030,2165–2167,2243,2244,2250` (13); `eda/src/stack.rs:511` (doc of a `#[cfg(test)]` fn); `ir/src/orientation.rs:100`; `parts.rs:1638`; `solve.rs:2162`; `pincheck/src/identity.rs:1031`; `svd-hal/src/board.rs:6295`; `cmsis.rs:748`; `twin/src/tests.rs:8` |
| comment | 6 | `eda/src/pcb.rs:672`, `:897`; `eda/src/route.rs:137`; `ir/src/connectors.rs:66`; `ir/src/ds009.rs:12`; **`svd-hal/src/board/firmware.rs:140`** (a doc example, `sentinel-01-board` → `sentinel_01_board`; not in the architect's list) |
| string in non-test code | **1** | `cli/src/ledger.rs:388`, `GATE_SPI`: an evidence citation naming `boards/sentinel-01/sentinel-01.repl`. It decides nothing. It is the one place the §6 sentence "outside tests and comments" is not true |
| manifest | 2 | `Cargo.toml:56` (`sentinel-01-board`); `crates/aethelgard-svd-hal/Cargo.toml:25` (a dev-dependency) |
| plain sense | 18 | `cli/src/renode.rs` 16 (log-line markers, before its test module at L3609); `cli/tests/fixtures/forge-log/src/main.rs:6,65` (a test fixture the `src/**` glob reaches) |

**Board-shaped literals in non-test code (21 hits; none decides anything board-specific):**
- `chaos/src/synthetic.rs:26`: `NETS = ["VDD_3V3","VBAT","/SDA","Net-(U1-Pad3)"]`, labels for seeded synthetic events (L99);
- `ir/src/ds009.rs:70`: DS-009's pin-role word, checked by domain voltage, not by name (L229);
- `ir/src/power.rs:82`: `BATTERY_NET = "VBAT"`, the one literal in logic. It is a reserved rail name for any board with a tree, so it is generic;
- the rest are doc examples (`bridge/src/request.rs:8`, `ir/src/classes.rs:8–9`, `ir/src/power.rs` module doc) and the MCU's own FLASH peripheral (`cli/src/ledger.rs:617`, `:1559`, `renode/foreign.rs:425`, `svd-hal/src/board.rs:2178`, `svd-hal/src/board/ledger.rs:592–594`, `twin/src/boot.rs:349`). `SENSORS`, `EXT`, `U2`–`U4`, `FLASH_CS` and `CONSOLE_TX` appear only in tests.

**Part names in code** (a separate question from board names, for D4):
- `eda/src/schematic.rs`: I2C arms BME280 L1327, BMP581 L1353, TMP117 L1373, refusal `UnknownPart` L1388–1393; SPI arms W25Q64JVxIQ L1434, ICM20948 L1455, ICM42688P L1481, refusal L1499–1505; the MCU L1194; connectors L470–492;
- `ir/src/names.rs` L32 (`VENDORED_PARTS`) and L38–45 (`device_nets`);
- `cli/src/ledger.rs` L1328–1353 (`MODEL_SETS`);
- `twin/src/model.rs` L142–151 (by Renode model name).
- **ICP10111 has a parts row and no emitter arm** (read at source: absent from `schematic.rs` and `VENDORED_PARTS`). A specification naming it validates, then `emit` refuses `UnknownPart`. Not run.

## 6. What `4400g` moves (§1.7)

`4400g` (#238) had not merged at the stop (head `46a44a31`, OPEN). Nothing was re-measured. When it merges, the follow-ups re-check:
- M's PWM witness. The twin's HB period is 12,544 ns and the servo's 5,119,744 ns, ARR × 256 ns: a timer clock that is not the silicon's, and ARR rather than ARR + 1 (architecture L396).
- M's `campaign` (trace 13 → 14).
- One `replay`.
- Every archive hash above.
- A spike specification bumped to 17 validates.

1315 had not merged either; `driver-tree` was not run.

## 7. Findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F1 | **A BOR droop's recovery is judged at a fixed instant, against a one-line witness.** Not new: `2303` F11 found it on the Sentinel-02 shape (roadmap L803, owner: the author of `4500`'s droop scenario). New here is that it is general: any board whose first scripted device waits before the boot word is written (`svd-hal/src/board/firmware.rs` L819–901) fails every BOR droop as `brownout_unrecovered`. Two parts: (i) the probe sits at release + 100 µs (`RECOVERY_BOUND_NS`, "Measured: 20 µs" on Sentinel-01), and L's boot word lands at 19.52 ms; (ii) the probe's witness takes every line of the first device since the release and demands exactly the identity step's one line (`cli/src/analog.rs` L477–480), so a later probe would still fail (two lines then, three after the data read). The twin also runs the settle about 1.95× long (19.52 ms for 10 ms), so a bound in declared µs is in the wrong time base. Coverage does not catch it: a droop that crosses no BOR threshold covers the rail (b) | `chaos/src/power.rs` L87; `cli/src/renode.rs` L751, L3558–3567; `cli/src/analog.rs` L477–480; `verify.rs` L783–796 | risk | judge recovery as the bring-up completing (the boot word, then the first identity line among the witness lines) within a bound the twin derives from the boot it ran, not a constant; and require a BOR-crossing droop for the MCU rail's coverage when `bor_level` > 0 | yes: fold into `2303` F11's owner, or a twin/chaos WO before `4500` |
| F2 | **A PWM output's idle is a servo or ESC's minimum pulse.** M got a fabrication package with it (batch 2). The firmware starts every PWM output at `pulse_min_us`, and `validate` refuses 0. An H-bridge cannot idle at coast. Two IN/IN inputs on one timer chop brake at 2 % from boot. A PH/EN bridge drives at 2 %. A servo idles at its end stop, not its centre. Every gate passes it (b) | `ir/src/motor.rs` L66–70, L259, L262; `ir/src/lib.rs` L3542–3547 | risk | a declared idle per output, separate from the range (an `idle` of low, high, or a pulse in µs, with 0 allowed), and for a bridge a declared pair, so the firmware never drives both inputs of one bridge | yes: a specification field (D7) |
| F3 | **Coverage has no motor-output target and no fault kind reaches one.** M met coverage with none of its four outputs faulted, the outputs being the board's purpose, and its package's verdict says `campaign_faults_every_bus_and_rail: met` (b) | `cli/src/verify.rs` L903–944; `chaos/src/record.rs` L173–191 | risk | coverage lists each motor output, `unmet (no fault kind reaches it)` as rails do today, until a kind exists (an output held, an output open, a load absent) | yes: whether "unmet by kind" blocks a package for a driver (it does for rails) |
| F4 | **A bus is covered when only unmodelled devices sit on it.** L's `TEMP` (four TMP117s, no model) is covered by a tolerated pull-up fault judged by ngspice, while nothing on it answers in the twin (b) over (d) | `cli/src/verify.rs` L907–908 | risk | coverage counts a bus "used" by its modelled devices, or reports `covered (electrical only: no device witnessed)` beside it | yes |
| F5 | **Gates that judged nothing pass like gates that judged something.** On M: `extract` `lines: []`, `speeds` `judged: 0`, the droop observation with no device witness | `cli/src/extract.rs` L187; `cli/src/speeds.rs` L16–17; `chaos/src/campaign.rs` L1215–1220 | note | a verdict that judged nothing says so (`pass, nothing in scope`), so a board's gate list shows what was judged | no |
| F6 | **No device without a bus; no GPIO; no ADC input.** A driver IC, its sleep and fault pins, and its current sense cannot be declared (c) | `ir/src/lib.rs` L102–152, L337–388 | scope-question | §2's rows; a device kind without a bus and declared GPIO and ADC signals | yes: D2, D7 |
| F7 | **The power input is an XT30 and a series pack, or nothing.** No 3.3 V regulator from 1S, no buck-boost kind, no USB or wall input (c) | `ir/src/connectors.rs` L165; `power.rs` L200–213, `Kind` | scope-question | §2 | yes: D7 |
| F8 | **No device off the board.** A probe on a cable exists only as copper capacitance; the engine never knows the sensor is there (c) | `ir/src/lib.rs` L363 (`I2cDevice`) | scope-question | an `offboard` placement on a device (its connector and cable), which `extract` already half-models for lines | yes |
| F9 | **Serial roles are the flight ecosystem's plus `console`.** `console` serves a command port and a readout; there is no role for a non-console protocol (c) | `ir/src/serial.rs` L31–37 | note | none now | no |
| F10 | **PWM ticks in whole microseconds at any rate.** 50 steps at 20 kHz (c) | `ir/src/motor.rs` L253–259 | note | with F2: a declared resolution, PSC derived from it | with F2 |
| F11 | **The unsplit schedule proof's cost is SPI's, and the split is keyed on timers.** The schedule is split into per-peripheral parts only when the board has a timer (`plan.timers.is_empty()`, `svd-hal/src/board.rs` L3897–3909), so every board without motor outputs, Sentinel-01 included (`boards/sentinel-01/src/proofs.rs` L407), proves it whole. Measured: I2C only (`logger-i2c-only`) 110.7 s, 4.62 GB. Two SPI buses, no I2C (`logger-spi-only`): past 900‰ of 8G at 417 s (killed). Two SPI buses at 16G (`logger-spi-only-cap`): 390.4 s, 10.65 GB. I2C + SPI (L): OOM-killed at 8G (1,032 s), passed inside `verify` at 16G (13.06 GB scope peak, page cache included: an upper bound). M's timer-split parts: ≤2.49 GB. 4201 F4 measured the OOM at 8 GiB on Sentinel-01 | `svd-hal/src/board.rs` L3897–3909 | risk | a work order's `prove` cap keys on "no timer and an SPI bus" (16G), not on an I2C bus; a follow-up lifts the `timers.is_empty()` gate, since `parts()` already makes one part per I2C/SPI peripheral | yes: the cap is the architect's |
| F12 | **Acknowledgements dominate a specification and are learned by refusal.** M: 41 entries, 75 % of its bytes. The refusal names fields but not classes (the ledger in a second run does); the 200-byte ASCII reason rule shows up only as `InvalidSource`. The easy path, copying the ledger's own `judge` text, makes an acknowledgement say nothing the artifact did not (northstar §6, "so hard to write that people hand-edit outputs") | `cli/src/verify.rs` L1233–1246; `cli/src/ledger.rs` L1676–1693 (fields only); `ir/src/board.rs` L575–585 | risk | the refusal prints `register · class · fields` per entry with an empty reason, which a person must fill (the `InvalidSource` message does already state the 1–200 byte rule; it is met only after the reasons are written) | yes |
| F13 | **The §6 board-name pathspec is vacuous** and roadmap §6's measured sentence may rest on it; one non-test string names Sentinel (`ledger.rs:388`) | roadmap §6 (protected); `cli/src/ledger.rs:388` | note | D5 | yes (D5) |
| F14 | **`check pinout` cannot judge the generated firmware** (`NoSymbolTable`: the ELF is stripped), and `generate hwdef` writes each H-bridge input and servo as an ArduPilot motor line (`PA8 TIM1_CH1 TIM1 PWM(1)`), with their ranges as comments (`not_emitted`: `MOT_PWM_TYPE`, `MOT_PWM_MIN/MAX`) | `pincheck/src/elf.rs` L127; `hwdef/src/lib.rs` L485 | note | D1 excludes both by name for a board with no ArduPilot | D1 |
| F15 | **Every part costs code** (arms in four files) and **ICP10111 has a row and no arm** (`validate` passes, `emit` refuses) | §5 | note | D4 | D4 |
| F18 | **Placement and routing do not read the declared line-capacitance bound, so the engine can lay out a board that fails its own `extract`.** With a second SPI bus, `LOG_SCK` was routed 48.9 mm through 8 vias: 37.5 pF against the 30 pF its specification declares (Sentinel-01's figure, copied). The author has no lever but loosening the bound. On Sentinel's one-SPI shape the flash sits close enough that it never showed (b) | `eda/src/pcb.rs`, `eda/src/route.rs` (no `net_constraints`); `eda/src/extract.rs` | risk | the placer and router take each declared `max_capacitance_femtofarads` as a constraint (a length and via budget derived from it), or refuse by name when they cannot meet it, before `extract` judges | yes: an eda WO; whether before the P4 baseline (no specification change) |
| F17 | **A missing pin-capacitance declaration is refused at `extract`, not at `validate`.** `logger-spi-only` validated, solved, emitted, generated and built, then `extract` and `verify` refused `UnboundedPinCapacitance` for an ICM42688P whose row cites no maximum. `validate` has everything it needs to refuse it: the part's row, the bus and the empty declaration (a) | `eda/src/extract.rs` L1070; `ir/src/lib.rs` `SpiDevice.pin_capacitance` | note | `validate` refuses a device on a driven SPI or I2C line whose row cites no maximum and whose specification declares none (`INV-06`, earlier) | no |
| F16 | `logger-1s` refused `DuplicateReference`: a power tree's references share the board's namespace, so copying a tree block onto a board collides (a) | `ir/src/lib.rs` L3568–3574 | note | none: a correct refusal; worth a line in the authoring guide | no |

## 8. What did not work, and why

- **The first cap stop.** The work order gives `prove` 8 GiB, and L's schedule proof needs more (F11). The stop froze the rest until the ruling.
- **The second stop.** The ruling keyed 16G on an I2C bus. The cost is the SPI part of the unsplit schedule (F11), so `logger-spi-only` (no I2C) passed 900‰ of 8G. `logger-offboard` and `driver-tree` did not run in batch 2; they ran in batch 3 after the 18:06Z ruling.
- **`logger-spi-only` as written** left out U40's `pin_capacitance`, so `extract` and `verify` refused it. The labelled `logger-spi-only-cap` (batch 3) declares it, and then fails `extract` on the flash's clock line instead (F18).
- **My first stage-2 run for M** was refused `InvalidSource`: a reason over 200 bytes. It is kept in `out/driver.stage2-invalidsource/`. A second, fixed set was written and frozen before it ran.
- **A run I started by mistake on the un-acknowledged file** was killed while it waited on the lock: no row, no container.
- **The droop failure** was first read as 4201's BMP581 fault, then reported to the architect as new. It is `2303` F11 (roadmap L803), seen here on a non-flight board, plus the witness rule the adversarial pass found (F1).
- **`replay` and `attribute`** ran on the first archived run in directory order, not on the failing one.

## 9. Deliberately not built

- No crate, model, parts table, golden, test, example, scenario, board, workflow or `ci/` file changed. The fence diff check is in the PR.
- No parts row was added; §2 names them.
- No cap was raised, and no run was retried at a larger cap.
- No package was written, so none was ordered or uploaded.
- Every specification written was run. No variant beyond `logger-1s-u50` and `logger-spi-only-cap` (each labelled, each a correction of my own malformed variant) was added.

## 9a. Gates, adversarial pass, reduction pass

- **Fence:** `git diff --stat cd43b207 -- crates bridge models examples scenarios boards fab ci .github container Cargo.toml Cargo.lock` is empty.
- **`cargo fmt --all -- --check`:** passes.
- **Clippy and `cargo test --workspace`:** not run. No Rust file changed, so both are `main`'s own (CLAUDE.md: "for a spike-only branch, run what applies and say so"). No test reads `spike/` or `docs/work-orders/`: `workspace_shape.rs`'s `files()` walks named crate directories and `boards/` only (L72–88, L492, L648), and the two other tests that mention `spike/` cite it in doc comments.
- **`PREDICTIONS.md`** has exactly one commit, `e57f7cb8`.
- **Adversarial pass:** a fresh subagent on Opus tried to refute F1–F4, F11, F12 and §3's tables, from source and from `spike/4600/out/` (report `~/.cache/aet-4600/research/adversarial.md`). I checked each correction at source before adopting it.
  - **F2, F3, F4: confirmed.**
  - **F1: weakened, and two sentences refuted.**
    - It is `2303` F11, already on roadmap L803.
    - Coverage *can* be met by a droop that crosses no BOR threshold.
    - It added the witness rule (`analog.rs` L477–480) and the twin's time base.
  - **F11: weakened.** The split depends on a timer, not on a bus.
  - **F12: the recommendation was half moot.**
  - **F14: misstated.** hwdef writes `PWM(n)` lines, not parameters.
  - **Two §3 replay rows named the wrong run.**
  - All of these are corrected above.
- Reduction pass (diff): −1 net · 1 duplicate removed (`sh/scenarios.py` restated Sentinel-01-bench's droop; it now reads it from `scenarios/sentinel-01-bench/droop-bor3.json`, scenarios regenerated byte-identical) · 1 dead path removed (`sh/step.sh`'s unused `-stdin`; host smoke run passes) · 0 protected kept · 1 finding: `sh/acks.py`'s I2C and SPI reasons are exercised only by L's stage 2, which the stop froze.

## 10. Decisions

**D1 · What "every gate" means for the clause.**
- **Recommend:** `verify --scenarios … --fab-out` writes a package, and `check speeds` passes. `generate hwdef` and `check pinout` are excluded by name for a board with no ArduPilot.
- **Why the package and not a passing `verify`:** `verify` without `--scenarios` runs no campaign (§4), so it is not `INV-03`. The package is where coverage and acknowledgements bite (F1, F3, F12).
- **Why exclude hwdef and pinout:** pinout cannot read the generated ELF at all (F14), and hwdef's output is ArduPilot's.
- **Add:** the gate's board must show, per gate, what was judged (F5), or a vacuous pass counts as a gate.
- Any wording change is Scotty's.

**D2 · Which board.**
- **Recommend: the logger.**
  - It exercises more of the engine today: two I2C buses, SPI, the twin's witnesses, an unmodelled part, and the BOR path.
  - It needs first: F1, which is both the recovery instant and the witness rule (the latter is more than a constant), F4, and a 1S supply if it is to be a field logger (§2: a buck-boost kind and a non-XT30 input).
- **The driver** passes every gate today, and `verify --fab-out` wrote its package (batch 2), with little judged beyond its outputs' timing (computed by `check_silicon` and witnessed as edges): no output is faulted (F3) and two gates judge nothing (F5). Making it honest needs F2, F3, F6 and a driver-IC row with an emitter arm: more specification surface than the logger.
- Either would serve. The driver is the one a reviewer can most easily fit to the engine.

**D3 · The (b) findings.**
- **Recommend:** group by crate, each written in general terms:
  - **chaos/twin:** F1, the recovery bound from the declared boot path, and F4;
  - **ir/svd-hal:** F2 and F10, a PWM output whose idle is 0 % or a declared pulse, and a bridge pair never driven high together;
  - **cli:** F3, coverage of outputs, and F5, gates that judged nothing.
  - **eda:** F18, placement and routing driven by the declared capacitance bounds.
- **Order:**
  - F1 joins `2303` F11 (owner: the author of `4500`'s droop scenario), with the witness rule added. It is needed before `4500`'s droop can pass, since the Sentinel-02 shape boots in 39 ms.
  - F2 and F3 are independent of `4500` (Sentinel-02's ESCs idle at the minimum by design). Put them after it, before the gate.
  - F11's split of the schedule harness is CI-relevant; offer it to `1911`.

**D4 · Is a parts row an engine change?**
- **Recommend:** the clause allows a cited data row added at the gate. Today a new device part costs four files of code (§5), so the clause as written would be refused by its own test.
- **Recommend:** the emitter's per-part arms become data (a pin map, straps and passives per part in the row), so that a part enters through a row alone, the parts-table version of §6's rule. ICP10111 (F15) shows the two drifting apart already.

**D5 · The board-name hits.**
- `ledger.rs:388`: keep the citation but move the board path into a doc comment or a test; the evidence is about the platform emitter, not the board.
- The two manifests: keep. They are the workspace member and a trybuild dev-dependency.
- The comments: keep; they cite where a value was measured.
- **Recommend a tier-1 test** beside `workspace_shape.rs` that greps `crates/*/src/**`, `bridge/src/**` and `models/**`, skipping each file's `#[cfg(test)]` region, and fails on a board name in non-test code. Its pathspec is pinned by a test that plants a hit and expects a failure (F13: the obvious pathspec is vacuous).
- **Scotty's:** roadmap §6's measured sentence is re-measured or reworded.

**D6 · Keeping the clause honest.**
- **Recommend all three of the architect's points**, plus one:
  - the gate's board is written by a fresh session from a one-paragraph brief, with no access to `spike/4600/`;
  - `spike/4600/` is deleted when its follow-ups merge;
  - the gate's independent review is told to check that the board was not fitted to the engine. It compares its specification with what a datasheet-led design would declare (§2's list is that baseline).
- **The addition:** the brief names the board's purpose and environment (a field logger on a cell, a driver for a real bridge part), never its buses.

**D7 · Order against the schema epoch.**
- These (c) and (b) findings add specification fields: F2 (an output's idle, a bridge pair), F6 (a device without a bus, GPIO, ADC), F7 (a power input kind, a buck-boost), F8 (an off-board device), and the LSE if a logger keeps time.
- **Recommend:** F2 and F6 land **before** the P4 baseline. They change what a motor output and a device are, which later additive fields cannot undo cleanly.
- F7 and F8 are additive and can follow the baseline.
- F1, F3, F4, F5 and F11 need no specification field.
