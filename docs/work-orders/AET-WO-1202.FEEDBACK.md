# AET-WO-1202 — FEEDBACK

**Branch:** `wo/aet-1202-bus-ceiling` · **Base:** `3f3924e` (`AET-WO-1200` merged, #32) · **Work order:** `~/.cache/aet-1202-wo.md` (sha256 `9ac2a8a8…`), from the docs branch of PR #43 (not merged). This PR does not carry the work order, so as not to collide with #43 · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, the 1200 session · **Result:** **§7 STOP at §1.3, before any build.** Checked-in examples and solve fixtures have I2C buses without a constraint on one or both lines. Nothing in §1.1 or §1.2 was built, and no constraint was added.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §7 stop: I2C buses with no constraint on a line (§1.3)

Measured with a script over every `examples/*.json` and `crates/aethelgard-cli/tests/fixtures/solve/*.json`. It lists each I2C bus whose `sda.net` or `scl.net` has no `net_constraints` entry.

| File | I2C buses | Lines with no constraint |
|---|---|---|
| `examples/sentinel-01-minimal.json` (the reference) | 1 | `SENSORS.scl` (`I2C_SCL`). `I2C_SDA` is constrained |
| `examples/invalid-zero-domain.json` | 1 | `SENSORS.scl` (`I2C_SCL`). Validation refuses this file before any ceiling (`ZeroNominalVoltage`), so its behaviour would not change |
| `crates/aethelgard-cli/tests/fixtures/solve/stress.json` | 4 | every `sda` and `scl` (8 lines). `net_constraints` is empty |
| `crates/aethelgard-cli/tests/fixtures/solve/infeasible-shared.json` | 4 | every `sda` and `scl` (8 lines) |
| `crates/aethelgard-cli/tests/fixtures/solve/thrash-pins.json` | 4 | every `sda` and `scl` (8 lines) |
| `crates/aethelgard-cli/tests/fixtures/solve/pigeonhole-spi.json` | 0 | none |

Under §1.2, every file above except `pigeonhole-spi.json` would refuse `MissingBusConstraint` before the pin search. That includes the P1 gate's clause-2 and clause-4 inputs (the reference, and `stress.json`).

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S1 | The reference and three of four solve fixtures leave I2C lines unconstrained | above | §7 stop | The architect rules the constraint values, or rules these files out of §1.2's reach | **yes** |
| S2 | **Constraining SCL like SDA does not make the reference pass.** At 2,200 Ω and 400,000 fF (400 pF), `t_r = ceil(8473 × 2200 × 400000 / 10⁷)` = **745,624 ps (745.6 ns)**, so the ceiling is **100 kHz**. The bus declares `speed_hz` 400,000, so §1.2 would refuse `BusTooFast`. The reference's own constraint says `max_rise_time_nanoseconds` 300, which these values cannot meet. To keep 400 kHz at 2,200 Ω, `C_b` must be at most **160,939 fF**; at 400 pF, `R_p` must be at most **885 Ω** (both computed with the WO's integer formula) | `examples/sentinel-01-minimal.json` | design input | Rule one of: lower the reference's capacitance, lower its pull-up, or lower its speed to 100 kHz. Each changes the reference's bytes, so the 1200 digests change too | **yes** |
| S3 | The spike-derived fixtures (`stress`, `infeasible-shared`, `thrash-pins`) are byte-for-byte copies whose digests the 1200 tests pin, as is the `AET-WO-1200.repro/` pair behind 1200 FEEDBACK A1 (`fuzz-natural-6spi` has 2 I2C buses and `core-infeasible` 4, none constrained). Adding constraints changes their bytes. Leaving them as they are makes them refuse before the search, so they no longer exercise it, and A1's reproducer stops reproducing | CLI fixtures, `docs/work-orders/AET-WO-1200.repro/` | consequence | If constraints are ruled in, record the old and new digests (as §1.4 already asks for the output change), and re-measure A1 on the new bytes | yes |

## For the build, once ruled (found while checking, nothing built)

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| P1 | **`rise_time_ns` can exceed `u64`.** `u128` holds every product: 8473 × `u32::MAX` × `u64::MAX` ≈ 6.7 × 10³², against `u128::MAX` ≈ 3.4 × 10³⁸. But the rise time that `RiseTimeTooSlow` reports can reach about 6.7 × 10²² ns, above `u64::MAX` ≈ 1.8 × 10¹⁹. The WO does not name that field's type, nor how picoseconds round to the reported nanoseconds | §1.2 `RiseTimeTooSlow` | gap | Type it `u128`, or saturate; report `ceil(t_r_ps / 1000)` | yes |
| P2 | Each net's own `max_rise_time_nanoseconds` is never compared with the computed `t_r`. The reference states 300 ns and computes 745.6 ns, and §1.2 as written would refuse only on `speed_hz` | §1.2 | design question | Rule whether a computed `t_r` above a line's stated maximum also refuses | yes |
| P3 | The exact boundaries are reachable from integer inputs for 120 ns (`R_p` 1,000 Ω, `C_b` 141,626 fF) and 300 ns (2,500 Ω, 141,626 fF); my small search found none for 1,000 ns. §1.4's boundary tests can call the mode choice on `t_r_ps` directly, so this does not block them | §1.4 | note | Test the mode choice on picoseconds, and the formula separately | no |
| P4 | `aethelgard-ir/tests/solve.rs` and `validate.rs` build I2C buses with no `net_constraints`. They would need constraints, which is inside the fence | ir tests | note | Add them in the build | no |

## Gates

- No code was changed. The only file in the diff is this FEEDBACK. `cargo fmt`, `clippy` and `test` are unchanged from `3f3924e`, where CI ran on #32's merge.
- **Adversarial pass (targeted, §5):** not run on code, since nothing was built. Its three targets were checked against the formula as the WO states it:
  - (1) rounding: `ceil` in exact integer arithmetic, with boundaries compared in picoseconds, gives no wrong-way rounding (P3);
  - (2) overflow: none in `u128`, but the reported nanoseconds overflow `u64` (P1);
  - (3) a ceiling with no constraint behind it: §1.2's `MissingBusConstraint` forbids it.

  The pass on built code is owed after the ruling.
- **Reduction pass (diff):** nothing to remove (no code in the diff).
