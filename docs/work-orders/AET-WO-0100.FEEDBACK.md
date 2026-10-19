# AET-WO-0100 — FEEDBACK

**Branch:** `wo/aet-0100-workspace` · **Base:** `29e1dfe` (the merge of PR #3; the work order's own kickoff names `5fa3b12`, which #3 superseded, as the operator's kickoff message said) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, tmux `aet` · **Date:** 2026-10-19.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## 1. What did not work, and why

The part the next builder needs. Each of these was tried, failed, and was replaced.

1. **serde's derived `Deserialize` accepted a second wire form.** The first IR derived `Deserialize` with `deny_unknown_fields` and was believed to refuse anything that was not the documented object. The adversarial pass found that serde's derive also accepts a positional JSON array, so `["p",[],[]]` returned `"ok":true` with exit 0. `deny_unknown_fields` and missing-field errors never run in that form. **Replaced** with `#[serde(remote = "Self")]` on the three types plus a ten-line `object_only!` macro that calls `deserialize_map` and hands the map to the derived code through `MapAccessDeserializer` (`crates/aethelgard-ir/src/lib.rs:55`). Duplicate, unknown, and missing-field detection still come from the derive. The positional form is now `SchemaViolation`, exit 2, with a regression test. **Lesson:** `deny_unknown_fields` is not "objects only".
2. **clap's derive default for a missing subcommand.** With a required subcommand, clap sets `arg_required_else_help`, so a bare `aethelgard` took the display-help path. The JSON `message` was the crate's about text, which says nothing about the error. **Replaced** with `arg_required_else_help = false` (`crates/aethelgard-cli/src/main.rs:27`), which gives "'aethelgard' requires a subcommand". The reduction reviewer later proposed deleting that setting as "clap's default". The behaviour observed before the setting was added disproves that claim, so the setting stays (§6, F18).
3. **A wrong claim about deep nesting.** The first CLI README said nesting deeper than 128 is `InvalidJson`, exit 3, from serde_json's recursion limit. Probing showed the limit is never reached. The typed schema refuses the first `[` or `{` where a string or number belongs, as `SchemaViolation`, exit 2. A 1,000,000-deep array is refused in 0.00 s with 5.7 MB RSS. The README was corrected before commit.
4. **The first workspace-shape test inspected only `src/lib.rs`, and dependency keys only by name.** The adversarial pass hid a `pub struct VoltageDomain` in `src/bin/x.rs`, a sibling `src/main.rs`, `tests/`, `examples/`, and behind a `[lib] path` override. It also slipped `serde = { package = "serde_json" }` past the allow-list under an allowed key. All six passed the test. **Replaced:** a library crate must be exactly `Cargo.toml` plus `src/lib.rs`, with only `[package]`, `[dependencies]` and `[lints]`, and dependency checks read the `package =` rename (`crates/aethelgard-cli/tests/workspace_shape.rs:42`, `:64`). All 12 mutations were re-run against the fixed test and every one fails it (§3).
5. **Closed stdout cannot be made an exit-70 case.** The adversarial report attributed `validate >&-` exiting 0 (or 2) with no report to std "swallowing EBADF". That is not the cause. `/proc/<pid>/fd/1` points at `/dev/null` while the process runs, because the Rust runtime reopens closed standard descriptors on `/dev/null` before `main`. That cannot be told apart from a caller's own `>/dev/null` without `unsafe`, so it is documented rather than fixed (F8). The exit code still reports the verdict.
6. **Toolchain install pulled two copies of the same compiler.** `rustup-init --default-toolchain stable` installed `stable`. The first `cargo` in the repo then saw `rust-toolchain.toml`, and rustup 1.29.1 auto-installed `1.99.1` as a separate toolchain. Two 617 MB directories hold the same version (F15).
7. **Small ones:** `&MEMBERS[1..6]` is rejected in a `const` on 1.99.1 ("`Index` is not yet stable as a const trait"), so the reduction became a one-line `fn libraries()`. The first write of the IR test file failed because its directory did not exist yet.

## 2. Gates line

Run on Furnace in `~/aethelgard` at the final tree, raw output kept by the builder:

- `cargo fmt --all -- --check` → exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0, no warnings.
- `cargo test --workspace` → exit 0, **36 tests**: `aethelgard-ir` 18, CLI contract 10, workspace shape 8. Named as §3.3 requires:
  - serde round trip: `serde_round_trip_of_valid_graph`.
  - One negative per `IrValidationError` variant: `refuses_empty_project_name`, `refuses_empty_domain_id`, `refuses_duplicate_domain_id_including_whitespace_variants`, `refuses_zero_nominal_voltage`, `refuses_tolerance_out_of_range`, `refuses_empty_net_name`, `refuses_duplicate_net_name_including_whitespace_variants`, `refuses_zero_capacitance`, `refuses_zero_rise_time`, `refuses_zero_impedance`.
  - Unknown fields: `unknown_top_level_field_is_a_deserialization_error`, `unknown_nested_field_is_a_deserialization_error`.
  - Workspace shape: 8 tests in `crates/aethelgard-cli/tests/workspace_shape.rs`, which is where the test lives (not an `xtask`).
- `cargo run -q -p aethelgard-cli -- validate < examples/sentinel-01-minimal.json` → `{"ok":true,"project_name":"sentinel-01","voltage_domains":1,"net_constraints":2}`, exit 0.
- `… validate < examples/invalid-zero-domain.json` → `{"ok":false,"error":{"kind":"ZeroNominalVoltage","id":"VDD_3V3",…}}`, exit 2.
- `printf 'not json' | … validate` → one JSON line (`InvalidJson`), exit 3, 0 bytes on stderr.
- `… validate < /dev/null` → `InvalidJson`, exit 3, no panic.
- `… --version` → `aethelgard 0.1.0`, exit 0.
- Protected surfaces: `git diff --stat origin/main -- docs/northstar.md docs/architecture.md docs/roadmap.md README.md LICENSE-MIT LICENSE-APACHE docs/inputs` prints nothing.
- CI: CI_RUN_PENDING
- `unsafe`: an injected `unsafe {}`, `#[allow(unsafe_code)] unsafe fn`, or `#![allow(unsafe_code)]` is refused by `cargo check --workspace --all-targets` in all seven crates. That covers the crate root, an integration test, and the crate root with the `#![forbid]` line removed (the workspace lint alone): 42 variants by the adversarial subagent. The builder re-ran 3 of them itself (CLI `main.rs`, `chaos`, an `ir` test), and all three gave E0453.
- Reduction pass (diff), by a fresh read-only subagent, applied by the builder: −2 lines net · 1 duplicate removed (the library list restated `MEMBERS`) · 0 dead paths removed · 0 protected kept · 2 findings below (F17, F18).

## 3. Adversarial pass (§5), by a fresh subagent in a separate worktree

A Sonnet subagent ran in `~/worktrees/aethelgard/adv-0100` at `c8bf457` and reported commands and exit codes. Its two medium findings were spot-checked by the builder before acting. One held (positional form). One held with the wrong cause (closed stdout, §1.5).

- **§5.1 hostile JSON** (98 inputs, each via stdin and `--file`, every output exactly one JSON line):
  - Integer inputs: u32 overflow, u64 overflow, negatives, `-0`, `1e3`, `3300.0` → exit 2.
  - `null` for each of the nine required fields, each field missing, unknown fields at all three levels, duplicate keys at all three levels → exit 2.
  - 50 MB valid-looking and garbage documents → `InputTooLarge`, exit 3, 0.00 s, 20 MB RSS. Exactly 16 MiB valid → exit 0.
  - 10,000- and 1,000,000-deep nesting at the top level and inside fields → exit 2, no stack overflow.
  - BOM, invalid UTF-8, raw NUL, lone surrogates → exit 3. CRLF → exit 0, correctly.
  - A 1 MB whitespace id, and ids of U+00A0, U+2003 or U+3000 → `EmptyDomainId` (the trim is Unicode-aware).
  - Confusables and U+200B → accepted (F4).
  - `/dev/zero` on stdin and via `--file` → `InputTooLarge`, no hang. A directory, `chmod 000`, a symlink loop, `/proc/self/mem` → `UnreadableInput`, exit 3. A FIFO with no writer hangs (F9).
  - No panic and no exit 0 on any malformed input after the positional-form fix.
- **§5.2 `INV-01` by hand:** `pub struct VoltageDomain` in `eda` and `serde_json` in `twin` are both caught. So are `mod`, `macro_rules!`, `#[path]`, `include!`, `type` aliases, and target-, dev- and build-dependencies. Six bypasses were found and closed (§1.4).
- **§5.3 `unsafe`:** see the gates line.
- **§5.4 SIGPIPE:** `validate | head -c 1`, 200 loops each on the valid and invalid example → exit 0 / 2, 0 bytes of stderr, no panic. The output fits the pipe buffer, so this does not reach EPIPE. A 2 MB error report into `head -c 1` does reach it: exit 70, one stderr line `cannot write report to stdout: Broken pipe`, no panic. `>/dev/full` → 70. The code is documented in `crates/aethelgard-cli/README.md`.
- **§5.5 lock:** `rm Cargo.lock && cargo test --workspace` re-locks 41 packages byte-identically, and the tests pass. The lock is committed.
- **§5.6 luck:** the suite passes with `LC_ALL=C`, with an empty `HOME`, with `env -i` and a minimal `PATH`, and via `--manifest-path` from `/` and from another directory. Tests locate files from `CARGO_MANIFEST_DIR`, never from the working directory.

## 4. Findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F1 | `impedance_ohms` must be present as a key; `null` means unconstrained. serde would otherwise turn a missing key into `None`, and the work order says a missing field is a refusal | `crates/aethelgard-ir/src/lib.rs:47` | note | Keep. Relax it only by ruling | no |
| F2 | Deserialization errors split by `serde_json::Error::classify()`: `Data` (valid JSON, wrong shape) → `SchemaViolation`, exit 2; `Syntax`/`Eof`/`Io` → `InvalidJson`, exit 3 | `crates/aethelgard-cli/src/main.rs:116` | note | Keep; `AET-WO-0300` inherits it, and the README documents it | no |
| F3 | Input is capped at **16 MiB**, read through `take(cap + 1)`, so a hostile document is refused unread. The number is the builder's choice: a 120,000-net document is 13.7 MB and validates in 0.17 s | `crates/aethelgard-cli/src/main.rs:24` | risk | Rule on the cap. Raise it when a real specification approaches it | **yes** |
| F4 | Distinct strings are distinct ids. `VDD_3V3` and the same with Cyrillic `В` are two domains. U+200B (zero-width space, not White_Space) is a valid non-empty id. Nothing in §1.3 refuses either, and no charset rule was invented | `crates/aethelgard-ir/src/lib.rs:149` | scope-question | Decide whether ids are restricted, for example to printable ASCII, which KiCad net names like `+3V3` and `/SDA` fit, and assign it to `AET-WO-1100` | **yes** |
| F5 | Empty `voltage_domains` or `net_constraints` arrays validate. They are not in §1.3's refusal list | `crates/aethelgard-ir/src/lib.rs:149` | scope-question | Decide whether a specification with no domains is malformed (`INV-06` reading) or merely trivial | **yes** |
| F6 | Positional-array form accepted with `"ok":true` (§1.1). **Fixed** in this branch | `crates/aethelgard-ir/src/lib.rs:55` | bug | Done. Any future derived spec type goes through `object_only!` | no |
| F7 | Workspace-shape test bypassable six ways (§1.4). **Fixed** in this branch | `crates/aethelgard-cli/tests/workspace_shape.rs:42` | bug | Done | no |
| F8 | `validate >&-` behaves as `>/dev/null`: no report, exit reflects the verdict, never 70. The Rust runtime reopens closed fd 1 on `/dev/null` | `crates/aethelgard-cli/README.md` | note | Documented. Exit codes remain truthful | no |
| F9 | `--file <FIFO with no writer>` blocks forever in `open`. The same happens with a stdin pipe that never closes, which is inherent to reading stdin | `crates/aethelgard-cli/src/main.rs:149` | risk | Either refuse non-regular files after `open` (which would also refuse `--file /dev/stdin`), or leave hangs to the caller's timeout under `architecture.md` §6's subprocess posture in `AET-WO-0300` | **yes** |
| F10 | A refusal names the id twice (`id` and inside `message`), so a 1 MB id makes a ~2 MB line. Bounded by the 16 MiB cap | `crates/aethelgard-ir/src/lib.rs` error strings | note | Truncate ids in `message` if the size ever matters; `id` is the contract | no |
| F11 | Integers beyond u64 are described as "floating point" in the `SchemaViolation` message (serde_json parses them as f64). `kind` and exit are right | serde_json | note | None; `message` is not contract | no |
| F12 | `--file -` is a file named `-`, not stdin | `crates/aethelgard-cli/README.md` | note | Documented | no |
| F13 | CI adds `--locked` to the clippy and test steps, beyond §1.6's literal commands, so CI fails if the committed lock drifts | `.github/workflows/ci.yml:24` | note | Keep | no |
| F14 | Rust was installed without touching shell rc files (`rustup-init --no-modify-path`), because `~/.bashrc.d` is owned by furnace777's `bootstrap-builders.sh`. `rustup-init` came straight from `static.rust-lang.org` with its `.sha256` verified (`dda72343…`). A fresh shell therefore has no `cargo` on `PATH` until `~/.cargo/bin` is added | Furnace `~/.cargo` | scope-question | Add `~/.cargo/bin` to `PATH` in furnace777's `~/.bashrc.d/furnace.sh` by PR, so it is not drift | **yes** |
| F15 | Two 617 MB toolchains (`stable` and `1.99.1`) hold the same compiler (§1.6) | Furnace `~/.rustup/toolchains` | note | `rustup toolchain uninstall stable` when convenient; next time `rustup-init --default-toolchain none` | no |
| F16 | Kickoff base `5fa3b12` in the work order §8 was stale; actual base `29e1dfe` | `docs/work-orders/AET-WO-0100.md` §8 | note | Architect's practice: name the base at kickoff, not at authoring | no |
| F17 | The forbid-attribute check on `ir` and `cli` reads `src/lib.rs`/`src/main.rs` and would miss a `[lib]`/`[[bin]] path` override on those two crates. The workspace lint still enforces the forbid; only the visible-statement check is affected | `crates/aethelgard-cli/tests/workspace_shape.rs:82` | note | Leave until a path override is proposed. Not worth more test code now | no |
| F18 | Disagreement between subagents, surfaced rather than averaged: the reduction reviewer called `arg_required_else_help = false` a no-op default. The builder's observed run before the setting existed disproves it | `crates/aethelgard-cli/src/main.rs:27` | note | Kept | no |
| F19 | Push outcome: PUSH_PENDING | — | note | — | no |

## 5. Deliberately not built (§4 held)

Checked against the diff:

- No `z3` and no constraint solving.
- No `svd-parser` and no MCU type.
- No `kicad-cli`, `renode`, `ngspice` or `cargo kani`, and no product code that spawns a process. The CLI contract tests spawn the built `aethelgard` binary itself, which is test code exercising the contract.
- No `rmcp` and no listener.
- No Cap'n Proto, trace archive or R2.
- No subcommand but `validate`, plus clap's built-in `help`.
- No Dockerfile and no Nix.
- No Python and no shell scripts. The only shell is in the CI workflow steps.
- No `unsafe`, no `build.rs`, and no proc-macro crate of our own.
- No edits to the steering documents, the README, `LICENSE-*` or `docs/inputs/**`. `CLAUDE.md` is unchanged and still true.

`Cargo.lock` is committed. Dependencies are `serde`, `serde_json`, `thiserror` and `clap`, plus `syn` and `toml` as CLI dev-dependencies for the shape test.

## 6. Measured facts for the architect to promote (`architecture.md` §2)

| Fact | Value | Command |
|---|---|---|
| Pinned Rust | `1.99.1` (`58cf8f9c3 2026-10-13`) | `rustc --version` |
| Edition 2024 | accepted | `[workspace.package] edition = "2024"` builds |
| `cargo` | `1.99.1 (5f94df478 2026-08-27)` | `cargo --version` |
| `rustup` | `1.29.1 (d95a37b6a 2026-08-13)` | `rustup --version` |
| `cargo test --workspace`, cold target dir (registry already downloaded) | 2.61 s | `cargo clean && /usr/bin/time cargo test --workspace` |
| `cargo test --workspace`, warm | 0.11 s | `/usr/bin/time cargo test --workspace -q` |
| Furnace host | x86_64, 20 cores, 60 GiB | `uname -m`, `nproc`, `free -g` |
| Egress | `static.rust-lang.org` (rustup-init and toolchains) and `crates.io` (41 packages) reached with no proxy or special configuration. Nothing else was fetched | the install and the first `cargo build` |

## 7. Promote-or-needs-none

This is the first work order; there is no prior FEEDBACK to promote. Needs none.

The decisions in F3, F4, F5, F9 and F14 need rulings before `AET-WO-0200` starts.
