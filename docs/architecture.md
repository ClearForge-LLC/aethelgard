# Aethelgard — architecture

**Status:** draft for ratification, 2026-10-14. **Supersedes** the unmerged architecture draft inside `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md`. Read `northstar.md` first.
**Author:** Claude Code, synthesizing the Argon (Gemini) planning session with Scotty · **Gate:** Scotty.

---

## 1. How to use this document

This is the source of truth for what is being built and why each choice was made. Implementation detail belongs in work orders; this document constrains them. The roadmap tracks sequence; this tracks shape.

**The divergence rule:** any deviation from this document is recorded as a dated amendment in §10 with its rationale. Undocumented drift is a defect, not a shortcut.

Invariants are **not** defined here. They live in `northstar.md` §4 and are cited below as `INV-01` through `INV-07`.

## 2. Ground truth — measured, not assumed

Measured on 2026-10-14 in the Claude Code cloud container that produced this document, before anything below was designed. Commands are cited so a reader can re-run them. This container is not the build host; the deterministic build environment is `AET-WO-0200`'s job, and its measurements supersede these.

**Repository.** One commit on `main`, a twelve-byte `README.md`, no code (`git log --oneline`, `ls -la`).

**Toolchains present.** `cargo 1.97.0`, `rustc 1.97.0`, `python3 3.13.16`, `node v22.22.0`, `docker 29.8.2`, `nix 2.34.6` (`<tool> --version`).

**Toolchains absent.** `kicad-cli`, `renode`, `ngspice`, `kani` are not on `PATH` (`which`). Nothing in this document has been run against them.

**Crates exist on crates.io** for every library Argon's plan names (`cargo info <crate>`): `z3`, `svd-parser`, `svd2rust`, `capnp`, `embassy-stm32`, `embassy-rp` (tagged `rp2040`, `rp235x`), `kani-verifier`, `rmcp`, `esp-hal`. Existence only; none was compiled.

**Renode.** Its supported-boards page lists `stm32h7`, `stm32h743`, `stm32h747`, `stm32h753`, and `nucleo_h753zi` platforms. No RP2040, RP2350, ESP32, or ESP32-S3 platform appears in the first 100,000 characters of that page; the remaining 447,752 characters were not read, so their absence is **unverified**. Antmicro published an STM32H753 Renode reference platform with a physical devkit and a digital twin on 2026-07-16, running Zephyr. Renode's time framework models a CPU as an integer number of instructions per virtual microsecond (default 100 MIPS) with a virtual-time resolution of one nanosecond. The page never uses the words "cycle accurate."

**KiCad 9.0 CLI.** `kicad-cli sch erc` and `kicad-cli pcb drc` exist and accept `--format json` and `--exit-code-violations`, which makes the exit code 5 when violations are found. The documentation does not describe the exit code when the flag is omitted; a third-party source reports it is 0 even with violations. **Unverified here**; `AET-WO-0200` tests it.

**Kani.** Kani stubs functions it cannot analyze (inline assembly, FFI) with hand-written mocks that are not checked against the real implementation, and it does not detect data races or all provenance-related undefined behaviour. How it treats volatile memory-mapped register access was not confirmed from a primary source. **Unverified**; `AET-WO-1400` spikes it.

### 2.1 Corrections to the planning conversation

The wrong belief is the instructive half, so both columns stay.

| Assumed in the Argon session | Measured |
|---|---|
| Renode gives a "cycle-accurate" digital twin, and traces are indexed by "simulation picosecond" | Renode is instruction-level: an integer instruction count per virtual microsecond, with one-nanosecond virtual-time resolution. There is no public RTL for these microcontrollers, so nothing cycle-accurate is available. `INV-04` is now worded "at the simulator's own time resolution" and every "picosecond" in Argon's drafts is read as "trace timestamp" |
| `kicad-cli sch erc` and `pcb drc` "pass with 0 errors" is a usable exit gate | Without `--exit-code-violations` the documented nonzero exit does not apply, so a gate written as "exit 0" can pass with violations present. Every KiCad gate in the roadmap passes the flag and expects exit 5 on an injected violation |
| STM32H7, RP2350, and ESP32-S3 are equal candidates for the reference probe | Only STM32H7 has documented Renode platforms, plus a vendor-published reference board with a digital twin. The other two are unverified in Renode and the Xtensa core in ESP32-S3 has only a sample controller listed |
| Argon could push a branch to GitHub | It could not; both of its bootstrap scripts were never executed. The second script also has a bug: an unquoted heredoc containing backticks around `${CRATE}` would execute each crate name as a shell command |
| The steering documents were "Ratified" | Nobody ratified them. These drafts say so in their status lines |
| The seven-crate workspace "compiles" | It was never compiled anywhere. The workspace is built and measured by `AET-WO-0100` |
| Verilator is an alternative to Renode for "RISC-V/ARM cores", and Miri an alternative to Kani for the driver patch (founding doc, capture 00) | Verilator simulates RTL, and no RTL exists for STM32, RP2350, or ESP32 silicon; it applies only to open cores such as VexRiscv, which would be a different reference target. Miri interprets ordinary Rust tests and cannot execute `no_std` firmware against memory-mapped registers. Neither is a drop-in alternative; both are recorded here so they are not re-proposed as one |
| Planning documents belong in `docs/genesis/GENESIS.md` and `STEERING.md` | The ClearForge `repo-genesis` contract places them at `README.md`, `docs/northstar.md`, `docs/architecture.md`, `docs/roadmap.md`. Argon corrected this itself in capture 05; this document keeps that correction |

The Renode correction is the load-bearing one. A plan that promises cycle accuracy and picosecond replay will be believed, and the first time a timing bug slips through simulation the project's credibility goes with it. Promising instruction-level fidelity with an honest resolution is a claim the tool can keep.

**Blocked, not faked.** Three numbers Argon's roadmap used as gates have never been measured: the solver's time on a realistic pin table ("< 1000 ms"), the trace volume of a fault campaign, and Kani's runtime on a generated register block. `AET-WO-1200`, `AET-WO-0400`, and `AET-WO-1400` produce them. No estimates are recorded in their place.

## 3. Shape — one representation, two projections, one loop

```
                 specification (declarative)
                           │
                           ▼
                 ┌──────────────────┐
                 │  aethelgard-ir   │  solved graph: pins, nets, domains, timing bounds
                 └───────┬──────────┘
            ┌────────────┴─────────────┐
            ▼                          ▼
 ┌──────────────────┐        ┌───────────────────┐
 │  aethelgard-eda  │        │ aethelgard-svd-hal│
 │ schematic, layout│        │ no_std typestate  │
 │ BOM, fab package │        │ board crate       │
 └────────┬─────────┘        └────────┬──────────┘
          │ parasitics (R, L, C)       │ firmware image
          └─────────────┬─────────────┘
                        ▼
              ┌──────────────────┐
              │  aethelgard-twin │  digital simulator + analog solver, lock-stepped
              └────────┬─────────┘
                       ▼
              ┌──────────────────┐
              │ aethelgard-chaos │  fault scenarios in, trace archive + verdict out
              └────────┬─────────┘
                       ▼
              ┌──────────────────┐
              │ aethelgard-evolve│  attribute, then propose a specification change
              └────────┬─────────┘
                       │ proposal (never a merge)
                       └──────────────► back to the specification
```

Seven parts, in a Rust workspace plus one tool server:

| Part | Owns | Never touches |
|---|---|---|
| `crates/aethelgard-ir` | the specification schema, the solved graph, validation, serialization | external processes, network |
| `crates/aethelgard-eda` | schematic, layout, bill of materials, fabrication package emission | firmware |
| `crates/aethelgard-svd-hal` | vendor register description ingestion, generated board crate | board files |
| `crates/aethelgard-twin` | simulator platform descriptions, analog model, the lock-step bridge | the specification (read-only) |
| `crates/aethelgard-chaos` | fault scenarios, campaign runner, trace archive, verdicts | suppressing a panic or a timeout |
| `crates/aethelgard-evolve` | attribution from traces, mutation proposals | merging anything, bypassing a gate |
| `mcp/aethelgard-mcp-server` | tool calls over the crates and the external CLIs | raw credentials in source or logs |

**Seams that are security or blast-radius boundaries**, named because a module boundary and a trust boundary are maintained differently:

- **Tool server → external CLIs** (`kicad-cli`, `renode`, `ngspice`, `cargo kani`) is a **trust boundary**. Arguments are passed as arrays, never shell strings; each process runs in a resource-bounded sandbox; a missing binary is an error (`INV-06`). This is the seam an injected specification would try to pivot through.
- **Evolve → specification** is a **blast-radius boundary**. A proposal is a diff on a branch with its attribution record attached. Nothing in the loop can merge (`INV-05`, northstar `NG-04`).
- **Chaos → trace archive sink** is a **credential boundary**. The local archive needs no credentials; the optional remote archive is referenced by environment-variable name only and its absence halts the upload step, not the run (`INV-06`).
- **IR → every other crate** is a module boundary only. It is the thing `INV-01` protects, but it is enforced by the type system, not by a trust mechanism, and it should be said plainly that it is not a security boundary.

## 4. Ratified rulings

Each carries its reason. Departures from Argon's plan are labelled as departures so they are not later mistaken for drift.

| # | Question | Ruling | Reason |
|---|---|---|---|
| `RUL-01` | Implementation language and layout | One Rust Cargo workspace; the tool server lives in the same workspace | One type system carries the solved graph from solver to simulator to tool server with no marshalling layer in which `INV-01` could drift |
| `RUL-02` | Pin-mux, voltage-domain, and timing resolution | A constraint solver, with SMT via the `z3` crate as the prior. **Spike first** (`AET-WO-1200`): the prior is held loosely, because a pin-mux table may be small enough for a pure-Rust constraint search and the Z3 system dependency is a real cost in the container | Heuristic or model-guessed pin selection is exactly the hallucination `INV-01` and `INV-03` exist to catch |
| `RUL-03` | Board artifact format | Emit KiCad S-expressions (`.kicad_sch`, `.kicad_pcb`) directly; validate with `kicad-cli` passing `--exit-code-violations` and `--format json` | Text format, diffable, headless checks exist. The flag is mandatory because of the exit-code finding in §2.1 |
| `RUL-04` | Firmware hardware interface | A generated `#![no_std]` board crate whose pin, clock, and peripheral states are typestates, built atop the `embassy` ecosystem for the chosen target; any `unsafe` register block is bounded-model-checked. **Spike first** (`AET-WO-1400`): how Kani sees volatile register access decides whether the proof is real or a stub | `INV-02` is enforced by `rustc` at zero runtime cost only if illegal states are unrepresentable; Kani is the only Rust tool that proves absence of a class of UB rather than testing for it |
| `RUL-05` | Digital twin | Renode for instruction-level execution, `ngspice` for the analog nets that matter (bus rise time, rail droop), lock-stepped through a bridge whose transport is spiked (`OQ-02`). **Named as instruction-level, not cycle-accurate** | Renode is the only open simulator with the target family's peripherals modelled; the analog solver supplies the physics Renode idealises away. Claiming more than instruction fidelity would violate the honesty `INV-06` demands of measurements |
| `RUL-06` | Trace format and storage | Cap'n Proto trace schema, **local-first**: the archive of record is on disk, and Cloudflare R2 is an optional mirror. **Departure from Argon's plan**, which streamed to R2 as the primary store | `INV-04` must hold with no network and no credentials; a replay that needs a cloud account is a replay that fails on the day it is needed. R2 remains attractive for cost once volume is measured (`AET-WO-0400`) |
| `RUL-07` | What the evolution loop may do | Propose a specification diff with an attribution record; never apply it to `main`, never alter a gate | `INV-05`, `INV-07`, and northstar `NG-04` together: the loop proposes, the gates judge, a human merges |
| `RUL-08` | First reference target | **Claude's prior, awaiting `OQ-01`:** STM32H7 (`STM32H743` or `H753`). RP2350 is the planned second target, precisely because it is a different vendor | §2 measured Renode platform coverage for H7 and a vendor reference board with a digital twin; neither exists for the alternatives. A second vendor later is what keeps the IR from becoming one vendor's register map (northstar §6) |
| `RUL-09` | Where gates are enforced | In the chaos runner and the CI job, never in the proposing agent | `INV-07`; a check the proposer performs on itself is not a boundary |

## 5. Runtime and deployment

Everything runs headless in one pinned container image built by `AET-WO-0200`, used identically by a local builder and by CI. Docker and Nix are both present on the measuring host; which one builds the image is decided by that work order with reasons recorded here by amendment.

Operational facts worth writing down now, so the first builder does not rediscover them:

- `kicad-cli` must be given `--exit-code-violations`, or a gate passes with violations present (§2).
- Renode is run headless with its console disabled and a script file; a run that waits on an interactive monitor looks like a hang.
- `ngspice` runs in batch mode; interactive mode blocks on a prompt that never comes.
- `cargo kani` installs its own backend on first run; the image must pre-install it or the first CI job will time out looking like a proof failure (`INV-06` would have it halt, and it would halt for the wrong reason).

## 6. Security posture

The tool server is the only part that executes external programs, so it carries the posture: argument arrays with no shell, a sandbox per invocation, tool-scoped capabilities, and secrets by environment-variable name only (`AETHELGARD_R2_BUCKET`, `AETHELGARD_R2_ACCESS_KEY_ID`, `AETHELGARD_R2_SECRET_ACCESS_KEY`), absent means the remote mirror is skipped with a logged error. Generated firmware is never executed on the host; only inside the simulator. The ClearForge fleet MCP standard (`clearforge-mcp-standard` skill) is the reference posture if this server is ever exposed beyond localhost; see `OQ-05`.

## 7. Traceability

None. This project has no governing standard. DO-178C and ISO 26262 appear in the northstar's beneficiary list as standards *users* may answer to; Aethelgard does not claim conformance to either. Recorded explicitly so a reader knows the question was asked.

## 8. Open questions

Each names a decider and what it blocks. None is a placeholder.

- **`OQ-01` Which microcontroller anchors the reference probe?** Decider: Scotty, with `RUL-08` as Claude's prior and Argon's three candidates in capture 05. Blocks `AET-WO-1100` (register description ingestion) and everything after it. Does not block P0.
- **`OQ-02` How do the digital simulator and the analog solver exchange state in lock-step?** Candidates: a shared-memory ring buffer or a Unix domain socket, as Argon listed; a third is driving `ngspice` as a library through its shared-library interface rather than as a process. Decider: the spike in `AET-WO-2100`, ratified by Scotty. Blocks P2's work orders beyond the spike.
- **`OQ-03` What is the trace volume of one fault campaign, and does full-fidelity recording survive it?** Decider: measured by `AET-WO-0400`, ratified by Scotty if sampling is ever proposed (which touches `INV-04`). Blocks the P2 exit gate.
- **`OQ-04` Can Kani prove anything useful about volatile register access, or only about stubs?** Decider: the spike in `AET-WO-1400`. Blocks the wording of the P1 exit gate's proof clause.
- **`OQ-05` Tool server language: Rust via `rmcp` in the workspace, or TypeScript per the ClearForge fleet standard?** Decider: Scotty. Rust keeps the shared types (`RUL-01`); TypeScript matches every other ClearForge server. Blocks `AET-WO-0300`.
- **`OQ-06` Who proposes mutations in the evolution loop: deterministic rules, a language model, or both in sequence?** Decider: Scotty and Argon, informed by P3's spike. Blocks `AET-WO-3200`. Northstar `NG-04` fixes that the proposer never judges; it does not fix who proposes.

## 9. What this document does not cover

Work-order-level implementation detail, and sequence. Sequence lives in `roadmap.md`; this document constrains the order without setting it. Argon's crate-level type sketches (`TopologyGraph`, `VoltageDomain`, `NetConstraint`) are preserved in `docs/inputs/argon-session/` and are input to `AET-WO-0100`, not rulings.

## 10. Amendments

| Date | Section | Change | Rationale |
|---|---|---|---|
| 2026-10-14 | — | Initial draft | Seed. `RUL-06` local-first and `RUL-07` through `RUL-09` are departures from or additions to Argon's draft, each with its reason in §4 |

New sections continue the sequence and never reuse a number.

## 11. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, the founding Google Doc and the five Argon session captures under `docs/inputs/argon-session/`, and the measurements in §2, taken in the Claude Code cloud container on the same date. Sources read for §2: Renode's supported-boards and time-framework documentation, Antmicro's 2026-07-16 STM32H7 reference platform post, the KiCad 9.0 CLI reference, and the Kani paper and book excerpts returned by search.
