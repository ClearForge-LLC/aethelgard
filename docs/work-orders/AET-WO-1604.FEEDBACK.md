# AET-WO-1604 — FEEDBACK

**Branch:** `wo/aet-1604-cli-backlog` · **Base:** `ecc6be9` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. · **Cadence:** build, with one measured stop (§1.3). · **Work order:** `docs/work-orders/AET-WO-1604.md` on `origin/claude/argon-repo-docs-synthesis-rjrb0d` (`e5a87ae`, docs PR #198).

Labels: **measured** (run here), **read** (read at source), **inferred**.

## 0. Read at source before the predictions

These were read, not run, before any prediction below was written. JLCPCB's pages and PR #190's package were also read before this file existed, so §1.3's facts are recorded here as read, never as predictions.

- **R1 · `run.rs` has no standalone sandbox probe.** The Landlock ABI and the seccomp result exist only as the trampoline's status line (`run.rs` `confine_main`, `:1201-1276`): `sandbox abi=N uid_boundary=B`, or `unavailable <layer> <reason>`, which `parse_status` (`:826`) turns into `SandboxReport` or `Stop::sandbox(layer, …)` (exit 4, `SandboxUnavailable`). A real `Invocation::run` is the only way to learn either without a second implementation.
- **R2 · The scratch check is `main.rs`'s `scratch_base()` (`:625`), not `run.rs`'s** as §1.1 says. It creates the default base (`$HOME/<USER_SCRATCH>`, or `ROOT_SCRATCH` as root) when `TMPDIR` is unset, and refuses `ScratchUnsafe` (exit 4) on any ancestor not owned by root or this uid, writable by group or other, or (as root) not searchable by others, or a real path outside `[A-Za-z0-9/._-]`. `Scratch::new()` (`:722`) depends on it, so with an unsafe scratch no tool can be probed at all.
- **R3 · Every version probe already runs inside the sandbox:** `crate::version(Invocation)` (`main.rs:794`) for `kicad-cli version` (`:912`, `:1016`, `fab.rs:174`, pinned by `kicad::pinned` at `:912` only), `cargo --version` (`firmware.rs:643`), `cargo kani --version` (`:1279`, with `KANI_HOME`, `RUSTUP_HOME`, `PATH` from `required_var`), and `renode --config <file> --version` (`:1666`, with the two `DOTNET_*` variables and `/proc`, `/dev/urandom` read-only), the version pinned to `Renode v1.17.0` only when a boot is planned.
- **R4 · ngspice has no binary and no version probe.** It is `libngspice.so.0`, linked by soname into `aethelgard-bridge` (`bridge/build.rs:22`), which is looked for beside the CLI (`analog.rs:120`). The bridge prints its identity line first in every mode (`bridge/src/main.rs:78`) and, with no argument, exits 2 with its usage (`:91-93`). The `44.2` the CLI names (`analog.rs:39`) is a constant, checked against what the bridge's answers say; no request-free run reports it. `bridge/` is outside this work order's fence.
- **R5 · The reaper's record carries no time.** Its status file is the sandbox line, `ended …`, then `peak rss_kib=N` for a bounded run (`run.rs:1000-1048`); `2302` surfaced the peak as the verdict's `memory` (`main.rs:1825`, `verdict.rs:126`). The bounded boot test runs with `--spec`, so its verdict carries `memory`.
- **R6 · `LineBound` has no test-pad field** (`eda/src/extract.rs:597`); the eda counts a test pad's copper as `Load::TestPad` (`:1020-1023`, `:1138`) from the schematic's parts, which come from `spec.declared_test_points()`. `eda/src/extract.rs` is outside the fence.
- **R7 · `extract`'s `conditions` reach the fabrication notes.** `verify` copies the last `extract` report's `conditions` into the package's `FAB-NOTES.md` (`verify.rs:1590`, `:1336`, `fab.rs` `notes`), which `MANIFEST.json` hashes. A board with a test pad on a bounded line (Sentinel-01's bench board, `TP1` on `I2C_SDA`, PR #190) therefore gets a changed `FAB-NOTES.md` and manifest from §1.5, beyond §2's "only the two new files". See Q2.
- **R8 · The MCP tool list is computed in the binary crate** (`mcp.rs:172`), and `mcp_tier1.rs` checks the tool names only, not their schemas.

## 1. Predictions (written before any measured run)

| # | fixture | prediction | label |
|---|---|---|---|
| P1 | `doctor` on Furnace's host, outside the image, `TMPDIR` unset | exit 1; `kicad-cli`, `renode`, `cargo-kani` and the bridge missing, each naming the subcommands it blocks; `cargo` found; scratch safe; the sandbox line from a sandboxed `aethelgard --version` reports Landlock ABI ≥ 6, seccomp installed, `uid_boundary` false (`tool_uid`) | inferred |
| P2 | `doctor` with `TMPDIR=/tmp` | exit 1, `ScratchUnsafe` naming `/tmp` (mode 1777), every tool reported as found or missing on `PATH` but not version-probed, and no sandbox line | read (R2) |
| P3 | `doctor` with `PATH` holding a `kicad-cli` stub that prints `10.0.7` | the version is the stub's, judged by `kicad::pinned` as `check` judges it, and the stub ran confined (its sandbox line is the probe's) | inferred |
| P4 | `doctor` inside the pinned tools image under `capped-run`, as root | exit 0 except the bridge if the image does not carry it beside the binary; `uid_boundary` true unless rootless podman makes the namespace non-initial (`user_namespace`, `AET-WO-1601` R4) | inferred |
| P5 | the snapshot test after a required argument is added to a real subcommand | fails, naming the tool and `inputSchema.required` and `inputSchema.properties.<long>` | inferred |
| P6 | the bounded memory boot on Furnace | ends 5–15 s after start, `memory.peak_rss_kib` > `memory.bound_kib` (1,048,576) | inferred |
| P7 | `extract` on a spec with `TP1` on `I2C_SDA` | today: the verdict names no probe; after §1.5, one condition naming `TP1` and `I2C_SDA` | read (R6) |
| P8 | `cargo test` on the CLI after §1.5 | no existing tier-1 golden moves; a tier-2 package golden carrying a test point on a bounded line would move (`FAB-NOTES.md`) | inferred |
