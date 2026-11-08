# AET-WO-2101 — FEEDBACK (spike)

**Branch:** `wo/aet-2101-lockstep-spike` · **Base:** `ecdb86c` (`origin/main`, as the kickoff set it) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (x86_64, kernel 7.0.0-38, 20 CPUs). Two Sonnet subagents measured in parallel: S1/S2 (`spike/2101/ngspice/`) and S3 (`spike/2101/renode/`). The lead re-derived their headline hashes and outcomes from the raw files before citing them (below, "re-checked"). The lead did S4 to S7 and every sandbox run. · **Cadence:** spike, then **STOP** for the architect's ruling and Scotty's ratification of `OQ-02`.

**Where everything ran.** The pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`) under rootless podman: Renode 1.17.0 (`1.17.0+20260907gitf1dd1b4af`, .NET 8.0.12), ngspice and `libngspice0` 44.2 (`44.2+ds-1`), Rust 1.99.1 with `thumbv7em-none-eabihf`. Below, `$IMAGE` is that image. Nothing was installed. The image's one gap is a finding (F1).

**§2 and §7, held:**
- Only `spike/2101/**` and this file differ from `ecdb86c` (`git diff --stat ecdb86c -- . ':!spike'` is empty).
- Every probe crate declares its own `[workspace]`, and the root `Cargo.toml` is unchanged.
- No workspace crate gained `unsafe`. The probes' `unsafe` is counted in S1 and S7.
- No route needed a socket or an inherited descriptor above 2 (S5).
- Every S1 route completes a transient, and every S3 route gets the 0x60 read to the firmware.
- No `sudo`, system package or secret.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## What the spike added (a build deletes all of it)

- `spike/2101/probes/`: the hinge probes; `sbx.sh`, which runs any command under the CLI's own sandbox, unchanged; and the S5 scripts.
- `spike/2101/ngspice/` (`ng-probe`): the S1/S2 probe crate. `src/ffi.rs` holds the hand-declared libngspice FFI, the only `unsafe`, with the pinned header's line numbers cited. `src/pattern.rs` is the byte-window source and its step hooks, and `src/bin/s1c.rs` is route c. The Python drivers are for routes a and b and the summaries.
- `spike/2101/renode/`: the S3 probe firmware (`fw/`, zero dependencies, with its own `link.x` and `global_asm!` reset), the built ELFs, the `.repl` files, the route-i Python handler, the route-ii `Sensor.cs`, one monitor-command file per run, and the runners.
- `spike/2101/lockstep/`: the S4 owner (`lockstep`, std only and no `unsafe` of its own; it depends on `ng-probe`), the two line decks, and the S4 inputs and matrix scripts.
- `spike/2101/out/`: every output cited below, each beginning with its `# cmd:` line.

## Hinges, measured before anything was built

| | Question | Answer | Output |
|---|---|---|---|
| H1 | Does Renode take monitor commands on a piped, non-TTY stdin? | **Yes, line by line.** `emulation RunFor` blocks the next line until done, and an `echo` sentinel lands after it. Virtual time advances by exactly what was asked (10 µs against Renode's own 100 µs quantum). | `out/h1-renode-stdin.txt` |
| H2 | And under the CLI's sandbox? | **Yes.** The CLI's own reaper and trampoline run unchanged (`probes/sbx.sh` starts the built `aethelgard --aethelgard-internal-reaper …` from a shell, which passes the `--cli-pid` check). Status `sandbox abi=8`, exit 0, same output. | `out/h2-sandbox-stdin.txt` |
| H3 | Can the FFI be declared from source? | **Yes.** `sharedspice.h` was fetched at the `ngspice-44.2` tag (sha256 `4041c4e2…`, outside the repository, in `~/.cache/aet-2101/src/`). `ffi.rs` cites its lines: `ngSpice_Init` L372–373, `ngSpice_Init_Sync` L386, `ngSpice_Command` L391, `ngGet_Vec_Info` L395, `ngSpice_Circ` L427, `ngSpice_SetBkpt` L452, `vector_info` L176–183, and the callback typedefs L247–319. The image has no `libngspice-dev` (F1), so the probe links `-l:libngspice.so.0`. | `ngspice/src/ffi.rs`, `ngspice/build.rs` |

The production `Invocation` (`run.rs` L339) gives every tool `stdin` from `/dev/null`. H2 used an inherited pipe on fd 0, which the descriptor check allows. A build that feeds Renode's monitor from the CLI changes `Invocation`, so the S7 proposal avoids that (F6).

## S1 — Argon's measurement: 1,000 synchronous transients, three routes

**The circuit.** One line:
- R from 3.3 V to `line`, and C = 100 pF to ground;
- the open-drain driver is `S1 line 0 ctl 0 swm`, with `.model swm sw vt=1.65 vh=0 ron=10 roff=1e12`.

**The stimulus.** One I2C byte window: 9 bits at 400 kHz, 22.5 µs, with 1 ns control edges.

**Settings.** `.tran 1n 22.5u 0 5n`, with `.options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap`, identical in every route. Transient i carries byte `i mod 256` with ACK 0, at R = 2.2 kΩ.

**Repeat check.** Each route ran all 1,000 twice, and the per-transient hashes were compared:
- **a and b:** the hash is over `wrdata` text at `numdgt=15`;
- **c:** the hash is over raw f64 `time` and `line` bytes.

| Route | Startup | min | median | p95 | max | Total, 1,000 | run 1 = run 2 | Output |
|---|---|---|---|---|---|---|---|---|
| **a**: one `ngspice -b` per transient | in every transient | 9.40 ms | 10.00 | 10.44 | 11.71 | 10.03 s | yes, aggregate `933642a7…` | `out/s1-route-a.txt` |
| **b**: one long-lived `ngspice -p` (under a pty, F2) | 4.8 ms | 5.22 | 5.93 | 6.38 | 7.76 | 5.95 s | yes, `933642a7…` | `out/s1-route-b.txt` |
| **c**, `SetBkpt` at the edges | 2.0 ms | 3.94 | 4.03 | 4.09 | 6.56 | 4.04 s | yes, `70327c4a…` | `out/s1-route-c-bkpt.txt` |
| **c**, `*delta` clamped in `GetSyncData` | 1.2 ms | 3.94 | 4.07 | 4.29 | 10.04 | 4.12 s | yes | `out/s1-route-c-clamp.txt` |

All four rows, side by side: `out/s1-summary.txt`.

- **a and b agree exactly.** All 1,000 per-transient hashes are equal. Re-checked from the raw per-transient files, which stay outside the repository in `~/.cache/aet-2101/ng-scratch/` on Furnace: `grep -v '^#' <run>.tsv | cut -f3 | sha256sum` gives `933642a7…` for a1, a2, b1 and b2, and `70327c4a…` for both c-bkpt runs.
- **c against a/b.** The hash forms differ, so the comparison is on rise time (S2). `SetBkpt` reproduces a/b to 0.0001 ns, and the clamp is within 9 ps of the edge.
- **With neither control,** the solver steps over the edge start: the 30 % crossing lands 2.1 ns late (`out/s2-rise.txt`, mode `none`).
- **Re-run mechanism in b.** `alter @vctl[pwl] = [ t v … ]` works, including with a different point count. `alter vctl = pwl(…)` is a syntax error (`out/s1-b-pty.txt`).
- **Calls per transient in c** (`out/s1-route-c-bkpt.txt`, `-clamp.txt`, unverified by the desk spike):
  - `GetVSRCData`: median 9,074 (9,022–9,166);
  - `GetSyncData`: median 9,056;
  - accepted points: median 4,524;
  - only locations 0 and 1 occur;
  - `redo` is non-zero on 0–25 calls per transient;
  - the host never returned 1, and time never went backwards.
- **The `unsafe` surface of c:** 12 lines contain `unsafe`, and 82 lines sit inside unsafe regions, all in `ngspice/src/ffi.rs`: the `extern` block, the callbacks, `init`, the wrapper methods, and the S4 helper `vec_last` the lead added. `out/s7-unsafe.txt` counts it after that addition; `out/s1-unsafe.txt` (11 and 74) predates it. The probe firmware's `unsafe` (MMIO, `static mut` result words, the `global_asm!` reset) is listed by line in `out/s3-unsafe.txt`.

## S2 — The measured rise time against the ceiling

**Pattern.** Byte 0x7F with NACK: the line is held low for the first bit, then released for 20 µs, so one release edge completes inside the 22.5 µs window.

**Thresholds.** 30 %–70 % is 0.99 V → 2.31 V. Theory, 0.8473 × R × C, gives 186.41 ns at 2.2 kΩ and 3,982.31 ns at 47 kΩ. The ceiling is 300 ns.

| R | a: `.meas` | b: `.meas` | c, `SetBkpt` | c, clamp | vs theory | vs 300 ns |
|---|---|---|---|---|---|---|
| 2.2 kΩ | 186.4014 ns | 186.4014 | 186.4014 | 186.4067 | −0.002 % | 113.6 ns **below** |
| 47 kΩ | 3,982.3000 ns | 3,982.3000 | 3,982.3001 | 3,982.2996 | 0.000 % | **exceeds by 3,682 ns (13.3×)** |

- **Two methods agree in c.** The host's own interpolation of the vectors and ngspice's `meas`, issued through `ngSpice_Command`, give the same value.
- **The bus failure itself.** With a toggling byte (0x55) at 47 kΩ, the line peaks at **1.362 V, 41 % of VDD**, in every bit window, and never reaches 70 %. `meas` reports "out of interval" in all three routes.
- **At 2.2 kΩ the same byte passes:** the line reaches 3.30 V in 186.4 ns.

Output: `out/s2-rise.txt`.

## S3 — How a fault reaches the firmware

**The probe firmware.** `renode/fw/src/main.rs`, zero dependencies:
- **What it does:** one read of the reference BME280's ID register (0x76, register 0xD0, which reads 0x60) on I2C1 (`STM32F7_I2C` at `0x40005400`), using the I2Cv2 write-then-read sequence.
- **Its waits:** each is a bounded spin of 20,000.
- **Its result block:** written at `0x20000000` (magic, status, bytes, ISR, spins per wait).
- **The judge:** reads the block between sentinels, with `ExecutedInstructions` and `PC`.
- **The run:** each is 10 ms of virtual time, exactly 1,000,000 instructions.

**Outcomes** (side by side in `out/s3-summary.txt`). Every row was identical over 10 runs: one sha256 over the whole block, the instruction count and the PC (`out/s3-determinism.txt`). The lead re-checked four of these hashes independently, sandboxed and unsandboxed (S5).

| Route | Mode | What the firmware observes | Log | Output |
|---|---|---|---|---|
| **iii** stock `I2C.BME280 @ i2c1 0x76` | present | OK, 0x60, ISR 0x21 | — | `out/s3-route-iii.txt` |
| | never declared | **TIMEOUT_TXIS** (20,000 spins), ISR 0x1 | `Unknown slave at address 118.` | |
| | removed mid-run: `sysbus.i2c1 Unregister sysbus.i2c1.bme` | TIMEOUT_TXIS from the next transaction. Re-adding with `machine LoadPlatformDescriptionFromString "bme: I2C.BME280 @ i2c1 0x76"` gives OK again | same | |
| **i** `Mocks.DummyI2CSlave` + generated Python (`i-handler.py`, 20 lines) | normal | OK, 0x60 | — | `out/s3-route-i.txt` |
| | withhold (`s3_fault 1`) | **OK, byte 0x00, never a timeout:** the model pads a short read with zeros | `Not enough data in buffer, filling rest with zeros.` | |
| | corrupt (`s3_fault 2`) | OK, byte 0x61 | — | |
| **ii** run-time C# (`Sensor.cs`, 22 lines, `include`d before the platform) | normal | OK, 0x60 | — | `out/s3-route-ii.txt` |
| | no bytes (`sysbus.i2c1.sensor Fault 1`) | **TIMEOUT_RXNE**, ISR 0x1, silently | — | |
| | corrupt (`Fault 2`) | OK, 0x61 | — | |
| | short, NBYTES=2 (`Fault 3`) | 0x60, then TIMEOUT_RXNE at the second byte, with TC and STOPF set early | — | |

- **Cross-route equality.** i normal = ii normal = iii present, and i corrupt = ii corrupt: the hashes are identical. Without a timeout the routes are indistinguishable to the firmware.
- **Mid-run fault changes work in all three.** They were measured between `RunFor` calls with the repeat firmware (`renode/fw/src/bin/repeat.rs`: read, ~100 µs delay, counters) and were deterministic over 10 runs.
- **Controller reset after a timeout is not needed in this model.** The `no-reset` build recovers identically.
- **Correction to the desk spike, Q5.** An absent address leaves **TXIS** unset (the write phase never starts), not only TC. Which wait times out therefore depends on the route.

**`RUL-12` cost, and wall time per run** (whole podman + Renode process; Renode starts in about 1.0 s):

| Route | Language layer | Wall time per run | Can produce |
|---|---|---|---|
| **iii** | **none:** stock models and monitor commands | 1.97 s | timeout only |
| **i** | a generated Python snippet, inside `RUL-12` as written | 2.02 s | wrong data only |
| **ii** | **C#, a new layer:** a `RUL-12` amendment | 2.65 s. Roslyn compiles `Sensor.cs` at every launch, +0.67 s (1.70 s against 1.04 s, five runs each, `out/s3-roslyn.txt`) | timeout, short transfer, wrong data |

## S4 — Lock-step

**The owner.** `lockstep/src/main.rs` stands in for the CLI and drives each quantum in the work order's order:
1. Renode, on its stdin: `emulation RunFor "<q>"`, then the reads between whole-line sentinels. The reads are the repeat firmware's ten-word result block, `ExecutedInstructions` and `PC`.
2. ngspice advances to the same time.
3. The fault state for the next quantum is set by a monitor command.

**The backends:**
- `pipe` is route b. A long-lived `ngspice -p` under a pty, advanced by `stop when time > T` and `resume`.
- `lib` is route c. The same commands, through `ngSpice_Command`.
- `ioc` is route c by inversion of control. One `tran` runs, with `ngSpice_SetBkpt` at every boundary. When the solver lands on a boundary, Renode's quantum runs from inside `GetSyncData`, on ngspice's thread.

**The span** is 10 ms of virtual time: the boot, then about 83 ID reads at 2.2 kΩ, or 39 at 47 kΩ (the faulted reads wait out their timeouts). It is the `AET-WO-2100` reference boot's length, a tenth of `boot::MAX_VIRTUAL_NS`.

**The analog net** is the line deck (`line-2k2.cir`, `line-47k.cir`). The owner's driver repeats the S2 byte window every 22.5 µs, and the stimulus is synthetic (F9).

**The sensor routes:**
- route ii is `sysbus.i2c1.sensor Fault {0|1}`, sent every quantum;
- route iii is `Unregister`, then a re-add, sent on transitions only (`--fault-on`, `--fault-off`);
- route i is `s3_fault {0|1}`.

**`INV-04`: every configuration run twice gave the same state-sequence hash, 32 of 32** (`out/s4-matrix.txt`; raw lines in `out/s4-matrix-logs.txt`). The hash covers simulator state only (F10). Beyond the repeats:
- **b and c agree exactly in lock-step.** `pipe` and `lib` give identical hashes in all six configurations, for example `f83942ea…` at 47 kΩ and 100 µs. `ioc` differs only because it lands exactly on each boundary (F8). Its firmware counters are identical.
- **Without a fault the routes coincide.** Routes i, ii and iii at 2.2 kΩ and 100 µs all hash `29e7b964…`, with 83 of 83 reads OK.
- **What the firmware recorded at 47 kΩ** (10 ms span, 100 µs quanta):

| Route | Attempted | OK | Wrong | Timed out | Last status |
|---|---|---|---|---|---|
| ii | 39 | 1 | 0 | 38 | RXNE |
| iii | 39 | 1 | 0 | 38 | **TXIS** |
| i | 83 | 1 | **82** (0x00) | 0 | OK |

- **At 1 ms quanta, 9 reads pass before the fault lands** (F7).

**Cost** (`out/s4-timing.txt`; a fresh `HOME` per run, as the sandbox gives; route ii; one run per row):

| Quantum | Quanta in 10 ms | `ioc` | `lib` | `pipe` | `ioc`, one read per quantum |
|---|---|---|---|---|---|
| 1 ms | 10 | 2.25–2.31 s (225–231 ms per quantum) | 2.17–2.27 s | 2.26–2.46 s | — |
| 100 µs | 100 | 2.69–2.80 s (27–28 ms) | 3.86–4.00 s | 3.98–4.05 s | 2.55 s |
| 10 µs | 1,000 | **8.9–9.3 s (8.9–9.3 ms)** | 20.8–23.0 s | 21.3–21.5 s | **5.05 s** |
| 1 µs | 10,000 | about 70 s, extrapolated from 1,000 quanta over 1 ms (6.7–7.1 ms per quantum), with the history fix (F16) | 7.9 ms per quantum | — | 2.3 ms per quantum |

- **At 1 ms quanta the cost is ngspice's.** Simulating 10 ms of the line at a 5 ns step limit takes about 2.1 s, whatever the quantum.
- **At 10 µs and below the cost is Renode's monitor.** Renode alone with one read costs 1.8–2.0 ms per quantum. With twelve reads, 6.6 ms per quantum at 1 µs (F11).
- **The 1 µs × 10 ms run never finished.** It exceeded 900 s. Renode's monitor history grows with every command, about 160,000 of them in that run, and each command slows as it grows (F16).

**Which quantum is enough:**
- **A static rise-time fault needs no in-run lock-step.** Its measurement completes inside the first byte window, 22.5 µs, and never changes: every quantum from 1 ms to 1 µs reached the same verdict. The quantum only adds latency (F7). One analog pass before the boot is enough: about 4 ms per transient (S1), and the fault applies from the first instruction.
- **A rail droop needs about 10 µs** (REASONED). A regulator's load-step recovery takes tens of µs, so 10 µs resolves the dip, and 100 µs would step over it. Measured: 9 s per 10 ms span with `ioc` (5 s reading one word per quantum), against 20.8–23.0 s with `lib` or `pipe`.

## S5 — Each route under the CLI's sandbox

Every run went through the CLI's own reaper and trampoline, unchanged (`probes/sbx.sh`):
- the built `aethelgard` binary at `ecdb86c`;
- Landlock ABI 8, the seccomp filter (sockets refused) and the capability drop;
- `--ro` for exactly the `SYSTEM_DIRS` plus what `simulate` adds for Renode: `/proc`, `/dev/urandom` and the tool's directories;
- `--rw` on the workdir only, unless a row names an extra grant;
- an empty environment except `HOME` and `TMPDIR` = the workdir, `DOTNET_EnableDiagnostics=0`, and `PATH=/usr/local/bin:/usr/bin:/bin`. The CLI's `Invocation` passes no `PATH`; the probes did not test without it, though every tool is started by absolute path.

| Route | Result under the sandbox | Output |
|---|---|---|
| **a** `ngspice -b` | **Fails:** `tmpfile(): Permission denied`. glibc's `tmpfile()` uses `/tmp`, not `TMPDIR`. It passes only with `/tmp` granted read-write, which is too broad to rule (F3). | `out/s5-ngspice-diag.txt` |
| **b** `ngspice -p` | **Fails three ways.** Without a tty it segfaults (F2). Under `script`'s pty, `/dev/ptmx` is denied. With `/dev/ptmx`, `/dev/pts`, `/dev/null`, `/dev/tty` and `/tmp` granted, it runs. | `out/s5-ngspice-c.txt`, `out/s5-ngspice-diag.txt` |
| **c** libngspice in-process | **Runs with no extra grant.** The 1,000 transients hash `70327c4a…` sandboxed and unsandboxed. 4.50 s wall, 4.27 s in transients. | `out/s5-ngspice-c.txt` |
| **S3 i, ii, iii** (Renode, monitor on stdin) | **All run.** Six command files give the same result hash sandboxed and unsandboxed: i normal and withhold, ii normal and no-bytes, iii present and absent. Route ii's Roslyn compile works with `TMPDIR` = the workdir. | `out/s5-renode.txt` |
| **The S4 owner** (`lockstep`, libngspice in-process, Renode as its child), 47 kΩ, 100 µs × 100 | **`ioc` and `lib` run with no extra grant,** with the same state-sequence hash sandboxed and unsandboxed (`590fe06e…` and `f83942ea…`). `pipe` runs only with the five route-b grants. | `out/s5-lockstep.txt` |

- **No route needed a socket, and none needed an inherited descriptor above 2.** The only channels are the stdin and stdout pipes on fds 0 and 1 between the owner and its children, and the owner's own memory for libngspice.
- **The named FIFO is not needed** for any route.

## S6 — Trace honesty: two events, never one

The S4 owner writes its events to `<run>.events.jsonl` (`lockstep/src/main.rs`, `Owner::finish`). Three kinds of record have separate sources, and no field is computed from another kind:

1. **`analog.rise_time_measured`,** `source: "ngspice"`. The evidence. It holds the net, `rise_30_70_ns` from the solver's own vectors (by `meas` in `lib`/`pipe`, and by the hook's online interpolation of accepted points in `ioc`), `ceiling_ns`, and `exceeds`.
2. **`bridge.fault_state`,** `source: "owner"`. The owner's decision, with `from: "analog.rise_time_measured > ceiling_ns"`. It is the **only coupling** between the solvers: no voltage reaches Renode's transaction-level model, which has no pins and no bit timing (`architecture.md` §2.7).
3. **`firmware.observed`,** `source: "renode"`. The effect. It holds the firmware's own result block, read from RAM, and is emitted only when what the firmware recorded changes.

The excerpt is from route ii at 47 kΩ, 100 µs quanta (`out/s6-events-47k.jsonl`):

```
{"kind":"analog.rise_time_measured","source":"ngspice","quantum":1,"t_virtual_ns":100000,"net":"line","rise_30_70_ns":3982.2999,"ceiling_ns":300,"exceeds":true}
{"kind":"bridge.fault_state","source":"owner","quantum":1,"t_virtual_ns":100000,"fault":true,"from":"analog.rise_time_measured > ceiling_ns"}
{"kind":"firmware.observed","source":"renode","quantum":1,"t_virtual_ns":100000,"reads":["0x21015252","0x00000001","0x00000001","0x00000000","0x00000000","0x00000000","0x00000060",…]}
{"kind":"firmware.observed","source":"renode","quantum":3,"t_virtual_ns":300000,"reads":["0x21015252","0x00000002","0x00000001","0x00000000","0x00000001","0x00000003","0x00000000",…]}
…
{"kind":"firmware.observed","source":"renode","quantum":99,"t_virtual_ns":9900000,"reads":["0x21015252","0x00000027","0x00000001","0x00000000","0x00000026","0x00000003","0x00000000",…]}
```

- **At 47 kΩ:** the first read, in quantum 1, is OK, before the fault takes effect (F7). Then 38 of 39 time out.
- **At 2.2 kΩ** (`out/s6-events-2k2.jsonl`): `exceeds: false`, no `bridge.fault_state` record, and 83 of 83 reads OK.

- **Status 3 is TIMEOUT_RXNE.** Counters run: attempted, OK, wrong, timed out.
- **The firmware never sees a rise time,** and the ngspice record never says what the firmware did.
- **The firmware's view comes from a stand-in.** It saw a timeout because Renode 1.17.0 has no NACK (`architecture.md` §2.7). A real STM32 would see NACKF (REASONED: at 1.36 V the address byte is unreadable, so no device acknowledges; S2). The trace keeps that difference visible: the evidence is the measured 3,982 ns, and the timeout is labelled as the model's rendering, never as a measured waveform.

## S7 — Proposal (≤ 300 words)

**Transport: c.** libngspice in-process, stepped synchronously. Fastest (4.0 ms per transient; b 5.9, a 10.0), byte-identical, exact on quantum boundaries. The only ngspice route the unchanged sandbox runs: a needs `/tmp`, b a pty (F2, F3).

**`unsafe`.** It lives in one crate outside the workspace, excluded like `boards/`, in a single `ffi` module: 82 lines, declared from the pinned header (F1). The crate builds a bridge binary, which the CLI runs as a sandboxed tool. The bridge starts Renode as its own child, so the CLI stays free of `unsafe` and `Invocation` keeps stdin `/dev/null` (F6). Not a `RUL-12` amendment (Rust; ngspice consumed, not patched), but a workspace exception, as `RUL-04` made for `mmio`.

**Fault path: iii.** Stock `BME280`; the fault is its absence, either never declared or `Unregister`ed. No language layer; the firmware times out at TXIS, deterministically. It stands in for the NACK an unreadable address draws (1.36 V peak, below VIH), and the trace says so (S6). Route i remains available within `RUL-12` for wrong data. Route ii (C#) is not proposed.

**Quantum.** A static rise fault needs no in-run lock-step: one analog pass runs before the boot. A rail droop needs about 10 µs (REASONED), measured at 9 s per 10 ms span (S4).

**Campaign.**
1. For each of 2.2 kΩ and 47 kΩ: solve, then emit the `.repl` and the netlist.
2. The bridge measures each line's rise (`analog.rise_time_measured`).
3. If the rise exceeds the ceiling, the bridge records `bridge.fault_state` and declares no device.
4. Renode boots, bounded by `RUL-13`, and the firmware's result is recorded (`firmware.observed`).
5. The verdict: 47 kΩ fails (3,982 > 300 ns; TXIS timeouts) and 2.2 kΩ passes.
6. The run is archived and replayed twice.

*285 words, counted with `wc -w` on the section above.*

## Gates (spike)

- **Workspace gates,** run in `$IMAGE` on the PR head: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. All three pass: fmt clean, clippy clean, and 31 test suites with 530 passed and 0 failed (`out/gates.txt`). The workspace is `ecdb86c`'s; nothing in it changed.
- **The probe crates build without warnings:** `ng-probe` and `aet2101-lockstep` (`cargo build --release` in `$IMAGE`). The firmware's `no-reset` build carries one `unused_mut` warning (`out/s3-build.txt`).
- **Adversarial pass:** at the build, as the work order's header sets it (`roadmap.md` §6). This PR is the spike only.
- **Reduction pass (diff):**
  - −3 files and 1 code path, about 40 lines;
  - 0 duplicates removed;
  - 3 dead paths removed:
    - `probes/s5-ngspice.sh`, superseded by `s5-ngspice-diag.sh` and `s5-ngspice-c.sh`, and cited by no output;
    - the S4 smoke inputs `blank.setup` and `blank.reads`;
    - the dummy-owner swap in the `ioc` path, replaced by an `Option`;
  - 0 protected kept;
  - 1 finding: `probes/sbx.sh` copies `run.rs`'s `SYSTEM_DIRS` by design, so the probe exercises the sandbox as it stands. The build deletes it with the rest of `spike/`.

## Findings

- **F1 · The image has no `libngspice-dev` · `container/` · medium · gap · decision-needed.**
  - The image has no `sharedspice.h` and no unversioned `libngspice.so`; only `libngspice.so.0` (H3).
  - So `bindgen`, the route `RUL-12` names, cannot run in the image.
  - The probe hand-declares eight functions and four types from the header fetched at the pinned tag, cites its line numbers, and links `-l:libngspice.so.0`. A dependency's `rustc-link-arg` does not reach its dependents, so each binary needs its own `build.rs`.
  - Recommendation: keep the hand declaration, pinned by the header's sha256 and checked by a test that the declared signatures match. Alternatively, add `libngspice-dev` to the image, an image change for the architect.
- **F2 · `ngspice -p` and `-i` segfault without a tty · ngspice 44.2 · high for route b · measured.**
  - Exit 139, after "no graphics interface", even on the simplest deck.
  - `-t dumb` does not help, and `-s` reads the netlist from stdin instead.
  - It works under a pty (`script -qec "ngspice -p -t dumb" /dev/null`): `out/s1-b-pty.txt`, `out/s4-ngspice-stop-resume.txt`.
  - Route b therefore needs a pty, which the sandbox denies (F3).
- **F3 · ngspice's `tmpfile()` and the pty both need grants the sandbox does not give · `run.rs` Landlock · high for routes a and b · measured.**
  - Batch mode calls `tmpfile()`, which glibc places in `/tmp` regardless of `TMPDIR`. Route a passes only with `/tmp` read-write.
  - Route b also needs `/dev/ptmx`, `/dev/pts`, `/dev/null` and `/dev/tty` (`out/s5-ngspice-diag.txt`, `out/s5-ngspice-c.txt`).
  - Route c needs none of these.
  - Recommendation: route c (S7). Granting `/tmp` would let a tool write every other process's files there.
- **F4 · Correction to the desk spike, Q5 · `architecture.md` §2.16 · low · correction.** An absent I2C address leaves **TXIS** unset, so the write phase never starts. The firmware's timeout is at TXIS, not TC (`out/s3-route-iii.txt`). §2.16 says "an absent device leaves the transfer incomplete", which holds. Q5's "TC is never set" is true but is not the first wait to expire.
- **F5 · `DummyI2CSlave` cannot produce a timeout · Renode 1.17.0 · medium · measured.** It pads a short read with zeros, so route i delivers only wrong data, never no completion. A no-response fault in route i therefore reads as 0x00, not as a bus failure (`out/s3-route-i.txt`).
- **F6 · The CLI's `Invocation` gives every tool `stdin` from `/dev/null` · `run.rs` L339 · medium · build constraint · decision-needed.**
  - S4's channel to Renode is a pipe on its stdin (H1, H2), so a CLI that drove Renode itself would need `Invocation` to pipe stdin and stream stdout.
  - The proposal avoids that: the bridge, a sandboxed tool, owns Renode as its child (S7).
  - No route needed a descriptor above 2 or the named FIFO.
- **F7 · The fault lags its measurement by one quantum · the lock-step order · medium · measured.**
  - In the work order's order (Renode, then ngspice, then the fault for the next quantum), the firmware runs quantum 1 unfaulted.
  - At 1 ms quanta it completed 9 good reads before the 47 kΩ fault took effect; at 100 µs, 1 (`out/s4-matrix.txt`).
  - For a static fault, the analog measurement belongs **before** the first quantum (S7).
- **F8 · `stop when` / `resume` overshoots the boundary; inversion of control lands on it · ngspice · low · measured.**
  - `stop when time > T` halts at the first accepted step past T: 1.000005e-4 for T = 1e-4. It also stays armed until `delete all`.
  - The `ioc` backend's breakpoints land exactly on T: `1.000000000000e-4` (`out/s4-ngspice-stop-resume.txt`, and the `spice_t` column of the `.seq` files).
  - Both are deterministic. They give different hashes, and the same firmware counters.
- **F9 · The S4 waveform is synthetic · the lock-step itself · high · honesty · decision-needed.**
  - Renode's I2C model is transaction-level, so the owner gets no bit timing from it. The analog line in S4 is driven by the owner's own byte-window pattern, not by the firmware's traffic.
  - For a static pull-up fault that is enough: the rise time depends on R and C, not on the bytes.
  - For a fault that depends on traffic (a load-dependent droop), `AET-WO-2200` must derive the stimulus from the transactions the sensor model sees: its `Write`/`Read` calls and their virtual times. REASONED; not measured here.
- **F10 · `INV-04` was measured as two live runs agreeing, not as replay from the archive · S4 · medium · scope.** The state-sequence hash is over simulator state only: quantum, virtual time, the result block, `ExecutedInstructions`, `PC`, the line's time and voltage, the rise time and the fault state. Two runs agree in every configuration. Replay from a CBOR trace without the solvers, which is the literal `INV-04`, needs the bridge's record set in `AET-WO-0400`'s archive and is `AET-WO-2200`'s to show.
- **F11 · Monitor overhead dominates small quanta · Renode monitor on stdin · medium · measured.** With a fresh `HOME`, Renode alone costs 1.8–2.0 ms per quantum with one read and 6.6 ms with twelve (`out/s4-timing.txt`). That is the floor below 10 µs. Recommendation: the bridge reads only what changed, or reads at events.
- **F12 · `Unregister` deletes the peripheral's name · Renode 1.17.0 · low · measured.** Re-adding takes `LoadPlatformDescriptionFromString` and creates a fresh model: state is reset, and `Register` with the old name fails. That is enough for a removal fault, but it is not a pause.
- **F13 · Roslyn compiles at every launch · route ii · low · measured.** +0.67 s per Renode start, not cached. The `include @….cs` must precede `LoadPlatformDescription`.
- **F14 · The committed `fw.elf` differs from a rebuild in 38 bytes · `spike/2101/renode/` · low · provenance.**
  - The differing bytes are the mangled-name hash text in the symbol string table, past offset 131,561.
  - The loadable sections are identical (`cmp -l`; the target directory differs). The S3 outputs were taken on the committed ELF.
  - `fw-repeat.elf` (`951ca2cd…`), which S4 used, matches its rebuild exactly.
- **F15 · TIMINGR is tagged, not modelled · Renode `STM32F7_I2C` · info.** Every run logs `Unhandled write to offset 0x10`. Bus speed is not modelled at all, which is one more reason the trace must not present a Renode-side timing as a measured one (S6).
- **F16 · Renode's monitor history slows every command as it grows · Renode 1.17.0 · high for the bridge · measured · decision-needed.**
  - Every monitor command is appended to `$HOME/.config/renode/history`, and each later command is slower: 200,000 lines take a quantum from 28.5 ms to 113 ms, with the state hash unchanged (`out/s4-history.txt`).
  - A lock-step run feeds its own history. At 1 µs × 4 ms, 64,000 commands make it 15.5 ms per quantum, against 7.4 ms with the history symlinked to `/dev/null` (`out/s4-timing.txt`). The 1 µs × 10 ms run never finished inside 900 s.
  - The sandbox gives each call a fresh `HOME`, so history never carries across calls, but within one long call it still grows.
  - It also inflated the wall times of the matrix runs, which shared one container's `HOME`. Those times are not cited; the hashes are.
  - Recommendation: the bridge points Renode's history at `/dev/null` (measured above) or finds a Renode option that turns it off (none was found).
- **F17 · The spike's memory caps did not cover its containers · Furnace practice · medium · process · measured.**
  - `systemd-run --user --scope -p MemoryMax=…` around `podman run` caps only the podman client: 30 MB in the scope. Rootless podman places the container in its own `libpod-<id>.scope`, whose `memory.max` read `max`.
  - Renode never ran away here, but `F21` from `AET-WO-2100` (an aborted machine growing without bound) was not fenced by those scopes as the spike assumed.
  - Recommendation for Furnace's contract: cap the container itself (`podman run --memory`), or run the work inside the scope with no container.
