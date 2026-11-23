# AET-WO-2309 §1.1 — results, measured after the predictions (`4a84ce75`)

Main is `28fba951`. The rules were built in the worktree as `patches/rules-full-reading.diff`, measured, then taken out of `crates/`. **This branch changes no byte of `crates/` or `fab/`.** The work order stops at §7 (FEEDBACK, "§7 stop").

## Sweep A: the committed archives

`sh/sweep-a.sh`, over `git ls-files '*/trace.cbor'`: four archives under `spike/3305/out/archives`.
- **Before** (main's CLI) and **after** (the branch's CLI, built with the full patch): byte-identical (`out/sweep-a-before.txt`, `out/sweep-a-after.txt`).
- Each is refused as `UnknownVersion`, "unknown format version 14 in header", exit 3. Today's reader is trace 18, so no record rule ever runs on them.

## Sweep B: current-version archives, made once by main's CLI

`sh/measure.sh` is 3306's groups unchanged. It ran in the tools image under `capped-run` (8 GiB, swap 0, read back), with main's CLI from a `git archive` of `28fba951`. It made 64 archives. `bme-bmp` and `bme-bmp-0x47` no longer generate on main (`generate bmp failed`): `spike/3306/spec/bme-bmp.json` is a 3306 spike specification, not a committed group.

`sh/sweep-b.sh` replays every archive with main's CLI and the branch's (full patch): **64 of 64 exit 0, byte-identical output** (`out/sweep-b.txt`).

`census/` (`census2309`, through `replay_dir` alone) describes each archive (`out/census-b.txt`):

| | Measured |
|---|---|
| Runs | 63 read and booted, 1 read and not booted (`seeded-no-boot-word`), **0 unread** |
| Kind 11 | 1,890 read, 0 unread |
| Kinds 12 and 21, value and status | 96 and 128 read, 0 unread |
| Kinds 13 and 22 | 96 and 128 read, 0 unread; none unread holding `absent` or `unrouted` |
| Kinds 24 and 26 | 15 runs hold 4 of each; 49 hold none. The unbooted run holds no motor output |
| Largest trace | 9,056 B |

**The flooded log, made for P8** (`sh/flood.sh`, in the image, main's CLI). One campaign run of the tier-2 flooding firmware (`tests/fixtures/flood`) on `sentinel-01-minimal`, expecting a pass, so it archives whatever the judge finds (`out/flood/`):
- read and booted;
- kinds 11, 12 and 21 all read;
- **kinds 13 and 22 unread** (the log passed the filter's budget).

It replays byte-identically under main's CLI and under the branch's full patch, exit 0 (`out/flood/replay.txt`). The archive is committed as `out/flood/7a624e30…`: the first trace-18 archive in the repository.

## Sweep C: every `check_run` input the tier-1 suite builds

`sh/sweep-instrument.diff`, applied to a copy of main only, wraps `replay_dir` and `check_run`. For each call it logs:
- the sha256 of its input;
- the test thread;
- main's result;
- what each candidate rule would say of the same records.

Nothing panics, so every fixture of every test is judged. The first run used v2 of the patch. The CLI tests spawn the CLI under `env_clear()`, so v2 missed the spawned CLI's own calls. v3 (the committed diff) logs those through a marker under `$HOME` and adds two candidates, and reran chaos, evolve and the four CLI binaries that spawn `campaign`, `replay`, `attribute` or `propose`: `out/sweep-c-v3.txt`. The rerun adds one full-reading fixture, `cli/tests/campaign_tier1.rs:2081`, a resealed run with `good()`'s shape. It measures `unlisted` (D3) at 0 fixtures and `motor_read` (D4) at 1 (`serial.rs:1750`). Its inputs: 860 read and booted, 323 read and not booted, 25 unread. `cargo test --workspace --no-fail-fast`, with the target inside the copy (a target outside it makes `fab_tier1`'s `BaseOutsideRoot` fail on main: an artefact of the sweep, not of main). `sh/sweep-c.py` reads the log. Its output is `out/sweep-c.txt`.

The candidates:
- **motor**: rule (2) as the work order says it;
- **full**: rule (3) with kinds 11, 12 and 21 (value and status) read exactly when kind 10 is, and kinds 13 and 22 unread with it;
- **status**: as full, but kinds 12 and 21 bound both ways by their status and their value one way. In `patches/rules-full-reading.diff` that is one line, `observed.is_some() == read && status.is_some() == read` replaced by `status.is_some() == read && (read || observed.is_none())`.

Main's suite under the instrumentation: **2,784 passed, 0 failed**. 1,229 distinct (input, test) pairs reached `check_run` (1,182 distinct inputs); 1,424 `replay_dir` calls. Full output: `out/sweep-c.txt`.

| Candidate | Refuses, of what main accepted | Refuses, of what main already refused |
|---|---|---|
| motor (rule 2) | **0** | 2 pairs in 1 test, already refused for other reasons ("names no solved output", "is there twice") |
| full (rule 3) | **43 pairs in 20 tests**: 19 in `chaos/tests/campaign.rs`, 1 in `evolve/tests/attribute.rs` | 116 pairs in 43 tests. Where the new rule runs first, these change their reason: on the branch the chaos suite fails 47 tests, not 20 |
| status (rule 3) | **6 pairs in 2 tests**: `a_boot_that_read_nothing_names_its_bound` (chaos, 5) and `a_register_with_nothing_observed_is_no_evidence` (evolve, 1) | 15 pairs in 2 tests |

The 20 tests whose accepted fixtures the full reading refuses:

| Test | File:line |
|---|---|
| `the_47k_shaped_run_matches_what_it_expects` | `aethelgard-chaos/tests/campaign.rs:1064` |
| `a_run_expecting_fewer_kinds_does_not_match` | `aethelgard-chaos/tests/campaign.rs:1073` |
| `a_run_expecting_more_kinds_does_not_match` | `aethelgard-chaos/tests/campaign.rs:1082` |
| `a_failing_run_expecting_to_pass_does_not_match` | `aethelgard-chaos/tests/campaign.rs:1089` |
| `info_findings_never_count_toward_a_match` | `aethelgard-chaos/tests/campaign.rs:1098` |
| `a_held_low_run_is_ok_and_matches` | `aethelgard-chaos/tests/campaign.rs:1273` |
| `the_good_run_carries_one_solved_bus_and_passes_the_joins` | `aethelgard-chaos/tests/campaign.rs:1353` |
| `two_solved_records_for_one_bus_or_unsorted_buses_are_refused` | `aethelgard-chaos/tests/campaign.rs:1367` |
| `a_witness_must_name_a_solved_device_on_its_own_controller` | `aethelgard-chaos/tests/campaign.rs:1442` |
| `two_devices_on_one_controller_have_two_witnesses` | `aethelgard-chaos/tests/campaign.rs:2265` |
| `pc_outside_firmware_is_exactly_a_pc_outside_the_archived_segments` | `aethelgard-chaos/tests/campaign.rs:2355` |
| `the_boot_read_is_whole_and_the_pc_findings_are_the_judges` | `aethelgard-chaos/tests/campaign.rs:2447` |
| `a_device_with_three_steps_round_trips_and_replays` | `aethelgard-chaos/tests/campaign.rs:2585` |
| `several_configurations_each_by_its_own_name_replay_in_order` | `aethelgard-chaos/tests/campaign.rs:2657` |
| `the_brown_out_findings_are_exactly_what_kind_10_shows` | `aethelgard-chaos/tests/campaign.rs:2893` |
| `a_line_bound_only_by_another_bus_or_another_line_is_refused` | `aethelgard-chaos/tests/campaign.rs:3037` |
| `a_boot_that_read_nothing_names_its_bound` | `aethelgard-chaos/tests/campaign.rs:3073` |
| `boot_word_missing_is_named_exactly_when_the_word_is_wrong` | `aethelgard-chaos/tests/campaign.rs:3142` |
| `a_record_judged_without_the_boot_word_is_refused` | `aethelgard-chaos/tests/campaign.rs:3189` |
| `a_register_with_nothing_observed_is_no_evidence` | `aethelgard-evolve/tests/attribute.rs:1581` |


## Predictions against results

| # | Prediction | Result | |
|---|---|---|---|
| P1 | Sweep A vacuous, identical | four `UnknownVersion`, identical | held |
| P2 | Sweep B 60–80 archives, all identical | 64, all identical | held |
| P3 | Read boots read every 11, 12, 21; unread boots none | read half: held on 64 runs. Unread half: no unread archive in sweep B; measured on the 25 unread tier-1 inputs of sweep C's rerun (the status reading refuses only `unread()` among them) | held, unread half on fixtures only |
| P4 | Unread witnesses only with an unread boot, or past the budget | held: none in sweep B; the flooded run is the one | held |
| P5 | One kind 26 per kind 24 in every read run | held on the 15 booted motor runs; **no read-unbooted run holds a motor output**, so the unbooted half is unmeasured | half measured |
| P6 | Largest trace under 64 KiB | 9,056 B | held |
| P7 | Sweep C: every main outcome unchanged | **missed**: the §7 stop (FEEDBACK §2) | missed |
| P8 | The flooded read boot replays after | measured, replays identically | held |
| P9 | The writer's 256 MiB test under 10 s | not measured: no test was written before the stop | open |
| P10 | One mutant each, killed by name | not run: the stop came first | open |
