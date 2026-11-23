# AET-WO-2311 — Serial options seen in the twin: what ArduPilot writes to `USART_CR2` and the serial pins' `PUPDR` at boot, judged against the specification

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2311-serial-options-twin`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 3. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `2310`** (trace 19, scenario format 6) **and `4408`** (the four-port generated board in the ArduPilot matrix) merge.

**Grounds** (roadmap §7.1 L1056: "The serial options' run-time effect in `USART_CR2`, read back in the twin. `4404` reads it at source"; triage W3-TWIN, row 1028):
- **The owner cell is wrong.** L1056 and L1054 (the run-time pull-ups) name `AET-WO-4408`, but `4408`'s work order carries neither. This work order takes L1056. L1054 is triage class B, "ruling needed": the pull is recorded here, and judged only if ruled at §1.2.
- **What ArduPilot does** (`4404` FEEDBACK §2.1, §2.5, read at source, not measured): `set_options` (`UARTDriver.cpp:1562-1717`; H7 branch `:1589-1635`) sets RXINV, TXINV and SWAP in `_cr2_options`, applied to `USART_CR2` at every `begin` (`:435-438`), AP_RCProtocol's included (`AP_RCProtocol.cpp:335`). On H7 it sets a TX and RX pull-up whenever a line is not inverted (`:1611`, `:1624`); `set_pushpull` (`:1697`) runs after. SWAP leaves the GPIO alternate functions unchanged, so no GPIO judge sees it.
- **What Renode can show:**
  - `STM32F7_USART` stores no CR2 field but `STOP`: RXINV and TXINV are tags (measured, `4300b` P4.1), and SWAP is neither stored nor in the twin's tag list (`twin/src/serial.rs:526-550`, `:572-600`). **A read-back of CR2 after boot reads 0 for all three.** They can be seen only as written words.
  - `PUPDR` is stored and read back (`renode.rs` test strings at `:6557-6571`).
- **The foreign judge today** (`cli/src/renode/foreign.rs`): `RULES` row "USART CR1, CR2" (`:79-81`) compares CR2's `STOP` only (`register_rule`, `:806`); the solved pins' `MODER` and `AFR` are kept, and `PUPDR` is not judged. It logs one write per SPI controller's `CFG2` through a hook (`RULES`, `renode_error_log`, D7 (b)): the pattern for CR2 here.
- **The specification:** `serial_ports[].framing.inverted` is both lines (`ir/src/serial.rs:82`). It has no swap, so SWAP must stay clear. ArduPilot's RC input inverts RX only, and the ruled RC rule leaves RXINV to it and requires TXINV clear (`4404` FEEDBACK §2.5).
- **The ledger's CR2 rows** (`cli/src/ledger.rs:1553-1566`, class `Ign`) are acknowledged by name in the bench package (`fab/sentinel-01-bench/package/FAB-NOTES.md:149`). Changing a class may make that acknowledgement stale (`ledger::stale`, called at `cli/src/main.rs:2901`), which moves the bench specification and its package.

> **What this is:**
> - **(1)** Under `simulate --foreign`, every CR2 word ArduPilot writes to each declared port's USART is logged by a hook (the `CFG2` pattern). The judge holds the last word before the window's end against the specification: SWAP clear, and TXINV/RXINV per `inverted` and the RC rule. The expectation comes from the specification alone (`INV-07`), never from the ELF's `var_info`.
> - **(2)** Each declared port's TX and RX `PUPDR`, read after the run, reported as an info finding naming the pull. It is judged only if §1.2's ruling says how.
> - **(3)** The four-port board's ELF from `4408`'s matrix job runs under `ci/foreign-judge.sh`, with a swap mutation (`4404`'s `adv-opt-swap`) and an inversion mutation, each refused by name.
> - Each refusal comes with a test, and each test with a mutant it kills.
>
> It is NOT:
> - a trace-format bump, unless §1.1 measures it necessary (a stop);
> - any change to the generated firmware, the generators, `pincheck`, or the IR;
> - any change to a witness-ledger row's class;
> - a ruling on L1054's pull-ups.

## 1. Build

1. **Measure first,** with predictions committed in `spike/2311/`, on the four-port ELF under `--foreign`:
   - every CR2 write per port, with its virtual time; whether each port's `begin` runs inside `FOREIGN_MAX_VIRTUAL_NS` (5 s, `foreign.rs:30`);
   - whether an RC port's CR2 changes more than once while AP_RCProtocol searches;
   - the log volume the hook adds, against the 8 MiB budget;
   - each serial pin's `PUPDR` at the run's end;
   - whether the verdict's new findings need a trace field, and whether any `fab/` byte moves (none is predicted: the bench board is generated, not foreign).
2. **Stop with:** the measurement; which write is judged on a port whose CR2 changes; the finding kinds and their text; (2) recorded or judged.
3. **Build to the ruling.** `foreign.rs`'s `RULES` row is rewritten to say what is now judged and what is still weakened.

## 2. Acceptance

- **The four-port board passes,** and each mutation is refused by name, in CI's ArduPilot workflow.
- **A planted CR2 word** with SWAP set, TXINV set on an RC port, or RXINV missing on an inverted port is refused, each by a tier-1 test on a recorded log.
- **Mutants:** the SWAP check removed; TXINV and RXINV exchanged; the first write judged in place of the last. A test kills each.
- **No `fab/` byte moves, and no ledger row changes.**
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `crates/aethelgard-cli/src/renode/foreign.rs`, `src/renode.rs` (the hook and its parse), and their tests; `crates/aethelgard-twin/src/serial.rs` (the expected options, from the topology); `ci/foreign-judge.sh`; `ardupilot-image.yml`'s foreign job only, to hand it the four-port ELF; `spike/2311/` and FEEDBACK.
- **Not:** `cli/src/ledger.rs`, `verify.rs`; the IR, the generators, hwdef, pincheck, chaos; `ci/` except weight rows.
- **Running beside you:** the backlog wave in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-2311/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A CR2 write the hook misses:** a byte or half-word write, a write through an alias, or a write before the hook is armed.
- **An expectation read from the artifact:** any path where the judge's SWAP/INV comes from the ELF or the generated hwdef (`INV-07`).
- **A generated firmware that sets SWAP:** does `judge_writes` refuse it today? Unverified; say which.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement and the rulings it needs.
- **A trace-format bump is needed** to carry the findings. Stop with why.
- **A port's `begin` does not run inside the 5 s window.** Stop: widening the bound is a ruling of its own (`foreign.rs:20-30`).
- **Any ledger row's class or text would change,** or a `fab/` byte moves. Stop with the diff.
- **`4408`'s four-port fixture has no inverted or RC port** to judge. Stop and say what a fixture would need.
