# AET-WO-1603 — FEEDBACK

**Branch:** `wo/aet-1603-sandbox` · **Base:** `8030fa5` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. · **Cadence:** build, with one measured stop (§1.1).

Labels: **measured** (run here), **read** (read at source), **inferred**.

## 0. Read at source before the predictions

These were read, not run, before any prediction below was written.

- **R1 · The engine's own generated firmware crate carries a Cargo configuration.** `generate` writes `firmware/.cargo/config.toml` (`svd-hal/src/board/firmware.rs:150`, `config()` at `:205`): `[build] target = "thumbv7em-none-eabihf"`, `target-dir = "../../../target/<name>-firmware"`, and `[target.thumbv7em-none-eabihf] rustflags = ["-C", "link-arg=-Tlink.x"]`. The tier-2 fixtures (`tests/fixtures/flood/.cargo/config.toml`) do the same. §1.2 as written ("refuse `.cargo/config.toml` anywhere from the crate root up") would refuse every generated crate, which is a §7 condition. See D1.
- **R2 · No generated crate holds `rust-toolchain*`, a `build.rs` or a proc macro** (`svd-hal/src/board.rs:212-216`, `board/firmware.rs:148-152`; `board.rs:5504` asserts no `build.rs`).
- **R3 · The copy (`input::crate_dir`) refuses symlinks and special files and writes every file with the default mode** (`input.rs:53-86`, `:181`): nothing in the copy is executable.
- **R4 · `build` already refuses, after cargo has run, a unit from a manifest outside the copy, a source outside it, and any build script** (`firmware.rs:170-200`). `prove` has no such check (`main.rs:1034-1102`).
- **R5 · Where a firmware's bytes can reach Renode's stdout.** Every USART is given a file backend (`renode.rs:430-450`), so no analyzer prints to the log. Witness lines need a whole Renode log line, `[hh:mm:ss.nnnnnnnnn] [NOISY] <device>: …` (`renode.rs:1741-1790`); sentinels and read values are whole bare lines (`:1795`, `:1723`).
- **R6 · ArduPilot's real generated `hwdef.h` has conditionals, but not around the 33 macros.** step-g (`~/.cache/aet-4400a/d8/step-g/hwdef.h`, 1,459 lines) has about 25 `#ifndef`/`#if defined(…)` regions; all 33 `VAL_GPIOx_{MODER,AFRL,AFRH}` definitions are at conditional depth 0. seed-pkg (`~/.cache/aet-4400a/adv/cases/00-real-seed-pkg/hwdef.h`) is the same: 20 regions, 33 macros at depth 0. seed-pin and seed-af are not on Furnace; `ardupilot-image.yml` runs all four on any PR touching `crates/**`. The same file also defines `PIN_MODE_INPUT(n)` and friends (step-g L227-230).
- **R7 · `/dev/null`.** `build` and `prove` already grant `/dev/null` read-write with every ABI-3 right (`firmware.rs:332`, `main.rs:1082`); no other tool has it (`analog.rs:231`, `:313`, `fab.rs:170`, `main.rs:843`, `:1453`).
- **R8 · The ETXTBSY race.** `tools_tier1.rs:50-53` writes a stub with `fs::write` and `set_permissions`; every test spawns processes on other threads.

## 1. Predictions (written before any measured run)

| # | fixture | prediction | label |
|---|---|---|---|
| P1 | `build`: the board crate's firmware depends by path on a proc-macro crate inside the copy; the macro writes a marker into its cwd, `$HOME`, the copy, the original crate's absolute path, `/tmp` and the caller's real `~/.cache` | The macro runs inside the sandbox. Markers land only under the tool's directory (cwd, `HOME`, the copy); the others fail with `EACCES`. The build **passes** and writes an ELF: the macro's package has no `source` and its manifest is inside the copy | inferred |
| P2 | `prove` on the same crate | The macro runs, confined the same way; `prove` judges the harness as usual | inferred |
| P3 | a `build.rs` in the copy writing the same markers | Runs, confined the same way; `build` then refuses `CrateHasDependency` (after cargo ran); `prove` does not refuse | read (R4), inferred |
| P4 | a path proc-macro dependency outside the copy, at an absolute path outside the read set | cargo cannot read its manifest: exit 101 with no `build-finished`, `ToolError`; nothing runs | inferred |
| P5 | `firmware/.cargo/config.toml` in the copy setting `build.rustflags` (an unknown `-C` flag), `target.<triple>.runner` and `target.<triple>.linker = "/bin/false"` | rustflags and linker honoured (the build fails at the flag or at link: `build_failed`); `runner` ignored by `cargo build` | inferred |
| P6 | a `.cargo/config.toml` in a parent of the scratch directory that Landlock does not grant (`~/.cache/.cargo/`) | cargo sees it exists but cannot read it: `ToolError`, never a pass | inferred |
| P7 | the same in a parent Landlock does grant (scratch moved under a read-only granted directory via `TMPDIR`) | honoured: its rustflags reach rustc | inferred |
| P8 | `rust-toolchain.toml` in the copy naming a toolchain not installed | rustup tries to install it, the socket is refused: `ToolError`. With `RUSTUP_TOOLCHAIN` set by the caller, the file is ignored | inferred |
| P9 | `rust-toolchain.toml` with `path = "<dir in the copy>"` | rustup takes the copy's directory as the toolchain; its `bin/cargo` is not executable (R3): `ToolError` | inferred |
| P10 | a firmware whose ELF symbol and section names, USART bytes, semihosting `SYS_WRITE0` and unmapped accesses each carry a unique canary | no canary reaches Renode's stdout: symbols are not logged (no `LogFunctionNames`), USARTs go to file backends, semihosting is not enabled on the platform's Cortex-M, unmapped-access lines carry only numbers | inferred |
| P11 | a firmware that skips a device's transaction while replaying its passing witness lines through every channel | `fail` with the device's read unwitnessed; no forgery reaches `pass` | inferred |
| P12 | `hwdef.h` whose only `VAL_GPIOA_MODER` definition sits inside `#if 0 … #endif` | accepted today (the reader ignores directives) | read |
| P13 | a stub tool that opens `/dev/null` (as `Stdio::null()` does) under the sandbox, today | `EACCES`, as 2301 B2 measured | measured elsewhere (2301 B2) |
| P14 | the same with `/dev/null` granted read and write alone | opens; `/dev/zero`, `/dev/tty`, `/dev/urandom` and `/dev/full` still `EACCES` | inferred |
| P15 | a stress loop: one thread writes and execs a stub while another forks continuously | `ETXTBSY` within 200 iterations without a lock; write-to-temp then `rename` does not remove it (the inode's writers are counted, and a forked child holds the descriptor until it execs); a process-wide lock that excludes every spawn while a stub is open for writing does | inferred (execve(2)) |

## 2. Decisions taken

- **D1 · The pre-flight refuses a Cargo configuration unless it is exactly the one `generate` writes** (R1). §1.2's "refuse by name" and "the engine's own generated crates must still pass" conflict for `firmware/.cargo/config.toml`. The pre-flight accepts that one path only when its parsed keys are exactly `build.target = "thumbv7em-none-eabihf"`, `build.target-dir` (a string; `CARGO_TARGET_DIR` overrides it) and `target.thumbv7em-none-eabihf.rustflags = ["-C", "link-arg=-Tlink.x"]`, and refuses every other configuration file by name. So no generated crate is refused, and §7's condition does not fire. **Question for the architect:** confirm this reading.
- **D2 · `hwdef.h`: the refusal is scoped to the 33 macros' regions** (R6). The real headers hold `#ifndef` and `#if defined(…)` regions elsewhere, so refusing every region the reader cannot evaluate would refuse every CI board (a §7 condition). The reader refuses `HwdefConditionalUnsupported` when one of the 33 names is defined or mentioned inside any conditional region other than exactly `#if 0` (dead: its lines are not read) or `#if 1` (live), after splicing continuations and stripping comments. A target macro at depth above 0 on a CI board would be the §7 stop; on step-g and seed-pkg none is (read). **Question for the architect:** confirm this reading.

## 3. §1.1 Measured (main `8030fa5`'s binary, before any change)

**Where.** The pinned tools image (`container/IMAGE`, `aethelgard-tools@sha256:c69ff9bc…`) under `~/bin/capped-run` (scope `memory.max` 8589934592, `memory.swap.max` 0, `pids.max` 8192, read back for every run: `~/.cache/aet-1603/caps.log`), `--network none`, as the image's root, so every tool ran under the uid boundary's 65534 (`uid_boundary: false`, `user_namespace`, as rootless podman always gives). The ETXTBSY and `/dev/null` rows ran on Furnace as `forge`. Fixtures, scripts and raw output: `~/.cache/aet-1603/m1/` (caller crates), `m2/` (Renode), `evidence/`.

### Caller crates (P1-P9)

Each fixture is the reference board crate (`boards/sentinel-01`) with one change. The proc macro and the build script try a marker in the tool's cwd (the copy), `$HOME` (the tool's directory), `/tmp`, `/cache`, `/repo`, `/var/lib` and `/usr/local`, and report what succeeded through a compile error.

| # | fixture | measured | prediction held? |
|---|---|---|---|
| M1 | `build`, a path proc macro inside the copy, reporting | **The macro ran in the sandbox.** Only `$HOME` (the tool's directory) took a marker; the copy (root's 0444, uid boundary) and every other place gave `EACCES`. No marker outside the sandbox. Its report came back as `build_failed` | yes, but the copy itself is not writable under the uid boundary |
| M2 | the same, silent | **`pass`, an ELF written.** Nothing refuses an in-copy proc macro today | yes |
| M3 | `build`, a `build.rs` in the firmware, reporting through a generated `compile_error!` | ran in the sandbox; `$HOME` only, as M1; `build_failed`. A silent one would be refused after cargo ran (`firmware.rs:176`, read) | yes |
| M4 | `build`, a proc macro by absolute path outside the copy (`/cache`, not granted) | `ToolError`: `failed to get evilpm as a dependency`; nothing ran | yes |
| M5 | `firmware/.cargo/config.toml` + an extra `-C link-arg` | honoured: `build_failed`, `linking with rust-lld failed` | yes |
| M6 | `firmware/.cargo/config.toml` + `linker = "/bin/false"` | honoured: `/bin/false` executed as the linker | yes |
| M7 | `firmware/.cargo/config.toml` + `runner` | ignored by `cargo build`: `pass` | yes |
| M8 | the crate root's `.cargo/config.toml` (`linker = "/bin/false"`), an ancestor of `firmware/` | honoured by `build` (`linking with /bin/false failed`). A root `build.rustflags` alone is shadowed by the firmware's `target.<triple>.rustflags` (`pass`, the reference ELF) | yes; shadowing is a new detail |
| M9 | `prove`, the crate root's `.cargo/config.toml` (`build.rustc-wrapper = "/bin/false"`) | honoured: `ToolError`, Kani's `cargo metadata` ran `/bin/false … rustc -vV` | yes |
| M10 | `prove`, a path proc macro, then a build script, in the board crate | both ran; reported as `compile_error`. Silent proc macro: `pass` | yes |
| M11 | a `.cargo/config.toml` in a parent of the scratch directory that Landlock does not grant (`/var/lib/.cargo/`, scratch in `/var/lib/aethelgard`) | `build`: `ToolError`, `could not load Cargo configuration`; `prove`: `ToolUnusable`, the version probe exited 101. Never a pass | yes |
| M12 | the same in a parent Landlock grants (`TMPDIR=/usr/local/aet-scratch`, `/usr/local/.cargo/config.toml`) | honoured: `build` linked with `/bin/false`; `prove` ran the wrapper. A parent is reachable only when the scratch is under a granted directory, and every scratch ancestor is root's or the caller's (`AET-WO-1601`) | yes |
| M13 | `firmware/rust-toolchain.toml` naming an uninstalled channel; the legacy `rust-toolchain`; `path = "../tc"` (a toolchain directory in the copy) | each `ToolUnusable`, exit 4: rustup honoured the file in the version probe and failed (the copy's `tc/bin/cargo` is not executable, R3) | yes |
| M14 | the uninstalled channel with `RUSTUP_TOOLCHAIN=1.99.1` from the caller | `pass`: the variable overrides the file | yes |
| M15 | the reference crate, unchanged | `pass`, ELF `45a17992…`, the baseline's | — |

**What this means.** Every route ran inside the sandbox, and no marker reached any place outside the tool's own directory. The hardening the work order asks for is the pre-flight: today an in-copy proc macro passes `build` and `prove` (M2, M10), and a configuration or toolchain file reaches cargo or rustup before anything refuses it. **A parent configuration can be refused statically** (M12): the pre-flight walks every ancestor of cargo's working directory, so `--config` overrides are not needed (§1.2's condition for them does not hold).

### Renode's log (P10, P11)

The fixture `tests/fixtures/forge-log` carries one text, a leading newline then a whole line shaped like U2's I2C witness (`[00:00:00.000100000] [NOISY] i2c1.u2: C1603-…`) and the bare sentinel `AETHELGARD-BOOT-BEGIN`, through: two ELF names (a function symbol and a section name, patched in the string tables), USART1 (clocked, `UE|TE`, 85 bytes to `TDR`), semihosting `SYS_WRITE0` (`bkpt 0xab`), RCC, GPIOA and USART1 writes and an unimplemented USART1 offset (with the platform's access logging on), the FMC window, and a jump out of the firmware.

| # | measured | prediction held? |
|---|---|---|
| M16 | Renode 1.17.0 run directly on the reference platform with **every** log source at NOISY and access logging on for RCC, GPIOA and USART1 (a superset of what any boot command enables): **0** lines carry the canary. Semihosting is `[WARNING] cpu: Semihosting handler is not registered; R0=0x4 R1=0x80000E6` (numbers only); the unimplemented offset is `[WARNING] usart1: Unhandled read from offset 0x3FC.`; the jump is `Trying to find the mapping for offset 0xD0000000.` No line names a symbol. The USART's 85 bytes went to its file backend only | yes |
| M17 | `aethelgard simulate` on the same ELF and the reference specification: **`fail`**; U2's and U4's records invalid (no transfer), `pc_outside_firmware`, the serial capture `serial_capture_forged`. No forgery reaches `pass` | yes |
| M18 | No boot command enables `LogFunctionNames`, an analyzer or a semihosting handler (`grep` of `crates/` and `models/`: none) | read |

**§7 does not fire.** No forgery reached `pass`, and nothing measured needs a trace-format or specification change.

### `/dev/null` and the ETXTBSY race (P13-P15)

| # | measured | prediction held? |
|---|---|---|
| M19 | Tier 1's stand-in tool, `mode=devices`, under the real CLI on Furnace: `/dev/null` read, write, and a child started with `Stdio::null()` each `errno=13`; `/dev/zero`, `/dev/full`, `/dev/random`, `/dev/urandom`, `/dev/tty`, `/dev/ptmx`, `/dev/kmsg`, `/dev/mem` and a create in `/dev/shm` each `errno=13` (`channels_tier1::dev_null_alone_opens_inside`, failing before the fix) | yes |
| M21 | `hwdef.h` whose only `VAL_GPIOA_MODER` definition sits inside `#if 0 … #endif`: **accepted** by main's reader, the dead definition read as the value (`check::a_definition_inside_a_conditional_is_evaluated_exactly_or_refused`, failing before the fix) | yes |
| M20 | A stress program (`etx`, scratchpad): one thread writes a `#!/bin/sh` stub and execs it, 2,000 times, while four threads spawn `/bin/true` without pause. Plain write: **49** `ETXTBSY`. Write to a temporary name, close, `rename`: **78**. A process-wide `RwLock`, the write held exclusively while a stub is open for writing and shared around every `spawn`: **0** | yes: `rename` does not remove the race |
