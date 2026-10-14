# Aethelgard — roadmap

**Status:** draft for ratification, 2026-10-14. **Supersedes** the unmerged roadmap draft inside `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md`. Read `architecture.md` first.
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
  - Scotty rules on `OQ-01` (reference microcontroller) before `AET-WO-1100` can start.
  - Scotty rules on `OQ-05` (tool server language) before `AET-WO-0300` can start.
  - A physical development board for the chosen target has to be bought before the end of P2, so P3's simulated verdicts can be compared against one real board (northstar `NG-05`).
  - A fabrication order and its budget are needed for the P3 gate; the gate is "package emitted and verified", but the northstar's scar section stays empty until a board exists.
  - A Cloudflare R2 bucket and scoped credentials only if the optional mirror in `RUL-06` is wanted; nothing in any gate requires it.

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
**Depends on:** nothing. `OQ-05` must be ruled before `AET-WO-0300`.

**Work orders**

- `AET-WO-0100` — Rust workspace skeleton: the seven parts from `architecture.md` §3 as crates, `#![forbid(unsafe_code)]` at every crate root, and the first fail-closed specification types in `aethelgard-ir` with negative tests (`INV-06`). Argon's type sketch in capture 05 is the input.
- `AET-WO-0200` — pinned container image with `kicad-cli`, `renode`, `ngspice`, and `cargo kani` installed, and a smoke test for each. Measures the `kicad-cli` default exit code and reads the rest of Renode's platform list; both land in `architecture.md` §2 by amendment.
- `AET-WO-0300` — tool server exposing build, ERC/DRC, proof, and simulation runs as tool calls, with the subprocess posture from `architecture.md` §6.
- `AET-WO-0400` — Cap'n Proto trace schema, local append-only archive, deterministic replay reader, and the optional remote mirror behind named credentials (`RUL-06`). Measures trace bytes per simulated second and records it for `OQ-03`.

**Exit gate**

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all exit 0 inside the container.
- Inside the container, `kicad-cli version`, `renode --version`, `ngspice --version`, and `cargo kani --version` each exit 0.
- `kicad-cli pcb drc --exit-code-violations` on a checked-in board with one deliberate violation exits 5, and on a clean board exits 0.
- A synthetic 10,000-event trace written to the local archive replays to a state sequence whose hash equals the original's (`INV-04`'s falsification, run as a test).
- With the remote-mirror credentials unset, the archive step logs one error naming the missing variable and the run still produces a verdict; with `renode` removed from `PATH`, the run halts with an error naming it and produces no verdict (`INV-06`).

### P1 — One source, two projections

**Goal:** one specification lowers to a checked board and a board crate, and the two cannot disagree.
**Depends on:** P0. `OQ-01` must be ruled before `AET-WO-1100`.

**Work orders**

- `AET-WO-1100` — specification graph: microcontroller package and alternate-function table ingested from the vendor register description, buses, voltage domains, passives with electrical bounds; round-trip serialization.
- `AET-WO-1200` — spike, then build: the constraint solver (`RUL-02`). The spike measures solve time on the chosen part's full pin table with both the SMT prior and a pure-Rust search, and records the number in `architecture.md` §2 by amendment before the build half starts.
- `AET-WO-1300` — board emitter: `.kicad_sch`, `.kicad_pcb`, bill of materials, and the parasitic extraction the twin will need, validated by `kicad-cli` with `--exit-code-violations`.
- `AET-WO-1400` — spike, then build: the generated typestate board crate (`RUL-04`). The spike answers `OQ-04` with a harness that proves one real register-block property or shows that it cannot, and records which.

**Exit gate**

- From a checked-in specification for the reference probe, one command emits a schematic and a board; `kicad-cli sch erc --exit-code-violations` and `kicad-cli pcb drc --exit-code-violations --schematic-parity` both exit 0 on them (`INV-01`, `INV-03`).
- The same specification with one pin deliberately assigned twice makes the solver exit nonzero naming both claimants, and emits no board files (`INV-06`).
- A `trybuild` suite of at least five firmware snippets that each touch an unconfigured pin, an ungated peripheral clock, or a bus speed above the solved rise-time ceiling fails to compile, and one valid driver compiles (`INV-02`'s falsification, run as a test).
- Regenerating from the specification twice produces byte-identical board files and board crate (`INV-01`).
- Either `cargo kani` proves the harness from `AET-WO-1400` on the generated register block, or `architecture.md` §8 records under `OQ-04` why it cannot and what replaces it. A gate that cannot be checked is fixed by amendment, not waved through.

### P2 — The twin and the faults

**Goal:** generated firmware runs on the generated board model with real electrical behaviour on the nets that matter, and faults injected there leave replayable traces.
**Depends on:** P1. Shape is partly unknown until the `OQ-02` spike, so work orders here are intent with reserved ids, to be scoped after the spike.

**Work orders (intent)**

- `AET-WO-2100` — spike, then build: emit the Renode platform description from the solved graph, boot the generated firmware headless, and decide the lock-step transport (`OQ-02`) by measuring all three candidates on one I2C transaction.
- `AET-WO-2200` — analog bridge: the extracted parasitics as an `ngspice` netlist for the designated nets, driven in lock-step with the digital run.
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

## 7. Reconciliation — plan versus built

Rows of **number as built → what it actually was → what the roadmap had planned**. History is left as written; the table explains divergence, it does not erase it.

| number as built | what it actually was | what the roadmap had planned |
|---|---|---|
| — (pull request #1, no work order) | the four steering documents and the five session captures, written before any work order existed | Argon's `AET-WO-0100` had bundled the documents with the workspace skeleton; the documents landed first so the skeleton has something to cite |
| `AET-WO-0100` | reserved: workspace skeleton only | Argon drafts: `AET-WO-0001` "Repo Genesis & Context Pyramid Bootstrap" |
| `AET-WO-0200` | reserved | Argon drafts: `AET-WO-0002` |
| `AET-WO-0300` | reserved | Argon drafts: `AET-WO-0003` |
| `AET-WO-0400` | reserved, now local-first (`RUL-06`) | Argon drafts: `AET-WO-0004`, R2-primary |
| `AET-WO-1100` … `AET-WO-1400` | reserved, `1200` and `1400` now spike-then-build | Argon drafts: `AET-WO-1001` … `AET-WO-1004` |
| `AET-WO-2100` … `AET-WO-2300` | reserved as intent pending the `OQ-02` spike | Argon drafts: `AET-WO-2001` … `AET-WO-2003`, fully scoped |
| `AET-WO-3100` … `AET-WO-3300` | reserved as intent | Argon drafts: `AET-WO-3001` … `AET-WO-3003`, fully scoped |

Run a reconciliation audit at every phase boundary: count what was planned, what merged, and what did not, and confirm the gap is deliberate.

## 8. Amendments

| Date | Section | Change | Rationale |
|---|---|---|---|
| 2026-10-14 | — | Initial draft | Seed. Week ranges removed, KiCad gates given `--exit-code-violations`, P2 and P3 reduced to intent pending spikes, human-track items added |

## 9. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`. Rests on `northstar.md`, `architecture.md`, and the phase and work-order structure in Argon's captures 03 and 05 under `docs/inputs/argon-session/`.
