# AET-WO-1600 — Close the host's non-socket channels: the seccomp filter grows, and capabilities drop

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1600-host-filter`.
**Author:** Claude (architect) · **Builder:** a **strong** builder with a **full** adversarial pass (`docs/roadmap.md` §6: the host sandbox is a trust boundary, in hand-assembled BPF). A fresh session on Furnace.
**Phase:** P1. It must merge before the launch-readiness clause puts a crates.io-installed binary on hosts.

**Grounds:**
- **`docs/architecture.md` §6, "Still open on a host":** System V shared memory, `unshare(CLONE_NEWUSER)` and `pidfd_open`. These are non-socket channels to other processes with the same uid. Inside the published image, Docker closes them, so the image is today's supported boundary.
- **`docs/roadmap.md`'s 1600 line:** Argon added `ptrace`, `process_vm_readv`, `process_vm_writev` and `pidfd_getfd`, which Landlock already confines across domains. That makes them defence in depth. Argon also added dropping every capability before `exec`.
- **`docs/architecture.md` §2.6 and `AET-WO-0300a`:** the existing filter. It is hand-assembled, default-allow BPF installed by the trampoline after Landlock. It kills non-x86_64 and x32 calls, refuses `socket` except `AF_UNIX` streams and `socketpair`, and refuses `io_uring`. Its exact instruction sequence is pinned by a unit test (ruled on PR #15).
- `docs/northstar.md` §4: `INV-06`. `docs/architecture.md` `RUL-12` (Rust only) and `RUL-13`.
- **The seams:** `crates/aethelgard-cli/src/run.rs` (`confine_main`, the filter builder and its tests), the tier-1 sandbox tests, and the tier-2 tool tests.

> **What this is:** the filter refuses the listed channels too, and the trampoline drops every capability before it becomes the tool. Every tool still passes its tier-2 tests.
>
> It is NOT:
> - a default-deny allowlist, unless the spike shows one is cheap and stable (S2);
> - a change to Landlock rules, to the descriptor check, or to any verdict;
> - an image change.

**Cadence:** spike first, then STOP for the architect's ruling, then build, all in one session. One PR for both halves, left unmerged.

## 0. Spike — the questions it must answer

Record every answer in `docs/work-orders/AET-WO-1600.FEEDBACK.md` under "Spike", with the command or CI run that measured it. Then STOP. Probes go under `spike/1600/`, and the build deletes them. There is no `strace`, and installing it would need a system package, so measure by refusal: deny a call and see what still passes.

- **S1 What each tool needs.** Make a candidate filter that refuses, with `EPERM`:
  - System V IPC: `shmget`, `shmat`, `shmdt`, `shmctl`, `msgget`, `msgsnd`, `msgrcv`, `msgctl`, `semget`, `semop`, `semtimedop`, `semctl`;
  - namespaces: `unshare`, `setns`, and `clone` with any `CLONE_NEW*` flag (`clone`'s flags are a register, so they can be tested);
  - pidfds and cross-process access: `pidfd_open`, `pidfd_getfd`, `pidfd_send_signal`, `ptrace`, `process_vm_readv`, `process_vm_writev`, `kcmp`;
  - other kernel surfaces: `keyctl`, `add_key`, `request_key`, `bpf`, `perf_event_open`, `userfaultfd`.

  Then:
  - Run every tier-2 test, in CI's `tools` job, with that filter.
  - For each call a tool needs, record which tool and the evidence, and take it out of the candidate.
  - Renode runs on Mono, so measure whether Mono's shared area needs System V shared memory.
- **S2 `clone3`.** Its flags sit in user memory, where seccomp cannot read them. Measure two options:
  - returning `ENOSYS` for `clone3`, so glibc and Mono fall back to `clone`, whose flags the filter can test;
  - refusing `clone3` outright.

  Record which tools still run under each.
- **S3 An allowlist, honestly costed.** Record the distinct system calls each tool makes, measured with `SECCOMP_RET_LOG` or another method that needs no system package. Say whether a default-deny allowlist is stable across kernel, glibc and tool versions. The prior is that it is not, and that the filter stays default-allow with these denials.
- **S4 Capabilities.** Measure whether `rustix` (or `std`) offers safe calls to:
  - empty the bounding set;
  - clear the effective, permitted and inheritable sets;
  - clear the ambient set.

  Test both as root in the image and as a user on a host. Show that `no_new_privs`, which Landlock already requires, stays set. If a step needs `unsafe`, it is a §7 stop.
- **S5 What the published image already blocks.** Under Docker's default seccomp profile, check each S1 call: record whether it is already refused, and whether the new filter conflicts with the profile anywhere.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. The architect rules on the PR and gives the build goal in the same message.

## 1. Scope — the build half, after the ruling

1. **The filter** gains the ruled denials. The x86_64 and x32 rules and the socket rules stay exactly as they are. The unit test that pins the instruction sequence is extended to cover every instruction, as it does today.
2. **The trampoline drops every capability** before `exec`, as ruled in S4. If a drop fails, the call refuses with `SandboxUnavailable`, `id` `capabilities`.
3. **Tests:**
   - one tier-1 test per denied family, each probing from inside the sandbox and refused with `EPERM`;
   - a probe that holds a capability before the drop and none after;
   - every existing tier-1 and tier-2 test passes unchanged;
   - the filter installs on Furnace's host kernel and in the image.
4. **`architecture.md` §6's "Still open on a host" paragraph** is the architect's to rewrite, from the FEEDBACK.

## 2. Invariants and fences

- **`INV-06`:** a filter or a drop that cannot be applied refuses the call. Nothing runs half-confined.
- **No `unsafe`. No new dependency** without a ruling: `rustix` is already present.
- **Fence:** `crates/aethelgard-cli/src/run.rs`, the CLI's tests, `spike/1600/` (the spike only), and this work order's FEEDBACK. Protected surfaces diff to empty, as in every work order. `container/` and `.github/workflows/` are protected too.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

1. Every §0 answer is in FEEDBACK with its measurement.
2. After the build, §1.3 passes in `gates` and `tools`.
3. CI is green on the first push after the ruling.

## 5. Adversarial pass — full

1. Reach another same-uid process from inside the sandbox by any channel the filter leaves open: System V IPC, a namespace, a pidfd, `ptrace`, `/proc/<pid>/mem`, or a keyring.
2. Make the filter wrong for an architecture or an ABI: an x32 call, an `int 0x80` 32-bit call, or `clone3`.
3. Keep a capability past the drop, or regain one through a setuid or file-capability binary.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1600.FEEDBACK.md`, same schema. It carries:
- the S1 to S5 measurements;
- the calls each tool needs;
- the gates line;
- whether the host can then be named a supported boundary for tool subcommands.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A denial that breaks a tool, which only a weaker filter would fix.
- `unsafe`.
- A change to Landlock, to the descriptor check, or to the image.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-1600 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited in FEEDBACK, on an open unmerged PR from wo/aet-1600-host-filter, and the build half has not started.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
