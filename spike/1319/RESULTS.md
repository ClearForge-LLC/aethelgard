# AET-WO-1319 — results, beside the predictions

The predictions were committed first, at `65910b18`. Base: `main` `c1e444dd`. Kit: `harness/run.sh` measures one tree, `harness/compare.sh` judges two, `harness/mutants.sh` plants the mutants. Every tree is a `git archive` copy, never the worktree. Each build was checked to compile `aethelgard-eda` from its own tree, because a shared target dir could otherwise reuse another tree's build. Evidence: `~/.cache/aet-1319/m/`.

## The measurement

| measured | base `c1e444dd` against | result |
|---|---|---|
| **control:** the branch before the port (`65910b18`'s code = base) | base | sweep identical, 274 files; d4diff identical, 277 cases / 926 files; d1319 identical, 69 cases / 230 files |
| **the port** (`c59ba5c5`) | base | sweep identical, **274 files on 33 specifications**; d4diff identical, **277 cases / 926 files**; d1319 identical, **69 cases / 230 files** |
| **the port, d1319 with the reviewer's inputs** (84 cases) | base | identical, **84 cases / 270 files** |
| **the READY head** | base | see the final table below |

- **The sweep:** every committed specification's `validate`, `solve` and `emit`, with exit codes, printed JSON (the output directory normalised by `same.sh`) and every emitted file.
- **d4diff:** the 1211d reviewer's harness, copied unchanged. For each case it records validate, solve, the draw (kind and message, or the part dump), and `emit`'s three files.
- **d1319:** this work order's harness. Every case declares a pin device or crosses kinds, and is drawn against a donor's topology where the IR refuses it.

## Predictions beside results

| # | predicted | measured |
|---|---|---|
| B1 | 33 committed specifications | **33**, 274 files |
| B2 | d4diff compiles with a `dump` shim and writes 274 cases | compiles. It writes **277 cases** (926 files): 1211d's "274" was its sweep's file count, not the harness's case count. Its Nov 20 output also has 277 case directories. **Missed** |
| B3 | d1319 covers pin devices and kind crossing | 69 cases, 84 with the reviewer's inputs. Its outcomes include draws, emits, and refusals of 9 kinds |
| P1 | sweep identical | **identical** |
| P2 | both harnesses identical | **identical** |
| P3 | no feature beyond what exists | **held**. The pin-device features moved verbatim (`pin_pins` is the old `device()` body). Two new things are row data, not features: the bus kind as `Wiring`, and the stand-in. |
| P4 | net +60 to +200 lines in `crates/` | **+672: +479 code, +193 tests. Missed** (below) |
| P5 | `parts_arms_tier1` passes; `TIER2_MIN` 201 | passes (2/2); `TIER2_MIN` untouched |
| M1 | W25Q `WP`/`HOLD` swap moves W25Q cases and committed specs | **killed:** 23 specifications (69 files); d4diff 73 cases; d1319 2 cases |
| M2 | BMP581's second decoupler moves BMP581 cases and `sentinel-01-ext-bmp581` | **killed:** 7 specifications (28 files, `sentinel-01-ext-bmp581` among them); d4diff 35 cases |
| M3 | DRV8833 CVCP/CVINT reorder moves d1319's DRV8833 cases and no committed specification | **killed:** d1319 29 cases; sweep and d4diff identical, as predicted |

## Line count (§1.3)

`git diff --numstat c1e444dd HEAD -- crates`: +1074 / −402.

| | before | after | net |
|---|---|---|---|
| `schematic.rs` code | 2312 | 2159 | −153 |
| `schematic.rs` tests | 382 | 350 | −32 (the source scan replaced) |
| `schematic_pin_device.rs` code | 575 | 1207 | +632 |
| `schematic_pin_device.rs` tests | 0 | 225 | +225 |

What the +479 of code is, against the patch's +108:
- **the six bus rows:** 148 lines, against the patch's 131 (doc comments kept);
- **the compile-time table check, `check()`:** 137 lines (`const fn`, so `while` loops). The patch had none; it is what refuses the four hazards when the table is built;
- **`arm()`:** 203 lines, one interpreter for both kinds. Each passive's ends are resolved before the part is pushed, and a pin device's declared resistors are drawn after it, both as before. Each kind mismatch the table check already excludes is refused `Internal`;
- **plumbing:** the bus devices' `bus_pins` (45), `on_bus`/`pin_arm`/`draws` and the `Declared`/`Context` types.

The pin-device arms change the patch's figure in two ways. They are already table-shaped, so their rows cost little (+10 lines of wrapping). Folding their interpreter into the bus devices' costs the `Context` split and the per-kind match arms.
