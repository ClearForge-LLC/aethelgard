# AET-WO-1202 — The solver derives each I2C bus's speed ceiling from its declared rise time

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` at or after `3f3924e` (`AET-WO-1200` merged), named in the kickoff.
**Branch:** `wo/aet-1202-bus-ceiling`.
**Author:** Claude (architect) · **Builder:** a strong builder with a targeted adversarial pass (`docs/roadmap.md` §6). It is the 1200 session, which knows the solver.
**Phase:** P1, inserted at the ruling on `AET-WO-1400`'s build question (PR #42). **Gate clause served:** clause 3's "a bus speed above the solved rise-time ceiling". `AET-WO-1400` emits the typestate. This work order supplies the ceiling it is built from.

**Grounds:**
- **`RUL-16`.** The board-crate generator reads a `SolvedTopology`, never a `Specification`. The ceiling is a fact the solver derives from the specification, so it belongs in `SolvedTopology`. `AET-WO-1400`'s builder asked, and option 2 was ruled.
- **The mode limits, from NXP UM10204, the I²C-bus specification:** the maximum rise time is 1000 ns in Standard-mode (100 kHz), 300 ns in Fast-mode (400 kHz) and 120 ns in Fast-mode Plus (1 MHz).
- **Amended 2026-11-08, at the §1.3 stop on PR #44.** The first draft derived the ceiling from `t_r = 0.8473 × R_p × C_b`, using the declared maximum capacitance. That refuses at P1 the very specification the P2 gate needs to run: `roadmap.md` P2 changes the pull-up to 47 kΩ and expects the *twin* to measure the rise time over the solved ceiling.
  - So the ceiling is now the board's **declared** promise, its `max_rise_time_nanoseconds`.
  - The pull-up and capacitance physics belongs to the twin, against extracted parasitics (`AET-WO-1302`, `2200`).
  - The draft's `u128` arithmetic and its overflow finding (P1 on PR #44) go with it.
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
   - **`t_max`** is the smaller `max_rise_time_nanoseconds` of the `net_constraints` entries whose `net_name` is the bus's `sda.net` and `scl.net`.
     - If either line has no constraint, refuse `MissingBusConstraint { bus, net }`, naming the first missing net in document order.
     - Do not default.
   - **The ceiling** is the fastest mode whose limit is at least `t_max`: 1 MHz if `t_max ≤ 120`, 400 kHz if `≤ 300`, 100 kHz if `≤ 1000`.
   - **If `t_max > 1000`,** refuse `RiseTimeTooSlow { bus, rise_time_ns: t_max }`, where `rise_time_ns` is a `u32`.
   - **If the bus's `speed_hz` is above its ceiling,** refuse `BusTooFast { bus, speed_hz, ceiling_hz }`.
   - **Order.** These checks run after `validate` and before the pin search.
   - **Kinds and exit code.** The three refusals use the existing refusal path and exit 2, with `kind` strings matching the variant names.
   - `pull_up_ohms` and `max_capacitance_femtofarads` are not read here.
3. **The examples and fixtures, as ruled on PR #44.**
   - **`sentinel-01-minimal.json`** gains an `I2C_SCL` constraint equal to `I2C_SDA`'s.
   - **Both lines' `max_capacitance_femtofarads` drop to 100,000 (100 pF).** As declared, 2.2 kΩ and 400 pF cannot meet the bus's own 300 ns, since 0.8473 × 2200 Ω × 400 pF = 746 ns. At 100 pF it is 186 ns, so the reference is self-consistent at 400 kHz. The solver does not check this; the twin will.
   - **The three CLI solve fixtures** get constraints on every I2C line: 100 pF and 300 ns, unless a fixture's purpose needs otherwise.
   - **`invalid-zero-domain.json`** is unchanged, because validation refuses it first.
   - **Every test that pins a specification's sha256 or a solve digest** is updated, with the old and new values in FEEDBACK.
4. **Tests:**
   - a unit test of the mapping at each boundary: exactly 120, 300 and 1000 ns, and 1 ns over each;
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

1. Make the mapping off by one at a boundary.
2. Get a ceiling with no constraint behind it, or from only one of the two lines.
3. Make the stricter of the two lines lose to the looser one.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- A change outside the fence.
- A test that can only go green by being weakened.
