# AET-WO-4310 reduction pass: read-only proposals (diff adbd5c66..HEAD)

Line numbers are at HEAD. Nothing edited; no builds run. "WO" = inside this WO's diff; "pre" = pre-existing subsystem code.

## Proposals, best first

1. supply.rs: four copies of "largest bounded pin or None" (Q1). WO + pre.
   - Copies: closure `worst` in `drop_bound` (~1923-1933), closure `worst` in `drop_bound_shared` (~1949-1959), closure `largest` in `supplies` (~1785-1795), and the new `fn largest_bound` (~1970-1980, added by this WO as a fourth copy, not a replacement).
   - Do: delete the three closures and call `largest_bound` in all three places.
   - Net: about -30 lines.
   - Risk: supply.rs unit tests `the_drop_is_the_worst_pair_times_the_load` (~3073) and the shared-module tests. Mutant: make `largest_bound` return `min` instead of `max`, or `Refused` map to `Some(0)`. It is a pure move.

2. supply.rs: `drop_bound` and `drop_bound_shared` are one formula (Q3). pre.
   - `drop_bound_shared(.., None)` equals `drop_bound`: `rest = load`, no `own` term. Make one fn taking `Option<&Shared>`. That also removes the `match &shared` at ~1809.
   - Both are `pub`, but grep finds no caller outside supply.rs.
   - Net: about -12 lines.
   - Risk: the drop-bound tests, plus a mutant that drops the `checked_sub` on the shared path.

3. supply.rs: the stray doc line (defect, not a reduction). WO.
   - `/// The note a drop within 1.5x ...` (~1968) now sits above `largest_bound` instead of `pitch_note` (~2108). The WO inserted the new fns between a doc line and its fn. Move it back.
   - Do this with item 1: the same hunk is touched.

4. supply.rs: ground-pad lookup written twice in `supplies` (Q1). WO + pre.
   - The shared-module closure `of(net)` (~1779) and the WO's closure `ground(reference)` (~1818) are the same `pads_of(board, ref).filter(net == X)`.
   - Do: one `pads_on(reference, net)` helper. `ground(r)` becomes `pads_on(r, gnd)`.
   - Net: about -6 lines.
   - Risk: low. Covered by the sentinel gate tests and the shared-module test.

5. supply.rs: `entry_bounds` mints a `u64::MAX` sentinel that `charge` immediately decodes (Q2/Q3). WO.
   - `entry_bounds` maps `PinResult::Refused` to `Ok(u64::MAX)` (~2000). `charge` then matches `Ok(v) if v != u64::MAX` and `Ok(_) => unread.push("a pad refused")` (~2056-2058).
   - Do: return `Err(format!("{}: refused", p.number))` from `entry_bounds`. This deletes the sentinel and one match arm in `charge`.
   - Also: the two identical closures `&|ps| entry_bounds(board, &read.routed, gnd, ret_pad, ps, &model)` (~1855, ~1888) become one `let solve = ...;`.
   - Net: about -8 lines.
   - Risk: the `charge` unit tests (unread paths) and the `Other.unread` text. The existing "a pad refused" string would change, so check that no test pins it. Mutant: swap the Err arm to push nothing.

6. supervisor.rs: `devices()` (148-182) duplicates the I2C/SPI loop inside `powered()` (259-287) (Q1). WO (powered) + pre (devices).
   - Both walk `declared_i2c_devices` and `declared_spi_devices`, look up the bus speed (`hz`) by bus id, and call `parts.get(part).supply_min_at(hz.unwrap_or(MAX))`.
   - `powered()` itself repeats the i2c and spi hz lookup as two near-identical `for` loops.
   - Do: one private iterator `bus_devices(spec) -> Vec<(ref, part, rail, Option<u32> min)>`, chaining i2c and spi. Both `powered()` and `check` use it, and `check` filters `rail == mcu.supply`.
   - One semantic difference to keep: `devices()` skips a part absent from parts.json (`continue`), while `powered()` gives `min: None`. In `check` that part would then read `None -> u32::MAX -> refuse`, where it is skipped today. `declared_*_devices` parts are presumably validated earlier, so this may be unreachable. Confirm with a test before merging, or keep the filter `min.is_some()` in `check`.
   - Net: about -25 lines.
   - Risk: `readiness_4300k.rs` Path B tests (the `SupervisorBelowDeviceFloor` cases) and `supply_floors_4310.rs`. Mutant: invert the rail filter in `check`.

7. supervisor.rs: `floors_above_reset` (~478) and `named` (~491) are test-only (Q2). WO.
   - Production readiness uses only `judged()` (readiness.rs:169, which filters Floor and Named itself). `supervisor::named` and `floors_above_reset` are called only from `tests/readiness_4300k.rs` and `tests/supply_floors_4310.rs`.
   - Do: delete both fns (about 22 lines) and give the two test files one local helper over `judged`. Or keep one generic `fn pick<T>(spec, f)`. Net: about -12 after the test helper.
   - Caution: they are `pub`, and the module doc names them (lines 6-13). If the architect wants a stable public API for them, label them "keep: public API" instead.
   - Risk: none in production. The tests must still import the same assertions.

8. radio.rs `not_judged` repeats the sentences `judged` already builds (Q1). WO.
   - `not_judged` (radio.rs ~975-1002) restates two judged texts for the module's own part:
     - "...is held by no monitor: nothing holds `{rail}`'s consumers in reset or off, so it may run out of its range while the MCU runs", the same sentence as supervisor.rs:392.
     - "...above the reset's falling minimum ... may run out of its range before the reset holds", the same as `Floor::text`.
   - The radio min and citation are also re-derived from the radio table, as `powered()` already does at supervisor.rs:288-301.
   - Do: replace both branches and the `reset_floor` call with a lookup in `supervisor::judged(spec)` for the entry whose `device == m.reference`. Append its `Floor::text()` or `Named::text()`.
   - Net: about -15 lines, and one wording becomes the single source.
   - Risk: output wording changes (", unjudged" and "named (AET-WO-4310 (2))" tails). Check `supply_floors_4310.rs:178` and the cli README (`crates/aethelgard-cli/README.md`), which quote the text, and any golden package or FAB-NOTES that includes the radio line. If any is pinned, this is not worth a re-ruling. Classify as "do only if goldens are unaffected".

9. power.rs `protection`: the pack-series clause is built twice (Q1). WO.
   - `series.as_ref().map_or_else(String::new, |s| format!(" and the pack's series {r_pack} uOhm ({})", s.source))` appears in the `input_path_resistance_at_25c` condition (~2858) and the following `ctx.note` (~2866).
   - Do: bind it once as `pack_txt`.
   - Net: about -2 lines and one fewer copy of the formula.
   - Risk: none. Same strings, so the existing headroom and protection tests catch a typo.

10. supervisor.rs and power_parts: repeated literal `"crates/aethelgard-ir/data/power-parts.json"` in `reset_floor_against` (5 times, lines 92-139) (Q1). Mostly pre (the WO added none).
    - Do: `const TABLE: &str`.
    - Net: about -4 lines (net after adding the const), and one copy of the path.
    - Marginal. Include only if the WO is already touching `reset_floor_against`, which it is not. Low value; the scope test says skip.

11. verify.rs and ledger.rs: "is this part modelled by the twin" is written inline twice (Q1). WO + pre.
    - verify.rs `coverage` (~1962-1968) has a local closure `modelled`, and `ledger.rs:2319` has `aethelgard_twin::model::kind(part)...is_none()`. A third use is the twin's own test.
    - The twin's own split is `model::kind(part)`. Do: add `pub fn modelled(part) -> bool` next to it in `aethelgard-twin/src/model.rs` and use it in both. The ledger one propagates the error with `?`, so keep the error path there or have `modelled` return `Result<bool>`.
    - That is a new fn to remove two inline copies, so net is about -3 lines. Per the guardrail, a new layer is not a reduction. Treat as optional, likely NOT worth doing. Note that the two sites also differ on error handling (verify swallows the error with `.ok()`, ledger propagates), so merging them would change behaviour. The "unmodelled -> not witnessed" rule is, however, stated in verify.rs and ledger.rs. If left, add a one-line cross reference.

12. verify.rs `coverage`: the `witnessed` match repeats the device-by-bus walk done on the line above (Q1). WO.
    - `used` comes from `topology.*_devices.values().any(|d| d.bus == b.id)`, and `witnessed` walks `spec.declared_*_devices()` for the same bus.
    - Do: compute `used` and `witnessed` together from the topology devices with `d.part` (the topology device has `.part`, as ledger.rs:2297+ uses). Fold the `(used, target)` tuple into `(used, witnessed, target)` inside the two `.map` closures, and delete the `match &target` block (about 10 lines).
    - Net: about -8 lines.
    - Risk: `a_bus_of_unmodelled_devices_alone_is_named_uncovered` and `each_motor_output_is_a_target...`. The topology's device set must equal the declared set. If the topology drops a declared device, "used" semantics change. Verify with the spi-only fixture. Mutant: make `witnessed` always true.

## Judged NOT worth doing

- Gate `SupplySinkUncounted` loop (supply.rs ~1543-1560) vs the new `others` consumer predicate (~1829-1842). The predicates share `!Device:C`, `!Device:L`, `!TEST_POINT` and the regulator-input exclusion. They differ deliberately: the gate loop excludes `MCU_` parts and the domain's own regulator output, because those are counted elsewhere. The `others` loop includes the MCU, which is the principal consumer, and excludes the other regulator's own reference. A shared `draws_current(part, rail, flags)` would need a flag and an exclusion list, so it adds a mechanism and saves about 5 lines. If kept as is, add a comment cross-referencing each (label, do not remove).
- power.rs `radio_lines` `release` vec and the `lines` vec. They are parallel tuples; `release` carries an extra `banded` flag. Merging means one loop judging both with a per-line error prefix and note, and the two use different error text. Not a clear reduction.
- fab.rs `OUTLINE_KEY`/`LADDER_KEY` vs rebind.rs `ADMITTED_NOTES`. `ADMITTED_NOTES` is already derived from the two keys (a single source). `built_lines` writes with the keys and `states_built`/`admitted` read with them. Nothing duplicated.
- verify.rs `stress` `MotorOutputOpen` arm and `classify` `matches!` list entry. Each is the minimal per-override registration, parallel to `SerialRxOpen`, not a duplicate. `Target::Motor` and `MOTOR_OPEN` are each used.
- verify.rs `gate_report` `built: if name == "extract" {...}`. Only the extract gate carries it, and `conclude` reads it back. Single use, no dup.
- supervisor.rs `Floor::text` vs `Named::text`. Two formats for two different facts. A merge would need a `kind` enum and gains nothing.
- supervisor.rs `check_against` `unreachable!` and the `rising_u`/`vdd` conversions. Pre-existing and tested; not part of this WO's accretion.
- `Shared` vs `Other` in supply.rs. Both are "charge a current at min(bound, load's bound)". They differ in structure (a module's two sides on VDD and GND versus one entry on GND), and a merge is a redesign, beyond a reduction.
- Dead branches: none found for the WO's new code. `Judgement::Elsewhere` is produced (battery-fed regulator) and consumed (readiness filter treats it as pass). `Powered.elsewhere` is read by `judged` only.

## Totals

Items 1, 2, 4, 5, 6, 7, 8 (if goldens allow), 9, 12 sum to about -115 lines of production code, with no behaviour change except item 8's wording. Items 1 to 3 are one contiguous hunk in supply.rs and are the best ratio: about -40 lines, no risk to numerics beyond one shared fn.
