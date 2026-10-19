# Aethelgard — north star

**Status:** ratified 2026-10-19 by merge of pull request #1; first drafted 2026-10-14. **Supersedes** the unmerged northstar draft inside `docs/inputs/argon-session/05-bootstrap-script-v2-repo-genesis.md`. Read before `architecture.md`.
**Author:** Claude Code, synthesizing the Argon (Gemini) planning session with Scotty · **Gate:** Scotty · **Builder:** a separate build session per work order.

> Ratification is the gate's act, not the author's. Argon's draft carried the word "Ratified" on documents nobody had ratified; this one carried "draft" until Scotty merged it.

---

## 1. The one sentence

**Aethelgard generates a circuit board and the firmware that drives it from one specification, and refuses to call either finished until the pair has survived simulated electrical and software faults together, before any board is manufactured.**

The sentence names the capability (board and firmware from one source) and the two constraints that shape every decision downstream: the pair is judged together, and the judgement happens before fabrication. It names no language, no simulator, no vendor. If the whole stack were replaced, the sentence would not change.

## 2. The problem, stated precisely

A smart physical device is two halves of one machine. The board is drawn in an electronics design tool; the firmware is written in a compiler toolchain. Each tool checks its own half against itself. The schematic checker confirms the netlist is internally consistent. The compiler confirms the firmware type-checks. Neither checks the pair: that the pin the driver toggles is the pin the sensor is wired to, that the bus speed the driver configures is one the copper and the pull-up resistor can actually deliver, that the brownout the regulator permits is one the firmware recovers from.

The defect therefore surfaces at the most expensive moment: after fabrication, when a physical board arrives and does not work, and the question "is it the hardware or the software?" costs weeks of bench time and a board respin.

One-shot generative design makes this worse, not better. A hallucinated line of software fails to compile in seconds. A hallucinated pin assignment or an unrecovered bus lockup is invisible until the fabrication lead time and the parts budget have been spent, or until deployed units fail in the field.

**This project has no scar of its own yet.** No Aethelgard board has been fabricated, so there is no incident to cite. The evidence above is the general pattern of the industry (respins, bring-up delays, field truck-rolls), which is weaker than a specific failure. The first fabricated reference board will produce the first real evidence, and the architecture's corrections table should record it either way.

### 2.1 Why the existing tooling did not already solve this

KiCad's ERC and DRC prove a schematic and a layout agree with their own rules. `rustc` proves the firmware agrees with its own types. Renode runs real firmware against a board model whose signals are ideal: a logic-level transition is instantaneous and a bus never rings. Each tool is good at its half. The gap was never "can we check the board" or "can we check the firmware." It was that **no tool holds both halves and the physics between them in one place**, so no tool can ask whether the pair survives a fault that crosses the seam.

## 3. End state

A builder who has not read this document in a month writes a specification for a telemetry probe: one microcontroller, two sensors on an I2C bus, a flash chip on SPI, one power rail. They run Aethelgard and get back a schematic, a layout, a board-support firmware crate, a trace archive, and one verdict that names every artifact it judged.

Concretely:

- They write a driver that reads a pin the specification never configured. It does not compile.
- They change the I2C pull-up in the specification to a value too large for the trace capacitance. The fault campaign fails, and the verdict says *electrical: SDA rise time exceeds the 300 ns ceiling the bus speed requires*, not *firmware timeout*.
- The loop proposes a smaller pull-up, regenerates both halves, reruns every gate, and the verdict passes. The proposal, the attribution, and the rerun are all in the trace archive.
- They ask for the fabrication package and get Gerbers, a bill of materials, and a firmware image whose hashes match the verdict.
- The first physical board is still tested on a bench, because simulation narrows the search; it does not replace the board.

## 4. Invariants — true regardless of what any work order asks

Numbered, because work orders and code cite them by number. Each carries the observation that would disprove it. `INV-07` is new relative to Argon's draft and the reason is given beside it; Scotty decides whether it stays.

| # | Invariant | How to falsify |
|---|---|---|
| 1 | **`INV-01` One source.** Every board artifact (schematic, netlist, layout constraint, bill of materials) and every firmware hardware interface (pin assignment, peripheral configuration, timing bound) is regenerated from the one specification. Nothing emitted is hand-edited and kept. *This invariant is the project.* | Regenerate from the specification and diff against what was shipped; any difference the specification does not explain falsifies |
| 2 | **`INV-02` Illegal hardware use fails before a binary exists.** Firmware that touches a pin, peripheral, clock domain, or timing outside what the specification solved does not build. | Write such firmware; a successful build falsifies |
| 3 | **`INV-03` "Verified" means all three gates ran on that exact pair.** An artifact pair is verified only when the electrical rule check, the memory-safety proof, and the fault campaign all passed against the same hashes. | Find any pair marked verified for which one gate did not run on those hashes |
| 4 | **`INV-04` Every simulated run replays from its own trace.** A run's recorded trace is sufficient to reproduce its state sequence deterministically without re-running the solvers, at the simulator's own time resolution. | Replay a trace twice and get different state sequences; or find a run with no trace |
| 5 | **`INV-05` Attribution precedes mutation.** No change is applied to the design in response to a failure until the failure is attributed to the electrical domain or the software domain with recorded evidence. A software retry never masks an electrical defect, and vice versa. | Find a mutation with no attribution record, or a software patch applied for a failure the record attributes to electrical cause |
| 6 | **`INV-06` Absence halts.** A missing component model, missing tool, or missing credential halts synthesis with an error naming what is missing. Nothing substitutes a permissive default or a fabricated measurement. | Remove a tool or a model; a run that still produces a verdict falsifies |
| 7 | **`INV-07` The gates are not computed from the artifact they judge.** Pass criteria come from the specification's declared requirements and from independently authored fault scenarios, never from the generated firmware or board. *New in this draft: without it, the evolution loop can satisfy its own gates by changing what they measure.* | Find a gate whose expected value is read from the generated artifact under test |

Seven is inside the working range. Argon's draft had six; `INV-07` was added because the loop in `INV-05` mutates the design until gates pass, and a loop that can also move the gates proves nothing.

## 5. Non-goals — deliberately not built

Each carries its reason, because the reason is what stops it being relitigated every third work order.

- **`NG-01` A general-purpose operating system.** Aethelgard produces bare-metal board support and deterministic sensor and actuator loops, not a POSIX userland. *Reason:* unbounded dynamic allocation and a process model destroy the determinism `INV-02` and `INV-04` depend on.
- **`NG-02` Interactive schematic editing.** There is no drag-and-drop capture tool and no supported path for editing emitted files. The escape hatch is editing the specification. *Reason:* a hand edit to an output is exactly the drift `INV-01` forbids, and a GUI invites it.
- **`NG-03` Silicon tape-out.** Board-level systems with commercial microcontrollers, sensors, and passives only; no transistor-level design. *Reason:* the bottleneck being attacked is board bring-up, and silicon needs a foundry toolchain this project will never own.
- **`NG-04` Letting a language model be the admission authority.** A model may write specifications and propose mutations. It never decides pass or fail. *Reason:* this is the tempting-but-wrong solution. The whole premise is that a plausible-looking hallucination is caught before fabrication; a model judging its own output recreates the one-shot problem the project exists to fix.
- **`NG-05` Proving the physical board correct.** Simulation narrows the search for defects; it does not prove the fabricated board works. The first physical board remains a test, not a formality. *Reason:* parasitics, silicon behaviour, and sensor models are approximations. Claiming otherwise is the "green for fourteen months" failure in hardware form.

## 6. What would make this a failure

- Verdicts pass because the loop learned to satisfy the gates rather than the requirement.
- Simulation fidelity drifts so far from the physical board that the first real board fails in ways simulation never saw, and nothing feeds that back into the models.
- Trace volume grows until traces are sampled or dropped, and `INV-04` holds on paper only.
- The specification becomes so hard to write that people hand-edit outputs, and the project becomes a KiCad script generator with a slogan.
- One microcontroller family is supported so deeply that the intermediate representation is secretly that vendor's register map.

## 7. Amendments

| Date | Section | Change | Rationale |
|---|---|---|---|
| 2026-10-14 | — | Initial draft | Seed. Synthesized from the five Argon session captures; `INV-07` and `NG-04`, `NG-05` added by Claude with reasons stated inline |
| 2026-10-19 | status | Ratified | Scotty merged PR #1 with `INV-07`, `NG-04`, and `NG-05` called out in the review summary, and delegated the remaining open questions to Claude. No invariant text changed |

## 8. Provenance

Written 2026-10-14 by Claude Code in session `claude/argon-repo-docs-synthesis-rjrb0d`, from the founding Google Doc and the five captured turns of the Gemini "Capabilities and Dream Projects" session between Scotty (Addison Aliff, ClearForge-LLC) and Argon (Gemini), preserved under `docs/inputs/argon-session/` as captures 00 through 05. The one sentence, `INV-01` through `INV-06`, and `NG-01` through `NG-03` are reworded from Argon's draft northstar in capture 05; the wording changes make each invariant falsifiable and remove stack names. Rests on nothing else.
