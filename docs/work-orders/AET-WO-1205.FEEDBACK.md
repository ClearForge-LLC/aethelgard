# AET-WO-1205 — FEEDBACK

**Branch:** `wo/aet-1205-declared-speed` · **Base:** `366e8fd` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only. No §7 stop.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## What was built (§1)

1. **`SolvedTopology.i2c_speeds`**: a `BTreeMap` of bus id to the declared `speed_hz`, keys exactly those of `i2c_ceilings`, duplicate keys refused. `solve` writes it from `I2cBus::speed_hz`. `SOLVED_SCHEMA_VERSION` is now 2 (`aethelgard_ir`); `UnsupportedSchemaVersion` carries `expected` so its message says `not 2` for a topology and `not 1` for a specification.
   - **A version-1 file is refused by `UnsupportedSchemaVersion`, not a missing key.** `SolvedTopology` now deserializes through a private wire struct whose `i2c_speeds` may be absent. Absent under any version other than 2 reads as empty, and `validate` refuses the version first. Absent under version 2 is `missing field i2c_speeds`: never defaulted.
2. **`validate`**, after the ceiling check (`validate_i2c_speeds`): `SpeedBusMismatch` (key sets differ; `id` is the stray key), `UnsupportedI2cSpeed` (not 100k/400k/1M; this is "a speed `i2c_mode` does not admit", since `i2c_mode` itself maps anything above 400 kHz to Fast-mode Plus), `BusTooFast`, `PullUpBelowSinkFloor` (every line, at its own rail) and `BusCapacitanceOverLimit`. The last three are new `IrValidationError` variants with the same kind, fields and message as `SolveError`'s; `kind()` and `id()` agree, so a hand-written topology and a specification refuse alike.
3. **Generator:** `generate_with` admits the modes at or under the lowest declared speed (`i2c_speeds`), not the lowest ceiling. The count check against `i2c_ceilings` stays. The emitted doc comment now reads ``The speeds at or under `I2C1`'s declared speed of 400000 Hz.``
4. **Readers** that built or parsed a topology: `ir/tests/{validate,solve}.rs`, `cli/src/analog.rs` (JSON literal), `cli/src/campaign.rs` (JSON fixture), `cli/tests/solve_cli.rs` (digests), `twin/src/tests.rs` (`REFERENCE`; the "wrong version" edit is now 2→3), `twin/src/analog.rs` (one test, below), `svd-hal/src/board.rs` tests. No other reader of a solved topology's JSON exists: every other consumer gets it from `solve`.

## §3 acceptance, on the final tree

- **Reference:** `aethelgard solve` gives `"schema_version":2`, `"i2c_speeds":{"SENSORS":400000}`.
  - **New `topology_sha256` of the reference: `c655bf2ee3097e37be0268ddf688a879345c9a80e9b3de59441c5a981c3f6701`** (was `d684cd6d…`). The stress fixture's solve digest is now `6eda552c…` (was `dd192c61…`). Both are in `solve_cli.rs`. Run ids and `topology_sha256` change for every run; there is no committed archive or proposal on `main` to regenerate (checked at build: none).
- **`boards/sentinel-01/**` differs from `main` in one line**, regenerated with `aethelgard generate --spec examples/sentinel-01-minimal.json` and copied, never edited by hand. `diff -r` of the regenerated `board/` against the committed tree showed this and nothing else:
  - `boards/sentinel-01/src/board.rs:114`: `/// The speeds at or under `I2C1`'s solved rise-time ceiling of 400000 Hz.` → `/// The speeds at or under `I2C1`'s declared speed of 400000 Hz.`
- **Refused by name** (`validate.rs`): a version-1 file (`UnsupportedSchemaVersion`, with and without the key); a speed key missing or extra (`SpeedBusMismatch`); 1.5 MHz (`UnsupportedI2cSpeed`); 1 MHz under a 400 kHz ceiling (`BusTooFast`); 966 Ω at 3.3 V and 400 kHz refused, 967 Ω validates; 400 001 fF refused and 400 000 validates at 400 kHz; 550 000 validates and 550 001 refuses at 1 MHz. Also the 2000/2001 mV rail boundary (799/800 and 533/534 Ω), a 1 MHz ceiling over a 400 kHz bus (floor and limit follow the speed), and `solve` carrying a 100 kHz bus's own speed under a 400 kHz ceiling.
- **Generator:** `the_declared_speed_not_the_ceiling_decides_the_admitted_modes`: 100 kHz declared under a 400 kHz ceiling admits only `Standard100k` and the harness drives only `i2c1_speed::Standard100k`.
- `TIER2_MIN` unchanged (71): no tier-2 test added.

## Mutation (sized to the diff, one mutant each, scratch copy)

Nine mutants of the guards this WO adds: `<`→`<=` on the floor, `>`→`>=` on the limit and on the ceiling, the missing-key gate, the version check, the key-set check, generator speed→ceiling and min→max. Eight were killed on the first run. **One survived**: the floor computed at the ceiling instead of the declared speed (every test had speed = ceiling). It was killed by `the_floor_and_the_limit_follow_the_declared_speed_not_the_ceiling`, which was added; that mutant and the same one on the capacitance limit now fail it.

## Adversarial pass (targeted, fresh subagent, read-only)

Nothing found in the three hunts (§5): a sweep of 100k/400k/1M against rails of 1 to 5001 mV, 65535, 10^6 and `u32::MAX` at floor−1, floor and floor+1 on each line, against an independent computation, 0 mismatches; no generator path reads `i2c_ceilings` for modes, and `Plan::new` validates first (checked: `board.rs:449`); a topology with a version other than 2 and no `i2c_speeds` deserializes (by design) and is refused only by `validate`; every in-repo consumer validates.

## Reduction pass (diff): −0 lines net · 0 duplicates removed · 0 dead paths removed · 0 protected kept touched · 2 findings below

## Findings and gaps

- **F1 · `IrValidationError` and `SolveError` both carry `BusTooFast`, `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit` · `ir/src/lib.rs`, `solve.rs` · reduction · keep one: have `solve` return them as `InvalidSpecification(…)` and delete the `SolveError` copies, which edits tests that construct `SolveError::PullUpBelowSinkFloor` (`ir/tests/solve.rs`) and the CLI's `claimants` · outside the fence · decision-needed: yes, backlog.** Likewise `solve::i2c_electrical` and `validate_i2c_speeds` state the same two rules; `solve` then re-validates its output, so one could go once F1 is taken.
- **F2 · A twin test lost a value · `twin/src/analog.rs`, `values_reach_the_deck_as_written` · type: test edit, not a weakening of any guard · the test fed `capacitance_femtofarads = u64::MAX` and expected it in the SPICE card; `decks` validates, and a topology above 400 000 fF at 400 kHz is now refused, so the value is 400 000 (the largest valid) and the expected card `C1 line 0 400000f`.** What the test shows (a value reaches the deck as written) holds. What it no longer exercises is `u64` width in the formatter. `validate.rs`'s "huge values" test changed the same way (capacitance 400 000; the other three fields stay at their maxima). Flagged because §7 names tests outside the fence: both are in the twin's and `ir`'s own fixtures, which §1.4 puts in scope.
- **Stale prose, not touched (outside the fence):** `crates/aethelgard-ir/README.md` (the `SolvedTopology` field list, the `validate` order, the schema example), `crates/aethelgard-svd-hal/README.md:61` ("The I2C ceiling"), `crates/aethelgard-cli/README.md:58` (the `solve` output example), and the comment in `svd-hal/tests/ui/fail/bus_too_fast.rs`. They describe version 1 and the ceiling. Wording for the architect or a docs WO.
- `aethelgard-cli`'s `propose.rs` still reads the declared speed from the `Specification` (`facts`), on its own comment's reasoning. `i2c_speeds` now offers the same fact from the topology; moving it is `RUL-16` tidying, not this WO.
- Steering docs not touched (protected): `docs/roadmap.md` and `architecture.md` `RUL-16` list the solved fields and name version 1.
- A hand-written topology's `domain_millivolts` per line cannot be checked against any rail, since the topology carries no specification (the adversarial pass's observation). The floor is as honest as that value.

## Where everything ran

Tier 1: host, `cargo test --workspace --no-fail-fast -j6` in a 32 GB cgroup scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`): green. `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: green. Tier 2: the pinned tools image (`container/IMAGE`) under rootless podman, `--memory 24g --memory-swap 24g --pids-limit -1`, `0` written to `memory.swap.max` after each start. Read back for every container: `memory.max` 25769803776, `memory.swap.max` 0, `pids.max` max. Build, bridge build and test, fmt and clippy (with `--features aethelgard-cli/with-tools`) exited 0; the test step's result is in the PR description. The mutation and adversarial runs were on scratch copies under the session scratchpad and `~/.cache`.
