# AET-WO-4240 measurements (scratch knob, never committed)

Measured 2026-11-19 by the measuring hand. Source: `git archive cd43b207` of `~/aethelgard-4240`, exported twice:
`/home/forge/.cache/aet-4240/pristine` (untouched) and `/home/forge/.cache/aet-4240/scratch` (the knob). The real worktree and every
other repository were not touched. Every figure here is **[measured]** unless it is labelled **[inferred]**.

## 0. The knob (a labelled deviation): `knob.patch` = `diff -ruN pristine scratch` (6 files, 364 lines)

- One `option_env!("AET4240_TREE")` site: `crates/aethelgard-ir/src/knob4240.rs`, a `const fn` parser (a malformed value is a
  compile error). Unset or `HSI` gives main's constants and `ACTIVE = false`; every edited reader branches on `ACTIVE` or reads a
  constant that equals main's when unset. A second `option_env!("AET4240_RESERVE")` (`PH0PH1` / `PH0`) serves M6 only.
- `SolvedClock::reset()` returns the knob's clock when active. That one edit routes `solve` (solve.rs:293), validate step 21
  (lib.rs:2728), the old-schema serde default (lib.rs:984) and lifts `UnsupportedClock` (lib.rs:4022). `CrystalNotSupported` was
  not touched: no fixture declares `mcu.hse_hz` (grep), and the solved `hse_hz` is taken from the knob.
- No new field on `SolvedClock` (it is `deny_unknown_fields`). `source` is `HSE` (H0) or `PLL1` (H1, H2, HSIPLL).
- **Choices** (stated, not derived from a ruling):
  - H0: every kernel = `hse_hz`. SPI1–3 `per_ck` = `hse_ck` (CKPERSEL 10), SPI4–6 `hse_ck` (101).
  - H1: SPI1–3 on `pll1_q` = 64 MHz. SPI4–6 on their APB clock (code 000), 64 MHz.
  - H2: `D1PPRE` = `D3PPRE` = 2 as well, as ArduPilot's `stm32h7_mcuconf.h` lines 291/294 set them, so I2C4's pclk4 is 100 MHz.
    SPI1–3 `pll1_q` = **80 MHz**: mcuconf 8 MHz HSE, DIVM 1 (l.128), DIVN 100, DIVQ 10 (l.194–195), VCO 800 MHz / 10; at
    25 MHz DIVM 2 (l.152), DIVN 64, DIVQ 10 (l.240–242). This equals the parts table's `ardupilot.spi_kernel_hz` 80 MHz. SPI4–6 = pclk2/pclk4 100 MHz, which is also
    ArduPilot's `pll2_q` (8 MHz ×75 /6). USART and I2C stay on their reset selects (pclk = 100 MHz), as the brief says, not
    ArduPilot's `pll2_q`/`pll3_r`. Timers: `TIMPRE` 0 with ppre 2 gives 200 MHz.
  - HSIPLL: every kernel = `sys_hz` (I2C and SPI included); HSI's own ppm. Only the serial figures are read.
  - The `SpiKernel` tag and the select code the firmware writes stay main's (`per_ck`/`hsi_ker_ck`); only the Hz and ppm move.
    A new tag has no `SpiSelect::code`, so `spi_words` would return `None` and generation would fail.
- **The knob firmware does not program RCC.** A knob ELF still runs on HSI at reset. Only the constants it was sized with change.
- **Read back every time:** each harness binary prints its compiled knob, clock, ppm, every kernel function's output, and
  `silicon::fast_hz/slow_hz(64 MHz)`. For each container CLI build, `env-dep:AET4240_TREE=<value>` was grepped from
  `aethelgard_ir-*.d`. The two sims below show two `.d` lines because `with-tools` builds a second `aethelgard_ir`.
- **Disclosure, my error:** one H1 container invocation was launched without `TDIR`, so `img.sh` defaulted to
  `/cache/target-main`. It rebuilt **the main CLI baseline** from the scratch source with `AET4240_TREE=H1:…` (env-dep read back
  afterwards: `=H1:8000000:50:50`, binary 15:01:24). The ELF that run produced came from the untouched `target-scratch` CLI, so
  it was main's. The readback caught it, and every H1 figure here comes from a later build whose `target-scratch` env-dep was
  read back. Fix:
  - `target-main` was rebuilt from `pristine` with the knob unset (`restore-target-main.log`): no `AET4240` env-dep, and the
    bench ELF is `375fd31a…9c02` again.
  - main's `UnknownPart` (U6 ICP10111) refusal of `imu-baro-1v8` was re-captured from the restored binary.
  - No number in this file came from the contaminated binary.

**Harness** (outside the repository): `/home/forge/.cache/aet-4240/harness` (path deps on the scratch `aethelgard-ir` and
`aethelgard-svd-hal`). For each file it runs:
- `Specification::validate`, then `aethelgard_ir::solve` under `catch_unwind`, so a panic is recorded as its own verdict.
- the static checks: `serial::check_silicon`, `motor::check_silicon`, `silicon::check_spi/check_i2c/check_brr`,
  `SolvedTopology::validate`, and svd-hal's `board::check_silicon`. A sweep of every `ok` spec record in `m1/` and `m2/` (393
  records, every tree) found `board::check_silicon` = Ok, `SolvedTopology::validate` = Ok, and every `check_spi`, `check_i2c`,
  `check_brr` and `motor::check_silicon` `holds` = true, with no exceptions.
- each declared serial port and motor output built on the knob's clock, as `port_on` / `motor_timing` build them, so the check's
  columns exist even where `validate` refuses.

Binaries: `m/bins/<tree>`; output: `m/m1/<tree>.jsonl`, `m/m2/<tree>.jsonl`.

## First acceptance (before any tree) — PASS

| build (container, pinned image, `/cache/target-scratch`) | `sentinel-02-bench` ELF sha256 |
|---|---|
| scratch CLI, knob compiled in, `AET4240_TREE` unset (`accept-unset.log`) | `375fd31a28f4442b796110f0df82d31a9e133ce4a12f2a982fbb923cfe369c02` |
| scratch CLI, `AET4240_TREE=HSI` (`accept-hsi.log`, env-dep `=HSI` read back) | `375fd31a28f4442b796110f0df82d31a9e133ce4a12f2a982fbb923cfe369c02` |
| (M7) scratch CLI, `AET4240_TREE=H1:8000000:50:50` (`m7-h1.log`) | `b86459a7888262aeb2ab9afb29649e469f0fb290acb02d7cd60071280db22601` |

Also, with the knob unset: `cargo test -p aethelgard-ir --lib` 47/47 pass (`ir-lib-test-unset.log`). The harness gave the same
`HSI` and `HSIx` (`=HSI`) output on all 31 files (`cmp` equal).

## Grep coverage (pristine tree, `grep-hits.tsv`, 567 hits)

Patterns: `64_958_976|64958976|63_ddd_ddd|14_984|14984|25_894|25894|64_000_000|64000000`.

| class | hits | what was done |
|---|---|---|
| tolerance copy 1: `ir/motor.rs:278,282` `HSI_FAST/SLOW_PPM` (read by `serial.rs:25,428–433` and `motor::check_silicon`) | 2 | `= knob4240::FAST/SLOW_PPM as u32` |
| tolerance copy 2: `ir/silicon.rs:43–44` (`fast_hz`/`slow_hz`: `check_spi`, `check_timingr`, svd-hal's oracle delay `board.rs:5755`) | 2 | `= knob4240::…` |
| tolerance copy 3: `ir/solve.rs:644–645` (`hsi_fastest/slowest`: `solve_mbr`, `spi_controller`, `synthesize_timingr`) | 2 | `= knob4240::…` |
| tolerance copy 4: `svd-hal/board.rs:176–177` (`settle_spins`, the `settle` doc figure, `eot_delay_reads`) | 2 | `= aethelgard_ir::knob4240::…` |
| `svd-hal/board.rs:182` `RESET_HZ` (the core clock at its fastest: `settle_spins` firmware.rs:317, doc 557, `eot_delay_reads` 643) | 1 | `= knob4240::SYS_HZ` (core = `sys_ck`, `D1CPRE` 1) |
| `ir/motor.rs:123` `SolvedClock::reset` 64 MHz | 1 | bypassed: `reset()` returns `knob4240::clock()` when active |
| `ir/silicon.rs:66–67` `RESET_HSI_KER_HZ`, `RESET_PCLK_HZ` (read by `kernel_at_reset` ← `spi_kernel_hz`, and `i2c_kernel_hz`) | 2 | bypassed: `spi_kernel_hz` and `i2c_kernel_hz` return the knob's Hz when active |
| `chaos/src/campaign.rs:2397` `RESET_KERNEL_HZ` (chaos campaign's serial judge) | 1 | **not routed**: no chaos campaign in M1–M7. Under any non-64 MHz serial kernel it would mis-judge |
| `boards/sentinel-01*/src/{board,proofs}.rs` `0x64000000` | 8 | false positive: a GPIOA `PUPDR` word, not a clock |
| `boards/…/board.rs` comments | 4 | committed generated crates, comments; not regenerated |
| `models/renode/ICM42688P.cs:892` | 1 | false positive: an ODR table |
| `#[cfg(test)]` modules and `tests/` | 160 | tests; not run under the knob (they encode main's figures by design) |
| `.json` (spike/4400f outputs, topology fixtures), `.diff`, `.log` | 381 | data; the 5 topology fixtures are M1 rows |

**Clock-taking functions, not literals** (no grep finds them):
- Already general in the clock, verified by readback per tree, no edit: `serial::kernel_hz`, `silicon::usart_kernel_hz`,
  `SolvedClock::timer_kernel_hz` and `solve::i2c_kernel_hz`.
- `solve::spi_kernel` returns `None` unless `source == "HSI"`; edited to return the knob's Hz.
- `hwdef/lib.rs:384` compares the solved `hse_hz` with ArduPilot's `OSCILLATOR_HZ`. Not routed, and not exercised in M1–M7.

**Proof of coverage beyond grep:** `H1:8000000:0:0` (`m/m1/H1_8_0.jsonl`) contains no 64,958,976 or 62,342,784; HSI's has 154
and 115.
- Every motor slowest/fastest figure equals its nominal (M3 rows `H1_8_0`).
- Every kernel's fast/slow is 64,000,000.
- The generated H1 firmware's settle figure is 64,003,200.
**Conclusion: all four tolerance copies and every kernel path are covered for M1–M7. The only unrouted clock literal is chaos's
`RESET_KERNEL_HZ`, which M1–M7 never reach.**
| fixture | kind | HSI | H0_8 | H0_16 | H0_25 | H1_8 | H2_8 |
|---|---|---|---|---|---|---|---|
| aethelgard-cli/…/connectors.json | spec | ok | **panic** | **panic** | ok; 2 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel (of 12) | ok; 3 TIMINGR, 2 spi achieved_max (of 12) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel (of 12) |
| aethelgard-cli/…/d1-24-i2c.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 12 TIMINGR, 4 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) | ok; 12 TIMINGR, 1 spi achieved_max (of 20) | ok; 1 SPI MBR, 12 TIMINGR, 4 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) |
| aethelgard-cli/…/d3-24-i2c-12-spi.json | spec | ok | **panic** | **panic** | ok; 4 SPI MBR, 12 TIMINGR, 4 i2c kernel, 4 spi achieved, 4 spi achieved_max, 4 spi kernel (of 32) | ok; 12 TIMINGR, 4 spi achieved_max (of 32) | ok; 4 SPI MBR, 12 TIMINGR, 4 i2c kernel, 4 spi achieved, 4 spi achieved_max, 4 spi kernel (of 32) |
| aethelgard-cli/…/four-layer.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-cli/…/four-layer-power.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-cli/…/imu-baro-1v8.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-cli/…/motors-dshot600.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) | ok; 3 TIMINGR, 1 spi achieved_max (of 20) | ok; 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) |
| aethelgard-cli/…/motors-pwm.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 4 TIM PSC, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) | ok; 3 TIMINGR, 1 spi achieved_max (of 20) | ok; 1 SPI MBR, 4 TIM PSC, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 20) |
| aethelgard-cli/…/pair-standin.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-cli/…/sentinel-01-bench.json | spec | ok | **panic** | **panic** | ok; 1 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 10) | ok; 3 TIMINGR, 1 spi achieved_max (of 10) | ok; 1 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 10) |
| aethelgard-cli/…/sentinel-02-own.json | spec | ok | **panic** | **panic** | ok; 1 BRR, 2 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 1 usart kernel (of 14) | ok; 3 TIMINGR, 2 spi achieved_max (of 14) | ok; 1 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 1 usart kernel (of 14) |
| aethelgard-cli/…/sentinel-02-shaped.json | spec | ok | **panic** | **panic** | ok; 2 BRR, 2 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 2 usart kernel (of 28) | ok; 3 TIMINGR, 2 spi achieved_max (of 28) | ok; 2 BRR, 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 2 usart kernel (of 28) |
| aethelgard-cli/…/sentinel-03.json | spec | ok | **panic** | **panic** | ok; 3 BRR, 2 SPI MBR, 4 TIM ARR, 4 TIM CCR, 6 TIMINGR, 2 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 3 usart kernel (of 34) | ok; 6 TIMINGR, 2 spi achieved_max (of 34) | ok; 3 BRR, 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 6 TIMINGR, 2 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel, 3 usart kernel (of 34) |
| aethelgard-cli/…/serial-dshot-9600.json | spec | ok | **panic** | **panic** | ok; 1 BRR, 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 22) | ok; 3 TIMINGR, 1 spi achieved_max (of 22) | ok; 1 BRR, 1 SPI MBR, 4 TIM ARR, 4 TIM CCR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 22) |
| aethelgard-cli/…/serial-four.json | spec | ok | **panic** | **panic** | ok; 4 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 4 usart kernel (of 16) | ok; 3 TIMINGR, 1 spi achieved_max (of 16) | ok; 4 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 4 usart kernel (of 16) |
| aethelgard-cli/…/solve/infeasible-shared.json | spec | Infeasible | same refusal | same refusal | same refusal | same refusal | same refusal |
| aethelgard-cli/…/solve/pigeonhole-spi.json | spec | Infeasible | same refusal | same refusal | same refusal | same refusal | same refusal |
| aethelgard-cli/…/solve/stress.json | spec | ok | **panic** | **panic** | ok; 4 SPI MBR, 12 TIMINGR, 4 i2c kernel, 4 spi achieved, 4 spi achieved_max, 4 spi kernel (of 32) | ok; 12 TIMINGR, 4 spi achieved_max (of 32) | ok; 4 SPI MBR, 12 TIMINGR, 4 i2c kernel, 4 spi achieved, 4 spi achieved_max, 4 spi kernel (of 32) |
| aethelgard-cli/…/solve/thrash-pins.json | spec | Infeasible | same refusal | same refusal | same refusal | same refusal | same refusal |
| aethelgard-cli/…/spi-only.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-cli/…/two-bme.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |
| aethelgard-hwdef/…/sentinel-01.topology.json | topology | ok | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** |
| aethelgard-hwdef/…/sentinel-02.topology.json | topology | ok | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** |
| aethelgard-hwdef/…/serial-dshot-9600.topology.json | topology | ok | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** |
| aethelgard-ir/…/sentinel-02-power.json | spec | ok | **panic** | **panic** | ok; 2 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel (of 12) | ok; 3 TIMINGR, 2 spi achieved_max (of 12) | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 2 spi achieved, 2 spi achieved_max, 2 spi kernel (of 12) |
| aethelgard-pincheck/tests/common/topology.json | topology | ok | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** |
| aethelgard-svd-hal/tests/ui/motors-pwm/topology.json | topology | ok | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** | **UnsupportedClock** |
| examples/invalid-zero-domain.json | spec | ZeroNominalVoltage | same refusal | same refusal | same refusal | same refusal | same refusal |
| examples/sentinel-01.json | spec | ok | **panic** | **panic** | ok; 1 BRR, 1 SPI MBR, 6 TIMINGR, 2 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 14) | ok; 6 TIMINGR, 1 spi achieved_max (of 14) | ok; 1 BRR, 1 SPI MBR, 6 TIMINGR, 2 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 14) |
| examples/sentinel-01-minimal.json | spec | ok | **panic** | **panic** | ok; 1 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 10) | ok; 3 TIMINGR, 1 spi achieved_max (of 10) | ok; 1 BRR, 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel, 1 usart kernel (of 10) |
| examples/sentinel-02-bench.json | spec | ok | **panic** | **panic** | ok; 1 SPI MBR, 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) | ok; 3 TIMINGR, 1 spi achieved_max (of 8) | ok; 3 TIMINGR, 1 i2c kernel, 1 spi achieved, 1 spi achieved_max, 1 spi kernel (of 8) |

**What the table shows:**
- **Refusals that lift:** none. No spec is refused on HSI except the 4 deliberate ones, and they stay refused.
- **Refusals that appear:** none in H0_25, H1 (any `hse_hz`) or H2.
- **Panics in H0 at 8 and 16 MHz:** all 22 solvable specs `panic` in `solve` at solve.rs:415 (main: solve.rs:410)
  (`i2c_timing(..).expect("the reset clock gives every I2C peripheral 64 MHz, …")`). `FastPlus1M` needs a kernel of at least
  17 MHz at its slowest (`synthesize_timingr` returns `None`), and the solver turns that `None` into a panic, not a named refusal.
  `validate` (step 21: motor and serial timing) passes at 8 MHz for every spec.
- **Topology fixtures:** all 5 become `UnsupportedClock` in every non-HSI tree, and svd-hal's `check_silicon` refuses their
  stored SPI words. This is expected: their stored clock and SPI figures are HSI's. It is a knob artefact, not a finding.

Distinct value changes vs HSI (all fixtures):
- H0_25 BRR: 1111 -> 434; 152 -> 60; 556 -> 217; 640 -> 250; 6667 -> 2604
- H0_25 SPI MBR: 1 -> 0; 3 -> 1; 4 -> 2; 6 -> 4
- H0_25 TIM ARR: 106 -> 41
- H0_25 TIM CCR: 40 -> 16
- H0_25 TIM PSC: 63 -> 24
- H0_25 TIMINGR: 0x00b20e2d -> 0x00400410; 0x10c7123c -> 0x00a40d2c; 0x50d32a40 -> 0x10f23149
- H0_25 i2c kernel: 64000000 -> 25000000
- H0_25 spi achieved: 16000000 -> 12500000; 2000000 -> 3125000; 4000000 -> 6250000; 500000 -> 781250
- H0_25 spi achieved_max: 16239744 -> 12500625; 2029968 -> 3125156; 4059936 -> 6250312; 507492 -> 781289
- H0_25 spi kernel: 64000000 -> 25000000
- H0_25 usart kernel: 64000000 -> 25000000
- H1_8 TIMINGR: 0x00b20e2d -> 0x00a20e2d; 0x10c7123c -> 0x10c7123b; 0x50d32a40 -> 0x50d32a3f
- H1_8 spi achieved_max: 16239744 -> 16000800; 2029968 -> 2000100; 4059936 -> 4000200; 507492 -> 500025
- H2_8 BRR: 1111 -> 1736; 152 -> 238; 556 -> 868; 640 -> 1000; 6667 -> 10417
- H2_8 SPI MBR: 1 -> 2
- H2_8 TIM ARR: 106 -> 332
- H2_8 TIM CCR: 40 -> 125
- H2_8 TIM PSC: 63 -> 199
- H2_8 TIMINGR: 0x00b20e2d -> 0x10830c23; 0x10c7123c -> 0x20d8133e; 0x50d32a40 -> 0x70f3314a
- H2_8 i2c kernel: 64000000 -> 100000000
- H2_8 spi achieved: 16000000 -> 10000000; 16000000 -> 12500000; 2000000 -> 2500000; 4000000 -> 5000000; 500000 -> 625000
- H2_8 spi achieved_max: 16239744 -> 10000500; 16239744 -> 12500625; 2029968 -> 2500125; 4059936 -> 5000250; 507492 -> 625031
- H2_8 spi kernel: 64000000 -> 100000000; 64000000 -> 80000000
- H2_8 usart kernel: 64000000 -> 100000000

**H1's words do not depend on `hse_hz` [measured]:**
- H1 at 16 MHz and 25 MHz vs 8 MHz: 426 solved words compared, 0 differ, and every verdict is the same. Only `clock.hse_hz` moves.
- H0 at 16 MHz panics exactly as H0 at 8 MHz does.
- H0 at 25 MHz solves everything. Its BRR, PSC/ARR/CCR, MBR and TIMINGR move as the 25 MHz kernel dictates (above).

## M2. CRSF on serial-four, sentinel-02-shaped and sentinel-03 (`m2/fx/*-{416666,420000}.json`, only the CRSF baud changed)

All three fixtures give identical CRSF columns. The columns come from `serial::check_silicon` on the port built as `port_on`
builds it. "verdict" is `validate`+`solve` of the whole spec. Peer 8,002 ppm is the fixtures' declared term. The table shows the
CRSF port only. Every other port's columns (MAVLINK, GPS, SBUS, CONSOLE) are in `m2/<tree>.jsonl` under `serial_columns`.

| tree | baud | verdict (3 fixtures) | kernel_hz | BRR | quant ppm | clock term ppm | receiver | peer | worst | Table 407 | margin | CRSF check holds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| HSI | 416666 | SerialBaudTolerance | 64,000,000 | 154 | 2,596 | 25,894 | 28,423 | 8,002 | 36,425 | 33,300 | -3,125 | False |
| HSI | 420000 | ok | 64,000,000 | 152 | 2,507 | 25,894 | 23,453 | 8,002 | 31,455 | 33,300 | 1,845 | True |
| H1_8_20 | 416666 | ok | 64,000,000 | 154 | 2,596 | 20 | 2,616 | 8,002 | 10,618 | 33,300 | 22,682 | True |
| H1_8_20 | 420000 | ok | 64,000,000 | 152 | 2,507 | 20 | 2,527 | 8,002 | 10,529 | 33,300 | 22,771 | True |
| H1_8 | 416666 | ok | 64,000,000 | 154 | 2,596 | 50 | 2,646 | 8,002 | 10,648 | 33,300 | 22,652 | True |
| H1_8 | 420000 | ok | 64,000,000 | 152 | 2,507 | 50 | 2,557 | 8,002 | 10,559 | 33,300 | 22,741 | True |
| H1_8_100 | 416666 | ok | 64,000,000 | 154 | 2,596 | 100 | 2,696 | 8,002 | 10,698 | 33,300 | 22,602 | True |
| H1_8_100 | 420000 | ok | 64,000,000 | 152 | 2,507 | 100 | 2,607 | 8,002 | 10,609 | 33,300 | 22,691 | True |
| H1_8_200 | 416666 | ok | 64,000,000 | 154 | 2,596 | 200 | 2,796 | 8,002 | 10,798 | 33,300 | 22,502 | True |
| H1_8_200 | 420000 | ok | 64,000,000 | 152 | 2,507 | 200 | 2,707 | 8,002 | 10,709 | 33,300 | 22,591 | True |
| H1_8_22760 | 416666 | ok | 64,000,000 | 154 | 2,596 | 22,760 | 25,297 | 8,002 | 33,299 | 33,300 | 1 | True |
| H1_8_22761 | 416666 | SerialBaudTolerance | 64,000,000 | 154 | 2,596 | 22,761 | 25,298 | 8,002 | 33,300 | 33,300 | 0 | False |
| H2_8 | 416666 | ok | 100,000,000 | 240 | 2 | 50 | 52 | 8,002 | 8,054 | 37,500 | 29,446 | True |
| H2_8 | 420000 | ok | 100,000,000 | 238 | 401 | 50 | 451 | 8,002 | 8,453 | 33,300 | 24,847 | True |
| H0_8 | 416666 | panic | 8,000,000 | 19 | 10,528 | 50 | 10,579 | 8,002 | 18,581 | 33,300 | 14,719 | True |
| H0_8 | 420000 | panic | 8,000,000 | 19 | 2,507 | 50 | 2,557 | 8,002 | 10,559 | 33,300 | 22,741 | True |
| HSIPLL_100 | 416666 | sentinel-02-shaped: ok; sentinel-03: ok; serial-four: SerialBaudTolerance | 100,000,000 | 240 | 2 | 25,894 | 25,893 | 8,002 | 33,895 | 37,500 | 3,605 | True |
| HSIPLL_100 | 420000 | SerialBaudTolerance | 100,000,000 | 238 | 401 | 25,894 | 25,505 | 8,002 | 33,507 | 33,300 | -207 | False |
| HSIPLL_200 | 416666 | ok | 200,000,000 | 480 | 2 | 25,894 | 25,893 | 8,002 | 33,895 | 37,500 | 3,605 | True |
| HSIPLL_200 | 420000 | SerialBaudTolerance | 200,000,000 | 476 | 401 | 25,894 | 25,505 | 8,002 | 33,507 | 33,300 | -207 | False |
| HSIPLL_240 | 416666 | panic | 240,000,000 | 576 | 2 | 25,894 | 25,893 | 8,002 | 33,895 | 37,500 | 3,605 | True |
| HSIPLL_240 | 420000 | panic | 240,000,000 | 571 | 751 | 25,894 | 25,163 | 8,002 | 33,165 | 33,300 | 135 | True |

**Notes:**
- **Table 407 row:** at BRR 240 and 480 (`DIV_Fraction` 0) the row is 37,500; at 238 and 476 it is 33,300. P2's 3,605 margin
  is against the 37,500 row.
- **serial-four 416666 at HSIPLL 100 MHz** is refused by its **SBUS** port, not by CRSF. SBUS is 100,000 baud, 8E2, M = 01, at
  BRR 1,000 (fraction ≠ 0, row 30,300): worst 30,894, margin −594.
- **HSIPLL 240 MHz panics** at the same I2C `.expect`. `Standard100k`'s SCLDEL+1 (at most 16) × PRESC+1 (at most 16) = 256
  cycles is below the 1,250 ns tr+tSU;DAT at ~243.6 MHz (305 cycles). This comes from my choice of I2C kernel = `sys_hz` in
  HSIPLL. The CRSF columns are unaffected.
- **H0 at 8 MHz panics** (I2C, M1). Its CRSF figures hold: 416666 at BRR 19, margin 14,719.

**Largest crystal ppm at which 416666 still solves at 64 MHz [measured]: ±22,760 ppm.**
- Desk arithmetic gave 22,760 from the check's formula, and two builds confirm it: `H1:8000000:22760:22760` solves all three
  fixtures (worst 33,299, margin +1). `…:22761:22761` is refused `SerialBaudTolerance` (worst 33,300, not below 33,300).
- Only the slow side binds. BRR·baud = 64,166,564 > 64 MHz, so a slow kernel worsens the error and a fast one improves it.
- The same arithmetic for 420000 gives ±22,733 [inferred, not built].
- Sensitivity: 416666's margin moves exactly 1 ppm per ppm of crystal: 22,682 / 22,652 / 22,602 / 22,502 at ±20 / 50 / 100 / 200.

## M3. DShot and PWM: `motor::check_silicon` per motor fixture per tree (ps; "slowest–fastest" = at the kernel's slow and fast extremes)

The timings come from the spec's outputs solved on the tree's timer kernel and judged on timer 1. Both APBs give the same kernel
in every tree. H0 rows are figures only: `solve` panics on I2C there. The five motor fixtures give the same rows (the 4 DShot
fixtures are identical), so one row is shown per protocol.

| tree | protocol | kernel Hz | PSC/ARR/CCR | period nominal | period slowest–fastest | high nominal | high slowest–fastest | holds |
|---|---|---|---|---|---|---|---|---|
| HSI | Dshot600 | 64,000,000 | 0/106/40 | 1,666,666 | 1,716,317–1,647,193 | 624,999 | 641,613–615,773 | True |
| HSI | Pwm | 64,000,000 | 63/2499/1000 | 2,500,000,000 | 2,566,455,806–2,463,093,014 | 1,000,000,000 | 1,026,582,322–985,237,205 | True |
| H0_8 | Dshot600 | 8,000,000 | 0/12/5 | 1,666,666 | 1,625,081–1,624,918 | 624,999 | 625,031–624,968 | True |
| H0_8 | Pwm | 8,000,000 | 7/2499/1000 | 2,500,000,000 | 2,500,125,006–2,499,875,006 | 1,000,000,000 | 1,000,050,002–999,950,002 | True |
| H0_16 | Dshot600 | 16,000,000 | 0/26/10 | 1,666,666 | 1,687,584–1,687,415 | 624,999 | 625,031–624,968 | True |
| H0_16 | Pwm | 16,000,000 | 15/2499/1000 | 2,500,000,000 | 2,500,125,006–2,499,875,006 | 1,000,000,000 | 1,000,050,002–999,950,002 | True |
| H0_25 | Dshot600 | 25,000,000 | 0/41/16 | 1,666,666 | 1,680,084–1,679,916 | 624,999 | 640,032–639,968 | True |
| H0_25 | Pwm | 25,000,000 | 24/2499/1000 | 2,500,000,000 | 2,500,125,006–2,499,875,006 | 1,000,000,000 | 1,000,050,002–999,950,002 | True |
| H1_8 | Dshot600 | 64,000,000 | 0/106/40 | 1,666,666 | 1,671,958–1,671,791 | 624,999 | 625,031–624,968 | True |
| H1_8 | Pwm | 64,000,000 | 63/2499/1000 | 2,500,000,000 | 2,500,125,006–2,499,875,006 | 1,000,000,000 | 1,000,050,002–999,950,002 | True |
| H1_8_0 | Dshot600 | 64,000,000 | 0/106/40 | 1,666,666 | 1,671,875–1,671,875 | 624,999 | 625,000–625,000 | True |
| H1_8_0 | Pwm | 64,000,000 | 63/2499/1000 | 2,500,000,000 | 2,500,000,000–2,500,000,000 | 1,000,000,000 | 1,000,000,000–1,000,000,000 | True |
| H2_8 | Dshot600 | 200,000,000 | 0/332/125 | 1,666,666 | 1,665,083–1,664,916 | 624,999 | 625,031–624,968 | True |
| H2_8 | Pwm | 200,000,000 | 199/2499/1000 | 2,500,000,000 | 2,500,125,006–2,499,875,006 | 1,000,000,000 | 1,000,050,002–999,950,002 | True |

- **DShot600 bit:**
  - Under H1 ±50 the band is 1,671,791–1,671,958 ps: ±0.084 ns about **1,671.875 ns**. That is 107 ticks (ARR 106 + 1) at
    64 MHz, not 1,656.25 ns (106 ticks).
  - On HSI the band is 1,647,193–1,716,317 ps.
  - The solved bit itself is +3,125 ppm off the nominal 1,666.67 ns, the half-tick quantization. That error dominates a crystal.
- **PWM idle pulse:** 999.950–1,000.050 µs under H1 ±50, against 985.24–1,026.58 µs on HSI. The period is 2,500 µs ± 0.125 µs.
- **The nominal verdict:** `holds` is true in every tree, H0 8 MHz included. There DShot600's bit is 13 ticks (ARR 12) =
  1,625.0 ns, −2.5 %, which is within half a tick and so holds.

## M4. I2C `TIMINGR` and SPI `MBR` when the kernel extremes narrow (H1 ±50 vs HSI; `m4.md`)

- **Every `TIMINGR` word moves:** 99 of 99 words in the 22 solved specs, one field each. The judge's extremes go from
  64,958,976 / 62,342,784 to 64,003,200 / 63,996,800, and every moved word holds `check_i2c` (tight and least `PRESC`).
  - `Standard100k` `0x50d32a40` → `0x50d32a3f`: SCLL 64 → 63. tHIGH min margin 2.5 ns → 62.3 ns; SCL max 99,631 → 99,077 Hz.
  - `Fast400k` `0x10c7123c` → `0x10c7123b`: SCLL 60 → 59. tHIGH 615.8 → 625.0 ns; SCL max 396,092 → 395,082 Hz.
  - `FastPlus1M` `0x00b20e2d` → `0x00a20e2d`: SCLDEL 11 → 10. tHIGH 261.7 → 265.6 ns; SCL max 999,369 → 984,665 Hz.
- **No `MBR` moves:** 0 of 33 SPI controllers. Every `achieved_max_hz` moves, e.g. 16,239,744 → 16,000,800 and
  2,029,968 → 2,000,100.
- At ±0 ppm (`H1_8_0`) more moves: `Standard100k` → `0x40f3324c` (PRESC 5 → 4), and SPI MBR 4 → 3 and 6 → 5 where
  the declared speed equals kernel/2^(MBR+1) exactly.

| fixture | I2C ctrls | TIMINGR moved | SPI ctrls | MBR moved | achieved_max moved |
|---|---|---|---|---|---|
| connectors.json | 1 | 3/3 | 2 | 0/2 | 2/2 |
| d1-24-i2c.json | 4 | 12/12 | 1 | 0/1 | 1/1 |
| d3-24-i2c-12-spi.json | 4 | 12/12 | 4 | 0/4 | 4/4 |
| four-layer.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| four-layer-power.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| imu-baro-1v8.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| motors-dshot600.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| motors-pwm.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| pair-standin.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| sentinel-01-bench.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| sentinel-02-own.json | 1 | 3/3 | 2 | 0/2 | 2/2 |
| sentinel-02-shaped.json | 1 | 3/3 | 2 | 0/2 | 2/2 |
| sentinel-03.json | 2 | 6/6 | 2 | 0/2 | 2/2 |
| serial-dshot-9600.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| serial-four.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| stress.json | 4 | 12/12 | 4 | 0/4 | 4/4 |
| spi-only.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| two-bme.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| sentinel-02-power.json | 1 | 3/3 | 2 | 0/2 | 2/2 |
| sentinel-01.json | 2 | 6/6 | 1 | 0/1 | 1/1 |
| sentinel-01-minimal.json | 1 | 3/3 | 1 | 0/1 | 1/1 |
| sentinel-02-bench.json | 1 | 3/3 | 1 | 0/1 | 1/1 |

## M5. The waits

**Formula** (`svd-hal/src/board/firmware.rs:316–321`): `settle_spins(us) = ceil(us × hsi_fastest(RESET_HZ) / 10⁶)`, where
`hsi_fastest(RESET_HZ)` is the core at its fastest, `ceil(64 MHz × (1 + fast_ppm))`. It is emitted once, as the largest
cited start-up (`settle(N)` before the first data read).

| | HSI (64,958,976 Hz) | H1 ±50 (64,003,200 Hz) |
|---|---|---|
| `sentinel-02-bench` (20,000 µs), read from the generated `main.rs:243` | `settle(1299180)` | `settle(1280064)` |
| `imu-baro-1v8` (40,000 µs) [same formula; the sim below witnesses the difference] | 2,598,360 | 2,560,128 |
| SPI EOT erratum delay (`eot_delay_reads`), `sentinel-02-bench` SPI1 MBR 4 | 34 reads | 33 reads (3 rounds of 16 either way) |

**Twin, `imu-baro-1v8`:**
- The run is `twin_tier2::the_imu_and_barometer_fixture_runs_every_script_step_witnessed`, scratch tree, container. It needed
  the bridge built beside the CLI and a cargo home merged from aet-4230's.
- The CLI's `generate` refuses this fixture on main as well: `UnknownPart` U6 ICP10111, measured with `/cache/target-main`. The
  test builds the board through `generate_with` and the twin's model list instead.
- Both runs pass, 100 ms window.
- The boot word: **78,016,480 ns (HSI)** and **76,869,520 ns (H1 knob firmware)**.
  - The difference, 1,146,960 ns, is 38,232 fewer spins × 3 instructions at Renode's 100 MIPS, exactly.
  - The only other verdict differences are the firmware sha and peak RSS.
- **Label:** twin at main's stock platform clock, re-check after 4400g. The knob firmware does not program RCC.
- Logs: `m5-sim-{HSI,H1}.log`; verdicts: `m5-verdict-*.json`.

## M6. PH0/PH1 reserved

`AET4240_RESERVE=PH0PH1` (crystal) and `=PH0` (bypass), HSI tree. The 22 solved and 4 refused specs were compared against
unreserved (`m6.md`).
- **No placement moves**, and no verdict changes.
- No fixture places anything on PH0 or PH1 today.
- In the pin table PH0 and PH1 have no alternate function at all (read back: `[]`), so only a `gpio` or analog net could ever
  land there.

## M7. The bench ELF

- Unset and `HSI` give `375fd31a…9c02` (the first acceptance).
- H1 ±50 gives `b86459a7…2601`. `diff -ru` of the two generated `board/` trees (`m7-board.diff`, 78 lines) shows four changes:
  1. `settle(1299180)` → `settle(1280064)`, and the doc figure 64,958,976 → 64,003,200 Hz.
  2. `I2C1` `TIMINGR` `0x50d32a40` → `0x50d32a3f` and `0x10c7123c` → `0x10c7123b`, in `board.rs` and `proofs.rs`. The
     bench never writes `FastPlus1M`.
  3. The EOT-delay comment and Kani bound go from 34 to 33 reads. The emitted rounds are unchanged.
  4. Nothing else changes: no PSC/ARR/BRR, no RCC write, and the Renode platform is the same.

## Predictions scored (P3 and P5 were not measured by this hand)

- **P1 — confirmed:**
  - H1 ±50, CRSF 416666: BRR 154, quantization 2,596, peer 8,002, Table 407 33,300, margin **22,652**.
- **P2 — confirmed:**
  - 416666 margin **3,605** at 100, 200 and 240 MHz.
  - 420000 at 100 and 200 MHz: **−207**.
  - 420000 at 240 MHz: +135, which P2 did not claim to fail.
  - At 240 MHz `solve` panics on I2C under my HSIPLL kernel choice; the serial check holds.
- **P4 — half confirmed:**
  - PWM: idle pulse 1,000.0 ± 0.05 µs, from up to 1,026.58 µs.
  - DShot600: the band width (±0.084 ns) is confirmed, but the centre is **1,671.875 ns** (ARR+1 = 107 ticks), not
    1,656.25 ns.
  - The HSI band is 1.6472–1.7163 µs as stated.
- **P6 — confirmed:** unset and `HSI` both give 375fd31a….
- **P7 — half confirmed:**
  - Correct: no refusal under H1, and every PSC/ARR/CCR and BRR is unchanged.
  - Wrong: "no TIMINGR moves". **All 99 move**, one field each, in every fixture. No MBR moves.
- **P8 — refuted as stated:**
  - No motor refusal at 8 MHz: DShot600 solves 0/12/5 and holds.
  - Every fixture with an I2C bus (all 22 solvable) **panics** in `solve`, because FastPlus1M needs at least 17 MHz.
  - That is a panic, not a named refusal: a latent engine defect for any kernel under 17 MHz.
- **P9 — confirmed in substance:**
  - Every motor fixture's timer words move under H2: PWM PSC 63 → 199 (ARR unchanged), DShot ARR/CCR 106/40 → 332/125
    (PSC 0 unchanged).
  - No new refusal.

## Files (all under `/home/forge/.cache/aet-4240/`)

- `m/knob.patch` is the deviation.
- `harness/` is the measuring bin.
- `tools/` holds `hb.sh` (build the harness per knob), `allbins.sh`, `m1table.py` and `diffwords.py`.
- `m/m1/*.jsonl`, `m/m2/*.jsonl` and `m/m2/fx/` are the raw records.
- `m/m1-table-{a,b}.md`, `m/m3.md`, `m/m4.md` and `m/m6.md` are the full tables.
- `m/grep-hits.tsv` is the grep coverage list.
- `runs/s-{unset,hsi,h1}{,.elf}` and `runs/main-restore{,.elf}` are the generated boards and ELFs.
- Logs: `m/accept-*.log`, `m/m7-h1.log`, `m/m5-sim-*.log`, `m/restore-target-main.log`.
- The untracked `spike/4240/{ds,py,reads}` in the real worktree (mtime 14:59:39) were not written by any command of this hand:
  every write went under `/home/forge/.cache/aet-4240`.
- Targets: `target-scratch` (4.4 G), `target-harness`, `target-host`. A merged cargo home is in `cargo-home-full`.
