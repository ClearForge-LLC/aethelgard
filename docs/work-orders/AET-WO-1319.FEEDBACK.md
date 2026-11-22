# AET-WO-1319 — FEEDBACK: the device arms as rows

**Branch:** `wo/aet-1319-arms-as-rows` from `main` `c1e444dd`; `main` `e3aea224` merged (docs only). **Builder:** Claude (Opus 5.5 orchestrating). Two subagents, each report checked at source before use:
- a fresh Opus reviewer ran the adversarial read;
- a fresh reviewer ran the reduction pass.

Written 2026-11-22.

**Status: READY on green CI.** **The answer, in five lines:**
- **One table, `ARMS`, and one interpreter, `arm()`, in `aethelgard-eda` now draw all eight device parts.** These are the six bus devices (BME280, TMP117, BMP581, W25Q64JVxIQ, ICM20948, ICM42688P) and both pin devices (DRV8876PWPR, DRV8833PWPR). The hand-written arms and `device()` are deleted.
- **Nothing moved, measured against base `c1e444dd`:**
  - all 33 committed specifications (274 files: exit codes, printed JSON, every emitted file);
  - the 1211d harness (**277** cases, 926 files);
  - this work order's pin-device and kind-crossing harness (84 cases, 270 files);
  - `diff` empty at every point (`spike/1319/RESULTS.md`).
- **The harnesses can fail:** three planted mutants, one per kind of row, each killed. The W25Q `WP`/`HOLD` swap moved 23 specifications; BMP581's second decoupler, 7; the DRV8833 passive order, 29 pin-device cases.
- **The four hazards of `1211d` §4 are refused when the table is built,** by a `const` check over the real table (a compile error), with a test per hazard over a bad table.
- **Net +669 lines in `crates/`: +476 code, +193 tests,** against the patch's +108 (§3). Predicted +60 to +200: missed.

## 0. How it was made

- **Predictions first:** `spike/1319/PREDICTIONS.md`, committed at `65910b18` before any run.
- **Kit:** `spike/1319/harness/`.
  - `run.sh` measures one tree: the CLI sweep (1211d's `sweep.sh`) and both harnesses.
  - `compare.sh` judges two trees: `same.sh` for the sweep, `diff -r` for the harnesses.
  - `mutants.sh` plants M1 to M3.
- **Trees:** each tree is a `git archive` copy with the harnesses dropped in and a `Drawing::dump` shim (`{:#?}` of the parts). The shim is applied identically to both trees and never committed. Each run's log was checked to compile `aethelgard-eda` from its own tree, so a shared target dir could not reuse another tree's build.
- **Control:** the branch before the port, measured against base, is identical, so the kit is deterministic.
- **Bus:** Q1 (file layout) and my §7 reading were posted at the start (`~/.cache/aet-bus/architect.md`). No ruling had come by READY; I built my stated fallback (b), §2.

## 1. What was built

- **`crates/aethelgard-eda/src/schematic_pin_device.rs`, now `mod arms`:**
  - **`ARMS`:** eight rows in `KNOWN_PARTS`' order. Each row carries its part, its symbol and footprint, its `Wiring` (an I2C or SPI pin → role map, or a pin device's thermal pad and its `near` rule), its passives (decouplers, an own-net capacitor, or a pin device's required passives with value, rating, dielectric and page), and an optional stand-in (W25Q's ordering code and note).
  - **`check()`:** a `const fn`, evaluated over `ARMS` at compile time (`const _: () = match check(ARMS) …`).
  - **`arm()`:** the one interpreter. In order: the pins; each passive's ends; the part; a pin device's declared resistors and analog outputs; then the row's passives. For a bus device, the pins are `bus_pins`, row order, with straps and own nets claimed as each is reached. For a pin device they are `pin_pins`, the old `device()` body moved verbatim.
  - **Lookups:** `on_bus(kind, …)` gives `UnknownPart` exactly where the old `match d.part.as_str()` did, and `draws` gives `PinDeviceNotDrawn` exactly where the old `ARMS.iter().any` did.
- **`schematic.rs`:** the three call sites stay where they were: I2C, then `behind`, then SPI, then the pin devices after `power::draw`. Each builds its `Declared` and calls `arm()`. `symbol_value` replaces `device()`, because `strict_fence` forbids `.property(` in an `Other` module.
- **Tests:**
  - **`the_arms_are_the_known_parts`**, the same name, now in `arms`. The rows equal `KNOWN_PARTS`, each once and in its order, and each row's kind is its parts row's (I2C, SPI, pin). It replaces the source scan.
  - **`each_bus_rows_pins_are_its_symbols`:** every row pin is a symbol pin, listed once.
  - **`each_bus_rows_own_nets_are_device_nets`:** holds the rows to `aethelgard_ir::names::device_nets`, which restates them (§4 F1).
  - **`each_arms_footprint_is_its_symbols`.**
  - **One test per hazard, and two for the kind fences** (§5).
- **`parts_arms_tier1` passes** (2/2, every part drawn). `TIER2_MIN` is unchanged at 201. No new test binary, so no `ci/` weight row.

## 2. Decisions

- **D1, the layout (Q1, built as (b)).** `strict_fence.rs` (CLI tests, outside the fence) lists every file of eda's `src/` by role and fails on a new, renamed or deleted file.
  - So the table and the interpreter live in the existing `schematic_pin_device.rs`, as `#[path = "schematic_pin_device.rs"] mod arms;`, with a comment saying why.
  - The cost is a file whose name says pin devices while it holds every arm. **(a)** is a rename to `schematic_arms.rs` plus a one-line path edit in `strict_fence.rs`, if you want it.
- **D2, the §7 reading.** The pin-device arms already needed what §4's eight do not list: the thermal pad, the VM-derived rating, a pin-to-pin passive, the anchor and its `near` rule, `PassiveValueUnstated`, and a pin map from the IR row plus the declaration.
  - §1.2 says to fold them in, so I read them as existing features moved, not new ones.
  - No part needed a feature it lacked before.
- **D3, where the footprint lives.** A bus row now carries its footprint, as the work order asks, where it was read from the symbol. A test holds every row's footprint equal to its symbol's `Footprint`. For the pin rows, which were literals already, that is a new constraint; it holds.
- **D4, the hazards as a `const` check, not run-time.** "Refused when the table is built" is read literally, so an address role on an SPI row is again a compile error, as it was before the table. The same `check()` runs over bad tables in the tests.

## 3. The line count (§1.3)

| | net |
|---|---|
| `schematic.rs` code (the arms, `device()`) | −153 |
| `schematic.rs` tests (the source scan) | −32 |
| `schematic_pin_device.rs` code | +629 |
| `schematic_pin_device.rs` tests | +225 |

The patch's +108 was six rows and an interpreter with no checks. On top of that come:
- the `const` table check (137 lines);
- folding the pin devices' interpreter in (the `Declared`/`Context` split, and an `Internal` refusal for each kind mismatch the check already excludes);
- the six rows with their datasheet comments kept (148).

The pin-device rows themselves cost +10 (wrapping), because they were table-shaped already. **The gain is that a new part is a row, not that the code is shorter.**

## 4. Findings

- **F1, the one fact still stated twice: `aethelgard_ir::names::device_nets` restates the bus rows' own-net names** (SDO; WP, HOLD; REGOUT).
  - It is not derived, by the fence, and is held to the rows by a test.
  - **Deriving it, or `KNOWN_PARTS`, would need the table in the IR** (a `parts.json` section for pin map, symbol and passives), because the IR cannot depend on eda. `KNOWN_PARTS` derived in `ARMS` order keeps its bytes. `device_nets` derived needs a row order for the own nets; the rows' order reproduces today's.
  - That is the schema-epoch decision the work order names.
- **F2, a stale doc comment outside the fence.** `crates/aethelgard-cli/tests/parts_arms_tier1.rs` says an arm with no row "is the source scan's in `schematic.rs` (`the_arms_are_the_known_parts`)". The test keeps its name but is now in `schematic_pin_device.rs`, and it compares the table with the list; it no longer scans the source. One line, for whoever next touches that file.
- **F3, what `check()` does not hold** (from the adversarial read; observations, not divergences):
  - an I2C row with no `Addr` (`validate` still refuses an unsupported address);
  - the count of a second, third or fourth decoupler (only the byte sweep holds those);
  - a strap and an own net of one name in one row (refused at run time as `ReservedName`, which blames the specification, and caught by the `device_nets` test);
  - a pin listed twice in a row (a unit test, not the `const` check).
  None is reachable by today's rows. Each would be a line of `check()` if you want it.
- **F4, 1211d's "274-input harness" is 277 cases.** Its "274" was the sweep's file count at 33 specifications. The case count was 277 in its own Nov 20 output too. Corrected here (prediction B2, missed).
- **F5, the mutants of 1211d's D4 are not recorded by name.** `d4out-mut` shows one, the W25Q swap. M1 reconstructs it; M2 and M3 are new (`harness/mutants.sh`).

## 5. Adversarial pass (§5), full

- **Inputs the 274 do not cover: `spike/1319/harness/d1319.rs`, 84 cases, all identical between the deleted arms (base, built from `c1e444dd`) and the interpreter.**
  - **Every pin-device variation:** straps as a tie to each rail, as a resistor, and left out; the nFAULT pull as a resistor, as the MCU's pull, and undeclared; IPROPI without its reservoir and undeclared; bridge outputs on GND, VM, 3V3 and on no connector.
  - **Ratings and names:** five VM domains for the rating's arithmetic; references and own nets that collide with the emitter's numbering, case variants included.
  - **Boards:** two DRV8833s, in both orders; a pin device beside I2C and SPI devices; two I2C buses.
  - **Kind crossing both ways:** a bus part (or `FOO`, or a near spelling) as a pin device, and each DRV part on the I2C bus, on the SPI bus and behind the crossing.
- **The reviewer's read** (fresh, given the code and not my conclusion):
  - **Result:** no divergence, CONFIRMED or PLAUSIBLE, across fallible-call order, lookups, value, footprint, note, rails and pin-device order.
  - **Its seven inputs** are in d1319 §11. They cover which refusal wins: VM not a domain; a strap on a pin the row lacks, alone and with a loose bridge; one pin on two connections; an unsolved analog net, alone and with a loose bridge; a mixed board with declared references spelled like emitter numbers; two I2C buses; an own-net collision crossed with an unsupported address.
  - **Outcomes:** they exercise UnsolvedNet over a loose bridge, UnsupportedAddress over ReservedName, and DuplicateReference on `r5`/`c7`. All identical.
- **The four hazards, each refused by a test:**

| hazard (`1211d` §4) | refused by |
|---|---|
| the own-net capacitor's `format!` name | gone: its net is its `Own` pin's drawn connection. `check()` refuses a capacitor that names no `Own` pin of its row, an own net no capacitor (or two) closes, and an own net named twice (`an_own_net_capacitor_names_its_own_pin`) |
| an address role on an SPI row | `check()`, so a compile error over `ARMS` (`an_address_strap_on_an_spi_row_is_refused`), with an I2C line on an SPI row, an SPI line on an I2C row, two address straps, and a strap target other than GND, the supply or a bus line |
| a duplicate row | `check()` (`a_part_with_two_rows_is_refused`, bus and pin rows) |
| an I2C decoupler left out | `check()`: a bus row with no decoupler, and an I2C row whose first passive is not its 100 nF (`an_i2c_row_without_its_decoupler_is_refused`); an SPI row's decouplers stay its own |

- **The test that could not fail,** `the_table_passes_its_own_check` (the `const` assertion already evaluates it), was removed at the reviewer's note.

## 6. Mutants (§2)

| mutant | sweep (33 specs) | d4diff (277) | d1319 (69) | verdict |
|---|---|---|---|---|
| M1 W25Q `WP`/`HOLD` swapped | 23 specifications, 69 files | 73 cases | 2 cases | killed |
| M2 BMP581's second decoupler left out | 7 specifications, 28 files | 35 cases | — | killed |
| M3 DRV8833 CVCP/CVINT reordered | — (CLI `emit` refuses a pin device until 1211b) | — | 29 cases | killed |

Each mutant's log shows `aethelgard-eda` rebuilt from the mutated tree.

## 7. Predictions beside results

In `spike/1319/RESULTS.md`. Two missed:
- **B2:** 277 cases, not 274 (F4);
- **P4:** +669 net, not +60 to +200 (§3).

## 8. Gates, fence, reduction pass

- **Gates** at the READY code head: see the final line below.
- **Fence:** `crates/aethelgard-eda/src/{schematic.rs, schematic_pin_device.rs}`, `spike/1319/`, and this file. Nothing in the IR, the generator, the twin, the specifications, the workflows, `ci/` or `aethelgard-kicad-lib`.
- **Reduction pass (diff): −8 net · 1 duplicate removed (a second pin-number lookup per required passive's end) · 1 dead test removed (`the_table_passes_its_own_check`) · 0 protected kept · 2 findings** (F1, F2).
  - A fresh reviewer proposed three cuts. One was taken. One, folding the pin-device fields into `Context`, was measured here before: +5 after rustfmt, so it was dropped. The last, inlining `draws`, is a rename.
