# AET-WO-1204 — FEEDBACK

**Branch:** `wo/aet-1204-ir-fences` · **Base:** `8d02e3f` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only. No §7 stop.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Grounds, proven first (§7, kickoff item 2)

On this repository's pinned toolchain (cargo 1.99.1, clippy 0.1.99), in a scratch copy with a crate-local `clippy.toml` in `aethelgard-cli`:

- A direct call, `let sp = &spec; sp.declared_i2c_devices()`, and a function-pointer alias `let f = aethelgard_ir::Specification::declared_i2c_devices;` each fail `cargo clippy -p aethelgard-cli -- -D warnings` with `use of a disallowed method`, exit 101.
- Renaming the path in `clippy.toml` leaves the call unflagged: `warning: ... does not refer to a reachable function`, and the exit is **0** under `-D warnings`. That is why §1.1(d) exists.

## §3 acceptance, on the final tree

`cargo clippy --workspace --all-targets -- -D warnings` in a scratch copy, one mutant at a time (script output kept in the session scratchpad):

| Mutant | Result |
|---|---|
| cli: `let sp = &spec; let _ = sp.declared_i2c_devices().len();` (the review's mutant) | `error: use of a disallowed method`, exit 101 |
| cli: `let f = Specification::declared_spi_devices;` | same, exit 101 |
| twin `src`, svd-hal `src`, cli `tests/` (each one call) | same in each, exit 101 (each crate's file is picked up; `--all-targets` covers tests) |
| cli: `spec.i2c_devices.len()` | `E0616` field is private |
| `declared_i2c_devices` renamed in `aethelgard-ir` alone | `rul16_fence` fails to build: `E0599 no associated function ... declared_i2c_devices` |
| `declared_spi_devices` misspelt in `twin/clippy.toml` | `each_runner_disallows_exactly_the_two_accessors` fails, naming the file and both lists |

F14: `ceiling_ns: u32::MAX` is refused (`CeilingNotDerived`, `derived_hz` `None`); a 400 kHz ceiling over lines of 120 ns is refused (`derived_hz` 1,000,000); the whole workspace suite is green, so every topology `solve` produces in it validates. Tests: `validate::solved_refuses_*`, `solved_derives_the_ceiling_from_the_smallest_bound_of_the_bus`.

3c: `emit_tier1::names_that_differ_only_in_case_from_the_emitters_are_refused` (`u1_vcap1` as a signal net → `ReservedName`, id `U1_VCAP1 and u1_vcap1`; reference `u1` beside the MCU's `U1` → `DuplicateReference`, id `U1 and u1`; `u2` beside `U2` → `DuplicateReference`, the message names `U2`), plus `validate::references_that_differ_only_in_case_are_duplicates`. All exit 2.

## Choices the work order left to the builder

- **§1.1(a) literals:** the fields are private and the integration tests of `aethelgard-ir` build and edit specifications through JSON (`tests/common/mod.rs`, a guard that copies the lists out and writes them back on drop). No constructor, no public `&mut`, no writer: nothing outside `ir` needed one. (The smaller change: ~47 sites were a mechanical rewrite; a constructor would have been a new public API.)
- **§1.1(f):** the `spec.i2c_devices` scan is gone; `the_eda_readers_are_exactly_the_known_two` is **retargeted** to `the_eda_reader_is_exactly_the_schematic` (the accessor calls in `eda/src` sit in `schematic.rs` alone), because `eda` stays unfenced and a second reader should still go to the architect.
- **§1.3 error shape:** no new `EmitError` variant. A case-only collision is `ReservedName` / `DuplicateReference` whose string is `A and B`, the form `pcb.rs`'s pair check already uses. In `validate`, `DuplicateReference` gained a `first` field; the id is still the offending reference.
- **`emit`'s own duplicate-reference check** is kept: through the CLI `validate` runs first and refuses a duplicate, but `emit` is a library function and `Builder::reference` needs the folded set anyway.
- **`ceiling_hz`** moved to `electrical::ceiling_hz` with its test.
- **§1.2 rule is exact in both directions:** a ceiling for a bus with no analog line is refused (`derived_hz` `None`), and so is a bus that does not hold exactly one SDA and one SCL line (adversarial B1). `solve` always produces both.

## Fixtures that moved (§1.2)

No topology a test got from `solve` moved, no committed generated file changed, and the regeneration guard passed. Hand-built topologies corrected to what `solve` derives:

| Where | Old | New |
|---|---|---|
| `ir/tests/validate.rs` `solved()` | SDA line carried signal `I2C1_SCL` | `I2C1_SDA` (a fixture slip; the B1 rule counts lines by signal) |
| `validate::solved_accepts_each_mode_and_refuses_any_other_ceiling` | ceilings 100k / 400k / 1M over lines of 300 ns | lines of 1000 / 300 / 120 ns respectively |
| `svd-hal/src/board.rs` tests (three) | ceilings set to 100k / 1M / `A`,`B` over lines of 300 ns | `set_ceiling` sets the bound that derives the ceiling |
| `svd-hal` `ceilings_must_number_the_i2c_peripherals...` | a ceiling for bus `OTHER` expects `CeilingMismatch` | expects `InvalidTopology` (validate refuses a bus with no line first) |

No chaos or propose path builds a deliberately underived ceiling (none mutates `i2c_ceilings`; the suite is green).

## Adversarial pass (targeted; a fresh subagent, then spot-checked)

| Job | Finding | Disposition |
|---|---|---|
| B1 | A bus with one line (SDA at 100 ns, SCL deleted) and a 1 MHz ceiling validated | **Fixed** (exactly one SDA and one SCL per bus), test `solved_refuses_a_half_bus_whose_one_line_would_set_the_ceiling` |
| A1 | `#[allow(clippy::disallowed_methods)]` in cli/twin silences the fence | `forbid` in `[workspace.lints.clippy]` was tried and **cannot** hold: clap's derives emit `allow(clippy::style)`, `E0453`. Compensating control: `solved_devices::no_fenced_crate_allows_the_disallowed_methods_lint` refuses the lint's name in the three crates' `src` and `tests`. |
| A2 | `cli` calls a `pub fn` added to `eda` that returns the list | Residue, not fixed: `eda` is unfenced by ruling (2026-11-09); the dependants test and the schematic-only reader test do not see an eda API. Fix if wanted: a test pinning `eda`'s pub items, or `pub(crate)` discipline. **decision-needed** (architect) |
| A3 | A doc-test in twin calls the accessor | Residue: clippy does not run on doc-tests, and the `Specification` grep skips comments. Low |
| A4 | `CLIPPY_CONF_DIR`, `-A clippy::disallowed_methods`, a `[lints]` override | Residue: needs CI or manifest control, which is review's ground |
| A5 | JSON / serde round trip, `{:?}`, reading the file as `Value` | The honest residue: the fence binds the types, not the bytes |
| A6–A13 | path-as-value, UFCS, `use as`, fn pointer, closure/trait wrapper, macro from `ir` or `eda`, `build.rs`/`tests/`/`examples/`, `Deref`/derives | Held |
| B2–B9 | two buses, case-differing bus key, unused bus, lines on different peripherals, zero bound | Held |
| C | ~85 variants against emitter nets, rails, references, power symbols (`gnd`, `u1_vcap1`, `u2_sdo`, `r1`, `c1`, `U01`, `#pwr01`, domains, non-ASCII) | None accepted; every collision exits 2. Non-ASCII is unreachable (`identifier` is printable ASCII). The unmodified example still emits |

## Reduction pass

- `solve.rs`'s private `ceiling_hz` and its test moved, not duplicated.
- **Now unreachable, outside the fence, not touched:** `svd-hal`'s `BoardError::CeilingMismatch` (`board.rs`). `validate` now guarantees one ceiling per I2C bus and one bus per peripheral, so the count check cannot fail on a validated topology. Backlog: delete it with its variant and kind.
- `Taken` replaces two `BTreeSet`s plus an exact-duplicate branch; nothing else was added to `eda`.

## Gaps and backlog

- **The pull-up floor and the capacitance limit on a hand-written topology** stay unchecked (`AET-WO-1203`). Both depend on the bus's declared speed, which `SolvedTopology` does not carry. Carrying it means one new `speed_hz` per `i2c_ceilings` entry (or per `analog_lines`) in the solved schema, `solve` writing it, and `validate` calling the two `electrical` functions: a schema change, left for a ruling.
- Steering documents, not touched (protected): `architecture.md` `RUL-16` says "A test pins its readers at exactly two"; since this work order the compiler fences `cli`, `svd-hal` and `twin`, and the `eda` readers are pinned by file. Wording for the architect.

## Where everything ran

Tier 1: host, `cargo test --workspace --no-fail-fast` in a 32 GB cgroup scope, 6 streams: all green. Tier 2: the pinned tools image (`container/IMAGE`) under rootless podman, `--memory 24g --memory-swap 24g --pids-limit -1`, `0` written to `memory.swap.max` after start. Read back for every container: `memory.max` 25769803776, `memory.swap.max` 0, `pids.max` max. Build (with network), `cargo fmt --all -- --check` and `cargo clippy ... --features aethelgard-cli/with-tools -- -D warnings` exited 0 on the first tree; the test step is in the PR description. `TIER2_MIN` is unchanged (67): no tier-2 test was added.
