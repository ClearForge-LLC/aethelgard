# AET-WO-1603 — Sandbox and tool-input hardening: the second review's open A15 notes, `/dev/null`, and a test race

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1603-sandbox`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (a trust boundary, roadmap §6).
**Phase:** backlog (roadmap §7.1), written 2026-11-16 at Scotty's direction: clear the backlog before new roadmap items. **Cadence: build, with one measured stop (§1.1).**

**Grounds (§7.1 rows, each with its source):**
- **Review 2, A15** (`docs/reviews/2026-11-15-mythos-5-2.md` §A15, a subagent's notes, unverified):
  - `build` and `prove` on a caller's crate run a path-dependency proc macro and honour the crate's `.cargo/config.toml` and `rust-toolchain.toml` inside the sandbox. The build-script refusal comes after cargo has run (`crates/aethelgard-cli/src/firmware.rs:176`).
  - Whether a Renode log call can print ELF-derived strings, which would let a firmware forge a witness line, was not settled.
  - `hwdef.h`: a `#define` inside a dead `#if 0` is accepted (`crates/aethelgard-pincheck/src/header.rs:39-46`).
- **`2301` B2:** a tool spawned under the sandbox cannot open `/dev/null`. `Stdio::null()` fails with `EACCES`, so the bridge inherits stdio.
- **`4220` F15:** a `tools_tier1` race. A stub `kicad-cli` written by a test is exec'd while a sibling test thread, forking, holds its write descriptor, and `execve` fails with `ETXTBSY`.

> **What this is:** each claim measured first, then closed by a refusal or a grant that a test pins, each fail-before and pass-after.
>
> It is NOT:
> - the uid boundary's deployment half (`AET-WO-1601`, the operator's);
> - `mcp --root` (deferred by Scotty);
> - a change to any emitted byte, trace format or specification version.

## 1. Build

1. **Measure every claim before changing anything (the stop).** Write FEEDBACK predictions first, then a hostile fixture for each:
   - a caller crate with a path proc-macro dependency that writes a marker file;
   - a `.cargo/config.toml` (in the crate and in a parent of the sandbox's working directory) setting `build.rustflags`, `target.<triple>.runner` or `linker`;
   - a `rust-toolchain.toml` naming another toolchain.

   For each, record what ran, where, and whether the marker escaped the sandbox's write set. Do the same for the Renode log: write a firmware that sends a byte string shaped like a witness line through every channel that reaches Renode's log (a USART with an analyzer, semihosting if the platform enables it, a register name), and record whether the judge accepts it.

   **STOP** with the measurements if a forgery reaches a `pass` verdict, **or** if closing a confirmed path needs a trace-format or specification change. Otherwise continue.
2. **Close the caller-crate path before cargo runs.** Add a pre-flight in `build` and `prove` that refuses the following by name, before any cargo invocation:
   - `.cargo/config` or `.cargo/config.toml` anywhere from the crate root up to the sandbox root;
   - `rust-toolchain` or `rust-toolchain.toml`;
   - a path dependency outside the crate that is a proc macro.

   The engine's own generated crates (`verify`, `generate`) must still pass. Registry proc macros pinned by the lockfile (`cortex-m-rt`'s) are not refused: name that boundary in FEEDBACK. Pass cargo `--config` overrides only if §1.1 shows that a parent config cannot be refused statically.
3. **The log channel.** Make the judge accept a witness line only from a source tag the engine's platform emits, with firmware-reachable output (an analyzer, semihosting) prefixed and never parsed as a witness. If §1.1 found no route, add the hostile firmware as a tier-2 test that pins the absence.
4. **`hwdef.h` conditionals.** The cross-check refuses any `#if`, `#ifdef`, `#ifndef`, `#elif` or `#else` region it cannot evaluate (`HwdefConditionalUnsupported`), rather than reading through it. If ArduPilot's real generated `hwdef.h` for the CI board contains such regions, evaluate only `#if 0` and `#if 1` exactly and refuse the rest.
5. **`/dev/null`.** Grant read and write on `/dev/null` alone in the Landlock rule set, so `Stdio::null()` works. Prove that no other device opens. Keep the bridge on inherited stdio unless the grant makes that unnecessary, and say which in FEEDBACK.
6. **The `ETXTBSY` race.** Remove the race; never retry on `ETXTBSY`. Create every stub once, under a process-wide lock, written and closed before any test execs one (for example a `OnceLock` set-up), or by write-to-temp then `rename`. Per roadmap §6, the fix forces the outcome: a test that execs a stub while a forking thread runs must pass every time (show 200 consecutive runs).

## 2. Acceptance

- Each §1 item has a fail-before and pass-after test (tier 1 where possible; tier 2 for Renode).
- Every existing board, firmware, verdict and trace is byte-identical.
- `verify` on the reference passes.
- **The CLAUDE.md gates pass.** CI is green on the balanced 8-shard plan (§6), and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-06` (refuse, never default); `INV-07`; `RUL-13`, `RUL-19`; architecture §6 (the sandbox).
- **Fence:**
  - `aethelgard-cli`: `run.rs`, `firmware.rs`, `renode.rs`'s log parsing only, and the pre-flight;
  - `aethelgard-pincheck`: `header.rs`;
  - tests and fixtures; FEEDBACK.

  `AET-WO-4230` (PR #161) is changing `renode.rs` and `firmware.rs`. Read its branch before touching those files, keep your hunks small, and merge `main` when it lands.
- No new trace kind, no specification field, no version bump.
- One PR, left unmerged. Write `~/.cache/aet-1603/pr-title-body.md`.

## 5. Adversarial pass, full

A fresh, read-only subagent attacks the finished pre-flight and log rule:
- a config reached by a symlinked directory;
- a `CARGO_HOME` or `RUSTUP_TOOLCHAIN` inherited from the caller's environment;
- a proc macro reached through a workspace member;
- a witness-shaped line split across two analyzer writes;
- `hwdef.h` with a conditional hidden by line continuation.

Every finding is fixed or named.

## 7. Flag-and-stop conditions

- **§1.1 finds a forgery that reaches `pass`,** or a fix that needs a format or specification change.
- **The pre-flight would refuse one of the engine's own generated crates.**
- **ArduPilot's generated `hwdef.h` needs conditionals the cross-check cannot evaluate exactly.**
