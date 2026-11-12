# AET-WO-1204 — `RUL-16` as a compiled fence; hand-written topologies hold only derived ceilings; emitter names fold case

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1204-ir-fences`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P1 refinement, from the Mythos 5.2 review of 2026-11-12 (A7.1, B6) and two carried items (`AET-WO-2200` F14; `AET-WO-1900` adversarial 3c). Backlog `docs/roadmap.md` §7.1.

**Grounds:**
- **The review's A7.1, reproduced by mutant:** `crates/aethelgard-cli/tests/solved_devices.rs` matches the literal text `spec.i2c_devices`. `let sp = &spec; let _ = sp.i2c_devices.len();` in `main.rs` passes it.
- **Only `aethelgard-eda`'s `schematic.rs` reads a specification's devices today.** Every other `.i2c_devices` outside `aethelgard-ir` is a `SolvedTopology`'s. `aethelgard-chaos` and `aethelgard-evolve` do not depend on `aethelgard-ir` at all. The crates that do are `cli`, `eda`, `svd-hal` and `twin`.
- **The architect's experiment (a scratch workspace, clippy 0.1.99):**
  - A crate-local `clippy.toml` with `disallowed-methods = [{ path = "ir::Spec::declared_devices", reason = "RUL-16" }]` fails `cargo clippy --workspace -- -D warnings` on a direct call, on `let sp = &s; sp.declared_devices()`, and on a function-pointer alias `let f = ir::Spec::declared_devices;`. The path is resolved by the compiler, not matched as text.
  - **But renaming the method leaves the fence dead:** clippy prints `warning: ... does not refer to a reachable function`, and the exit is **0** even under `-D warnings`. §1.1(d) closes that.
- **`AET-WO-2200` F14:** a hand-written `SolvedTopology` with `ceiling_ns: u32::MAX` validates. `solve` would refuse it (`RiseTimeTooSlow`). Nothing reaches this through the CLI, which only ever solves a specification, but the twin and `svd-hal` each validate a topology they are handed.
- **`AET-WO-1900` adversarial 3c:** a signal net `u1_vcap1` beside the emitter's own `U1_VCAP1` is accepted by `emit`. `Builder::own_net` (`crates/aethelgard-eda/src/schematic.rs`) compares names exactly. `validate` already folds case for nets and domains (PR #87). Device references are compared exactly in both `validate` (step 8) and `emit`.

> **What this is:** three narrow hardenings at the specification and solved-topology boundary, each proven by the mutant that defeats it today.
>
> It is NOT:
> - any change to the JSON either type reads or writes;
> - any emitted byte for a specification that passes;
> - static electrical rules on a hand-written topology (see §1.2, last bullet).

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

### 1.1 `RUL-16` as a compiled fence (review B6)

a. **`Specification.i2c_devices` and `.spi_devices` become private to `aethelgard-ir`.** Serialization, `deny_unknown_fields`, the missing-key refusal and every error stay exactly as they are. Any test that builds a `Specification` literal outside the crate moves to a constructor or to JSON. Choose the smaller, and say which in FEEDBACK.

b. **Reading goes through two accessors only:** `Specification::declared_i2c_devices(&self) -> &[I2cDevice]` and `Specification::declared_spi_devices(&self) -> &[SpiDevice]`.
   - `RUL-16` fences reading. If a runner needs to write the lists (a scenario override), add the narrowest writer and record why.
   - `aethelgard-ir`'s own code uses the fields directly.

c. **A `clippy.toml` at each of `crates/aethelgard-cli`, `crates/aethelgard-svd-hal` and `crates/aethelgard-twin`** lists both accessors under `disallowed-methods`, each with `reason = "RUL-16: ..."`.
   - No `clippy.toml` exists anywhere today, so these lose no configuration. Confirm that before you add them.
   - `aethelgard-eda` gets none: the schematic draws declared intent (`RUL-16` scope, 2026-11-09).

d. **A guard test in `crates/aethelgard-ir/tests/`**, a crate with no fence of its own. It:
   - names each accessor as a compiled item (`let _: fn(&Specification) -> &[I2cDevice] = Specification::declared_i2c_devices;`), so a rename fails to compile until the test is edited;
   - asserts that each of the three `clippy.toml` files lists exactly those two paths, spelled as the test names them.

e. **A `workspace_shape.rs` assertion:** the members depending on `aethelgard-ir` are exactly `cli`, `eda`, `svd-hal` and `twin`. A fifth dependant would sit outside the fence. Extend that file's existing helpers; do not add a parser.

f. **Retire the text greps in `solved_devices.rs`.**
   - The `spec.i2c_devices` scan goes.
   - `the_eda_readers_are_exactly_the_known_two` is either retargeted to count calls of the two accessors in `eda/src`, or removed with its reason (reduction pass). Choose one.

### 1.2 A hand-written topology holds only the ceilings `solve` derives (`AET-WO-2200` F14, ruled here)

- **`SolvedTopology::validate` re-derives every `i2c_ceilings` entry from its own `analog_lines`:**
  - the derived ceiling is the mapping of the smallest `ceiling_ns` among that bus's lines;
  - the mapping is `solve.rs`'s `ceiling_hz`, which moves to `aethelgard_ir::electrical` as a `pub fn`, so `solve` and `validate` share one function;
  - a mismatch refuses with a new named error, for example `CeilingNotDerived { bus, ceiling_hz, derived_hz }`;
  - a `ceiling_ns` slower than every mode refuses with that error, `derived_hz` being `None`;
  - the error's CLI and JSON mapping follows the existing variants.
- **Fixtures that move:**
  - A topology a test produced by `solve` cannot move. If one does, that is a §7 stop.
  - A hand-built topology is corrected to what `solve` would derive, and listed in FEEDBACK with the old value and the new.
  - A committed generated file that changes is a §7 stop.
- **Chaos and propose:** check every path that builds or mutates a `SolvedTopology` after `solve` (the CLI's campaign runner, `propose`, the twin's and `svd-hal`'s tests). If any builds a deliberately underived ceiling, that is a §7 stop. Do not weaken the rule.
- **Not in scope:** the pull-up floor and the capacitance limit on a hand-written topology. Both depend on the bus's declared speed (`AET-WO-1203`), and `SolvedTopology` does not carry it. Carrying it is a schema change, left for a ruling. Record the gap in FEEDBACK with one line on what carrying it would take.

### 1.3 Case folding at the emitter's names (`AET-WO-1900` 3c)

- **`own_net` refuses a name that folds (ASCII) onto a name already taken:**
  - the check is case-insensitive, like `validate`'s net and domain checks;
  - the refusal names both spellings;
  - the error is `ReservedName`, or a new variant if naming two is cleaner. The CLI exits 2.
- **Device references fold in `validate`'s step 8,** so `U2` beside `u2` is `DuplicateReference`. If `emit`'s own exact-duplicate check becomes unreachable, remove it (reduction pass) or keep it with the reason.
- **Prove each with a tier-1 test** through `emit`, without `kicad-cli`:
  - `u1_vcap1` as a signal net;
  - a device reference `u1` beside the MCU's `U1`;
  - `U2` beside `u2`.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-06`, `RUL-16`. No `unsafe`.
- **Fence:**
  - `aethelgard-ir`;
  - `aethelgard-eda`'s `schematic.rs` (the accessor calls and `own_net`);
  - the three new `clippy.toml` files;
  - the CLI's error mapping;
  - tests and their fixtures;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical.** The regeneration guard passes with no change.
- **`AET-WO-3301` is running in parallel** (`~/aethelgard-3301`: the CLI, `svd-hal`'s board tests, the twin's platform). Never touch another worktree. If it lands first, merge `origin/main` with a merge commit; any new reader of a specification's devices it added must go through the accessors, or it is a §7 stop.
- `cargo fmt -p <crate>`, never `--all`, while another builder runs.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **The review's mutant now fails.** In a scratch copy, `let sp = &spec; let _ = sp.declared_i2c_devices().len();` in `crates/aethelgard-cli/src/main.rs` fails `cargo clippy --workspace --all-targets -- -D warnings`. Record the output.
- **Each other route also fails:**
  - a function-pointer alias fails clippy;
  - `spec.i2c_devices` fails to compile (`E0616`);
  - renaming `declared_i2c_devices` in `aethelgard-ir` alone fails the build of the guard test.

  Record each in FEEDBACK.
- **F14:**
  - `ceiling_ns: u32::MAX` is refused by `SolvedTopology::validate`;
  - a 400 kHz ceiling over lines of 120 ns is refused (`derived_hz` 1,000,000);
  - every topology `solve` produces in the test suite validates.
- **3c:** the three §1.3 inputs are each refused with a JSON error naming both spellings. Every currently passing specification still emits byte-identically.
- **The gates** (`CLAUDE.md`) pass. So do `gates` and `tools` in CI. The tier-2 canary floor (`TIER2_MIN`) is unchanged unless you add tier-2 tests; if you do, raise it by exactly that many, with a comment.

## 5. Adversarial pass — targeted

1. **Read a specification's devices from `cli`, `svd-hal` or `twin` without clippy objecting.** Try macros, a trait in a new module, `Deref`, serde round-tripping a `Specification` into a `serde_json::Value`, and a helper in `eda` that re-exports the list. Each one that works goes in FEEDBACK with a fix or a reason it is out of scope. A JSON round trip is the honest residue: the fence binds the types, not the bytes.
2. **Find a hand-written topology that validates with a ceiling `solve` would not derive.**
3. **Find a case collision `emit` accepts,** against an emitter-owned net, rail, reference or power symbol.

## 7. Flag-and-stop conditions

- Clippy's `disallowed-methods` does not fire, as the grounds say it does, on this repository's toolchain.
- A test-produced `SolvedTopology`, a committed generated file, or a seeded scenario moves under §1.2.
- A path builds a deliberately underived ceiling.
- Making the two fields private needs a public `&mut` to either list outside the narrow writer of §1.1(b).
