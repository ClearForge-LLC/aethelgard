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
  - The workflows solve that fixture and generate its hwdef, which is unchanged except for its header's specification sha256.
  - **Correction (D7):** I told the architect that the workflows only solve it. That was wrong. Three generated tier-2 runs boot our own firmware on it, and a declared clock refuses them. They now boot on a copy without the declaration.

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

Each figure was re-pinned from a run, and each moved for one stated reason. Where a figure could be set back, setting it back reproduces the old one.

- **`aethelgard-chaos` golden trace** (`bcd082c2…`, state `ada1b631…`): the version byte only. Set back to 13, the trace hashes to the version-13 golden `25392c25…`.
- **`solve_cli` digests:** schema 17's `schema_version` and solved 11's `foreign_clock: null`. The old file, version 10 and no `foreign_clock` key, give the old digests (checked).
- **`aethelgard-svd-hal` `order_2203`** (`main.rs` `5c770e1c…`, platform `88ab17b6…`, faulted `2a8151a8…`): the spec sha and the platform's clock block. Stripping both gives the old digests (checked).
- **Board crates** (`boards/sentinel-01*`, re-blessed): the spec sha and the clock entries, LPTIMs included.
- **hwdef fixtures:** the header's spec sha only.
- **Motor fixtures** (chaos, evolve):
  - PWM: period 2,499,000 ns, first rise at `PWM_ANCHOR + PERIOD`.
  - DShot600: 1,657/625 ns, lead 1,700 ns, 1,206 rises.
  - Late cases: 559 and 603.
- **`fab/sentinel-01-bench/package/MANIFEST.json`:** derived by the architect's dispatch (F4).

## §2 Measured

### §2.1 ArduPilot at its declared clock, 5 s (**measured**)

- **The run:** `--foreign 5000ms` on `sentinel-02-own`'s platform, with ArduPilot Copter-4.7.2 `a799dfb6` from run 44347566685's `generated-sentinel-02` artifact.
- **Verdict:** `pass`, with the foreign judge, `tool.build` `1.17.0+20260907gitf1dd1b4af.aet1605`, and 89,697,415 instructions.
- **RCC read back:** `RCC_CR` `0x3F003025`, `RCC_CFGR` `0x0000801B`, `RCC_D1CFGR` `0x48`, `RCC_D2CFGR` `0x440`, `RCC_PLLCKSELR` `0x408040`, `RCC_PLL1DIVR` `0x01090231`.
  - These decode to `sys_ck` 400 MHz and timers 200/200 MHz, and both declarations hold.
  - The eight USARTs are `declared_only` at 72 MHz.
- **Witnesses (ns):** identical to 4400f (c) to the ns.
  - U4 identity 161,141,150.
  - U2 identity 385,637,340, data 435,855,540.
  - U7 identity 385,831,960, data 435,812,030.
  - U5 identity 4,342,643,780, first FIFO read 4,393,959,380.

### §2.2 Generated firmware at the solved clock (**measured**, tier 2)

- **DShot600** (`serial-dshot-9600`, `motors-dshot600`): passes, period 1,657 ns and high time 625 ns.
- **PWM** (`motors-pwm`): passes, period 2,499,000 ns, high 1,000,000 ns, at least 40 rises.
- **A board without motors:** its platform moves by the clock block alone. Its verdict moves by the platform's sha256 and the new `tool.build`, and in nothing judged.
  - 4400f measured this on Sentinel-02.
  - Here the derived bench MANIFEST (F4) will show it on Sentinel-01-bench's verdict, diffed against main's.
- **No generated verdict flipped:**
  - On the merged head before D7, 2,457 tests passed and 4 failed: the bench MANIFEST (F4) and D7's three refusals.
  - After D7, those three pass (§6).

### §2.3 F1: the 5 s window under concurrency (**measured**; stop over 900 ‰)

Each run is a full `--foreign 5000ms` ArduPilot boot under the 1 GiB bound, in a second container under `capped-run`.

| Set | Beside | Load (1 min) | Runs | Peak RSS ‰ | Wall |
|---|---|---|---|---|---|
| light | nothing | 1.4–2.1 | 3/3 pass | 792–807 | 11.4–11.6 s |
| busy | a tier-2 suite starting | 9.2→6.2 | 3/3 pass | 800–811 | 11.3–11.6 s |
| **of record** | container A's full tier-2 suite | 2.1–3.5 | **5/5 pass** | **773, 781, 791, 800, 773** (peak 839,756 KiB) | 11.2–11.6 s |

- No run was killed, and none came within 89 ‰ of the stop. The highest was 811 ‰.
- The bound is the boot's own 1 GiB. The container's 8 GiB is never near.

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

- **D7, our own firmware on `sentinel-02-own` boots without the declaration.** The fixture declares ArduPilot's clock, as ruled at 03:22, and the WO refuses a declared clock on a generated run.
  - `twin_tier2`'s `own_board` therefore derives a copy with `mcu.foreign_clock` removed and puts `sentinel-02-own-base.json` beside it, as `ci/foreign-judge.sh` derives its undeclared spec.
  - Cost: the committed `sentinel-02-own-base.json` no longer runs a generated campaign as committed.
  - The alternative was a sibling fixture that carries the declaration. That reverses 03:22 and copies a large fixture.
  - **RULED (ii), accepted (architect, 14:18Z).** A declared clock refused on a generated run of a shared fixture is an input refusal, not a flipped verdict, so it is not the WO's stop. The derived undeclared copy stays.
  - **Its cost, named:** `sentinel-02-own-base.json` no longer runs a generated campaign as committed. Only the derived copy, with the declaration removed, does.

- **D8, F5 as ruled: the bench specification moves to schema 17, and no walker skips a specification.**
  - **The bump.** `examples/sentinel-02-bench.json` moves to schema 17, its version line only. Its sha256 goes from `25597b3d…f4c3` to `fe89ba38…97fe`.
  - **3304's evidence, re-derived** (**measured**: this branch's CLI in the pinned image, `generate` then `build`):
    - **The ELF does not move:** sha256 `375fd31a…9c02`, 3304's.
    - **The board crate's input hash moves,** from `7127d94a…f55b` to `4a5571e2…c75c`. Diffed against 3304's crate (`~/.cache/aet-3304/runs/gen1/board`), two things differ:
      - the specification sha256 in the header comments of eight files;
      - the platform's clock block (§1.1), 29 update entries in `sentinel-02-bench.repl`.

      Neither reaches the firmware's bytes.
  - **The walkers.** `silicon_rules`, `ledger_tier1`, `hwdef_tier1` and `silicon_words` walk `examples/`, and some walk more. They used to skip a specification that did not parse, and one `solve` refused for its version.
    - Each now calls one rule (`tests/common/spec.rs`, and a copy in `silicon_words`, which cannot reach that file). A JSON object with both `mcu` and `board` is a specification: every committed one that parses has both, and no other committed JSON does.
    - It must parse and carry the current schema version, or the walker fails naming it.
    - Refusals for any other reason keep their skip: those are the deliberately invalid fixtures, and each walker states its scope as the specifications that solve.
    - No other test walks specifications: the remaining walkers read sources, crate trees or scenarios.
  - **The mutant:** a schema-16 copy of `sentinel-01-minimal.json` left in `examples/`. It is killed by all four walkers, each naming the file and "a committed specification at another schema version" (five tests).

## §4 Findings

| # | Finding | For |
|---|---|---|
| F1 | `.github/workflows/ci.yml` L69 calls its crate list "also the order docs/releasing.md publishes in", and lists `aethelgard-twin` before `aethelgard-chaos`. `docs/releasing.md` L35–36 does the same. The twin now depends on chaos, so chaos must publish first. `cargo package` in one invocation is unaffected, but a publish in that order would fail. Both files are outside this fence. | architect: a docs/workflow edit |
| F2 | The architecture §2.27 figures move: ArduPilot at 1.00000×, the window at 5 s, and the witness times (U4 0.1611 s; the barometers 0.3856/0.4359 s; the ICM 4.343/4.394 s). | the docs PR |
| F3 | A foreign firmware clocked from HSE can never decode: `CrystalNotSupported` holds `mcu.hse_hz` at 0, so the run fails `foreign_clock_undecodable`, which fails closed. | backlog, with the crystal part |
| F5 | `examples/sentinel-02-bench.json` (3304, `cd43b207`) was at specification schema 16, which this branch refuses. A walker that cannot use a specification skipped it, so a committed board fell out of the rule checks and no test failed (`INV-06`). | **ruled 14:39Z, built (D8)** |
| F4 | **Done.** `fab/sentinel-01-bench/package/` is from the architect's `write=true` dispatch, run 44544348287 on `afb91036`, artifact 14542655254, committed byte-for-byte. Of its 17 files, two moved and the other 15, the ELF and every fab file among them, are byte-identical. **`MANIFEST.json`:** `specification_sha256` `8e069b3e…` → `4846be8b…` (`examples/sentinel-01.json`'s `schema_version` 16 → 17, its only change); `topology_sha256` `ffb85c0e…` → `116ae1d9…` (solved schema 10 → 11, with `foreign_clock: null`); `verdict_sha256` `3b585ef4…` → `3e0bc749…` (the verdict names both hashes and now records `tool.build`); and `FAB-NOTES.md`'s hash. **`FAB-NOTES.md`:** line 4 names the specification's sha256. `bench_board_tier1` passes with it. | done |

## §5 Adversarial pass (full)

Three reviewers, each on one or two of the WO's §5 lenses, on `9c98c8f8`. Each ran in a git worktree of its own, with tier-1 suites and mutants on the host, and no container. Their findings, and what was done with each:

| # | Lens | Finding | Severity | Outcome |
|---|---|---|---|---|
| A1 | a frequency typed in | LPTIM1–5 stayed at the stock 32 MHz and were not named; the completeness test saw only three model kinds | medium | **fixed**: (a) clocks them at `rcc_pclk1`/`rcc_pclk4` (§8.7.21–§8.7.22); (c) names them in `boot.clock.stock`; the test now covers every instance with a `frequency:` line, and holds the watchdog (LSI) apart by name |
| A2 | a frequency typed in | SysTick's stock 480 MHz and the watchdog's 56 kHz were not stated | low | **named** (§1.6): SysTick follows the RCC (measured, 4400f F4); no judge reads the watchdog |
| A3 | the decode | APB1 and APB2 were indistinguishable: every fixture had equal buses, and HSIDIV was never exercised (mutants M4, M6 and M8 survived) | medium | **fixed**: a split-bus declaration, a split `D2CFGR`, and HSIDIV 01 tests (M4, M6 and M8 killed) |
| A4 | the decode | Renode's `SWS` mirrors `SW` whether or not the source is ready | low | **fixed**: the decode requires the source's ready flag (`NotReady`) |
| A5 | the decode | `DIVM1` at its reset 32: Renode's prescaler stays 1 until the field changes | low–medium | **fixed**: refused as `Unmodelled` |
| A6 | the decode | no PLL range check: 4.096 GHz decoded | low | **fixed**: a 1–16 MHz reference and a 150–960 MHz VCO (§8.7.13), `OutOfRange` |
| A7 | a contradicted declaration passing | removing the clock check from the judge survived every tier-1 test; only CI's ArduPilot case caught it | medium | **fixed**: `a_foreign_clock_the_rcc_registers_contradict_fails_by_name` (mutant Mb killed) |
| A8 | a non-pinned Renode | a plain `simulate --platform` load judged on any build and version | medium | **fixed** (D4): every `simulate` is pinned; `a_load_on_another_renode_build_is_refused_by_name` (Mk killed) |
| A9 | a non-pinned Renode | doctor's build pin, `tool.build` recording and a boot-level refusal had no test | medium/low | **fixed**: a doctor case for the stock build (Md killed), `tool.build` asserted (Mg killed), and `a_boot_on_another_renode_build_is_refused_by_name` |
| A10 | the declaration's rules | the second USART mux group and the topology-side check were untested (mutants survived) | low | **fixed**: tests added (Me and Mf killed) |
| A11 | replay | the witness arm's "no static timing check" repeats the final count check, and kind 25's F is pinned to 64 MHz by `silicon_check` | low (reduction) | **kept**: the early return is the lookup's own guard, and the final check keeps its own message; noted for the reduction pass |
| A12 | the verdict | after a bound kill, `boot.clock.checked` lists both buses with `decoded_hz` null and `holds` false | info | **kept**: the run fails by its bound; nothing reads as checked |
| A13 | the declaration | a foreign firmware on HSE cannot decode (`mcu.hse_hz` is held at 0) | info | finding F3 |

**What held,** across all three reviewers:
- No judge predicts the nominal 1,656.25 ns, and the silicon verdicts use `ARR + 1` exactly.
- Replay refused every forged or missing kind 25, a non-64 MHz F, the old 4 ns periods, and kind 26 before kind 25. `realized_ns` cannot overflow.
- Every field position, encoding and page of the decode matches RM0433.
- The emitted text is deterministic and the update entries' syntax is valid.
- `campaign` reaches `ready()` with `foreign: false`. `--for` and `--foreign` cannot be combined.
- Every reader of an archive goes through the version check.

**The guards' mutants** (`~/.cache/aet-4400g/mut/run.py`; each applied alone, its suite run, then reverted with `git checkout`). Killed:
- **(a) `ForeignClockUndeclared` disabled, and (b) `ForeignClockOnGeneratedRun` disabled:** by `boot_tier1` `a_clock_never_crosses_its_run_kind`.
- **(c) the RCC compare forced to hold:** by `renode::foreign::tests`. The check cut from the judge: by `boot_tier1` (Mb).
- **(d) the build pin disabled:** by `campaign_tier1`'s 2205 test. Doctor's pin forced: by `doctor_tier1` (Md). A load not pinned: by `tools_tier1` (Mk).
- **(e) the trace reader accepting 13:** by `archive` `an_unknown_version_is_refused`. `TRACE_VERSION` left at 13: by `golden`.
- **The decode:** HSIDIV ignored (M6), D2PPRE swapped (M8), the ready flag ignored (Mh), DIVM1 32 accepted (Mi) and the VCO range dropped (Mj).
- **The declaration:** the second mux group dropped (Me), and the topology's check dropped (Mf).
- **Elsewhere:** `tool.build` not recorded (Mg), and the declared APB buses swapped (M4).

**Before the fixes:**
- M4, M6, M8, Mb, Md, Me, Mf, Mg and Mk survived.
- `realized_ns` rounding down, `witness_consistent` dropping `high_ns` and the mux-split check dropped were already killed (reviewer A).

## §6 Gates

- **`cargo fmt --all --check`** and **`cargo clippy --workspace --all-targets --locked -D warnings`**: clean on the final head.
- **The whole workspace in the pinned image,** `--features aethelgard-cli/with-tools`, on the merged head `77bda410`: 2,457 passed and 4 failed.
  - The four failures: the bench package's MANIFEST (F4), and the three generated runs D7 fixed.
  - `fab_tier1` passes in the image, so its host failure is the host's target directory.
- **After D7** (`twin_tier2`, the image):
  - all 87 pass; the three runs D7 fixed are listed below;
  - `the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed`, `without_the_cited_wait_each_data_read_answers_its_reset_value` and `the_fixture_s_campaign_archives_each_configuration_and_replays` pass (the campaign runs from the derived spec's directory, since `campaign` refuses a base outside it).
- **The whole workspace again on the final head `4bb2485f`** (the image, with-tools): 2,460 passed and 1 failed. The failure is the bench package's MANIFEST (F4) alone.
- **`ci/foreign-judge.sh /cache/boards`** (the image, `a799dfb6` and `noimu`): every case holds, exit 0.
  - positive at 5000 ms;
  - window-before-u5 at 4300 ms and window-before-u5-data at 4370 ms;
  - undeclared (`FirmwareOutsideMemory`), clock-undeclared and clock-on-generated-run (each exit 3);
  - clock-contradicted (two `foreign_clock_contradicted`);
  - no-spi2-rx-dma and no-u5-model (`PlatformNotEmitted`);
  - no-imu-line.
  - Its first full run caught the script's own defect: a case named `undeclared` wrote its verdict over the derived spec of the same name. Derived specs are now `spec-<how>.json`.
- **CI on `c07c6ebd`** (run 44553437741): green, 27 checks passed and 4 skipped (publish, quickstart, and `bench-package`, which runs only on main's push). `tier2_tests_passed=190`, floor 190, so `TIER2_MIN` is exact. The tools shards ran 28–46 min, all under 55.
- **`TIER2_MIN`:** no tier-2 test was added or removed against main, so it stays at 190 and no workflow moves.
- **Mutants:** 12 of the guards' mutants and (a)–(e) killed (§5).
- **Reduction pass (diff):** −18 net · 2 duplicates removed · 1 dead path removed · 0 protected kept · 2 findings below.
  - **Removed:**
    - `ClockSource` and `clock_source()`, a second mechanism for `foreign_clock.is_some()`;
    - the `--foreign` help's stale "at most 4000ms", which now says "the foreign window", with its MCP snapshot re-blessed;
    - `foreign-judge.sh`'s six `5000ms`, now one `window`.
  - **Kept:**
    - `ClockReport.source`, a verdict key;
    - A11's two replay guards, which catch different absences.
  - **Findings, kept with their reasons:**
    - `unclocked()`'s seven names restate `clocked()`'s seven entries in another order. One list would reorder the emitted platform or the verdict, and the timer-routes test pins both.
    - `foreign_clock_values` re-states the USART mux groups that `serial::kernel_hz` states for the reset clock.
