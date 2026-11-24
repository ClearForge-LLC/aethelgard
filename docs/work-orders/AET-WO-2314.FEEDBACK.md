# AET-WO-2314 — FEEDBACK

Base: `main` 72d85eef; main 1cb994c1 merged in (1324, the WO's own docs, 3309's docs). I stopped at §1.1 with the
measurement (`spike/2314/`, predictions c73d9e94 committed first, PR #387 at e5e3a094). The architect ruled at 10:30Z:
D1 (a), D2 (a) with (i) and the CI plant, D3, D4 a row, D5. Built to that ruling.

## 1. The measurement (§1.1)

`spike/2314/RESULTS.md` holds the measurement. In short:
- Six endings of the undisturbed boot all gave the same bare `PlantedEventUnplaced`.
- The halt from a fault-vector hook holds: the planted HardFault peaks at 856–873 MB, against `memory_cap_exceeded` at 1.58 GB.
- The R3 flood is `ISR` reads. `TDR` is 600 lines.
- R4: 56 callers, cheap but not sure.
- P1.3 was wrong in its argument; RESULTS says why.

## 2. What is built

- **(1) The campaign names the cause** (`campaign.rs` `undisturbed`, `place`, `place_droop`, `run_one`).
  - When the undisturbed boot wrote no boot word, `PlantedEventUnplaced` ends with that boot's own end:
    - each non-info finding of its verdict, as `kind: message`;
    - `renode_log_flood`;
    - for a bound, "memory: Renode peaked at N KiB against the bound's M KiB";
    - when the word reads `BOOT_MAGIC` but no hook saw it written, which no finding names, a sentence of its own.
  - The droop and the battery sag pass the same `&Result` to `place`. There is no new verdict field, and the dropped count is not named (D1 (a)).
- **(2) `foreign_fault`** (`renode/foreign.rs`; `elf.rs` `loaded_words`; `main.rs` wiring).
  - The four fault handlers come from the ELF's vector table at `FLASH_BANK1_BASE`, the address the boot writes to VTOR.
  - **Refused before the boot** as `ForeignFaultVectorUnusable` when:
    - an entry is 0;
    - an entry is outside every executable `PT_LOAD`;
    - no single `PT_LOAD` holds the entries.
  - **The hook:** one `cpu AddHook` per distinct handler. When `ICSR.VECTACTIVE` is a vector that handler serves, it:
    - logs `AETHELGARD-FAULT <n> <handler> <frame> <stacked PC>`;
    - sets `IsHalted`.
  - **The finding:** `foreign_fault` names the vector, the PC, the frame's address and the time.
  - **On the record:** one RULES row (D2 (a)); the mark is kept past the log budget; a run killed at a bound after the mark names both.
- **(3)** `FOREIGN_LOG_FILTER.discard` = the default discard, or a stock USART's write to `TDR` at any width (`unread_access_line`).
  - `LOG_FILTER`, `LOG_FILTER_HSE` and `is_unjudged_access` are untouched: the generated judge reads `TDR`.
- **CI:** `ci/foreign-judge.sh` gains a `udf` branch in `plant()` and the `late-hardfault` case.
  - The branch sits after the shared site search, so `panic` and `panic-unnamed` are unchanged.
  - No existing case changed.

**Outside the listed fence, needed by the ruling:**
- `main.rs`: the `faults` field, the two call arguments, the bounded arm's fault finding.
- `elf.rs`: one new function. D2 (i)'s refusal before the boot sits where `ForeignPanicSymbolMissing`'s does.
- No IR, twin, `verify.rs` or validate change.

## 3. Tests, and the mutant each kills

`spike/2314/mut/mutants.py`: 22 mutants, each run against the suite its test lives in, all killed.

| Guard | Test | Mutants |
|---|---|---|
| the carried end (findings, flood, memory, magic sentence, the message) | `campaign_tier1` `an_undisturbed_boot_with_no_boot_word_names_its_end_in_the_refusal` (7 plants: memory bound, flood, fault word, park code, no write, magic with no hook, flood and fault word together) | C1–C6 |
| the vector table refusal (0 entry, also with code at ITCM's 0; outside; no table) | `boot_tier1` `a_foreign_firmware_whose_fault_vector_cannot_be_hooked_is_refused_by_name`, `the_foreign_judge_is_declared…` | F1, F2 |
| the hook text (VECTACTIVE gate, halt, PSP/MSP frame), the mark kept | `foreign::tests::a_fault_entry_is_the_cpus_own_mark_at_a_hooked_handler_for_its_vector` | F3, F4, F5, F7 |
| the mark parse, the hook emitted, the finding, its vector and frame | `boot_tier1` `a_foreign_boot_that_takes_a_fault_fails_by_name_with_its_vector_and_pc` (the stub refuses a command line without the hook, exit 91) | F6, F8, F9, F10, F11 |
| a fault, then a bound | `boot_tier1` `a_foreign_boot_that_faults_then_reaches_a_bound_names_both` | B1 |
| the `TDR` discard (any width, USART only, writes only) | `foreign::tests::the_foreign_filter_discards_tdr_writes_the_generated_filter_keeps` | D1–D4 |
| real Renode | `ci/foreign-judge.sh` `late-hardfault`; tier-2 R1 foreign tests on the moved fixture | — |

**Fixtures moved:**
- tier-1 foreign tests boot `foreign_code()`, which is `code()` with a table; the flash hash is computed in the test.
- The D2-only ELF is refused for having no table. Its positive leg now has a flash segment holding the table.
- tier-2's `foreign_code` moved its code to `0x40` and names a `b .` at `0x70` that never runs.
- No committed golden changed.

## 4. Evidence

All of it from the pinned tools image, capped (`~/bin/capped-run`, swap 0), on CI's ELFs (`ardupilot-image` run
45342089937, main 0faaaec3).

**The planted HardFault:**

| Run | Peak RSS KiB (cap 1,572,864) | Wall s | Findings |
|---|---|---|---|
| main (before) | 1,575,296; 1,580,632 | 25.0; 25.3 | `memory_cap_exceeded` only |
| this branch (after, 3 runs) | 861,228–878,608 | 8.5–10.1 | `core_in_exception`, `foreign_fault` (HardFault, PC `0x080360aa`) |
| this branch, beside my own cargo gate | 1,135,664 | 24.0 | the same |

**(3), the `TDR` drop:**
- It moves no peak: R3's flood is `ISR` reads, discarded already.
- The positive case keeps 1,547,185 bytes of log, against 1,598,657 before: 51,472 bytes, the 600 `TDR` lines.
- This is hygiene, not a memory fix (D3).

**Other runs:**
- `ci/foreign-judge.sh`: every case passes, unchanged, plus `late-hardfault` (local, final tree: `positive` keeps 1,547,229 bytes at 899,016 KiB; `late-panic` 901,332 KiB; `late-hardfault` 855,404 KiB).
- A direct call to `HardFault_Handler` from thread mode (`plant.py call`) logs no mark. It ends `memory_cap_exceeded` as before: no false `foreign_fault`, and a residual.

## 5. D5: the BMP581 replay

**It did not reproduce on Furnace.**
- Shard 11's failing plan is run 45312081588's `shard.tsv`: 288 tests, byte-identical to the three 4410 runs that failed.
- It ran through `ci/shard.sh run`, pinned to 4 CPUs (`taskset -c 0-3`, `nproc` 4), in the tools image, on this branch's final tree.
- Result: **10 passes of 10**, 473–573 s each, with Furnace's load average at 5–22. `spike/2314/sh/replay.sh`.

**1610's failing plan** (run 45397844312, the architect's 13:40Z note): five runs were going at the push, and the result goes on the bus and here before READY.

**So the cause is not named yet.** This branch's refusal now carries the undisturbed boot's end (§2 (1)), and the next CI failure will print it. Measured so far (`spike/2314/RESULTS.md`):
- The failing runs lasted 55.0 s and 57.5 s, against 96.2 s when the same packing passed.
- The undisturbed boot takes 4.1 s alone and 21.8 s with 8 sharing 4 CPUs.
- Memory peaks at 47 % at most, the log at 0.1 %.

The wall bound fits these numbers. If CI names it, the fix and its numbers come back to the architect (D5).

## 6. Adversarial pass (a fresh Opus reviewer, read-only, then mine)

| # | Finding | Disposition |
|---|---|---|
| 1 | tier-2's fault entries named the spin the core runs on, so the hook ran each pass | fixed (`b .` at `0x70` that never runs); the shared-default-handler cost is in the RULES row |
| 2 | a fault then a bound named only the bound | fixed (bounded arm adds `foreign_fault`; test + B1) |
| 3 | the sag call site has no planted test | no tier-1 stub boots a power-tree board (`campaign_tier1`'s own note); both sites pass the same `&Result`, so the sag site has no conversion of its own. Row below |
| 4 | the D2 positive leg was replaced by a refusal | fixed (two-segment ELF, judged foreign) |
| 5 | the RULES row did not name a handler reached without an exception | fixed |
| 6 | an unreadable frame reads as PC 0 | the finding now names the frame's address; the row says it |
| 7 | "no PT_LOAD" said "single" in the doc only; a Thumb-bit-clear entry is hooked | message fixed; the entry is a row |
| 8 | a misplaced doc comment | fixed |
| 9 | no fallback for an empty end | not added: every end without a word has a finding or the magic sentence (planted, all six); the doc says why |
| 10 | the CI script's edit is inside `plant()` | noted in §2 |

## 7. Reduction pass (diff, a fresh subagent, then mine)

`Reduction pass (diff): −5 net · 2 duplicates removed (the ICSR address and VECTACTIVE mask from the twin's constants; one ELF builder for two) · 2 dead paths removed (the magic sentence's bound guard, the empty-end fallback) · 1 protected kept (elf.rs loaded_words' file bound, labelled) · 3 findings below`

## 8. Rows (for §6)

- **D1 (a)'s row:** the flood's dropped count is in `run::Output` only. Naming it needs `main.rs`'s `simulate` to pass it beside the verdict.
- **R4 (D4):** a cross-check of the panic hook against the ELF's call sites is cheap but not sure.
  - Measured: 56 direct `BL` callers (54 on four-port and noimu), all passing a string literal, and 0.04 s in Python.
  - The rule "the most-called function whose callers all pass a literal" caught every symtab-only decoy tried.
  - A firmware built against it defeats it: a decoy with more literal callers, or a panic reached only through `BLX`, a vtable or `B.W`. See `spike/2314/r4/`.
- **Untested vectors:** only HardFault was exercised, by an escalated `udf`. MemManage, BusFault and UsageFault are hooked the same way but never entered. Renode answers an unmapped read with a log line, so a BusFault plant may not be reachable.
- **A Thumb-bit-clear table entry** is hooked at its address. The core cannot enter it (INVSTATE, then lockup), so the hook never fires and the run is judged as before.
- **The sag call site** has no tier-1 plant (no stub boots a power-tree board). A tier-2 one would need a generated firmware that writes no boot word.
- **The Renode logging boilerplate** is now in three hooks (`panic_hook`, `log_hook`, `fault_hooks`). Two predate this WO, in 1609's code.
- **`usarts()`** wraps `stock_uarts` in a `OnceLock`, as the twin's private `usart_sources` does. Sharing it needs the twin's function made public.
- **`campaign.rs`'s bound kinds** restate three of `aethelgard_chaos::campaign::UNREAD_FINDINGS`. Using the list would add the memory line to a load error's refusal.
- **Memory under load:** the halted HardFault peaks at 1.14 GB at 24 s beside a cargo build, and at 0.86 GB alone. Contention raises Renode's peak, as it raised the BMP581's (616–745 MB against 516 MB).

## 9. Gates

- `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace --no-fail-fast`: **3,069 passed, 0 failed** (host, e452a1a0; the merge of main 1cb994c1 included).
- In the image, with tools:
  - `ci/foreign-judge.sh` all cases pass;
  - `twin_tier2` `bor_recovery` (3) and the foreign R1 tests (3) pass;
  - `ci/shard.sh build` and 10 shard-11 runs pass.
- Mutants: 22/22 killed (`spike/2314/mut/mutants.py`, restored by `git checkout --` with a fresh mtime). A clean rebuild passed after the sweep.
