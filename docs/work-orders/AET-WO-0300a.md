# AET-WO-0300a — Close the socket gap: a seccomp filter denies the tool every socket Landlock cannot see

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0300a-seccomp`.
**Author:** Claude (architect) · **Builder:** Claude Code session, tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation), an inserted refinement of `AET-WO-0300`.
**Phase exit gate:** none new. This work order is the precondition `docs/roadmap.md` §5 sets for `AET-WO-1500`, and for any tool ever installed on a host with a session bus.

**Grounds:**
- `docs/northstar.md` §4: `INV-06`.
- `docs/architecture.md` §2.5. Landlock has no rule for pathname Unix sockets or UDP at any ABI through 8. A sandboxed `build.rs` on Furnace reached the user's D-Bus session bus and tmux's socket. TCP is confined only at Landlock ABI 4 and later.
- `docs/architecture.md` §6: the sandbox as built, and its known gap.
- `docs/architecture.md` §4: `RUL-10` (exit codes 4 and 5) and `RUL-12`.
- The code seam: `crates/aethelgard-cli/src/run.rs`. `confine_main` is the Landlock trampoline; it opens the status file, calls `confine`, then `exec`s the tool. `SandboxReport` and the `unavailable` status line live there too.
- AET-WO-0300's FEEDBACK, findings M1 and B3.

> **What this is:** a seccomp-bpf filter, installed by the trampoline after Landlock and before `exec`, so that every tool process and all its descendants are refused any socket they could reach beyond themselves.
>
> It covers exactly what Landlock leaves open:
> - pathname Unix sockets;
> - UDP;
> - raw sockets;
> - TCP on kernels below Landlock ABI 4;
> - `io_uring`, which can open and connect sockets without the `socket` and `connect` system calls a filter watches.
>
> It is NOT mount or user namespaces, NOT a change to what the tools may read or write, NOT network access for any tool, and NOT a new subcommand.
>
> **Why now:** `AET-WO-1500` puts `prove` in front of agents. A crate an agent supplies runs its own `build.rs` inside the sandbox, so today that `build.rs` could reach a session bus and start commands outside it.

**Cadence:** spike first, then STOP for the architect's ruling, then build. One PR for both halves, left unmerged. It is small; one push per half should do.

**The constraint that shapes everything:** Furnace has no container runtime and none of the tools, but it has the thing this work order defends against: a user session bus at `/run/user/$UID/bus`, and tmux's socket. So:
- reproduce the M1 attack, and prove the fix, **on Furnace**, with a stand-in process under the same trampoline;
- prove every real tool still works under the filter **in CI**, inside the image pinned by `container/IMAGE`.

## 0. Spike first — the questions the spike must answer

Record every answer in `docs/work-orders/AET-WO-0300a.FEEDBACK.md` under "Spike". Cite its CI run, or name the Furnace command that measured it. Then STOP. Probes go under `spike/`; the build half deletes them.

- **S1 Can the trampoline install its own filter?**
  - Measure on Furnace's kernel, and inside the image under Docker's default seccomp profile on `ubuntu-latest`.
  - Can an unprivileged process, already confined by Landlock with `no_new_privs` set, install a seccomp-bpf filter through `seccompiler`'s safe API, with no `unsafe` in a workspace crate?
  - Record the crate version and its licence.
- **S2 Which socket families each tool needs, if any.**
  - Run each tool's pass case and fail case with `socket` refused, `EPERM`, for every family, and `socketpair` allowed.
  - For any tool that breaks, find the family and type it needs by allowing one at a time, and say why it needs it.
  - .NET's diagnostic IPC socket in Renode is the expected case: record whether `DOTNET_EnableDiagnostics=0` makes the need disappear.
  - Never allow a family the tool can use to reach outside itself merely because it is convenient. That is a flag-and-stop (§7).
- **S3 What `io_uring` looks like in each environment.** Is `io_uring_setup` available inside the image under Docker's default profile, and on Furnace? The filter denies it either way. The question is only whether the denial is ever the thing that stops an attack.
- **S4 The attack, before and after, on Furnace.** Run a stand-in program under the existing trampoline. It tries five things:
  - `connect` to `/run/user/$UID/bus`;
  - `connect` to tmux's socket;
  - `connect` to an abstract socket;
  - a UDP `sendto` to `127.0.0.1`;
  - a TCP `connect` to `127.0.0.1`.

  Record each result without the filter (it should match AET-WO-0300 M1) and with the filter (each should be refused). Report `errno` exactly.
- **S5 Cost and architecture.** Measure the filter's install time. Confirm the filter refuses a system call made through the x32 ABI, or a foreign architecture, rather than letting it through. The image is amd64-only (`RUL-15`), so anything else should kill the process.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. A question that cannot be measured is answered when the reason is recorded. The architect rules by a comment on the PR and a new goal.

## 1. Scope — the build half, after the ruling

1. **The filter.** It is built in `run.rs`, installed by the trampoline after `confine` and before `exec`, with default action `Allow`. It refuses:
   - `socket`, for every family S2 did not prove a tool needs, including `AF_UNIX`, `AF_INET`, `AF_INET6`, `AF_PACKET` and `AF_NETLINK`;
   - `io_uring_setup`, `io_uring_enter` and `io_uring_register`;
   - any call through an architecture other than native x86_64, the x32 ABI included, by killing the process.

   Denials return `EPERM`, so tools fail as they would on a host with no network, not with a signal. `socketpair` stays allowed: it creates a connected pair inside the process and reaches nothing outside.
2. **Required, like Landlock.** If the filter cannot be installed, the call refuses with exit **4**, `SandboxUnavailable`, `error.id` = `seccomp`. There is no flag or variable that skips it (`INV-06`, and the PR #12 ruling's reason: an escape hatch is a default by another name).
3. **Reported.** `SandboxReport` gains `sockets_denied: true`, always true when a verdict exists, because the filter is required. Keep `tcp_confined` meaning exactly what Landlock confines; the README explains that seccomp now denies TCP as well.
4. **Tests, in the existing two tiers.**
   - **Tier 1, which runs everywhere including `gates` and Furnace:**
     - the S4 attack set, run by a stub tool under the real trampoline, each attempt refused with `EPERM`;
     - `io_uring_setup` refused;
     - a refusal test for `SandboxUnavailable` with `id` `seccomp`, if one can be provoked without `unsafe`. If it cannot, say why in FEEDBACK.
   - **Tier 2, in `tools`, inside the image:** every existing pass and fail case still passes under the filter, unchanged.
5. **Documentation.** Update `crates/aethelgard-cli/README.md`:
   - the sandbox section;
   - the "Not confined" list, which should end up empty, or state exactly what is left and why;
   - the `id` `seccomp` refusal;
   - the new `sandbox` field.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06` Absence halts.** A filter that cannot be installed refuses the call; it never falls back to running unfiltered.
- **No `unsafe` in any workspace crate** (`CLAUDE.md`). `seccompiler`'s safe API, or an equivalent, is the route. Name the dependency.
- **The existing contract does not move.** Exit codes, refusal kinds, verdict keys and every tier-1 and tier-2 test from AET-WO-0300 stay as they are, except for the additions in §1.
- **Protected surfaces diff to empty:**
  - the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `crates/aethelgard-ir`;
  - everything under `container/`, the image included. The filter is in the CLI, not the image;
  - `.github/workflows/tools-image.yml`.
- **Secrets by name only, and no new secret.** One PR, left unmerged. Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. Every §0 answer is recorded in FEEDBACK with its CI run or Furnace command.
2. On Furnace, the S4 attack set succeeds without the filter and is refused, with `EPERM`, with it. Paste both outputs into FEEDBACK.
3. Tier 1 passes in `gates` and on Furnace, including the new socket and `io_uring` tests.
4. Tier 2 passes in `tools` on the PR's head, inside the image by digest, under `--network none`, with every AET-WO-0300 case unchanged.
5. The workspace gates (fmt, clippy `-D warnings`, test) pass in `gates` and inside the image.
6. CI is green on the first push after the ruling. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- Mount, user or network namespaces, `chroot`, or `pivot_root`.
- Any change to the Landlock rules, unless S2 shows one is needed. Record it if so.
- Filtering any system call other than those in §1. A general system-call allowlist is a different and much larger work order.
- Changing the tools image, its smoke test, or `container/IMAGE`.
- The `mcp` subcommand (`AET-WO-1500`), the trace archive (`AET-WO-0400`).

## 5. Adversarial pass — break it before calling it done

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Reach a socket another way.** Try each of these from inside the sandbox:
   - `io_uring`;
   - `socketcall` (the 32-bit multiplexer);
   - an x32-ABI `socket`;
   - a `socket` made by a child after `fork` and after `exec`, since the filter must be inherited;
   - a file descriptor passed in from outside. Check what the tool actually inherits.
2. **Remove the filter.** Try making the tool's process unfiltered: `prctl`, `seccomp` with a weaker filter, `exec` of a setuid binary. `no_new_privs` must hold, and a second filter can only add restrictions.
3. **Look for an unwired path.** Confirm every tool subcommand goes through the trampoline that installs the filter, and that no code path runs a tool without it, version queries included.
4. **Look for assumptions that hold by luck.** Is the filter correct only because the image is amd64? Only because Docker's default profile already blocks something?

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0300a.FEEDBACK.md`, same schema as before: `finding · where · type · recommendation · decision-needed`.

It must carry:
- what did not work, and why;
- the gates line with CI run URLs;
- what was deliberately not built;
- the measured facts for `architecture.md` §2.5 and §6;
- whether the "Not confined" list is now empty;
- a promote-or-needs-none pass over AET-WO-0300's FEEDBACK. Its findings were promoted at its close into §2.5, §6, `RUL-10` and the roadmap through PR #13, and B3 is this work order. Say so.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A tool that cannot do its real function without a socket family that can reach outside the tool. That needs a different design, not an allowance.
- A filter that can only be installed with `unsafe` in a workspace crate.
- A design in which some tool call, version queries included, runs without the filter.
- Anything that needs a new secret, a personal access token, or a change to repository or organization settings.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo` or a system package on Furnace.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force (AET-WO-0300's achieved build goal), and dismiss it.

**Step 2, in its own turn, set the goal for the spike half:**

```
/goal AET-WO-0300a spike: every §0 question has a measured answer or a recorded reason it cannot be, cited to its CI run or Furnace command in FEEDBACK, on an open unmerged PR from wo/aet-0300a-seccomp, and the build half has not started.
```

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base and anything that changed since this file was written.

The architect sets a second goal for the build half after ruling on the spike.
