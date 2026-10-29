# AET-WO-0300b — Close two verdict-path gaps: a Renode load with errors, and an uncapped input copy

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0300b-verdict-gaps`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6, spend by tier) in tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation), an inserted refinement of `AET-WO-0300`.
**Phase exit gate:** none new. `docs/roadmap.md` §5 makes this a precondition of `AET-WO-1500`.

**Grounds:**
- `docs/northstar.md` §4: `INV-03`, `INV-06`.
- `docs/architecture.md` §4: `RUL-10` (exit codes 1, 3 and 5) and `RUL-13` (the 16 MiB input cap); §2.4 (a failed Renode load exits 0); §6 (copy first, then hash, then judge).
- Argon's red-team findings at its 2026-10-29 review, as triaged in the architecture and roadmap amendments of that date:
  - **RT-02:** `simulate` can pass a partly failed load.
  - **RT-04:** `prove` copies a crate with no size cap.
- The code seams:
  - `crates/aethelgard-cli/src/renode.rs`, `judge`. A pass today needs only a listed CPU and no `There was an error executing command` line. No `[ERROR]` line is examined.
  - `crates/aethelgard-cli/src/input.rs`. `crate_dir` walks and copies every file, and `copy_hashed` copies any regular file, with no bound on either. `MAX_INPUT_BYTES` in `main.rs` bounds only `validate`.
  - `crates/aethelgard-cli/README.md`, the exit-code table.

> **What this is:** two checks on the existing verdict path.
>
> It is NOT:
> - a new subcommand;
> - a change to the sandbox;
> - a change to any other verdict rule.
>
> **Why now:** `AET-WO-1500` puts these verdicts in front of agents. An agent can supply a platform file or a crate, so both gaps become reachable from outside.

**Cadence:** build-only. There is no spike and no STOP for a ruling, except the flag-and-stop conditions in §7. Item 1.1's measurement is a step of the build, recorded in FEEDBACK. One PR, left unmerged.

## 1. Scope

1. **Renode: a load that logs an error is not a pass.**
   1. **Measure first, in `tools`, inside the image pinned by `container/IMAGE`.** Load `platforms/boards/nucleo_h753zi.repl` and `platforms/cpus/stm32h753.repl` the way `simulate` does, then record every line that contains `[ERROR]` or `[WARNING]`. Paste the lines, or "none", into FEEDBACK with the run URL.
   2. **Build a fixture** that lists a CPU and also logs `[ERROR]`. For example, a `.repl` that includes the Nucleo board and adds one peripheral Renode cannot register. Record the exact `[ERROR]` line it produces. If no input can be made to produce both, record what you tried, and go on to item 3 with a captured transcript as the test input instead.
   3. **Change `judge`.** When the machine lists a CPU and the output also carries any `[ERROR]` line, the verdict is **fail**, exit 1, with a `platform_load_error` finding whose detail is the first such line, trimmed. Today's other three outcomes are unchanged. `[WARNING]` lines do not change the verdict; item 1.1 records them only.
2. **Every tool input copy is bounded.**
   - The bound is `RUL-13`'s 16 MiB, across the whole input: the sum of every copied file, or the one copied file. A crate also has a cap of 4,096 files, counted after `target/` is skipped.
   - Over either cap, the call refuses with exit 3, `InputTooLarge`, naming the input and the cap.
   - Enforce the byte cap **while copying**, not only by summing metadata first. `copy_hashed` takes the remaining budget and reads at most one byte past it, as `validate` does. A file that grows between the walk and the copy is still caught.
   - The 16 MiB constant has one definition, shared by `validate` and the tool inputs.
3. **Documentation.** `crates/aethelgard-cli/README.md`:
   - exit 3's row names `InputTooLarge` for tool inputs and the file cap;
   - `simulate`'s section says an `[ERROR]` line fails the load.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06`.** An input over a cap is refused unread past the cap. It is never truncated and judged.
- **`INV-03`.** The verdict's sha256 is still the hash of exactly the bytes judged.
- **The existing contract does not move**, apart from the additions in §1: exit codes, refusal kinds, verdict keys, and every tier-1 and tier-2 test.
- **No `unsafe` in any workspace crate.** No new dependency.
- **Protected surfaces diff to empty:**
  - the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `crates/aethelgard-ir`, and every crate but `aethelgard-cli`;
  - `container/`, apart from one new fixture under `container/fixtures/` if item 1.2 needs one, recorded in FEEDBACK;
  - `.github/workflows/`.
- **Secrets by name only, and no new secret.** Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. Item 1.1's lines, or "none", are in FEEDBACK with the `tools` run URL.
2. **Tier 1, in `gates` and on Furnace:**
   - `judge` on a captured transcript that lists a CPU and carries an `[ERROR]` line gives fail with `platform_load_error`;
   - a crate one byte over 16 MiB is refused with `InputTooLarge`, and so is a crate with 4,097 files. A sparse file made with `set_len` is fine for the first;
   - a single board file one byte over 16 MiB is refused with `InputTooLarge`;
   - a crate and a board file each exactly at the cap are accepted.
3. **Tier 2, in `tools`:** the item 1.2 fixture fails with `platform_load_error`, if one could be made. Every existing case still passes unchanged.
4. The workspace gates pass in `gates` and inside the image. CI is green on the first push, and a red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- Any other change to how Renode, KiCad or Kani output is read.
- A cap on time, which belongs to the caller (`RUL-13`).
- An allowlist of "benign" `[ERROR]` lines. If item 1.1 finds any on a clean load, that is §7's first stop.
- Anything in `AET-WO-0400`, `AET-WO-1500` or `AET-WO-1600`.

## 5. Adversarial pass — full, on the two changed checks

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Pass a broken load.**
   - Log an error under a tag other than `[ERROR]`.
   - Split the line across two writes.
   - Use colour codes.
   - Make Renode print to stderr only.

   Say which of these the check misses, and whether `--plain` rules each out.
2. **Defeat the cap:**
   - many files just under the per-file size;
   - a file that grows during the copy;
   - hard links to one large file;
   - a sparse file;
   - `target/` used as a hiding place.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0300b.FEEDBACK.md`, same schema as before: `finding · where · type · recommendation · decision-needed`. It carries:
- item 1.1's measurement;
- the gates line with CI run URLs;
- what was not built;
- a promote-or-needs-none line for `architecture.md` §2 and §6.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A clean load of the Nucleo platform prints any `[ERROR]` line. Which lines are harmless is a ruling.
- A check that can only go green by changing an existing test's expectation.
- Anything that needs `unsafe`, a new dependency, a new secret, `sudo`, or a change to repository settings.
- A test that can only go green by being skipped, ignored, or weakened.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-0300b: §1 is built and every §3 item passes in gates, tools and on Furnace, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-0300b-verdict-gaps.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
