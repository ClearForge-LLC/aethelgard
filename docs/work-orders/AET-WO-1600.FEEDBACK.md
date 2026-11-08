# AET-WO-1600 — FEEDBACK

**Branch:** `wo/aet-1600-host-filter` · **Base:** `6525506` (PR #50) · **PR:** #51, unmerged · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Half:** spike, STOPPED for the architect's ruling. **The build half (§1) has not started:** the shipped filter, `socket_filter()` and its pinned test are unchanged, and nothing drops a capability.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

### How it was measured

All probes are under `spike/1600/`, plus three files the build deletes with that directory:
- **The seam:** `crates/aethelgard-cli/src/run.rs`, compiled only with `--features with-tools`.
  - `Invocation::run` forwards `AETHELGARD_SPIKE_1600` past `env_clear`.
  - After `socket_filter`, `confine_main` installs that variant's program (`spike/1600/hook.rs`) as a second seccomp filter.
  - `confine_main` removes the variable before exec.
  - Seccomp keeps the most restrictive action across all filters, so a variant can only add refusals.
  - An unknown variant refuses the call with `SandboxUnavailable`, `id` `seccomp`. The `bogus` runs show that the variable really reaches the trampoline (15/15 tier-2 tests refused with that kind).
- `crates/aethelgard-cli/examples/spike_1600_probe.rs` → `spike/1600/probe.rs`. It is a stand-in tool run by the real CLI inside the real sandbox:
  - as `kicad-cli`, it reports per-channel results as findings;
  - as `renode`, its `ToolError` line carries the tool's own `/proc/self/status` capability sets and `NoNewPrivs`.
- `crates/aethelgard-cli/tests/spike_1600.rs` contains:
  - a BPF interpreter that checks every variant against `unistd_64.h` and `linux/sched.h`;
  - the probe, as the baseline and under every variant;
  - the S4 crate, built offline beside the workspace;
  - with `with-tools`, **the whole tier-2 binary `with_tools` re-run** once per variant and under the capability drop. It is the unchanged suite, run as a child process, so the WO's "run every tier-2 test with that filter" holds exactly.
- `spike/1600/caps/`: S4, safe `rustix` with `thread`, in its own `[workspace]`.
- `spike/1600/census/`: S3, the per-tool distinct system calls from an `strace -f` log.

**Witnesses:**
- **CI run [42434959976](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42434959976)** at `910757c`: `gates` and `tools` both green.
  - The `tools` job is inside the image pinned by `container/IMAGE`, under Docker's default profile, as uid 0, on kernel `7.0.0-1017-azure`.
  - The `gates` job is the runner host, as uid 1001, on the same kernel.
  - The lines are in `spike/1600/results/ci-42434959976-{tools,gates}.answers`, fetched with `gh run view 42434959976 --log | grep -o 'ANSWER.*'`.
- **Furnace** (kernel `7.0.0-38-generic`, glibc 2.43):
  - **F-host:** `cargo test -p aethelgard-cli --features with-tools --test spike_1600`, with Kani from `~/.cache/aet-0300-kani` and Renode from `~/.cache/aet-0300a-renode`, and descriptors above 2 closed. Output: `results/furnace-host.answers`.
  - **F-podman:** CI's own test command inside the pinned image under rootless podman 5.7.0, with `--pids-limit=-1`. Output: `results/furnace-podman-image.answers`.
  - **F-strace:** `strace -f -qq -s 64` around the `with_tools` binary:
    - in the image, using the host's `strace` through the host's loader (`-v /usr:/host/usr:ro`), twice;
    - on the host, once.

    `spike/1600/census` reduced the logs to `results/census-*.txt`.

### S1 — What each tool needs from the candidate list: nothing

**Measured by refusal (CI, the `tools` job).** The tier-2 suite gives `passed=16 failed=0` under each family alone and under all four together:
- `sysv`, all 12 System V calls;
- `ns`: `unshare`, `setns`, and `clone` with any `CLONE_NEW*` bit;
- `pidfd`: `pidfd_open`, `pidfd_getfd`, `pidfd_send_signal`, `ptrace`, `process_vm_readv`, `process_vm_writev`, `kcmp`;
- `kernel`: `keyctl`, `add_key`, `request_key`, `bpf`, `perf_event_open`, `userfaultfd`;
- `s1`, all four together.

The baseline (`none`) and an empty stacked filter (`allow`) also give 16/16. Furnace's image run (F-podman) gives the same 16/16 for each. On the Furnace host (F-host), the 11 Kani and Renode tests pass under every family. The five KiCad tests are `ToolMissing` there in every run, because KiCad exists only in the image. **No call is taken out of the candidate.**

**The filter is live, not vacuous.** Under the probe (CI `tools`, and both Furnace runs), each family refuses its calls with `EPERM` inside the sandbox and leaves the others alone:
- `ipcmk -M/-Q/-S` → `Operation not permitted` under `sysv` and `s1`;
- `unshare --user` → EPERM under `ns`. On the host this is new; in the image Docker already refuses it;
- `pidfd_open` → `errno=1` under `pidfd`.

The interpreter checks every variant for numbers 0–1023, x32 and i386:
- the 29 numbers match the kernel's `unistd_64.h`, in CI and on Furnace;
- the clone mask `0x7e020000` equals the `CLONE_NEW*` bits of `linux/sched.h` that fit `clone`'s flags word;
- each of `clone`'s 32 flag bits is refused exactly when it is a namespace bit, with the high word set to all ones.

**What the tools actually call (F-strace, the census over the whole tier-2 suite):**

| Tool (label) | Distinct calls in the image | S1 calls made |
|---|---|---|
| `kicad-cli` | 51 | `clone3` ×240 |
| `renode` | 69 | `clone3` ×60 |
| `cargo` (Kani: cargo-kani, kani-driver, cbmc, rustc) | 88 | `clone3` ×517, **`pidfd_open` ×4** |

- **Kani's `cargo` calls `pidfd_open` and tolerates `EPERM`:** the `pidfd` and `s1` runs pass 16/16. It is made, not needed.
- No tool made any System V, `unshare`, `setns`, `clone`-with-`CLONE_NEW*`, `ptrace`, `process_vm_*`, `kcmp`, keyring, `bpf`, `perf_event_open` or `userfaultfd` call.

**Mono's shared area does not arise.** Renode 1.17.0, in the image and in Furnace's kit, is a .NET CoreCLR application: `/opt/renode/libcoreclr.so`, `libclrjit.so` and `libhostfxr.so`, with no Mono runtime. The census shows it makes no `shm*`, `sem*` or `msg*` call. It uses `memfd_create` for its own memory. Under `sysv`, both `simulate` tests pass.

### S2 — `clone3`: return `ENOSYS`; refusing it outright breaks every tool

| Variant | CI `tools` (image, glibc 2.41) | Furnace image | Furnace host (glibc 2.43) |
|---|---|---|---|
| `clone3-enosys` | 16/16 | 16/16 | 11/11 (+5 ToolMissing) |
| `s2-enosys` (S1 plus `clone3` → `ENOSYS`) | 16/16 | 16/16 | 11/11 (+5 ToolMissing) |
| `clone3-eperm` | **1/16**: 15 `ToolUnusable` | 1/16 | 1/16 |
| `s2-eperm` | **1/16** | 1/16 | 1/16 |

- Under `EPERM`, each tool fails its version check:
  - `kicad-cli --version` exits 255 (the CI sample);
  - `cargo --version` exits 101 and `renode` exits 137 (Furnace image). The probe shows why: glibc's `pthread_create` and `posix_spawn` try `clone3` first and fall back to `clone` only on `ENOSYS`. So with `EPERM`, thread creation gives `errno=1` and every child spawn gives `spawn-errno=1`.
- The test that still passes, `each_tool_removed_from_path_is_named`, runs no tool.
- Inside the image, `ENOSYS` is what Docker's own profile already returns for `clone3` without `CAP_SYS_ADMIN` (S5), so every tier-2 run in CI exercises glibc's fallback today.

### S3 — An allowlist, honestly costed: not stable, so the prior holds

**Distinct calls per tool**, over the tier-2 suite (F-strace; `results/census-*.txt` hold the lists):

| | `kicad-cli` | `renode` | `cargo`/Kani | union |
|---|---|---|---|---|
| image, run 1 | 51 | 69 | 88 | 105 |
| image, run 2 | identical sets | identical | identical | 105 |
| Furnace host, same Renode 1.17.0 and Kani 0.68.0 | n/a (not on host) | 67 | 85 | 99 |

The trampoline itself makes 31 calls. Between its `seccomp(` and its `execve` of the tool, every trampoline in the image trace makes only `write` (96 calls) and `rt_sigaction` (32 calls), which is Rust's `exec` resetting `SIGPIPE`. Measured by splitting each trampoline pid's lines at `seccomp(`.

**Stable run to run, but not across environments, even at the same tool versions on the same kernel:**
- Renode makes `rt_sigreturn` and `tgkill` in the image but not on the host.
- Cargo/Kani makes `fchown`, `restart_syscall` and `waitid` in the image only.
- `restart_syscall` and `rt_sigreturn` appear only when a signal interrupts a call, so they depend on timing as much as on the environment.

**The census commands** (Furnace, from the worktree root; `$IMG` is `container/IMAGE`'s digest, `$WT` the feature build of `with_tools` under `target/podman/debug/deps/`):

```
podman run --rm --network none --pids-limit=-1 -v /usr:/host/usr:ro -v "$PWD":/repo:Z \
  -w /repo/crates/aethelgard-cli -e CARGO_HOME=/repo/target/podman/cargo-home \
  -e CARGO_TARGET_DIR=/repo/target/podman "$IMG" \
  /host/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 --library-path /host/usr/lib/x86_64-linux-gnu \
  /host/usr/bin/strace -f -qq -s 64 -o /repo/target/podman/s3/image.trace /repo/$WT --test-threads=1
cargo build --offline --locked --manifest-path spike/1600/census/Cargo.toml --target-dir target/spike-1600
target/spike-1600/debug/aet-1600-census target/podman/s3/image.trace --calls
```

Host strace runs under the host's loader and libraries, and every traced program keeps the image's own glibc. The host run is the same `strace` line around the host feature build of `with_tools`. It used `env -i` with Kani's and Renode's `~/.cache` kits on `PATH`, `KANI_HOME` and `RUSTUP_HOME`, with descriptors above 2 closed.

**What was not measured, and why:**
- **The kernel axis.** The image has no `strace`, `.github/workflows/` is protected, so no step can mount one, and `SECCOMP_RET_LOG` is unreadable for `forge` (`dmesg_restrict=1`, no `adm` or `systemd-journal` group, `journalctl -k` empty). Inside a Docker container it is unreadable too (no `CAP_SYSLOG`). CI and Furnace both ran 7.0 kernels this week. `architecture.md` §2.5 already recorded the runner moving from 6.17 to 7.0 between two runs.
- **Inputs beyond the tier-2 fixtures.** These sets cover small fixtures only. A larger board, a longer simulation or a crate with build scripts can take paths these runs never touched.

**Cost.** An allowlist would be about 105 numbers plus `clone`'s flag test. Any miss on a new glibc, kernel, tool version or input path becomes an `EPERM` or a kill inside a tool, which surfaces as a confusing `ToolError`. **Recommendation:** keep the filter default-allow and add the denials: 27 `EPERM` numbers, `clone`'s namespace mask, and `clone3` → `ENOSYS`. Measured above, they cost nothing.

### S4 — Capabilities: safe `rustix` can do all three steps; the bounding set cannot be emptied by a user

**The safe calls**, all in `rustix` 1.1.5 behind its `thread` feature, which this workspace does not enable (root `Cargo.toml`: `features = ["fs", "process"]`). `std` has none of them.

| Step | Call | Where |
|---|---|---|
| empty the bounding set | `remove_capability_from_bounding_set(CapabilitySet::from_bits_retain(1 << n))` | `src/thread/prctl.rs:489` |
| read the bounding set | `capability_is_in_bounding_set` | `src/thread/prctl.rs:467` |
| clear E, P and I | `set_capabilities(None, CapabilitySets { all empty })` | `src/thread/libcap.rs:136` |
| clear the ambient set | `clear_ambient_capability_set()` | `src/thread/prctl.rs:725` |
| read or set `no_new_privs` | `no_new_privs()`, `set_no_new_privs(true)` | `src/thread/prctl.rs:618`, `:631` |

The spike crate compiles under `unsafe_code = "forbid"`.

**Measured** (`ANSWER caps …` lines; "after-exec" is the state of a program exec'd after the drop):

| Where | Before | Forward: bounding, ambient, capset | Reverse: capset, ambient, bounding |
|---|---|---|---|
| **root in the image** (CI `tools`) | `eff=prm=bnd=a80425fb`, Docker's 14 capabilities | **all ok**. After exec: `inh=prm=eff=bnd=amb=0`, `nnp=1` | bounding refused at cap 0, `errno=1`. **Bounding stays `a80425fb`** |
| root in the image (Furnace podman) | `800405fb`, podman's 11 | all ok, all zero after exec | bounding `errno=1` |
| **a user on a host** (CI `gates`, uid 1001) | `bnd=1ffffffffff` | **bounding refused at cap 0, `errno=1` (41 tried)**; ambient and capset ok | the same |
| a user on a host (Furnace, `forge`) | `inh=0000000800000000`, `bnd=1ffffffffff` | bounding `errno=1` (41 tried); ambient and capset ok; inheritable cleared | the same |

- **`no_new_privs` stays set** through every step and across the exec (`nnp=1`, `nnp_api=true`) in all four places.
- **The tools need no capability.** The whole tier-2 suite run under the forward drop as root in the image (`aet-1600-caps exec with_tools`) gives **16/16 in CI** and 16/16 on Furnace. So does the drop together with `s2-enosys`. The drop was applied above the CLI, so it is a superset of a drop in the trampoline.
- **What tools hold today.** In the image every tool runs as uid 0 with all 14 of Docker's capabilities effective: `status variant=none`, `eff=a80425fb`, `nnp=1`. On Furnace the tool inherits `CapInh` bit 35 (`CAP_WAKE_ALARM`) from the login session, with nothing permitted.
- **No step needs `unsafe`.** The §7 stop does not arise.

### S5 — What Docker's default profile already blocks inside the image

The profile was read at moby `v28.0.4`, the Docker that AET-WO-0300a's run recorded on `ubuntu-latest`, and at `v28.3.3`. It is identical for every call below at both tags (`results/docker-default-profile-v28.txt`). This job did not print the runner's Docker version (runner image `ubuntu-24.04` `20261101.360.1`). The container runs as root with Docker's default capabilities, so neither `CAP_SYS_ADMIN` nor `CAP_SYS_PTRACE` is held. Live is the CI `tools` probe at `none`, against the gates host.

| Call | Docker's rule | Live in the image | Already refused? |
|---|---|---|---|
| `shmget` … `semctl` (12) | allow | `ipcmk` ok | no, but Docker gives the container its own IPC namespace |
| `unshare` | allow only with `CAP_SYS_ADMIN` | `unshare --user` → EPERM (ok on the gates host) | **yes** |
| `setns` | allow only with `CAP_SYS_ADMIN` | not probed: no safe path | **yes** (profile) |
| `clone` + `CLONE_NEW*` | allowed only if `flags & 0x7e020000 == 0`, the same mask as ours | not probed | **yes** (profile) |
| `clone3` | **`ENOSYS`** without `CAP_SYS_ADMIN` | threads and spawns ok, consistent with the profile's `ENOSYS`; the fallback itself was not observed under Docker (no `strace` in CI) | returns `ENOSYS` (profile) |
| `pidfd_open`, `pidfd_send_signal` | allow | ok | no |
| `pidfd_getfd` | allow only with `CAP_SYS_PTRACE` | **errno=1** (ok on the gates host) | **yes** |
| `ptrace`, `process_vm_readv`, `process_vm_writev` | allow (kernel ≥ 4.8) | not probed | no; Landlock confines them across domains |
| `kcmp` | allow only with `CAP_SYS_PTRACE` | not probed | **yes** (profile) |
| `keyctl`, `add_key`, `request_key`, `userfaultfd` | not listed, so the default `EPERM` | not probed | **yes** (profile) |
| `bpf`, `perf_event_open` | allow only with `CAP_SYS_ADMIN`, `CAP_BPF` or `CAP_PERFMON` | not probed | **yes** (profile) |

**Conflicts.**
- Every new denial returns `EPERM`, the errno Docker's own default gives. The exception is `clone3`.
- When two filters both return an errno, the kernel keeps the most recently installed one's data. That is ours, because the trampoline installs after Docker. So **a `clone3` → `EPERM` rule would override Docker's `ENOSYS`.** It was measured: under `clone3-eperm` in the `tools` probe, `thread_spawn=errno=1`, and 15/16 tier-2 tests fail.
- **With `clone3` → `ENOSYS` there is no conflict anywhere.** The new filter repeats Docker for 10 of its 27 `EPERM` calls (`unshare`, `setns`, `pidfd_getfd`, `kcmp`, the three keyring calls, `userfaultfd`, `bpf`, `perf_event_open`), for `clone`'s namespace bits and for `clone3`. Inside the image it adds the refusal of the other 17: the 12 System V calls, `pidfd_open`, `pidfd_send_signal`, `ptrace` and `process_vm_*`.

### Spike findings

- **F1 · WO §0 ("There is no `strace`") · premise correction.** Furnace has `/usr/bin/strace` 6.19, installed outside this work order, and it works for `forge` (ptrace scope 1, tracing its own descendants). S3 used it, from the host and bind-mounted into the image. It needed no package and no `sudo`. **Rec:** amend the WO line and `roadmap.md`'s note. **Decision:** none.
- **F2 · WO §0 S1 ("Renode runs on Mono") · premise correction.** Renode 1.17.0 runs on .NET CoreCLR (`/opt/renode/libcoreclr.so`, single-file host). It makes no System V call (S1). **Rec:** amend wherever Mono is named. **Decision:** none.
- **F3 · WO §1.2 · ruling needed. A user cannot empty the bounding set,** so "if a drop fails, the call refuses" would refuse **every** tool call on a host.
  - `PR_CAPBSET_DROP` needs `CAP_SETPCAP` effective. An unprivileged process has a full bounding set and no `CAP_SETPCAP`: measured `errno=1` on the `gates` host and on Furnace.
  - Under `no_new_privs`, which Landlock already sets and which the measurements show persists, exec cannot add capabilities. setuid and file capabilities are ignored, so a full bounding set grants a capability-less process nothing.
  - **Rec:**
    - Clear ambient and E/P/I always; refuse if any clear fails.
    - Empty the bounding set when `CAP_SETPCAP` is effective, i.e. whenever there is something to drop, as root in the image.
    - Then verify that E, P, I and ambient are empty and `no_new_privs` is set, and refuse (`SandboxUnavailable`, `capabilities`) if not.
    - Do not require an empty bounding set where it cannot be emptied.

  **Decision:** yes.
- **F4 · S4 · ordering is load-bearing.** Bounding must come before `capset`, which removes `CAP_SETPCAP`. The reverse order left the bounding set at `a80425fb` as root in the image (CI). **Rec:** the build pins the order and tests it as root in the image. **Decision:** none.
- **F5 · root `Cargo.toml` · ruling needed.** The capability calls need `rustix`'s `thread` feature. Enabling it is a one-word change in the root `Cargo.toml` or the CLI's `Cargo.toml`, neither of which is in this WO's fence. It is no new dependency (`linux-raw-sys`'s `prctl` is already enabled through `process`). **Rec:** admit `rustix/thread` to the fence for the build. **Decision:** yes.
- **F6 · S2 · ruling needed. Return `ENOSYS` for `clone3`; never refuse it with `EPERM`.** `EPERM` breaks every tool everywhere, and in the image it overrides Docker's `ENOSYS` (S5). `ENOSYS` costs nothing: 16/16 in CI. The `clone` flags test then catches namespace creation on the fallback path. **Rec:** as stated. **Decision:** yes, the shape of the filter.
- **F7 · S3 · ruling needed. No default-deny allowlist.** It is unstable across environments at the same tool versions, a miss fails inside a tool, and the WO's prior is confirmed. **Rec:** default-allow, plus the 27 `EPERM` numbers, plus `clone`'s namespace mask, plus `clone3` → `ENOSYS`. **Decision:** yes (S2 of the WO's "It is NOT").
- **F8 · `spike/1600/hook.rs` · type: the test that earns its place.** My first assembler put `ret EPERM` first in the return block, so a variant whose rules all missed fell through into `EPERM`: the empty `allow` variant refused every call. The pinned-instruction style of test pins whatever was written and would have passed it. The interpreter test (`every_variant_does_what_its_spec_says_under_an_interpreter`) caught it before any run.
  - **Rec:** the build keeps an exhaustive interpreter test (every number, both ABIs and every `clone` flag bit) beside the extended pinned test.

  **Decision:** whether a ~60-line test interpreter is admitted to the CLI's tests.
- **F9 · WO §1.3 ("one tier-1 test per denied family, each probing from inside the sandbox") · ruling needed. Not every family can be probed live without `unsafe`.**
  - Safe live paths exist for:
    - System V, through util-linux `ipcmk` and `ipcrm`, present on the host, the runner and the image;
    - `unshare`, through util-linux `unshare`;
    - the three `pidfd_*` calls, through `rustix` `process`, already enabled.
  - No safe path exists for:
    - `setns`: rustix `thread`, plus a `/proc/<pid>/ns` path that Landlock denies to most tools;
    - `clone` with `CLONE_NEW*`;
    - `ptrace`, `process_vm_*`, `kcmp`;
    - the keyring calls, `bpf`, `perf_event_open`;
    - `userfaultfd`: rustix's is an `unsafe fn` (`src/mm/userfaultfd.rs:28`).
  - **Rec:** cover the reachable families live, and the rest by the interpreter test plus the pinned sequence.

  **Decision:** yes.
- **F10 · not measured · for the §5.1 adversarial pass, not this scope.** These are same-uid channels outside the WO's list, noted from reading, not measurement:
  - POSIX message queues (`mq_open`), which no Landlock right mediates;
  - `prlimit64` and `sched_setaffinity` or `setpriority` on another same-uid pid;
  - signals where Landlock is below ABI 6 (the floor is ABI 3).

  **Rec:** the adversarial pass probes them. Any finding goes to the architect, not into this build silently. **Decision:** whether the build's denial set grows.
- **F11 · Furnace environment · type: note.** `apparmor_restrict_unprivileged_userns=1` does not stop `unshare(CLONE_NEWUSER)`: `unshare --user` succeeds outside and inside today's sandbox on Furnace and on the runner host. On a host, the filter is the only barrier. Separately, rootless podman's default `pids-limit` (2048) makes `mcp_tier1`'s process-storm test fail with `EAGAIN`. That is podman's limit, not this code; `--pids-limit=-1` passes. **Decision:** none.
- **F12 · fence · type: flag.** The probe's entry point, `crates/aethelgard-cli/examples/spike_1600_probe.rs`, sits in `examples/`, which the fence does not name. It follows the pattern of tier 1's own stand-in tool, `examples/sandbox_probe.rs`, and is 11 lines that `#[path]`-include `spike/1600/probe.rs`. A stand-in tool must be its own executable, and `cargo test` builds examples, so there was no in-fence place for it. **Rec:** accept it for the spike; the build deletes it. **Decision:** whether that is acceptable.
- **F13 · protected surfaces.** `container/`, `.github/workflows/`, the steering documents, `README.md` and `LICENSE-*` diff to empty (`git diff 6525506 --stat`).

### Whether the host can be named a supported boundary for tool subcommands

**Not yet.** After the spike, the measured channels close on a host with no cost to any tool, given:
- the S1 denials;
- `clone3` → `ENOSYS`;
- the F3 capability drop.

On the host, the probe already shows System V, `unshare` and `pidfd_open` refused under `s2-enosys`. The naming waits on the build passing §1.3 in `gates` and `tools`, and on the §5 adversarial pass, F10 in particular.

## Gates (spike)

- **Furnace, at `910757c`:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (with and without `--features aethelgard-cli/with-tools`) and `cargo test --workspace` all pass. `cargo test` was run with descriptors above 2 closed.
- **CI:** run 42434959976, `gates` and `tools` green.
- **Adversarial and reduction passes:** belong to the build half (WO §5, §5a), not run.
