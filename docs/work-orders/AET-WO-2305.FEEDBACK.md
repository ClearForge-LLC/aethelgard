# AET-WO-2305 — FEEDBACK (build, one measured stop)

Branch `wo/aet-2305-bor-recovery`, from `main` at `2fda6e13`. The work order was read from the docs branch (copy at `~/.cache/aet-2305-wo.md` on Furnace); it is not committed here.

Every figure is labelled **measured**, **read at source** or **inferred**. The predictions (`spike/2305/PREDICTIONS.md`, commit `f08ad2b9`) were committed before the runs (`b4510253`). One trial came before them: Sentinel-01 minimal, run once to learn the log's format. Its rows are marked *seen*.

## How it was measured

- **The CLI:** `main`'s, built in the pinned tools image from a scratch copy carrying one measurement-only patch, never committed. The patch writes Renode's whole log and the composed monitor commands to files (`AET2305_LOG`, `AET2305_CMDS`).
- **The runs:** one campaign per board, with its undisturbed boot first (`spike/2305/scenarios/*-undisturbed.json`, no override), then its droops. `spike/2305/sh/run-all.sh` runs them under `capped-run` (8 GiB, swap 0, read back).
- **The analysis:** `spike/2305/sh/analyze.py` reads the archive (kinds 13, 18, 19) and the log.
  - **The boot word's instant** is exact: the boot's start plus the `AETHELGARD-BOOTWORD` hook's instruction count × 10 ns. The start is the release + the 10 ns nudge, or 0 undisturbed.
  - **A device line's instant** is Renode's stamp, which is good only to a quantum (100 µs).
- **The results:** `spike/2305/out/measure-main.jsonl`, `measure-phase.jsonl` and `measure-timebase.jsonl`.

## §1.1 The measurement table (measured on `main` 2fda6e13), predictions beside

| Run (board, droop) | Release (ns) | Boot word after the boot's start | First identity line (stamp, after start) | Witness lines at the probe | First device's lines by the boot word | Verdict | Predicted |
|---|---|---|---|---|---|---|---|
| S-01 minimal, `droop-bor3` | 1,231,690 | 25.70 µs | 0 | 2 (U2: `Write D0`, `Read 60`) | 2 | pass | *seen* |
| S-01 minimal, `droop-tolerated` | no crossing, no reset | 25.70 µs | 0 | — | 2 | pass | *seen* |
| S-01 bench (`examples/sentinel-01.json`), `droop-bor3` | 1,231,690 | 28.13 µs | 0 | 2 | 2 | pass | 1.22–1.24 ms, 25–40 µs, 2, pass: **met** |
| `motors-dshot600`, `dshot600-droop` | 331,690 | 21.75 µs | 0 | 2 | 2 | pass | 0.32–0.34 ms, 25–60 µs: **boot 3.25 µs earlier than the range**; the rest met |
| `four-layer`, `four-layer-droop` (BOR 0, 3.1 V) | no crossing | 16.70 µs | 0 | — | 2 | pass | ~1.23 ms release, 25–40 µs: **missed. No BOR0 crossing, so no reset; my prediction overlooked BOR level 0** |
| `spi-only`, `spi-only-droop-bor3` | 1,231,650 | 15.24 µs | 0 | 8 (U4, SPI) | 9 | pass | "2 (U3's identity)": **missed. The first check is U4 on SPI, not U3**; boot 20–40 µs missed by 4.76 µs |
| Sentinel-03, `droop-tolerated` (3.0 V) | no crossing | 39,127.18 µs | 66.17 µs (U7) | — | 3 | pass | no crossing, pass: **met** |
| **Sentinel-02-shaped, `s02-droop`** (`2303` F11) | 332,670 | **39,072.41 µs** | 28.42 µs (U2) | 2 | 2 | **fail, `brownout_unrecovered`** (boot word 0 at +100 µs) | 39.0–39.5 ms and the fail: **met**; "4–8 lines by the boot word": **missed, 2.** U2 (BME280) has only its identity step |
| **`4600`'s logger, `droop-bor3`** | 1,231,850 | **19,522.27 µs** | 22.17 µs (U20) | 2 (U20's identity **and** configure: the witness already fails) | 3 | **fail, `brownout_unrecovered`** | 19.5–19.7 ms and the fail: **met**; "U10's identity": **missed. The first check is U20 (BMP581)**; 4–8 lines: missed, 3 |
| **`sentinel-01-ext-bmp581`, `droop-bor3`** | 1,231,770 | **19,523.76 µs** | 0 (U2) | 2 | 2 | **fail, `brownout_unrecovered`** | met, except 4–8 lines (2) |

**Undisturbed boots (measured):** on all nine boards, the undisturbed boot word counted from 0 equals the recovered boot word counted from the boot's start, to the instruction. **Predicted: met.**

**Release phase (measured):** the release was moved through one quantum, by six `start_ns` offsets from 0 to 99,990 ns, on Sentinel-01 minimal and on the logger. The recovered boot word did not move: 25,700 ns and 19,522,270 ns each time, a spread of 0.

## §1.1 The twin's time base

- **Measured, exactly:** I doubled the BMP581's `settle_us` in a scratch copy of `parts.json` (10,000 → 20,000; never committed) and ran ext-bmp581's undisturbed boot.
  - Boot word: 19,523,760 → 39,011,460 ns, so Δ = 19,487,700 ns.
  - Spins: 649,590 → 1,299,180.
  - **So 30.000 ns per spin:** three instructions at Renode's 100 MIPS.
- **Read at source:** `settle_spins` sizes the loop at `ceil(us × hsi_fastest / 10⁶)`, and HSI's fastest is 64.959 MHz (`svd-hal` `board.rs`, `AET-WO-4230` amendment 1).
- **Inferred from both:** the twin runs the settle **1.9488 × the declared time** (3 × 64.959 / 100), the same on every board.
  - The "about 1.95×" in `2303`, `4600` and `4202` is this constant plus the board's own bring-up:
    - logger 34.57 µs;
    - ext-bmp581 36.06 µs;
    - S-02-shaped 97.01 µs;
    - S-03 151.78 µs.
  - That is the boot word less 30 ns × spins.
- **The time base is a constant per board,** and one undisturbed run fixes it. §7's third condition does not fire.

## §1.2 The bound: the stop, its ruling, and what was built

**The stop (§7 condition 2), measured and read at source:**
- **The campaign makes no baseline run.** `cli` `campaign.rs` `run_one` boots each scenario alone, and the runs share nothing.
- **No record held a boot word's instant.**
- **Replay pinned the constant.**

I reported this at 04:50:44Z. **Ruled in at 05:10Z**, with the fence widened to:
- the baseline boot inside the droop's own `simulate`;
- one new record, and trace 17;
- the replay rule;
- one quantum of margin;
- `scenarios/sentinel-03/droop-bor3.json`.

**Built:**
- **Where the undisturbed boot comes from: a run the judge makes and archives.**
  - When a droop's schedule releases a reset, `simulate` first runs the same prepared boot with no reset (`main.rs`, `plan_with`). It has the same firmware copy, booted platform, models, cold writes, levels, stimuli and other overrides, and runs through the same bridge. Its serial capture goes to a directory of its own.
  - It reads that boot's boot-word hook, whose instant is exact (count × 10 ns from 0), in `renode::undisturbed_boot_word_ns`.
  - A unit test holds the two command lines equal outside the droop's schedule (`the_undisturbed_boot_is_the_droops_own_without_its_resets`).
- **What the archive records: kind 32, `RecoveryBound`.**
  - Fields: the undisturbed boot word's instant, the margin (100,000 ns) and the bound, their sum. The instant and bound are absent when the undisturbed boot wrote no boot word, or its log could not be read whole.
  - Rank 6, beside the rail, once exactly when the droop releases a reset.
  - Trace 17. The golden is re-pinned, and setting the version byte back to 16 gives `7eccc893…5d31` again.
- **The bound:** `power::recovery_bound` = the instant + `RECOVERY_MARGIN_NS`. The doc comment states the measurement: a spread of 0 over 9 boards and 12 release phases.
- **The probe:** `power::probe_ns` = release + nudge + bound.
- **Replay:** `check_droop_records` checks the bound = the instant + the margin (in `Record::check`, which also pins the margin) and places each probe there, inside the run.
  - With no bound there is no observation, and every release is `brownout_unrecovered`.
  - The span check lost its constant (`droop_end_ns`). After the measurement, `simulate` refuses `RecoveryBoundPastRun` when a probe would lie past the boot's end or past the next reset's assertion (`recovery_fits`).
- **No boot word means no bound:** each release is then `brownout_unrecovered`, "no recovery bound for the release at … ns: the same boot, run first with no reset, wrote no boot word that could be read".

## §1.5 The re-judge (measured; `spike/2305/PREDICTIONS-2.md` committed first, `fa99c081`)

The branch's CLI against `main` at `416a6dd3`. Both have the same patch and the same firmware (`out/measure-branch.jsonl`, `out/measure-main-416a6dd3.jsonl`, `out/compare-main-branch.txt`). **Every predicted value was met exactly.**

| Run | Bound (ns) | Probe (ns) | Witness lines | main 416a6dd3 | branch |
|---|---|---|---|---|---|
| S-01 minimal `droop-bor3` | 125,700 | 1,357,400 | 2 | pass | pass |
| S-01 minimal `droop-tolerated` | — | — | — | pass | pass |
| S-01 bench `droop-bor3` | 128,130 | 1,359,830 | 2 | pass | pass |
| `dshot600-droop` | 121,750 | 453,450 | 2 | pass | pass |
| `four-layer-droop` | — | — | — | pass | pass |
| `spi-only-droop-bor3` | 115,240 | 1,346,900 | 8 | pass | pass |
| S-03 `droop-tolerated` | — | — | — | pass | pass |
| **S-03 `droop-bor3` (new)** | 39,227,180 | 40,871,280 | 3 | fail, `brownout_unrecovered` | **pass: recovers** |
| **S-02-shaped `s02-droop`** | 39,172,410 | 39,505,090 | 2 | fail, same | **pass: recovers** |
| **logger `droop-bor3`** | 19,622,270 | 20,854,130 | 3 (U20: identity, configure, data) | fail, same | **pass: recovers** |
| **ext-bmp581 `droop-bor3`** | 19,623,760 | 20,855,540 | 2 | fail, same | **pass: recovers** |

**Every byte that moved, record by record (`compare.py`):**
- **Runs with no release** (every undisturbed boot, both `droop-tolerated`, `four-layer-droop`): no record differs. Their state hashes move because the trace version is in the header the chain starts from.
- **Each releasing Sentinel-01-shaped droop:** kind 32 is added, and kind 19 moves from release + 100 µs to release + nudge + bound. Its readings and witness lines are unchanged. **No Sentinel-01 verdict, count or reading moved.**
- **The four waiting runs:**
  - kind 32 is added;
  - kind 19 moves, its boot word read as `0x2100b007` (was `0x0`);
  - its witness holds the device's later steps where it has them;
  - kind 14 loses `brownout_unrecovered`.
- **So the Sentinel-01 bench package's `MANIFEST.json` moves** (the verdict's trace hashes). It joins the queue after 4240v. It is not hand-edited.
- **Measured, CI's bench-package step run locally** (`verify --against-fab fab/sentinel-01-bench/package`, the pinned image, 16 GiB, 139 s, peak 3.34 GB):
  - every gate passes;
  - all five bench scenarios match (`droop-bor3`: pass);
  - every fab precondition is met;
  - the one difference is `"MANIFEST.json differs from the derived file"`. **No other file of the package moves.**

## §1.5 again, after `4240v` merged (main 943f7112; `PREDICTIONS-3.md` committed first, `ba2d0017`)

The same two patched CLIs, main 943f7112 against the branch merged with it (`out/measure-main-943f7112.jsonl`, `out/measure-branch-after-4240v.jsonl`, `out/compare-main-943f7112-branch.txt`).
- **Recovered still equals undisturbed** to the instruction, on every board. **Met.**
- **Every verdict is unchanged from the first re-judge.** The four waiting runs fail on main and recover on the branch. Sentinel-01's records again differ only by kind 32 and kind 19's instant. **Met.**
- **The boot words moved by +850 to +1,210 ns** (S-01 minimal +920, bench +920, dshot600 +970, four-layer +970, spi-only +890, S-03 +1,210, S-02 shape +850, logger +880, ext-bmp581 +950). This is the VOS2 bring-up `4240v` runs before the clock. **"One Δ on every board" missed**: the shift depends on the board, though it is inside the predicted 0.1–5 µs. The bound absorbs it, being read from each board's own undisturbed boot.
- **Re-pinned:** `campaign_tier2`'s Sentinel-01 values: boot word 26,620 (was 25,700), bound 126,620, probe 1,358,320 (release 1,231,690 + 10 + 126,620).
- **The logger fixture is at spec 19,** with `mcu.core_supply` as the reference's (the same MCU, its VCAP-decoupled LDO), and validates. The spike scenarios now use it, and the spike's spec-18 copy is removed.

## §1.3, §1.4 The witness and coverage

- **§1.3 The witness (`chaos` `power::witness_recovers`, one rule for the CLI judge and for replay):** accepted as proposed.
  - The lines from the release must begin with the first device's identity lines.
  - Every later line must continue that device's own later steps, in the script's order, each step whole.
  - **A repeated identity read fails:** that is a boot that started over (a reset loop), not a recovery.
  - A wrong identity value still fails `brownout_unrecovered`, by its word and by its line.
  - An archive written before this build would still replay by this rule: an exact one-step witness is its first case.
- **§1.4 Coverage (`cli` `verify.rs` `uncrossed`):**
  - On the MCU's supply, at the specification's `mcu.bor_level` > 0, a droop below the band whose loaded floor stays at or above that level's falling threshold covers nothing.
  - Its row names the crossing it lacks, for example: ``rail `VDD_3V3` (`x` crosses none: at BOR level 3 a droop must cross VBOR3's falling level, 2610 mV; its loaded floor is 2850 mV)``.
  - The level is the board's, not the droop's field, so no scenario declares its way past it.
  - It covers nothing for the whole scenario, as an eased override does, so it fails closed.
  - The L697 loaded-floor test moved from Sentinel-01 (level 3) to `four-layer` (level 0), where the band alone decides.
  - **Sentinel-03:** its `droop-tolerated` no longer covers `VDD_3V3`. The new `droop-bor3` (ruled) does, and it recovers (§1.5), so Sentinel-03's coverage is whole again and its `verify --fab-out` refuses at the ledger's 280 rows, as before.

## §2 Mutants (measured; each applied as a text edit and restored by its inverse, never by `git checkout`)

| Mutant | Where | Killed by |
|---|---|---|
| A constant bound: `recovery_bound` returns 100 µs whatever the undisturbed boot took | `chaos` `power.rs` | tier 2: `bor_recovery::the_bmp581_extension_recovers_after_its_settle` and `the_sentinel_02_shape_recovers_after_its_imus_settle` (the campaign misses its expectation) |
| An exact one-step witness | `chaos` `power::witness_recovers` | tier 2: `bor_recovery::the_logger_recovers_with_its_first_devices_later_steps_witnessed`; unit: `analog::tests::a_probe_recovers_by_its_reads_and_its_first_devices_own_lines` |
| A wrong identity passes (only the boot word is compared) | `cli` `analog::holds` | unit: `a_probe_recovers_by_its_reads_and_its_first_devices_own_lines` |
| Coverage counts a droop that crosses no BOR threshold | `cli` `verify.rs` `stress` | unit: `verify::tests::a_droop_covers_the_mcu_rail_only_by_crossing_its_bor_level` |

## §5 Adversarial pass (full)

- **A board that never recovers but passes.**
  - *An identity line from before the droop:* the witness window opens at the release's sentinel. `probes_in` takes only the lines after it (`a_probe_is_read_only_between_its_own_whole_line_markers`: lines before the release give an empty witness, which `witness_recovers` never passes). The judge also re-arms the record words at each assertion.
  - *A boot word from a reset loop:* a loop that re-reads its first device before the probe repeats the identity lines, and `witness_recovers` refuses a repeated step (unit tests in `power.rs` and `analog.rs`).
  - **Residual, named:** a firmware that writes its boot word and then hangs, or that resets more slowly than one bound, can pass a probe. For the last boot the boot judge's end-of-run checks still apply (`pc` on a self-loop, thread mode); for an earlier release in a multi-reset schedule they do not. No committed droop renders more than one reset.
- **A bound so wide it passes anything.**
  - The margin is one quantum, pinned in `Record::check` (an archive with another margin is refused) and in a unit test.
  - Measured spread: 0 ns over 9 boards and 12 release phases. A recovered boot more than 100 µs slower than its own undisturbed boot fails.
  - **Residual, named (F3):** the bound is relative. A firmware whose undisturbed boot is itself slow (up to the run's 100 ms, less the release) gets a bound as long. `RecoveryBoundPastRun` refuses a probe past the run.
- **A baseline boot under a different firmware or scenario.**
  - The undisturbed boot is built by the same closure (`plan_with`) from the same `Ready` and booted platform, with only the resets, the bound and the capture directory changed.
  - A unit test holds its commands equal to the droop's outside the schedule (`the_undisturbed_boot_is_the_droops_own_without_its_resets`).
  - The run's header names the one firmware. Kind 32 does not repeat its hash: it is that run's, by construction.
- **A probe inside the next reset, or past the boot:** refused before the boot by `RecoveryBoundPastRun` (`main.rs` `recovery_fits`; unit test `a_probe_past_the_boot_or_into_the_next_reset_is_refused_by_name`: exactly at the end fits, 1 ns past is refused, a second reset before the probe is refused, an overflow is refused), and by replay ("a release's probe lies past the run").

## Gates

- **Host, on 752cd167 (main c9c5f85c merged):**
  - `cargo fmt --all -- --check`;
  - the bridge's fmt check;
  - `cargo clippy --workspace --all-targets -- -D warnings`, with and without `with-tools`;
  - `cargo test --workspace`: **2,355 passed, 0 failed** (141 result lines).
- **Tier 2, in the pinned image (16 GiB, swap 0, peak 6.0 GB):**
  - `twin_tier2`: `bor_recovery::*` (8.2, 9.6 and 13.6 s on Furnace), `battery_sag::sentinel_03_s_campaign_…` (126.2 s, with `droop-bor3`) and `the_dshot600_fixture_s_campaign_passes_inside_its_window`;
  - `campaign_tier2`'s two droop tests;
  - `board_tier2`'s `every_admitted_identifier_character_survives_every_output`;
  - `verify_47k_tier2` and `four_layer_fab_tier2`.
  - All pass. The rest of tier 2 is CI's.
- **CI on 771f6e6c (run 44688610112):** green except `bench-package`. tools-build, all 12 tools-shards and `tools` pass (`tier2_tests_passed=197`, floor 197), with gates, route, cli, foreign and the images. `bench-package` fails on `MANIFEST.json` only (`against_differs`); it waits on the architect's `write=true` dispatch.
- **The first CI run (ff894e30) failed every tools shard at `plan`:** `twin_tier2` is lock-serialised, so every test in it needs a weight row. The three rows were added (Furnace × 2.8), and `ci/shard.sh plan` passed in the image before the push.
- **`TIER2_MIN` 194 → 197:** the three `bor_recovery` tests. Each carries a weight row, as every test of a lock-serialised binary must (see above).
- **Reduction pass (diff), by a fresh reviewer:** nothing to remove · 0 duplicates removed · 0 dead paths removed · 1 kept by ruling · 2 findings below.
  - **Kept by ruling:** kind 32's `margin_ns` and `bound_ns` are determined by its instant, since `Record::check` forces both. The ruling of 05:10Z names all three for the archive. Dropping them would save about 25 lines and two fields; that is the architect's call.
  - **R1:** `recovery_bound` and `probe_ns` use checked arithmetic (`Option`) for a sum that cannot reach `u64` inside a 0.1 s run. Saturating adds would drop three error paths (about 15 lines). Kept, to match `power::schedule`'s checked style.
  - **R2:** `verify.rs` `uncrossed` and `stress`'s droop arm each look up the domain, band and loaded floor. A shared helper saves about 4 lines, but the two treat a floor the drop reaches (`rail_levels` `None`) differently. Kept.

## Findings

- **F1 · §1.4 left Sentinel-03's MCU rail unmet** (measured, by the rule; ruled 05:10Z: add `scenarios/sentinel-03/droop-bor3.json`).
  - Its one droop then, `droop-tolerated`, is at BOR level 3 with a loaded floor of 2.95 V, so it crosses nothing.
  - Until the new droop landed, `battery_sag_tier1` measured `CampaignCoverageShort` naming the crossing it lacks.
  - With `droop-bor3` (2.4 V, hold 600 µs, its expectation written before any run), the rail is covered by a droop that crosses VBOR3. It recovers at 40,871,280 ns under its 39,227,180 ns bound, and the test's assertion stands as before (the ledger's 280 rows).
- **F2 · The predictions' misses** (above): BOR level 0 renders no reset at 3.1 V; `spi-only`'s first check is SPI; the logger's first check is U20, not U10. The judge's first check is the image's first, and each board's differs.
- **F3 · Close to `INV-07`** (inferred). A bound read from the generated firmware's own undisturbed boot is computed from the artifact under test. It judges that a droop leaves the bring-up as it was, not that the bring-up is fast enough. What caps it is the margin and the boot judge's own checks on the undisturbed boot. No declared requirement bounds a bring-up's length today.
- **F4 · The probe and the first serial injection share an instant** (measured).
  - The bound is the undisturbed boot word + one quantum, and the composition injects a port's first frame byte at the recovered boot word + one quantum (`AET-WO-2302` D4).
  - On a board whose recovered boot equals its undisturbed one, both land on the same 10 ns. On Sentinel-01 minimal that is 1,357,400 ns, in kind 19 and as kind 29's injection instant.
  - Harmless as measured: the probe's monitor reads run at the `RunFor` boundary before the core runs the injection, and every serial witness on the re-judged boards is unchanged.
  - A later change to either bound or margin would separate them. Named so it is not a surprise.
- **F5 · The narrowed witness (§1.3) is exercised only where the first device is on I2C with later steps** (the logger's U20, Sentinel-03's U7). On `spi-only` the first device is the SPI flash, whose one step is all its lines. No committed board puts a scripted SPI device with later steps first. The rule is bus-blind (`witness_recovers` takes the steps' lines), so this is a residual of coverage, not of logic.
- **F6 · Documents outside the fence that this changes** (for the architect):
  - `docs/roadmap.md` §7.1, the row for `dshot600-droop` on the Sentinel-02 shape (`2303` F11, `4600` F1, `4202` F1): done by this work order.
  - `crates/aethelgard-cli/README.md` documents neither kind 32 nor `RecoveryBoundPastRun`.
  - I updated `crates/aethelgard-chaos/README.md`'s version rule (it still said 15, so `3305`'s 16 had not reached it), kind 19's row, a new kind 32 row and the replay rule. That is documentation of the record the ruling added. The table has never had a kind 31 row (`2304b`); I added none.
