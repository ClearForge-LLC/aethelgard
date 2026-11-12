# AET-WO-1205 — The declared I2C speed in `SolvedTopology`: hand-written topologies held to the floor and the limit, and the generator held to intent

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1205-declared-speed`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P1 refinement. Ruled 2026-11-12 by the architect, with Scotty's delegation, from `AET-WO-1204` §1.2's recorded gap.

**Grounds:**
- **`RUL-16`:** generators and the judge read only the `SolvedTopology`. It carries each I2C bus's ceiling (`i2c_ceilings`, the fastest speed the declared rise-time bounds allow), but not the speed the person declared (`I2cBus.speed_hz`).
- **Two consequences:**
  - `SolvedTopology::validate` cannot hold a hand-written topology to `AET-WO-1203`'s pull-up floor or capacitance limit, because both depend on the declared speed (`AET-WO-1204` FEEDBACK, "Gaps").
  - `aethelgard-svd-hal`'s generator admits every mode at or under the ceiling (`board.rs`, "The speeds at or under ... ceiling"). Firmware may therefore run a bus faster than the specification declared, as long as the bus is below its physics ceiling. Intent is not carried to the generator.
- **The reference is unaffected in substance:** SENSORS declares 400 kHz, and its 300 ns lines derive a 400 kHz ceiling.

> **What this is:** one field the solver copies, the checks it enables, and the generator honouring it.
>
> It is NOT:
> - a change to the specification schema;
> - a change to any rule's constants.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **`SolvedTopology` gains `i2c_speeds`:** a map from bus id to the bus's declared `speed_hz`, written by `solve` and keyed exactly as `i2c_ceilings` is.
   - **The solved schema version goes from 1 to 2.**
     - A version-1 file is refused by `UnsupportedSchemaVersion`, never by a missing-key error and never by a default (`INV-06`).
     - Every keyed collection follows `RUL-16`: a `BTreeMap`, with duplicate keys refused.
2. **`SolvedTopology::validate`, after `AET-WO-1204`'s ceiling check,** refuses with named errors:
   - an `i2c_speeds` key set that differs from `i2c_ceilings`'s;
   - a speed `electrical::i2c_mode` does not admit;
   - a speed above the bus's ceiling;
   - any line of the bus whose `pull_up_ohms` is below `electrical::i2c_pull_up_floor_ohms(speed, domain_millivolts)`;
   - any line whose `capacitance_femtofarads` exceeds `electrical::i2c_bus_capacitance_limit_femtofarads(speed)`.

   Reuse `solve`'s own errors where the meaning is identical (`PullUpBelowSinkFloor`, `BusCapacitanceOverLimit`), so one fact has one name. A hand-written topology and a specification then refuse alike.
3. **The board generator admits the modes at or under the bus's declared speed,** not its ceiling. `solve` already refuses a declared speed above the ceiling (`BusTooFast`), so this only narrows.
   - The generated doc comment says what it now means.
   - `boards/sentinel-01/**` may change **only** in that comment text. Regenerate it with the composition function (`aethelgard generate`, `AET-WO-3301`), never by hand, and list every changed line in FEEDBACK.
   - If any other byte changes, that is a §7 stop.
4. **Every reader of a solved topology's JSON follows the new schema:** fixtures, the CLI's test topologies (`analog.rs`, `campaign.rs`) and the twin's. Record each in FEEDBACK.
   - **Run ids and `topology_sha256` will change.** Say so in FEEDBACK, with the reference's new `topology_sha256`.
   - No committed archive or proposal exists on `main` to regenerate. If one does by the time you build, that is a §7 stop.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-06`, `RUL-16`, `RUL-19`. No `unsafe`.
- **Fence:**
  - `aethelgard-ir`'s `lib.rs` and `solve.rs`;
  - `aethelgard-svd-hal`'s `board.rs` (the admitted modes and their comment);
  - `boards/sentinel-01/**`, by regeneration only;
  - test fixtures;
  - this work order's FEEDBACK.
- **`AET-WO-1601` may run in parallel** (`~/aethelgard-1601`: the CLI's `run.rs`, `main.rs` scratch, `kicad.rs`). Never touch another worktree. If it lands first, merge `origin/main` with a merge commit.
- `cargo fmt -p <crate>`, never `--all`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **The reference solves to a version-2 topology** with `i2c_speeds` `{"SENSORS": 400000}`.
  - `aethelgard verify` on it passes with `pullup-2k2`, in the image.
  - `boards/sentinel-01/**` differs from `main` only in the comment lines listed in FEEDBACK.
- **Hand-written topologies, each refused by name:**
  - a version-1 file;
  - a speed key missing or extra;
  - 1.5 MHz;
  - a speed above the ceiling;
  - at 3.3 V and 400 kHz, a 966 Ω pull-up (967 Ω validates);
  - at 400 kHz, 401 pF (400 pF validates; at 1 MHz, 550 pF validates).
- **The generator:** a topology whose declared speed is 100 kHz under a 400 kHz ceiling admits only `Standard100k`, and its harness drives only that speed.
- **The gates** (`CLAUDE.md`) pass. So do `gates`, `tools` and `image` in CI. `TIER2_MIN` is unchanged unless you add tier-2 tests; if you do, raise it by exactly that many, with a comment.

## 5. Adversarial pass — targeted

1. **Find a hand-written topology that validates** with a pull-up below the floor or a capacitance over the limit at its declared speed: rounding, a rail at the 2 V boundary, a speed at a mode boundary, or overflow.
2. **Find a generated board crate that admits a mode above the declared speed.**
3. **Find a reader of a solved topology that accepts version 1 or defaults the new field.**

## 7. Flag-and-stop conditions

- Any committed generated byte other than the comment lines changes.
- A committed archive or proposal would need regenerating.
- A test outside the fence must weaken to pass.
