# AET-WO-1202 — FEEDBACK

**Branch:** `wo/aet-1202-bus-ceiling` · **Base:** `3f3924e`, then merged with `origin/main` `ab8b17d` (the amended work order, #45) · **Work order:** `docs/work-orders/AET-WO-1202.md` as amended at `437c6dd`. It is identical to what was built from (`~/.cache/aet-1202-wo.md`, sha256 `e4061555…`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, the 1200 session · **Result:** stopped at §1.3 (first push, below); ruled on PR #44 on 2026-11-08; §1 built under the amended work order. Every §1.4 test passes on Furnace; CI is in the Gates section.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## F1 — the ruled two-line rule overstates the ceiling (decision-needed: **yes**)

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **F1** | **"`t_max` is the smaller of the two lines" gives a ceiling the board never promised whenever the lines differ.** Take SDA declared ≤ 120 ns and SCL declared ≤ 1000 ns: the ruled rule gives 1 MHz. Fast-mode Plus needs every line to rise within 120 ns, and SCL is only promised 1000 ns. A bus is as fast as its *slower* line, so the safe ceiling is the fastest mode whose limit is at least the **larger** declared value: here, 100 kHz. §5.3's framing ("the stricter line must win") equates a stricter *line* constraint with a stricter *ceiling*, but they point opposite ways. A tighter bound on one line relaxes nothing about the bus. `AET-WO-1400` will build firmware typestate from this ceiling. **Built as ruled.** `the_stricter_line_sets_the_ceiling_whichever_it_is` pins the ruled behaviour, with a comment naming F1. | `solve.rs` `i2c_ceilings` | wrong rule (as ruled) | **(a)** Take the larger: `t_max.min(…)` becomes `.max(…)`, with `t_max` starting at 0, and that test's four assertions invert. The mutant doing exactly this is killed today, so the flip is guarded both ways. Or **(b)** refuse when the two lines declare different rise times, which avoids choosing. The reference and every fixture declare equal values, so neither option changes any golden digest | **yes** |

## Build

### What was built

- **`aethelgard-ir`:**
  - `SolvedTopology.i2c_ceilings: BTreeMap<String, u32>`, with the same duplicate-key refusal (`unique_keys`). `validate` adds the `i2c_ceilings.<bus>` identifier paths and `InvalidCeiling { id }` (any value but 100,000, 400,000 or 1,000,000), checked after alternate functions and before `NetAssignedTwice`.
  - `solve.rs`: `ceiling_hz`, the UM10204 mapping (≤ 120 ns gives 1 MHz, ≤ 300 gives 400 kHz, ≤ 1000 gives 100 kHz), and `i2c_ceilings`, run right after validation and the hash check, before the part check and the pin search.
  - `SolveError` gains `MissingBusConstraint { bus, net }`, `RiseTimeTooSlow { bus, rise_time_ns: u32 }` and `BusTooFast { bus, speed_hz, ceiling_hz }`. For each, `id()` is the bus and `claimants()` is `[bus]`. Only the message names the missing net (C2).
  - `pull_up_ohms` and `max_capacitance_femtofarads` are not read.
- **Both READMEs:** the field, the check order, the ceiling section and refusals rows 1a, and the reference's new constraints.
- **Examples and fixtures (§1.3, as ruled):**
  - `sentinel-01-minimal.json` gains an `I2C_SCL` constraint equal to `I2C_SDA`'s, and both drop to 100,000 fF.
  - The three CLI solve fixtures get a 100 pF / 300 ns constraint on every I2C line (8 each). No fixture's purpose needed otherwise: the three adversarial ones still refuse `Infeasible` with their 1200 refusal bytes unchanged.
  - `invalid-zero-domain.json` is unchanged.
- **Tests:**
  - the mapping at 120/121, 300/301 and 1000/1001 ns: a unit test in `solve.rs`, and again through `solve`;
  - each refusal at the library and at the CLI (`each_ceiling_refusal_is_exit_2_naming_the_bus`: exit 2, `kind`, `id`, `claimants`, message);
  - the reference ceiling stated as `{"SENSORS": 400000}`;
  - two solves byte-identical (the 1200 clause-4 test, on the new digests);
  - the stricter line in either order;
  - each line missing alone;
  - the check order against an infeasible specification and an invalid one;
  - pull-up and capacitance not read;
  - `InvalidCeiling` at every non-mode value, and its order.

### Values that moved (§1.3, §1.4)

| What | Old | New |
|---|---|---|
| `examples/sentinel-01-minimal.json` sha256 | `9a42e84f7d2dab3127ee3d9a4ff8e87accdd6320902291370a520294e4e263d2` | `acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1` |
| `fixtures/solve/stress.json` sha256 | `0dbe67019555203d4afd994a2a9b9f958f73a456a502ea614581f12314923583` | `6d40861470aaf40cbbaa49e6cd136f4800aa2ef11ba542ff496d1b21f1168f3a` |
| `fixtures/solve/infeasible-shared.json` sha256 | `2f2c045b1a4273b4501f0f14f12b44ae8aa7afed515c989a07cd3ecbc6dada10` | `b2a9ffce33f973e633813134ab189c8aeeeae8860f72c7fade57fb43e9330a32` |
| `fixtures/solve/thrash-pins.json` sha256 | `d1811fa80caa1d989a67a9193292781aa8920a884c1245f7177e6e7731e345ba` | `60050e51705e543bb12bc0b53c2ae3def0cf14e1b270d042e34097c2407b4f4e` |
| Reference solve output (`REFERENCE_DIGEST`) | `c7f59a497e8ed989e83491a3a55af2a86a076f06470c8ac13c64e987e561f0cd` | `9e5b38ff615c734c23c7664486ee6d42476e4fdfd77be5243be4fc40a7991ec9` |
| Stress solve output (`STRESS_DIGEST`) | `759f41463aaf205dec2eed472c0929e38a300f749cfe51ff49f3937c23e4b578` | `77fd96e2d59875ae631b26b2a55085b099779bc62ce7c206653f0eafbb828310` |
| `validate` on the reference: `net_constraints` | 1 | 2 |

The fixture rewrites were made by a load-and-dump that first reproduced each file's old bytes exactly, so the diff is the constraints alone. The reference was edited as text. The pin assignments in both success outputs are unchanged; only `specification_sha256` and the new `i2c_ceilings` differ.

### Changes outside the fence, and other notes

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| C1 | **`crates/aethelgard-cli/tests/validate_cli.rs`:** `valid_example_exits_0` expects `net_constraints` 2, not 1. The test is outside §2's fence (only the CLI's *solve* tests are named), but it reads the reference, and the ruled §1.3 change to the reference moves this count. Nothing about what it checks is weakened. `examples/sentinel-01-minimal.json` is itself outside the listed fence and changed by ruling | `validate_cli.rs:54` | change outside the fence (consequence of a ruling) | Ratify. A one-line revert if not, but then the ruled example change cannot land | yes |
| C2 | `MissingBusConstraint`'s `net` reaches a caller only in `message`; `id` and `claimants` carry the bus, as for every bus-level refusal. A caller fixing the specification has to read the message | `solve.rs` | contract choice | Accept, or put the net in `claimants` | yes |
| C3 | The `AET-WO-1200.repro/` specifications (1200 FEEDBACK A1) have unconstrained I2C buses, so they now refuse `MissingBusConstraint` before the search and no longer reproduce A1. They are outside this fence and were left alone | `docs/work-orders/AET-WO-1200.repro/` | consequence | Add 300 ns constraints to them in whichever work order takes up A1 | no |
| C4 | `SolvedTopology::validate` cannot check that `i2c_ceilings`' keys are the specification's I2C buses: it never sees the specification. A hand-written topology could carry a ceiling for a bus that does not exist. `solve` emits exactly one per bus (tested) | `lib.rs` | limit of the type | None; `RUL-16` keeps solver output solver-made | no |
| C5 | The "check first" pass at the stop also covered P1 (`u128` overflow) and P2 (the computed `t_r` never compared with the declared maximum). The ruling retired both by dropping the R × C formula. P3 and P4 were done: the boundary tests call the mapping directly, and the ir tests' synthetic buses now carry constraints | — | closed by ruling | none | no |

## Adversarial pass (§5, targeted)

Run by the builder. Each of the three targets was attacked by mutation, a test, and an independent model.

| # | Target | Result |
|---|---|---|
| A1 | **The mapping off by one at a boundary.** Each boundary moved by one (120→119, 300→301, 1000→1001) | killed by the mapping unit test (and the CLI test for 1001) |
| | An independent model over the grid {1, 119, 120, 121, 299, 300, 301, 999, 1000, 1001, `u32::MAX`}² × {100 k, 400 k, 1 M}, every case through the real `solve` on the reference (a scratch harness, not committed) | **363 cases, 0 mismatches** |
| A2 | **A ceiling with no constraint behind it, or from one line only.** Mutants reading only SDA, reading SCL first, and swallowing the ceiling refusals | all killed |
| | Validation already refuses a duplicate `net_name`, so no line can have two constraints, and it refuses a signal net shadowing a domain, so no line can borrow a domain's constraint. A `SolvedTopology` ceiling with no bus is C4 | — |
| A3 | **The stricter line losing to the looser.** The `min`→`max` mutant | killed by `the_stricter_line_sets_the_ceiling_whichever_it_is` and `a_rise_time_slower_than_every_mode_is_refused` |
| | The ruled rule itself is the subject of **F1** | — |

No `HashMap`, no `unsafe`, and no clock in the new code. The ceiling is a pure function of the specification, collected into a `BTreeMap`.

## Gates

- **Furnace**, at the merge with `ab8b17d`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass, **359 passed, 0 failed**. The CI example step (`validate` on the reference) exits 0 with `net_constraints` 2. Diffing against `origin/main`, the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `container/`, `.github/` and every crate but `aethelgard-ir` and `aethelgard-cli` all come out empty.
- **Mutation** (one mutant per new guard, against `aethelgard-ir`'s suites and `solve_cli.rs`): **13 of 13 killed**:
  - each boundary moved by one (3);
  - `min`→`max`;
  - only SDA read;
  - SCL read first;
  - `>` → `>=` in `BusTooFast`;
  - `BusTooFast` off;
  - ceiling refusals swallowed;
  - the ceiling pass moved after the search;
  - `InvalidCeiling` off;
  - the ceiling identifier check off;
  - duplicate ceiling keys kept.
- **Reduction pass (diff):** nothing to remove · 0 duplicates removed · 0 dead paths · 0 protected · 1 finding: R1, below.
  - Run by the builder over the diff (about 150 lines of code), not by a fresh subagent.
  - **R1:** the three I2C mode speeds are now listed in `SolvedTopology::validate` and in `ceiling_hz`, and already were in `Specification::validate` (`UnsupportedI2cSpeed`). One constant would hold them, but `Specification::validate` is outside this work order's fence.
- **CI**, first push of the build (`c760621`): `gates` and `tools` both green, https://github.com/ClearForge-LLC/aethelgard/actions/runs/42429273875. `gates` ran every §1.4 test in its debug profile. The engine-image workflow (`AET-WO-1501`) also ran on that push: `image` green, `publish` skipped (main only). The stop's first push (`ac9660d`) was green too.

## The §7 stop that preceded the ruling (first push, `ac9660d`)

§1.3 of the first draft found the reference (`I2C_SCL`) and three of the four solve fixtures (every line) without line constraints, and the stop was recorded here. Before ruling, the first draft's R × C formula was worked against the reference: 2.2 kΩ and 400 pF gave 745.6 ns against the bus's own declared 300 ns. The architect ruled on PR #44 on 2026-11-08: the ceiling comes from the declared rise time, the reference drops to 100 pF and gains its SCL constraint, and the fixtures are constrained at 100 pF / 300 ns. The amended work order is `437c6dd`.
