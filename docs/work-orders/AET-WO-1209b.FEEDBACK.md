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
  - No drawn part has zero connected pins in the two powered designs [measured: `parts.txt`; every test point has one net]. No reference form names an off-board part: §1.2's case does not arise, and the rule needs no exemption for it.
- **Documentation:** `crates/aethelgard-cli/README.md`'s exit-code row names `LoadPartOffRail` beside `LoadNamesUndrawnPart`, as `1209` documented its kinds. *Fence note:* the work order's fence lists `cli/src`, not the crate README. The README is the CLI's documented exit-code contract (`main.rs:3`, `validate_cli.rs:1`), so one clause is added and flagged here.

## 2. Acceptance (§2)

| Item | Result |
|---|---|
| **The refusal, by name** | `cli/tests/load_references_tier1.rs` `a_load_naming_a_part_with_no_pin_on_its_rail_is_refused`. Sentinel-03 as committed exits 0: its `R6` and `R7` are on `VDD_3V3` [measured]. The copy with the `R7` row renamed to `R8` exits 2, `LoadPartOffRail`, `id` `R8`, `path` `power.loads[6].reference`, with message: "power.loads[6].reference on \`VDD_3V3\` names \`R8\`, which the emitter draws with no pin on \`VDD_3V3\`: its pins are on \`BOOT0\`, \`GND\`". A part on another rail (`R10`, on `U10_FB` and `VDD_5V`) is refused the same way. |
| **Mutants (§1.3)** | Two, both killed by the second test (the R8 copy exits 0 where 2 is asserted) [measured: `mut/m1.log`, `mut/m2.log`]. **M1**, checking only that the part exists (the rail guard made `false`), fails all three tests in the file. **M2**, checking that any pin is on any rail (`GND` or any declared domain), also fails all three. Under M2 the `R8` copy passes because `R8` touches `GND`. The tree is restored from a saved copy, compared with `cmp`. |
| **Nothing committed newly refused** | 52 committed JSON files carry `voltage_domains`, `1209`'s filter. `validate` output is byte-identical between `main`'s binary (`416a6dd3`) and this branch's: 26 pass and 26 are refused, as before [measured: `corpus-main.txt`, `corpus-branch.txt`, `cmp`]. Every referenced load in a committed specification is on its rail [measured: `dump.txt`; Sentinel-03's 13 and `sentinel-02-power`'s 11, including `U13` and `R40`]. |
| **No emitted byte moves** | `emit` of Sentinel-03, `sentinel-02-power`, `sentinel-02-bench`, `sentinel-01` and `sentinel-01-ext-bmp581` on both binaries: 15 files, sha256 identical [measured: `emit-main.sha`, `emit-branch.sha`]. `loads_drawn` is called only from `validate()`. `verify` and the bench package do not reach it [read], so the package cannot move. CI's bench-package compare is the run of record. |
| **Gates** | See §6. |
| **`TIER2_MIN`** | Unchanged at 194. No tier-2 test is added; the new tests sit in the existing tier-1 binary `load_references_tier1`, so `ci/docs-tests.tsv` needs no row [measured: `bash ci/route.sh check ci/docs-tests.tsv`: inventory ok]. |

## 3. One expectation moved

`1209`'s `a_load_naming_a_resistor_the_emitter_does_not_draw_is_refused` added a load naming `SW1` on `VDD_3V3` (with `mcu.reset_switch`) and asserted exit 0.

- The reset switch is drawn on `NRST` and `GND` only [measured]. So this rule refuses that load, correctly: a switch draws no current from a domain.
- No committed specification carries such a load. The test builds it.
- **The assertion now reads:** `SW1` is drawn, so it is not refused as `LoadNamesUndrawnPart`, but it is refused as `LoadPartOffRail`. The `U1` load still passes on its own.
- `1209`'s point, that the CLI never refuses an emitted name as undrawn, still holds.

## 4. Adversarial pass, targeted (§5)

**A1. A part with a pin on the rail that is not its supply pin.**

- **A pull-up's far end is a correct load.** Take `R1` (`I2C_SDA`, `VDD_3V3`). A two-terminal passive's current enters at one pin and leaves at the other. While the line is driven low, it leaves the rail through `R1`'s `VDD_3V3` pin. So "has a pin on the rail" is exactly where its current comes from. The same holds for `R3`, `R4`, `R6`, `R7`, `R20`, and every decoupling `C<n>` (leakage) [measured nets: `parts.txt`].
- **For an IC, a pin on the rail need not be a supply pin.** The rule accepts any pin, so it is a necessary condition, not a sufficient one.
- **In the two powered designs, only two parts touch more than one domain** [measured]:
  - **The translator `U13`** (`VCCA`/`VCCB` on `VDD_3V3` and `VDD_1V8`) draws from both. A load on either rail is correct, and passes.
  - **The LDO `U12`** has its input on `VDD_3V3` and its output on `VDD_1V8`.
    - A load naming `U12` on `VDD_1V8`, the rail it makes, passes [measured: `adv/s02-U12-VDD_1V8.json`, exit 0]. So does one on `VDD_3V3`, its input.
    - A regulator's own current is counted by its part row (`ir/src/power.rs` `Load`). A load naming it double-counts on its input, and is wrong on its output. **F1** below.
- No other part in either design touches a domain on a non-supply pin. The MCU is on `VDD_3V3` only, and each connector's domain pin is its supply or sense pin [measured nets; the pin roles read from the specification's connector rows].

**A2. A load on a rail the part reaches only through another part.**

- **`sentinel-02-power`'s ICM-20948 (`U5`)** sits behind `U13` on `VDD_1V8`. `U12` makes `VDD_1V8` from `VDD_3V3`.
  - A load naming `U5` on `VDD_3V3` is refused, `LoadPartOffRail` [measured: test `a_part_reached_through_another_part_is_not_on_that_rail`].
  - This is correct: its current is on `VDD_1V8`. `VDD_3V3` sees it only through `U12`, whose row counts its own input.
- **Other cases the rule refuses, also correctly:**
  - the buck `U11` named on `VDD_3V3` (reached through `L2`; a regulator, counted by its row);
  - Sentinel-03's `R5`, the BMP581's SDO pull-down, named on `VDD_3V3`. Any current it carries comes from `U7`'s SDO driver, so it is `U7`'s.
- **The general case is a series chain**, such as an LED behind a resistor. The rule makes the load name the part that touches the rail. Naming the farther part is refused rather than guessed (`INV-06`). The two powered designs have no such chain: every LED-like or series part is absent from `parts.txt` [measured].

## 5. Findings

| # | Finding | Kind |
|---|---|---|
| F1 | A load naming a regulator passes on its input and on its output rail (`U12` on `VDD_3V3` or `VDD_1V8`), though a regulator's current is counted by its part row. A rule "a `power.loads` reference names no regulator" would close it. Nothing committed names one [measured: `dump.txt`]. | §7.1 row, the architect's |
| F2 | `R7` (a load named by its emitted number, on a part that is still on the rail) stays unseen, as the work order says. This rule is the partial catch only. | §7.1 row (existing) |
| F3 | The crate README is outside the literal fence; one clause names the new kind (§1). | fence note |

## 6. Gates, the reduction pass, and CI

- **`main` merged:** `c9c5f85c` (docs only: the roadmap, and this work order, identical to the copy built to) as `7c41b6e6`. Clean [measured: `git diff --stat 416a6dd3 c9c5f85c`].
- **`cargo test --workspace`:** 2,351 passed, 0 failed, 0 ignored [measured: `test-workspace.log`, run in a capped scope]. That run began before the reduction pass below and the README clause. After both, these passed on the final tree: `load_references_tier1`, `validate_cli`, `readme_status`, `hot_plug_tier1`, `outline_tier1`, and `aethelgard-eda`'s unit tests (177 tests; with the five CLI suites, 216).
- **`cargo fmt --all -- --check`** and **`cargo clippy --workspace --all-targets -- -D warnings`:** exit 0 on the final tree.
- **The corpus and `emit`, re-measured with the final binary:** byte-identical to `main` (`corpus-final.txt`, `emit-final.sha`).
- **The reduction pass:** `drawn_nets` merged entries for a reference drawn twice. `design()` claims each reference once (`Taken`), and `Part` has no unit, so it is now a plain `collect` like `drawn`. Both mutants were re-run after it and are still killed [measured: `mut/m1-after.log`, `mut/m2-after.log`]. Nothing else in the diff is removable:
  - the one accessor is the fence's;
  - the two arms of one `match` share one map;
  - the three tests hold the work order's cases and §5's measured answer.
- **CI** is the run of record; its result is in the PR.
