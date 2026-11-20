# AET-WO-1209c FEEDBACK — a power load never names a part whose current its own row already counts

Builder: Claude (Opus 5.5), a fresh `1209c` session, 2026-11-20, from ~08:50 UTC. Worktree `~/aethelgard-1209c`, branch `wo/aet-1209c-load-counted-once` from `origin/main` `a82b42b5`. Built to `~/.cache/aet-1209c-wo.md`, the architect's copy; `main` `bb1bcb8c` later carried the same text (`diff`: identical).

Labels: **measured** (run here), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1209c/`.

## 1. What is built

- **One kind, `LoadNamesCountedPart`,** in `aethelgard-ir` (`lib.rs`), beside `LoadNamesNoPart` [read].
  - Fields: `path` (`power.loads[i].reference`), `domain` (the load's), `reference`, and `counted_by`, the row that already counts the part.
  - `kind()`, `id()` (the reference) and `path()` carry it, as `LoadNamesNoPart` does.
  - Message: "`<path>` on \`<domain>\` names \`<reference>\`, whose current is already counted by `<counted_by>`".
  - `counted_by` is "the MCU's table row" or "regulator \`U<n>\`'s input on \`<input>\`".
  - **The name:** the work order's own. It joins `1209`'s `LoadNamesNoPart` and `LoadNamesUndrawnPart`; `1209b`'s `LoadPartOffRail` judges the drawn rail, a different question.
- **The check is in step 20a, `names::loads`,** after `LoadNamesNoPart`, entry by entry in document order [read].
  - An entry naming no part is refused as `LoadNamesNoPart`, as before, with unchanged bytes.
  - An entry naming a part is then refused if its reference is the MCU's (`mcu_reference`, exactly as `1209` accepts it) or a regulator's (`power.regulators[].reference`, exactly).
  - Nothing else in `power::references` is matched: feedback resistors, external parts, the sense divider, crossings and their pull-ups stay loads.
  - The rule is the one `power.rs`'s module doc and `Load::reference`'s doc already state; both docs are unchanged.
- **The IR README** gains the row it lacked for step 20a: one line, naming both of its kinds (F1).

## 2. Acceptance (§2)

| Item | Result |
|---|---|
| **The refusal, by name** | `ir/tests/load_references.rs` [measured]. `a_load_naming_a_regulator_is_refused_naming_its_input_row`: `U12` on `VDD_3V3` and on `VDD_1V8` (the `1209b` `adv/s02-U12-*` copies: the fixture plus one 1 mA load at index 13), each refused, with the full error value and message; `U10`, fed from the battery, on `VDD_5V`. `a_load_naming_the_mcu_is_refused_naming_its_table_row`: `U1`, and the fixture with its I2C device renamed `U1`, where the MCU becomes `U2`: `U1` passes and `U2` is refused. The binary on the `1209b` copies: `{"kind":"LoadNamesCountedPart","id":"U12","path":"power.loads[13].reference","message":"power.loads[13].reference on \`VDD_1V8\` names \`U12\`, whose current is already counted by regulator \`U12\`'s input on \`VDD_3V3\`"}`, exit 2; `main`'s binary exits 0 on both (`adv/u12.txt`). |
| **Order (§1.2)** | `a_regulator_s_own_parts_pass_and_no_part_is_judged_first`: a no-part entry before a regulator's gives `LoadNamesNoPart`, and the reverse gives `LoadNamesCountedPart`. Fork D2. |
| **Mutants (§1.3)** | Both killed [measured: `mut/m1.*`, `mut/m2.*`; tree restored from `mut/names.rs.orig`, `cmp`]. **M1, regulators only** (`if *r == mcu` → `if false`): fails `a_load_naming_the_mcu_is_refused_naming_its_table_row` (and, before the reduction pass, form (ii) of `each_form_of_a_part_the_board_has_passes`). **M2, the MCU only** (`.find(\|g\| g.reference == *r)` → `.find(\|_\| false)`): fails `a_load_naming_a_regulator_is_refused_naming_its_input_row` and `a_regulator_s_own_parts_pass_and_no_part_is_judged_first`. |
| **Nothing committed newly refused (§1.4, §7)** | Before building, no committed specification names the MCU or a regulator in `power.loads`: 5 files carry referenced loads, 26 references, 0 hits (`dump.txt`; the MCU check `1209b` left open is confirmed). `validate` over the 52 committed JSON files carrying `voltage_domains`: byte-identical between `main`'s binary (`a82b42b5`) and this branch's, 26 pass and 26 refused (`corpus-main.txt`, `corpus-branch.txt`, `cmp`). The 93 renamed and sized variants the tests build (`order_only` seeds 0–40 and `sized` 1, 2, 4, 6, 8 for Sentinel-03 and `sentinel-02-power`, plus `power_tree()`; `1209b`'s lesson: `board_tier2` prints a refusal and passes) all pass on both binaries, identically (`variants-main.txt`, `variants-branch.txt`). |
| **No emitted byte moves** | `emit` of `sentinel-01`, `sentinel-01-ext-bmp581`, `sentinel-02-bench`, `sentinel-02-power` and Sentinel-03 on both binaries: 15 files, sha256 identical, and identical to `1209b`'s `emit-main.sha` (`emit-main.sha`, `emit-branch.sha`). |
| **`TIER2_MIN`** | Unchanged. The new tests are in the existing tier-1 binary `load_references`; no test binary is added. |
| **Gates** | §6. |

## 3. Two expectations moved (D1)

Both asserted that a load naming the MCU passes. The work order now refuses it, so both move:

- `ir/tests/load_references.rs` `each_form_of_a_part_the_board_has_passes`, form (ii). It is now a pointer to `a_load_naming_the_mcu_is_refused_naming_its_table_row`, which asserts the refusal by kind, so it is not refused as no part (§6, reduction).
- `cli/tests/load_references_tier1.rs:84`: `U1` on `VDD_3V3` asserted exit 0, and now asserts exit 2 with `LoadNamesCountedPart`. The point the test makes, that the CLI never refuses the emitter's `U<n>` as undrawn, still holds: the IR answers first. *Fence note:* the fence says "tests" and "Not: the CLI"; this is a CLI test, not CLI code. Only that assertion and its comment move.

## 4. Adversarial pass, targeted (§5)

**A1. A part that is both a load and counted elsewhere is not refused.**

- **A sweep, on both binaries** [measured: `sweep.txt`, `sweep.py`]. For Sentinel-03 and `sentinel-02-power`, each candidate in turn becomes a 1 mA load on the MCU's supply. Any load already naming it is dropped, so the duplicate check does not mask it. The candidates are every reference the specification declares, `U1`–`U20`, `R1`–`R60`, `C1`–`C60` and `SW1`: 318 in all.
  - **Exactly 7 change kind,** each to `LoadNamesCountedPart`: the MCU (`U1`) on each board, `U10` and `U11` on each, and `U12` on Sentinel-02.
  - **The other 311 are identical** on both binaries: 79 pass, 84 `LoadPartOffRail`, 123 `LoadNamesUndrawnPart`, 25 `LoadNamesNoPart`. The 79 that pass are the parts drawn on `VDD_3V3`. They include:
    - `U11`'s divider top `R20` and its inductor `L2`, series chains (`1209b` §4 A2), and its external capacitors on the rail (`C30`, and others);
    - on Sentinel-02, the translator `U13` and its pull-up `R40`;
    - the emitter-numbered pull-ups.
  - **Parts on another rail keep `main`'s kind:** `U10`'s `R10` and `L1`, and the battery-sense divider `R50`/`R51`, stay `LoadPartOffRail`. The IR test names `R10`, `C10` and `L1` on `VDD_5V`, and the IR's `validate` passes each. The rail is the CLI's check, not the IR's.
- **A regulator's enable pull-up has no reference to name** [read]. A control is `strap` (`input`, `open`, `vcca`, `vccb`; `power_parts::Strap`) or `wire`, and neither carries a part. Any drawn pull-up is an emitter-numbered `R<n>`, which the sweep covers.
- **The rule matches `power.regulators[].reference` only.** Tests pin `R10`, `C10`, `L1`, `C30`, `C31`, `U13` and `R40` as passing.
- **One effect on the CLI's order:**
  - On `main`, a battery-fed buck named on `VDD_3V3` (`U10`, `U11`) was refused by the CLI's rail check, `LoadPartOffRail`.
  - It is now refused by the IR first, `LoadNamesCountedPart`, as §1.2's order puts it.
  - The CLI's code is unchanged.

**A2. The MCU named by a form `1209` accepts that this rule misses.** None found.

- `1209` accepts the MCU in one form: `mcu_reference`'s exact `U<n>`, the first the specification does not spell [read: `names::loads`].
  - Case and leading-zero spellings (`u1`, `U01`) are `LoadNamesNoPart`, before this rule (`1209`'s test).
  - `emitter_numbered(r, "")` never matches a `U`.
- The rule compares against the same `mcu_reference` value, so it moves with the numbering.
  - The test with a device renamed `U1` shows it: the MCU becomes `U2`; `U2` is refused and `U1` passes.
  - The sweep shows it too: `U2`–`U20` keep their `main` kinds on both boards.
- The domain does not matter: the MCU and each regulator are refused on every rail named (`U12` on both its rails, `U10` on `VDD_5V`).

## 5. Findings

| # | Finding | Kind |
|---|---|---|
| D1 | Two tests asserted that a load naming the MCU passes (`1209`'s form (ii) in the IR, and `1209b`'s CLI `U1` line); both now assert the refusal (§3). The CLI one is a test under "Not: the CLI". | decision-needed: accept the moved assertions |
| D2 | **Order: per entry, in document order** (each entry judged a part, then counted), as `1209b` judged `LoadNamesUndrawnPart` then `LoadPartOffRail`. "Judged first" could also mean all `LoadNamesNoPart` before any `LoadNamesCountedPart`. Only a specification with both errors tells them apart; the test pins the per-entry reading. | decision-needed: accept per entry |
| D3 | **A battery-fed regulator's counting row reads "regulator \`U10\`'s input on \`battery\`"**: `battery` is the spec's own spelling of the input (`power::BATTERY`), not a domain. No load can be on it, so the message names the battery as the row. | decision-needed: accept the wording |
| F1 | The IR README had no row for step 20a: `1209`'s `LoadNamesNoPart` was never listed. The one line allowed lists both kinds. | informational, within fence |
| F2 | `crates/aethelgard-cli/README.md`'s exit-code row lists `LoadNamesNoPart` and `1209b`'s kinds, but not this one. It is outside this fence (`1209b` F3 widened its fence to that clause). The row's IR kinds are introduced "such as", so it is not wrong, only incomplete. | backlog: one clause, next CLI order |
| F3 | On the output rail the refusal names the regulator's input row, as §1.1 says. The truer reason there is that the regulator is the rail's source (the variant's doc says both). The message gives one row for both rails. | informational |

## 6. Gates, the reduction pass, and CI

- **`cargo test --workspace`:** 2,362 passed, 0 failed, 0 ignored [measured: `test-workspace.log`, in a capped user scope, 32 GiB, swap 0]. That run preceded the reduction pass below, which changed `ir/tests/load_references.rs` only. On the final tree, `load_references` passes 5 of 5 [measured: `mut/base-after.log`].
- **`cargo fmt --all -- --check`** and **`cargo clippy --workspace --all-targets -- -D warnings`:** exit 0 on the final tree [measured: `clippy-final.log`].
- **Reduction pass (diff): −13 lines net · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 0 findings below.** Run inline by the builder, not by a fresh subagent: the diff is about 45 source lines.
  - **`with_load_on` restated `with_load`** with a domain. `with_load` now calls it.
  - **Form (ii) repeated the MCU test's first assertion.** It is now a one-line pointer.
  - **Proof:** after both, `load_references` passes and both mutants are still killed [measured: `mut/m1-after.log` fails the MCU test, `mut/m2-after.log` fails both regulator tests].
  - **Considered, not cut:** the path `format!` is written once per arm of the two refusals, as `1209` wrote it. A shared closure is not fewer lines.
- **`main` merged:** `bb1bcb8c`, docs only (the roadmap; this work order, identical to the copy built to; `4405`'s).
- **CI:** see the PR. This file's READY commit names the run.
