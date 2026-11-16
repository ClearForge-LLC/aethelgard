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
