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
- `AET-WO-0300c` — inserted at review of PR #29, written, build-only, strong tier: closes the reaper's start-up race, which `AET-WO-1500`'s adversarial pass measured. 17 of 2,500 immediate MCP cancels left a tool running under PID 1.
  - The reaper and guardian take the CLI's pid from the CLI instead of sampling it.
  - The guardian starts before its directory exists.
  - The CLI dies with its parent.
  - `mcp` handles SIGHUP.

  It must merge before `AET-WO-1501` publishes the binary, and before any release.
- `AET-WO-0300d` — **merged (PRs #39 and #41)**, inserted at review of PR #32, build-only, cheap tier: once the CLI is gone, the guardian retries removing its directory, within 10 s, instead of trying once. PR #32's `gates` run left one directory with no surviving process. The trampoline can still be creating `status-*` when the guardian's single `remove_dir_all` runs, and nothing retries it. A leaked directory, not a leaked process, so `AET-WO-1501` does not wait on it.
- `AET-WO-0400` — the trace archive in deterministic CBOR (`RUL-06` as amended), with a local append-only archive, a strict reader, deterministic replay, and the optional remote mirror's credential boundary. **Build-only now:** `architecture.md` answers its old spike questions, and the work order fixes the profile, layout, commit protocol and golden hashes. It measures encoded bytes per synthetic event, labelled as plumbing; `OQ-03`'s bytes per simulated second waits for real events in P2.
- `AET-WO-0400a` — inserted at review of PR #21, written, build-only. It makes three changes:
  - the state chain hashes each event's change, so replay is linear, and the manifest moves to version 2;
  - decoding is bounded by shape, with a 256 MiB trace cap;
  - a stale `mirror.cbor.tmp` no longer orphans a committed run.

  It must merge before any trace this project did not write is replayed, and before `AET-WO-2300`'s first campaign.

**Exit gate: passed, as reconciled in §7.** Every clause below is owned by merged work.

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

- `AET-WO-1100` — written, build-only. It carries `architecture.md` `RUL-14` (the identifier set, at least one voltage domain) and `RUL-16` (`Specification` and `SolvedTopology` as separate types, no `HashMap`). The specification graph is version 1, covering:
  - the microcontroller;
  - voltage domains;
  - I2C and SPI buses, each with an optional hand-pinned signal;
  - I2C and SPI devices;
  - the I2C pull-up;
  - net constraints, with round-trip serialization.

  **It breaks today's examples on purpose.** The same PR rewrites both, keeping their file names so the `gates` step needs no change, and the P0 gate's `validate` clause holds on the new shape.
- `AET-WO-1101` — written, build-only: the microcontroller's pin table.
  - **The source.** It vendors ST's BSD-3-Clause `STM32_open_pin_data` files for the STM32H753ZI, pinned by commit and sha256 with ST's notice (`architecture.md` §2.8). The pin and alternate-function tables are not in the SVD, which has neither (`architecture.md` §2.1).
  - **The parse.** `aethelgard-svd-hal` parses the files into an IR pin table. That graduates the crate from the stub checks, with a pure-Rust XML parser as its one new dependency.
  - **The golden facts.** 114 I/O pins, and PB8 carries `I2C1_SCL` on AF4.
  - **The order.** It depends on `AET-WO-1100`, and `AET-WO-1200` depends on it.
- `AET-WO-1200` — **merged (PR #32)**, spike then build: the constraint solver (`RUL-02`, whose prior is now a pure-Rust search), as `solve` in `aethelgard-ir` and `aethelgard solve` in the CLI. The spike measures Z3 only if the search exceeds 1 s on a stress specification. It also settles how `SolvedTopology` records a GPIO-driven chip select, which has no alternate function. The spike measures solve time on the chosen part's full pin table for the search and for SMT. It records both numbers in `architecture.md` §2 by amendment before the build half starts, and it shows two solves of the same specification returning the same assignment.
- `AET-WO-1201` — reserved at review of PR #32, cheap tier: the forced-instance check proposed by `AET-WO-1200`'s adversarial reviewer. When the buses left of a kind equal that kind's placeable instances, every instance is forced, so pin matching runs over (instance, signal) rows. It can only prune, so a differential test over fuzzed specifications, with and without the check, must show no change except `BudgetExceeded` becoming `Infeasible`. Its inputs are the reproducers in `docs/work-orders/AET-WO-1200.repro/`. It closes `AET-WO-1200` A1 and revisits A2, `BudgetExceeded` sharing exit 2.
- `AET-WO-1300` — written, spike, then build: the schematic emitter, and the gate's own DRC rules (`check drc --spec`). It emits `.kicad_sch` and the bill of materials, and passes `kicad-cli sch erc --exit-code-violations`. The spike answers two questions:
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
- `AET-WO-1400` — written, spike ruled on PR #42 (`OQ-04` answered yes, `RUL-04` amended to a dependency-free crate beside `embassy`), build in progress: the generated typestate board crate (`RUL-04`), generated from `SolvedTopology` in `aethelgard-svd-hal`. The tools image has no `thumbv7em` target, so the spike measures what adding one would take, and proposes it for a ruling. The spike answers `OQ-04` with a harness over a harness-owned register image that catches one seeded typestate bug, or shows that it cannot, and records which. **The proof harness is generated from the specification, never taken from the crate under proof** (`INV-07`). `prove` reads Kani's exit 0 honestly, but a crate that supplies its own trivial harness earns that exit 0 legitimately. The harness is named by its full path, so `prove` always passes `--exact`. Argon raised this at its review of `main` at 9818c4f.
- `AET-WO-1500` — the `aethelgard mcp` subcommand: `rmcp` over stdio, with the tool list generated from the command tree so the CLI and MCP surfaces cannot drift (`RUL-10`). Written, build-only. Depends on `AET-WO-0300a` and `AET-WO-0300b`, both merged.
  - **Calls.** Each tool call runs the binary itself, as a child, with an argument vector built only from that subcommand's declared arguments. So `mcp` maps every tool to a `clap` subcommand and never forwards raw arguments (`architecture.md` §6), and every call keeps the CLI's sandbox and exit codes.
  - **The proof.** A stdio protocol test in CI proves the gate's MCP clause, and MCP Inspector is the documented human check.
  - **Parallel.** It runs in parallel with everything else in P1. Nothing in P1 depends on it except the launch-readiness clause.
- `AET-WO-1501` — written, build-only: publishes a second, engine image, `ghcr.io/clearforge-llc/aethelgard`. It is the tools image, pinned by digest, plus the binary built inside it from source. The engine image follows `RUL-15`'s publish path exactly, and the tools image is untouched. The original intent was to add the binary to the published tools image, so one pull gives a user or an agent both the engine and its tools. **Split from `AET-WO-1500`,** because it changes `container/` and the publish job (`RUL-15`), which `AET-WO-1500`'s fence protects. The launch-readiness clause depends on it, and it depends on `AET-WO-0300c`.
- `AET-WO-1502` — reserved at the `AET-WO-1400` spike ruling (F5), cheap tier: when `prove`, or any tool subcommand, cannot recognise the tool's outcome, its `ToolError` message carries the tool's first `error:` line. Today a crate whose dependency cannot resolve gives `ToolError` with no hint why.
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
- **Tier 2 is proven in CI, not on Furnace.** Furnace has no tools image pulled. Since 2026-11-07 it does have rootless `podman` 5.7.0, so a builder *may* pull the pinned image there, but it is never required, and CI stays the witness. So a work order's "passes on Furnace" means tier 1 and the workspace gates. Tier 2 is accepted from CI's `tools` job, inside the image pinned by `container/IMAGE`. Added at review of PR #29, after `AET-WO-1100`, `1101` and `1500` each had to record the same stop.
- **A ruling that adds a stop condition, or removes a clause from a goal, comes with a replacement goal.** A builder holding a goal it can no longer meet loops on it. This was learned in `AET-WO-0200` and in `AET-WO-0300a`. It was learned a third time in `AET-WO-1200`: a steer said not to run tier 2 on Furnace while the goal still demanded it, and the goal's stop hook blocked nine times.
- **A test that asserts the outcome of a race must force that outcome.** If scheduling can let either side win, the test is nondeterministic, however often it passes. Review checks this. `AET-WO-0300d`'s first bound test asserted that a writer always beats `remove_dir_all`. It passed on its own PR, then failed on every later `tools` run. The follow-up, PR #41, replaced it with a unit test whose remover always fails.
- **A fixed shared test that goes red elsewhere is the architect's first job.** One nondeterministic test on `main` turned three open PRs red at once. The fix was a follow-up PR merged first, then a note telling every builder to merge `main`.

**Spend by tier.** Work is tagged by who builds it and how deep the adversarial pass goes, so that spend follows risk. A *cheap builder* is a lower-cost model working from a fully ruled, build-only work order. A *strong builder* handles any spike or open design. The architect always reviews against CI logs, whatever the tier.

| Work order | Builder | Adversarial pass | Why |
|---|---|---|---|
| `AET-WO-0300b` | cheap | full on the two changed checks | small and fully specified, but it sits on the verdict path |
| `AET-WO-0300c` | strong | full | the sandbox lifecycle is a trust boundary |
| `AET-WO-0300d` | cheap | targeted, on the guardian | one function, a fully specified retry; the reproduction comes first |
| `AET-WO-0400` | cheap | targeted: forgery, interrupted writes, hostile archives | the format and protocol are ruled; the risk is integrity, not escape |
| `AET-WO-0400a` | cheap | targeted | three measured fixes, each fully specified |
| `AET-WO-1100` | cheap | targeted | data modelling, with every rule fixed in the work order (Argon's tier) |
| `AET-WO-1101` | cheap | targeted: hostile XML, pinned-hash refusal | a parse of two pinned files, checked against golden facts |
| `AET-WO-1201` | cheap | targeted: the differential test | a pruning check whose only allowed effect is earlier refusal |
| `AET-WO-1200` | strong for the spike, cheap for the build if the ruling is tight | targeted: determinism, conflict naming | a wrong assignment is caught by the P1 gate, not hidden |
| `AET-WO-1300` … `AET-WO-1302` | strong | full on the DRC rule injection | headless place and route is unknown, and the rule injection is `INV-07` |
| `AET-WO-1400` | strong | full | Kani over generated `unsafe` is the proof the project leans on |
| `AET-WO-1500` | cheap | full, by a strong reviewer, on the argv rule | a thin layer over the `clap` tree, but it is the agent-facing trust boundary (Argon's tier) |
| `AET-WO-1501` | cheap | targeted: the published image still passes smoke under `--network none` | packaging, inside `RUL-15`'s existing publish path |
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
| `AET-WO-0300c` | **merged (PR #34):** the reaper's start-up race is closed. The CLI's pid is passed instead of sampled, the guardian starts first, and the parent-death signal is armed only under `mcp`. Storm survivors went from 8 to 0, three runs in a row (`architecture.md` §6) | none; `AET-WO-1500`'s adversarial pass |
| `AET-WO-0300d` | **merged (PRs #39 and #41):** the guardian retries removal every 20 ms for up to 10 s, and exits 70 if it cannot. Reproduction on `main`: 10 of 10 tries leaked; with the fix, 0. The first bound test was nondeterministic and was replaced by a unit test (§6) | none; CI on PRs #32 and #36 |
| `AET-WO-1200` | **merged (PR #32):** `solve` and `aethelgard solve`, a fixed-order search with instance and pin matching, a 10 M-node budget, and reserved SWD and oscillator pins. Chip selects are placed last into a `gpio` map. 2.4 to 3.0 ms end to end on Furnace (`architecture.md` §2.11). Adversarial A1, an infeasible shape that spends the budget, goes to `AET-WO-1201` | Argon drafts: `AET-WO-1002`, a Z3-first solver |
| `AET-WO-0300b` | **merged (PR #18):** a CPU listed beside an `[ERROR]` line fails the load, and every tool input copy is capped at 16 MiB, with 4,096 files for a crate. Measured that Renode 1.17.0 never logs a platform failure at `[ERROR]`, so that check is a backstop (`architecture.md` §2.9). The first cheap-builder pilot: one session, 22 minutes, CI green on the first push, and no high or medium adversarial finding. `AET-WO-1500`'s second precondition is met | none; Argon's red-team findings RT-02 and RT-04 |
| `AET-WO-0400` | **merged (PR #21):** the trace archive in a hand-written strict CBOR codec, a commit protocol proven by SIGKILL tests, replay with a state-hash chain, and the mirror's credential boundary. `aethelgard-chaos` graduated from the stub checks. Golden constants were computed on Furnace and reproduced in `gates` (`architecture.md` §2.10). It satisfies the P0 gate's fifth clause and the mirror half of its sixth. Second cheap-builder pilot: one session, 24 minutes, CI green on the first push | Argon drafts: `AET-WO-0004`, R2-primary, Cap'n Proto |
| `AET-WO-0400a` | inserted at review of PR #21, written: adversarial findings 1, 2 and 3 | none; `AET-WO-0400`'s adversarial pass |
| **P0 gate** | **passed.** Planned: `0100`, `0200`, `0300`, `0400`. Inserted: `0300a`, `0300b`. All six merged, and every gate clause is owned by merged work. `0400a` is a post-gate refinement. The one deliberate gap is still the `build` subcommand (see the `0300` row) | the plan's four work orders |
| `AET-WO-1100` … `AET-WO-1400` | reserved, `1200` and `1400` now spike-then-build. `1100` written build-only, and `1101` split from it for the pin table, after the architect's licence check. `1300` split into `1300`, `1301` and `1302` at Argon's review, because a DRC that must exit 0 needs a placed and routed board | Argon drafts: `AET-WO-1001` … `AET-WO-1004` |
| `AET-WO-1500` | written, build-only: the `mcp` subcommand of the CLI binary, moved from P0 to P1. The image half was split out as `1501` | Argon drafts: part of `AET-WO-0003`, a separate server crate |
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
| 2026-11-07 | §5, §6, §7 | `AET-WO-1100` written build-only. `AET-WO-1101` split from it for the pin table, taken from ST's own BSD-3-Clause data. Tier rows updated | The licence question that held `AET-WO-1100` is answered (`architecture.md` §2.8), and a separate pin-table work order keeps each build small enough for a cheap builder |
| 2026-11-07 | §7 | `AET-WO-0300b` reconciled as built | PR #18 merged. The cheap-builder tier held on its first pilot |
| 2026-11-07 | §5, §6, §7 | `AET-WO-0400` reconciled as built. `AET-WO-0400a` inserted. The P0 gate is marked passed after the phase-boundary audit | PR #21 merged. Every P0 clause is owned by merged work. The audit counted four planned and two inserted work orders, all merged, and one deliberate gap |
| 2026-11-07 | §5, §6, §7 | `AET-WO-1500` written build-only. `AET-WO-1501` split from it to add the binary to the published image | The image change touches `container/` and the publish job, which a cheap builder on the `mcp` work order should not. Splitting keeps each fence simple |
| 2026-11-07 | §5, §6 | `AET-WO-0300c` inserted, and `AET-WO-1501` made to depend on it | `AET-WO-1500`'s adversarial pass found that a CLI killed during start-up leaves its tool running, because the reaper samples its parent's pid. The bug predates `mcp`, but `mcp` makes it routine, since every cancel kills the CLI |
| 2026-11-07 | §5, §7 | `AET-WO-0300c` merged. `AET-WO-1501` written, as a second engine image rather than a change to the tools image | PR #34 merged with zero survivors. The tools image rebuilds only when `container/` changes and cannot see the source, so the engine needs its own image, built on the tools image's pinned digest |
| 2026-11-07 | §5, §6 | `AET-WO-0300d` inserted | PR #32's `gates` run (42424760372) left one scratch directory with 0 survivors in 0300c's parent-kill test. The guardian removes its directory once and ignores the result, while the trampoline may still be writing in it |
| 2026-11-08 | §5, §6, §7 | `AET-WO-1200` and `AET-WO-0300d` reconciled as built. `AET-WO-1201` reserved. The `AET-WO-1400` spike ruled. §6 gains the race-test rule and the shared-test rule, and widens the replacement-goal rule. The tier-2 rule now records podman on Furnace | PRs #32, #39, #41 and #42. One nondeterministic test on `main` reddened three PRs. A steer that dropped a goal clause without replacing the goal stalled a builder |

## 9. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, `architecture.md`, and the phase and work-order structure in Argon's captures 03 and 05 under `docs/inputs/argon-session/`.
