# AET-WO-1607 — results, beside `PREDICTIONS.md`

Measured 2026-11-23 on `main` `28fba951` (base) and the branch. Evidence: `~/.cache/aet-1607/`.
- **Trees:** each a `git archive` of one commit (`base/`, `br1/` = `cb5dd37c`, the code of (1) and (3)–(6); later commits touch tests and the kit only, plus what §(6) solve records).
- **Binaries:** host debug builds of each tree, run in the pinned tools image (`container/IMAGE`, `e5fc7ddb…`) under `~/bin/capped-run` (8 GiB, swap 0, 8,192 tasks, read back in `caps.log`; 16 GiB for `verify`), `--network none`, one container at a time.
- **Kit:** `sweep.sh` (every committed specification: `validate`, `solve`, `emit`, `check erc`, `check drc` with and without `--spec`, `check support`, `generate`, optionally `prove`) and `same.sh` (byte comparison, each sweep's own directory name normalised); `mutants.py`.

## The baseline

- **B1, hit.** 34 specifications. `validate` 0 on 32, `solve` 29, `emit` 25; `check erc`, `check drc --spec` and `check support` 0 on all 25 emitted; plain `check drc` 0 on 23 (2 judged by KiCad's defaults fail, as on base); `generate` 0 on 29.
- **B2, hit.** 27 specifications carry a `validate` banner: 25 the NRND flash alone, `sentinel-02-power` the flash and the EOL ICM-20948, `imu-baro-1v8` the EOL ICM-20948 and ICP-10111. Of the 25 that emit, 23 carry one; `driver-judges-nothing` and `sentinel-02-bench` carry none.
- **B3, hit.** Two base sweeps are identical, 1,178 files.

## Which lines move (the null sweep, base against branch, no `prove`)

- **V1, hit for the swept commands; V3, V5 hit.** 69 files differ, all stdout lines: `emit`, `check drc --spec` and `check support` of the 23 bannered drawn specifications. Each, parsed, differs from base only by a `lifecycle` key equal to that specification's `validate` banner. Every emitted file (`emit/`'s three files, `generate`'s whole tree) and every other line (`validate`, `solve`, `check erc`, plain `check drc`, `generate`) is byte-identical. No `fab/` byte moves.
- **V2, hit.** Plain `check drc` is unchanged on all 25.
- `prove` and `check pinout` need Kani and an ArduPilot ELF; they are covered by `cli_backlog_tier1` with stub tools (§(6) below).

## (2) and (3) on base

Base `aethelgard-cli` tests with `HOME` a private directory holding `.cargo/config.toml` (`CARGO_HOME`, `RUSTUP_HOME` the real ones) and `CARGO_TARGET_DIR` outside the repository: **37 failures** (`m/base-failures.txt`).
- **P2, hit, narrower than predicted.** 36 `CargoConfigRefused`: `tools_tier1` 35 (every `prove` stub test) and `generate_tier1` 1 (`build_and_prove_refuse_what_cargo_would_run_before_any_tool`). None in `lifecycle_tier1`, `boot_tier1` or `mcp_tier1`.
- **P4, hit.** Both files leave `TMPDIR` unset and take the CLI's default under `$HOME`, so the fix reaches them (outside §3's file list; on the bus 00:43Z).
- **P5, partly missed.** `fab_tier1` fails only `a_rail_or_bus_no_fault_kind_reaches_is_named` (`BaseOutsideRoot`). The tests at lines 350 and 602 pass: their runs are refused before `read_base`.
- **P6, missed.** No other file failed: `campaign_tier1` and `attribute_seeded` pass with the target outside.
- **After (2) and (3):** the touched suites under the same condition, 207 tests, 0 failures (`m/accept-1.log`).

## The null sweep at the final head (`8ff08248`, D1 (b) built)

- **94 files differ, all named by (6):**
  - the 69 lines above, unchanged in kind;
  - 25 `solve` stderr files, each exactly that specification's `validate` banner, one line each. These are the 29 that solve, less the 4 that carry none.
  - `solve`'s stdout is byte-identical on all 29 (`same-head.txt`).
- **V4, resolved by the ruling:** the banner goes to stderr, so the `hwdef` and `pinout` verdicts' `topology_sha256` do not move.
- **V6:** see `verify`, below.

## `prove` with Kani (V1 for `prove`)

On `sentinel-01-minimal`, `sentinel-02-power` and `sentinel-02-bench`, base and branch (`sw/prove-*`):
- every `prove` exits 0 with `pass`;
- `prove --spec` differs only by `lifecycle`, equal to `validate`'s;
- plain `prove` is unchanged.

## Mutants

`mutants.py`, under the acceptance condition (fake `HOME` with `.cargo/config.toml`, target outside the tree): **22 of 22 killed** at `d2a5737a` and again at `8ff08248`. Each is killed by its named test, and every selection passes on the pristine tree (`mut/result-final.txt`).
