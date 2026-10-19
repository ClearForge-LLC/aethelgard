# AET-WO-0100 — Workspace skeleton: seven crates that build, a fail-closed specification type, and a `validate` subcommand that proves the CLI contract

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (currently `5fa3b12`, the merge of PR #2; verify live HEAD before branching).
**Branch:** `wo/aet-0100-workspace`.
**Author:** Claude (architect, this session) · **Builder:** Claude Code session, tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation) · **Phase exit gate:** `docs/roadmap.md` §5 P0. This work order satisfies the first and fifth clauses of that gate (`cargo fmt`/`clippy`/`test` green; `aethelgard validate` fail-closed) and prepares nothing else.
**Grounds:** `docs/northstar.md` §4 `INV-01`, `INV-06`; `docs/architecture.md` §3 (the seven parts and the seams table), §4 `RUL-01`, `RUL-04` (the `unsafe` carve-out is for *generated* board crates, none of which exist yet), `RUL-10` (the CLI is the contract), `RUL-11`, `RUL-12`; `docs/roadmap.md` §5 P0 and §7. Input material, not rulings: Argon's type sketch in `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md` (`TopologyGraph`, `VoltageDomain`, `NetConstraint`, `IrValidationError`).

> **What this is:** the first code in the repository. One Cargo workspace with the seven parts `architecture.md` §3 names, every crate forbidding `unsafe`, the first specification types in `aethelgard-ir` with a `validate()` that refuses rather than defaults, and an `aethelgard` binary whose only subcommand is `validate`, so the JSON-in, JSON-out, exit-code contract from `RUL-10` exists as running code before `AET-WO-0300` fills it in. It is NOT the solver, NOT any emitter, NOT the tool server, and NOT the container. Why now: nothing in P0 or P1 can be measured until something compiles, and the steering documents are ratified.

**Cadence:** build. Nothing here is novel enough for a spike. One PR, left unmerged for review.

## 1. Scope

1. **Root `Cargo.toml`**, `resolver = "2"`, members exactly: `crates/aethelgard-ir`, `crates/aethelgard-eda`, `crates/aethelgard-svd-hal`, `crates/aethelgard-twin`, `crates/aethelgard-chaos`, `crates/aethelgard-evolve`, `crates/aethelgard-cli`. `[workspace.package]`: `version = "0.1.0"`, `edition = "2024"` if the pinned toolchain accepts it, else `"2021"` and say which in FEEDBACK; `license = "MIT OR Apache-2.0"`; `repository`. `[workspace.lints.rust] unsafe_code = "forbid"` and every crate opts in with `[lints] workspace = true`. Also put `#![forbid(unsafe_code)]` at each crate root; the workspace lint is the enforcement and the attribute is the visible statement, and the test in item 7 checks both.
2. **`rust-toolchain.toml`** pinning the stable channel to the exact version `rustup` installs on Furnace on the day you run it (`rustc --version`), so CI and the box agree. Record the version in FEEDBACK.
3. **`crates/aethelgard-ir`**: `TopologyGraph { project_name, voltage_domains: Vec<VoltageDomain>, net_constraints: Vec<NetConstraint> }`, `VoltageDomain { id, nominal_millivolts: u32, tolerance_permille: u16 }`, `NetConstraint { net_name, max_capacitance_femtofarads: u64, max_rise_time_nanoseconds: u32, impedance_ohms: Option<u32> }`, `IrValidationError` as a `thiserror` enum with one variant per refusal, each naming the offending id. All `serde` types carry `#[serde(deny_unknown_fields)]`: an unknown field is a refusal, not a silent drop (`INV-06`). `TopologyGraph::validate(&self) -> Result<(), IrValidationError>` refuses: empty or whitespace-only `project_name`; empty, whitespace-only, or duplicate domain `id`; `nominal_millivolts == 0`; `tolerance_permille` outside `1..=500`; empty or duplicate `net_name`; `max_capacitance_femtofarads == 0`; `max_rise_time_nanoseconds == 0`; `impedance_ohms == Some(0)`. Trim before comparing ids, and refuse ids that differ only by surrounding whitespace as duplicates. Do not add fields beyond these three types: the graph proper is `AET-WO-1100`.
4. **The five library crates** `eda`, `svd-hal`, `twin`, `chaos`, `evolve`: each a `lib.rs` with the crate-level doc comment stating its one-line responsibility from `architecture.md` §3, the `forbid`, and `pub use aethelgard_ir::*` re-exports of the shared types. **No other code.** They exist so the dependency direction (`architecture.md` §3: every crate depends on `ir`, `ir` depends on none of them) is enforced by Cargo from day one, and so a later work order cannot "accidentally" define a second `VoltageDomain` (`INV-01`).
5. **`crates/aethelgard-cli`**, binary name `aethelgard`, built on `clap` with derive. Subcommands in this work order: `validate` and the implicit `--version`. `validate` reads one JSON `TopologyGraph` from stdin (or from `--file <path>`), runs `validate()`, and writes exactly one JSON object to stdout: on success `{"ok":true,"project_name":"…","voltage_domains":N,"net_constraints":M}`; on refusal `{"ok":false,"error":{"kind":"<variant name>","id":"<offending id or null>","message":"…"}}`. Exit codes: `0` valid, `2` invalid specification, `3` unreadable or non-JSON input, `64` usage error, `70` internal error. Nothing but JSON goes to stdout; diagnostics go to stderr. Document the codes in `crates/aethelgard-cli/README.md`, because `AET-WO-0300` and `AET-WO-1500` inherit them.
6. **CI**: `.github/workflows/ci.yml` on pull requests and pushes to `main`, Ubuntu runner, toolchain from `rust-toolchain.toml`, steps `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, then a shell step that pipes `examples/sentinel-01-minimal.json` into `cargo run -q -p aethelgard-cli -- validate` and asserts exit `0`, and pipes `examples/invalid-zero-domain.json` and asserts exit `2`. Both example files are checked in under `examples/`.
7. **A workspace-shape test** (in `crates/aethelgard-cli/tests/workspace_shape.rs` or a small `xtask`, your call, say which): parses the root `Cargo.toml` and every member's `Cargo.toml` and `src/lib.rs`/`main.rs`, and fails if any member lacks `[lints] workspace = true`, lacks `#![forbid(unsafe_code)]`, or (for the five library crates) has a dependency other than `aethelgard-ir`, `serde`, and `thiserror`. This is the mechanical half of the `INV-01` dependency direction.
8. **`CLAUDE.md`** at the root already exists on `main` (the architect wrote it alongside this work order). Keep it true if anything you build changes what it says; do not expand it.
9. **`.gitignore`** for `target/`, and nothing that would hide a source file.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-01` One source.** Shared types live in `aethelgard-ir` only. The five library crates re-export; none declares a struct, enum, or type alias of its own. The CLI declares only its report and error-envelope types, which are presentation, not specification.
- **`INV-06` Absence halts.** `validate()` and the `validate` subcommand refuse every malformed input with a typed error naming what was wrong. No default is ever substituted: a missing field is a refusal, an unknown field is a refusal, a zero where a quantity must be positive is a refusal. Exit codes are nonzero on every refusal path, and a refusal never prints `"ok":true`.
- **Protected surfaces diff to empty:** `docs/northstar.md`, `docs/architecture.md`, `docs/roadmap.md`, `README.md`, `LICENSE-*`, `docs/inputs/**`. If the build needs any of them changed, that is a FEEDBACK `decision-needed: yes`, not an edit.
- **Secrets by name only.** Nothing in this work order needs a credential. If you find yourself typing one, stop.
- **One PR, left unmerged.** Push with `gh-write` per the furnace777 runbook; each push is minted and tapped on the phone. The builder never merges.

## 3. Tests and acceptance, proven not asserted

All of these run on Furnace in the `aet` session, and again in CI on the PR. Record the exact commands and their output tails in FEEDBACK's gates line.

1. `cargo fmt --all -- --check` exits 0.
2. `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
3. `cargo test --workspace` exits 0 and the count includes, by name: a serde round-trip of a valid graph; one negative test per `IrValidationError` variant; a test that an unknown top-level field and an unknown nested field each produce a deserialization error; the workspace-shape test from §1.7.
4. `cargo run -q -p aethelgard-cli -- validate < examples/sentinel-01-minimal.json` prints one line of JSON with `"ok":true` and exits 0.
5. `cargo run -q -p aethelgard-cli -- validate < examples/invalid-zero-domain.json` prints one line of JSON with `"ok":false` and `"kind":"ZeroNominalVoltage"` (or your variant name) and `"id":"<the domain>"`, and exits 2.
6. `printf 'not json' | cargo run -q -p aethelgard-cli -- validate` exits 3 with a JSON error on stdout and nothing else on stdout.
7. `cargo run -q -p aethelgard-cli -- validate < /dev/null` exits 3, not 0, and not a panic.
8. `cargo run -q -p aethelgard-cli -- --version` prints `aethelgard 0.1.0`.
9. CI on the PR is green on the first push after you have run 1 through 8 locally. A red first CI run is a finding, not a retry.

## 4. Scope fence, what is NOT in this work order

- No `z3`, no constraint solving of any kind (`AET-WO-1200`).
- No `svd-parser`, no vendor register description ingestion, no MCU type (`AET-WO-1100`).
- No invocation of `kicad-cli`, `renode`, `ngspice`, or `cargo kani`, and no code that shells out to anything (`AET-WO-0200`, `AET-WO-0300`).
- No `rmcp`, no `mcp` subcommand, no network listener (`AET-WO-1500`).
- No Cap'n Proto, no trace archive, no R2 (`AET-WO-0400`).
- No subcommand other than `validate` (`AET-WO-0300` owns `build`, `check`, `prove`, `simulate`).
- No Dockerfile, no Nix flake (`AET-WO-0200`).
- No Python, no shell scripts beyond the CI workflow steps (`RUL-12`).
- No `unsafe`, anywhere, for any reason. The `RUL-04` carve-out is for generated board crates, which are not workspace members and do not exist yet.
- No edits to the steering documents or the README. Findings about them go in FEEDBACK.

## 5. Adversarial pass, break it before calling it done

Hand this to a fresh subagent if you can spawn one; otherwise do it yourself after a deliberate mode switch, as an attack on the finished branch. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. **Hostile JSON into `validate`:** numbers that overflow `u32`/`u64`, negative numbers, floats where integers are declared, `null` for required fields, duplicate keys, a 50 MB document, nesting 10,000 deep, a BOM at the start, CRLF, an id of 1 MB of whitespace, Unicode confusables as ids (`VDD_3V3` vs a Cyrillic `V`). The required behaviour is a refusal with exit 2 or 3 and a JSON error. A panic, a hang, or exit 0 is a bug.
2. **Violate `INV-01` by hand:** add `pub struct VoltageDomain` to `aethelgard-eda` and confirm the workspace-shape test fails. Add a `serde_json` dependency to `aethelgard-twin` and confirm it fails.
3. **Violate the `unsafe` forbid:** put `unsafe {}` in each crate in turn and confirm `cargo check` refuses every one, including the CLI.
4. **Interrupt:** send SIGPIPE by piping `validate` into `head -c 1` with a valid input; confirm no panic trace reaches stderr in a way that would confuse a caller, and the exit code is documented.
5. **Remove a dependency:** delete `Cargo.lock` and rebuild; confirm the lock regenerates and tests still pass. Commit the lock (this is a binary workspace).
6. **Look for assumptions true by luck:** does anything depend on the CWD? On `HOME`? On a locale? Run the test suite with `LC_ALL=C` and from a different directory.

## 5a. Reduction pass, diff-sized

Run the `reduction-pass` skill against the finished diff after the adversarial pass. Likely targets: error variants that cannot be reached, re-exports nobody uses yet, CLI flags added "for later." One line in FEEDBACK's gates with what was removed.

## 6. Upward-feedback directive

Write `docs/work-orders/AET-WO-0100.FEEDBACK.md` on the branch. The file is the record; a chat summary is not. Schema per entry:

`finding · where (file:line) · type (note | risk | scope-question | bug) · recommendation · decision-needed (yes/no)`

It must carry:

- **What did not work, and why**, alongside what did. The approach you abandoned and the reason is the part the next builder needs.
- **A gates line**: every command from §3 with its exit code and test count, the clippy status, the protected-surface diff (`git diff --stat origin/main -- docs/northstar.md docs/architecture.md docs/roadmap.md README.md` must print nothing), the CI run URL, and the reduction-pass one-liner.
- **What was deliberately not built**, confirming each line of §4 held.
- **Measured facts for the architect to promote**: the pinned Rust version; whether edition 2024 was accepted; Furnace's `cargo` and `rustup` versions; how long `cargo test --workspace` takes cold and warm on Furnace; whether Furnace's egress reached crates.io without any special configuration. These land in `architecture.md` §2 by amendment, so state them as numbers with the command that produced them.
- **Promote-or-needs-none**: this is the first work order; there is no prior FEEDBACK to promote. Say so.
- Every `decision-needed: yes` gets a ruling from the architect before `AET-WO-0200` starts.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears. Do not route around them.

- Any step that needs `sudo`, a group change, or a system package. The deny list on Furnace blocks every spelling; if a build step seems to need one, the work order is wrong, not the deny list.
- Any credential, token, or key about to land in a file, an environment variable in the repo, a log, or the FEEDBACK.
- A design that needs `unsafe`, a `build.rs` that executes anything, or a proc-macro crate of our own.
- Any change required to a protected surface (§2).
- A gate in §3 that cannot be shown green and cannot be explained as a defect in this work order. A gate that is wrong is a finding; a gate that is skipped is a stop.
- `rustup` or `cargo` needing to fetch from anywhere other than `static.rust-lang.org`, `crates.io`, and GitHub. Furnace's egress posture is the operator's, not yours.

## 8. Kickoff prompt

**Step 1, in its own turn, confirm the goal mechanism is live:** send `/goal` alone and read back what is in force.

**Step 2, in its own turn, set the goal** (short enough to type inline; the detail is in this file):

```
/goal AET-WO-0100: a PR from wo/aet-0100-workspace is open and unmerged, CI green on its head, in which `aethelgard validate` refuses every malformed input with a JSON error and a nonzero exit, no crate can compile unsafe code, and FEEDBACK records what did not work.
```

Confirm the `Goal set:` echo. No echo means it did not take; send it again.

**Step 3, the kickoff message:**

```
Sync first: `git -C ~/aethelgard fetch origin && git -C ~/aethelgard checkout -b wo/aet-0100-workspace origin/main`. Expected base: 5fa3b12 (the merge of PR #2). If HEAD differs, read what landed before you start.

Read, in order: CLAUDE.md, docs/northstar.md, docs/architecture.md, docs/roadmap.md, then your work order at docs/work-orders/AET-WO-0100.md. Argon's type sketch in docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md is input, not a ruling; where it and the work order differ, the work order wins.

Cadence is build. No spike. One PR from wo/aet-0100-workspace, left unmerged.

Ratified and not up for relitigation, with the reasons in architecture.md §4: Rust everywhere in this repo (RUL-12); the CLI is the contract and the MCP server is a later subcommand of the same binary (RUL-10); MIT OR Apache-2.0 (RUL-11); unsafe is forbidden in every workspace crate, because the RUL-04 carve-out is only for generated board crates that do not exist yet.

Invariants you touch: INV-01 (shared types live only in aethelgard-ir; the five library crates re-export and declare nothing) and INV-06 (validate refuses, never defaults; unknown fields are refusals; every refusal path exits nonzero). Protected surfaces that must diff to empty: the three steering docs, README.md, LICENSE-*, docs/inputs/**.

Flag-and-stop: anything needing sudo or a system package; any credential anywhere; any unsafe; any protected-surface edit; a §3 gate you cannot show green.

Furnace facts you need: there is no Rust on the box yet. Install it as forge with rustup's minimal profile into ~/.cargo (no sudo), then pin the exact stable version in rust-toolchain.toml and record it. Push with gh-write; each push is tapped on the phone, so batch your pushes.

Close: the §5 adversarial pass (fresh subagent if you can), the §5a reduction pass, then docs/work-orders/AET-WO-0100.FEEDBACK.md with the schema and the gates line, then the PR, left unmerged.
```

## 9. Operator setup before kickoff (human-track, Scotty, from the phone)

None of this is the builder's. Each line is short enough to paste from Termux. `furnace` is the existing SSH host alias, **and it logs in as the operator user `ikthys777`**; every builder step below runs as `forge`, so each one overrides the user with `-l forge` (the phone's key is `forge`'s only login, per `furnace777` `docs/runbooks/agent-user.md`). The architect got this wrong once on 2026-10-19, created `aet` under `ikthys777`, and had to kill it; the lines below are the corrected ones.

1. Clone, as `forge`, in a plain SSH shell (not inside an agent pane). The repository is public, so no credential helper is needed:
   ```
   ssh -l forge furnace 'git clone -q https://github.com/ClearForge-LLC/aethelgard.git ~/aethelgard'
   ```
2. Create the durable session with the keeper, which maps it, stages `cc-resume`, and forces a save:
   ```
   ssh -l forge furnace 'cc-new aet ~/aethelgard'
   ```
   Then `ssh -l forge furnace 'bash ~/furnace777/scripts/user/verify-sessions.sh'` must end `0 failure(s)` with `aet` reported as an unmanaged map.local line (a WARN, not a FAIL).
3. Promote the map line by PR to `ClearForge-LLC/furnace777`, file `host/home/forge/.cc-sessions/map`, one added line: `aet:cc=~/aethelgard`. Until that merges, a restore after a reboot still brings `aet` back (map.local counts for restore), but `verify-sessions.sh` keeps warning.
4. The S25 alias, matching the `art`/`stake` precedent but with the user override: `alias aet='ssh -t -l forge furnace tmux attach -d -t aet'` in `~/.bashrc` on the phone, then `source ~/.bashrc`.
5. Attach with `aet`, start `claude` in the pane, and send §8's three steps.

**Why the name `aet`:** it is the work-order prefix, the branch prefix, and now the session and alias, so one string finds everything about this project in a `grep`, a `tmux ls`, and a shell history. The thematic alternative `anvil` (the furnace heats, the anvil shapes, Argon shields the weld) reads well but would be the only place that word appears.
