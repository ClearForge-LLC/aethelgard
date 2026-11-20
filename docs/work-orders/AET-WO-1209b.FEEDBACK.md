# AET-WO-1209b FEEDBACK — a power load's part must have a pin on the load's rail

Builder: Claude (Opus 5.5), a fresh `1209b` session, 2026-11-20, from ~05:00 UTC. Worktree `~/aethelgard-1209b`, branch `wo/aet-1209b-load-on-rail` from `origin/main` `416a6dd3`. The work order is read from `~/.cache/aet-1209b-wo.md`, the architect's copy of the docs branch's `docs/work-orders/AET-WO-1209b.md`. This branch does not commit it.

Labels: **measured** (run here), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1209b/`.

## 1. What is built

- **One read-only accessor, `aethelgard_eda::schematic::drawn_nets`** (`eda/src/schematic.rs`, beside `drawn`) [read].
  - It maps each drawn part's reference to every net and rail its pins are drawn on, with no-connects left out.
  - A domain appears by its id and ground as `GND`, as `Builder::rail` and `Builder::ground` draw them.
  - It is read from `design()`, the same parts the sheet, the BOM and the board are emitted from.
  - The emitter's behaviour is unchanged.
- **The refusal, `LoadPartOffRail`, in the CLI's `validate`** (`cli/src/main.rs` `loads_drawn`) [read].
  - For each `power.loads` entry with a reference, the part as drawn must have a pin on the entry's `domain`.
  - The refusal carries `id` (the reference) and `error.path` (`power.loads[i].reference`).
  - Its message names the rail and every net the part's pins are on, sorted.
  - The check runs on the solve `validate` already runs (`quiet_solve`, shared with `outline_holds`). The second §7 condition does not arise.
- **`LoadNamesUndrawnPart` is unchanged.** It is still judged first, for each entry, from the same map. Its message bytes, and `R40`'s and `R99`'s test, are unchanged.
- **Forms covered.** Every form a reference can take reaches the rule: a declared reference, the MCU's `U<n>`, the reset switch's `SW<n>`, and an emitter-numbered `R<n>`/`C<n>`. This is because `LoadNamesUndrawnPart` has already required each to be a drawn part.
  - A load with no reference is out of scope, as before.
  - No reference form names an off-board part, and no drawn part has zero connected pins in the two powered designs [measured: `parts.txt`; every test point has one net]. So §1.2's case does not arise today.
  - **What the code does if it ever does:** a drawn part with no connected pin is refused as `LoadPartOffRail`, with an empty net list. It is not skipped. §1.2 calls the case out of scope; refusing is the `INV-06` reading, because a load whose part touches nothing has no rail to be on. **D2** below asks for a ruling.
- **Documentation:** `crates/aethelgard-cli/README.md`'s exit-code row names `LoadPartOffRail` beside `LoadNamesUndrawnPart`, as `1209` documented its kinds. *Fence note:* the work order's fence lists `cli/src`, not the crate README. The README is the CLI's documented exit-code contract (`main.rs:3`, `validate_cli.rs:1`), so one clause is added and flagged here.

## 2. Acceptance (§2)

| Item | Result |
|---|---|
| **The refusal, by name** | `cli/tests/load_references_tier1.rs` `a_load_naming_a_part_with_no_pin_on_its_rail_is_refused`. Sentinel-03 as committed exits 0: its `R6` and `R7` are on `VDD_3V3` [measured]. The copy with the `R7` row renamed to `R8` exits 2, `LoadPartOffRail`, `id` `R8`, `path` `power.loads[6].reference`, with message: "power.loads[6].reference on \`VDD_3V3\` names \`R8\`, which the emitter draws with no pin on \`VDD_3V3\`: its pins are on \`BOOT0\`, \`GND\`". A part on another rail (`R10`, on `U10_FB` and `VDD_5V`) is refused the same way. |
| **Mutants (§1.3)** | Two, both killed by the second test (the R8 copy exits 0 where 2 is asserted) [measured: `mut/m1.log`, `mut/m2.log`]. **M1**, checking only that the part exists (the rail guard made `false`), fails all three tests in the file. **M2**, checking that any pin is on any rail (`GND` or any declared domain), also fails all three. Under M2 the `R8` copy passes because `R8` touches `GND`. The tree is restored from a saved copy, compared with `cmp`. |
| **Nothing committed newly refused** | 52 committed JSON files carry `voltage_domains`, `1209`'s filter. `validate` output is byte-identical between `main`'s binary (`416a6dd3`) and this branch's: 26 pass and 26 are refused, as before [measured: `corpus-main.txt`, `corpus-branch.txt`, `cmp`]. Every referenced load in a committed specification is on its rail [measured: `dump.txt`; Sentinel-03's 13 and `sentinel-02-power`'s 11, including `U13` and `R40`]. **The renamed specifications the tests build** were also checked, because `order_only` permutes device references and with them the emitter's `R<n>` numbering. `board_tier2`'s size samples print a `validate` refusal and still pass, so a new refusal there would not fail CI. 93 variants were generated with the tests' own helpers: `order_only` seeds 0–40 and `sized` 1, 2, 4, 6 and 8 for each of Sentinel-03 and `sentinel-02-power`, plus `power_tree()`. All 93 pass, byte-identical on both binaries [measured: `adv/variants/`]. |
| **No emitted byte moves** | `emit` of Sentinel-03, `sentinel-02-power`, `sentinel-02-bench`, `sentinel-01` and `sentinel-01-ext-bmp581` on both binaries: 15 files, sha256 identical [measured: `emit-main.sha`, `emit-branch.sha`]. `loads_drawn` is called only from `validate()`. `verify` and the bench package do not reach it [read], so the package cannot move. CI's bench-package compare is the run of record. |
| **Gates** | See §6. |
| **`TIER2_MIN`** | Unchanged at 194. No tier-2 test is added; the new tests sit in the existing tier-1 binary `load_references_tier1`, so `ci/docs-tests.tsv` needs no row [measured: `bash ci/route.sh check ci/docs-tests.tsv`: inventory ok]. |

## 3. One expectation moved (D1)

`1209`'s `a_load_naming_a_resistor_the_emitter_does_not_draw_is_refused` added a load naming `SW1` on `VDD_3V3` (with `mcu.reset_switch`) and asserted exit 0.

- The reset switch is drawn on `NRST` and `GND` only [measured]. So this rule refuses that load, correctly: a switch draws no current from a domain.
- No committed specification carries such a load. The test builds it.
- **The assertion now reads:** `SW1` is drawn, so it is not refused as `LoadNamesUndrawnPart`, but it is refused as `LoadPartOffRail`. The `U1` load still passes on its own.
- `1209`'s point, that the CLI never refuses an emitted name as undrawn, still holds.
- **The consequence, for ruling (D1):** `1209`'s ruled reset-switch form (iii) now passes the IR's step 20a but can never pass the CLI's `validate` on any domain. A load's domain is a declared voltage domain, never `GND` (`ir/src/power.rs` `known`), and `NRST` is not a domain. I read that as correct, since a pushbutton to ground draws nothing from a rail. The alternative, keeping the form's loads valid, would need the switch exempted from this rule.

## 4. Adversarial pass, targeted (§5)

**A1. A part with a pin on the rail that is not its supply pin.**

- **A pull-up's far end is a correct load.** Take `R1` (`I2C_SDA`, `VDD_3V3`). A two-terminal passive's current enters at one pin and leaves at the other. While the line is driven low, it leaves the rail through `R1`'s `VDD_3V3` pin. So "has a pin on the rail" is exactly where its current comes from. The same holds for `R3`, `R4`, `R6`, `R7`, `R20`, and every decoupling `C<n>` (leakage) [measured nets: `parts.txt`].
- **For an IC, a pin on the rail need not be a supply pin.** The rule accepts any pin, so it is a necessary condition, not a sufficient one.
- **In the two powered designs, only two parts touch more than one domain** [measured]:
  - **The translator `U13`** (`VCCA`/`VCCB` on `VDD_3V3` and `VDD_1V8`) draws from both. A load on either rail is correct, and passes.
  - **The LDO `U12`** has its input on `VDD_3V3` and its output on `VDD_1V8`.
    - A load naming `U12` on `VDD_1V8`, the rail it makes, passes [measured: `adv/s02-U12-VDD_1V8.json`, exit 0]. So does one on `VDD_3V3`, its input.
    - A regulator's own current is counted by its part row (`ir/src/power.rs` `Load`'s doc: "other than the MCU and the child regulators, which their part rows count") [read]. So a load naming it would count it twice on its input, and would be wrong on its output [inferred: the supply gate was not run on these copies]. **F1** below.
- No other part in either design touches more than one domain [measured]. The MCU is on `VDD_3V3` only. Each connector has one domain pin (pin 1) [read: Sentinel-03's connector rows]. The rows carry nets, not roles. That pin is the supply or sense pin by the loads' own `source` text (`J2` "the debug probe's target-voltage sense", `J3` "the EXT port's budget") [read]; that it is the pin the current flows through is [inferred].

**A2. A load on a rail the part reaches only through another part.**

- **`sentinel-02-power`'s ICM-20948 (`U5`)** sits behind `U13` on `VDD_1V8`. `U12` makes `VDD_1V8` from `VDD_3V3`.
  - A load naming `U5` on `VDD_3V3` is refused, `LoadPartOffRail` [measured: test `a_part_reached_through_another_part_is_not_on_that_rail`].
  - This is correct: its current is on `VDD_1V8`. `VDD_3V3` sees it only through `U12`, whose row counts its own input.
- **Other cases the rule refuses, also correctly:**
  - the buck `U11` named on `VDD_3V3` (reached through `L2`; a regulator, counted by its row);
  - Sentinel-03's `R5`, the BMP581's SDO pull-down, named on `VDD_3V3`. Any current it carries comes from `U7`'s SDO driver, so it is `U7`'s.
- **The general case is a series chain.** The rule makes the load name the part that touches the rail; naming the farther part is refused rather than guessed (`INV-06`).
  - **The designs do have chains:** the feedback dividers (`R20`/`R21`, `R10`/`R11`), the battery sense divider (`R50`/`R51`), and each buck's inductor (`L1`, `L2`) [measured: `parts.txt`]. There is no LED.
  - Sentinel-03 already names a divider's whole current on its top resistor: `R20`, "3.465 V / (105 kOhm + 46.4 kOhm)". The same load renamed to the bottom resistor `R21` (`GND`, `U11_FB`) is refused, `LoadPartOffRail` [measured: `adv/s03-R20-as-R21.json`].
  - So the rule fixes a convention the committed specification already follows: a chain's load is named on the part at the rail.

## 5. Findings

| # | Finding | Kind |
|---|---|---|
| D1 | The reset switch's `SW<n>` (`1209`'s form (iii)) can never pass the CLI's rail check (§3). Accept, or exempt the switch? | decision needed |
| D2 | A drawn part with no connected pin is refused (empty net list), not skipped (§1). Accept, or skip? No current form reaches it. | decision needed |
| F1 | A load naming a regulator passes on its input and on its output rail (`U12` on `VDD_3V3` or `VDD_1V8`) [measured], though a regulator's current is counted by its part row [read]. A rule "a `power.loads` reference names no regulator" would close it. Nothing committed names one [measured: `dump.txt`]. | §7.1 row, the architect's |
| F2 | `R7` (a load named by its emitted number, on a part that is still on the rail) stays unseen, as the work order says. This rule is the partial catch only. | §7.1 row (existing) |
| F3 | The crate README is outside the literal fence; one clause names the new kind (§1). | fence note |
| F4 | `schematic::drawn`, `anchored` and `drawn_nets` each re-run `design()` for one view of the same parts. `drawn` now has one caller, a test (`hot_plug_tier1.rs`). One read-only view of the drawn parts would serve all three. Outside this fence (eda beyond one accessor). | reduction finding |

## 6. Gates, the reduction pass, and CI

- **`main` merged:** `c9c5f85c` (docs only: the roadmap, and this work order, identical to the copy built to) as `7c41b6e6`. Clean [measured: `git diff --stat 416a6dd3 c9c5f85c`].
- **`cargo test --workspace`:** 2,351 passed, 0 failed, 0 ignored [measured: `test-workspace.log`, run in a capped scope]. That run began before the reduction pass below and the README clause. After both, these passed on the final tree: `load_references_tier1`, `validate_cli`, `readme_status`, `hot_plug_tier1`, `outline_tier1`, and `aethelgard-eda`'s unit tests (177 tests; with the five CLI suites, 216).
- **`cargo fmt --all -- --check`** and **`cargo clippy --workspace --all-targets -- -D warnings`:** exit 0 on the final tree.
- **The corpus and `emit`, re-measured with the final binary:** byte-identical to `main` (`corpus-final.txt`, `emit-final.sha`).
- **Reduction pass (diff): +1 line net · 0 duplicates removed · 1 dead path removed · 0 protected kept · 1 finding below (F4).** The pass was run inline by the builder, not by a fresh subagent: the diff is about 60 source lines.
  - **The dead path:** `drawn_nets` merged entries for a reference drawn twice. `design()` claims each reference once (`Taken`), and `Part` has no unit, so it is now a plain `collect` like `drawn`. Rustfmt makes it one line longer.
  - **Proof:** both mutants were re-run after the change and are still killed [measured: `mut/m1-after.log`, `mut/m2-after.log`].
  - **Not removable:** the one accessor is the fence's; the two refusals share one map, so `validate` no longer calls `drawn`; the three tests hold the work order's cases and §5's measured answer.
- **CI on `2cb8edb2` is green** [measured]:
  - runs: `ci` 44682553849, `ardupilot-image` 44682553669, `engine-image` 44682553737;
  - 33 jobs pass, and 3 are skipped (`publish` ×2, `quickstart`), as on every PR;
  - `bench-package` passes against the committed package;
  - `tools`: 2,576 tests passed, tier-2 194 against a floor of 194.
- `main` is still `c9c5f85c`, merged. This READY commit changes this file only.
