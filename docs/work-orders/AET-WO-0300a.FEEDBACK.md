# AET-WO-0300a — FEEDBACK

**Branch:** `wo/aet-0300a-seccomp` · **Base:** `6ce599d` (PR #14) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Half:** spike, stopped for the architect's ruling. The build half has not started: no change under `crates/` or `container/`.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

Probes are under `spike/`:
- `spike/probes/src/bin/seccomp-wrap.rs`: the candidate filter, in safe Rust;
- `spike/probes/src/bin/standin.rs`: the attack stand-in;
- `spike/s4.sh`: the harness;
- `.github/workflows/spike-seccomp.yml`: the CI half.

The filter was measured in two ways, without changing the CLI:
- **Inserted into the real trampoline chain:** `aethelgard --aethelgard-internal-confine … -- seccomp-wrap -- standin`. This is exactly the order the build half will use: Landlock first, then the filter, then the tool.
- **Wrapped around the whole CLI** (`seccomp-wrap -- aethelgard …`). The filter is inherited across `fork` and `exec`, so every tool then runs under the real Landlock sandbox and the filter together.

**Furnace commands**, both run from `~/aethelgard` at this branch, with the probes built into `~/.cache/aet-0300a-target` by `cargo build --release --manifest-path spike/probes/Cargo.toml --target-dir ~/.cache/aet-0300a-target`:
- **F1** = `bash spike/s4.sh "$PWD/target/debug/aethelgard" ~/.cache/aet-0300a-target/release`.
- **F2** = the CLI's real Renode and Kani cases with and without `seccomp-wrap --` in front, against Renode 1.17.0 (re-downloaded to `~/.cache/aet-0300a-renode` and checked against the sha256 in `container/Dockerfile`) and Kani 0.68.0 (`~/.cache/aet-0300-kani`), plus `strace -f -e trace=socket,socketpair,connect,bind,io_uring_setup` on each tool.

### S1 — Can the trampoline install its own filter? Yes, on Furnace and inside the image

- **Furnace (F1).** The real trampoline (`aethelgard --aethelgard-internal-confine`) applies Landlock (`sandbox abi=8`, which sets `no_new_privs`) and execs `seccomp-wrap`. Running unprivileged as `forge`, `seccomp-wrap` installs a 14-instruction filter (`installed 14 instructions in 58 us`), and the attacks that follow are refused. The filter therefore installs on top of Landlock with no privilege.
- **Inside the image** under Docker's default profile on `ubuntu-latest`: **yes**, in spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072) (job `probe`). The runner was `ubuntu-latest`, kernel `6.17.0-1022-azure`, Docker 28.0.4, with `docker info` showing `name=seccomp,profile=builtin`. Under the real trampoline (`sandbox abi=7`), `seccomp-wrap` printed `installed 14 instructions in 39 us`, and every attempt after it was refused. Docker's default profile allows a process to add its own filter.
- **The crate is `seccompiler` 0.5.0, licensed `Apache-2.0 OR BSD-3-Clause`.**
  - The spike uses only its public, safe API: the `sock_filter` struct and `apply_filter`. `apply_filter` itself sets `PR_SET_NO_NEW_PRIVS`, then calls `seccomp(SECCOMP_SET_MODE_FILTER)`, inside the crate's own `unsafe`.
  - `seccomp-wrap` has `#![forbid(unsafe_code)]` and compiles.
- **Why the BPF is assembled by hand, not through `seccompiler::SeccompFilter`.** The crate's rule compiler validates the architecture against `AUDIT_ARCH_X86_64` and matches system-call numbers exactly. x32 calls share that architecture value and differ only by bit `0x40000000` in the number, so a rule on `socket` (41) never sees x32's `socket` (`0x40000000|41`), and a default-allow filter lets it through. That is exactly §5.4's "correct only by luck". The hand-built program checks that bit itself. It is 14 instructions plus one per allowed family, and the build half can unit-test it instruction by instruction.

### S2 — Which socket families each tool needs: none

- **Renode and Kani (F2).** With every family refused and `socketpair` allowed, the CLI's real cases give exactly the verdicts they give without the filter:

  | Case | Exit | Verdict |
  |---|---|---|
  | `simulate` on the Nucleo | 0 | pass, with `cpus` `["cpu"]` |
  | `simulate` on `renode-missing-include.repl` | 1 | fail, `platform_load_error` |
  | `prove`, `add_in_range` | 0 | pass |
  | `prove`, `add_overflows` | 1 | `proof_failed` |
  | `prove`, `kani-compile-error` | 1 | `compile_error` |

- **What each tool actually calls** (`strace`, F2):
  - **Renode**, with .NET diagnostics at their default, makes exactly one `socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC)` and `bind`s it to an abstract address. That is .NET's diagnostic IPC server. Under the filter that `socket` gets `EPERM`, and Renode carries on, with the same verdicts and output apart from timestamps.
  - **Renode with `DOTNET_EnableDiagnostics=0`** makes **no socket call at all**, so the need disappears.
  - **Kani** (cargo, kani-driver, cbmc) makes only `socketpair(AF_UNIX, SOCK_SEQPACKET)` and `socketpair(AF_UNIX, SOCK_STREAM)`, which reach nothing outside, and no `socket` call.
- **KiCad and every tier-2 case under the filter in CI:** tier 1 gave **101 passed, 0 failed** and tier 2 **12 passed, 0 failed**, with the test binaries run as `seccomp-wrap -- <test binary>` inside the image under `--network none` (spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072)). Tier 2 is every pass and fail case for `check drc` (including exactly one `clearance` finding), `check erc`, `prove` and `simulate`, plus the tool-removed cases. So the real KiCad 10.0.7 needs no socket family either. The image has no `strace` (`ANSWER s2.strace=absent in the image`), so KiCad's need for no family rests on behaviour, every case passing with every family refused, rather than on a syscall trace. Renode under the filter in the image gives `rc=0 cpu=1` with `DOTNET_EnableDiagnostics` unset and with it set to `0`.
- **No family was ever allowed to make a tool work.** The flag-and-stop in §7 does not arise.

### S3 — What `io_uring` looks like in each environment

- **On Furnace, `io_uring_setup` is available,** both bare and under Landlock (`ok`, F1). Landlock does not mediate it, so on Furnace the filter's denial (`EPERM`) is the only thing between a tool and `io_uring`'s socket and connect operations.
- **Inside the image:** `io_uring_setup` is **already refused bare**, `errno=1 (EPERM)`, because Docker's default seccomp profile blocks it (spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072)). Inside the image the filter's `io_uring` rule is redundant with Docker's; on a host such as Furnace it is the only barrier (S3-F).

### S4 — The attack, before and after, on Furnace

The attack works before the fix. The raw lines are at the end of this section.

| Attempt | Bare | Landlock only, the AET-WO-0300 sandbox | Landlock + filter |
|---|---|---|---|
| `connect` to `/run/user/1001/bus` (the session bus) | ok | **ok** (AET-WO-0300 M1 reproduced) | `errno=1 (EPERM)` |
| `connect` to `/tmp/tmux-1001/default` | ok | **ok** | `errno=1 (EPERM)` |
| `connect` to an abstract socket (a listener outside, from `standin listen`) | ok | `errno=1 (EPERM)` (Landlock scope, ABI 6+) | `errno=1 (EPERM)` |
| UDP `sendto 127.0.0.1:9` | ok | **ok** | `errno=1 (EPERM)` |
| TCP `connect 127.0.0.1` (a listener outside) | ok | `errno=13 (EACCES)` (Landlock ABI 4+) | `errno=1 (EPERM)` |
| `socketpair` (allowed by design) | ok | ok | ok |
| `io_uring_setup` | ok | **ok** | `errno=1 (EPERM)` |

With the filter, every attempt is refused at `socket()` with `EPERM`, before it can reach a path, a port or a scope check. Inside the image: the same table holds, spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072), with uid 0. There is no session bus or tmux in the image, so those two give `ENOENT` bare and under Landlock, and `EPERM` under the filter, refused before the path is looked at. Abstract, UDP and TCP give `ok` bare; under Landlock (ABI 7) abstract gives `EPERM`, UDP `ok` and TCP `EACCES`; under the filter all are `EPERM`. `socketpair` is `ok` throughout.

### S5 — Cost and architecture

- **Install time** (F1, five runs of `seccomp-wrap -- /bin/true`): 323, 206, 179, 185 and 186 µs. In the chain under Landlock it took 58 µs. That is negligible beside a tool's own version query (30–120 ms, AET-WO-0300 S6).
- **i386** (`int 0x80`, `socket` = 359): bare it **succeeds** on Furnace, because the kernel has IA-32 emulation. Under the filter the process is **killed** (exit 159 = 128 + SIGSYS).
- **x32** (`0x40000000 | 41`): bare it gives `errno=38 (ENOSYS)`, because Furnace's kernel does not enable the x32 ABI. Under the filter the process is **killed** (exit 159). The kill rule is what protects a kernel that does enable x32.
- **Inside the image:** install time over five runs: 34, 36, 35, 56 and 25 µs (39 µs in the chain). x32 gives `ENOSYS` bare on the runner too, and the process is killed under the filter (159). i386 succeeds bare, and the process is killed under the filter (159). Spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072).

### Spike findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| S1-F | `seccompiler::SeccompFilter`'s compiled filter does not treat x32 specially. Under a default-allow filter, an x32 `socket` would slip past a rule keyed on `socket`'s number | `spike/probes/src/bin/seccomp-wrap.rs` | risk | Build half: assemble the program from `seccompiler::sock_filter` (public, safe) and install it with `seccompiler::apply_filter` (safe), as the spike did. Do not use `SeccompFilter`'s rule compiler. Unit-test the program's instructions | **yes** (the build route) |
| S2-F | Renode's .NET runtime makes one `AF_UNIX` stream socket for its diagnostic IPC server and binds it to an abstract address. The filter refuses it harmlessly, and `DOTNET_EnableDiagnostics=0` removes the attempt | S2 | note | Build half: also set `DOTNET_EnableDiagnostics=0` in Renode's environment, as named passthrough is set for Kani, so the runtime never tries. That is defence in depth; the filter alone already suffices | no |
| S3-F | `io_uring` is refused inside the image by Docker's own profile, and **available** on Furnace's host kernel. On the host, the filter is the only barrier | S3 | note | Tier 1, which runs on hosts, must test `io_uring_setup` refusal, because the image's own refusal would hide a regression there | no |
| S5-F | Neither kernel measured enables x32 (`ENOSYS`), so the x32 kill rule cannot be shown stopping a *successful* x32 call here, only that the call is killed rather than reaching the kernel. i386 (`int 0x80`) is enabled on both kernels and is killed by the architecture check | S5 | note | Keep both rules; the x32 rule protects kernels that enable it | no |
| S4-F | Pathname Unix sockets, UDP, raw sockets and TCP below Landlock ABI 4 are all refused at `socket()`, so the "Not confined" list in `crates/aethelgard-cli/README.md` can become empty once the build half lands. `socketpair` stays allowed; it reaches nothing outside | S4 | note | The build half empties the list and says what remains: nothing at the socket layer | no |
| S-TEST | A tier-1 test of `SandboxUnavailable` with `id` `seccomp` needs the filter install to fail, which happens only if `seccomp()` is unavailable. Docker's profile and Furnace both allow it. There is no safe, test-only way to make it fail without a hook in the code | build half | scope-question | Either (a) a cfg-test-only injection point in the trampoline, or (b) record that the refusal path is covered by code reading only, as the WO allows | **yes** |

**Unsafe code, named.** `seccomp-wrap`, the candidate filter, has none (`#![forbid(unsafe_code)]`). The stand-in uses `libc::syscall` and inline assembly for the `io_uring_setup`, x32 and i386 attempts, which no safe API offers. The stand-in is a spike probe outside the workspace, and the build half deletes it.

**AET-WO-0300 FEEDBACK, promote-or-needs-none:** its findings were promoted at its close into `architecture.md` §2.5, §6, `RUL-10` and the roadmap through PR #13, and its B3 is this work order. Nothing else needs promoting.

**Gates for the spike half:**
- `ci` [39545817284](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817284) passed, with `gates` and `tools` green.
- Spike run [39545817072](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39545817072) passed.
- `git diff --stat origin/main` over the protected surfaces prints nothing: the steering docs, `README.md`, `LICENSE-*`, `docs/inputs`, `crates/`, `container/`, `tools-image.yml` and `ci.yml`.

**STOP (spike half, 2026-10-21).** Ruled on PR #15 the same day; the build half follows below. At the time: the build half (the filter in `run.rs`, `sockets_denied`, the tests, the README) had not started and waited for the architect's ruling on S1-F and S-TEST.

**The raw output of F1 on Furnace** (kernel `7.0.0-38-generic`, uid 1001, with a live session bus and tmux socket):

```
ANSWER env=kernel=7.0.0-38-generic uid=1001 bus_exists=yes tmux_exists=yes
ANSWER bare unix_path_session_bus=ok
ANSWER bare unix_path_tmux=ok
ANSWER bare unix_abstract=ok
ANSWER bare udp_sendto_127_0_0_1=ok
ANSWER bare tcp_connect_127_0_0_1=ok
ANSWER bare socketpair=ok
ANSWER bare io_uring_setup=ok
ANSWER landlock unix_path_session_bus=ok
ANSWER landlock unix_path_tmux=ok
ANSWER landlock unix_abstract=errno=1(EPERM)
ANSWER landlock udp_sendto_127_0_0_1=ok
ANSWER landlock tcp_connect_127_0_0_1=errno=13(EACCES)
ANSWER landlock socketpair=ok
ANSWER landlock io_uring_setup=ok
ANSWER landlock.status=sandbox abi=8
ANSWER landlock+seccomp unix_path_session_bus=errno=1(EPERM)
ANSWER landlock+seccomp unix_path_tmux=errno=1(EPERM)
ANSWER landlock+seccomp unix_abstract=errno=1(EPERM)
ANSWER landlock+seccomp udp_sendto_127_0_0_1=errno=1(EPERM)
ANSWER landlock+seccomp tcp_connect_127_0_0_1=errno=1(EPERM)
ANSWER landlock+seccomp socketpair=ok
ANSWER landlock+seccomp io_uring_setup=errno=1(EPERM)
ANSWER landlock+seccomp.status=sandbox abi=8
ANSWER landlock+seccomp.install=installed 14 instructions in 58 us, allowed families []
ANSWER bare x32_socket=errno=38(ENOSYS)
ANSWER bare.x32.exit=0
ANSWER landlock+seccomp.x32.exit=159 (128+31: SIGSYS, killed)
ANSWER bare i386_socket=ok
ANSWER bare.i386.exit=0
ANSWER landlock+seccomp.i386.exit=159 (128+31: SIGSYS, killed)
ANSWER install_time.run1=in 323 us
ANSWER install_time.run2=in 206 us
ANSWER install_time.run3=in 179 us
ANSWER install_time.run4=in 185 us
ANSWER install_time.run5=in 186 us
```

## Build half

Built to the architect's rulings on PR #15 (2026-10-21).

**The filter.** `socket_filter` in `crates/aethelgard-cli/src/run.rs` is a fixed program assembled from `seccompiler::sock_filter` (0.5.0, Apache-2.0 OR BSD-3-Clause) and installed with `seccompiler::apply_filter`. Both are safe; `seccompiler::SeccompFilter` is not used (S1-F). The trampoline, `confine_main`, installs it after Landlock and before `exec`. In order, the program:
- kills on any architecture but x86_64;
- kills on any x32 call;
- returns `EPERM` for `socket` and for the three `io_uring` calls;
- allows `socketpair` only for `AF_UNIX` stream or seqpacket, and returns `EPERM` for any other `socketpair` (B1);
- allows everything else.

It is **18 instructions**. The ruling's "14" was the spike's count with an empty family list. Dropping the family machinery removed 3 instructions (11), and closing B1 added 7 (18).

**Every tool call goes through it.** Every `Invocation::run`, the version queries included, starts the reaper, which starts the trampoline, which installs Landlock and then the filter. The adversarial pass measured a stub whose `version` answer tried a socket: `sock=-1 errno=1 nnp=1 seccomp=2`.

**The rest of the rulings:**
- **Required.** An install failure, or a host that is not x86_64, writes `unavailable seccomp …`. `parse_status` maps that to exit 4, `SandboxUnavailable`, with `id` `seccomp`. There is no escape hatch.
- **Reported.** `SandboxReport` gains `sockets_denied: true`.
- **S2-F.** Renode runs with `DOTNET_EnableDiagnostics=0`, passed by name.
- **S3-F.** Tier 1 tests `io_uring_setup` refusal on hosts.
- **S4-F.** The README's socket-layer "Not confined" list is empty and says so.
- **S-TEST, option (b).** The install-failure branch is covered by code reading: `confine_main` writes the line, and `seccompiler::apply_filter` returns `Err` when `prctl` or `seccomp(SET_MODE_FILTER)` fails (crate source, lib.rs 336–376). A unit test proves the line maps to exit 4 with `id` `seccomp`.

### §3 acceptance

| §3 | Check | Result |
|---|---|---|
| 1 | Every §0 answer recorded | Spike section above |
| 2 | On Furnace, the S4 attack set succeeds without the filter and is refused with `EPERM` with it; both outputs pasted | **Without:** spike F1's `landlock` lines above (session bus, tmux, UDP and `io_uring` all `ok` under the same trampoline). **With:** the production CLI below, against Furnace's real session bus and tmux socket. And tier 1's test, with the filter mutated out, reproduces the attack (below) |
| 3 | Tier 1 in `gates` and on Furnace, including the socket and `io_uring` tests | `gates` green on `83dc85b` ([39551672323](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39551672323)), with `the_s4_attack_set_is_refused_inside_the_sandbox` passing, tier 1 at 101, and unit tests at 5 (at the time). Green again at `3cbfbff` in [39552571828](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39552571828), and on Furnace |
| 4 | Tier 2 in `tools`, inside the image, under `--network none`, every AET-WO-0300 case unchanged | `tools` green on `83dc85b` ([39551672323](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39551672323)): tier 2 12/12, tier 1 101/101, the socket test, the unit tests. At `3cbfbff`, 12/12 in [39552571828](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39552571828) under the narrowed `socketpair` rule (real KiCad, Renode and Kani), and 7/7 Kani and Renode on Furnace |
| 5 | Workspace gates in `gates` and inside the image | fmt, clippy `-D warnings` and test all green in both jobs of [39551672323](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39551672323) |
| 6 | Green on the first push after the ruling | **Yes**: `83dc85b`, [39551672323](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39551672323) |

**The production filter against Furnace's real targets** (`aethelgard check drc` with the stand-in as `kicad-cli`, at `83dc85b`):

```
== unix target /run/user/1001/bus
sandbox: {"landlock_abi": 8, "tcp_confined": true, "scoped": true, "sockets_denied": true}
   unix_path=errno=1
   unix_abstract=errno=1
   udp_sendto=errno=1
   tcp_connect=errno=1
   socketpair=ok
   io_uring_setup=errno=1
== unix target /tmp/tmux-1001/default
   (the same six lines)
```

**Tier 1 with the filter install mutated out of the trampoline**, on Furnace. The test fails, and its output is the AET-WO-0300 M1 attack:

```
left: ["unix_path=ok", "unix_abstract=errno=1", "udp_sendto=ok", "tcp_connect=errno=13", "socketpair=ok", "io_uring_setup=ok"]
```

**Mutations of the guards this WO adds** (one each, on Furnace):

| Mutation | Caught by |
|---|---|
| No `apply_filter` | the socket test |
| The `io_uring_setup` jump retargeted to allow | the socket test, the exact-sequence unit test and the jump-target unit test |
| The `socketpair` rule made allow-all | the socket test, reproducing B1 with `unix_dgram_pair_connect=ok` |

The adversarial subagent ran 11 more mutations; see below.

### Adversarial pass (§5)

A fresh subagent ran it in its own worktree at `83dc85b`, with real Renode and Kani and the live Furnace sockets, using a static C probe outside the workspace for raw system calls. Every finding was re-checked before it was acted on.

**Caught, with the result observed:**
- **`socket()`:** every family from 0 to 45, and `AF_UNIX`, `AF_INET`, `AF_NETLINK`, `AF_PACKET` and vsock with `SOCK_CLOEXEC|SOCK_NONBLOCK`, gave `EPERM`.
- **Other ABIs and number tricks:**
  - x32 `socket` was killed (SIGSYS);
  - i386 `int 0x80` 359 and `socketcall` 102 were killed;
  - `41|1<<32` gave `EPERM` (the kernel truncates the number to 32 bits);
  - `nr|1<<31` gave `ENOSYS`.
- **Inherited by children:** `socket` after `fork`, after `exec`, and after `setsid` plus a double fork all gave `EPERM`.
- **`io_uring`:** all three calls gave `EPERM`.
- **`open()` on the bus and tmux paths:** `EACCES` from Landlock.
- **Removing the filter:**
  - `PR_SET_NO_NEW_PRIVS 0` gave `EINVAL`, with `NoNewPrivs 1` and `Seccomp 2` visible from inside;
  - a second allow-all filter, and a LOG filter with TSYNC, both install, and `socket` stays `EPERM`;
  - `sudo` from inside: "no new privileges flag is set".
- **Unwired paths:** none. The four `.run()` call sites and every version query go through the trampoline (measured, above).
- **Mutations:** 11 instruction and constant mutations were all caught by the unit tests or the socket test. One survived: ignoring the `apply_filter` error. That branch can't be provoked, as S-TEST accepted.
- **Regressions:** none. Tier 1, and the Kani and Renode tier-2 cases, ran with no behaviour change.

| # | Finding (severity) | Done | Proof |
|---|---|---|---|
| B1 | **High.** `socketpair(AF_UNIX, SOCK_DGRAM)`, allowed under the ruled "reaches nothing outside", can `connect` or `sendto` any pathname datagram socket outside. Landlock does not mediate the connect, and BPF cannot filter `sendto`'s address. The adversary wrote "ADVERSARY-0300a: message written from inside the aethelgard sandbox" into the systemd journal (`/run/systemd/journal/dev-log`), with the verdict saying `sockets_denied:true`. It also had a two-way exchange with an outside datagram server. And a safe-Rust `build.rs` under `prove` (`UnixDatagram::pair()`, `.connect`, `.send`) reached an outside server while the verdict was `pass`. Stream and seqpacket pairs give `errno 91 (EISCONN)` on `connect`: they cannot be redirected | **Fixed** in `3cbfbff`: `socketpair` is allowed only for `AF_UNIX` with type & `0xf` = `SOCK_STREAM` or `SOCK_SEQPACKET`. `SOCK_DGRAM` and `SOCK_RAW` (datagram in `AF_UNIX`) get `EPERM`. Refusing every `socketpair` was measured by the adversary to break Kani (5 of 5 prove cases); the narrowed rule keeps all 7 Kani and Renode tier-2 cases green | Tier 1 now tries `unix_dgram_pair_connect` against a live outside datagram socket, proven reachable by a control send: `errno=1`, and nothing arrives. Mutating the rule back to allow reproduces the bypass and fails the test |
| B2 | **Medium, depends on the caller.** The tool inherits any descriptor the CLI's caller leaves open without close-on-exec: a connected socket passed as fd 3 was written through from inside. No layer between the CLI and the tool closes descriptors, and closing or marking a raw descriptor has no safe API in `rustix` 1.1.5 or the current `nix` (each needs `unsafe`) | **Fixed after the architect's review** (option (a), ruled on PR #15). Before Landlock (which denies `/proc` to most tools), the trampoline reads `/proc/self/fd` and each `/proc/self/fdinfo/N`. Any descriptor above 2 whose `flags` lack `O_CLOEXEC` (`0o2000000`) refuses the call with exit 4, `SandboxUnavailable`, `id` `descriptors`, naming the numbers. An unreadable `/proc` or a missing `flags` line refuses the same way. Safe Rust only | `tests/descriptors_tier1.rs`: `bash` opens a connected TCP socket as fd 3 and becomes the CLI, which refuses with exit 4, `id` `descriptors`, message `[3]`, no `verdict`, and the stub tool never runs. Removing the check fails that test (mutation on Furnace). **Measured to leak nothing:** `the_test_runner_leaks_no_descriptor_into_the_cli` finds no inheritable descriptor above 2 in the test process, on Furnace (printed `[]`) and in both CI jobs (head run below). Every other tool test also runs under the check and passes, which it could not do if anything leaked. No runner or `cargo` passes a descriptor, a jobserver included, so nothing needed an allowlist |
| B3 | Note. The reaper is unconfined, same uid, and an ancestor of the tool; with Yama `ptrace_scope=1` every `ptrace`, `process_vm_*` and `pidfd_getfd` attempt gave `EPERM`. The adversary's worry was `ptrace_scope=0` | Landlock itself restricts `ptrace` to processes inside the tracer's own domain, independently of Yama, so the reaper (outside the tool's domain) is not traceable from inside either way. That is from the kernel's Landlock design, not measured here (changing `ptrace_scope` needs root) | — |
| B4 | Note. A filter written for x86_64 would kill every system call on another architecture, undiagnosed | **Fixed:** `socket_filter` is `cfg(target_arch = "x86_64")`. Elsewhere it is `None`, and the call refuses with `SandboxUnavailable`, `id` `seccomp` | Reading (no other architecture here) |
| B5 | Note. The filter is default-allow, so non-socket system calls remain open. Observed: `shmget` (System V shared memory with other same-uid processes), `unshare(CLONE_NEWUSER)` and `pidfd_open`. bpf, `perf_event_open`, `userfaultfd` and `mq_open` were refused by the kernel or Landlock | Outside this WO (§4: a general system-call allowlist is a different work order). The README now names non-socket channels as outside the filter's scope | — |

### Build-half findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| B1 | `socketpair` was narrowed to `AF_UNIX` stream and seqpacket, which departs from the ruled "socketpair stays allowed". The ruling's reason holds only for connected pairs. **Confirmed by the architect on PR #15** | `src/run.rs` `socket_filter` | bug | Done | no |
| B2 | Inherited descriptors reached the tool. **Ruled (a) and fixed:** the trampoline refuses them (`id` `descriptors`) | `src/run.rs` `inherited_descriptors`, `confine_main` | risk | Done | no |
| B4 | The filter is unavailable off x86_64, so the tool subcommands refuse there | `src/run.rs` | note | Consistent with the amd64-only image (`RUL-15`) | no |
| B5 | Non-socket IPC (System V shared memory, user namespaces) is outside this WO and remains open to a tool | README | scope-question | A later work order on a system-call allowlist, if the threat model wants it | **yes** |
| B6 | The install-failure branch can't be provoked without a hook, so it is covered by reading (S-TEST (b)) and by the `parse_status` unit test. The adversary's one surviving mutation (ignoring the error) is in exactly that branch | `src/run.rs` | note | As ruled | no |

**Reduction pass (§5a).** A fresh read-only reviewer proposed 4 cuts, verified 1–3 in a scratch copy, and the builder applied all four in `fb16d88`:
1. One `SandboxUnavailable` message for both layers. The unknown-layer arm was unreachable, because only the trampoline writes the line.
2. The two `unavailable` unit tests merged into one over both layers.
3. The `sockets_denied` unit test dropped, because tier 1 asserts the field on real CLI output.
4. The filter's doc comment no longer restates its per-instruction comments.

Rejected:
- `seccompiler` without default features: its json feature is already off.
- Dropping `io-uring`: it is the safe route the S3-F test needs.
- The jump-target test: it is a different guard from the exact-sequence test.

`Reduction pass (diff): −27 lines net (13 added, 40 removed) · 2 duplicates removed · 1 dead path removed · 0 protected kept · 0 findings`.

**Gates line:**
- `83dc85b`: `ci` [39551672323](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39551672323), with `gates` and `tools` green.
- **B2 head** (the commit carrying this line, the PR head): the `ci` run on it, with `gates` and `tools`, in which `the_test_runner_leaks_no_descriptor_into_the_cli` and `an_inherited_socket_refuses_the_call_and_the_tool_never_runs` run on the runner host and inside the image. One push for this round, as ruled, so the run ID is not in this file; it is on the PR.
- `3cbfbff`, with the B1 fix and the reduction: `ci` [39552571828](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39552571828), `gates` and `tools` green. Inside the image: unit tests 3, the socket test (with the datagram-pair route), tier 1 101/101 and tier 2 12/12, all under the 18-instruction filter.
- The protected-surface diff prints nothing: the steering docs, `README.md`, `LICENSE-*`, `docs/inputs`, `crates/aethelgard-ir`, `container/` and `tools-image.yml`.

**What was deliberately not built (§4 held):**
- no namespaces, `chroot` or `pivot_root`;
- no Landlock rule change;
- no system call filtered beyond `socket`, `socketpair` (B1), the three `io_uring` calls and the architecture checks;
- no image, smoke or `container/IMAGE` change;
- no `mcp` subcommand and no trace archive.

**Measured facts for `architecture.md` §2.5 and §6:**
- S1–S5 above.
- The filter is 18 instructions and installs in 25–323 µs.
- No tool needs any socket family. Kani uses only `AF_UNIX` stream and seqpacket `socketpair`, and Renode only .NET's diagnostic socket, which `DOTNET_EnableDiagnostics=0` removes.
- `seccompiler::SeccompFilter` lets x32 calls through a default-allow filter (S1-F).
- A datagram `socketpair` reaches outside (B1).
- Docker's default profile already blocks `io_uring`; Furnace's host kernel does not.

**Is the "Not confined" list empty?** At the socket layer, yes, and inherited descriptors are now refused (B2). What remains is stated in the README: Landlock's read set, the temporary directory, and non-socket IPC (B5). B5 is out of this WO; the image stays the supported boundary (ruled on PR #15).

**AET-WO-0300 FEEDBACK, promote-or-needs-none:** promoted at its close into §2.5, §6, `RUL-10` and the roadmap (PR #13). Its B3 is this work order.

## Flag-and-stop: the GitHub runner host leaks two pipes (B2 measurement)

**Parked on `b8024cd`.** `ci` [39553415557](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39553415557) is **red in `gates` and green in `tools`**. This is the case the B2 ruling named: a runner that legitimately passes a descriptor is a flag-and-stop, never an allowlist entry.

- **The leak.** In `gates`, where `cargo test` runs directly on the `ubuntu-latest` host, the test process holds two descriptors without close-on-exec:

  ```
  the runner leaks ["142 -> Some(\"pipe:[12135]\")", "145 -> Some(\"pipe:[12136]\")"]
  ```

  The CLI therefore refuses every tool call there, as ruled: the refusal test saw `inherited without close-on-exec: [3, 142, 145]` where it expects `[3]`. `cargo test` stopped at that first failing binary, so the other tier-1 tool tests did not run in `gates`, and they would refuse the same way.
- **Where it does not leak.**
  - `tools`: the same tests inside the image behind `docker run`. Both descriptor tests pass, and so do tier 1 (101), the socket test, and tier 2 (12/12).
  - Furnace's `cargo test` (printed `[]`).
- **The source is not established.** It is two pipe descriptors, high-numbered, present on the runner host and absent behind `docker run`. That points at the GitHub Actions runner process handing pipes to the step's shell, or at a jobserver passed down through `cargo`. Telling them apart needs one diagnostic run that prints `CARGO_MAKEFLAGS`, `MAKEFLAGS`, and `/proc/$$/fd` with `fdinfo` from the step's shell, before `cargo` and inside `cargo test`. This round allowed one push, so that run has not been made.

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| B7 | The `ubuntu-latest` host passes pipe descriptors 142 and 145, without close-on-exec, into `cargo test` in `gates`, so B2's refusal makes every tool call refuse there | `.github/workflows/ci.yml` `gates`, and the runner | risk | Options, none taken without a ruling: (a) find the source with one diagnostic push, then close the descriptors in the CI step's own shell before `cargo` (for example a step wrapper that `exec`s with those descriptors closed), which changes CI, not the CLI; (b) run tier 1's tool tests in `gates` inside a clean `env -i`, `setsid` process whose descriptors are closed by the step's shell; (c) accept that the tool subcommands refuse on such hosts, because they are leaking, and move tier 1's tool tests to `tools` only, which would weaken `gates`, so it is not recommended. The CLI is behaving as ruled; the question is what the CI host hands it | **yes** |
