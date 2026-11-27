# AET-WO-4310 FEEDBACK — P4 slice D: power, package and coverage honesty

Builder: the 4310 builder (a fresh strong-tier session), 2026-11-27. Base `adbd5c66`; `main` `d4c68028` merged (AET-WO-1802's 0.3.0, D8), and `INV-08` re-run against its build.

**The one-paragraph status.**
- (2)–(6) are built to the rulings of 06:58Z, 07:42Z, 07:49Z, 08:22Z and 08:55Z.
- **Sentinel-03 is refused twice, as ruled, until Scotty rules on its board:**
  - readiness refuses U30 and U31.B, on `VDD_3V3_RF`, which no monitor holds;
  - `extract` refuses both drop limits, `VDD_3V3` at most 271.3 mV against 110 and `VDD_3V3_RF` at most 240.8 against 144, each an upper bound.
- So `bench-package-s03` cannot pass, and the merge waits on Scotty's two answers (08:55Z).
- Every other board, and Sentinel-01-bench, goes as ruled. Its package moves by its scenario list, FAB-NOTES' two built lines and the droop's hash. The ten files `ORDERED.sha256` binds do not move.

## 1. Part (1): the measurement and its stops

- `spike/4310/PREDICTIONS.md` was committed (`141a7082`) before any probe ran, and is scored in `RESULTS.md`.
- `STOP.md` holds the table by board and by part, posted to the bus at 06:54Z.
- **Two predictions were wrong, both found by reading at source:**
  - **P2.4:** the MCU's 1620 mV is not its floor. DS12117 Table 111 note 1 says "functionality guaranteed down to VBOR0 min".
  - **P2.3:** U31's minimum is held. I first read the timing row's 3.0 V band; the architect ruled the supply minimum is SCES935A p. 6 §7.3's 1.08 V.
- **Stops after (1), each ruled before its part was built:**
  - 07:01Z, `LineLevelUnmet` at the release floor: (a).
  - 07:01Z, does naming U30 block the write: it blocks readiness.
  - 07:45Z, D4's consequence: option 3.
  - 08:31Z, the specification version: (b).
  - 08:53Z, Sentinel-03's ground limits under D5: accepted, and the figures go to Scotty.

## 2. What was built

| Part | Commit | What |
|---|---|---|
| (2) | `7c51b776`, `7f94b3e7`, `50f4e353` | `supervisor::powered` and `judged`: every powered part against its own rail's monitor (D1 (B)). Covers the MCU at VBOR0's minimum with Table 111 note 1 (D2), each I2C, SPI and radio device, the supervisor, each translator side at SCES935A §7.3's 1.08 V (D3 (ii)), each rail-fed regulator's VIN, and each pin device's supply pin by its row's range (adversarial F1). Battery-fed regulators are named "judged by the headroom". A part on a rail no monitor holds, or with no held minimum, is named, and readiness refuses it (ruled 3). An SPI crossing's timing stated only above the reset is named (08:22Z (1)). A UART crossing's MCU-side lines are judged again at the reset's release floor, the 2.3–3.0 V band taking the worse of SCES935A's two rows, named as an assumption (2 (a)). power-parts schema 14 |
| (3b) | `566230d1` | The pack's `series_milliohms` in the headroom, the FET's gate drive and the input current's one iteration; undeclared, it is named (`headroom_pack_series_undeclared`) |
| (3a) | `20659b30` | Each droop's R and load are `{value, source}`, the source `held` or `stimulus`, named on the campaign line and in `verify`'s run. Scenario schema 7; `DroopSourceUndeclared`; `DroopLoadUnderTopology` |
| (3c) | `3e3e366b` | Every other current in the ground copper (each other regulated domain's load at its consumers' ground pads, each other battery-fed buck's input current at its PGND) charged at I · min(R₁₁, the largest entry bound), by the maximum principle, using certified upper bounds only. Entry pads are solved farthest first, six at a time, stopping at R₁₁ |
| (4) | `3e3e366b`, `50f4e353` | `emit` records a laddered board's rung and pour step in its title block. `extract`'s reader admits that block and nothing else in it, reports `capacitance_ladder` (or `not_run`) and `outline_mm`, and refuses a missing, malformed or unexpected record by name. `verify` carries both into the extract gate's `built`, and FAB-NOTES gains two lines (`- Outline as built:`, `- Capacitance ladder:`). A package is never written without both (F9). `rebind` admits those two keys only where the committed notes lack them (F10 (b)), and admits scenario entries the package lacks. `--against-fab` refuses a package whose FAB-NOTES states neither (`PackageBuiltUnstated`) |
| (5) | `215c7a53` | (i) A raised pull-up or line capacitance covers nothing: no tolerance is declared, so it needs a detected fault. (ii) A bus of unmodelled devices alone is named uncovered; a device-less bus stays `no_device`. (iii) Each motor output is a target. The override `motor_output_open` (record kind 10, scenario 7) leaves the output's observer off its pin, and a new rule E4 attributes the silence electrical. Sentinel-01-bench gains `held-low-sda`; Sentinel-03 gains `motor-open-motor1`–`4` |
| (6) | `2035c735` | Reduction pass (§7) |

## 3. Acceptance

- **Each rule refuses its planted board or scenario by name and passes its clean twin:**
  - `crates/aethelgard-ir/tests/supply_floors_4310.rs` (12 tests) and `supervisor.rs`'s SPI-timing unit test, `supply_bounds_4310.rs` (4);
  - `crates/aethelgard-eda/src/supply.rs`'s `charge` tests (3);
  - `crates/aethelgard-cli/tests/package_built_4310_tier1.rs` (3);
  - `verify.rs`'s (5) tests (4), `rebind.rs`'s tests (3);
  - `crates/aethelgard-evolve/tests/motor.rs`'s E4 tests (2), `crates/aethelgard-twin/tests/motor_platform.rs`'s open-observer test, `crates/aethelgard-chaos/tests/droop.rs`'s source and override tests (4).
- **Mutants, one per guard, all killed but one equivalent:**
  - (2) 20/20, (3b) 4/4, (3a) 8/8, (3c) 5/5 (re-run after the reduction), (4) 10/10, (5) 12/12, adversarial 3/3 (PA-F1, PA-F9, PA-F10).
  - P5-M12 is equivalent: a failing timing check is decided first as I4. It is labelled `belt-and-suspenders:` at the site.
  - Kit and results: `spike/4310/mutants/`.
- **Committed boards** (`spike/4310/out/inv08-head.txt`):
  - All 43 committed JSON files were emitted and solved by both builds. 27 draw and 16 are refused, as on `main`, and every refusal keeps its kind.
  - Every schematic and BOM is byte-identical. Every board is byte-identical but the four laddered ones, each by its three-line title block (D6).
  - `solve` is byte-identical but the three power trees, as ruled (3b). Every extract verdict gains `capacitance_ladder` and `outline_mm` (D6).
- **Measured on the committed ELFs in the tools image:**
  - Sentinel-03's two rescaled droops (67 mΩ stimulus at the held 750 mA, the same 50.25 mV drop) pass and match their `expect_counts`.
  - `motor-open-motor1`–`4` each fail by `motor_output_unwitnessed` alone, replay (`INV-04`), and attribute `electrical` E4 with subject `output:MOTORn`.
  - Sentinel-01-bench's `held-low-sda` fails as expected and attributes E1.
  - Evidence: `spike/4310/out/s03-droop-campaign.txt`, `motor-open-campaign.txt`.
- **The packages** (the architect's write dispatch re-derives them):
  - Sentinel-01-bench moves by its scenario list (`held-low-sda` added, the droop's new hash), FAB-NOTES' two lines and its verdict.
  - Sentinel-03 cannot be written until Scotty rules (§8).
  - Until the dispatch, `bench_board_tier1`'s committed-package check and both `--against-fab` jobs fail by design: the package on disk predates these rules.

## 4. Rulings applied

- **D1 (B), D2, D3 split (U31 (ii), U30 named), D4 option 3, D5 (maximum principle, Sentinel-03 stops), D6, D7 (i)–(iii), D8:** as ruled, see §2.
- **07:42Z:** the SCES935A copy is held at `~/.cache/aet-held/ti-txu0304-sces935a.pdf` (sha256 `425338d9…550f`, `SOURCE.txt` beside it).
- **08:22Z:**
  - (1) SPI timing named;
  - (2) rebind admits, by key;
  - (3) the workflow lists and `bench_board_tier1`'s `SCENARIOS` only;
  - (4) U31's bench row under the existing id, with B-9 gaining VCCI 2.87 V;
  - (5) power-parts 14, scenario 7, record kind 10.
- **08:55Z (1): the specification version is not bumped.** The release-floor `LineLevelUnmet` is a `validate` refusal added without a bump, under `1215` D4's own clause ("or the work order says why not"):
  - no field's meaning changed;
  - it refuses no committed board;
  - it reaches only boards with a UART crossing, today Sentinel-03 alone, which was already wrong at that floor under 31.
- **One deviation, named:** the lower band's VT+ is 679 ‰, not the 678 the ruling wrote. 1.56 V / 2.3 V = 678.26 ‰, rounded toward refusal, as the stated rows' 296 ‰ is rounded down.

## 5. Adversarial pass (full; `spike/4310/ADVERSARIAL.md`)

- **Fixed:**
  - F1: a pin device's supply pin was named as holding no minimum; its row holds a range.
  - F9: the writer accepted a missing `built`. A passing run's package is now refused without both lines (`PackageBuiltUnstated`, in `package`, after the verdict, so a failing run stays the verdict's under `2308` LP2).
  - F10 (b): rebind stripped the built lines from both sides forever.
- **For the architect:** F7. An ADC input is covered by any `analog_value` that differs from its idle, even 1 mV, under `1211c`'s ruled (a). That is the shape D7 (i) forbids for I2C. It is latent: no committed scenario uses `analog_value`.
- **Named, not fixed:**
  - **F3, `spi_timing`:** it takes the lowest non-UART row, not the row the crossing reads, and compares at the reset's source, not less the drop. It is latent: TXU0304 has one such row.
  - **F4, (3c):**
    - (a) a buck fed from a rail charges its input current at that rail's other consumers' pads (latent);
    - (b) a missing domain load is zeroed (unreachable);
    - (c) a consumer with no ground pad is skipped while its current stays in the charge. On Sentinel-03 that is R90, R91, R30 and R32, microvolts.
  - **F5:** the floor compares a part's minimum with the reset at the monitor, not at the part's pin less the drop. This predates 4310 for bus devices, and 4310 widened it.
  - **F6:** the release-floor driver cites DS12117 Table 50's 2.7–3.6 V row. At 2,870 − 110 = 2,760 mV that row holds for Sentinel-03; a release floor under 2.7 V would need the 1.62–3.6 V / 4 mA row.
  - **F8:** E4 reads back the twin's own disconnection, the `INV-07` shape `verify.rs` names for held inputs. It cannot cover falsely: it needs a passing baseline, and fewer than two rises is a failure (D7 (iii), ruled).
  - **F10 (a):** rebind admits any new scenario entry, not only D7's; renames and removals are still refused.
  - **F11:** off-board peers (J3's EXT budget, J2) are powered by the board and judged by nothing, and no verdict line names them.

## 6. Rows

- **Sentinel-03's two refusals,** to Scotty (08:55Z): (i) a copper change, (ii) a certified lower bound (the dual solve) as its own engine work order, (iii) acceptance by name. The architect leans (ii) then (i). U30 and U31.B: (i) a monitor on U30's EN at or above 3.0 V with the rail raised, or (iii).
- **A certified lower bound on a two-terminal resistance** (the dual, Dirichlet, solve on an outer approximation of the copper). It would turn the √ or maximum-principle charges into brackets. An engine work order.
- **Path B (`supervisor::check`, a `validate` rule) stays on bus devices** (ruled 08:22Z). Extending it is a row.
- **(5)(i)'s unmet bus names no reason.** `coverage()` sees the scenarios' classes, not the overrides, so "a raised pull-up within no declared tolerance" is not stated beside it. A row.
- **`supply_tier1`'s colocated block is heavier:** every Sentinel-03 extract is about 1.9× longer (562 → 1,078 s on Furnace). Its `ci/colocate.tsv` row is raised to a stated estimate and refit from this PR's CI run under the REFIT RULE.
- **The F3–F6, F10 (a) and F11 items above.**

## 7. Reduction pass, gates

- **Reduction pass (diff and subsystem):**
  - 5 removals in the supply bounds: one `drop_bound` (the `Shared` charge an `Option`), one `largest_bound` for four copies, one `pads_on` for two filters, no `u64::MAX` sentinel in `entry_bounds`, and the pack's text built once.
  - 1 protected kept: E4's `holds: true`, labelled.
  - 4 findings not done (`spike/4310/REDUCTION.md`):
    - `devices()` against `powered()`: Path B's unknown-part case differs;
    - `floors_above_reset` and `named` are the rule's named API;
    - the radio line's wording is pinned;
    - `coverage`'s two walks are under the bar.
  - The line count is not a clean figure. The reduction commit's +114/−103 in `supply.rs` and `power.rs` includes `cargo fmt` reflowing (3c)'s code, which had been committed unformatted. The five removals themselves are about −45 lines.
- **Gates (at the READY head):**
  - `cargo fmt --all -- --check` passes.
  - `cargo clippy --workspace --all-targets -D warnings` passes, with and without `--features aethelgard-cli/with-tools`. It found three things in this branch's code, all fixed: a type alias, `is_multiple_of`, and (5)(ii) reading the specification's device lists where `RUL-16` requires the solved topology.
  - `cargo test --workspace`: see §8.
  - `ci/route.sh check` and `ci/test-route.sh` pass.
  - `ci/shard.sh plan` was run in the pinned image on `6c9a2529`: guard ok, 3,598 test names in 20 shards. `supply_tier1` is alone in shard 2 at 1,350 s, under shard 1's 1,715 s, so the longest shard does not grow. Re-run on the push head (`main` merged): guard ok, 3,603 names, the same shard times.
- **CI weights:** `supply_tier1`'s colocated block is raised 703 → 1350, by the 1.92× measured on Sentinel-03's extract. `package_built_4310_tier1`'s three tests run 5.2, 1.0 and 1.7 s alone on Furnace; their rows use 2.8× those (AET-WO-1310's factor). Each is a stated estimate, to be refit from this PR's first CI run (REFIT RULE 3 and 5).

## 8. Status

READY, unmerged. The merge waits on Scotty's two Sentinel-03 answers (08:55Z), because `bench-package-s03` cannot pass on a board the engine now correctly refuses. `main` `d4c68028` (1802) is merged; the architect dispatches the package write on this head.

**`cargo test --workspace` (tier 1):**
- On `f4110ddc`, `main` merged: 3,352 passed, 12 failed.
- One failure is by design: `bench_board_tier1`'s committed-package check, until the write dispatch.
- The other eleven were in `fab_tier1`. They are fixed in `f7355fb0`, and `fab_tier1` passes 19/19; the bin's 280 and `package_built_4310_tier1`'s 3 pass again too. No other code changed after the full run. The two causes are below.

**What the full run found that part (1) did not.** I own both; part (1)'s survey looked at committed boards and packages, not at the test sets.
- **D7 (i) reaches five test sets** whose only cover for `SENSORS` was a tolerated pull-up: `fab_tier1`'s own two sets, the ready fixture's (`common/ready.rs`, read by `verify_2k2_tier2` and `against_fab_tier2`), `four_layer_fab_tier2`'s and `board_tier2`'s renamed set. Each gains a detected held-low SDA, as Sentinel-01-bench does. `fab_tier1`'s coverage cases now name `SENSORS` wherever a pull-up was its only cover, and add a case where the held-low SDA covers it. In the pinned tools image on `f7355fb0` (16G cap, 4 test threads), all pass: `verify_47k_tier2` 2/2 (151 s), `verify_2k2_tier2` 2/2 (337 s; its campaign now judges 6, and 3 faults are attributed), `against_fab_tier2` 4/4 (458 s), `board_tier2`'s `every_admitted_identifier_character_survives_every_output` (149 s) and `four_layer_fab_tier2` 2/2 (438 s).
- **`states_built` ran before coverage** and read FAB-NOTES through rebind's reader. An empty `--against-fab` directory was therefore refused as `RebindNoPackage` before anything else. A directory with no readable FAB-NOTES is now left to the comparison, as before 4310; only a FAB-NOTES that is read and lacks a key is `PackageBuiltUnstated`.
