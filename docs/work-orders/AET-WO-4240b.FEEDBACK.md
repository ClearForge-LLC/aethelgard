# AET-WO-4240b — FEEDBACK (build)

## §0 The answer in one screen

- **Both HSE fixtures generate, build and simulate to `pass`** [measured, tools image, `--for 100ms`].
  - `hse-crystal-16` (ABM8AIG-16, 16 MHz) has ELF `07c43d49…`; `hse-external-8` (8 MHz external) has ELF `762c304d…`.
  - The generated platform sets `hseFrequency`. The twin's clock rule names nothing, and the boot word is `BOOT_MAGIC`.
  - Tier 2: `twin_tier2::each_hse_fixture_boots_to_pass_under_the_clock_rule`, 5.7 s and 5.5 s.
- **No `null` board's ELF or verdict moved** [measured: 29 committed specifications plus ArduPilot's foreign run, against `main` `8fdc9422`'s baseline].
  - Emitted bytes moved by two causes only, each ruled:
    - the version and sha lines (specification 23, solved 15 (renumbered from 22/14 after `1211a`));
    - `4240v` F2's `Scale2` lines (calls and signatures, Q2, D-2).
- **The bring-up** (`Hse::start`), for an HSE board only, writes these words, each whole and once:
  - `RCC_CR` from its reset `0x0000_0001`: `HSEBYP` (external), then `HSEON`;
  - a bounded `HSERDY` wait;
  - `HSECSSON`;
  - `PLLCKSELR`, `PLLCFGR` and `PLL1DIVR` as solved;
  - `PLL1ON`, then a bounded `PLL1RDY` wait;
  - `FLASH_ACR` as solved, read back;
  - `SW` = PLL1, then a bounded `SWS` wait.

  Each timeout halts by name: `HSE_NOT_READY` `0x2100_B0F4`, `PLL1_NOT_LOCKED` `B0F5`, `SW_NOT_TAKEN` `B0F6`, `FLASH_ACR_NOT_TAKEN` `B0F8` (D-1).
- **`SolvedClock`** is produced only after `SWS` reads PLL1, and every `init`/`start_outputs`/`configure` takes it. No kernel select is written: `SPI123SEL`/`SPI45SEL`/`SPI6SEL` stay at reset, and the topology says `pll1_q`/`pclk` (Q3). No `MBR` moved (1 and 4, as `sentinel-02-shaped`).
- **The CSS NMI** (`css_hold`, `HSE_CSS_FAULT` `0x2100_B0F7`) drives **every** motor low and parks (D-3).
- **Kani:** `hse_start_is_in_order_bounded_and_named` takes 1 s on the crystal fixture and 2 s on the external one.
  - Every ready read and the `FLASH_ACR` read-back are nondeterministic, and the three bounds are symbolic (1–2 reads).
  - The 8 writes are pinned to register, value and order, and each timeout reaches its own fault with nothing written after it.
- **The twin's rule** (`aethelgard_twin::hse::judge_clock`) allows exactly the solved words, once each, in order, with the polled reads between. Each departure is named in a `clock_access` finding.
- **`HseNotGenerated`** is lifted for H1; any other HSE tree is refused, the refusal naming the tree.
- **`4240v` F1:** RM0433 Rev 9 gives no encoding for `ACTVOS` (p. 306: "VOS currently applied … reflect[s] the last VOS value applied"), and DS12117 Rev 10 never names it. The residue stays stated and the row open.

## §1 Predictions, scored (`spike/4240b/PREDICTIONS.md`, `c0bb888a`)

| # | Prediction | Result |
|---|---|---|
| P1 | No `null` specification moves under the mask | **Confirmed for every ELF and verdict.** Bytes beyond the version mask moved by F2 alone, ruled after the predictions (Q2, D-2) |
| P2 | The crystal's write sequence | **Confirmed**, with the CR words made whole: `0x0001_0001`, `0x0009_0001`, `0x0109_0001` |
| P3 | The external sequence | **Confirmed**: `0x0004_0001`, `0x0005_0001`, `0x000D_0001`, `0x010D_0001`; `PLLCFGR` `0x0003_0002`, `PLL1DIVR` `0x0103_069F` |
| P4 | Boot words `B0F4`–`B0F7` | **Confirmed**, plus `B0F8` (D-1) |
| P5 | Bounds in polls | **Confirmed**: `PLL1RDY` 9,744. `HSERDY` 1,299,180 (ruled 20,000 µs). `SWS` 9,744 (ruled 150 µs, Q5) |
| P6 | Settle and EOT at the crystal's fastest | **Confirmed**: 20 ms settle is 1,280,131 (crystal) and 1,280,064 (external) against 1,299,180. EOT is 33 reads against 34 (§4 D-4) |
| P7 | Renode takes `hseFrequency`; `sys_ck` 64 MHz | **Confirmed** (§2.2) |
| P8 | Boot-to-`CEN` earlier by the settle ratio | **Confirmed**: −567,580 ns (§2.3) |
| P9 | Kani under 600 s | **Confirmed**: 1 s and 2 s |
| P10 | `ACTVOS` has no encoding | **Confirmed** |

## §2 Measured

### §2.1 The `null` boards (`~/.cache/aet-4240b/kit/sweep.sh`, `compare.py`)

- Baseline: `main` `8fdc9422`, the tools image, `--for 100ms` (`base100`).
  - 20 boards pass. 2 fail `spi_transaction_unwitnessed` on `main` itself (`d3-24-i2c-12-spi`, `solve/stress`).
  - `sentinel-02-own` is refused as foreign-clock by design.
  - 4 are refused by design, and `imu-baro-1v8` does not generate on `main`.
- After: `29332408` (`after1`). Every ELF is identical, and every verdict is identical apart from `imu-baro-1v8`.
  - `imu-baro-1v8` now generates and passes because of `1210`'s `board.drawn` (merged from `main`), not this branch.
- The remaining byte differences, line by line:
  - lines naming the specification sha;
  - the solved topology's `schema_version` (13 → 14);
  - in `board.rs`, `firmware/src/main.rs` and `proofs.rs`, `Scale2`'s signature, doc and call lines and its `#[derive(Clone, Copy)]`;
  - `generate.json` and `build.json`, by the files' hashes and paths.
- The `order_2203` "boot image" digest is the `Debug` text of the twin's expected image. It moved by its shape alone: `BrownOut.clock_acr: None`. With that removed it hashes to `221f6b59…6d70`, the previous digest, exactly.

### §2.2 Renode 1.17.0 (`spike/4240b/renode/`)

- `hseFrequency` is taken as an update block after the stock include (`rcc:` / `    hseFrequency: 16000000`). A second full `rcc:` declaration is `E02`.
- Read through `nvic Frequency` (SysTick = `sys_ck` / `D1CPRE`), first moved off 64 MHz with `SW` = CSI (4 MHz). After `SW` = PLL1 it is **64,000,000 Hz** on both fixtures, and 32,000,000 Hz on the control without the parameter.
- `PLLCFGR` and `HSEBYP` are tags (`PLLCFGR` reads `0x01FF_0000` after the write). The kernel-select registers read 0. `FLASH_ACR` stores what is written and reads it back. CSS has no detection and no NMI.

### §2.3 The boot shift (§1.6)

- Boot-to-`CEN` (TIM1) on the 16 MHz fixture is 42,106,410 ns; its `null` base `sentinel-02-shaped` is 42,673,990 ns. The difference, −567,580 ns, is the settle spins falling 1,299,180 → 1,280,131 (≈ 571 µs at 3 instructions per spin, 100 MIPS) less the bring-up's added instructions.
- The external fixture is 42,104,550 ns.
- **What the twin under-counts against silicon:**
  - the `HSERDY` wait: `tSU(HSE)`, 2 ms typical with no published maximum, bounded by the ruled 20 ms;
  - `PLL1RDY`: up to 150 µs;
  - the `SWS` handover.

  Renode's RDY bits read their ON bit at once.

### §2.4 The hwdef fixtures at specification 23 (the architect's 18:26Z note)

CI's `ardupilot-image` job runs `solve` then `generate hwdef --topology` and `cmp`s the result against `crates/aethelgard-hwdef/tests/fixtures/`. Re-taken the same way at `3c800118`. Each file was checked by putting the old sha back in the regenerated file and comparing it with the committed one byte for byte:

| File | Moved | Old → new specification sha256 |
|---|---|---|
| `sentinel-01.topology.json`, `.hwdef.dat` | the sha line only | `db17a0f8…` → `5e840088…` |
| `sentinel-02.topology.json`, `.hwdef.dat` | the sha line only | `bd02ba36…` → `79147f4b…` |
| `serial-dshot-9600.topology.json`, `.hwdef.dat` | the sha line only | `b38c2406…` → `75fe0fca…` |

```
- # From the SolvedTopology of specification sha256 bd02ba361872948ecf1222f328ed85a2a4347a3d3bcfbacab19e7176c72b60bf and the parts tables alone (RUL-16). Do not edit: regenerate.
+ # From the SolvedTopology of specification sha256 79147f4b9f046ebb48f818a624d8c128ace114f5e3c6820225dc117aec621417 and the parts tables alone (RUL-16). Do not edit: regenerate.
```

`sentinel-02-noimu` is `sentinel-02`'s file with its `IMU` line dropped, so it follows. A grep of the tree, outside `spike/` and `docs/`, for every pre-renumber specification sha (each committed specification at `main` `25203c58`, and the HSE fixtures at 22) finds:
- `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L9: its sha and "specification 22" re-taken (`ac54b806…` → `ceccebff…`, 23);
- the bench package's `FAB-NOTES.md` and `MANIFEST.json`, which the write=true dispatch replaces.

## §3 What was built

- **IR:**
  - `startup_max_us` and `startup_source` on both HSE modes, 0 refused;
  - `SolvedHse.flash_acr`, from the solver's own copy of Table 17's Scale 2 rows, with `vos::flash_at_reset` judging the written word, reserved bits included;
  - `SpiKernel::{Pll1Q, Pclk}`, with clock-aware kernels and budgets in the solver, `check_spi` and the VOS judge;
  - `HseCopyNotSolved` (§1.3).
- **Generator:** `Hse::start`, `ClockFault`, `SolvedClock`, `css_hold`, the Kani harness, `HsePollsOverflow`, F2, and the svd-hal ledger rows.
- **Twin:** `hse.rs` (`ClockRule`, `judge_clock`), `hseFrequency` on the platform, the boot words, the residual, the unobservables, and `LOG_FILTER_HSE`.
- **CLI:**
  - `generable` (H1 only);
  - ledger rows for every field of every whole word the bring-up writes, with the polled `HSERDY`/`PLL1RDY`;
  - the `clock_access` finding;
  - H1 acknowledgements judged.
- **§1.3, the copied declaration:** `generate` re-solves from the specification. `SolvedTopology::validate`, which `generate hwdef --topology` and `check pinout` both run, re-solves the clock from the declaration the topology carries and refuses any derived field that differs (`HseCopyNotSolved`).
  - The judge stays free of the solver's word choice (`4240a` A6).
  - **Residue:** a declaration edited together with every field derived from it is another specification, bound only by `specification_sha256`, which a topology-only path does not hold.

## §4 Decisions and deviations

- **D-1, a fifth boot word.** A `FLASH_ACR` read-back that differs from the written word halts as `FLASH_ACR_NOT_TAKEN` `0x2100_B0F8` and never switches. CR words are whole, built from RM0433 Rev 9's reset value `0x0000_0001` (p. 383).
- **D-2, `Scale2` by value (ruled 17:26Z).** It replaces the ruled `&Scale2`. `&Scale2` moved the `d3-24-i2c-12-spi` ELF by 24 bytes: its enables are not inlined, and each of 12 calls passed one more pointer. That is the ruled stop.
  - A zero-sized `Copy` token passed by value leaves all 24 null ELFs identical.
  - The gate holds: no token exists without `pwr.scale2`, and a copy needs a token.
  - The UI test `pwr_enable_without_scale2` fails to compile as required.
- **D-3, the NMI drives every motor (ruled 17:26Z).**
  - RM0433 Rev 9 p. 1614 (TIM1/8 `BDTR`): "Bit 12 BKE: Break enable — This bit enables the complete break protection (including all sources connected to bk_acth and BRK sources) … 0: Break function disabled".
  - p. 1818 (TIM15/16/17): "0: Break inputs (BRK and CCS clock failure event) disabled".
  - The generated firmware writes `BDTR` as `MOE` alone, so no break is armed, and WO §1.2's "held by the hardware break" does not hold for it.
  - `css_hold` drives every motor low (`BSRR` reset, then `MODER` output) and parks. The board's doc lists each motor by pin, net and timer channel.
  - A CSS taken before a port's clock is on leaves that pin at reset (analog), never timer-driven.
- **D-4, the EOT delay** on `pll1_q`/APB kernels takes the source's slowest kernel as well as its fastest core, so an HSE board gets 33 reads against the null board's 34. Both are 3 rounds of 16.
- **D-5, `startup_source` is ASCII.** The ruled text's "10×" is written "10x", because the field takes 1–200 bytes of printable ASCII.
- **D-6, the fixtures** are `sentinel-02-shaped` with `mcu.hse`, and MOTOR3/4 pinned to PA0/PA1 so that TIM2 carries motors (the NMI's case).
- **D-7, an HSE board's `FLASH.ACR` silicon-only acknowledgement is stale** (the board writes and judges the register) and is refused by name; `generate_tier1` shows both cases.
- **D-8, ledger.** There is no separate polled `CFGR.SWS` row: `RCC.CFGR.SWS@reset` holds that id, and the CLI refuses duplicates. `SWS` is judged by the twin's clock rule and by Kani.

- **D-9, the external fixture is a twin stand-in (`board.drawn: false`) since `4240c`.**
  - `4240c` (ruled Q4 (a)) refuses `emit` for any external clock (`HseSourceNotDrawn`): no connector pin or test point can carry it to PH0 yet. So `generate`, which also writes `kicad/`, refused `hse-external-8` once `4240c` merged.
  - With `board.drawn: false` (`1210`), `generate` writes its board crate and platform and no `kicad/`, which is the truth for an external-clock board today.
  - It still generates, builds and simulates to `pass`: ELF `762c304d…`, unchanged.
  - `4240c`'s two tests were brought to this work order's fixtures:
    - its external and crystal declarations carry the ruled start-up fields;
    - "no committed specification without a crystal draws one" skips the two HSE fixtures and counts them.

## §5 Adversarial pass (full)

A fresh Opus subagent, given the code at `50f84cee` and the work order's §5 list but not my conclusions, tried every attack. Its probes, specs and logs are in `~/.cache/aet-4240b/adv/`. **No blocker.**

| # | Finding | Severity | Done |
|---|---|---|---|
| A1 | No gate read `sys_ck` back. The tier-2 test checked the `.repl` text, and only motor timing caught a wrong `hseFrequency`: with it mutated to 8 MHz, all four outputs were `motor_timing_mismatch`. A motorless HSE board would pass on a wrong `sys_ck` | should-fix | **Fixed** (the tier-2 test asserts every motor witnessed at its solved timing, at a 64 MHz kernel). The motorless gap is F4 |
| A2 | `startup_max_us` is refused only at 0. A declared 1 µs generates `HSE_POLLS` 65, which the twin passes and silicon would halt as `HSE_NOT_READY` on every boot | decision | Asked on the bus. The builder's lean: no floor here, the bypass bench measures `tSU` (F3) |
| A3 | `FlashWaitStatesBelowTable`'s text said "FLASH_ACR at reset" for an HSE tree's written word | note | **Fixed** (text) |
| A4 | `SpiControllerMismatch`'s text said "the reset clock" and "HSI's fastest" | note | **Fixed** (text) |
| A5 | `HseCopyNotSolved` names a `flash_acr` edit only as "a clock field" | note | Kept: the refusal is by name, and the field is one of the clock's |
| A6 | The solver and the judge size `FLASH_ACR` at the crystal's fastest. The word is in force on the HSI (≤ 64,958,976 Hz) until `SWS`; both fall in Table 17's VOS2 1 WS row today | note, latent | A tree under 55 MHz would need the HSI's fastest too. Recorded for any H2/H0 work order |
| A7 | A CSS NMI between `HSECSSON` and the GPIO enables writes an unclocked port; RM0433 states no bus response there | note, unverified | No motor pin is configured in that window. For the bypass bench |
| A8 | An HSE specification with `mcu.foreign_clock` generates a platform with no `hseFrequency` | note | `simulate --foreign` on HSE is `4400`'s (`4240a` F4) |
| A9 | A consistently edited topology (another solve's clock spliced in) passes the topology-only paths | note | The stated residue of §3 |
| A10 | `D3CFGR.D3PPRE` is read by `reset_clock` only with I2C4; on an HSE board SPI6's kernel `rcc_pclk4` depends on it too | note | At reset after any reset, and the twin flags any `D3CFGR` write. Backlog |
| A11 | The poll counts are a minimum time (≥ 1 cycle per poll at the HSI's fastest): a dead HSE halts after **at least** 20 ms, not at most | note | Stated here; the code claims nothing more |

**Attacked and held:**
- the words and order against RM0433 Rev 9;
- 13 synthetic-log probes of the twin's rule (16-bit reads, cut boots, HSI off after `SWS`, rewrites after `SW`, an `HSERDY` read of 0, no `D3CR`, an empty last boot, a select in an earlier boot), each named;
- Kani at head, and a mutant discarding the `PLL1RDY` result, refused;
- INV-07 provenance: the oracle's CR/CFGR words are its own RM-cited constants, not the emitter's;
- the bounds by hand and the `u32` loop, and `HsePollsOverflow` at 4e9 µs;
- the NMI vector and `css_hold` on six specifications;
- all 25 null ELFs byte-identical base to head, built independently;
- every single-field topology edit refused by `validate`, `check pinout` and `hwdef`.

## §6 Mutants (§1.9)

| Mutant | Caught by |
|---|---|
| Drop the `HSERDY` bound (`u32::MAX`) | Kani `hse_start_is_in_order_bounded_and_named` |
| `SW` before the `PLL1RDY` wait | Kani (same harness); the twin's rule on Renode-shaped lines (`judge_boot_holds_the_hse_bring_up_to_the_clock_rule`) and on synthetic traces (`tests/hse_clock.rs`) |
| `SPI123SEL` left at `100` | the twin's rule names it (`tests/hse_clock.rs`; the CLI unit test above) |
| Drop the NMI's TIM2–5 drive | `the_nmi_holds_every_motor_low` |

## §7 Findings

- **F1 · a §7.1 row, proposed (D-3): arm the advanced timers' break.** Set `BDTR.BKE` on TIM1/8/15/16/17 when they drive motors, so that a CSS clock-failure event forces their outputs inactive in hardware (RM0433 §8.5.3, p. 344). The NMI's GPIO drive would then be the second line, not the only one. That needs `BKP`, `AOE` and the idle levels declared, and a twin that models none of it (`4240b` D-3 ruling: not this work order).
- **F2 · `4240v` F7, the reduction proposals:**
  - P1, P6, R1 and R2 live in the IR's VOS rule, P8 in the twin's enable filter, and P7 is the CLI's and svd-hal's `CoreSupplyNotGenerated`. This rewrite touches none of them: declined, with no defect.
  - R4 was declined by ruling (Q2).
- **F4 · a motorless HSE board has no `sys_ck` read-back** (A1). Renode pushes `sys_ck` to SysTick and the timers; the verdict reads it only through motor timing. A gate reading `nvic Frequency` at the run's end would close it (a backlog row).
- **F3 · the bypass bench** (`4240` D7 (c)) measures the real `HSERDY` time and so tests the ruled 20,000 µs. Until then the twin and Kani cannot see it (§2.3).

## §8 Reduction pass

Diff-sized, on the branch's own additions:
- **Taken:** `clock::Startup`, a trait with one implementor, became `Hse`'s two inherent methods. That removes the trait and two imports.
- **Declined:**
  - `kernel_budget` (solver) beside `spi_kernel_budget` (rule): the pair is `RUL-20`'s, the solver's copy and the rule's.
  - The CR words built twice, by the emitter from the header and by the Kani oracle from its RM-cited constants: `INV-07`.
  - The twin's own copies of the five boot words: `belt-and-suspenders:`, out of bounds.

## §9 Gates and CI

- Local, on `3c1799d5`: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean. `cargo test --workspace` reports 147 binaries ok and one failure, `bench_board_tier1`'s package binding, which waits for the write=true dispatch.
- Tier 2 adds one test, so `TIER2_MIN` goes from 200 to 201. Its weight row is 50 s: two boards, each at `sentinel-02-shaped`'s 24.7 s row.

## §10 State

PR #293 (AWAITING-PACKAGE). `main` `25203c58` (`1211a`) is merged and the branch renumbered to 23/15 as its own commit. On the merged tree, against `main` `25203c58`'s baseline, every null ELF and verdict is identical and all 25 KiCad outputs are byte-identical. The bench package waits for the architect's write=true dispatch.
