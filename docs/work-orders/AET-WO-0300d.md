# AET-WO-0300d — The guardian removes its directory even while a writer races it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff. Verify the live HEAD before branching.
**Branch:** `wo/aet-0300d-guardian-retry`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass on the guardian (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P0, an inserted refinement of `AET-WO-0300c`'s guardian, found in CI after 0300c merged.

**Grounds:**
- **The failure.** PR #32's `gates` run 42424760372 (job 128560549661, head ae04803) failed `mcp_tier1::servers_sigkilled_while_calls_start_leave_nothing` with `parent-kill: servers=100 burst=5 descendants_seen=0 survivors=0 leftover_dirs=1`. No process survived; one `aethelgard-*` directory did. `main` was green on the same test before and after.
  - Two minutes later, PR #36's `gates` run 42424971511 (job 128561175483) failed the immediate-cancel storm with `calls=500 descendants_seen=681 survivors=0 leftover_dirs=1`.
  - Each failure passed on a single re-run.
  - On cancel, `mcp` kills only the CLI child (`kill_on_drop`). The guardian has its own process group and is not the reaper's descendant, so it lives on to retry. Both tests hit one mechanism.
- **The architect's reading,** from `main` at 1b4fe5f. Not yet reproduced: the first step below must confirm it.
  - When its parent is gone, `run.rs` `guard_main` calls `fs::remove_dir_all(&dir)` once, discards the result, and exits 0.
  - The directory is not quiet at that moment. The trampoline (`confine_main`) does `File::create(&internal.status)` on `root/status-*` (`main.rs` `Scratch`). The tool's stub may also still be writing under `root/tool` until the reaper's next 20 ms poll kills it.
  - So `remove_dir_all` can empty the directory, lose a race to a new entry, and fail on the final `rmdir` with `ENOTEMPTY`. Nothing retries it.
- `docs/architecture.md` §6, the sandbox lifecycle as `AET-WO-0300c` built it: "a guardian removes the temporary directory".
- `docs/northstar.md` §4: `INV-06`.

> **What this is:** the guardian keeps removing until the directory is really gone, within a bound, and says so by its exit code when it cannot.
>
> It is NOT:
> - a change to the reaper, the trampoline, Landlock, seccomp or the descriptor check;
> - a change to when the guardian decides its parent is gone;
> - a change to any verdict or user-facing exit code.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **Reproduce first.** Write a tier-1 test that starts the guardian (`aethelgard --aethelgard-internal-guard --cli-pid N <dir>`) from a parent that creates the directory and then dies, while a second process keeps creating and deleting files in that directory for a few hundred milliseconds after the death. On `main` it must leave the directory behind in at least one of a bounded number of tries. Record the counts in FEEDBACK.
   - If you cannot make it fail on `main`, record what you tried. Then build item 2 anyway: the single unretried `remove_dir_all` is a defect on its own reading. Note in FEEDBACK that the CI failure's cause stays unconfirmed.
2. **The fix, in `guard_main` only.** Once the parent is gone, the guardian loops:
   - `remove_dir_all(&dir)`;
   - on success, or on `NotFound` for the directory itself, exit 0;
   - on any other error, sleep 20 ms and try again;
   - after 10 s in total, exit 70. The directory is left behind, and the exit code says so.

   The guardian still never sets a death signal on itself, and still refuses a path that does not end in its own `aethelgard-*` name.
3. **Tests:**
   - item 1's test passes: the directory is gone once the writer stops, on every try;
   - a writer that never stops: the guardian exits 70 within 11 s, and the test then cleans up after it;
   - `servers_sigkilled_while_calls_start_leave_nothing` and the immediate-cancel storm pass 10 times in a row on Furnace, each with `leftover_dirs=0` recorded in FEEDBACK. Ten runs, not three, because each failure appeared about once in a few CI runs;
   - every existing tier-1 and tier-2 test passes unchanged.

## 2. Invariants and fences

- **`INV-06` and `architecture.md` §6:** no process and no directory outlives its call.
- **No `unsafe` in any workspace crate.**
- **Protected surfaces diff to empty:** everything outside `crates/aethelgard-cli/src/run.rs`'s `guard_main`, the CLI's tests, and this work order's FEEDBACK. The steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `container/` and `.github/workflows/` are all protected.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. §1.3 passes on Furnace and in `gates`, and tier 2 passes in `tools`.
2. FEEDBACK records item 1's reproduction counts on `main` and with the fix.
3. CI is green on the first push.

## 4. Scope fence

The reaper's poll interval; a guardian that waits on the reaper's process tree instead of retrying; anything in Landlock or seccomp.

## 5. Adversarial pass — targeted, on the guardian

1. Make the retry loop remove something it should not: a symlink swapped in for the directory between tries, or a path outside `aethelgard-*`.
2. Make the guardian spin forever or exit 0 with the directory still present.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0300d.FEEDBACK.md`, same schema. It carries:
- the reproduction counts;
- whether the CI failure's cause was confirmed;
- the gates line.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A fix that needs a change outside `guard_main`, or `unsafe`.
- A storm or parent-kill run with any leftover directory after the fix.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-0300d: §1 is built, every §3 item passes in gates, tools and on Furnace with zero leftover directories, or a §7 stop is recorded in FEEDBACK; the targeted adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-0300d-guardian-retry.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
