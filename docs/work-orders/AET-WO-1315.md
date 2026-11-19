# AET-WO-1315 — The emitter draws what the power tree declares: the hot-plug bulk capacitor on the board, and each control pin as its declaration says

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`cd43b207` when written; verify live HEAD).
**Branch:** `wo/aet-1315-hot-plug-emit`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4 board capability, before Sentinel-03 is ordered and before `4500`. Written 2026-11-19. **Cadence: build, with one hard stop: any need for a specification or parts-table schema change (§7).**

**Grounds:**
- **The row.** Roadmap §7.1 L719: "The emitter draws the hot-plug damping parts and the straps · `4300d3` D5 · the next eda work order, before `4500`". Bundled: §7.1 L812, the `Stack::of` nit. **Not** L808 (`fab.rs`'s via): that is `4500`'s.
- **The ruling.** `AET-WO-4300d3.FEEDBACK.md` §7 (PR #158, comment 6902904831): "D5: backlog, required before `4500`. The emitter draws `power.input.hot_plug` and reads `controls`; Sentinel-02's board must carry its bulk capacitor." `AET-WO-3400.FEEDBACK.md` names the same item E0 (L274) and adds: "and refuses a declared reference it would reuse". Roadmap L374 puts E0 on Sentinel-03.
- **What the check credits and the board lacks** [read]:
  - `aethelgard-ir/src/power.rs:2444` (`hot_plug`) passes `BatteryInputUndamped` on one declared `UCZ1H470MCL1GS`, 47 µF 50 V, `side: board`.
  - Nothing in `aethelgard-eda` reads `hot_plug`. `schematic_power.rs:276` (`draw`) draws regulators, crossings, sense and the connector only. `capacitor()` (`:161`) accepts `Kind::Capacitor` only; the row is `Kind::BulkCapacitor`.
  - The row names `Device:C_Polarized` and `Capacitor_SMD:CP_Elec_8x10` (`ir/data/power-parts.json`). **Neither is vendored** (`aethelgard-kicad-lib/data/kicad-10.0.7/`: `Device.kicad_symdir` holds C, L, R; `Capacitor_SMD.pretty` holds 0402, 0603, 1210). `PINS` and `FILES` hold 50 files.
- **The reference collision** (`3400` F1) [read]: `aethelgard_ir::power::references` (`power.rs:87`) omits `input.hot_plug`. The emitter's claim list (`schematic.rs:1208`) and `validate`'s name checks (`ir/src/names.rs:106`) both read it. So the emitter numbers its own decouplers from `C1`, the declared bulk capacitor's reference. `validate`'s own shape check (`power.rs:739-744`) does include `hot_plug`; the shared list does not.
- **The straps** [read]: the emitter ties by its own rule, not the declaration: buck EN to input and PG open (`schematic_power.rs:359-360`), LDO EN to input (`:438`), translator OE to VCCA (`:488`).
  - The rows allow (`power-parts.json` `controls`): buck EN `input`, PG `open`, both wirable; LDO EN `input`, wirable; TXU0304 OE `vcca` or `vccb`, wirable.
  - **Both committed power trees declare exactly what the emitter draws today** (sentinel-03.json, sentinel-02-power.json: EN `input`, PG `open`, OE `vcca`). So reading `controls` moves no committed byte. The new paths are OE `vccb` and every `wire`, which no committed board uses.
- **The verdict says it is not drawn.** `power.rs:2985-2992` adds the condition "the board emitter does not draw them … (FEEDBACK D5)"; `:2890` adds "and the board does not draw it (FEEDBACK D5)" for a wire. `ir/tests/power.rs:1174` asserts it. Once drawn, those sentences are false.
- **The nit** (L812) [read]: `stack.rs:388`, `if let Ok(Some(rule)) = RingRule::of(spec)`. An `Err` (a partial `fab_tolerances` block) is dropped and the engine's via kept. `emit` refuses first (`schematic.rs:872`, `FabToleranceUndeclared`). Other production callers do not: `cli/src/main.rs:849` and `:1024` (the `check drc` and board-read paths), `cli/src/extract.rs:191`, `cli/src/fab.rs:141, :336`, `eda/src/rules.rs:86`.
- **Only two committed specifications carry a power tree** [measured, every JSON under `examples/` and `crates/**/fixtures/`]: `cli/tests/fixtures/sentinel-03.json` and `ir/tests/fixtures/sentinel-02-power.json`. Sentinel-01, its bench board and every other example declare `power: null`.
- **Pinned figures this will move** [read]: `board_tier2.rs:1186` (17 loop rules on the power fixture); `1313c`'s name-free placement pin for Sentinel-03 (85.25 × 80.95 mm, least `extract` slack 9.2 pF, inside a 100 × 100 mm bound); `supply_tier1`, `via_ring_tier1`, `outline_tier1` emit Sentinel-03.

> **What this is:**
> - every declared `power.input.hot_plug` bulk capacitor drawn on the battery rail, placed by a stated rule, judged by the loop gate;
> - each regulator's and translator's control pin drawn as its `controls` entry says, never by the emitter's own rule;
> - the declared hot-plug reference claimed, so the emitter never gives it to another part;
> - the verdict's "not drawn" condition removed where it is no longer true;
> - `Stack::of` stops swallowing a ring-rule error.
>
> It is NOT:
> - a specification or parts-table schema change. `4400g` is taking specification 17 now. **Any need for one is a §7 stop;**
> - a TVS drawn: no held row has one (`4300d3` §0). Sentinel-02's TVS is `4500`'s (§7.1 L720);
> - the buck's input capacitors re-picked (§7.1 L718, `4500`);
> - `fab.rs`'s via at declared tolerances (§7.1 L808, `4500`);
> - an MCU pin for a control wire. A wire is drawn only to a net the specification already ends elsewhere (§1.4);
> - the electrolytic's height or a mechanical keep-out;
> - a fabrication package for Sentinel-03.

## 1. Build

0. **Predict, then build.** Commit a short prediction table to FEEDBACK before the first run: the loop count, Sentinel-03's outline and least slack, and which pinned figures move. Label each answer measured, read or inferred, as `4300d3` did.
1. **Claim the declared reference** (the one change in `aethelgard-ir`, §3).
   - `power::references` lists each `input.hot_plug` reference, after `input.reference`.
   - The emitter's decouplers then skip the declared `C1`. On both power boards every emitter-numbered capacitor moves by one. Say so in FEEDBACK with the before and after lists.
   - `validate`'s name checks then cover a hot-plug reference: a case-only twin, a `#` prefix, `unconnected-` (the `1206` F2/F3 pattern).
2. **Vendor the two library files.**
   - `symbols/Device.kicad_symdir/C_Polarized.kicad_sym` and `footprints/Capacitor_SMD.pretty/CP_Elec_8x10.kicad_mod`, byte for byte from KiCad's libraries at tag **10.0.7**.
   - Each gets its sha256 in `eda/src/library.rs` `PINS`, an entry in `kicad-lib` `FILES`, and a row in `kicad-lib/data/kicad-10.0.7/README.md` with its git blob SHA as GitLab's files API reports it and its raw URL. 50 files become 52.
   - Read the symbol's pin numbers and the footprint's `+` pad at source. Cite them in the module documentation.
3. **Draw the hot-plug parts.**
   - **A bulk capacitor** (`Kind::BulkCapacitor`): positive pin on `BATTERY_NET`, the other on `GND`, valued as `capacitor()` values a ceramic (`47u UCZ1H470MCL1GS`).
   - **Any other row kind** (a TVS, or anything a future table adds) is refused `PowerNotDrawable` by name. Never skipped, never drawn as something else (`INV-06`). Test it with a test-only table, as `4300d3` tested the clamp.
   - **Placement.** Anchored to the first battery-fed regulator in declaration order, at its VIN and ground pads, as the last part of that regulator's draw sequence (behind the ceramics: SNVSAN3F p. 23, 9.2.2.6, "in parallel with the ceramics").
     - The bound is an engine choice, set after measuring the placer, and its rule text says so (the `CIN_BULK_NM` precedent, `4300d2` D3). Record the measurement.
     - With no battery-fed regulator, draw it by its net alone, unanchored, and test that case.
   - The draw order, and so placement, must stay independent of the parts order (`under_a_bound_the_board_does_not_depend_on_the_parts_order`).
4. **Draw each control pin from its declaration.**
   - **Straps:** `input` to the regulator's input rail; `open` as no-connect; `vcca` and `vccb` to the translator's A and B rails.
   - **Wires:** the pin on the declared net, as a label. Draw it only when another part the specification declares ends on that net (a connector pin or a test point). Otherwise refuse `PowerNotDrawable`, naming the pin and the net. A one-ended net is a floating pin with a name.
   - **A row's control pin with no declaration** reaching the emitter (an unvalidated caller) is refused by name. It never falls back to the old tie.
   - On both committed power boards this step moves no byte. Prove that separately from step 3.
5. **The verdict stops saying "not drawn."**
   - In `power.rs`, remove the condition at `:2985-2992`, and from the wire condition at `:2890` the clause "and the board does not draw it (FEEDBACK D5)". "Neither the net's driver nor its level is judged" stays: still true.
   - `ir/tests/power.rs:1174` asserts the condition is gone.
6. **The nit: `Stack::of` never drops an error.**
   - First, a failing test on `main`: a specification with a partial `fab_tolerances` block, given to `check drc` or `extract` with a board, today reads it with the engine's via.
   - Then surface the error. The shape is yours (a `Result`, or the rule passed in); take the smaller diff. Each production caller refuses by name with `emit`'s kind, `FabToleranceUndeclared`. Tests may unwrap.
   - The CLI changes are call-site only (§3).
7. **Every moved figure is re-pinned from a measured run, cited to it:** the loop count; the placement pins; `extract`'s slack figures; any `ANSWER` line FEEDBACK reports. Name each in FEEDBACK with before and after.

## 2. Acceptance

- **Every committed board without a power tree is byte-identical to `main`.**
  - Run `main`'s binary and this branch's over every specification under `examples/` and `crates/**/tests/fixtures/` (`emit`, `solve`, `generate`). Compare unmasked. There is no version bump here, so nothing may move, not even a hash line.
  - **Sentinel-01 above all:** its package is about to be ordered. `aethelgard verify … --against-fab fab/sentinel-01-bench/package`, with the scenarios `ci.yml`'s `bench-package` job names, reports `identical`. Run it once yourself in the image, and cite CI's run where one exists. `fab/sentinel-01-bench/package/MANIFEST.json` does not move.
- **The two power boards gain the drawn parts and pass as before or better:**
  - Sentinel-03: `validate`, `generate`, `emit`, ERC, DRC with and without parity, `check support` and `extract`, each with no finding, inside its 100 × 100 mm bound (`the_sentinel_03_fixture_passes_every_gate_inside_its_bound`).
  - The power fixture: ERC, DRC with every loop rule passing (the new one included), `check support`, and `extract` as today (the reduced tree passes; the full tree is still refused `TranslatorPinUnbounded`, by that name only).
  - On both, `UCZ1H470MCL1GS` is on the schematic, the board and the BOM as `C1`, positive on `VBAT`. No emitter part is named `C1`.
- **A parity test, tier 1:** every reference `power::references` lists is on the emitted board with its declared part number, on both power boards. It reads the specification, not the emitter's part list (`INV-07`).
- **Each new refusal has a tier-1 test and a mutant a test kills:** a non-bulk hot-plug row; a one-ended wire; an undeclared control pin at the emitter; the partial `fab_tolerances` block at `check drc` and at `extract`.
- **OE `vccb` and a wire to a connector's net** are drawn as declared, shown by tier-1 tests on derived fixtures.
- **The CLAUDE.md gates pass. CI is green. `TIER2_MIN` is exact.**

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-03`, `RUL-11`, `RUL-14`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-eda`: `schematic_power.rs`, `schematic.rs`, `stack.rs`, `library.rs`, and `pcb.rs` only if the anchor needs it;
  - `aethelgard-kicad-lib`: the two vendored files, `FILES`, and `data/kicad-10.0.7/README.md`'s table;
  - **`aethelgard-ir`, a named carve-out and nothing else:** `power.rs`'s `references` (one list) and the two condition strings of §1.5, with `ir/tests/power.rs`'s assertion. No type, field, refusal or schema constant. `4400g` owns `ir`: say on the bus before you push it, and if `main` moves under you in `power.rs`, merge and re-gate;
  - the CLI: `Stack::of`'s call sites only (`main.rs`, `extract.rs`, `fab.rs`);
  - tests, goldens and FEEDBACK.
  - **Not:** `.github/workflows/`, `ci/` (`test-weights.tsv`, `SHARDS`, the `TIER2_MIN` line), the root `Cargo.toml`, `eda/src/fab.rs`'s via (L808), `aethelgard-twin`, `aethelgard-chaos`, the CLI's `simulate`, `foreign` and `campaign`, `boards/**` and every `.repl`, `container/`, `examples/sentinel-01*.json`, `fab/sentinel-01-bench/**`, the steering documents and the root `README.md`.
- **Running beside you:**
  - `AET-WO-1910` and `1911` own the workflows, `ci/` and the root profiles. **If you add a tier-2 test, say so on the bus.** The weights row and `TIER2_MIN` change are coordinated with them, never made here. Merge `main` before READY.
  - `AET-WO-4400g` owns `aethelgard-ir`, the twin, chaos, the CLI's simulate path and the board platforms, and is moving the specification to 17. You take no schema version.
  - `AET-WO-2304` (battery sag) and `AET-WO-4240` (the HSE clock) are spikes. They read your area and change nothing in it.
- One PR, left unmerged. Write `~/.cache/aet-1315/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A reference collision:** a hot-plug reference that an emitter number, a test point, a case-only twin, `#C1` or `unconnected-…` would take or shadow.
- **A reversed electrolytic:** the negative pin on `VBAT` anywhere (schematic, board or BOM).
- **A declaration ignored:** OE `vccb` drawn as `vcca`; a wire drawn as a strap; a strap drawn when the declaration names a wire.
- **The old default coming back:** a control pin missing from the declaration, given to `emit` without `validate`.
- **A floating pin with a name:** a wire to a net no other part ends.
- **A non-bulk row reaching the emitter,** through a test-only table.
- **`Stack::of` still quiet:** any production path that reads a board with a partial tolerance block and does not refuse.
- **Sentinel-01 moving:** the new library files, the reference numbering, or the error surfacing changing any byte of its outputs or its package.

## 7. Flag-and-stop conditions

- **Any need for a specification or power-parts schema change:** a field, an enum variant, `SPECIFICATION_SCHEMA_VERSION` (16, `4400g` takes 17) or `POWER_PARTS_SCHEMA_VERSION` (4). Stop with what is needed and why. A second bump would collide.
- **Any change to `aethelgard-ir` beyond the carve-out in §3.**
- **Any byte of a board without a power tree moves,** or `MANIFEST.json` does, or `--against-fab` reports anything but `identical`.
- **A gate that passes on `main` fails on a power board:** Sentinel-03's bound (`OutlineUnreachable`, `OutlineTooSmall`), `extract`'s capacitance, DRC or support. Stop with the numbers. Never widen a bound, move a fixture's declared bound, or edit a fixture to pass.
- **A vendored file cannot be fetched at tag 10.0.7, or its blob SHA disagrees.** Never hand-write or edit a KiCad library file.
