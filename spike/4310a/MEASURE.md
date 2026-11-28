# AET-WO-4310a: gates' `cargo test` time, measured

2026-11-28, on `main` `ef8c46f5`. Each binary's libtest `finished in` time, from the gates job's own log at each head, is in `ci-binaries.tsv`. The step's wall time is from the job's steps.

## The step's wall time on `main` and the slice D branch

| head | what | gates `cargo test` | binaries' sum |
|---|---|---|---|
| `649e81a8`–`20971352` (2026-11-26) | before 4411 | 23–30 min | 1,164 s (`20971352`) |
| `ba49e146` | 4411 merged | 61 min | 3,349 s |
| `2c319305`–`eb8bb31a` | `main` after 4411 (4412, 1919, 1802, docs) | 50–81 min | 2,817–2,923 s |
| `86a1f280` | 4310's head, before 4413 | 51 min | 2,801 s |
| `808766ea` | 4310 + 4413 | 104 min | 5,854 s |

## Answers

- **The growth is 4411's, not 4310's, 4413's or the version-32 regeneration's.**
  - 4310's head adds nothing over `main` just before it (2,801 s against 2,923 s).
  - The binaries that grew from `20971352` to `ba49e146` are each a run of Sentinel-03's `emit`, which 4411 made heavy (the radio module's group, the pour ladder):
    - `pour_ladder_tier1`, new, 707–926 s;
    - `radio_tier1`, new, 169–246 s;
    - `emit_tier1`, `hot_plug_tier1`, `readiness_tier1`, `battery_sag_tier1` and `pin_device_refusals`, each about 51–82 s → about 180–280 s.
  - Each such test emits Sentinel-03 once in its binary, and gates runs every binary serially on one runner.
- **`808766ea`'s 104 min is the runner.**
  - Every CPU-bound binary ran 1.5–2.7× slower than at `86a1f280`, among them binaries none of these work orders touched: `aethelgard-chaos`'s `bounds` (8.1 → 16.8 s) and the twin's unit tests (1.4 → 3.0 s).
  - The wall-clock-bound binaries are unchanged: `boot_tier1` (206.5 → 216.7 s), `campaign_tier1` and `doctor_tier1` (their scaled hang guards), and `lifecycle_tier1` (5.5 s at every head).
  - 2,801 s × 2.07 = 5,798 s, against the measured 5,854 s.
  - The tools shard of each run names its CPU: AMD EPYC 9V74 at `86a1f280`, Intel Xeon 6975P-C at `808766ea`. The gates job prints none, so the ratio is the evidence.
- **No hang.** The step finished green. No test ran past its own binary's normal share; the long ones are the Sentinel-03 emits and the hang guards above.

## The fix: routing (`ci/tools-only.tsv`, AET-WO-1910 D5 (a)'s condition)

The tools shards already run every test target, weighed. Gates runs these four a second time, serially, on one runner.

- **The four that move:** `pour_ladder_tier1`, `hot_plug_tier1`, `pin_device_refusals` and `readiness_tier1`.
  - **No sandbox.** Each runs only `emit` and the library: no `verify`, `check`, scratch or sandbox (a spawn of `emit` alone, or none).
  - **No libm site of its own.** The census of every non-test source file finds exactly 1910's five sites (`supply.rs:117`, `:122`, `:128`, `:859`, formerly `:838`, and `flow.rs:1771`), all still pinned in gates by `supply_libm`'s golden. `flow.rs`'s `ln` and `atan` are the crate's interval series.
- **The three that stay in both, by D5 (a)'s condition:**
  - `battery_sag_tier1` and `emit_tier1` run `verify`;
  - `radio_tier1` runs `check drc` with no `PATH`, and its refusal's order against the CLI's scratch is not shown.
- **Predicted:** gates' binaries fall by 1,284 s at `86a1f280`'s speed (707 + 214 + 183 + 180), from 2,801 to about 1,517 s (25 min). The tools shards are unchanged, since the four are already in their plan.

## Before and after, measured on Furnace as CI runs gates

`ci/gates.sh run`, inherited descriptors closed as ci.yml closes them, `RUST_TEST_THREADS=2`, both runs in one target directory (the after run built only what the tree changed, so its wall includes less build: the binaries' sum is the like-for-like figure).

| tree | gates wall | binaries' sum | binaries | failed |
|---|---|---|---|---|
| `808766ea` (before) | 2,313 s (38.5 min) | 2,173 s | 161 | 0 |
| `fb835f44` (after) | 1,521 s (25.4 min) | 1,386 s | 157 | 0 |

- −787 s of binaries: the four that moved (361 + 149 + 137 + 136 s), and nothing else.
- **On CI** at `86a1f280`'s runner speed, the same four were 1,284 s of its 2,801, so gates' binaries are predicted at about 1,517 s (about 25 min of the step's 51).
- **This PR's own `gates` run is the after** on CI. Its time is in the PR's FEEDBACK once it runs.
