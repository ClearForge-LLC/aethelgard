# AET-WO-0300c — FEEDBACK

**Branch:** `wo/aet-0300c-reaper-race` · **Base:** `c923cd3` (the merge of PR #30) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (kernel 7.0.0-38) · **Cadence:** build-only; no §7 condition arose.

Entry schema: `finding · where · severity · type · recommendation · decision-needed`.

## What was built

- **§1.1, the CLI passes its pid.** `Invocation::run` adds `--cli-pid <getpid>` to the reaper's arguments; `Scratch::new` adds it to the guardian's. `parse_internal` (shared by the reaper and the trampoline) requires it exactly once. `pid_arg` accepts only what this binary writes: digits, no sign, no leading zero, at most `i32::MAX`. Anything else exits 64. The reaper exits 70 before `set_child_subreaper` and before it spawns the trampoline when `parent_id()` already differs. Its loop compares against the passed pid; the sampled `parent` is gone from both the reaper and the guardian.
- **§1.2, the guardian before its directory.** `Scratch::new` chooses the path, spawns the guardian (`--aethelgard-internal-guard --cli-pid N DIR`), and only then creates the directory. If creation fails, it kills and waits the guardian, so that a directory already at that path (`EEXIST`) is never removed after the CLI exits. The guardian's loop is the ruled one, checking the parent first:
  - parent gone: remove the directory if it exists, exit 0;
  - directory present: mark it seen;
  - seen and now gone: exit 0;
  - otherwise wait.
- **§1.3, built, and narrowed by the ruling on PR #34 (A2),** through `rustix::process::set_parent_process_death_signal(Some(Signal::KILL))` (rustix 1.1.5, the `process` feature already enabled; no `Cargo.toml` change, no `unsafe`).
  - `mcp` puts its pid in `AETHELGARD_INTERNAL_PARENT_PID` on each CLI it starts (A1).
  - **Only when that variable is set**, and never for `aethelgard mcp` itself, `main` arms the signal after the command line parses. It then exits 70 if `getppid()` differs from the variable, which closes the window in which a parent died before the signal was armed (A1).
  - A malformed value is `UsageError`, 64; a failed `prctl` is exit 70 with a diagnostic.
  - A CLI run without the variable behaves exactly as before this WO (B2).
  - The reaper, guardian and trampoline are dispatched before this point and started with `env_clear`, so they never arm it, and the variable never reaches a tool.
  - The first build armed it unconditionally, before the parse, as §1.3 first ruled. The adversarial findings and mutants below refer to that build where they say `3129246` or `a7fb0dc`.
- **§1.4:** `mcp` handles `SIGHUP` like `SIGTERM`/`SIGINT`, in both `select!`s (before and after `initialize`), exit 129.
- **README:** the reaper/guardian lifecycle; the parent-death signal, only under `mcp`, and why; exit 70's new cases; `SIGHUP` 129; `SIGKILL` behaviour.

### Tests (§1.5), all in tier 1

| test | file | proves |
|---|---|---|
| `an_immediate_cancel_storm_leaves_no_process_and_no_directory` | `mcp_tier1.rs` | 500 `tools/call`, each cancelled at once or after up to 12 ms. Every descendant recorded from `/proc` (pid **and** start time, so a reused pid is never a survivor) and every process naming the test's `TMPDIR` is gone, and no `aethelgard-*` directory is left. Prints `survivors=` |
| `servers_sigkilled_while_calls_start_leave_nothing` | `mcp_tier1.rs` | Added for A1: 100 servers, each SIGKILLed 0–9.5 ms after a burst of 5 calls; nothing left |
| `sighup_during_a_call_leaves_nothing` | `mcp_tier1.rs` | §1.4: exit 129, every descendant gone |
| `sigkill_of_the_server_during_a_call_leaves_nothing` | `mcp_tier1.rs` | §1.3 with the tool running |
| `a_reaper_started_with_a_dead_clis_pid_exits_70_and_spawns_nothing` | `lifecycle_tier1.rs` | Exit 70, no status file, no tool. "Spawns nothing" is read from the zombie reaper's `cminflt` (the page faults of children it waited for: 0 exactly when it started none), because the loop would otherwise kill a started trampoline before it wrote anything. Control: a live pid runs the tool and `cminflt > 0` |
| `a_missing_or_malformed_cli_pid_is_refused_with_64` | `lifecycle_tier1.rs` | Reaper and guardian; includes `01`, a repeated `--cli-pid`, and `…/aethelgard-link/`, `…/.` and `…/..` (a linked victim stays intact) |
| `a_guardian_started_before_its_directory_removes_it_when_its_parent_dies` | `lifecycle_tier1.rs` | Shell parent: guardian started, directory made 0.3 s later, parent `kill -9`; directory removed, guardian gone |
| `a_guardian_waits_for_its_directory_and_exits_0_when_its_parent_removes_it` | `lifecycle_tier1.rs` | Still running before the directory exists and while it exists; exit 0 once it is removed |
| `a_cli_told_a_dead_parents_pid_exits_70_and_starts_nothing` | `lifecycle_tier1.rs` | Added for A1: exit 70, `cminflt` 0, no directory, no tool; malformed values are a 64 `UsageError`; the right pid passes |
| `a_cli_without_the_variable_outlives_the_thread_that_started_it` | `lifecycle_tier1.rs` | The ruling's test: a `check drc` CLI spawned without the variable from a thread that lingers 300 ms and then ends. It is still running after the thread ends, and completes with exit 0 and a `pass` verdict |
| `the_server_outlives_the_thread_that_started_it` | `mcp_tier1.rs` | `mcp` started with the variable set, from a thread that ends after 300 ms: still alive, and it answers `initialize` (it never arms the signal on itself) |

Every existing tier-1 test passes unchanged; `tests/common/mod.rs` gained `Scratch` and `stat_fields` (see the reduction pass).

## Storm survivor counts (§3.2)

| run | code | calls | descendants seen | survivors | leftover dirs |
|---|---|---|---|---|---|
| red, before the fix | `c923cd3` + the test | 500 | 1431 | **8** (guardians) | **8** |
| 1 | `3129246` | 500 | 1476 | 0 | 0 |
| 2 | `3129246` | 500 | 1424 | 0 | 0 |
| 3 | `3129246` | 500 | 1450 | 0 | 0 |
| 1 | `a7fb0dc` | 500 | 1413 | 0 | 0 |
| 2 | `a7fb0dc` | 500 | 1435 | 0 | 0 |
| 3 | `a7fb0dc` | 500 | 1432 | 0 | 0 |
| **1** | **`022eb17` (the ruled change; code at the PR head)** | 500 | 1484 | **0** | **0** |
| **2** | **`022eb17`** | 500 | 1467 | **0** | **0** |
| **3** | **`022eb17`** | 500 | 1485 | **0** | **0** |

The adversarial subagent also ran the storm three times at `3129246` (0, 0, 0; seen 1497, 1480, 1494), a 2,000-call variant twice in parallel (0, 0), and a variant ending in SIGKILL 43 times (all 0).

The parent-kill test (A1): red at `3129246` gave **52 survivors and 13 directories**. It gave 0/0 in three consecutive runs at `a7fb0dc`, and again in three consecutive runs at `022eb17`.

## Rulings (architect, PR #34, 2026-11-07)

- **A2: changed as required** in `022eb17`. The signal is armed only with the variable set, never in `mcp`; the test asked for is added. The A1 check stays as built.
- **B2: accepted for P0.** A direct caller owns its child's lifetime, as `RUL-13` rules for timeouts; `architecture.md` §6 will say so at close-out.
- **A1: the variable is accepted;** the architect names it in `architecture.md` §6 at close-out.
- **A3** accepted as a same-uid residual. **B1:** the WO wording is corrected at close-out. **B3** accepted as a test limit.

## Findings

Adversarial pass: a fresh Opus subagent, full, §5, in its own worktree at `3129246`. Its harnesses are kept under that worktree's `adv-scratch/h/`.

| # | finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|---|
| A1 | **The parent-death signal was armed too late.** A parent (`mcp`) killed after spawning the CLI but before the CLI's `prctl` sends no signal, and the whole call keeps running. Measured by the subagent: **49 of 550** random `mcp` SIGKILLs leaked (372 processes, 93 directories). Pausing at `cli_before_prctl` leaked 10/10; at `cli_after_prctl`, 0/10. SIGUSR1 and SIGALRM to `mcp` leak the same way. The storm never kills `mcp`, so it could not see this. | `main.rs` (prctl), `mcp.rs` (spawn) | high | bug | **Fixed** in `a7fb0dc`: `mcp` passes its pid in `AETHELGARD_INTERNAL_PARENT_PID` and the CLI checks it once the signal is armed. Tier-1 test red 52 survivors before, 0 after. This puts one `.env(…)` line in `mcp.rs`'s spawn, outside the literal fence ("`src/mcp.rs`'s signal handling"), and introduces an internal variable name | ruled: accepted; §6 names it at close-out |
| A2 | **`aethelgard mcp` sets the parent-death signal on itself too**, as §1.3 rules ("before any subcommand"). Linux fires it when the parent **thread** exits. Measured: `mcp` spawned from a thread that exits while its process lives on was SIGKILLed 10/10; a CLI started that way, 30/30; `sh -c 'nohup aethelgard … & sleep 0.1'`, 30/30 killed, so `nohup` no longer protects it. Under `mcp`, every CLI is spawned from `mcp`'s main thread (strace, including after more than 12 s idle; 90 long calls, none killed spuriously). `mcp` already ends its calls when its client goes, through stdin EOF. | `main.rs` | medium | behaviour change | Recommended skipping the `prctl` for `Command::Mcp`. **Ruled: armed only with the variable set, never in `mcp`; done in `022eb17`** | ruled |
| A3 | `EEXIST` window. If the CLI dies between spawning the guardian and killing it after a failed `mkdir`, the guardian removes the pre-existing directory. Instrumented `force_eexist`, CLI killed at that point: 20/20 removed. Reachable only through a same-uid collision on `aethelgard-<pid>-<nanos>`, or a same-uid process creating that exact path first (which could equally just delete it) | `main.rs` `Scratch::new` | low | residual | Accept. Otherwise the guardian could remove only an inode the CLI reports after its `mkdir`, at the cost of a channel and a mechanism | ruled: accepted |
| A4 | The guardian's name check was fooled by a trailing `/` or `/.`. `Path::file_name` reads past them, and `remove_dir_all("…/aethelgard-link/")` follows the link: a crafted direct invocation emptied a victim directory. A sandboxed tool cannot reach it (planted symlinks: victim intact 11/11) | `run.rs` `guard_main` | low | bug | **Fixed**: such a path exits 64; tested with a linked victim | no |
| A5 | `pid_arg` accepted leading zeros; a repeated `--cli-pid` was accepted and the last one won | `run.rs` | info | hardening | **Fixed**: both exit 64; tested | no |
| B1 | The work order places `Scratch::new` in `run.rs` and fences `main.rs` to "its first lines". It is at `main.rs:220`, and §1.1–1.2 require editing it. It was edited in place | WO §1/§2 | info | WO wording | Fix the wording in a later WO | ruled: corrected at close-out |
| B2 | **Residual for callers other than `mcp`:** a parent that dies within the CLI's first instant (before the `prctl`) and does not set the variable leaves that call to run to its end, its tool unbounded (`RUL-13`: the caller owned the timeout). The subagent measured a shell parent killed 0–4 ms after start at `3129246`: 43 of 200 leaked a call. The variable closes this only for `mcp` | `main.rs` | medium | residual | Decide whether direct callers get the same channel, or the residual is accepted for P0. Since `022eb17`, a direct CLI arms nothing, as before this WO | ruled: accepted for P0 |
| B3 | Mutant M2 (the reaper's loop compares against a sample taken after the start check) **survives**: a sample can differ from the passed pid only in a window of microseconds after the start check passes, which no storm hit. The code follows the ruling ("never against a sample"); it is kept | `run.rs` | info | test limit | None | ruled: accepted |
| B4 | Five `/tmp/aethelgard-*` directories from 21:24 UTC predate this session's runs. They are most likely from the other builder in `~/aethelgard` running the unfixed CLI: finding 2 of AET-WO-1500, still live on `main` until this merges. Left untouched | host | info | observation | They go away when this merges, or by hand | no |

Not broken (subagent, *ran*):
- SIGKILL of the CLI swept 0–45 ms in steps of 0.06–0.1 ms: 11,319 kills, **0 survivors**, cleanup at most 57 ms.
- 17 injected pause points, from before the `prctl` to `confine_before_exec`, 25 kills each: 0 survivors at every point.
- `mcp` SIGKILLed with tools already running: 0 in 40 iterations. Killing `mcp`'s whole process group: 0 in 140.
- SIGHUP, SIGTERM and SIGINT during bursts: 0 in 530; SIGHUP before `initialize` exits 129, 60/60.
- `--cli-pid` fuzz: every malformed value exits 64, for both the reaper and the guardian.
- A tool planting symlinks or flooding its directory with files: victim intact, 0 leftovers.
- Landlock, seccomp and the descriptor check are untouched (grep of the diff).

### Mutants (one per guard this WO adds; final code)

| mutant | killed by | result |
|---|---|---|
| M1 reaper start check removed | dead-CLI reaper test (`cminflt`) | killed 3/3 (survived before `cminflt` was added; that is why it was added) |
| M2 reaper loop vs a sample | storm | **survived** 0/3 (B3) |
| M3 guardian vs a sample | storm | killed 3/3 |
| M4 guardian exits while its directory is absent | guardian tests | killed |
| M5 directory before guardian | storm 1/3 at `3129246`, 0/3 at `a7fb0dc`; parent-kill test 3/3 (3–6 unguarded directories each) | killed |
| M6 no `PR_SET_PDEATHSIG` | SIGKILL-of-server test | killed |
| M7 no SIGHUP arm mid-call | SIGHUP test | killed |
| M8 `pid_arg` digits check removed | malformed-pid test | killed |
| M9 no parent-pid check | parent-kill test; dead-parent CLI test | killed, both |
| M10 trailing `/` and `/.` check removed | malformed-pid test | killed |
| M11 leading zero accepted | malformed-pid test | killed |
| M12 repeated `--cli-pid` accepted | malformed-pid test | killed |
| M13 signal armed unconditionally (the pre-ruling behaviour) | thread-exit CLI test; thread-exit server test | killed, both. They first **survived**: the thread ended before the child had armed anything (A1's own race). The spawning thread now lingers 300 ms |
| M14 `mcp` arms the signal on itself when the variable is set | thread-exit server test | killed |
| M6 and M9 rerun at `022eb17` (the code moved after the parse) | as above | killed |

The `EEXIST` kill in `Scratch::new` has no mutant: no test can reach `EEXIST` without a pid and nanosecond collision (A3).

## Gates

- **Reduction pass (diff):** `Reduction pass (diff): about −28 net · 3 duplicates removed · 2 dead paths removed · 0 protected kept · 1 findings below`. A fresh subagent proposed the cuts read-only; they were applied after the adversarial fixes, and the full mutant set was rerun afterwards:
  - one `Scratch` in `tests/common`, replacing `lifecycle_tier1`'s `Dir` and `mcp_tier1`'s copy;
  - one `stat_fields` parse, replacing the copies in `alive`, `start_time` and the `cminflt` read;
  - `pid_arg`'s dead `is_empty` clause and `p > 0` filter (both now subsumed by the leading-zero rule);
  - a duplicate guardian `"0"` row.

  Kept, as a finding: the "never sets a death signal on itself" sentences on `reaper_main` and `guard_main`. They are the only warning at the site a future edit would break.
- **Local on Furnace at `022eb17`:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (also with `--features aethelgard-cli/with-tools`), `cargo test --workspace`: green, 304 tests. `lifecycle_tier1` 6, `mcp_tier1` 21. At `a7fb0dc` it was 302.
- **Tier 2 did not run on Furnace:** `kicad-cli`, `cargo-kani` and `renode` are not installed here. `with_tools.rs` is unchanged, and compiles and lints clean with `with-tools`. It ran in the CI `tools` job (below).
- **Protected surfaces:** `git diff c923cd3 --name-only` lists only `crates/aethelgard-cli/{README.md, src/main.rs, src/mcp.rs, src/run.rs, tests/common/mod.rs, tests/lifecycle_tier1.rs, tests/mcp_tier1.rs}` and this file. Inside `mcp.rs`, the one `.env` line beyond the signal handling is A1. No `unsafe`, no `Cargo.toml`/`Cargo.lock` change, no secret, no sudo.
- **CI, first push** (`f7a9749`, run `42422903937`): `gates` **pass**, `tools` **pass** (tier 1 and tier 2 inside the image). §3.3 holds. For the push carrying `022eb17`, see the PR.

## Promote-or-needs-none pass

- `AET-WO-1500.FEEDBACK.md`:
  - findings 1, 2, 3 and 8a are **built here**;
  - 6 and 7 remain accepted (§4);
  - 3's "SIGQUIT and SIGUSR1": `SIGQUIT` and `SIGUSR1` (and `SIGALRM`) still kill `mcp` unhandled. Since A1, its CLIs then die by the parent-death signal, with no leak (measured before A1's fix: 12/150 leaked; the parent-kill test covers the mechanism, not each signal). Needs none.

## Not built (§4 held)

Concurrency limits, back-pressure, anything in Landlock or seccomp. Every finding that needed a decision has been ruled (above).
