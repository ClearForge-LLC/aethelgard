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

SPIKE_PENDING

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
