# AET-WO-4240v — The generated firmware at VOS2: a declared supply configuration, PWR_CR3 written once, ACTVOSRDY, VOS Scale 2, VOSRDY, and the clock limits judged per VOS

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`56639a89` when written; verify live HEAD). **It is written after `4400g` (#238) merges** (#240 ruling, comment 7014986899, D10).
**Branch:** `wo/aet-4240v-vos2`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4240` builder, reused), with a full adversarial pass.
**Phase:** P4, the flight path. It is the first slice of `4240`'s split (D10 item 0), ahead of `4240a`/`4240b`/`4240c`. Written 2026-11-19. **Cadence: build.**

**Grounds:**
- **The finding: F-VOS** (`AET-WO-4240.FEEDBACK.md` §10, L648–686). It was ruled a real finding on the bus (2026-11-19 15:20Z) and on #240.
  - Every generated firmware runs `pclk` and its APB, USART, I2C and SPI4–6 kernels at 64 MHz at the reset scale, VOS3. The limit there is 50 MHz:
    - DS12117 Rev 10 Table 111 (continued), p. 210, rev V: `fPCLK` 50 at VOS3, with footnote 6;
    - RM0433 Rev 9 Table 60, pp. 355–356.
  - VOS3 is the reset scale (RM0433 §6.6.2, p. 280). `PWR_D3CR` resets to `0x0000 4000` (§6.8.6, p. 310).
  - Nothing in the engine reads or sets VOS: no `PWR` write exists in `aethelgard-svd-hal`. Roadmap §7.1 has the F-VOS row (`docs/roadmap.md` L845).
- **The sequence** (FEEDBACK L667–671, the builder's correction on the bus):
  1. `PWR_CR3`'s supply configuration first. Its lower byte "is written once after POR and shall be written before changing VOS level or ck_sys clock frequency" (RM0433 §6.8.4, p. 308). An invalid `LDOEN`/`BYPASS` pair "will lock the register … The system shall be power cycled".
  2. A bounded wait for `PWR_CSR1.ACTVOSRDY`. Until it rises, "write accesses to the RAMs are not permitted and VOS shall not be changed" (§6.4.1, p. 262).
  3. `PWR_D3CR.VOS` = `10`, Scale 2.
  4. A bounded wait for `VOSRDY`.
  - Each wait's timeout ends in a named boot word.
  - `PWR_CR3` "is reset only by POR" (p. 307): `0x0000 0006` on rev Y, `0x0000 0046` on rev V. Its lock survives a warm reset (FEEDBACK L672–673).
- **The ruling on #240** (comment 7014986899):
  - D10: `4240v` comes first, with its own specification bump.
  - S14: judge VOS limits "at the source's fastest frequency, not at nominal" (`RUL-14`: no nominal stands in for a worst case). The generated trees clear it, 64 MHz against 75 and 100 MHz. A foreign firmware at a boundary is "reported as a named condition, not refused".
  - F-CITE strict: a figure is cited only with a document number and revision. A figure without them is marked uncited, and no recommendation rests on it.
  - D1: at H1 the flash row is written before the switch. That is `4240b`'s; this slice has no switch [D-7].
- **The bench:**
  - `3304.bench.md` L14–19 (the architect's note): the run is outside ST's guarantee at VOS3, and the bench is not held for it.
  - `AET-WO-3403.md` L33 (F14): Sentinel-01's B2 records `PWR_CR3`, `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR`, labelled `4240`'s row B-2 (FEEDBACK L93–99).
  - The Nucleo's die is rev V, measured: `DBGMCU_IDC` `0x20036450` (`3302.bench.md` L165).
- **The code seams on `main` `56639a89`** (re-check every line after `4400g` merges):
  - **`aethelgard-ir`:**
    - `lib.rs:81`: `SPECIFICATION_SCHEMA_VERSION` 16 (17 after `4400g`).
    - `lib.rs:288–303`: `Mcu` (`hse_hz`, `bor_level`, `supply_source`, `reset_switch`).
    - `lib.rs:447`: `SOLVED_SCHEMA_VERSION` 10 (11 after `4400g`), and its reader fallback at `lib.rs:984` (an old file gets `SolvedClock::reset()`).
    - `lib.rs:3095–3112`: `validate_declared`.
    - `lib.rs:4019–4023`: `clock != SolvedClock::reset()` gives `UnsupportedClock`.
    - `lib.rs:2728–2731`: `validate` feeds the reset clock.
    - `motor.rs:92–135`: `SolvedClock` and `reset()`.
    - `silicon.rs:1–28`: its independence statement. `silicon.rs:41–56`: its own HSI copy, +14,984/−25,894 ppm (DS12117 Rev 10 Table 36 rev Y = Table 133 rev V).
    - `silicon.rs:60–77, 306, 671`: its own reset kernels.
    - `solve.rs:293`: `clock: SolvedClock::reset()`.
  - **`aethelgard-svd-hal`:**
    - `board.rs:171–193`: the generator's own HSI copy and `RESET_HZ`.
    - `board.rs:200–310`: `check_silicon`, run first by `generate`.
    - `board.rs:1685–1693`: `RESET_CLOCK_FIELDS`; `:1700` `clock_fields`.
    - `board.rs:2490–2506`: `ResetClock` and `NotResetClock`; `:2342` `enum Block` (`Rcc`, `Flash`, …).
    - `board.rs:3158–3200`: `emit_reset_clock`.
    - The functions that take the token: `board.rs:2740` (I2C `init`), `:2845` (SPI), `:3142` (`start_outputs`), `:3236` (`select_spi_kernels`), `:3350` (USART).
    - `board.rs:4714–4750`: the Kani oracle, "RCC is not before plus exactly the solved clock gates and selections" (`:4720`, `:4745`). `board.rs:5365`: `reset_clock_is_ok_exactly_at_reset`.
    - `board.rs:3388`: `receive(r, polls)`, the bounded-wait precedent. `board.rs:6192–6196`: transfer proofs at a two-read bound.
    - `board/firmware.rs:32`: `BOOT_MAGIC` `0x2100_B007`. `:686–694`: the reset-clock check, called only `if plan.needs_clock()`. `:900`: the boot-word write. `:939–946`: `CLOCK_CHECK`.
    - `board/ledger.rs:1–23`: the field list.
  - **The ST header:** `data/st-cmsis-device-h7/stm32h753xx.h` (`v1.10.7`, sha256 `4f6def43…`):
    - `PWR_BASE` L2241 (`0x5802 4800`); `PWR_TypeDef` L1213–1224 (`CSR1` +0x04, `CR3` +0x0C, `D3CR` +0x18);
    - `PWR_CSR1_ACTVOS` 15:14 and `ACTVOSRDY` 13 (L14460–14466);
    - `PWR_CR3` `SCUEN` 2, `LDOEN` 1, `BYPASS` 0 (L14511–14518). **No SMPS field** for this part (L14492–14518);
    - `PWR_D3CR` `VOS` 15:14 and `VOSRDY` 13 (L14552–14558).
  - **`aethelgard-twin`:**
    - `serial.rs:1096–1124`: `RccRule`; `:1128` `rcc_rule`.
    - `boot.rs:36`: `BOOT_MAGIC`. `boot.rs:657–680`: `Image`, including `unobservable`.
    - The PWR stub, `tests/renode-1.17.0/platforms/cpus/stm32h7.repl` L728–745, a `Python.PythonPeripheral`. `CSR1` reads `0x6000` (`ACTVOSRDY` 1, `ACTVOS` `01`). `CR3` reads `0x0400_0000` (supply bits 0). `D3CR` reads `0x2000` (`VOSRDY` 1, `VOS` `00`). Every write is dropped (`spike/4240/reads/renode-ardupilot.md` L62–70).
  - **`aethelgard-cli`:**
    - `ledger.rs:418–597`: the RCC rows. An unclassified field is a hard error.
    - `renode.rs:4024`: the composition's `LogPeripheralAccess` lines.
  - **CI:** `TIER2_MIN` 190 (`.github/workflows/ci.yml` L390); `bench-package` L432–468 (`write` dispatch); the planner's guard against heavy unweighted tests (`ci/shard.sh` L211–252, `1910` D6).

> **What this is:**
> - **a declared supply configuration**, `mcu.core_supply` [D-1], with a cited source in every committed specification, and the specification schema bumped;
> - **one rule in `aethelgard-ir`** that judges DS12117 Table 111 and RM0433 Table 60 per VOS at each clock's fastest frequency, for every bus clock and every enabled kernel, on the reset HSI tree;
> - **the generated bring-up:** `PWR_CR3` written once and read back, a bounded `ACTVOSRDY` wait, `VOS` Scale 2, a bounded `VOSRDY` wait, each failure a distinct named boot word, and a token without which no peripheral is initialized;
> - **the Kani oracle, the twin's PWR rule and the verify ledger** for those words;
> - **every generated ELF, golden and committed board re-pinned** from measured runs, with the bench package re-derived by the architect's dispatch;
> - **bench row B-2 extended,** predicted before the bench.
>
> It is NOT:
> - **the HSE, the PLL, a crystal part, `FLASH_ACR` writes or kernel-select changes.** Those are `4240a`/`4240b`/`4240c`;
> - **VOS1 or VOS0 for the generated firmware.** The rule knows every scale; the generator writes Scale 2 only;
> - **a declared silicon revision** (F-REV, with `4240a`'s D4);
> - **USB, the backup domain, or any other `PWR` field;**
> - **a change to the vendored Renode platform** or any Renode source. An override goes in the emitted platform only [D-3];
> - **the bench run itself,** which is Scotty's.

**Ruled at writing (the architect, 2026-11-19).** Each recommendation of the drafting pass is accepted:
- **D-1:** `mcu.core_supply` = `{mode: ldo | bypass, source}`, required. `bypass` validates and solves, and `generate`, `emit` and `simulate` refuse it as `CoreSupplyNotGenerated`.
- **D-2:** the sequence runs first in `main`, in the Kani-proved crate.
  - State the residue that the reset handler writes RAM before `ACTVOSRDY`.
  - The bench row B-2 reads `ACTVOSRDY` at reset so silicon answers it.
- **D-3:** keep the `PWR_CR3` read-back, the only guard against a register locked since power-on. The emitted platform overrides the PWR stub. If Renode cannot, stop and say so.
- **D-4:** `foreign_clock` gains `vos` in this slice. A foreign clock over its limit is reported as a named condition (`vos_limit_at_boundary`), never refused.
- **D-5:** `3304`'s bench flashes its ELF by sha256, from its commit. The ELF is held with `SHA256SUMS` on Furnace.
- **D-6:** whichever of `3403` and `4240v` merges second re-predicts Sentinel-01's B2 row. The boards are flashed with the package ELF current on the day they arrive.
- **D-7:** `FLASH_ACR` is not written in this slice. Its reset 7 WS is judged against RM0433 Table 17's VOS2 row.

## 1. Build

0. **Predict, then build.** Commit a prediction table to FEEDBACK before the first run, labelled measured, read or inferred, as `4240` did:
   - which ELFs, goldens, platforms and fixtures move;
   - the count of Kani harnesses per board, before and after;
   - the new ledger rows and their classes;
   - the B-2 words of §1.9;
   - which `examples/` walkers fail before the specifications move.

1. **The declaration (`aethelgard-ir`; `INV-01`, `INV-06`).**
   - **`mcu.core_supply`: `{"mode": "ldo" | "bypass", "source": "<citation>"}`** [D-1]. It is required. A missing key is refused as ``missing field `core_supply` ``; `null` and any other mode are refused by name.
     - `source` takes `4400g`'s `foreign_clock` shape: 1–200 bytes of printable ASCII.
     - A source is cited only with a document number and revision (F-CITE strict). Write that rule into the field's documentation.
   - **What each mode means.** Read RM0433 Rev 9 §6.8.4 (pp. 307–308) and Table 34 at source, and cite each bit:
     - the `LDOEN`/`BYPASS`/`SCUEN` value each mode writes;
     - why no other pair is offered: an invalid pair locks `CR3` until a power cycle.
     - The H753 has no SMPS (header L14492–14518, and RM0433 Table 34's row for this part, read at source).
   - **`bypass` is a true declaration the engine does not build** [D-1].
     - `validate` and `solve` accept it.
     - `generate`, `emit` and `simulate` refuse it as `CoreSupplyNotGenerated`, by name: no emitted board draws an external VCORE, and under bypass the VOS is the external supply's (`VOSRDY` "forced by hardware to 1 when the Bypass mode is selected", §6.8.6, p. 310). The precedent is `4240` D10's `HseNotGenerated`.
   - **The schema.**
     - `SPECIFICATION_SCHEMA_VERSION` goes to **18** (after `4400g`'s 17), by the numbering rule at `lib.rs:42–80`. `AET-WO-2304b` also takes a bump: whichever merges second merges `main`, takes the next number and re-pins.
     - `SolvedTopology` gains `core_supply`, and `SolvedClock` gains `vos` (§1.3). `SOLVED_SCHEMA_VERSION` goes to **12** (after `4400g`'s 11), with the same rule against `2304b`.
     - **No fallback supplies a default** for an older solved file. `lib.rs:984`'s `reset()` path must not hand an old file a `vos` or a supply. An older file is refused at its version, and a test shows it.
   - `removing_any_key_of_any_object_is_refused` (`ir/tests/validate.rs:1075`) reaches the new keys through `full_document`.

2. **Every committed specification declares, with its source.** Commit a FEEDBACK table with one row per file: path, mode, source, and how the source was read.
   - **Boards the engine emits:** `examples/sentinel-01.json`, `sentinel-01-minimal.json`, `invalid-zero-domain.json` (still refused for its own reason), and every fixture under `crates/**/tests/fixtures/` and `crates/aethelgard-cli/tests/fixtures/`. They declare `ldo`.
     - **The source** is the board's own support group: DS12117 Rev 10 §3.5.1 p. 27 ("VCAP is VCORE … from the internal regulator"), Figure 65 p. 206, and Table 113 p. 211. That is the MCU row `power-parts.json` L1073–1077 and L1187–1198: 2 × 2.2 µF from each VCAP pin to ground, with no external VCORE.
     - Cite it with the revision. If the parts row lacks it, say so.
   - **`examples/sentinel-02-bench.json` (NUCLEO-H753ZI, MB1364).**
     - Its mode comes from UM2407 Rev 6, read at source: the copy is held at `~/.cache/aet-3304/ds/um2407-rev6.pdf`, sha256 `b493e14f…a4cd`. Cite the section and page.
     - **If UM2407 does not state the H753ZI's supply configuration, that board is a §7 stop.** The MB1364 schematic is not held (`4240` FEEDBACK L56). Never infer `ldo` from the part or from ArduPilot's hwdef.
   - **`sentinel-02-own.json`** (ArduPilot on the emitted Sentinel-02 board): `ldo`, from the board's support group as above.
     - ArduPilot writes `LDOEN | USB33DEN` itself (`4400c` FEEDBACK L30). That is the firmware's choice, not the board's, and it is not the source.
   - **Solved fixtures** (`aethelgard-hwdef`, `aethelgard-pincheck`, `svd-hal/tests/ui/*/topology.json`) are re-derived by `solve`, never hand-edited.

3. **The rule: VOS limits at the fastest frequency (`aethelgard-ir`; `RUL-14`, `RUL-20` (1)).**
   - **Home:** `silicon.rs`, or a `clock` module beside it. Choose, and say which in FEEDBACK. It shares no function, constant or table with `solve.rs`, `motor.rs`, `serial.rs` or `aethelgard-svd-hal` (`silicon.rs:5–16`). It computes every clock itself, in exact integers (`u128`, no floating point).
   - **The limits, each written with its citation:**

     | Clock | VOS0 / VOS1 / VOS2 / VOS3 (MHz) | Cite |
     |---|---|---|
     | `fCPU` (`rcc_c_ck`) | 480 / 400 / 300 / 200 | DS12117 Rev 10 Table 111 (cont.), p. 210, rev V |
     | `fHCLK` | 240 / 200 / 150 / 100 | same; footnote 5, `TJmax` 105 °C at VOS0 |
     | `fPCLK` 1–4 | 120 / 100 / 75 / 50 | same; footnote 6 |
     | TIM1–8, 12–17 kernel | 240 / 200 / 150 / 100 | RM0433 Rev 9 Table 60, pp. 355–356 |
     | SPI1, 2, 3 | 200 / 200 / 150 / 100 | same |
     | SPI4, 5; SPI6 | 125 / 100 / 75 / 50 | same |
     | I2C1, 2, 3; I2C4 | 125 / 100 / 75 / 50 | same |
     | USART1, 6; USART2, 3, UART4, 5, 7, 8 | 125 / 100 / 75 / 50 | same |
     | LPUART1 | — / — / — / 50 (FEEDBACK L651) | same: read VOS0–VOS2 at source |

     - Re-read every figure from the page images (`spike/4240/reads/rm0433-ds12117.md` L121, L166–172 are the spike's reads).
     - **Table 111's rows are rev V only.** Rev Y has no frequency rows (F-REV; reads L293). No revision is declared. So the verdict carries a named condition, `bus_limits_rev_v`, and the kernel limits hold on both revisions through Table 60.
     - A row the builder cannot cite with a document number and revision is marked uncited and judges nothing (F-CITE strict).
   - **What is judged:**
     - every bus clock, always;
     - the kernel of every peripheral the solved topology enables, by the rule's own reading of its select: today's `SPI123SEL` `per_ck`, `SPI45SEL`/`SPI6SEL` `hsi_ker_ck`, the USART and I2C `pclk` at reset (`silicon.rs:60–150, 306, 671`), and TIM by RM0433 Table 57 with `TIMPRE`.
   - **At its fastest** (#240 S14): each nominal is raised by the source's fast ppm, by the rule's own copy (`silicon.rs:47` `fast_hz`). On the reset tree, 64 MHz becomes 64,958,976 Hz. A clock passes only if its fastest is at or under its limit.
   - **`FLASH_ACR` at reset** [D-7]:
     - `LATENCY` 7 and `WRHIGHFREQ` `11` (reset `0x37`, RM0433 p. 201) are judged against Table 17 (p. 160): VOS2 with `hclk` in ]55; 110] MHz needs 1 WS and `WRHIGHFREQ` `01`.
     - It holds: p. 160 calls the default 7 WS "a safety margin". Nothing writes it.
   - **Refusals:** `VosLimitExceeded { clock, fastest_hz, limit_hz, vos, cite }`, one per clock, by name.
     - `SolvedTopology::validate` judges it. The equality at `lib.rs:4022` becomes "the reset RCC tree at `vos` Scale 2", the only tree this engine generates. Any other `vos` is `UnsupportedClock`, as now.
     - `check_silicon` (`board.rs:200`) runs it first, so a refused tree is never generated.
   - **Tests at every scale:**
     - the reset tree at VOS3 is refused, naming `fPCLK` 1–4 and each enabled USART, I2C and SPI4–6 kernel at 64,958,976 Hz against 50 MHz;
     - at VOS2 it passes, with margins of 75 and 150 MHz;
     - one exact boundary case each way: fastest = limit passes, limit + 1 Hz fails.
   - **A foreign firmware** [D-4]:
     - `mcu.foreign_clock` gains `vos`: `{value, source}`, with values `scale0` to `scale3`.
     - The foreign judge applies the rule at the run's end to `4400g`'s decode (`SolvedClock::from_rcc`: `sys_ck`, `hclk`, the APB timers) and to the declared USART kernels.
     - A clock over its limit at its fastest is the named condition `vos_limit_at_boundary`, never a failure. Example: ArduPilot's 400 MHz at VOS1 on the HSI, +14,984 ppm. A clock the decode does not reach is named as not judged.
     - `sentinel-02-own.json` declares `scale1` from `Copter-4.7.2` `stm32h7_mcuconf.h` L110 (`STM32_VOS = STM32_VOS_SCALE1`, read in `spike/4240/reads/renode-ardupilot.md` L237). Re-read it at source.

4. **The bring-up (`aethelgard-svd-hal`; `INV-02`, `RUL-04`, `RUL-20`).**
   - **A `Pwr` block** from ST's header (base, layout, fields), never typed in. It joins `Block` (`board.rs:2342`) and the board's singletons.
   - **The words are the generator's own,** computed from the header's bit positions and the declaration. It never calls the §1.3 rule for a word it emits. The oracle's expected words come from the rule (`board.rs:203–209`'s pattern), so the two are held apart.
   - **The sequence**, one board-crate API that consumes the `Pwr` singleton, so it runs once:
     1. **`PWR_CR3`:** read it, then write the declared supply bits. Write every other bit as read: RM0433 keeps reserved bits at their reset value, and rev V's resets to `0x46` (read §6.8.4 for bit 6). Read it back.
        - **Supply bits other than the declared ones** mean another writer locked `CR3` since POR, and the lock survives a warm reset (FEEDBACK L672). That is boot word `SUPPLY_NOT_DECLARED`. Never a second write.
        - Read back equal to the declared bits, it continues, whether this write or an earlier one set them [D-3].
     2. **`ACTVOSRDY`:** wait at most `polls` reads of `PWR_CSR1`. A timeout is `ACTVOSRDY_TIMEOUT`.
     3. **`PWR_D3CR`:** `VOS` = `10` (`0x0000_8000` as a whole word, FEEDBACK L345; the bring-up writes no other `D3CR` field).
     4. **`VOSRDY`:** wait at most `polls` reads of `PWR_D3CR`. A timeout is `VOSRDY_TIMEOUT`.
   - **The wait bounds.**
     - `polls` is a parameter, as `receive(r, polls)` is (`board.rs:3388`), so Kani proves it at a small bound.
     - The firmware's bound is an engine constant. Its documentation says whether DS12117 Rev 10 states a settling maximum for either bit (read at source). If none is stated, it says the bound is an engine choice, set generously, as `ECHO_POLLS` is.
   - **Three named boot words,** each distinct from `BOOT_MAGIC` `0x2100_B007`. Propose the values in FEEDBACK, for example `0x2100_B0F1`–`F3`.
     - Each is written to word 0, then the firmware parks, with nothing else written.
     - The twin's judge holds its own copies (`boot.rs`, as for `BOOT_MAGIC`). `tests/board.rs` checks that the two copies agree.
     - A failed boot names its word: `boot_supply_not_declared`, `boot_actvosrdy_timeout`, `boot_vosrdy_timeout`.
   - **The token.**
     - The sequence returns a `Scale2` token, which `Rcc::reset_clock` now takes. So no `ResetClock`, and no I2C, SPI, USART, timer or kernel-select call (`board.rs:2740, 2845, 3142, 3236, 3350`), can precede it. A test in `tests/ui` shows the compile failure.
     - `ResetClock`'s documentation says the RCC tree is at reset and the core is at VOS2.
     - `RESET_CLOCK_FIELDS` does not change: no RCC field moves.
     - The reset-clock read still comes before every RCC write.
   - **Placement** [D-2].
     - The sequence is the first hardware access in `main` (`firmware.rs:686–694`), ahead of `Board::take`'s first register access, the reset-clock read and the `FLASH_OPTSR_CUR` read.
     - **Every** generated firmware runs it, including one whose plan has no clock (`needs_clock()` false, `board.rs:1157`). Measure whether any committed board is such a plan.
     - The RAM writes before it (the reset handler's `.bss`/`.data`, the stack) are a residue, stated in the module documentation and in the ledger (§1.7).
   - **`FLASH_ACR` is not written.** The kernel selects do not change, and no `PSC`, `ARR`, `CCR`, `BRR`, `MBR` or `TIMINGR` moves. Any solved word that moves is a §7 stop.

5. **Kani (`INV-02`, `INV-03`).**
   - **The oracle** (`board.rs:4714–4750`) gains a `pwr` block in the harness image, with a clause: "PWR is not before plus exactly the declared supply and VOS Scale 2". `after.pwr` equals `before.pwr`, except `CR3`'s three supply fields, set to the rule's word, and `D3CR.VOS`, set to `10`. In a part, the `rely` mask covers `PWR` as it covers `RCC`.
   - **A new harness**, `pwr_scale2_is_once_in_order_and_bounded`, on a spy with nondeterministic reads and a bound of two polls (the transfer-proof precedent, `board.rs:6192–6196`). It proves:
     - the writes are exactly `[CR3, D3CR]`, in that order, with the rule's words;
     - no write follows a fault;
     - each fault returns its own name, and `Scale2` comes back only when `CR3` read back as declared and both bits read 1 within the bound;
     - no block but `Pwr` is touched;
     - `kani::cover!` reaches each of the four outcomes.
   - **`reset_clock_is_ok_exactly_at_reset`** (`board.rs:5365`) takes the token and still proves what it proves.
   - **Measure the added proof time and peak memory** on Sentinel-01 and on the largest fixture. `verify` already needs 16 GiB (`3304` F3, `4201` F4).

6. **The twin (`INV-07`, `RUL-20` (2)).**
   - **A `PwrRule`** beside `RccRule` (`serial.rs:1096–1124`), built from the topology and the §1.3 rule's words, never from the generator. It allows:
     - exactly one write to `PWR_CR3`, with the supply bits as the rule gives them and the other bits as read;
     - then exactly one write to `PWR_D3CR`, with `VOS` `10`;
     - both before any RCC enable write.
     - Any other `PWR` write, a second write, or the reverse order is a named finding.
   - **The log.** Add `sysbus LogPeripheralAccess sysbus.pwr true` to the composition (`renode.rs:4024`'s pattern). **Measure that Renode logs a `PythonPeripheral`'s accesses.** If it does not, the override below logs them itself. If neither works, it is a §7 stop.
   - **The stub** [D-3]. Stock (`stm32h7.repl` L728–745), `ACTVOSRDY` and `VOSRDY` always read 1, and `CR3`'s supply bits always read 0.
     - Under D-3 (a), the emitted platform overrides `pwr` with a script of the same shape. It returns `CR3`'s POR value (rev V `0x46`, the Nucleo's measured die) until the first write, then stores that write and ignores later ones. `D3CR.VOS` is stored, and every other offset answers as the stock stub does.
     - It is written from RM0433, never from the generator. Measure that an update entry can replace the script, as `4400g`'s `frequency` entries do.
     - ArduPilot's foreign run must stay byte-identical in its witnesses.
   - **The residue, written in the module documentation and the verdict:**
     - the twin never sees a wait time out;
     - it models no Run* mode;
     - no twin clock depends on VOS. A VOS3 firmware at 64 MHz runs the same, and only the §1.3 rule refuses it;
     - the `CR3` lock across a warm reset is modelled only as far as D-3 (a) stores the first write.
     - Name the `PWR` fields that are not stored in `Image::unobservable` (`boot.rs:657–680`).

7. **The ledger (`RUL-20`).** Add rows to `svd-hal` `board/ledger.rs` and `cli/src/ledger.rs`. Propose each class in FEEDBACK, citing the measurement:
   - `PWR.CR3.LDOEN`, `.BYPASS` and `.SCUEN` (a read-modify-write), and `PWR.D3CR.VOS` (written): `independent-rule`. The §1.3 rule gives the word, and the twin's rule and Kani hold the firmware to it. Note beside each that it is ignored by the model (the stub).
   - `PWR.CSR1.ACTVOSRDY` and `PWR.D3CR.VOSRDY` (polled), and `FLASH.ACR.LATENCY` and `.WRHIGHFREQ` at reset (depended on): `silicon-only`.
   - **Each new `silicon-only` row is acknowledged by name** in `examples/sentinel-01.json`'s `acknowledge_unwitnessed`, with its reason, or `verify --fab-out` refuses the package. Add the same acknowledgements wherever a test writes a package.

8. **Every moved figure is re-pinned from a measured run, each cited to it:**
   - **Goldens:** `svd-hal` `tests/order_2203.rs`, `tests/ui/{motors-pwm,serial-four}`, and `tests/board.rs`'s `settle` and bound lines if they move (they should not).
   - **Boards:** `boards/sentinel-01{,-bench}` (byte-compared by `cli` `generate_tier1`): `board.rs`, `proofs.rs`, `firmware/src/main.rs`, and the `.repl` under D-3 (a).
   - **Platforms and solved topologies:** every pinned platform text and hash, every embedded solved JSON (the list is at `4240` FEEDBACK L581), and every specification and topology sha256 a test pins.
   - **The ELFs and hashes:** every pinned ELF hash; `3304`'s `sentinel-02-bench.elf`, old (`375fd31a…9c02`, or `4400g`'s successor) and new, in FEEDBACK [D-5]; and `examples/sentinel-01.json`'s ELF in the package.
   - **The bench package.** `fab/sentinel-01-bench/package/` comes only from the architect's `ci.yml` `bench-package` dispatch with `write=true` on the READY head. The builder commits the artifact byte for byte, as `4400g` F4 did.
     - In FEEDBACK, name each of its 17 files as moved or byte-identical. Only the ELF, `MANIFEST.json` and `FAB-NOTES.md` may move.
     - **Any Gerber, drill, BOM, CPL or position file that moves is a §7 stop.** Those boards are made.
   - **`TIER2_MIN`:** no tier-2 test is planned. If you add one, say so on the bus. The `TIER2_MIN` line (`ci.yml` L390) belongs to `1911`, and the change is coordinated, never made silently.
   - **A new test over 60 s of CI time** (the Kani harness and `verify` runs) gets its row in `ci/test-weights.tsv` from a measured run (`1910` D6, `ci/shard.sh` L211–252). Say on the bus before you push it: `1911` is in `ci/`.

9. **Bench row B-2, extended** (Nucleo, t9o777, Scotty's). Write it as a section of FEEDBACK, in `3304.bench.md`'s form, for the architect to place [D-5]. Every value is predicted before the bench, with its source class: rule, generator or not predicted.
   - **After a power cycle**, then a boot of the new `sentinel-02-bench.elf` (sha256 named), halted after the boot word:
     - `probe-rs read b32 0x5802480C` (`PWR_CR3`): the predicted word, from your write and rev V's `0x46`;
     - `0x58024804` (`PWR_CSR1`): `ACTVOSRDY` 1 and `ACTVOS` `10`, as `0x0000_A000` under the mask `0x0000_E000`;
     - `0x58024818` (`PWR_D3CR`): `0x0000_A000` (`VOS` `10`, `VOSRDY` 1);
     - word 0 = `0x2100_B007`.
   - **The control:** under reset, before the firmware runs, `D3CR` predicts `0x0000_6000` (VOS3 and `VOSRDY` 1; "Following reset VOSRDY will be read 1", p. 310). So the row can fail.
   - **After a warm reset**, following a firmware that wrote another `CR3`, the prediction is `SUPPLY_NOT_DECLARED` or a boot, by the bits. Say which firmware Scotty may have on the board and what each gives. A row whose answer is unknown is labelled not predicted.
   - `probe-rs` syntax is checked against 0.32.0's `--help` (`4240` FEEDBACK L94).

## 2. Acceptance

- **Every generated firmware boots in the twin at VOS2 with the rule passing.**
  - Run every committed specification `generate` accepts, through `simulate` (and `verify` for those CI verifies). Each verdict passes the §1.3 rule, the twin's `PwrRule` and the existing gates.
  - The same specifications with `vos` forced to Scale 3 in a test-only topology are refused by the rule, by name.
  - **No verdict flips.** A flip is a stop.
- **No solved word moves.** Diff every `solve` output against `main`'s, masked only for the schema version and the two new fields. Everything else is identical.
- **`validate` refuses a missing `core_supply` by name** (``missing field `core_supply` ``, exit 2). It also refuses `null`, an unknown mode and an empty or overlong `source`, each by name. `generate`, `emit` and `simulate` refuse `bypass` as `CoreSupplyNotGenerated`.
- **Each refusal has a test and a mutant that a test kills:**
  - each `VosLimitExceeded` clock family: bus, TIM, SPI1–3, SPI4–6, I2C, USART, LPUART;
  - the boundary at fastest = limit;
  - nominal substituted for fastest (the S14 mutant);
  - `UnsupportedClock` for a non-Scale-2 generated tree;
  - `CoreSupplyNotGenerated`;
  - each of the three boot words;
  - each `PwrRule` finding: a second write, the reverse order, a wrong supply bit, a `VOS` other than `10`, a PWR write after an RCC enable;
  - the Kani clause, by dropping the `D3CR` write and then by writing `CR3` twice;
  - the token: a peripheral `init` before `Scale2` does not compile;
  - an older solved file refused, not defaulted.
- **The foreign run** [D-4]: unmodified ArduPilot passes the foreign judge as on `main`, with `vos_limit_at_boundary` named for each clock at VOS1's limit. A declaration missing `vos` is refused by name.
- **The bench package:** Gerbers, drill, BOM, CPL and positions are byte-identical to `main`. `--against-fab` reports `identical` against the committed package, which comes from the dispatch.
- **The CLAUDE.md gates pass. CI is green. `TIER2_MIN` is exact.** `1910`'s planner guards pass, with no heavy test unweighted and no stale row.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02`, `INV-03`, `INV-06`, `INV-07`; `RUL-04`, `RUL-14`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-ir`: `lib.rs` (`Mcu`, `SolvedTopology`, the schema constants, `validate`), `motor.rs` (`SolvedClock.vos`, `ForeignClock.vos`), and `silicon.rs` or its new sibling;
  - `aethelgard-svd-hal`: `board.rs`, `board/firmware.rs`, `board/ledger.rs`, its tests and `tests/ui`;
  - `aethelgard-twin`: `serial.rs` (the rule), `boot.rs` (the words), `platform.rs` (the `pwr` override, D-3 (a));
  - `aethelgard-cli`: `ledger.rs`, `renode.rs`'s composition, `renode/foreign.rs`'s condition, and the `CoreSupplyNotGenerated` refusal at `generate`, `emit` and `simulate`;
  - every committed specification and fixture (`mcu.core_supply` and its schema only), the solved fixtures as re-derived, and `boards/**` as regenerated;
  - `fab/sentinel-01-bench/package/`, **only** as the dispatch artifact;
  - `ci/test-weights.tsv`, rows for your own new tests only;
  - FEEDBACK.
  - **Not:** `.github/workflows/`, other `ci/` files, `container/`, any Renode source or the vendored `tests/renode-1.17.0/**`, `aethelgard-eda` (`emit`'s refusal is at the CLI), `aethelgard-chaos` (no trace kind or `RESET_KERNEL_HZ` moves: `campaign.rs:2397` stays 64 MHz), `fab/sentinel-01-bench/{BENCH.md,ORDER.md}` (`3403`'s), `docs/work-orders/AET-WO-3304.bench.md` (the architect's), the steering documents and the root `README.md`.
- **Running beside you:**
  - **`AET-WO-2304b`** (battery sag) also bumps the specification schema. Whichever merges second merges `main`, takes the next number and re-pins every pinned hash.
  - **`AET-WO-1911`** owns the workflows and `ci/`. Coordinate `TIER2_MIN` and the weight rows on the bus.
  - **`AET-WO-4404`** (hwdef and pincheck) edits the fixtures you re-derive. Merge `main` and re-derive; never hand-merge a solved file.
  - **`AET-WO-3403`** (Sentinel-01's bench plan): `fab/` files and a new `examples/sentinel-01-ext-bmp581.json`. Whichever merges second gives that file `core_supply` and the bump [D-6].
  - **`AET-WO-1315`** (eda emitter) edits `sentinel-03.json`'s power tree only. Merge `main` if it lands first.
- One container of yours at a time under `capped-run`, `--mem 16G` for `verify` and `prove`, at `nice -n 10`. You have no access to t9o777 or the bench.
- One PR, left unmerged. Write `~/.cache/aet-4240v/pr-title-body.md`.

## 5. Adversarial pass, full

Hand it to a fresh subagent with the primary PDFs, not this order's reads.
- **A nominal where a worst case belongs:** any limit judged at 64,000,000 Hz instead of its fastest, or a slow-side figure used on a maximum.
- **A limit typed twice and wrong once:** the rule's table against the page images, every cell; a VOS column shifted by one.
- **A word taken from the generator:** the oracle, the twin's rule or the ledger getting the `CR3` or `D3CR` word through `svd-hal`, or through a function the generator also calls.
- **The order broken:** any RCC or peripheral access before `Scale2` in any generated `main`, a `D3CR` write before the `CR3` read-back, or a write after a fault.
- **The warm reset:** `CR3` already holding the declared bits, holding the other valid configuration, or holding `USB33DEN` from ArduPilot. Each gives the named outcome, and no second write is attempted.
- **Hostile declarations:** `core_supply` missing, `null`, an unknown mode, a mixed-case mode, a `source` of 0 or 201 bytes, or a non-printable `source`. A version-17 file carrying the key, and a version-18 file without it.
- **The unwired path:** a plan with no clock still runs the sequence. A foreign run is never given the generated sequence or the `PwrRule` judgement.
- **The stub override:** ArduPilot's witnesses unchanged to the nanosecond, the override's script not reachable from the generator, and the stock file byte-identical.
- **A fabrication file moving** with the ELF.

## 7. Flag-and-stop conditions

- **UM2407 Rev 6 does not state the NUCLEO-H753ZI's supply configuration.** Stop for that board with what it does say. Never infer.
- **RM0433 §6.8.4 or Table 34 does not give an unambiguous supply word,** or `SCUEN`'s semantics cannot be cited. Stop with the text.
- **Any solved word moves** (`PSC`, `ARR`, `CCR`, `BRR`, `MBR`, `TIMINGR`, a kernel select), or **any generated-firmware verdict flips.**
- **A Gerber, drill, BOM, CPL or position file of `fab/sentinel-01-bench/package/` moves.**
- **Renode cannot log or override the `pwr` stub,** so the twin cannot see the writes. Stop: judging only by Kani and the rule is a weaker gate, and that is the architect's ruling.
- **A limit cannot be cited with a document number and revision,** and a refusal would rest on it.
- **The Kani addition pushes `verify` past 16 GiB,** or a tools shard past 55 minutes.
- **A design that needs `unsafe` in a workspace crate,** a change to the vendored Renode platform, or a workflow edit.
