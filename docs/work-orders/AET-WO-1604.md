# AET-WO-1604 — The CLI's backlog: `doctor`, the MCP tool-list snapshot, fabricator-format BOM and placement, the boot test's bound, and `extract`'s test-pad condition

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1604-cli-backlog`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog (roadmap §7.1), before Sentinel-01's order. Written 2026-11-17, after `AET-WO-2302` merged (`ecc6be9`), which had held the CLI. **Cadence: build, with one measured stop (§1.3).**

**Grounds (roadmap §7.1 rows, on `main`):**
- **The Grok review of 2026-11-17** (`docs/reviews/2026-11-17-grok.response.md` §2):
  - `aethelgard doctor`;
  - a CI snapshot of the MCP tool list (`cli/src/mcp.rs` computes it from the command tree, and nothing compares it with a committed copy).
- **`AET-WO-3401` review R3 (PR #190, comment 6940837600):** `verify --fab-out` writes KiCad's placement CSV and the emitter's BOM. Neither is in a fabricator's upload format, and two of Sentinel-01's boards are to be fab-assembled.
- **The `boot_tier1` row:** `a_boot_past_the_memory_cap_fails_and_names_it` bounds its wall at 30 s, and a loaded runner took 31.8 s. The kind already proves the reaper fired. The bound should only show the kill came before the stub's own 60 s sleep.
- **`1312` Q2 (PR #191):** `extract`'s verdict should state, when a line carries a test pad, that a probe clipped to it is an off-board load outside the bound.

> **What this is:** five small, independent CLI capabilities, each with its own tests.
>
> It is NOT:
> - a change to any gate's verdict on an existing board;
> - a change to any emitted byte, except the new fab-format files and `extract`'s new condition line where a test pad exists.

## 1. Build

1. **`aethelgard doctor`**, a subcommand that prints one JSON line and exits 0 only when everything the image's commands need is present. It reports:
   - the binary's version;
   - the Landlock ABI and whether seccomp can be installed (the same probes `run.rs` uses, never a second implementation);
   - which external tools resolve, and their versions (`kicad-cli`, `renode`, `ngspice`, the Kani toolchain);
   - whether the default scratch base passes `run.rs`'s ownership and mode checks.

   Each missing piece is named, with the subcommands it blocks. `doctor` itself never runs a tool beyond its version probe, and that probe runs inside the same sandbox.
2. **The MCP tool-list snapshot.** Commit the tool list `mcp.rs` computes (`crates/aethelgard-cli/tests/mcp-tools.json`, or a name you justify). A tier-1 test fails, naming the tool and field, when the computed list differs. Regenerating the snapshot is one documented command. The CLI README says the snapshot is the MCP contract.
3. **Fabricator-format BOM and placement. Measure first, then STOP.**
   - Read JLCPCB's current assembly upload formats at source (their help pages), and record the URL and date.
   - The BOM columns are Comment, Designator, Footprint and the fab part number. The placement columns are Designator, Mid X, Mid Y, Layer and Rotation.
   - **STOP** with the column map from what `verify --fab-out` has today. Name every column it cannot fill without a guess (the fab part numbers; each footprint's rotation offset). `INV-06`: an unknown column is left empty and named, never invented.
   - After the ruling, `verify --fab-out` writes the two files beside the package, listed in `MANIFEST.json`.
4. **`boot_tier1`'s bound.** Bound the wall time below the stub's 60 s sleep with a stated margin, and give the reason in a comment, as the backlog row asks. Read the time from the reaper's own record where `2302` made that available.
5. **`extract`'s condition.** When a bounded line carries a test pad (`1312`), the verdict states that a probe on it is an off-board load outside the bound, naming the pad. No bound changes.

## 2. Acceptance

- Each of the five has tier-1 tests. `doctor` is tested with a tool missing and with the scratch unsafe. The snapshot test fails on an added argument.
- **The fab-format files** exist only after the §1.3 ruling. Their unfillable columns are empty and named in `FAB-NOTES.md`.
- **No existing emitted byte changes,** except as allowed above. `verify`'s package changes only by the two new files and their `MANIFEST.json` lines.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-10`, `RUL-21`.
- **Fence:**
  - `aethelgard-cli` (src and tests);
  - `crates/aethelgard-eda/src/fab.rs` (§1.3 only);
  - the CLI README;
  - FEEDBACK.
- **Running beside you:**
  - `2302b` (evolve, chaos, `cli/tests/attribute*.rs`);
  - `4400b2` (pincheck, perhaps `cli/src/pinout.rs`);
  - `1313` (ir's `board`, `pcb.rs`);
  - `3401` (Sentinel-01's package, which `verify --fab-out` regenerates: tell it through the bus when §1.3's files land);
  - `2303` (a spike).
- One PR, left unmerged. Write `~/.cache/aet-1604/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **`doctor` reporting a sandbox capability it cannot actually install.**
- **A snapshot test that passes after a tool's required argument is added.**
- **A fab-format column filled with a plausible guess.**
- **A test-pad condition line on a line with no pad,** or missing on one with a pad.

## 7. Flag-and-stop conditions

- **§1.3 (the fab formats' column map).**
- **`doctor` needs a capability probe `run.rs` does not have.** Stop rather than write a second implementation.
