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

**STOP.** The build half (the filter in `run.rs`, `sockets_denied`, the tests, the README) has not started. It waits for the architect's ruling on S1-F and S-TEST.

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
