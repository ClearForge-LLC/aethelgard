# AET-WO-3303b FEEDBACK: the schedule harness split on every board, not only on a board with a timer

**Branch:** `wo/aet-3303b-split-every-board`, from `main` 3f989435. Builder: Claude Opus 5.5, 2026-11-19.

**Outcome:** built, with no §7 stop.
- Every board under `boards/` proves every harness in the pinned image.
- On a board without a timer, the schedule now proves as two or three parts. Each part passes where the whole passed.
- The largest peak of any part is 4.44 GB, against 12.16 GB for the bench board's whole harness.
- Recommended cap (§5): the **8 GiB default** for `prove` and `verify` on a board without a timer. The ruling is the architect's.

## 0. How the runs were made

- **Image and cap.** The pinned tools image (`container/IMAGE`) ran under `~/bin/capped-run --mem 16G` (`MemorySwapMax=0`, read back inside the scope), with `--network none` and `nice -n 10`. One container of mine ran at a time (`spike/3303b/sh/img.sh`, `flock`). Each harness had its own container, so each row's peak is that step's own scope `memory.peak`. Page cache is included, so a peak is an upper bound (`spike/4600/sh/step.sh`, used as it is on `main`).
- **Trees.** Neither phase read the live worktree. Each mounted a `git archive` snapshot read-only:
  - before: `main` 3f989435;
  - after: 13fef3ac. It holds the split (bba3e60c) and PREDICTIONS (2fb2f0fc). Later commits change comments, tests' text, spike files and this file only.
- **Binaries.** Each phase used its own CLI, built in the image (`debug`, `--locked`), plus the bridge:
  - before: `aethelgard` `6f121539…`;
  - after: `aethelgard` `4aa63ffd…`;
  - `aethelgard-bridge` `44abce24…` in both phases.
- **Boards.**
  - The two committed crates, proved with `--spec`: `boards/sentinel-01` from `sentinel-01-minimal.json`, and `boards/sentinel-01-bench` from `sentinel-01.json`. Every verdict says `harness_provenance: generated`.
  - `4600`'s `logger-i2c-only.json` and `logger-spi-only-cap.json`, copied to `~/.cache/aet-3303b/specs` and re-pinned from schema 16 to 17. That is the only line that changed in either (`diff` shows `2c2` and nothing else). Each phase's CLI generated them.
- **Runner:** `spike/3303b/run.sh <before|after> <board>`.
  - It proves each `#[kani::proof]` the crate declares.
  - It refuses a crate whose list holds both the whole harness and a part.
  - On a split crate it then proves the old name, `proofs::every_valid_sequence`.
  - With `VERIFY=1` it ends with `verify --spec` (no scenarios).
  - Rows are in `spike/3303b/out/<phase>/<board>/rows.tsv`. `spike/3303b/table.py` builds §1's table from them.
- **What went wrong in the kit, and what was re-run.**
  - The first before-`verify` (Sentinel-01) exited 4, `BridgeUnavailable`, because I had not built `aethelgard-bridge` beside the CLI (kept in `out/before/sentinel-01/verify-nobridge.tsv`). I stopped the bench before-`verify` it was running, built the bridge into both phases' targets, and re-ran all four before-`verify`s.
  - A broken wait loop started the after-chain while the before-`verify`s still ran. The `flock` serialised their containers, so one container at a time held throughout, and every row is its own scope's. The interleaving moves only wall times.
- **n = 1** for every row, on a shared Furnace. The noise is measured in §1.3.

## 1. Before and after, with the predictions

PREDICTIONS.md was committed alone (2fb2f0fc) before any run on split output. "Held?" compares each part with its predicted range (PREDICTIONS §2).

### 1.1 Every harness

| Board | Harness | Before (exit · wall · peak) | After (exit · wall · peak) | Predicted | Held? |
|---|---|---|---|---|---|
| sentinel-01 | `every_valid_sequence` | 0 · 314.3 s · 7.43 GB | (a part list) | | |
| sentinel-01 | `every_valid_sequence_i2c1` | — | 0 · 22.1 s · 1.44 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01 | `every_valid_sequence_spi1` | — | 0 · 53.3 s · 3.26 GB | 0 · 45 s · 2.0 GB | time held, peak held |
| sentinel-01 | `i2c1_transfer_is_bounded_and_ordered` | 0 · 9.0 s · 0.74 GB | 0 · 10.1 s · 0.70 GB | unchanged | held |
| sentinel-01 | `spi1_transfer_is_bounded_and_ordered` | 0 · 14.1 s · 1.05 GB | 0 · 18.1 s · 1.15 GB | unchanged | held |
| sentinel-01 | `spi1_transfer_refuses_too_many_frames` | 0 · 7.1 s · 0.52 GB | 0 · 8.1 s · 0.47 GB | unchanged | held |
| sentinel-01 | `reset_clock_is_ok_exactly_at_reset` | 0 · 2.0 s · 0.09 GB | 0 · 2.0 s · 0.10 GB | unchanged | held |
| sentinel-01 | `usart1_start_writes_the_solved_values` | 0 · 9.1 s · 0.83 GB | 0 · 7.0 s · 0.83 GB | unchanged | held |
| sentinel-01 | `flash_option_status_is_one_read` | 0 · 3.0 s · 0.09 GB | 0 · 2.0 s · 0.09 GB | unchanged | time −33 % (1 s floor) |
| sentinel-01 | `every_valid_sequence` (old name, split crate) | — | **1 · `harness_not_found`** · 2.0 s | refused | held |
| sentinel-01-bench | `every_valid_sequence` | 0 · **1224.4 s · 12.16 GB** | (a part list) | | |
| sentinel-01-bench | `every_valid_sequence_i2c1` | — | 0 · 22.1 s · 1.63 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01-bench | `every_valid_sequence_i2c2` | — | 0 · 18.1 s · 1.57 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01-bench | `every_valid_sequence_spi1` | — | 0 · 46.3 s · 2.87 GB | 0 · 45 s · 2.0 GB | time held, peak held |
| sentinel-01-bench | `i2c1_transfer_is_bounded_and_ordered` | 0 · 15.0 s · 0.99 GB | 0 · 11.1 s · 0.72 GB | unchanged | **peak −28 %** |
| sentinel-01-bench | `i2c2_transfer_is_bounded_and_ordered` | 0 · 14.1 s · 0.82 GB | 0 · 10.1 s · 0.71 GB | unchanged | **peak −13 %** |
| sentinel-01-bench | `spi1_transfer_is_bounded_and_ordered` | 0 · 19.1 s · 1.16 GB | 0 · 16.1 s · 1.10 GB | unchanged | held |
| sentinel-01-bench | `spi1_transfer_refuses_too_many_frames` | 0 · 9.0 s · 0.55 GB | 0 · 7.0 s · 0.50 GB | unchanged | held |
| sentinel-01-bench | `reset_clock_is_ok_exactly_at_reset` | 0 · 3.0 s · 0.10 GB | 0 · 3.0 s · 0.10 GB | unchanged | held |
| sentinel-01-bench | `usart1_start_writes_the_solved_values` | 0 · 12.1 s · 0.86 GB | 0 · 10.0 s · 0.86 GB | unchanged | held |
| sentinel-01-bench | `flash_option_status_is_one_read` | 0 · 3.0 s · 0.09 GB | 0 · 2.0 s · 0.09 GB | unchanged | time −33 % (1 s floor) |
| sentinel-01-bench | `every_valid_sequence` (old name) | — | **1 · `harness_not_found`** · 2.0 s | refused | held |
| logger-i2c-only | `every_valid_sequence` | 0 · 156.9 s · 4.93 GB | (a part list) | 4600's 110.7 s ±30 %, 4.62 GB ±10 % | **time MISS (+42 %)**, peak held |
| logger-i2c-only | `every_valid_sequence_i2c1` | — | 0 · 19.1 s · 1.53 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| logger-i2c-only | `every_valid_sequence_i2c2` | — | 0 · 19.1 s · 1.52 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| logger-i2c-only | `every_valid_sequence_gpio` | — | 0 · 3.0 s · 0.21 GB | 0 · 4 s · 0.3 GB | time held, peak held |
| logger-i2c-only | the five targeted harnesses | 0 ×5 · ≤12.1 s · ≤0.88 GB | 0 ×5 · ≤12.1 s · ≤0.85 GB | unchanged | held |
| logger-i2c-only | `every_valid_sequence` (old name) | — | **1 · `harness_not_found`** · 1.0 s | refused | held |
| logger-spi-only-cap | `every_valid_sequence` | 0 · 465.9 s · 10.65 GB | (a part list) | 4600's 390.4 s ±30 %, 10.65 GB ±10 % | held, held |
| logger-spi-only-cap | `every_valid_sequence_spi1` | — | 0 · 62.2 s · **3.64 GB** | 0 · 45 s · 2.0 GB (1.0–3.5) | time held, **peak MISS** |
| logger-spi-only-cap | `every_valid_sequence_spi2` | — | 0 · 102.5 s · **4.44 GB** | 0 · 45 s · 2.0 GB (1.0–3.5) | time held, **peak MISS** |
| logger-spi-only-cap | `spi1_transfer_is_bounded_and_ordered` | 0 · 16.1 s · 1.37 GB | 0 · 22.1 s · 1.18 GB | unchanged | **time +38 %, peak −14 %** |
| logger-spi-only-cap | `flash_option_status_is_one_read` | 0 · 2.0 s · 0.09 GB | 0 · 2.0 s · 0.12 GB | unchanged | **peak +32 %** |
| logger-spi-only-cap | the other five targeted harnesses | 0 ×5 · ≤16.1 s · ≤1.26 GB | 0 ×5 · ≤16.1 s · ≤1.26 GB | unchanged | held |
| logger-spi-only-cap | `every_valid_sequence` (old name) | — | **1 · `harness_not_found`** · 1.0 s | refused | held |

Every row, with no aggregation, is in `spike/3303b/out/table.md`.

### 1.2 The schedule, and `verify`

| Board | Schedule before | Schedule after: sum of parts · largest part | Predicted after | `verify` before (exit · wall · peak) | `verify` after |
|---|---|---|---|---|---|
| sentinel-01 | 314.3 s · 7.43 GB | **75.3 s · 3.26 GB** (2 parts) | ~60 s · 2.0 GB | 0 · 335.9 s · 7.53 GB | **0 · 114.7 s · 3.20 GB** |
| sentinel-01-bench | 1224.4 s · 12.16 GB | **86.5 s · 2.87 GB** (3 parts) | ~75 s · 2.0 GB | 0 · 1171.1 s · 12.22 GB | **0 · 149.9 s · 3.81 GB** |
| logger-i2c-only | 156.9 s · 4.93 GB | **41.2 s · 1.53 GB** (3 parts) | ~35 s · 1.0 GB | 0 · 150.8 s · 4.71 GB | **0 · 82.3 s · 1.78 GB** |
| logger-spi-only-cap | 465.9 s · 10.65 GB | **164.7 s · 4.44 GB** (2 parts) | ~90 s · 2.0 GB | 1 · 475.5 s · 10.67 GB (`extract`, F18) | **1 · 210.1 s · 4.72 GB** (`extract` only, every prove gate passes) |

- **Every after-`verify` names exactly the crate's harnesses as prove gates.** It runs each part, and there is no `proofs::every_valid_sequence` gate (`out/after/*/verify.json`).
- **`logger-spi-only-cap` is the same before and after:** it fails at `extract` alone (4600 F18: `LOG_SCK` 37.5 pF against 30 pF), and every prove gate passes.

### 1.3 The misses, and the noise

- **The two SPI parts of `logger-spi-only-cap` peaked above my range:** 3.64 and 4.44 GB against 1.0–3.5 GB. So did the largest part overall (PREDICTIONS §2 said under 3.5 GB) and the cap prediction (§4 said `prove` fits 4G). 4.44 GB is above 4 GiB (4.29 GB). Two SPI buses cost more per part than one SPI bus does on Sentinel-01 (3.26 GB). The reason was not measured. A likely cause is the rely: each SPI part havocs the other bus's footprint, its port bits included, inside a larger image.
- **The bench board's whole harness was costlier than my context estimate:** 1224 s and 12.16 GB against 350–700 s and 8–12 GB. It was the most expensive run of all, 3.9 times Sentinel-01's for one more I2C bus.
- **`logger-i2c-only`'s whole harness took 156.9 s against 4600's 110.7 s (+42 %).** Its peak held (+7 %).
- **`logger-i2c-only`'s `gpio` part has no output in it.** PREDICTIONS §1 called it "an output no unit owns". It holds port A's enable only, for the USART's pins (the adversarial pass, §6, spot-checked: zero `into_output`). See §6 on what that leaves untested.
- **Noise.**
  - Six unchanged harnesses fell outside my ±30 % time or ±10 % peak bands, though their text is byte-identical function by function on both committed boards (checked with `sha256sum` per function).
  - The time misses are 2–3 s harnesses against `step.sh`'s 1 s poll.
  - The peak misses run from −28 % to +32 %. So **a single scope peak moves by up to a third for the same harness**, page cache included. §4 and §5 use that bound.

## 2. The split

- **The change** (`crates/aethelgard-svd-hal/src/board.rs` L3905–3909): `emit_proofs` computes `parts(plan)` and splits whenever it yields two or more. A board with fewer keeps the whole harness. `check_parts` (L3829) is unchanged and runs on every split. `parts()`, `Scope::actions` and `footprint` are unchanged: nothing had to change for a board without a timer.
- **One behaviour change beyond the work order's words.** A board with one timer and no other unit used to emit a one-part split (`_tim1`); it now emits the whole harness. That is the rule "a split into one part is the whole". No fixture has that shape, and it was not run.
- **The reset-clock read with no timer.** `check_parts` does **not** treat it as one owner, as it does a port enable or the kernel selection. The work order's wording expected it to; this is what the code does:
  - `Scope::actions` schedules `ResetClock` in every part with a peripheral or a timer (L3676–3677), so with no timer each peripheral part schedules it.
  - Its footprint is empty: `Action::ResetClock => {}` (L3805).
  - The owner list holds each port enable and the one `SelectKernels` (L3842–3845). It filters only those two out of a part's own actions (L3851). So `ResetClock` sits in every part's own set and adds no bit to it, and two parts cannot overlap on it. Neither would a third part, or a timer's.
  - What proves the read writes nothing is the per-write footprint assertion in each part: its arm has no footprint row, so any write in it fails. So does `reset_clock_is_ok_exactly_at_reset` ("reset_clock wrote a register").
  - The motor boards' I2C and SPI parts already schedule it the same way, since `AET-WO-3303`.
- **Two SPI parts** (`logger-spi-only-cap`, the first such board proved). Each part schedules `SelectKernels` and footprints the select register whole on that arm. Neither havocs it in its rely, since it is one of its own actions. Each asserts the whole word at its end. The oracle's word holds every SPI's field (`oracle_selects`). Both parts pass (§1).

### 2.1 Which harnesses run, and that none drops out

- **The code that chooses:** `aethelgard verify` proves every `#[kani::proof]` that `harnesses()` reads from the generated `src/proofs.rs` (`crates/aethelgard-cli/src/verify.rs` L1041, used at L1204). `unplanned()` (L1027, called at L1208) counts every `kani::proof` in the crate and refuses a package when the count differs. `cli` was not changed: no §7 stop.
- **A no-timer board's parts all run under `verify`** (measured, §1.2). `verify_2k2_tier2` now pins the exact list of eight prove gates on Sentinel-01: both parts, the six targeted harnesses, and no `every_valid_sequence`.
- **`proofs::every_valid_sequence` no longer exists as a proof on a split board.** Its image and oracle text stay for the targeted harnesses, unannotated. Asked for by name, `prove` refuses with `harness_not_found`, exit 1, verdict `fail` (measured on all four boards, §1.1; `kani.rs`).
- **CI.**
  - `board_tier2::the_generated_board_proves` reads the parts from the committed crate, as `twin_tier2` does for the motor fixtures. It asserts exactly `[_i2c1, _spi1]` and proves each.
  - `attribute_seeded`'s re-gate proves each schedule harness of the proposal's crate: the whole, or its parts.
  - No `.yml` or `.sh` names the old harness. `generate_tier1.rs:350` names it in a call refused before any tool (`SchemaViolation`). `board_tier2`'s hand-written vacuous harness writes its own `proofs.rs`.

## 3. Tests

In `board.rs`'s `mod tests`:
- **Positive:** `a_board_without_timers_splits_its_schedule_into_its_parts`. A plan with no timer, I2C1, I2C2 and SPI1. Its parts are `i2c1`, `i2c2` and `spi1`, and `check_parts` passes. Each part's peripheral, pins, ports and actions are what `parts()` documents: the reset-clock read in each, and the kernel selection only in SPI's. All three are emitted as `#[kani::proof]`, and the whole is not.
- **Mutant 1:** `a_timerless_split_whose_parts_share_a_bit_is_refused`. PB10 is also put in `i2c1`, giving `FootprintOverlap` "the i2c1 part" / "the i2c2 part", `GpioB at 0x0, bits 0x00300000`.
- **Mutant 2:** `a_timerless_split_that_leaves_an_action_unscheduled_is_refused`. PB11 is dropped from `i2c2`, giving `UnscheduledAction` "the PB11 configure".
- **Boundary:** `a_board_with_one_part_keeps_the_whole_schedule_harness`. A one-part plan keeps the whole, with no `rely`. Sentinel-01's own fixture (I2C1 plus an output no unit owns) splits into `i2c1` and `gpio`, and that `gpio` part drives PA0. It replaces `a_board_without_timers_keeps_the_whole_schedule_harness`, which asserted the old rule.

**Each mutant fails on the unmutated tree with its condition removed.** `spike/3303b/mut.py` applies one mutant per guard this work order adds or changes to a `git archive` of 13fef3ac, then runs `board::tests`. Rows are in `out/mutants.tsv`.

| Mutant | Result | Killed by |
|---|---|---|
| M1 `check_parts` without the `UnscheduledAction` loop | killed | mutant 2, and the motor test |
| M2 `check_parts` skips `disjoint` | killed | mutant 1, and the motor test |
| M3 the split keyed on timers again | killed | positive, boundary |
| M4 `parts.len() < 1` | killed | boundary |
| M5 `parts.len() < 3` | killed | boundary |
| M6 `emit_proofs` without its `check_parts` call | **survived** | — |

M6 survives because no plan that `parts()` builds can fail `check_parts`: the call guards against a regression in `parts()`, and any such regression is caught by the direct `check_parts` tests above. The same call was unreachable on `main`'s timer branch. A residual, stated.

## 4. CI weights (REFIT RULE 5; fence widened to `ci/colocate.tsv` by the architect, 21:40Z)

**Method.** Every test or block whose cost the split moves is re-weighed at its in-shard time on PR #255's run 44621800144 (head 47983ca5). `main`'s run 44613208491 (96517013) is the control. Both runs' shard logs were read with `AET-WO-1910`'s `pertest.py`.
- A `board_tier2` prove is its `ANSWER` `wall_s`. Only that binary's provers take the lock, so "the previous completion" would count other tests.
- A `verify_2k2_tier2` or `against_fab_tier2` test is its `ok` less the previous lock-holder's.
- A colocated block is its libtest `finished in`.

| Test or block | Row before | `main`'s run | PR's run | New row |
|---|---|---|---|---|
| `board_tier2::the_generated_board_proves` | 414.9 | 404.6 | 105.8 (28.8 + 77.0) | 105.8 |
| `board_tier2::a_wrong_alternate_function_fails` | 369.7 | 317.1 | 17.9 (19.45 on the second run) | 19.5 |
| `board_tier2::a_write_past_the_block_fails` | 347.9 | 380.5 | 26.1 | 26.1 |
| `verify_2k2_tier2::verify_binds_the_reference_and_two_runs_print_the_same_bytes` | 1165.8 | 848.4 | 356.3 (434.3 on the second run) | 434.3 |
| `verify_2k2_tier2::verify_fab_out_writes_the_package_its_verdict_names_at_any_path` | 993.5 | 971.6 | 407.3 (448.9 on the second run) | 448.9 |
| `against_fab_tier2::verify_against_fab_finds_the_package_it_wrote_identical` | 1164.6 | 1081.9 | 450.8 | 450.8 |
| `against_fab_tier2::verify_against_fab_names_every_changed_and_extra_file` | 1108.9 | 891.6 | 418.0 | 418.0 |
| block `attribute_seeded` | 1086 | 985.98 | 386.04 | 387 |
| block `verify_47k_tier2` | 448 | 504.76 | 227.97 | 228 |
| block `four_layer_fab_tier2` | 1371 | 1272.99 | 581.82 | 582 |
| `hot_plug_tier1::the_declared_bulk_capacitor_is_c1_positive_on_vbat_on_both_power_boards` (`AET-WO-1315`, no row) | — | under 60 s | 60.48 | 60.5 |

- **The rows are lowered although the tests' code is the same**, departing from the rule's "keep the larger earlier row". The cost the earlier rows measured, the whole schedule harness on a board without a timer, no longer exists. `main`'s run is the evidence of what it was.
- **The other blocks are left alone.** `campaign_tier2`, `power_board_tier1`, `outline_tier1`, `supply_tier1` and `via_ring_tier1` run no timerless schedule proof. Their differences between the two runs are runner noise (`supply_tier1` 366 s on `main`, 684 s here).
- **P2 fired on this PR's first run, and was right to.** It fired on `hot_plug_tier1`'s test, which arrived with `main` (`AET-WO-1315`) with no row and ran 60.48 s, past libtest's 60 s line. On `main`'s own run it stayed under the line. Every test in that shard passed; the shard failed on P2 alone.
- **Planner.** `ci/shard.sh plan` in the pinned image on the refitted tree exits 0, so **P1 is silent**: 2,507 test names, 2,514 occurrences, 8 shards, each planned at 1,152 s. P2 is judged in the shards of the next run (§10).
- **Two runs, the worse of each:** run 44624532447 (the bench-package write dispatch, same head, green on every job) measured higher in three rows, which are raised to it:
  - `a_wrong_alternate_function_fails`: 19.45 s, row 19.5;
  - `verify_binds…`: 434.3 s;
  - `verify_fab_out…`: 448.9 s.

  Every other row keeps the first run's larger figure. Second-run figures for the rest: the generated board 25.4 + 68.3 s; `a_write_past_the_block_fails` 23.1 s; `against_fab_tier2` 364.1 and 280.2 s; `attribute_seeded` 321.89 s; `verify_47k_tier2` 226.99 s; `four_layer_fab_tier2` 485.65 s; `hot_plug_tier1` 49.9 s.
- **1911 merges first (architect, 22:10Z), with 12 shards and its own refit.** After it lands I merge `main` and re-weigh these rows against its layout by the same rule, before READY.

## 5. The cap (§1.5)

- **Largest peak measured after the split:**
  - for `prove`: **4.44 GB**, `logger-spi-only-cap`, `every_valid_sequence_spi2`;
  - for `verify`: **4.72 GB**, the same board, whose `verify` fails at `extract` after every prove gate passes;
  - for a passing `verify`: 3.81 GB on `boards/sentinel-01-bench`.
- **Recommendation: `--mem 8G`**, the default, for both `prove` and `verify` on a board without a timer. Retire the 16G rule of #245 (comment 7020363017).
  - At the measured noise of +32 % (§1.3), 4.72 GB becomes 6.2 GB. That is under `4600`'s 900 ‰ stop line at 8 GiB (7.73 GB).
  - **Not 4G:** the `_spi2` part alone is above 4 GiB.
  - **Not measured:** a board with three or more SPI buses, or two SPI and two I2C. Two SPI buses cost 4.44 GB per part against 3.26 GB for one, so a third may raise it. A new board shape should be measured once before trusting 8G.

## 6. Adversarial pass (full; a fresh Opus subagent, given the code, not my conclusions; its claims spot-checked by me)

| Attack | Result | What catches it |
|---|---|---|
| An action no part schedules | not possible on a generated board | `check_parts` `UnscheduledAction` (L3830–3838); mutant 2; M1 killed |
| A shared bit with two owners | not possible | `disjoint` (L3866); port enables and `SelectKernels` are single owners (L3842–3845); each part asserts every write against its running action's footprint, so an under-stated footprint fails the part's proof; mutant 1; M2 killed |
| An ordering the whole constrained and no part does | none found | every `assume` in a part is the whole's for the same action. The shared tokens (`clock`, `kernels`) come from shared actions that each part schedules itself, once. Residual stated: the I2C part havocs `rcc[20]` (D2CCIP1R) whole before `reset_clock` reads it (`boards/sentinel-01/src/proofs.rs` L342, L604), which is more permissive than the whole, not less. |
| A harness that no longer runs under `verify` or CI | none | `harnesses()`, `unplanned()`; `verify_2k2_tier2` pins the eight gates; `board_tier2` and `attribute_seeded` enumerate the parts; the old name is `harness_not_found` |
| A peak measured on a run that was not the split's | none | each prove copies the crate into fresh scratch and builds in its own target (no cache across rows); `run.sh` refuses a list with both the whole and a part; `out/after/*/harnesses.txt` holds no whole harness |
| Weights fitted to one noisy run | see §4 | the peak noise is ±32 % at n = 1 (§1.3), and wall times are single runs on a shared box; §4 weighs from CI by the documented rule, not from these |

**Untested shape (residual):** a timerless board's `gpio` part that configures an output pin, under Kani. Of the four boards run, only `logger-i2c-only` has a `gpio` part, and it holds a port enable alone. A `gpio` part with an output is covered by tier-1 text tests and by `AET-WO-3303`'s throwaway motor board (2.6 s), not by a run here.

## 7. Reduction pass (diff)

Reduction pass (diff): −27 lines net · 1 duplicate removed · 0 dead paths removed · 0 protected kept · 2 findings below.
- **Removed:** `spike/3303b/sh/step.sh`, a byte-identical copy of `spike/4600/sh/step.sh` on `main` (`cmp`). `run.sh` now copies `main`'s file into the cache. The runs above were made with the copy, which is the same bytes.
- **Finding R1 (outside the fence):** three copies of "the schedule harnesses of a board crate" (`strip_prefix("fn every_valid_sequence")`):
  - `board_tier2.rs:123`;
  - `attribute_seeded.rs:1180`;
  - `twin_tier2.rs:2507`.

  One helper in `tests/common/` would hold it, but `common/` and `twin_tier2` are outside this fence. Backlog.
- **Finding R2:** `spike/3303b/table.py` restates PREDICTIONS §2's numbers. Parsing the Markdown would add lines, not remove them, so it is kept: a record, not a mechanism.

## 8. Fence deviations and rulings (architect, 21:40Z, `~/.cache/aet-bus/3303b.md`)

- **F1, existing tests edited to name the parts: accepted.** One line each:
  - `board_tier2.rs`: `the_generated_board_proves` proves each schedule part of `boards/sentinel-01` (asserting exactly `_i2c1`, `_spi1`); the seeds `a_wrong_alternate_function_fails` and `a_write_past_the_block_fails` run on `_i2c1` (both measured failing on their own message there, in the image); the hand-written vacuous harness keeps its own name.
  - `attribute_seeded.rs`: the re-gate proves every schedule harness of the proposal's crate (`schedule_harnesses`), the whole or its parts.
  - `verify_2k2_tier2.rs`: eight prove rows where there were seven, and the prove gates pinned by name.
  - `svd-hal/tests/serial_ports.rs`: the `src/proofs.rs` digest row, from `AETHELGARD_PRINT_DIGESTS=1` (accepted as tooling output), the re-take logged in its doc comment.
- **F2, the bench package moves: accepted, the established flow.** `MANIFEST.json` binds `verdict_sha256`, whose verdict names the new board crate hash and three schedule gates. Pushed AWAITING-PACKAGE; the architect dispatches `ci.yml` `write=true`, and I commit the package unchanged.
- **F3, `ci/colocate.tsv`: the fence is widened** to the rows this change makes cheaper (`attribute_seeded`, `four_layer_fab_tier2`, `verify_47k_tier2`, any other measured), re-weighed by the documented method from measured CI runs (§4).
- **F4, the stale crate READMEs: fixed** (fence widened to those lines): `svd-hal/README.md` L67 (the split's rule and its reason) and `cli/README.md` L912 (the example prove gate is `proofs::every_valid_sequence_i2c1`, `cover_count` 6, as measured).
- **D1, the cap (§5):** 8G for `prove` and `verify` on a board without a timer. The architect's call.

### 8.1 Every caller of the unsplit name (`every_valid_sequence` not followed by `_`), grep of the whole tree and `~/.cache/aet-*` scripts

| Where | What it is | Needs |
|---|---|---|
| `svd-hal/src/board.rs` L4874 (`fn every_valid_sequence{sfx}`) and the tests at L7436, L7481, L7485 | the generator's name for the whole or a part; tests asserting which is emitted | nothing |
| `svd-hal/src/board.rs` (the SPI transfer harness's comment) and its two lines in each committed `proofs.rs` | named the whole harness as where `CR1` is checked idle | **fixed**: "the schedule harness or each of its parts" (comment only; boards re-blessed, digest re-printed; the measured after-tree 13fef3ac differs from the final head by these comment lines only) |
| `svd-hal/README.md` L67 | the split's rule | **fixed** (F4) |
| `svd-hal/README.md` L74 | "`every_valid_sequence`, and on a motor board each of its parts, takes no USART step" | outside the widened lines: "on a motor board" should read "on a board with two or more parts". The architect's |
| `svd-hal/README.md` L78–82 | a measurement table from `AET-WO-4300b` | nothing (a record) |
| `svd-hal/tests/motor_outputs.rs` L321; `cli/tests/twin_tier2.rs` L2507, `board_tier2.rs` L123, `attribute_seeded.rs` L1180 | prefix matches that enumerate the whole or its parts | nothing (finding R1, §7) |
| `cli/tests/board_tier2.rs` L424, L433; `generate_tier1.rs` L282; `cli/src/verify.rs` L2462 | hand-written harnesses named so in their own text | nothing |
| `cli/tests/generate_tier1.rs` L350 | `prove --spec` refused before any tool (`SchemaViolation`) | nothing; the name is never looked up |
| `cli/README.md` L912 | example verdict | **fixed** (F4) |
| `docs/roadmap.md` L808 | a §7.1 CI-cost row ("Kani's `every_valid_sequence` takes 158–245 s per seed") | the architect's: this work order removes that cost on timerless boards (§1.2) |
| `docs/work-orders/*` (3303, 3303b, 4300a/b, 1400, 1601, 1909, 1910, 3200, 3301, 3304, 4200, 4230, 4600 and their FEEDBACK), `docs/reviews/2026-11-15-mythos-5-2.md` | records | nothing |
| `spike/4600/**`, `spike/4201/out/*`, `spike/3303b/**` | run records | nothing |
| No `.yml`, `ci/` or repository `.sh` | — | — |
| Furnace, for information: `~/.cache/aet-4240v/a1/kani/batch.sh` L8, L9, L15 and `batch2.sh` L10, L11 | an active builder's runner proving `every_valid_sequence` by name on Sentinel-01 | after this merges, `harness_not_found` on a split crate: prove `every_valid_sequence_i2c1` and `_spi1` (and `_i2c2` on the bench board). Told on the bus |
| Furnace: `~/.cache/aet-4230/b1/kani-mutants.sh` L25, `~/.cache/aet-4230/r3fix/queue.sh` L4 | a merged work order's old runners | nothing |

## 9. Gates

- On bba3e60c (host): `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings` 0; `cargo test --workspace` 2,279 passed, 0 failed.
- Re-run on the final head: §10.

## 10. Final state

Filled at READY.
