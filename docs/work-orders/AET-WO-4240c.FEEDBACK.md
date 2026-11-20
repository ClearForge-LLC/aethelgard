# AET-WO-4240c — FEEDBACK (build)

Branch `wo/aet-4240c-hse-eda`, from `main` `8df353ee` (the kickoff's base). The work order is `docs/work-orders/AET-WO-4240c.md` on the docs branch (copy at `~/.cache/aet-4240c-wo.md`); it is not on `main`. The architect's rulings (Q1–Q5, A1–A7, 2026-11-20 16:34Z) are in `~/.cache/aet-bus/4240c.md`.

Every answer is labelled **measured** (run here, the log or file named), **read at source** (document, revision, page) or **inferred**.

## §0 The answer in one screen

- **No `hse: null` KiCad byte moved** [measured]. 29 committed specifications, 24 emit and 5 refuse, on base `8df353ee` and on every branch commit that could move an emitted byte, and again against `c1012a2d` after merging `main`. All 72 files (`.kicad_sch`, `.kicad_pcb`, `.bom.csv`), the `emit` stdout and the exit codes are byte-identical, with no mask. `extract`'s stdout and exit codes on all 24 emitted null boards are identical too. An independent subagent rebuilt both ends and confirmed it.
- **Vendored, byte for byte from KiCad 10.0.7** [measured]: `Device:Crystal_GND24` (git blob `97e45475…`) and `Crystal:Crystal_SMD_3225-4Pin_3.2x2.5mm` (`11874e16…`). Each equals GitLab's files-API `blob_id` at ref `10.0.7`. `PINS` and `FILES` are 54, with the README rows. The crystal row named `Crystal:Crystal_GND24`, which does not exist upstream; it is now `Device:Crystal_GND24` (Q3).
- **The crystal is drawn** (`schematic_hse.rs`):
  - crystal `Y1` on PH0/PH1 (LQFP-144 pins 23 and 24, by package position);
  - pads 1/3 on `U1_OSC_IN`/`U1_OSC_OUT`, pads 2/4 on `GND` (ABM8AIG p. 3);
  - one C0G 0402 capacitor per net to `GND`, picked by the declared value from the parts table (A2), else `LoadCapacitorNoPart`.
- **The cited C0G row** (A3, ruled 17:45Z): **KEMET C0402C180J5GACTU**, 18 pF, 0402, C0G, 50 V, J = ±5 %. Every field is cited to KEMET C1003_C0G rev 2/20/2025 (sha256 `a719330f…`, held by the operator): p. 1 ordering information; p. 5 Table 1A, 18 pF row; p. 4 operating temperature.
- **Routed with no via on either oscillator net** [measured]. The router lays them right after the stitched nets and never offers them a via.
- **The fixture passes** ERC, DRC with schematic parity, `check support` (its crystal step judging KiCad's own netlist export) and `extract` in the tools image, every findings list empty [measured at `fdd29ea2`, `~/.cache/aet-4240c/image-final.txt`].
- **External clock:** refused `HseSourceNotDrawn` (Q4 (a)).
- **`Cs`: report only, as ruled (§1.4 (b)).** `extract` reports each oscillator net's copper bound with `OSC_PIN_CONDITION` and DS12117's `CIO` 5 pF, labelled typical:
  - `U1_OSC_IN` 7,473 fF, 10.75 mm, crystal gap 5,310 µm;
  - `U1_OSC_OUT` 6,016 fF, 6.875 mm, gap 3,614 µm.
- **The declared 18 pF is a finding** (§7 F1). It fits CL 12 pF only near 6 pF of stray per net, which nothing bounds; the load at 18 pF lies anywhere in 9.0–17.9 pF. The declaration is not changed.
- **No `CrystalTooFar`** (Q5): the gap and the track length are reported, and pinned by a test.
- **§9's ±0.25 pF / ±1 % family is not built** (ruled 17:45Z): it was a recommendation that no held pullability source supports.
  - The emitter and the gate require a load capacitor row to **state** its tolerance, of any width.
  - Under the report-only load ruling, ±5 % changes no verdict.

## §1 Predictions, scored (`spike/4240c/PREDICTIONS.md`, committed `408c71f3` before any run)

| # | Prediction | Result |
|---|---|---|
| P1 | No null emitted byte moves, after every commit | **Held** (§2.1) |
| P2 | Vendoring moves no byte; `PINS`/`FILES` 52 → 54 | **Held** |
| P3 | Both blob shas equal upstream at 10.0.7 | **Held** |
| P4 | `Crystal:Crystal_GND24` does not exist upstream | **Held** (404; the symbol is in `Device`) |
| P5 | The crystal fixture emits, routes with no via, passes ERC and DRC; the crystal within a few mm | **Held** [measured]: gaps 5.3 and 3.6 mm |
| P6 | `Cs` cannot be bracketed from held sources; the §1.4 stop fires | **Held**: stopped 16:4xZ, ruled (b) |
| P7 | Each oscillator net's copper bound is under 1 pF | **Wrong**: 7.47 and 6.02 pF. The bound is Dirichlet's shell at the 0.2 mm clearance, an upper bound loose by construction |
| P8 | At 18 pF the load is about 15 pF, a mismatch | **Partly**: the load lies in 9.0–17.9 pF; the mismatch holds only at the top of that range (§7 F1) |

## §2 Measured

### §2.1 The null corpus (`spike/4240c/kit/emit-sweep.sh`, `same.sh`, `extract-sweep.sh`; record `spike/4240c/RESULTS.md`)

- Base: `git archive 8df353ee`, built apart. Corpus: every tracked specification outside `spike/`, `fab/`, `docs/` and `target/`, less the stored topologies and `output-speeds.json` (`kit/specs.txt`, 29 files).
- After vendoring (`13c67282`), after the emitter and router (`f7bbb1e0`), and after the gate, extract and CLI changes (`8a040d43`): **IDENTICAL, 72 files, 29 specifications.** The adversarial subagent repeated it at `cbeae11f`.
- `extract` stdout and exit codes on the 24 emitted null boards: identical between base and `8a040d43` (18 exit 0, 5 exit 1, 1 exit 2, all as on `main`).
- Outside the corpus, as predicted: the gate-written `fp-lib-table` now names `Crystal` (15 libraries). ERC and DRC on `sentinel-01-minimal`, `sentinel-01` (`sentinel-01-bench`) and `sentinel-02-bench` pass on the branch [measured, tools image].

### §2.2 The crystal fixture

- **The fixture:** `examples/sentinel-01-minimal.json` with `mcu.hse` set to `4240a`'s declaration (ABM8AIG-16, caps 18 + 18 pF, pull 30 ppm, −20…85 °C, 8,000 h), built in the test. No committed specification is added: the CLI fixtures directory is globbed by tests that `generate`, which refuses HSE until `4240b`.
- **In the tools image** (`spike/4240c/kit/img.sh`; `capped-run`, 8 GiB, swap 0 read back): ERC pass, DRC with schematic parity pass, `check support` pass, `extract` pass, every findings list empty.
  - So KiCad accepts the stacked GND pins 2 and 4 of `Crystal_GND24`.
  - It also accepts the private symbol fields left off instances and footprints.
- **Copper** [measured; `hse_eda_tier1`]:

| Net | Bound (from above) | Trace | Pads | Vias | Length | Crystal gap to the MCU pad |
|---|---|---|---|---|---|---|
| `U1_OSC_IN` | 7,473 fF | 5,118 | 2,355 | 0 | 10.75 mm | 5,310 µm |
| `U1_OSC_OUT` | 6,016 fF | 3,661 | 2,355 | 0 | 6.875 mm | 3,614 µm |

- **The load bracket** (`RESULTS.md`): each net's bound includes its mutual capacitance to the other oscillator net, which sits across the crystal. With `CIO` typical counted, the load at 18 pF lies in 9.0–17.9 pF, and the value giving CL 12 pF lies anywhere from about 6 to 24 pF.

## §3 What was built

- **Vendored** (`aethelgard-kicad-lib`): the two files, their `FILES` and `PINS` rows (52 → 54), and the README rows with sha256 and git blob. The shape test's file list and the gate-table count (15 footprint libraries) follow.
- **The parts table** (`power-parts.json`, schema 5 unchanged):
  - the crystal row's `symbol` is `Device:Crystal_GND24` (Q3);
  - the MCU row's `support.pins` gains `OSC_IN` ["23"] and `OSC_OUT` ["24"] under its existing `pins_at` (A6, own commit `90820c3c`);
  - the document `KEMET_C1003_C0G` and the row `C0402C180J5GACTU` (A3, `fdd29ea2`), its tolerance `{"permille": 50}`.
- **The parts reader:** the capacitor kind's optional `tolerance`, `{"permille": …}` or `{"femtofarads": …}`, cited and above zero (A2). It is absent on every row read before it.
- **`aethelgard_ir::names`** (Q1): `oscillator_nets(spec)`, `<mcu>_OSC_IN` and `<mcu>_OSC_OUT` beside a crystal and none otherwise; `own_nets` lists them.
- **The emitter** (`schematic_hse.rs`, wired in `schematic::design`):
  - the crystal on PH0/PH1 by ST's pin table, anchored by both pins;
  - its pads 1/3 on the oscillator nets and 2/4 on `GND`;
  - one load capacitor per net, anchored by its MCU pin. It is the first row by part number that is C0G, 0402, rated exactly the declared value, with a stated tolerance; else `LoadCapacitorNoPart`.
  - An external clock is `HseSourceNotDrawn` (Q4 (a)).
  - The support group leaves the row's `OSC_IN`/`OSC_OUT` untied otherwise, so a null board's PH0/PH1 are unconnected as before.
  - A symbol's private library fields (`Crystal_GND24`'s two `KLC_` notes) reach no instance or footprint (`library::private`). It is the only vendored symbol with any.
- **The router** (`route.rs`, `stack.rs`): `Stack.oscillator` (board names; empty without a crystal). Those nets are ordered right after the stitched nets and never offered a via (`via_free`), however often they are ripped up. On a null board the new sort-key term is constant.
- **The support gate's step 7** (`support.rs` `crystal`, independent of the emitter, `RUL-20`):
  - `OSC_IN`/`OSC_OUT` each on a net of its own;
  - exactly one populated crystal of the declared part and symbol, its ends 1/3 on those nets and 2/4 on `GND`;
  - on each net exactly one capacitor to `GND`, of the declared value, naming a C0G row rated that value with a stated tolerance;
  - no other pin on either net, the MCU's own included;
  - a crystal on a board that declares none.
  - Each failure is a named finding (`hse_pin_off_net`, `hse_crystal_missing`, `hse_crystal_off_net`, `hse_crystal_ground_floating`, `hse_load_capacitor_missing`/`_value`/`_dielectric`/`_tolerance`, `hse_net_extra_part`, `hse_crystal_undeclared`).
- **`extract`** (`extract.rs` `oscillator_lines`/`oscillator`, `Load::Oscillator`):
  - each oscillator net bounded as a bus line is, copper only;
  - its length, vias and pads, and the crystal-pad-to-MCU-pad gap, µm rounded up;
  - `OSC_PIN_CONDITION` and `OSC_PIN_CIO_TYPICAL_FEMTOFARADS`;
  - `Oscillator::refusal()` names a via `OscillatorNetVia`.
- **The CLI's `extract`** (A7, `cli/src/extract.rs` only):
  - the report gains `oscillator` (absent without a crystal) and the pin condition;
  - a via is a finding, so the verdict is fail with exit 1, and `verify`'s extract gate fails with it.
- **Tests:** `cli/tests/hse_eda_tier1.rs` (10), `eda` `schematic::hse::tests` (1), `ir/tests/hse.rs` (+1). The gate tests' netlist helpers are now one copy, `cli/tests/common/netlist.rs`. All tier 1; `TIER2_MIN` does not move.

## §4 Decisions and deviations

- **D-1, fixtures built in the test, not committed.**
  - The CLI fixtures directory and `examples/` are walked by tests that `generate`, `solve` and judge every committed specification, and `generate` refuses an HSE board until `4240b` (Q2).
  - The crystal fixture is `sentinel-01-minimal` with `mcu.hse` inserted.
  - `4240b`'s `hse-crystal-16`/`hse-external-8` fixtures (sentinel-02-shaped) will be drawn by this emitter once both are on `main` (peer note to 4240b, 16:2xZ).
- **D-2, the null guard as a test is structural.**
  - `no_committed_specification_without_a_crystal_draws_one` holds, for every committed specification: no `_OSC_` net in `own_nets`, an empty `Stack.oscillator`, and no oscillator line in the design.
  - The bytes themselves are the sweep's (§2.1).
  - Pinning 72 hashes would make every later emitter change edit this file.
- **D-3, the via mutant is applied to the board as read.**
  - A via hand-placed in the board text near the crystal broke clearance or connectivity at every spot tried (C1's GND pad, a GND via, Y1's courtyard).
  - The library test adds a `Via` to the `ReadBoard`; the judge counts the net's vias wherever they stand.
  - The router-side mutant (M1, `via_free` off) produces two vias per net, as the first probe did, and is killed.
- **D-4, the crystal and caps are placed by the existing anchor placer.** No new placement rule was added. The measured gaps are reported (Q5).
- **D-5, `library::private`.** KiCad's ERC and DRC parity accept instances and footprints without the symbol's private fields [measured, §2.2]. The sheet's embedded library symbol keeps them, byte for byte.

## §5 Adversarial pass (full)

Two fresh subagents, each on its own `git archive` copy of `cbeae11f`, given the code, the work order's §5 and the rulings, not my conclusions. Their scratch, probes and logs are in `~/.cache/aet-4240c/adv-a` (Opus: a crystal board that passes every gate yet cannot start) and `adv-b` (vendoring, the null bytes, the oscillator nets' neighbours). Each finding was reproduced or re-read before it was acted on.

| # | Finding | Severity | Done |
|---|---|---|---|
| A1 | Another MCU pin (a GPIO) joined to `U1_OSC_IN` or `U1_OSC_OUT` passed step 7: the "nothing else on the net" filter left out every MCU pin | should-fix | **Fixed** `70b47701`: only the MCU's own oscillator pin is left out. Tested; mutant M17 |
| A2 | The load capacitor's named row was checked for dielectric only, so `18p <a C0G row of another value>` passed | should-fix (latent: one C0G row) | **Fixed** `70b47701`: the row must be rated the declared value and state a tolerance (`hse_load_capacitor_value`, `_tolerance`; the width bound was dropped by the 17:45Z ruling) |
| A3 | **A noisy-net crossing passes every rule** (§5 item 2): on the fixture as routed, a `VDD_3V3` track on B.Cu, (86.375, 100.5)–(86.375, 104.75) mm, crosses under both oscillator nets, splitting the B.Cu GND pour under the crystal's fan-in; `NRST` (U1 pin 25) is 0.20 mm from `OSC_OUT`'s pad. No SPI, I2C, motor or serial net within 1 mm | finding | **Reported, not judged** (§7 F2): no held source states a guard (AN2867 uncited, as Q5 ruled for distance). Verified in the board text (line 12891) |
| A4 | `extract` alone accepts a load capacitor whose GND pad is unlabelled on the board | information | KiCad's DRC parity against the schematic is what catches a pad off its schematic net; `extract` judges copper, not the netlist |
| A5 | On a null board, a crystal drawn with a symbol that is no crystal row's (`Device:Crystal`) or a second crystal of another value elsewhere is not named; one message printed the right nets for a wrong pin | information | Message fixed. The rest is outside what the emitter can draw (no row names those symbols) |

**Confirmed by the subagents** [measured]:
- **Vendoring:** both files `cmp`-identical to the 10.0.7 raw files; blob shas equal the files-API `blob_id`; sha256 equal to `PINS` and the README.
- **The null corpus:** rebuilt both ends from `git archive`: 72 files, stdout and exit codes identical; `extract` on the 24 boards identical.
- **No hidden null path:**
  - PH0/PH1 assigned to a signal on a null board;
  - a null specification naming a net `U1_OSC_IN`;
  - the router's order key;
  - the private-field filter (no other vendored symbol or footprint has a private field).
  Each was emitted or read and found byte-identical.
- **Held under attack** (adv-a):
  - both crystal ends on one net;
  - a capacitor across the two nets, a resistor or test point on one;
  - a DNP or `exclude_from_board` capacitor;
  - a ground on `GND1`/`GNDA`;
  - a bare `18p`;
  - a second crystal of the declared part;
  - PH0 left unconnected;
  - an `OSC_IN` segment moved to B.Cu or relabelled (refused `NetLabelsDisagree` by `extract`'s reader).
- **No panic** is reachable from input in the added lines.

## §6 Mutants

**18 mutants, one per guard this work order adds (`spike/4240c/kit/mutants.py`), rerun on the reduced tree: 16 killed, 2 equivalent** [measured, `~/.cache/aet-4240c/mutants-3.log`].

- **The work order's four:**
  - a via on `RCC_OSC_IN` (M1, the router offered a via; M3, the refusal disabled);
  - the crystal moved away (`the_crystal_moved_away_moves_the_reported_gap`; M6, below);
  - `Cs` dropped from the report (M4, the pin condition; M5, the oscillator objects; ruled A1);
  - a capacitor of the wrong dielectric (M8 at the gate, M12 at the emitter).
  Each is killed but M6.
- **Equivalent:**
  - **M6** (the gap taken as the largest pad pair instead of the smallest): each oscillator net holds exactly one crystal pad and one MCU pad, which `bound` holds to the design, so the two agree.
  - **M18** (an unstated tolerance passes the gate): every C0G row in the committed table states its tolerance, so no reachable netlist can tell. The emitter's own requirement is killed (M12, by decoy rows: X7R, untoleranced, 0603).

## §7 Findings

- **F1 · the declared 18 pF against CL 12 pF · finding (ruled §1.4: reported, the declaration unchanged).**
  - `4240a`'s fixture declares `load_caps_femtofarads: [18000, 18000]`. In series that is 9 pF.
  - The crystal's load is that plus the strays: each net's ground share adds to its capacitor, and the mutual share across the crystal adds whole.
  - With `extract`'s upper bounds (7,473 and 6,016 fF, each holding the mutual) and DS12117's `CIO` 5 pF counted at its typical value, the load at 18 pF lies anywhere in **9.0–17.9 pF**. The value that would give CL 12 pF lies anywhere from **about 6 pF to 24 pF**.
  - So 18 pF fits only near 6 pF of stray per net, which nothing held bounds.
  - Its frequency effect stays inside the declared `pull_ppm` (`4240a`), as ruled.
- **F2 · a noisy or plane-splitting net under or beside an oscillator net · §7.1 candidate.** Adversarial A3: a B.Cu `VDD_3V3` track crosses under both oscillator nets and splits the B.Cu GND pour beneath them; `NRST` runs at the clearance beside `OSC_OUT`. No rule judges it, because no held source states one (AN2867 is uncited; ST's layout guidance is not held). A keep-out under the oscillator nets would be a router rule with a cited bound behind it.
- **F3 · `CrystalTooFar` · §7.1 row (ruled Q5).** The gap and the track length are reported per net and pinned: 5,310 and 3,614 µm; 10.75 and 6.875 mm. The refusal waits for a held source.
- **F4 · the external clock · §7.1 row (ruled Q4).** `HseSourceNotDrawn` until a connector pin or test point can carry the clock: an IR net name and a schema field.
- **F5 · `OSC_IN` routes round the crystal · information.** The 3225's ends are diagonal (pads 1 and 3), so one oscillator net must pass a ground pad: `OSC_IN` is 10.75 mm against `OSC_OUT`'s 6.875 mm on the fixture. The anchor placer chose the rotation; nothing here judges it (F3).
- **F6 · `fab_tier1` `a_rail_or_bus_no_fault_kind_reaches_is_named` · environment.** It fails only with the target directory outside the repository, as `4240a` F6 found. Not touched.
- **F7 · the CLI still refuses the crystal fixture at `emit` (`HseNotGenerated`) · for `4240b`.** `extract` and `check support` accept it now. `4240b` lifts `generable`; after both merge, its `hse-crystal-16` fixture is drawn by this emitter, and any of its goldens holding KiCad bytes moves with whichever lands second (peer note sent).
- **F8 · the load capacitor's tolerance · ruled 17:45Z, a §7.1 row.**
  - `4240` §9 named the C0G family "at ±0.25 pF or ±1 %". No held source supports that: the row's pullability is not stated, so no tolerance can be judged against a frequency effect.
  - The row is ±5 % (J), the part the operator saw stocked.
  - A tighter part (F, ±1 %) is a row for when a pullability source is held.

## §8 Reduction pass

**Reduction pass (diff): −122 net · 4 duplicates removed · 1 dead path removed · 2 protected kept · 0 findings.**

A fresh subagent proposed the cuts, reading only; the builder ruled on each and made them (`932dcce7`).
- **Removed:**
  - the gate tests' netlist helpers, one copy instead of three (`common/netlist.rs`; `support_tier1` and `bench_tier1` had their own);
  - the "crystal declared, so name the oscillator nets" guard, once in `names::oscillator_nets(spec)` instead of in `own_nets`, `Stack::of` and `oscillator_lines`;
  - `CRYSTAL_PADS`/`Role` (one use), inlined;
  - the load capacitor's unreachable `Kind` re-match and its two `Internal` arms;
  - two restating doc blocks;
  - `Debug` on `ReadBoard`.
- **Proof:** clippy clean; every affected suite green; the 18 mutants rerun (§6); the null sweeps identical after it.
- **Kept, protected by `RUL-20`:**
  - the gate's crystal pad roles beside the emitter's map;
  - the gate's C0G-and-tolerance check beside the emitter's pick.
- **Rejected:** making `oscillator_lines` private (a test calls it).
- **Findings:** none outside the diff.

## §9 Gates and CI

- **Local, at `fdd29ea2`, `main` `c1012a2d` merged** [measured, `~/.cache/aet-4240c/gates-test-4.log`]:
  - `cargo fmt --all -- --check`: 0;
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0;
  - `cargo test --workspace --no-fail-fast`: **2,469 passed, 1 failed**. The failure is `fab_tier1`'s `a_rail_or_bus_no_fault_kind_reaches_is_named`, which fails only with the target directory outside the repository (§7 F6). Run with `CARGO_TARGET_DIR` inside the repository, as CI runs it, it passes.
- **Mutants:** 18; 16 killed, 2 equivalent (§6) [`~/.cache/aet-4240c/mutants-4.log`].
- **Tier 2:** no tier-2 test was added, so `TIER2_MIN` does not move. The crystal fixture's KiCad checks were run in the tools image by hand (§2.2) because the CLI's `emit` refuses it until `4240b`.
- **CI:** on the PR; its run ids are in §10 at READY.
- Reduction pass (diff): −122 net · 4 duplicates removed · 1 dead path removed · 2 protected kept · 0 findings (§8).

## §10 State at READY

- Head: the READY commit. `main` `c1012a2d` (1210, 1912, the work order) is merged in. The null corpus was re-baselined against it: emit and `extract` identical.
- `4240b` (generator, twin, the CLI's `HseNotGenerated` lift) and `1211a` are not on `main` yet. Merge order: whichever lands second takes the other. `4240b`'s `mcu.hse` gains `startup_max_us`/`startup_source` (specification 22), so `hse_eda_tier1`'s fixture builder takes those fields then.
- The architect merges; the builder never does.
