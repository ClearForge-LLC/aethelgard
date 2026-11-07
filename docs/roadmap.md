# Aethelgard — roadmap

**Status:** ratified 2026-10-19 by merge of pull request #1; first drafted 2026-10-14. **Supersedes** the unmerged roadmap draft inside `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md`. Read `architecture.md` first.
**Author:** Claude Code, synthesizing the Argon (Gemini) planning session with Scotty · **Gate:** Scotty.

---

## 1. How this document works

Phases are **dependency-ordered, not calendar-ordered.** No dates are assigned, because a build done in whatever hours are available should be measured by gate completion, not by a schedule that will be wrong. Argon's drafts carried week ranges ("Weeks 1–2"); they are dropped here for that reason.

Every phase carries a **goal** in one sentence, its **dependencies**, its **work orders**, and an **exit gate** that someone who was not in the room can run and get an unambiguous yes or no. A phase ends when its gate passes, not when the code is written.

This document introduces no invariants. Those live in `northstar.md` §4 and are cited by number. A roadmap that asserts a new constraint is a defect.

**The divergence rule:** deviations are recorded as dated amendments in §8, and reconciled in §7 once plan and build diverge.

## 2. Work-order numbering

Scheme is `AET-WO-PSNN[a]`: project prefix, phase, subsection, number within the subsection, optional letter for an inserted refinement. **The scheme is defined in the ClearForge `repo-genesis` skill, §6; it is cited here, not redefined.**

Project prefix is `AET`.

**Legacy ids.** Argon's first two drafts used `AET-WO-0001` through `AET-WO-3003` (four digits, no phase encoding). None was ever merged, so there is no history to preserve; those ids are retired, mapped in §7, and never reissued. Argon's third draft (`capture 05`) already adopted this scheme and its ids are kept where the work order survived.

## 3. Critical path at a glance

```
P0 ──> P1 ──> P2 ──> P3
│       │
│       └── AET-WO-1300 (board emitter) and AET-WO-1400 (board crate)
│           run in parallel once AET-WO-1200 lands
│
└── AET-WO-0400 (trace schema) runs in parallel with all of P1
```

- **Risk concentrates in two places.** `P2`'s lock-step bridge (`OQ-02`) is where a bug produces a simulation that runs but measures nothing, and `P3`'s attribution is where a bug produces a *confident wrong answer* about which half failed. Everything else fails loudly. Those two get the adversarial pass and the most review.
- **What runs in parallel.** The trace schema and local archive (`AET-WO-0400`) depend only on the workspace, not on anything in P1. Within P1, the board emitter and the board crate both consume the solved graph and do not touch each other.
- **Human-track, not code.** These are the most likely things to block delivery while everyone stares at code, so they are named here:
  - A Nucleo-H753ZI development board (`architecture.md` `RUL-08`) has to be bought before the end of P2, so P3's simulated verdicts can be compared against one real board (northstar `NG-05`). `OQ-01` and `OQ-05` are ruled (`RUL-08`, `RUL-10`) and no longer block anything.
  - A fabrication order and its budget are needed for the P3 gate; the gate is "package emitted and verified", but the northstar's scar section stays empty until a board exists.
  - A Cloudflare R2 bucket and scoped credentials only if the optional mirror in `RUL-06` is wanted; nothing in any gate requires it.
  - **Crate names and the first publish.** `aethelgard` and every `aethelgard-*` member name were free on crates.io when checked (§8, the amendment that added this item). crates.io's usage policy (RFC 3463) removes crates that exist only to hold a name, so the names are claimed by the first real publish, not by placeholders. The first publish of a crate needs a crates.io token from Scotty's account, used once from his own machine; after that, crates.io trusted publishing lets CI release with no stored secret.
  - **The announcement, after P1's launch-readiness clause passes, never before.** Channels, in order: This Week in Rust, the Rust Embedded and embassy Matrix rooms, the KiCad forum, Antmicro (whose STM32H753 Renode twin `RUL-08` builds on), and r/embedded. Show HN waits for the P2 gate, because an injected fault traced and blamed on the right half is the headline. Announcing a scaffold spends credibility the project cannot earn back.

## 4. Phase map

| Phase | Name | Ends when |
|---|---|---|
| P0 | Foundation | The workspace builds, the pinned container runs every external tool, and a synthetic trace replays bit-for-bit |
| P1 | One source, two projections | A specification produces a checked board and a board crate in which illegal hardware use fails to compile |
| P2 | The twin and the faults | Generated firmware runs on the generated board model, and an injected electrical fault produces a replayable failure trace |
| P3 | Attribution and evolution | A failure is attributed to the right half, a proposal fixes it, and a fabrication package for one reference probe is emitted |

## 5. Sequenced work orders

### P0 — Foundation

**Goal:** make everything after this measurable: a workspace that builds, tools that run headless, and traces that replay.
**Depends on:** nothing.

**Work orders**

- `AET-WO-0100` — Rust workspace skeleton: the seven parts from `architecture.md` §3 as crates, `#![forbid(unsafe_code)]` at every crate root, and the first fail-closed specification types in `aethelgard-ir` with negative tests (`INV-06`). Argon's type sketch in capture 05 is the input.
- `AET-WO-0200` — pinned container image with `kicad-cli`, `renode`, `ngspice`, and `cargo kani` installed, and a smoke test for each. Measures the `kicad-cli` default exit code and reads the rest of Renode's platform list; both land in `architecture.md` §2 by amendment.
- `AET-WO-0300` — the `aethelgard` command-line binary: one executable, JSON on stdin and stdout, documented exit codes, subcommands for validate, build, check, prove, and simulate that each call into the crates or spawn an external tool with the subprocess posture from `architecture.md` §6. This is the contract every later caller (CI, a human, the tool server) uses (`RUL-10`).
- `AET-WO-0300a` — inserted at review of PR #12. A seccomp filter denies `socket(AF_UNIX, …)` and UDP sockets to every tool process, through a safe crate. It is spike-first, because each tool is measured with them denied (Renode's .NET diagnostic socket is the likely casualty). Landlock cannot confine either (`architecture.md` §2.5, §6). It must merge before `AET-WO-1500`, and before any tool is installed on a host with a session bus.
- `AET-WO-0300b` — inserted at Argon's review of `main` at 9818c4f. Two verdict-path gaps in the merged CLI, both cheap to close:
  - `simulate` passes a Renode load that lists a CPU, even if the log carries `[ERROR]` lines from a peripheral that failed to register. Measure which `[ERROR]` lines a clean Nucleo load prints, then refuse the rest.
  - The `prove` crate copy has no size cap. Apply `RUL-13`'s 16 MiB to the whole copy, with a file-count cap, and refuse with `InputTooLarge`.

  Build-only. It must merge before `AET-WO-1500` puts these verdicts in front of agents.
- `AET-WO-0400` — the trace archive in deterministic CBOR (`RUL-06` as amended), with a local append-only archive, a strict reader, deterministic replay, and the optional remote mirror's credential boundary. **Build-only now:** `architecture.md` answers its old spike questions, and the work order fixes the profile, layout, commit protocol and golden hashes. It measures encoded bytes per synthetic event, labelled as plumbing; `OQ-03`'s bytes per simulated second waits for real events in P2.

**Exit gate**

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all exit 0 inside the container.
- Inside the container, `kicad-cli version`, `renode --version`, `ngspice --version`, and `cargo kani --version` each exit 0.
- `kicad-cli pcb drc --exit-code-violations` on a checked-in board with one deliberate violation exits 5, and on a clean board exits 0.
- `aethelgard validate` reads a checked-in specification from stdin and exits 0 with a JSON report; the same with one zeroed voltage domain exits nonzero with a JSON error naming the domain (`INV-06`).
- A synthetic 10,000-event trace written to the local archive replays to a state sequence whose hash equals the original's (`INV-04`'s falsification, run as a test).
- With the remote-mirror credentials unset, the archive step logs one error naming the missing variable and the run still produces a verdict; with `renode` removed from `PATH`, the run halts with an error naming it and produces no verdict (`INV-06`).

### P1 — One source, two projections

**Goal:** one specification lowers to a checked board and a board crate, and the two cannot disagree.
**Depends on:** P0. Target is the STM32H753ZI (`architecture.md` `RUL-08`).

**Work orders**

- `AET-WO-1100` — carries `architecture.md` `RUL-14` (identifier set, at least one voltage domain) and `RUL-16` (`Specification` and `SolvedTopology` as separate types, no `HashMap`) as well as the graph itself. The specification graph covers buses, voltage domains, passives with electrical bounds, and round-trip serialization.
  - **The package pinout and alternate-function table come from ST's MCU database through `stm32-data`, not from the SVD,** which has neither (`architecture.md` §2.1). The work order checks `stm32-data`'s licence, and that of the ST data it carries, before any code depends on either. A licence that does not fit `RUL-11` is a flag-and-stop.
  - **It breaks today's examples on purpose.** New required fields make `examples/sentinel-01-minimal.json` invalid. The same PR updates both examples, `crates/aethelgard-ir/tests`, `crates/aethelgard-cli/tests/validate_cli.rs` and the `gates` step that validates the examples, so the P0 gate's `validate` clause holds on the new shape.
- `AET-WO-1200` — spike, then build: the constraint solver (`RUL-02`, whose prior is now a pure-Rust search). The spike measures solve time on the chosen part's full pin table for the search and for SMT. It records both numbers in `architecture.md` §2 by amendment before the build half starts, and it shows two solves of the same specification returning the same assignment.
- `AET-WO-1300` — spike, then build: the schematic emitter. It emits `.kicad_sch` and the bill of materials, and passes `kicad-cli sch erc --exit-code-violations`. The spike answers two questions:
  - how the board is to be placed and routed headless (split below);
  - how to prove that DRC applies the gate's own rule files, not the emitter's (`RUL-03` as amended).

  **Split at Argon's review of `main` at 9818c4f.** The P1 gate's DRC must exit 0, and `kicad-cli` counts every unconnected item as a finding (`crates/aethelgard-cli/src/kicad.rs` reads `unconnected_items`). So a board that is only emitted, never placed and routed, cannot pass. The original line did not say this.
- `AET-WO-1301` — reserved, scoped after `AET-WO-1300`'s spike: placement of the board.
- `AET-WO-1302` — reserved, scoped after `AET-WO-1300`'s spike. It routes the board, and it extracts the parasitics the twin will need. DRC runs against rules the gate derives from the specification (`INV-07`), with `--exit-code-violations` and `--schematic-parity`.
  - **Leads from Argon, unverified:**
    - `kicad-cli` 10 has no Specctra DSN export.
    - Freerouting would therefore need KiCad's Python module, which is Python outside Renode and so a `RUL-12` amendment. It would also need a Java runtime in the image, and its router is not known to be deterministic.
    - Zones may need their filled polygons written by the emitter for DRC to judge them.
  - **The prior is therefore a deterministic grid router in Rust inside `aethelgard-eda`.** It also gives the twin exact segment geometry for parasitics.
  - Argon's sizing, about 150 lines and under 5 ms, is unmeasured. Escape routing from the 0.5 mm-pitch package is where it will be tested.
- `AET-WO-1400` — spike, then build: the generated typestate board crate (`RUL-04`). The spike answers `OQ-04` with a harness over a harness-owned register image that catches one seeded typestate bug, or shows that it cannot, and records which. **The proof harness is generated from the specification, never taken from the crate under proof** (`INV-07`). `prove` reads Kani's exit 0 honestly, but a crate that supplies its own trivial harness earns that exit 0 legitimately. The harness is named by its full path, so `prove` always passes `--exact`. Argon raised this at its review of `main` at 9818c4f.
- `AET-WO-1500` — the `aethelgard mcp` subcommand: `rmcp` over stdio, with the tool list generated from the command tree so the CLI and MCP surfaces cannot drift (`RUL-10`). Depends on `AET-WO-0300a` and `AET-WO-0300b`. The `mcp` subcommand maps every tool to a `clap` subcommand and never forwards raw arguments (`architecture.md` §6). It also adds the `aethelgard` binary to the published tools image, so one pull gives a user or an agent both the engine and its tools. Runs in parallel with everything else in P1; nothing in P1 depends on it except the launch-readiness clause.
- `AET-WO-1600` — intent, inserted at review of PR #15: a host-side system-call allowlist for the tool sandbox, closing the non-socket channels `architecture.md` §6 lists as open on a host (System V shared memory, user namespaces, `pidfd_open`). At Argon's review of `main` at 9818c4f it also gained `ptrace`, `process_vm_readv`, `process_vm_writev` and `pidfd_getfd`, which Landlock already confines across domains, so this is defence in depth, not a gap. It also gained dropping every capability before `exec`. It must merge before the launch-readiness clause puts a crates.io-installed binary on hosts. Until then, the published image is the supported boundary for tool subcommands.

**Exit gate**

- From a checked-in specification for the reference probe, one command emits a schematic and a placed and routed board; `kicad-cli sch erc --exit-code-violations` and `kicad-cli pcb drc --exit-code-violations --schematic-parity` both exit 0 on them (`INV-01`, `INV-03`). DRC judges against design rules the gate derives from the specification, not those the emitted files carry (`INV-07`, `RUL-03`).
- The same specification with one pin deliberately assigned twice makes the solver exit nonzero naming both claimants, and emits no board files (`INV-06`).
- A `trybuild` suite of at least five firmware snippets that each touch an unconfigured pin, an ungated peripheral clock, or a bus speed above the solved rise-time ceiling fails to compile, and one valid driver compiles (`INV-02`'s falsification, run as a test).
- Regenerating from the specification twice produces byte-identical board files and board crate (`INV-01`).
- Either `cargo kani` proves the harness from `AET-WO-1400` on the generated register block, or `architecture.md` §8 records under `OQ-04` why it cannot and what replaces it. A gate that cannot be checked is fixed by amendment, not waved through.
- MCP Inspector connected to `aethelgard mcp` over stdio lists every subcommand as a tool and completes one `validate` call; adding a throwaway subcommand to the command tree makes it appear in `tools/list` with no change to the `mcp` module; `grep` over the crate finds no network listener and no credential.
- **Launch readiness.** This clause covers a public release, not the announcement (§3).
  - On a clean machine with only Docker installed, the README's quickstart reproduces this gate's first two clauses with commands copied verbatim: the reference probe passes ERC and DRC, and the doubly-assigned pin is refused, naming both claimants. The quickstart runs against the published image, which then carries the `aethelgard` binary.
  - The same binary installs from crates.io under its published name, and its tool subcommands are safe on a host because `AET-WO-1600` has merged.

### P2 — The twin and the faults

**Goal:** generated firmware runs on the generated board model with real electrical behaviour on the nets that matter, and faults injected there leave replayable traces.
**Depends on:** P1. Shape is partly unknown until the `OQ-02` spike, so work orders here are intent with reserved ids, to be scoped after the spike.

**Work orders (intent)**

- `AET-WO-2100` — emit the Renode platform description from the solved graph, adding the SPI instances Renode's own `stm32h7.repl` omits (`architecture.md` §2.2), and boot the generated firmware headless.
- `AET-WO-2101` — spike, then build: decide the lock-step transport (`OQ-02`) by measuring the surviving candidates on one I2C transaction, starting with `libngspice` behind a process boundary. It begins from an architect's desk spike, which reads the remaining leads in `architecture.md` §8 at source. That spike opens with Argon's measurement: the wall-clock cost of 1,000 synchronous ngspice transients, each one I2C byte window long. `architecture.md` §2.7 already shows that the stock I2C model cannot carry a fault flag, so the spike also chooses how a fault reaches the firmware. **Split from `AET-WO-2100` at Argon's review of `main` at 9818c4f,** so that the risk the critical path names (§3) gets its own work order and its own adversarial pass.
- `AET-WO-2200` — analog bridge: the extracted parasitics as an `ngspice` netlist for the designated nets, driven in lock-step with the digital run. If Renode's I2C model is transaction-level (`OQ-02`'s third lead), an analog fault reaches the firmware only as a controller error flag set by the bridge. The trace then records that flag as the bridge's verdict, never as an observed waveform.
- `AET-WO-2300` — fault orchestrator: scenarios authored independently of the generated artifacts (`INV-07`), including a held-low data line, an oversized pull-up, a rail droop, and an interrupt storm; every campaign writes to the P0 archive.

**Exit gate (to be refined by amendment after the `AET-WO-2100` spike, never weakened)**

- The P1 firmware image boots in the container's Renode and logs at least one complete I2C transaction with the sensor model, with no host interaction.
- With the specification's I2C pull-up changed to 47 kΩ, the campaign reports a bus failure whose trace shows the rise time exceeding the solved ceiling; with 2.2 kΩ, the same campaign passes.
- Every failing campaign's trace replays from the archive to the same state sequence hash, twice (`INV-04`).
- Trace bytes per campaign are recorded in `architecture.md` §2 by amendment, and `OQ-03` is answered or escalated.

### P3 — Attribution and evolution

**Goal:** a failure is blamed on the right half with evidence, a proposed fix is regenerated and re-gated, and a fabrication package is emitted for one reference probe.
**Depends on:** P2. `OQ-06` ruled before `AET-WO-3200`. Human-track: a physical development board and a fabrication budget (§3).

**Work orders (intent)**

- `AET-WO-3100` — attribution: from a failing trace, classify electrical versus software with the evidence span recorded, tested against a seeded set where the true cause is known.
- `AET-WO-3200` — spike, then build: the proposal loop under `RUL-07`, with the proposer chosen by `OQ-06`.
- `AET-WO-3300` — the reference probe, `Sentinel-01`: specification, full gate run, fabrication package, and a bench comparison against the physical development board that records where simulation and reality disagreed.

**Exit gate (intent; refined after the P2 spike, never weakened)**

- On a seeded set of at least twenty failing traces with known cause, attribution names the correct half for every one, and names the evidence span in each record (`INV-05`).
- Given the 47 kΩ fault from P2, the loop produces a proposal that changes only the pull-up value, the regenerated pair passes every gate, and the proposal sits on a branch unmerged (`RUL-07`).
- A fabrication package (Gerbers, bill of materials, placement file, firmware image) is emitted whose file hashes match the ones named in the passing verdict (`INV-03`).
- The bench comparison's disagreements are written into `northstar.md` §2 as the project's first scar, by amendment, whatever they are.

## 6. Standing cadence

Every work order inherits: spike first on anything novel, then stop for review; one pull request per work order, left unmerged for the architect; an adversarial pass before the feedback document; `FEEDBACK.md` findings promoted into these documents by amendment rather than left in the branch; and no number faked, a blocked measurement is reported as blocked with the harness that would produce it. Work-order structure itself is owned by the ClearForge `co-architect-work-order` skill.

Three rules added at Argon's review, each learned from a work order that ran without it:

- **Graduating a stub crate is stated, not implied.** A work order that gives a library crate its first code names the checks in `crates/aethelgard-cli/tests/workspace_shape.rs` that the crate leaves, and the checks that replace them. `AET-WO-0400` as first written said only "extend the allowed dependencies", which would have left two stub checks failing.
- **Adversarial depth follows the trust boundary.** The table below sets it per work order. A full pass by a fresh subagent goes where a mistake becomes a false pass or an escape. Pure plumbing gets a targeted pass.
- **A ruling that adds a stop condition comes with a replacement goal.** A builder holding a goal it can no longer meet loops on it. This was learned twice, in `AET-WO-0200` and in `AET-WO-0300a`.

**Spend by tier.** Work is tagged by who builds it and how deep the adversarial pass goes, so that spend follows risk. A *cheap builder* is a lower-cost model working from a fully ruled, build-only work order. A *strong builder* handles any spike or open design. The architect always reviews against CI logs, whatever the tier.

| Work order | Builder | Adversarial pass | Why |
|---|---|---|---|
| `AET-WO-0300b` | cheap | full on the two changed checks | small and fully specified, but it sits on the verdict path |
| `AET-WO-0400` | cheap | targeted: forgery, interrupted writes, hostile archives | the format and protocol are ruled; the risk is integrity, not escape |
| `AET-WO-1100` | cheap, once the architect has checked `stm32-data`'s licence and fixed the types | targeted | data modelling is cheap work once its open questions are closed at the desk (Argon's tier) |
| `AET-WO-1200` | strong for the spike, cheap for the build if the ruling is tight | targeted: determinism, conflict naming | a wrong assignment is caught by the P1 gate, not hidden |
| `AET-WO-1300` … `AET-WO-1302` | strong | full on the DRC rule injection | headless place and route is unknown, and the rule injection is `INV-07` |
| `AET-WO-1400` | strong | full | Kani over generated `unsafe` is the proof the project leans on |
| `AET-WO-1500` | cheap | full, by a strong reviewer, on the argv rule | a thin layer over the `clap` tree, but it is the agent-facing trust boundary (Argon's tier) |
| `AET-WO-1600` | strong | full | the host sandbox is a trust boundary, in hand-assembled BPF |
| `AET-WO-2100` | cheap once `AET-WO-1100` lands | targeted | platform emission is text generation, checked by Renode itself |
| `AET-WO-2101`, `AET-WO-2200`, `AET-WO-2300` | strong | full | `OQ-02` is where a simulation runs but measures nothing (§3) |
| `AET-WO-3100` … `AET-WO-3300` | strong | full | attribution is where a bug becomes a confident wrong answer (§3) |

## 7. Reconciliation — plan versus built

Rows of **number as built → what it actually was → what the roadmap had planned**. History is left as written; the table explains divergence, it does not erase it.

| number as built | what it actually was | what the roadmap had planned |
|---|---|---|
| — (pull request #1, no work order) | the four steering documents and the five session captures, written before any work order existed | Argon's `AET-WO-0100` had bundled the documents with the workspace skeleton; the documents landed first so the skeleton has something to cite |
| `AET-WO-0100` | **merged (PR #4):** seven crates, `unsafe` forbidden by workspace lint and crate attribute, fail-closed IR types that accept only JSON objects, `aethelgard validate` with exit codes 0/2/3/64/70, CI on every PR. Found and fixed serde's positional-array acceptance (`architecture.md` §2.3). Satisfies the P0 gate's `validate` clause; the fmt/clippy/test clause holds on Furnace and in CI, and inside the container since `AET-WO-0200` | Argon drafts: `AET-WO-0001` "Repo Genesis & Context Pyramid Bootstrap" |
| `AET-WO-0200` | **merged (PR #7):** spike then build. `container/Dockerfile` on the official KiCad 10.0.7 image by digest, with Renode 1.17.0, ngspice 44.2, Kani 0.68.0 and Rust 1.99.1 each pinned and checksummed. `container/smoke.sh` proves each tool on a real input, with and without the network. `container/check-pins.sh` refuses drift. A `main`-only publish job (`RUL-15`). Measured that `kicad-cli` without the flag exits 0 on violations and that a failed Renode load exits 0 (`architecture.md` §2.4). Satisfies the P0 gate's first three clauses: fmt, clippy and test inside the container; each tool runs; DRC exits 5 and 0. "Reads the rest of Renode's platform list" was already done in `architecture.md` §2.2 before this WO was written, so it was dropped from its scope | Argon drafts: `AET-WO-0002` |
| `AET-WO-0300` | **merged (PR #12):** spike, then build. `check drc`, `check erc`, `prove` and `simulate` each return one JSON verdict carrying the input's sha256 and the version of the tool that judged it, cross-checked by at least two witnesses. The sandbox is Landlock with a reaper and a guardian (`architecture.md` §6). A tier-2 `tools` job runs the real tools on every PR inside the image pinned by `container/IMAGE`. No `build` subcommand: nothing generates until `AET-WO-1300`, and a stub would have become a fake MCP tool. Satisfies the Renode half of the P0 gate's last clause. The roadmap line above still lists `build`; it is left as written, and this row explains why | Argon drafts: `AET-WO-0003`, a Rust MCP server in P0 |
| `AET-WO-0300a` | **merged (PR #15):** a required, hand-assembled seccomp filter refuses every socket a tool could reach outside itself, plus `io_uring`, x32 and non-native calls. The CLI also refuses a call that would hand the tool an inherited descriptor. The `gates` CI step closes the descriptors the Actions runner leaks. `AET-WO-1500`'s precondition is met | none; found by `AET-WO-0300`'s adversarial pass |
| `AET-WO-1600` | inserted at review of PR #15, intent: a host-side system-call allowlist; widened at Argon's review | none; the non-socket channels `AET-WO-0300a` measured as still open on a host |
| `AET-WO-0300b` | inserted at Argon's review, written, build-only: refuse Renode `[ERROR]` lines on a load, and cap the `prove` crate copy | none; Argon's red-team findings RT-02 and RT-04 |
| `AET-WO-0400` | written spike-first and held for budget, then rewritten after Argon's review as build-only in deterministic CBOR (`RUL-06` as amended), which answers its spike's format question | Argon drafts: `AET-WO-0004`, R2-primary, Cap'n Proto |
| `AET-WO-1100` … `AET-WO-1400` | reserved, `1200` and `1400` now spike-then-build. `1300` split into `1300`, `1301` and `1302` at Argon's review, because a DRC that must exit 0 needs a placed and routed board | Argon drafts: `AET-WO-1001` … `AET-WO-1004` |
| `AET-WO-1500` | reserved: the `mcp` subcommand of the CLI binary, moved from P0 to P1 | Argon drafts: part of `AET-WO-0003`, a separate server crate |
| `AET-WO-2100` … `AET-WO-2300` | reserved as intent pending the `OQ-02` spike. `2101` split from `2100` at Argon's review to hold that spike | Argon drafts: `AET-WO-2001` … `AET-WO-2003`, fully scoped |
| `AET-WO-3100` … `AET-WO-3300` | reserved as intent | Argon drafts: `AET-WO-3001` … `AET-WO-3003`, fully scoped |

Run a reconciliation audit at every phase boundary: count what was planned, what merged, and what did not, and confirm the gap is deliberate.

## 8. Amendments

| Date | Section | Change | Rationale |
|---|---|---|---|
| 2026-10-14 | — | Initial draft | Seed. Week ranges removed, KiCad gates given `--exit-code-violations`, P2 and P3 reduced to intent pending spikes, human-track items added |
| 2026-10-19 | status, §3, §5, §7 | Ratified by merge of PR #1. `AET-WO-0300` became the CLI, `AET-WO-1500` added for the `mcp` subcommand, P0 and P1 gates extended, `OQ-01`/`OQ-05` removed from human-track | Follows `architecture.md` `RUL-08`, `RUL-10`, `RUL-11` ruled the same day; the P0 gate had required a server nothing in P0 or P1 consumed |
| 2026-10-19 | §5, §7 | `AET-WO-0100` reconciled as built; `AET-WO-1100` carries `RUL-14` | PR #4 merged; its FEEDBACK F4 and F5 were ruled into work for the graph work order rather than widening `AET-WO-0100` |
| 2026-10-20 | §7 | `AET-WO-0200` reconciled as built | PR #7 merged. Three of the P0 gate's six clauses now hold, and the rest wait on `AET-WO-0300` and `AET-WO-0400` |
| 2026-10-20 | §3, §5 | Added a launch-readiness clause to the P1 exit gate. `AET-WO-1500` now also puts the `aethelgard` binary in the published image. Two human-track items added: the crate names with the first publish, and the announcement | Scotty asked for launch as a gate. Names checked against the crates.io API on this date. It sits at P1 because that is the first point with something true to show: one specification, a board and a crate that cannot disagree. The announcement itself stays human-track, since its outcome is not checkable by anyone outside the room |
| 2026-10-21 | §5, §7 | `AET-WO-0300` reconciled as built. `AET-WO-0300a` inserted, and `AET-WO-1500` made to depend on it | PR #12 merged. The insertion closes a sandbox gap that its adversarial pass measured, before an agent-facing surface can reach it |
| 2026-10-21 | §5, §7 | `AET-WO-0300a` reconciled as built. `AET-WO-1600` inserted as intent, and the launch-readiness clause made to depend on it | PR #15 merged. Host use of the tool subcommands still has non-socket channels open, and a crates.io install is host use |
| 2026-10-29 | §5, §6, §7 | Inserted `AET-WO-0300b`. `AET-WO-0400` made build-only in CBOR. `AET-WO-1100` now takes pins from `stm32-data` and breaks the examples on purpose. `AET-WO-1300` split into `1300`, `1301` and `1302`, and `AET-WO-2101` split from `2100`. `AET-WO-1400`'s harness now comes from the specification. `AET-WO-1600` widened. The P1 gate's first clause now names a routed board and spec-derived DRC rules. §6 gained three cadence rules and a spend-by-tier table | Rulings on Argon's review of `main` at 9818c4f, with the matching `architecture.md` amendments of the same date. The gate change strengthens a clause and weakens none. The tier table exists because builds are paused for budget, and the cheapest safe builder for each work order is now a planning input. Argon's answers to the architect's follow-up questions reached the `AET-WO-1302` line (routing) and the `AET-WO-2101` line (the I2C model, read at source in `architecture.md` §2.7). Its tier table moved `1100` and `1500` to cheap builders |

## 9. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, `architecture.md`, and the phase and work-order structure in Argon's captures 03 and 05 under `docs/inputs/argon-session/`.
