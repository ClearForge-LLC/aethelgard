# AET-WO-0300d — FEEDBACK

**Branch:** `wo/aet-0300d-guardian-retry` · **Base:** `1203c22` (PR #38) · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only; no §7 condition arose.

Entry schema: `finding · where · severity · type · recommendation · decision-needed`.

## What was built

- **§1.2, the retry, in `guard_main` only** (`crates/aethelgard-cli/src/run.rs`). Once the CLI is gone: `remove_dir_all`; success, or `NotFound` with the directory really absent, exits 0; any other error (including `NotFound` while the directory still exists) sleeps 20 ms and retries; after `GUARD_REMOVAL_BOUND` (10 s) it exits 70 and leaves the directory. The guardian still sets no death signal and still refuses a path not ending in its own `aethelgard-*` name.
- **§1.1/§1.3 tests,** both in `lifecycle_tier1.rs`:
  - `a_guardian_removes_its_directory_though_a_writer_races_it`: 10 tries; a writer thread keeps recreating four files in the directory for 500 ms while a guardian whose CLI is already gone (a dead pid is passed) removes it; all 10 must leave nothing, exit 0.
  - `a_guardian_that_cannot_empty_its_directory_exits_70_within_11_seconds`: the writer never stops; exit 70 between 9 s and 11 s, directory left, then the test cleans up.

## Reproduction counts (§1.1, §3.2)

- **On `main` (1203c22, run.rs reverted, final test):** directories left behind **10 of 10** tries, every guardian exit 0. The never-stopping writer: guardian exit 0, not 70. (An earlier writer that removed its own files left 9 of 10.)
- **With the fix:** **0 of 10** left, all exit 0; the bound test passes (3 consecutive runs of the lifecycle guardian tests, 10 s each).
- **Was the CI failure's cause confirmed?** The *mechanism* is confirmed: one unretried `remove_dir_all` loses to a concurrent writer and leaves the directory. That the CI failures were that writer (trampoline status file or tool stub) rather than something else is **not** observed directly; the two CI runs themselves were not reproduced, only the mechanism with a synthetic writer. Ten fixed-build runs showed none.

## Acceptance

- **§1.3 on Furnace:** `servers_sigkilled_while_calls_start_leave_nothing` and `an_immediate_cancel_storm_leaves_no_process_and_no_directory` ran **10 times in a row**, each in a capped cgroup: 20 of 20 passed, every run `survivors=0 leftover_dirs=0` (storm `calls=500`, parent-kill `servers=100 burst=5`).
- `cargo test --workspace`: all pass. Tier 2 / `tools` has no image on Furnace; it runs in CI's `tools` job. CI on first push: see the PR.

## Adversarial pass (§5, targeted)

1. A symlink named `aethelgard-link` pointing at a directory holding a file: the guardian removed the link only, the target and its file survived (std's `remove_dir_all` unlinks a top-level symlink). A path not ending in `aethelgard-*` (the target directory itself): exit 64, untouched. A swap between tries has the same behaviour on each try.
2. Spin or false exit 0: bounded at 10 s by an `Instant` deadline, proven by the exit-70 test (9–11 s). Exit 0 requires `Ok` or `NotFound` with `!dir.exists()`; `NotFound` with the directory present retries.

Not tested: a swap in the instant between `remove_dir_all`'s own checks (std's concern, not this loop's).

## Reduction pass (§5a)

Gates: diff-sized reduction pass on the finished diff: one loop and one constant in `guard_main`, two tests, one writer helper; nothing removable without losing a §1 item. The first writer helper (create/remove pair) was cut for the simpler recreate loop.

## Backlog

- None required. The README's description of exit 70 for the guardian is not updated here (protected surface); the architect may add "exit 70 after 10 s with the directory left" to architecture §6.
