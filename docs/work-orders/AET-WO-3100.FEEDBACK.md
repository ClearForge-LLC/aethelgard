# AET-WO-3100 — FEEDBACK

**Branch:** `wo/aet-3100-attribution` · **Base:** `034fcfd` (`origin/main`, as the kickoff set it).

**Where everything ran.**
- **Tier 1:** on Furnace.
- **Tier 2:** in the pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`) under rootless podman 5.7.0, with the CI `tools` job's steps, offline. Each container was started `--memory 16g --memory-swap 16g --pids-limit -1` (2300 F12).
- **The caps, read back from the container's own scope on every run:** `memory.max` = 17179869184, `memory.swap.max` = 0, `pids.max` = `max`. The swap cap is 0 only because the runner writes it (F11).

**§7, held after the ruling:**
- The one fact the archive lacked, the join from a symptom to its bus, is now kind 15, by the architect's ruling (S1).
- No other record kind was added, and nothing outside R4's fence changed.
- No test was weakened. Every test that changed is listed under "Tests changed", with the reason.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## S1 and the ruling

**S1 · The archive did not say which bus a symptom is on · §7 · stopped before the build (commit `942c56e`).**
- Every rule in §1.2 is per bus.
- Kind 12 named only a device reference. Kind 13 named a reference and a controller, and only for the last read on each controller.
- Kinds 7 to 9 named a bus by id, but nothing joined that id to a controller or a device. Nothing joined a pin to its controller either, which the GPIO and AF rule needs.
- The joins lived in the `SolvedTopology`, and the archive held only its hash. I did not build the "one bus, so it must be that one" inference: a re-sealed two-bus run with the faulted bus's records dropped would have been attributed `software`.

**The ruling (architect, 2026-11-09):** option (a), as one new record kind inside this work order.
- **R1:** kind 15, `I2cBusSolved`, written by `campaign` from the solved topology.
- **R2:** run rules that refuse a dropped or forged join.
- **R3:** trace version 3.
- **R4:** the fence widened to exactly chaos's kind 15, rules and version, and `campaign` writing it. `simulate --firmware` stays byte-identical to main.
- **R5:** GPIO and AF joined through kind 15's pin and AF, with names and positions from svd-hal.
- **R6:** re-measure `OQ-03`.
- **R7:** a two-bus tier-1 archive for "a fault on another bus".
- Option (b), `attribute` reading the topology, was refused, so `attribute` reads the run directory alone (§1.3, `INV-04`).

## §3, acceptance

### Tier 2: the seeded set, P3 gate clause 1

`crates/aethelgard-cli/tests/attribute_seeded.rs`, in the image, with the real bridge, the real Renode, and each firmware built offline from a scratch copy of `boards/sentinel-01`. One campaign per firmware. 22 cases: **20 failing (10 electrical, 10 software) and 2 passing.**

| Case | Scenario | Mutant | Failing findings | `attribute` | Evidence (index:kind) | Masked |
|---|---|---|---|---|---|---|
| pullup-3k9 | seeded/pullup-3k9 | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| pullup-4k7 | seeded/pullup-4k7 | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| pullup-10k | seeded/pullup-10k | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| pullup-47k | pullup-47k | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| pullup-100k | seeded/pullup-100k | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| cap-220p-2k2 | seeded/cap-220p-2k2 | — | rise, timeout | electrical E1 | 3:7 4:7 5:9 | — |
| cap-470p-sda-2k2 | seeded/cap-470p-sda-2k2 | — | rise, timeout | electrical E1 | 4:7 5:9 | — |
| held-low-sda | seeded/held-low-sda | — | held low, timeout | electrical E1 | 4:9 | — |
| held-low-scl | seeded/held-low-scl | — | held low, timeout | electrical E1 | 4:9 | — |
| **trap-wrong-address-47k** | pullup-47k | wrong-address | rise, timeout | **electrical E1** | 3:7 4:7 5:9 | **15:13** |
| wrong-address-2k2 | pullup-2k2 | wrong-address | timeout | software S1 | 3:7 4:7 14:13 | — |
| **trap-wrong-address-3k3** | seeded/pullup-3k3 | wrong-address | timeout | **software S1** | 3:7 4:7 14:13 | — |
| wrong-register | pullup-2k2 | wrong-register | identity mismatch, unwitnessed | software S1 | 3:7 4:7 14:13 | — |
| wrong-nbytes | pullup-2k2 | wrong-nbytes | identity mismatch, unwitnessed | software S1 | 3:7 4:7 14:13 | — |
| i2c-clock-skipped | pullup-2k2 | i2c-clock-skipped | timeout, locked access, register | software S1 | 3:7 4:7 11:11 14:13 | — |
| scl-af-wrong | pullup-2k2 | scl-af-wrong | register | software S1 | 3:7 4:7 9:11 | — |
| sda-af-wrong | pullup-2k2 | sda-af-wrong | register | software S1 | 3:7 4:7 9:11 | — |
| scl-mode-output | pullup-2k2 | scl-mode-output | register | software S1 | 3:7 4:7 8:11 | — |
| gpiob-clock-skipped | pullup-2k2 | gpiob-clock-skipped | locked access, register | software S1 | 3:7 4:7 8:11 9:11 10:11 | — |
| no-boot-word | pullup-2k2 | no-boot-word | boot word missing | software S2 | 3:7 4:7 5:10 | — |
| **trap-unmutated-3k3** | seeded/pullup-3k3 | — | — | **none N1** | — | — |
| unmutated-2k2 | pullup-2k2 | — | — | none N1 | — | — |

Six tests in the file, all passing in the image:
- **Tier 1:** the manifest's counts and traps; every mutant applies exactly once to the board as committed; a patch matching zero or two times is refused, naming the mutant; every unmutated case's scenario expects exactly its manifest findings.
- **Tier 2:**
  - every case fails with exactly its manifest's findings, and every unmutated case's campaign line is `matched`;
  - `attribute` names the manifest's domain and rule for every case, with at least one evidence event for every failing case, each index in range and of a kind its rule allows;
  - the three traps, including that the unmutated 47 kΩ run masks nothing;
  - two `attribute` calls on every run print identical bytes, and a one-byte change is refused with `replay`'s exact bytes and exit code (`HashMismatch`, 3);
  - `OQ-03`.

### Tier 1

- **`crates/aethelgard-evolve/tests/attribute.rs`.** 22 tests on runs written through the chaos writer and read back by `replay_dir`. They cover:
  - each indeterminate path: `I1` unexplained finding, `I2` a line unmeasured or measured twice, `I3` a fault on another bus (R7, two buses, with the healthy symptom bus's rises present and in the evidence), `I4` healthy with no software evidence, and `I5` electrical and software split;
  - the three attacks of §5.1 to §5.3 in miniature;
  - each software evidence path: SADD, NBYTES gated by the read phase, TXDR gated by the write phase, a witness whose own expectation disagrees, the clock gate, the AF register, a board register, and the boot word;
  - held low as E1.
- **`campaign_tier1.rs` §7.** Through the CLI, with stub tools:
  - `none` twice in identical bytes;
  - the 47 kΩ run `electrical E1`, with kind 15 read back field by field;
  - an indeterminate run exits 1, naming its unexplained finding;
  - `attribute` refuses with `replay`'s exact bytes and code for a missing directory, a flipped byte, a renamed run, an extra file, and a dropped kind 15;
  - a run copied elsewhere, with an empty `PATH` and another working directory, gives the same bytes.
- **`aethelgard-chaos` `tests/campaign.rs` §7a.** A test for every R2 rule and every kind-15 field rule.

## Measured

**Every pull-up and capacitance case's rise** (tier 2, `ANSWER 3100` lines; the ceiling is 300,000 ps throughout):

| Case | SCL `rise_30_70_ps` | SDA `rise_30_70_ps` | Past the ceiling |
|---|---|---|---|
| pullup-2k2 (reference) | 186,402 | 186,402 | no |
| **pullup-3k3** | **279,601** | **279,601** | no: inside, as the trap needs |
| **pullup-3k9** | **330,443** | **330,443** | yes: just over |
| pullup-4k7 | 398,230 | 398,230 | yes |
| pullup-10k | 847,298 | 847,298 | yes |
| pullup-47k | 3,982,301 | 3,982,301 | yes |
| pullup-100k | 8,472,980 | 8,472,980 | yes (complete) |
| cap-220p-2k2 (both nets 220 pF) | 410,089 | 410,089 | yes |
| cap-470p-sda-2k2 (SDA 470 pF) | 186,402 | 876,105 | SDA only |
| held-low-sda | 186,402 | (held, not measured) | the held fault |
| held-low-scl | (held, not measured) | 186,402 | the held fault |

The work order's "about 330 ns" at 3.9 kΩ and "about 280 ns" at 3.3 kΩ hold: 330.443 ns and 279.601 ns. Every rise is complete (`complete=true`), 100 kΩ included.

**`OQ-03`, re-measured with kind 15 (R6):**

| Run | Events | `trace.cbor` | Before kind 15 (2300) |
|---|---|---|---|
| `pullup-2k2` | 16 | 993 B | 15, 907 B |
| `pullup-47k` | 17 | 1,421 B | 16, 1,335 B |
| The two together (0.2 s simulated) | 33 | 2,414 B | 31, 2,242 B |

Kind 15 costs 86 B per run on the reference board: one event, which is most of it the two lines' names and the two devices. Every seeded run is between 993 and 1,453 B.

**Golden constants, re-pinned (R3).** Setting the version byte back to 2 gives the old values again, so nothing else changed.
- `GOLDEN_TRACE_SHA256`: `e1d3921371b057fe…c86e8` (version 2) to `a2d551d1ee8e92b88ab9ba3ee1827351d47e693ceee89cbe7451ef272964d166`.
- `GOLDEN_STATE_HASH`: `eb6408eee6c621da…93edc` to `0db4b4f9398d16db958343199c145a819ff219d3d891b78db56584d2a9310cea`.

**`simulate --firmware`, byte-identical to main (R4).**
- **Method.** origin/main (`034fcfd`) was extracted with `git archive` and built in the image with its own target directory, with its own bridge beside it. Then `simulate --platform boards/sentinel-01/sentinel-01.repl --firmware <reference elf> --spec examples/sentinel-01-minimal.json --for 100ms` ran twice with main's binary and twice with this branch's.
- **Result.** All four stdouts have sha256 `05c9ff739cd63e32a1d93b49d25537a1108aa6919ee2c654fb9d380c3db82c81`.

## What was built

1. **`aethelgard-chaos` (R1 to R3).**
   - **Kind 15 `I2cBusSolved`:** `bus`, `peripheral`, two lines (SCL then SDA, each `signal, net, pin, alternate_function, ceiling_ps`), and `devices` (`reference, address`, sorted).
   - **Its field rules.** The signals are the peripheral's `_SCL` and `_SDA`, the two nets and the two pins differ, and the references are sorted and unique.
   - **`Record::before_boot`** replaces `FIRST_BOOT_KIND`, because kind 15 is pre-boot but numbered after the boot kinds.
   - **`check_run`** ranks kind 15 after kind 6 and before any kind 7, and applies R2 in `check_joins`.
   - **`TRACE_VERSION` is 3.**
2. **The CLI's `campaign`:** `solved_buses` writes kind 15 from `analog_lines`, `assignments` and `i2c_devices`. A topology whose lines and devices disagree is an internal error, never a guess.
3. **`aethelgard-evolve`:** `attribute(&Replayed) -> Result<Attribution, AttributeError>`, with the rules, the symptom table and the evidence definitions in `crates/aethelgard-cli/README.md` `## attribute`. Its dependencies are now `aethelgard-chaos` and `aethelgard-svd-hal`. The stub's `aethelgard-ir` re-export is gone (`RUL-16`).
4. **The CLI's `attribute --archive`:**
   - `replay`'s checks are one function, `campaign::checked`, which `replay` and `attribute` both call. Its output is unchanged: the tier-1 `replay` tests pass as they were.
   - It prints one JSON line, exit 0 or 1.
   - Its MCP tool comes from the tree, and both tool-name pins list it.
5. **The seeded set:**
   - `scenarios/sentinel-01/seeded/`: nine scenarios in 2300's format, unchanged;
   - `cases.json`: 22 cases and the 9 mutants, each a `file`, `find` and `replace` with its reason.

## Findings and the forks taken

**Forks (decision-needed: ratify):**

- **F1 · `ceiling_ps` is per line, not per bus · `record.rs` kind 15 · low · format · decision-needed (ratify).**
  - R1 lists a bus-level `ceiling_ps`. But `analog_lines` holds the rise-time ceiling per net, from each net constraint (`max_rise_time_nanoseconds`), and the two lines of one bus may differ.
  - So each line carries its own `ceiling_ps`, and R2 checks every kind 7 and 8 against its line's.
  - `i2c_ceilings` (the bus's speed in Hz) is not archived. Attribution does not use it.
- **F2 · `TXDR` and `NBYTES` are evidence only after the transfer phase that writes them · `aethelgard-evolve` · high · correctness · decision-needed (ratify).**
  - **What was read.** In the probe archives, a TXIS timeout leaves `TXDR` at 0 and `CR2` holding the write phase: `NBYTES` 1, `RD_WRN` 0. That is so even in the unmutated 47 kΩ run.
  - **Why it matters.** Counted unconditionally, both would be "software evidence" on any timeout. A healthy bus with correct firmware and a device that does not answer would then be attributed `software`: the confident wrong answer.
  - **The gating built.**
    - `SADD` counts whenever `CR2` was read, since both phases write it.
    - `NBYTES` counts only once the record's status shows the read phase began (`ok`, `timeout_rxne`, `timeout_stopf`).
    - `TXDR` counts only once the write phase ran (`ok`, `timeout_tc`, `timeout_rxne`, `timeout_stopf`).
    - This mirrors the judge, which judges the witness only on `ok`.
  - **What it reads from the firmware.** The phases are the generated `transfer`'s, read from `boards/sentinel-01/src/board.rs`. A generator that changed its phase order would need this table changed with it.
  - **Where NBYTES's expectation comes from.** `NBYTES`'s expected value is the witness reading's own (the part table's width): neither kind 12 nor kind 15 carries the width.
  - **Consistency check.** A witness whose own `SADD` or `TXDR` expectation is not the solved address and register is no evidence either way.
- **F3 · Findings that no archived event shows · `aethelgard-evolve` · medium · design · decision-needed (ratify).**
  - **`locked_peripheral_access`** comes from Renode's log and has no event. It is explained by its archived cause: a kind 11 in the RCC block (base and size from the header) with an expected enable bit clear. Its count is not compared, because the judge emits one finding for any number of accesses. Without this, both clock-skipped cases would be `I1`, and the set would be 18 failing cases.
  - **`renode_error_log`, the reaper bounds and `pc_outside_firmware`** are `I1`, with the kind named in `unexplained`. `pc_outside_firmware` needs the firmware's executable segments, which the archive does not carry. It is a residual; no seeded case needs it.
- **F4 · The mutant cases match by findings, not by `matched` · tests, `cases.json` · low · interpretation · decision-needed (ratify).**
  - §1.4 runs the mutants "with the `pullup-2k2` scenario", whose `expect` is `pass`, so their campaign lines are not `matched`. Meanwhile §3.1 asks that "its own `expect` matches".
  - Read as built:
    - every case's failing kinds must equal its `cases.json` `findings`;
    - every unmutated case's campaign line must also be `matched`, which every electrical case's is;
    - the scenario format is unchanged.
- **F5 · Several symptoms, one record · `aethelgard-evolve` · medium · design · decision-needed (ratify).**
  - The record has one domain and one rule, but a run can fail in several places. Each symptom is decided on its own.
  - **How the run is decided:**
    - a passing verdict is `N1`;
    - any unexplained finding makes the run `I1`;
    - otherwise any indeterminate symptom makes it indeterminate by the lowest such rule;
    - electrical and software together are `I5`, never averaged;
    - otherwise the run takes the one domain, the lowest rule that decided any symptom, and every symptom's evidence.
  - **A symptom on no bus** (the boot, or a register no bus owns) uses every bus. Any fault gives `I3`, an unmeasured bus gives `I2`, and no bus at all gives `I2`, so a board with no measured electrical layer is never `software` by default.
- **F6 · The trace index is the event index · output · low · contract.** `index` counts events after the header. A campaign run holds only records, so it is also the position in `State::records`.
- **F7 · `replay`'s checks were extracted, not copied · `cli/src/campaign.rs` · low · fence · decision-needed (ratify).**
  - §1.3 asks `attribute` to apply exactly `replay`'s checks. R4 keeps `replay`'s logic read-only "beyond accepting kind 15".
  - The three checks (the reader, the run id against the directory name, `check_archived`) moved unchanged into `checked`, which both call. `replay`'s bytes are unchanged; the tier-1 `replay` tests pass unmodified.
  - A copy would have let the two drift.
- **F8 · Widths are RM0433 literals, positions are the header's · `aethelgard-evolve` · low · `RUL-04`.**
  - The header's `_Msk` macros are shifts the svd-hal reader does not evaluate. So `SADD` 10 bits, `NBYTES` 8, `MODER` 2 and `AFSEL` 4 are literals, with a `belt-and-suspenders:` note, as the twin's judge does.
  - Every position comes from the header: `I2C_CR2_SADD_Pos`, `I2C_CR2_NBYTES_Pos`, `GPIO_MODER_MODE<n>_Pos`, and `GPIO_AFR{L,H}_AFSEL<n>_Pos`. Whether a pin's AF is in `AFRL` or `AFRH` is which of the last two the header defines.
  - Every register address comes from the header too: `enable_bit`, `base`, `layout`.
- **F9 · `evolve` graduated: three stub checks retired · `workspace_shape.rs` · low · roadmap §6.**
  - With `aethelgard-evolve` out of `libraries()`, the list is empty, so these three checks retired with it:
    - `library_crates_declare_nothing_and_only_re_export`;
    - `library_crates_are_only_a_manifest_and_lib_rs`;
    - `library_crates_depend_only_on_allowed_crates`.
  - **What replaces them:** `evolve_has_only_its_known_files_keys_and_dependencies`. It fixes evolve's files and keys, and its dependencies at exactly `aethelgard-chaos` and `aethelgard-svd-hal`, so no `aethelgard-ir` edge exists (`RUL-16`).
- **F10 · The extra software cases · `cases.json` · low · scope.**
  - The work order names six mutants and asks for at least eight software cases. The six give seven with the 3.3 kΩ trap.
  - Added:
    - `sda-af-wrong`;
    - `scl-mode-output`, which exercises the `MODER` field;
    - `gpiob-clock-skipped`, which exercises a port's gate;
    - the wrong address at 2.2 kΩ beside the 3.3 kΩ trap.
  - That makes ten software cases, matching the ten electrical ones.
- **F11 · podman 5.7.0 writes `--memory-swap` straight into `memory.swap.max` · Furnace runner · medium · harness.**
  - Rootless podman 5.7.0 here sets `memory.swap.max` to the `--memory-swap` value itself, not to `memory-swap − memory`. Measured with `--memory 1g` and `--memory-swap` of `1g`: swap max 1 GiB. With `--memory-swap 2g`: 2 GiB. With no `--memory-swap`: 2 GiB.
  - So `--memory X --memory-swap X` allows X of swap on top of X of memory: with Furnace's 8 GiB swap file, a container could hold 24 GiB.
  - The runner writes 0 into the container's delegated `memory.swap.max` after start and reads all three caps back.
  - **Recommendation:** the global contract's podman line should say to write and read back `memory.swap.max`, not rely on `--memory-swap`.

## Tests changed

- **`aethelgard-chaos` `tests/archive.rs`:**
  - `an_unknown_version_is_refused`: the version byte is now 3, and 1, 2 and 4 are refused. 2 is new and is the R3 refusal.
  - `field_level_refusals_name_their_own_variant`: the first unknown kind is 16, since 15 is now a record.
- **`tests/golden.rs`:** re-pinned (above), with the old values recorded beside the new.
- **`tests/campaign.rs`:**
  - `good()` and `one_of_each()` carry kind 15. Index-based edits moved by one, because kind 15 sits at index 2.
  - The round-trip expects kinds 5 to 15.
  - `a_fault_on_the_wrong_bus_is_refused` and `a_held_low_fault_on_another_bus_is_refused` now first meet R2's refusal (no kind 15 for `OTHER`). Each also asserts the original refusal once `OTHER` is solved, so the original rule is still tested.
- **The CLI's `campaign_tier1.rs`:** `reseal` uses `before_boot`, not `kind() >= 10`.
- **`mcp.rs` and `mcp_tier1.rs`:** the tool list gains `attribute`, and the throwaway count goes from 10 to 11.
- **`workspace_shape.rs`:** F9.

## The fence

Changed:
- `aethelgard-chaos`, the kind, its rules, the run rules and the version (R4);
- the CLI's `campaign.rs` (`solved_buses`, `checked`, `attribute`) and `main.rs` (the subcommand);
- `aethelgard-evolve`;
- `scenarios/sentinel-01/seeded/`;
- tests;
- the chaos, CLI and evolve READMEs;
- the root `Cargo.toml` (the evolve workspace dependency), with `Cargo.lock` regenerated by cargo;
- this FEEDBACK.

Not touched: `simulate --firmware` (measured byte-identical), the judge (`renode.rs`, `aethelgard-twin`), the firmware generator, `boards/`, and `AET-WO-3100.md`.

## Adversarial pass (full, §5)

(pending)

## Mutation testing (sized to the diff)

(pending)

## Reduction pass

(pending)

## Gates

(pending)
