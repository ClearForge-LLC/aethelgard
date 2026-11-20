# AET-WO-1210 FEEDBACK — a specification can declare that it draws no board

Builder: Claude (Opus 5.5), a fresh `1210` session, 2026-11-20, from ~09:40 UTC. Worktree `~/aethelgard-1210`, branch `wo/aet-1210-draws-no-board` from `origin/main` `943f7112`. The work order is read from `~/.cache/aet-1210-wo.md`, the architect's copy of the docs branch's `docs/work-orders/AET-WO-1210.md`; this branch does not commit it.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. The proposal (§1.1 and §1.2), written before any build

### 0.1 The field: `board.drawn`

- **Name and place:** `board.drawn`, a boolean beside `board.max_outline_um` and `board.forward`. `false` declares that no board is drawn from this specification. Absent, or `true`, is a drawn board, so every committed specification keeps its meaning.
- **Why this one, in one sentence:** it names the fact that rules 1 and 3 and the four refusals turn on (is this board drawn?), not a purpose, because the never-drawn files are the solver's search fixtures as well as the twin's stand-in (§0.2), and absent is the strict case, as `RUL-14`'s amendment on PR #191 requires of an optional key.
- **Shape:** by `mcu.reset_switch`'s precedent, mirrored.
  - The key is optional; absent reads as `true`, and `true` is not written back.
  - An explicit `null` is refused as the wrong type, as `test_points` and `mcu.reset_switch` refuse it.
- **Version:** specification 20, version lines only, by a renumbering script like `4240v`'s (`renumber19.py`); a version-19 file is refused by name (`UnsupportedSchemaVersion`). `4240a` also takes 20; the second to merge renumbers to 21.
- **Not chosen:**
  - top-level `twin_only: true` (the work order's other example), because `solve/stress.json` and `solve/thrash-pins.json` are never drawn and are not the twin's;
  - a key whose absence relaxes a check: absent here applies rules 1 and 3.

### 0.2 Who declares it (§1.2): the inventory

**The corpus:** the 27 committed specifications at version 19 (a JSON object with `mcu` and `voltage_domains`, outside `spike/`, `docs/` and `target/`): 21 CLI fixtures, `ir/tests/fixtures/sentinel-02-power.json` and 5 under `examples/`. The `spike/` files (versions 16 and 17) and the `1200` repros (version 1) are already refused by version and are not renumbered, as `4240v` left them.

**How "drawn" was found:**

- **Measured:** a local instrument, never committed, in `schematic::design`, the one function every drawing goes through. It logged the sha256 of each specification it drew, and the result. `cargo test --workspace` ran on Furnace with it (tier 1; every tier-2 test self-skips here): 2,393 passed, and 1 failed, `workspace_shape` refusing the instrument's own dependency. Each committed file's hash was computed by parsing and re-serialising it, as the instrument did.
- **Read:** every reference to each file that drew 0 times, in every test (tier 2 included), `.github/workflows/*`, `ci/`, `container/`, `scenarios/` and `fab/`.
  - The log is a lower bound. A CLI run whose environment is cleared does not log: `boot_tier1` `a_console_too_slow_for_the_boot_is_refused_before_it` runs `generate` on `serial-dshot-9600.json` itself, and that file logged 0.

| File | Drawn? | Evidence |
|---|---|---|
| `examples/sentinel-01-minimal.json`, `examples/sentinel-01.json`; CLI `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer`, `four-layer-power`, `motors-dshot600`, `motors-pwm`, `pair-standin`, `sentinel-01-bench`, `sentinel-02-own`, `sentinel-03`, `serial-four`; ir `sentinel-02-power` | **drawn** | tier 1 drew each 1 to 288 times [measured]. `pair-standin` is a stand-in pair of nets, not of parts: `supply_tier1` and `with_tools` emit it [read] |
| CLI `serial-dshot-9600` | **drawn** | `boot_tier1.rs` L2268 `generate`s it, which writes `kicad/` [read] |
| CLI `spi-only` | **drawn** | `campaign_tier2.rs` L1088 `generate`s it [read] |
| CLI `two-bme` | **drawn** | `twin_tier2.rs` L1091 and `boot_tier1.rs` L1713 `generate` it [read] |
| `examples/sentinel-01-ext-bmp581.json` | **drawn**, at the bench | `3403` ran `verify` on it, recorded in `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L34–42 [read] |
| `examples/sentinel-02-bench.json` | **drawn**, at the bench | `3304`'s bench specification, `verify` passed on it [read: `AET-WO-3304` record] |
| `examples/invalid-zero-domain.json` | **never drawn**: `validate` refuses it | it stays refused; no declaration, which would change nothing (§0.3) |
| CLI `imu-baro-1v8` | **never drawn** | the twin's 1.8 V stand-in (`4210` D1). Its six users never draw it: `board_tier2` L144 and `twin_tier2` L1673 use the board generator and the twin's emitter, as "KiCad cannot draw the stand-ins"; `own_models_tier1` and `svd-hal` `board.rs` the same; `speeds_tier1` asserts "no board can be drawn (UnknownPart:"; `fab_tier1` L273 is refused by `validate` first [read]. Tier 1 drew it once, refused `UnknownPart` [measured]. **Gains the declaration.** |
| CLI `sentinel-02-shaped` | **unclear (§7)** | no test or CI job draws it: `twin_tier2`'s `motor_board` only [read, measured 0]. But `1314` and `1314b` emitted and extracted it in their differentials, and roadmap §7.1's router row names it as a board that fails a declared capacitance bound |
| CLI `solve/stress`, `solve/thrash-pins` | **unclear (§7)** | `solve_cli` only solves them [read, measured 0]. `1314b`'s differential drew every specification that solves ("22 of 22"), and the same §7.1 row names `solve/stress` |
| CLI `solve/infeasible-shared`, `solve/pigeonhole-spi` | **never drawn**: `solve` refuses each, so no draw can start | as `invalid-zero-domain`: refused before any board; unclear whether to declare (§0.3) |

**Predicted** from the files' JSON, before any build: rules 1 and 3 refuse exactly one committed file, `imu-baro-1v8` (rule 1: U5 and U7, ICM-20948s on uncrossed bus `IMU`, no row maximum and none declared; rule 3: U6, ICP-10111). Every unclear file passes both rules undeclared. So the unclear list decides only whether `emit` refuses those files, never whether `validate` newly refuses one.

### 0.3 The §7 stop for four files, and what is built meanwhile

- **Built now:** `imu-baro-1v8` declares `board.drawn: false`. No other file is declared.
- **Q1, for the architect:** `sentinel-02-shaped`, `solve/stress`, `solve/thrash-pins`, and the two `solve` fixtures `solve` refuses: declare or leave?
  - **Recommendation: leave all five undeclared.**
    - The first three are drawable and have been drawn (`1314b`), and §7.1 keeps two of them as boards for a later router work order; a declaration would make `emit` refuse them.
    - The last two cannot be drawn either way. A declaration there tests nothing and adds a field no reader needs.
    - Undeclared is the strict case: rules 1 and 3 hold for each, and each passes them.
  - The other reading, the work order's letter ("never emitted in a test or in CI"), declares all five. It is a byte change to five fixtures and moves nothing else; I can apply it in one commit on the ruling.
