# AET-WO-2203 — The I2C devices move into `SolvedTopology` (`RUL-16`; `AET-WO-2202` W2)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2203-solved-devices`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P2, housekeeping. This carries forward `AET-WO-2202` W2, ruled on PR #69.

**Grounds:**
- **`RUL-16`:** generators read a `SolvedTopology`, never a `Specification`. Every fact a generator needs is derived once, by the solver.
- **`AET-WO-2202` derived the I2C devices outside the solver,** through `aethelgard_ir::parts::i2c_targets(&Specification, &SolvedTopology)`, because `AET-WO-2200` owned `SolvedTopology` at the time.

> **What this is:** `SolvedTopology` gains the devices, and every consumer of `i2c_targets` moves to them.
>
> It is NOT:
> - a change to any emitted byte, verdict or behaviour;
> - a change to the part identity table.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **`SolvedTopology` gains `i2c_devices: BTreeMap<reference, SolvedI2cDevice { part, bus, peripheral, address }>`.**
   - `solve` fills it, keeping `i2c_targets`'s refusals, `DeviceNotPlaced` included, and its order (the specification's order, kept by an explicit index field if a map would reorder).
   - `validate` refuses:
     - an address that is not 7-bit;
     - a bus that is not in `assignments`;
     - a key that is not an identifier.
   - The golden digests move. FEEDBACK records the old and new values.
2. **`parts::i2c_targets` is deleted.** The platform emitter, the firmware generator's caller and the boot judge read `topology.i2c_devices`, and nothing else.
3. **No consumer outside `aethelgard-ir::solve` takes a `&Specification` to learn about devices.** A test asserts it, by grep over the crates' sources.
4. **Every emitted file stays byte-identical.** That covers `boards/sentinel-01/**` and the `.repl`. Any difference is a §7 stop.

## 2. Invariants and fences

- `RUL-16`, `INV-01`, `INV-06`. No `unsafe`.
- **Fence:**
  - `aethelgard-ir`: `SolvedTopology`, `validate`, `solve` and `parts`;
  - the call sites in `aethelgard-twin`, `aethelgard-svd-hal` and the CLI that used `i2c_targets`;
  - tests;
  - this work order's FEEDBACK.
- **`AET-WO-2300` may run in parallel** in `aethelgard-chaos` and the CLI's `campaign` and `replay`. Do not touch those. If it lands first, merge `main`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- The regeneration guard passes with no change to any committed generated file.
- `i2c_targets` no longer exists.
- The grep test passes.
- §1.4 passes in `gates` and `tools`.

## 7. Flag-and-stop conditions

- Any emitted byte changes.
- A change outside the fence.
- A test that can only go green by being weakened.
