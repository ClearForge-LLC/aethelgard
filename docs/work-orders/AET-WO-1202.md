# AET-WO-1202 — The solver derives each I2C bus's speed ceiling from its rise time

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` at or after `3f3924e` (`AET-WO-1200` merged), named in the kickoff.
**Branch:** `wo/aet-1202-bus-ceiling`.
**Author:** Claude (architect) · **Builder:** a strong builder with a targeted adversarial pass (`docs/roadmap.md` §6). It is the 1200 session, which knows the solver.
**Phase:** P1, inserted at the ruling on `AET-WO-1400`'s build question (PR #42). **Gate clause served:** clause 3's "a bus speed above the solved rise-time ceiling". `AET-WO-1400` emits the typestate. This work order supplies the ceiling it is built from.

**Grounds:**
- **`RUL-16`.** The board-crate generator reads a `SolvedTopology`, never a `Specification`. The ceiling is a fact the solver derives from the specification, so it belongs in `SolvedTopology`. `AET-WO-1400`'s builder asked, and option 2 was ruled.
- **The physics, from NXP UM10204, the I²C-bus specification.**
  - The 30 %→70 % rise time of an open-drain line is `t_r = 0.8473 × R_p × C_b`.
  - The maximum rise time is 1000 ns in Standard-mode (100 kHz), 300 ns in Fast-mode (400 kHz) and 120 ns in Fast-mode Plus (1 MHz).
- **`INV-06`:** no default is substituted for a missing input.

## 1. Scope

1. **`SolvedTopology` gains `i2c_ceilings: BTreeMap<String, u32>`,** keyed by I2C bus id, whose value is the bus's maximum speed in Hz.
   - It is always present, empty when there are no I2C buses.
   - It uses the same duplicate-key refusal (`unique_keys`) as the other maps.
   - `validate` refuses:
     - a key that is not an identifier;
     - a value other than 100_000, 400_000 or 1_000_000 (`InvalidCeiling { id }`).
   - `schema_version` stays 1: nothing persisted outside this repository's tests carries a `SolvedTopology` yet.
2. **`solve` computes each ceiling.** For each I2C bus:
   - **`C_b`** is the larger `max_capacitance_femtofarads` of the `net_constraints` entries whose `net_name` is the bus's `sda.net` and `scl.net`.
     - If either net has no constraint, refuse `MissingBusConstraint { bus, net }`, naming the first missing net in document order.
     - Do not default.
   - **The worst-case rise time** is `t_r_ps = ceil(8473 × pull_up_ohms × C_b_fF / 10_000_000)`, in picoseconds. Use integer arithmetic in `u128`.
   - **The ceiling** is the fastest mode whose limit is at least `t_r`: 1 MHz if `t_r ≤ 120 ns`, 400 kHz if `≤ 300 ns`, 100 kHz if `≤ 1000 ns`.
   - **If `t_r > 1000 ns`,** refuse `RiseTimeTooSlow { bus, rise_time_ns }`.
   - **If the bus's `speed_hz` is above its ceiling,** refuse `BusTooFast { bus, speed_hz, ceiling_hz }`.
   - **Order.** These checks run after `validate` and before the pin search. A spec that cannot meet its own bus timing is refused before any search.
   - **Kinds and exit code.** The three refusals use the existing refusal path and exit 2, with `kind` strings matching the variant names.
3. **Check the checked-in examples and fixtures first.** If any I2C bus in `examples/` or the solve fixtures lacks a constraint on either line, list them in FEEDBACK and STOP for a ruling. Do not add constraints to make them pass.
4. **Tests:**
   - a unit test of the formula at each boundary (exactly 120, 300 and 1000 ns, and 1 ps over each);
   - one test for each refusal, at the library and at the CLI;
   - the reference specification's ceiling, stated;
   - two solves byte-identical.

   The 1200 golden digests change because the output gains a field. Update them and record the old and new digests in FEEDBACK.

## 2. Invariants and fences

- `INV-01`, `INV-06`, `RUL-16`. No `unsafe`. No `HashMap`.
- **Fence:** `crates/aethelgard-ir` (the `SolvedTopology` type, its `validate`, and `solve.rs`), the CLI's solve tests, fixtures and README rows, and this work order's FEEDBACK. Protected surfaces diff to empty, as in every work order.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

§1.4 passes in `gates` and `tools`. CI is green on the first push.

## 5. Adversarial pass — targeted

1. Make the formula round the wrong way at a boundary.
2. Overflow it with the largest `u32` and `u64` inputs.
3. Get a ceiling with no constraint behind it.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- An example or fixture without bus constraints (§1.3).
- A change outside the fence.
- A test that can only go green by being weakened.
