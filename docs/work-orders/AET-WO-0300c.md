# AET-WO-0300c — Close the reaper's start-up race: no tool outlives a CLI killed at any instant

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1500` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0300c-reaper-race`.
**Author:** Claude (architect) · **Builder:** a **strong** builder, with a **full** adversarial pass (`docs/roadmap.md` §6: the sandbox is a trust boundary). A fresh session on Furnace.
**Phase:** P0, an inserted refinement of `AET-WO-0300`'s sandbox, found after the P0 gate passed. **It must merge before `AET-WO-1501` publishes the binary, and before any release.**

**Grounds:**
- `docs/work-orders/AET-WO-1500.FEEDBACK.md`, adversarial findings 1, 2, 3 and 8a, ruled at review of PR #29. Their measurement: 17 of 2,500 immediate MCP cancels left a tool and its reaper running under PID 1; 30 guardians and 37 scratch directories leaked; and 7 directories had no guardian at all.
- `docs/architecture.md` §6: a reaper in its own process group, registered as a child subreaper, kills the tool's group and every descendant when the CLI dies by any signal. A guardian removes the temporary directory. This work order makes "when the CLI dies" true at every instant.
- `docs/northstar.md` §4: `INV-06`.
- The seams, in `crates/aethelgard-cli/src/run.rs`:
  - `Invocation::run`, which spawns the reaper;
  - `reaper_main`, which samples `parent_id()` at its own start;
  - `guard_main`, which does the same;
  - `Scratch::new`, which creates the directory, then spawns the guardian.

  And in `crates/aethelgard-cli/src/mcp.rs`, the signal handling.

> **What this is:** the reaper and guardian learn their CLI's pid from the CLI itself, never by sampling. A CLI that dies before they look is detected at once. The guardian is in place before the directory it guards exists. The MCP server also handles SIGHUP.
>
> It is NOT:
> - a change to Landlock, seccomp or the descriptor check;
> - a new subcommand;
> - a change to any verdict.

**Cadence:** build-only. The design is ruled below; stop only on §7. One PR, left unmerged.

## 1. Scope, with the rulings

1. **The CLI passes its own pid.**
   - `Invocation::run` adds `--cli-pid <getpid()>` to the reaper's internal arguments. `Scratch::new` adds the same to the guardian's.
   - **The reaper:** if `parent_id()` already differs from that pid when it starts, the CLI is gone. It exits 70 before it spawns the trampoline. Otherwise its loop compares `parent_id()` against that pid, never against a sample.
   - **The guardian:** if its parent is already gone at its first check, it removes the directory if it exists, then exits.
   - The internal arguments are still outside the command tree (`architecture.md` §6's `argv` rule), so the parse refuses a missing or malformed pid with exit 64.
2. **The guardian starts before the directory exists** (finding 2's seven unguarded directories).
   - `Scratch::new` chooses the path, spawns the guardian with it, and only then creates the directory.
   - The guardian's loop becomes:
     - **the parent has gone:** remove the directory if it exists, then exit 0;
     - **the directory exists:** remember that it has been seen;
     - **the directory was seen and is now gone:** exit 0, because the CLI cleaned up;
     - **the directory was never seen and the parent is alive:** keep waiting.
3. **The CLI dies with its parent, where `rustix` offers a safe setter.** At the very start of `main`, before any subcommand, the CLI sets its parent-death signal to SIGKILL. That is `prctl(PR_SET_PDEATHSIG)`, through `rustix::process` if a safe function exists there.
   - When `mcp`, or any parent, dies by any signal, SIGKILL included, the CLI dies too. Item 1 then makes its reaper and guardian clean up.
   - If no safe setter exists, record that in FEEDBACK and skip this item. It is not a §7 stop.
   - The reaper and guardian must **never** set a death signal on themselves. They have to outlive the CLI in order to clean up after it.
4. **`mcp` handles SIGHUP** exactly as it handles SIGTERM and SIGINT (finding 3).
5. **Tests:**
   - an **immediate-cancel storm** in tier 1, with the stub tool. At least 500 `tools/call` requests are each cancelled at once. Every descendant PID recorded from `/proc` is gone by a deadline, and no `aethelgard-*` directory is left in the test's `TMPDIR`. This is finding 8a's test, which could not pass before the fix;
   - **a reaper started with a dead CLI's pid** exits 70 and spawns nothing;
   - **a guardian started before its directory exists** removes it when its parent dies after creating it, and exits cleanly when its parent removes it;
   - **SIGHUP** to `mcp` during a call leaves no descendant;
   - **every existing tier-1 and tier-2 test** passes unchanged.

## 2. Invariants and fences

- **`INV-06` and `architecture.md` §6:** no tool process outlives its call, at any instant of a kill.
- **No `unsafe` in any workspace crate.** `pre_exec` is out, because it is `unsafe`.
- **Protected surfaces diff to empty:**
  - everything outside `crates/aethelgard-cli/src/run.rs`, `src/main.rs`'s first lines (item 3), `src/mcp.rs`'s signal handling, the CLI's tests, its README, and this work order's FEEDBACK;
  - Landlock rules, the seccomp filter and the descriptor check do not change.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. §1.5's tests pass on Furnace and in `gates`. Tier 2 passes in `tools`.
2. The storm test passes 3 times in a row on Furnace, and FEEDBACK records each run's survivor count, which must be 0.
3. CI is green on the first push.

## 4. Scope fence

Concurrency limits for `mcp` (finding 7, accepted); back-pressure on an unread client (finding 6, accepted for a local stdio server); anything in Landlock or seccomp.

## 5. Adversarial pass — full, on a strong model

1. Kill the CLI with SIGKILL at every point from `main`'s first line to the tool's `exec`, using sleeps injected under a test-only feature if needed. Look for any survivor.
2. Kill `mcp` itself with SIGKILL mid-call. Item 3 should leave nothing behind; if it is skipped, record what survives.
3. Make the guardian remove something it should not: a directory a different call owns, or a path that is not `aethelgard-*`.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0300c.FEEDBACK.md`, same schema. It carries:
- the survivor counts;
- whether item 3 was built, and through which function;
- what the adversarial pass reached;
- the gates line.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A fix that needs `unsafe`, or a change to Landlock, seccomp or the descriptor check.
- A storm run with any survivor after the fix.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-0300c: §1 is built, every §3 item passes in gates, tools and on Furnace with zero storm survivors, or a §7 stop is recorded in FEEDBACK; the full adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-0300c-reaper-race.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
