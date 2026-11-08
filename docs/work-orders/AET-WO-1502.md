# AET-WO-1502 — A `ToolError` says what the tool said

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1502-tool-error-line`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass. A fresh session on Furnace.
**Phase:** P1, reserved at the `AET-WO-1400` spike ruling (F5, PR #42).

**Grounds:**
- **`AET-WO-1400` FEEDBACK F5.** `prove` on a crate whose dependency cannot resolve returns `ToolError` with "its output matches none … of: failed proof, compile error, missing harness" and no line of what cargo said. The cause, `no matching package named …`, came only from a re-run outside the sandbox.
- `docs/northstar.md` §4 `INV-06`; `docs/architecture.md` `RUL-13` (caps on what the CLI copies).

> **What this is:** when `prove` or `simulate` cannot recognise its tool's outcome, the `ToolError` message ends with the tool's first `error:` line.
>
> It is NOT:
> - a change to any verdict, exit code or recognised outcome;
> - a change to `check drc` or `check erc`, whose `kicad.rs` `AET-WO-1300` is rebuilding.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **One helper in the CLI.** *Amended at the §7 stop on PR #47:* cargo's cause is often not on its first `error:` line. In Cargo style the helper therefore joins **every** line that starts with `error:`, in order, with ` | `, capped at 400 bytes. In Renode style it takes the first line that contains `[ERROR]`, capped at 240 bytes. The original text follows: it takes the tool's captured output and returns the first line that starts with `error:` (for Kani and cargo) or contains `[ERROR]` (for Renode), with these rules:
   - trimmed;
   - every byte outside printable ASCII `0x20`–`0x7E` replaced by `?`;
   - cut to 240 bytes, with `…` appended if it was cut;
   - nothing when there is no such line.
2. **Use it in `kani.rs`** (the `disagree` arm and the unexpected-exit arm) **and in `renode.rs`** (the nonzero-exit and unrecognised arms). The message gains `; the tool said: <line>`.
3. **Tests:**
   - the helper's unit tests: no line; the first of several; a line exactly 240 bytes long and one 241 bytes long; non-ASCII and control bytes; CR LF line endings;
   - one tier-1 test per call site, using a stub tool that prints a known `error:` line;
   - the tier-2 `prove` test of a crate with an unresolvable dependency, where the message names the package.

## 2. Invariants and fences

- `INV-06`. No `unsafe`. No new dependency.
- **Fence:** `crates/aethelgard-cli/src/kani.rs`, `renode.rs`, the new helper's module, the CLI's tests and README, and this work order's FEEDBACK. **`kicad.rs` is out of the fence.**
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

§1.3 passes in `gates` and `tools`. CI is green on the first push.

## 5. Adversarial pass — targeted

1. Make the quoted line break the JSON envelope or the terminal (escapes, newlines, a huge line).
2. Make the helper quote a line from the wrong stream, or one the tool did not print.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- A change to a verdict or an exit code.
- A change outside the fence.
- A test that can only go green by being weakened.

## 8. Kickoff prompt

```
/goal AET-WO-1502: §1 is built and §3 passes in gates and tools, or a §7 stop is recorded in FEEDBACK; the targeted adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1502-tool-error-line.
```
