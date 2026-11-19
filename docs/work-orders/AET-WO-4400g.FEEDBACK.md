# AET-WO-4400g — FEEDBACK (build)

Branch `wo/aet-4400g-twin-clocks-build`, from `main` at `964e2b23`. The work order is `docs/work-orders/AET-WO-4400g.md` (copy at `~/.cache/aet-4400g-wo.md`). It builds the ruling on #229 (comment 6996346321).

Every answer is labelled **measured**, **read at source** or **inferred**. Measurements ran on Furnace in the pinned tools image (`container/IMAGE`, Renode `1.17.0+20260907gitf1dd1b4af.aet1605`), each container under `~/bin/capped-run` (8 GiB, swap 0, read back). The kit is in `~/.cache/aet-4400g/kit`, outside the branch.

## §1 What was built

### §1.1 (a): the generated firmware at its solved clock

- **The emitter** (`aethelgard-twin` `platform.rs`, `emit_clock`, `clocked`, `timer_hz`) ends every platform with an update entry, `name:\n    frequency: F`, for every instance of the stock file that a clock sets:
  - TIM1–TIM17, the 14 the stock file carries, at `SolvedClock::timer_kernel_hz`;
  - USART1–UART8 at `serial::kernel_hz`;
  - LPUART1 and LPTIM2–5 at `rcc_pclk4`, and LPTIM1 at `rcc_pclk1`, their reset kernel clocks (RM0433 Rev 9 §8.7.21–§8.7.22, read at source);
  - the DWT at `core_hz`, `sys_ck / D1CPRE`.

  On the reset clock every entry is 64 MHz. A tier-1 test parses the vendored `stm32h7.repl` and holds the set to every instance with a `frequency:` line. The independent watchdog is the one exception: it counts LSI, outside the solved clock tree, and is held apart by name.
- **`TIMER_HZ` and `CYCLE_NS` are gone.**
  - Each observer carries `kernel_hz`, from the same `timer_hz`.
  - Its period and high time are `aethelgard_chaos::record::realized_ns(ticks, F)` = ⌈ticks × 10⁹ / F⌉ ns.
  - The model's convention is stated in `timer.rs` beside `ARR` against `ARR + 1`. The silicon check (`check_silicon`, kind 25) still judges the exact period.
  - DShot600 at 64 MHz: 106 ticks realize as 1,657 ns, and its high time is 625 ns. PWM at 400 Hz: 2,499,000 ns.
- **Every user of `TIMER_HZ` moved:** `timer.rs` and its unit test; `renode.rs`'s mismatch text, which now names the observer's `kernel_hz` and "realized in whole ns rounded up"; `timer_routes.rs`, whose stock check is now a local `STOCK_TIMER_HZ`; `motor_platform.rs`; and `aethelgard-evolve`'s and `aethelgard-chaos`'s motor fixtures, moved to the 64 MHz figures.
- **(a) never reaches `--foreign`.** `ready()` refuses `--foreign` without a declaration first, before any input is read: `ForeignClockUndeclared`, exit 3.

### §1.2 (c): a foreign firmware's declared clock

- **The declaration is `mcu.foreign_clock`,** in specification schema 17 (`aethelgard-ir`, `motor.rs` `ForeignClock`). Its shape, as proposed:
  ```json
  "foreign_clock": {
    "apb1_timers_hz": {"value": 200000000, "source": "…"},
    "apb2_timers_hz": {"value": 200000000, "source": "…"},
    "usarts": {"USART1": {"value": 72000000, "source": "…"}, …, "UART8": {…}}
  }
  ```
  - It is optional by the PR #191 precedent: absent means none, and an explicit `null` is refused.
  - `validate` step 2c requires each `source` to be 1–200 bytes of printable ASCII, no zero value, exactly the eight USARTs, and the USARTs one mux selects declared at one clock (`USART16SEL`, `USART234578SEL`, §8.7.21): `ForeignClockZero`, `ForeignClockUsarts`, `ForeignClockMuxSplit`.
  - `solve` copies the values into `SolvedTopology.foreign_clock` (solved schema 11, `null` when none), and the topology's `validate` holds them to the same rules.
- **Each clock serves one run kind.** `ready()`, before anything else:
  - `--foreign` with no declaration: `ForeignClockUndeclared`;
  - a generated run on a declaring specification: `ForeignClockOnGeneratedRun`;
  - both exit 3.
- **The platform** under a declaration builds the timers per APB bus and the USARTs at the declared figures. LPUART1, LPTIM1–5 and the DWT keep the stock file's figures, because no declaration sets them; the verdict names them (`boot.clock.stock`).
- **The timer cross-check** (`renode/foreign.rs` `ClockReport::check`, `aethelgard-ir` `SolvedClock::from_rcc`):
  - At the run's end the judge reads back `RCC_CR`, `RCC_CFGR`, `RCC_D1CFGR`, `RCC_D2CFGR`, `RCC_PLLCKSELR` and `RCC_PLL1DIVR`. The WO names five registers; `RCC_CR` is the sixth, for `HSIDIV` and the ready flags (D3).
  - It decodes them by RM0433 Rev 9, each field cited to its section and page, and holds each APB bus's declaration to the decoded timer kernel clock (Table 57 with `TIMPRE`).
  - A contradiction fails `foreign_clock_contradicted` with both figures; registers that do not decode fail `foreign_clock_undecodable`.
  - The exact-case test uses ArduPilot's registers, **measured** read back (§2.1): `0x3F003025`, `0x801B`, `0x48`, `0x440`, `0x408040`, `0x01090231` decode to 400 MHz `sys_ck` and 200 MHz timers.
- **The USART is declared and unchecked, and the verdict says so.**
  - `boot.clock.declared_only` lists each USART.
  - `boot.clock.conditions` names `usart_clock_declared_unchecked` (Renode drops `RCC_D2CCIP2R`), `pll1_fraction_not_read` (Renode tags `PLL1FRACEN` and `FRACN1`) and `stock_frequency_undeclared`.
  - No judgement is drawn from Renode's unhandled-write line.
- **`sentinel-02-own.json` declares ArduPilot's clock,** as the architect accepted (bus 03:22): timers at 200 MHz and USARTs at 72 MHz, each sourced to Copter-4.7.2's `stm32h7_mcuconf.h`.
  - Workflows only `solve` that fixture and generate its hwdef. The hwdef is unchanged except its header's specification sha256.

### §1.3 Replay carries the clock (F6)

- **Kind numbering, corrected (accepted on the bus, 03:22).** The WO's "kind 25 (`MotorOutputSolved`)" followed my own 4400f text, which was wrong.
  - In `record.rs`, kind 24 is `MotorOutputSolved`, and kind 25 is `MotorTimingCheck`, which already carries `kernel_hz`, the solved timer kernel clock and so (a)'s F.
  - No field was added to kind 24, so the clock is stated once.
- **One function, one home:** `aethelgard_chaos::record::realized_ns`.
  - The twin calls it; the twin's first dependency on `aethelgard-chaos` is the ruled new edge. The layering allows it: `aethelgard-chaos` depends on no member, and `docs/architecture.md`'s crate table forbids nothing here. `workspace_shape.rs` now names the edge.
  - Replay's `witness_consistent` checks kind 26's `period_ns` and `high_ns` against it at kind 25's `kernel_hz`.
  - `campaign.rs`'s `TWIN_CYCLE_NS` copy is deleted, and no twin constant is left in `aethelgard-chaos`.
- **`TRACE_VERSION` 13 → 14.** Every other version is refused by name, 13 included (`an_unknown_version_is_refused`). The golden trace moved in its version byte only: set back to 13, its bytes hash to the version-13 golden.

### §1.4 The foreign window: 5 s

- `FOREIGN_MAX_VIRTUAL_NS` is 5 s.
- `ci/foreign-judge.sh` runs at 5000, 4300 and 4370 ms, and gains three cases on the real firmware: `clock-undeclared`, `clock-on-generated-run`, and `clock-contradicted` (timers declared at 250 MHz).
- F1: §2.3.

### §1.5 The pinned build: recorded and refused

- **The constant.** `aethelgard_twin::platform::RENODE_BUILD` is typed once in Rust. A tier-1 test holds it equal to `container/smoke.sh`'s `build:` line, which is the image's pin (D5).
- **Recorded:** every `simulate` verdict carries `tool.build`.
- **Refused:** every `simulate`, a plain load included (D4), refuses any other build, or none named, as `RenodeBuildNotPinned`, exit 4. `doctor` holds Renode to the same pin.
- **The tests moved:**
  - `2205`'s any-build test now asserts the refusal for another build, the stock 1.17.0's, none, and a blank.
  - Every test stub prints the constant: `boot_tier1`'s four, `campaign_tier1`'s `BUILD`, `doctor_tier1`, `tools_tier1` and `twin_tier2`'s `PINNED_RENODE_BUILD`.

### §1.6 `INV-07` residues

- **(a)(2), the RCC push on a clock write.** `validate`'s `UnsupportedClock` message now states this reason, and the test `a_clock_programming_firmware_is_refused_because_renode_s_rcc_push_reaches_counters_not_channels` holds it.
- **(a)(3), DWT.** The platform sets the DWT by update entry: 64 MHz under (a). Under (c) no declaration sets it, so it is a stated condition, and `no_judge_reads_the_dwt` asserts that neither judge's commands name the DWT or its address block.
- **(a)(4), USART pacing.** Recorded in `emit_clock`'s documentation: upstream `88afd12bb` paces received characters at `BaudRate`, so from the next Renode a wrong F changes what the firmware receives.
- **SysTick and the watchdog,** found by the adversarial pass and named in `clocked`'s documentation:
  - SysTick follows the RCC: its constructor pushes HSI's 64 MHz, and a firmware's RCC writes push its own clock (4400f F4, measured).
  - The watchdog counts LSI (stock 56 kHz; silicon's LSI is 32 kHz, §8.5.2). No judge reads it.

### §1.7 Moved figures, each re-pinned from a run

To be completed with the measured runs: §2.

## §2 Measured

_(in progress)_

## §3 Decisions and deviations

- **D1, kind 25:** above, accepted.
- **D2, the declaration's names:** `mcu.foreign_clock.{apb1_timers_hz, apb2_timers_hz, usarts.<PERIPHERAL>}`, as the WO's shape.
- **D3, a sixth register, `RCC_CR`.** It holds `HSIDIV` (§8.7.2), which divides HSI before it clocks the system or PLL1, and the ready flags. The adversarial pass showed that Renode's `SWS` mirrors `SW` whether or not the source runs, so the decode also requires the selected source's ready flag (`HSIRDY`, `CSIRDY`, `HSERDY`, `PLL1RDY`).
- **D4, the build pin covers a plain load too.** §1.5 says `simulate` refuses. Before this branch, neither the version nor the build was held on a load. Now both are, and doctor's Renode blocks every `simulate`. `RENODE_BOOT` is gone, since it became the same list as `RENODE`.
- **D5, "read from the image's pin, never typed in twice."** The pin lives in `container/smoke.sh`, outside the fence. An `include_str!` across crates breaks the package gate (the 4400d lesson). So the build is typed once in code and tied to `smoke.sh` by a test. That is one copy in code and one in the image, held equal.
- **D6, decode refusals beyond the WO,** each found by the adversarial pass or at source, and each failing closed:
  - `D1CPRE` and `HPRE` `11xx`: Renode divides by 2^(n+1), RM0433 by 64 to 512;
  - `DIVM1` at its reset value 32: Renode's PLL1 prescaler stays 1 until the field changes (`STM32H7_RCC.cs` L78, L276);
  - a reference outside 1–16 MHz or a VCO outside 150–960 MHz (§8.7.13; `RCC_PLLCFGR` is not read, so the wider of the two VCO ranges holds);
  - a source not ready.

## §4 Findings

| # | Finding | For |
|---|---|---|
| F1 | `.github/workflows/ci.yml` L69 calls its crate list "also the order docs/releasing.md publishes in", and lists `aethelgard-twin` before `aethelgard-chaos`. `docs/releasing.md` L35–36 does the same. The twin now depends on chaos, so chaos must publish first. `cargo package` in one invocation is unaffected, but a publish in that order would fail. Both files are outside this fence. | architect: a docs/workflow edit |
| F2 | The architecture §2.27 figures move: ArduPilot at 1.00000×, the window at 5 s, and the witness times (U4 0.1611 s; the barometers 0.3856/0.4359 s; the ICM 4.343/4.394 s). | the docs PR |
| F3 | A foreign firmware clocked from HSE can never decode: `CrystalNotSupported` holds `mcu.hse_hz` at 0, so the run fails `foreign_clock_undecodable`, which fails closed. | backlog, with the crystal part |
| F4 | `fab/sentinel-01-bench/package/MANIFEST.json` moves (the specification's sha256, and `tool.build` in the verdict). It is derived by the architect's dispatch of `ci.yml` with `write=true` (bus, 03:10). | before READY |

## §5 Adversarial pass (full)

_(in progress)_

## §6 Gates

_(in progress)_
