# AET-WO-4240b — The crystal generated and simulated: the bring-up programs H1, bounded and halting by name, the twin allows exactly those words, and `HseNotGenerated` is lifted

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4240b-hse-bringup`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (Sentinel-02's clock). Written 2026-11-20. **Cadence:** a measurement, then a build with stops. **After `4240a` (#281) and `4241` (#279) merge:** both change the files this work order rewrites.

**Grounds** (`AET-WO-4240` FEEDBACK §4.5, §6, §8, D5, D6, D10 item 2, ruled on #240; `4240a` FEEDBACK D-9, F3; `4240v` F1, F2, F7; roadmap §5 and §7.1):
- **`4240a` puts the crystal in the specification and stops there.** `mcu.hse` is `null`, `crystal` or `external`. A crystal or external specification solves to H1's words, for example 16 MHz: `PLLCKSELR` `0x52`, `PLLCFGR` `0x0003_0004`, `PLL1DIVR` `0x0103_064F`. `generate`, `emit` and `simulate` refuse it as `HseNotGenerated`.
- **The bring-up writes no RCC clock word today.** `ResetClock` is a token for the reset tree (`svd-hal` `board.rs`, `emit_reset_clock`). Every peripheral `init` takes it. `4240` §8's `svd-hal` table lists each site that changes.
- **The twin's RCC rule refuses any write to `CR`, `CFGR` or a PLL register** (`twin` `serial.rs`, `rcc_rule`/`judge_rcc`). Renode's `STM32H7_RCC` reads every RDY bit as its ON bit, with no delay. CSS is a bare flag there, and the kernel muxes are tags that read 0 (`4240` §6, read at source). So the twin can witness the words and their order, but never a wait.
- **D5, ruled:** never fall back to the HSI.
  - Each wait is bounded: `HSERDY`, then `PLL1RDY`, then `SWS`. A timeout halts with a distinct boot word: `HSE_NOT_READY`, `PLL1_NOT_LOCKED`, `SW_NOT_TAKEN`.
  - `HSECSSON` is set after `HSERDY`. The NMI halts with a named fault word, and motor outputs are held safe.
  - The `HSERDY` bound is declared: DS12117 gives only "2 ms typ" (`RUL-14`).
- **D6, ruled:** every kernel select under H1 is left at its reset value. SPI1–3 move to `pll1_q` (`SPI123SEL` `000`, with `DIVQ1` = `DIVP1`), and SPI4–6 to `pclk` (`000`). No `MBR` moves, and the twin's tags, which read the reset value, report the truth by construction.
- **Carried here:**
  - `4240v` F2: the `Scale2` token gates no clock enable;
  - `4240v` F7: its reduction proposals P1, P6, P7, P8, R1, R2 and R4;
  - `4240a` F3: a topology's copied HSE declaration is trusted by paths that read only a topology;
  - `4240v` F1: the `VOSRDY` residue. Close it only if a held ST source gives `ACTVOS`'s encoding.

> **What this is:**
> - **The generator** emits H1's bring-up for an `hse` board, after `4240v`'s VOS2 sequence:
>   - `HSEON` (with `HSEBYP` for external), a bounded `HSERDY` wait, then `HSECSSON`;
>   - `PLLCKSELR`, `PLLCFGR` and `PLL1DIVR` as solved, then `PLL1ON` and a bounded `PLL1RDY` wait;
>   - `FLASH_ACR` per Table 17's VOS2 row, then `SW` and a bounded `SWS` wait;
>   - the kernel selects at reset. Each timeout is a named boot word.
> - **A `SolvedClock` token** replaces `ResetClock` for an `hse` board, produced only after `SWS`. Every `init` that took `ResetClock` takes it.
> - **The CSS NMI handler** halts with a named fault word. It drives each motor output with no hardware break (TIM2–5) to its safe level, and the verdict names those motors.
> - **The Kani oracle** becomes "before plus exactly the solved RCC words", with each ready poll nondeterministic and each wait bounded.
> - **The ledger** gets a row for each RCC field the bring-up writes.
> - **The twin's RCC rule** allows exactly the solved words, once each, in order. Any other RCC write is still a finding. The generated `.repl` sets `hseFrequency`.
> - `HseNotGenerated` is lifted at `generate`, `emit` and `simulate` for a declared crystal or external clock that solves to H1.
>
> It is NOT:
> - the footprint, the routing or `Cs` (`4240c`);
> - H2, or any tree other than H1 (a non-H1 `hse` tree stays `HseNotGenerated`, now naming the tree);
> - `simulate --foreign` on an HSE board (`4240a` F4, with `4400`);
> - ArduPilot's `hwdef` for an HSE board (`OscillatorNotCited` stays);
> - the bypass bench (`4240` D7 (c), its own work order after this).

## 1. Build

1. **Measure first,** in `spike/4240b/`, with predictions committed before the runs.
   - On `main`, generate every committed specification. Record every emitted file and the bench ELF's sha256.
   - **Prediction:** every `hse: null` specification's bytes, ELF and twin verdicts are unchanged by this work order. **Stop and report** if any moves, with the diff.
   - **Read the schema `4240a` merged.** Does `mcu.hse` carry a start-up bound for `HSERDY`?
     - If it does, the wait uses it.
     - If it does not, add one: `startup_max_us` on `crystal` and on `external`, refused at 0 (`4240a` D-6's rule). Take the next free specification version. `1210` holds 21, so it is 22 if `1210` has merged and 21 otherwise; say on the bus which you took. Every committed specification is renumbered by the repository's script, version lines only.
     - **Stop and ask** if the bound would need a figure no held source states for the 16 MHz fixture. The fixture declares one with its citation, or the architect rules the figure.
   - **`4240v` F1:** search the held ST sources (RM0433 Rev 9, DS12117 Rev 10 and rev Y, `~/.cache/aet-4240/ds/`) for `ACTVOS`'s encoding. Quote it, or say it is absent. If it is absent, the residue stays stated in `firmware.rs`, and the row stays open.
2. **The generator** (`aethelgard-svd-hal`), for a board whose solved clock is H1 with an HSE:
   - **The sequence**, each word from the solved topology and never recomputed:
     1. `4240v`'s supply and VOS2;
     2. `RCC_CR.HSEBYP` (external only), then `HSEON`;
     3. a bounded `HSERDY` wait at the declared bound, timing out to `HSE_NOT_READY`;
     4. `HSECSSON`;
     5. `PLLCKSELR`, `PLLCFGR`, `PLL1DIVR`, then `PLL1ON`;
     6. a bounded `PLL1RDY` wait (150 µs max, DS12117 Table 39/136, cited), timing out to `PLL1_NOT_LOCKED`;
     7. `FLASH_ACR` per Table 17's VOS2 row, read back before the switch;
     8. `CFGR.SW` = PLL1;
     9. a bounded `SWS` wait, timing out to `SW_NOT_TAKEN`.
   - **The bounds in spins** use the core clock at each wait's own point: the HSI before `SW`, at its fastest. Show the arithmetic in FEEDBACK.
   - **The kernel selects** are left at reset (D6). The generator's `SPI123SEL` and `SPI45SEL`/`SPI6SEL` writes are removed for an `hse` board. No `MBR` moves; show it.
   - **`settle` spins and the SPI `EOT` delay** take the board's source: the crystal's fastest after the switch. Show each count against `main`'s for a `null` board, which must not move.
   - **The token.** `SolvedClock` is produced by the bring-up only, after `SWS`. Each `init` and `start_outputs` takes it. A `null` board keeps `ResetClock` and its bytes.
   - **`4240v` F2:** every `enable_*` takes `&Scale2`. Show a UI test where enabling before `pwr.scale2` fails to compile.
   - **The CSS NMI handler.** It writes a named fault word to the boot record, drives each motor output on TIM2–5 to its declared safe level, and halts. A motor on TIM1/8/15/16/17 is held by the hardware break (RM0433 §8.5.3, cited). The emitted board's doc comment lists the motors with no hardware break.
3. **`4240a` F3, the copied declaration.** `generate` re-solves from the specification. A path that reads only a topology (`generate hwdef --topology`, `check pinout`) compares the topology's `hse` against what its own budget and words imply, and refuses a mismatch by name. Choose one design and say why.
4. **Kani.** The oracle's RCC clause becomes "before plus exactly the solved RCC words, in order".
   - Each ready poll is nondeterministic. A harness shows that each timeout reaches its boot word and that no path reaches `SolvedClock` without all three ready bits.
   - **One harness,** the size of `reset_clock_is_ok_exactly_at_reset`. Report its time. If it exceeds 600 s, stop and report.
5. **The ledger** (`cli` `ledger.rs`): a row for each field written: `CR.HSEON`, `HSEBYP`, `HSECSSON`, `PLL1ON`; `PLLCKSELR`; `PLLCFGR`; `PLL1DIVR`; `CFGR.SW`; `FLASH_ACR`. An unclassified field stays a hard error.
6. **The twin.**
   - The RCC rule allows exactly the solved words, once each, in the order of step 2. Any other RCC write, any repeat and any reordering is a finding, by name.
   - The generated `.repl` sets `hseFrequency` on the `rcc` line. Measure that Renode takes it: read `sys_ck` back after `SW`.
   - **The boot shift.** H1 adds the twin's zero-time waits only (RDY reads its ON bit). Measure boot-to-`CEN` on the 16 MHz fixture. State what the twin under-counts against silicon: up to `tSU` + 150 µs.
7. **`HseNotGenerated`:** lifted for H1 at `generate`, `emit` and `simulate`. Another tree is refused, naming the tree.
8. **`4240v` F7:** take P1, P6, P7, P8, R1, R2 and R4 where this rewrite reaches them. List each one taken or declined, with why.
9. **Tests:**
   - the 16 MHz crystal fixture and the 8 MHz external fixture (`4240a`'s) generate, build and simulate to `pass`. The twin witnesses every solved word in order, and `sys_ck` reads back as solved;
   - each timeout's boot word is shown, in Kani and by a generated-code test;
   - the RCC rule refuses: a missing word, a repeat, a reorder, `SW` before `PLL1RDY`, and a select written away from reset;
   - every `null` board's bytes, ELF and verdicts are unchanged.
   - **Mutants:**
     - drop the `HSERDY` bound: a test fails;
     - write `SW` before the `PLL1RDY` wait: Kani fails and so does the twin's rule;
     - leave `SPI123SEL` at `100`: the rule names it;
     - drop the NMI handler's TIM2–5 drive: a test fails.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No `null` board's byte, ELF or verdict moves.
- **Both HSE fixtures pass `simulate`,** with the twin's RCC witness in order and `sys_ck` read back as solved.
- **Each timeout reaches its boot word,** shown by Kani and by test.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count (one twin run per HSE fixture is expected). Each new heavy test has a weight row by `1910`'s rule.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-02`, `INV-06`, `INV-07` (the twin's rule and the IR's judge are not the generator; `RUL-20`).
- **Fence:**
  - `crates/aethelgard-svd-hal/src` (the bring-up, the token, the NMI handler, Kani, `&Scale2`);
  - `crates/aethelgard-twin/src` (the RCC rule, the `.repl`);
  - `crates/aethelgard-cli/src` (lifting `HseNotGenerated`, the ledger rows, the topology-copy check);
  - `crates/aethelgard-ir/src`, only for `startup_max_us` and the topology-copy check;
  - committed specifications (the version line only, if step 1 adds the field);
  - tests, UI tests and goldens;
  - `spike/4240b/`;
  - FEEDBACK.
  - **Not:** eda, `boards/` (unless a `null` board moves, which is a stop), `fab/`, the hwdef generator, workflows, or `ci/` (except weight rows).
- **Running beside you:**
  - `1210`: the specification version, and IR `validate`;
  - `3306`: evolve;
  - `4406`: pincheck.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`. Kani at most one harness at a time.
- One PR, left unmerged. Write `~/.cache/aet-4240b/pr-title-body.md`.

## 5. Adversarial pass, full

- **A bring-up the twin passes and silicon hangs or runs fast,** because the twin cannot wait: for example, `SW` while `PLL1RDY` is 0, or a select the twin's tag reads as 0.
- **A bound in spins that is too short** at the HSI's fastest, or one that never ends because the counter overflows.
- **An NMI path that continues on the HSI,** or leaves a TIM2–5 motor driven.
- **A `null` board whose bytes moved** through the token rewrite.
- **A topology edited consistently that still generates** (`4240a` F3).

## 7. Flag-and-stop conditions

- **A `null` board's byte, ELF or verdict moves.** Stop with the diff.
- **Renode does not take `hseFrequency`,** or `sys_ck` does not read back as solved. Stop with the read.
- **The `HSERDY` bound needs a figure no held source states** for the fixture. Stop and ask.
- **The Kani harness exceeds 600 s.** Stop with the time.
