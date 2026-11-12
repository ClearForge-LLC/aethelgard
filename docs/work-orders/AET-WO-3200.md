# AET-WO-3200 — The proposal loop: rules first, generator defects reported, gates never moved (`RUL-07`, `RUL-18`)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3200-propose`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a **full** adversarial pass (`docs/roadmap.md` §6). A proposer that can move what a gate measures satisfies its own gates and proves nothing (`INV-07`).
**Phase:** P3. It owns the P3 exit gate's second clause: "Given the 47 kΩ fault from P2, the loop produces a proposal that changes only the pull-up value, the regenerated pair passes every gate, and the proposal sits on a branch unmerged (`RUL-07`)."

**Grounds:**
- **`AET-WO-3100`, merged (PR #79).** `aethelgard attribute` names a domain, the deciding rule and the evidence from an archived run alone.
- **`RUL-18`** (`OQ-06`, ruled by Scotty 2026-11-12):
  - Proposals come from deterministic rules first. A model proposer may be consulted later, only where no rule matches, and is labelled as such.
  - This work order builds the rules and the proposer interface, not a model proposer.
  - A **software** attribution in generated firmware is a generator defect. It yields a structured generator bug report, never a specification diff.
- **`RUL-07`:** the loop proposes a specification diff with an attribution record. It never applies the diff to `main`, and never alters a gate.
- **`INV-05`:** attribution precedes mutation, so `indeterminate` and `none` propose nothing. **`INV-07`:** a constraint the gates judge (a rise-time ceiling, a capacitance bound) is never a proposal's lever.
- **`architecture.md` §2.18:** the measured rise is linear in R × C on this bridge.

> **What this is:**
> - `aethelgard_evolve`'s proposer interface, with one rules proposer;
> - `aethelgard propose`, which turns an archived run and its scenario into a proposal, a generator bug report, or a stated refusal;
> - the P3 gate's second clause, end to end, on a proposal branch.
>
> It is NOT:
> - a model proposer (a later work order, under `RUL-18`);
> - anything that writes to `main`, merges, or edits a gate, a ceiling or a constraint;
> - a change to `campaign`, `replay`, `attribute`, `simulate --firmware`, the judge, or any generator.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged, plus the proposal branch of §3.3.

## 1. Scope

1. **The proposer interface** in `aethelgard-evolve`.
   - A proposer takes an `Attribution` and the solved facts the CLI hands it, all as plain integers and identifiers. `evolve` gains no `aethelgard-ir` dependency (`RUL-16`, 3100 F9).
   - It returns exactly one of:
     - `Proposal`: closed-world changes in the scenario override form (`{"bus", "pull_up_ohms"}` and nothing else today);
     - `GeneratorReport`;
     - `NoProposal { reason }`.
   - Proposers run in a fixed order, and the first that answers wins. Today there is one: the rules proposer.
2. **The rules proposer: a closed table, each row citing its attribution rule.**
   - **`E1` with a rise past its ceiling** (kind 7 `exceeds`, the rise complete), whether caused by the pull-up or by the capacitance:
     - **Target:** `R_target = R_now × (ceiling × 0.8) / rise_worst`, where `rise_worst` is the bus's slowest measured line. The 0.8 is headroom for ±5 % resistor tolerance and an unmeasured capacitance spread. It is a named constant with that reason beside it.
     - **The value:** the largest E12 value at or below `R_target`.
     - **The floor:** that value must be at least `Rp_min = (VDD − 0.4 V) / 3 mA` (UM10204, standard and fast mode), with `VDD` the bus's `domain_millivolts`. If no E12 value lies in `[Rp_min, R_target]`, the result is `NoProposal`, never a value below the floor.
     - The proposal records `R_now`, the measured worst rise, the ceiling, the predicted rise, `Rp_min`, and the E12 value chosen.
   - **`E1` from a held-low line, a stuck-high line, or an incomplete rise:** `NoProposal("no rule; RUL-18 model proposer slot")`. A held line is a board fault no value change fixes.
   - **`S1` or `S2`:** a `GeneratorReport`, which names:
     - the run;
     - the attribution rule and its evidence indices;
     - for each piece of evidence, the solved expectation against the observed reading (the register or witness field, from 3100's join);
     - the generator the expectation belongs to (firmware or board crate).

     It contains no specification change.
   - **`I1` to `I5` and `N1`:** `NoProposal`, naming the attribution rule (`INV-05`).
   - **Never a lever:** `max_rise_time`, `max_capacitance`, any ceiling, any device, any address, any assignment. The override form allows only `pull_up_ohms`, and a test proves the proposer cannot name anything else.
3. **`aethelgard propose --archive <run dir> --scenario <file> --out <dir>`.**
   - **It checks first, refusing on any failure:**
     - It applies `attribute`'s checks, and with them `replay`'s.
     - The scenario file's sha256 and overrides must equal the run's kind 5.
     - Its `base` must resolve exactly as `campaign` resolves it, with the base's hash equal to the archived one (A11).
     - The solved topology's sha256 must equal kind 6.
   - **It reads** `R_now` and `VDD` from that solved topology's `analog_lines`, and attributes the run.
   - **It writes** to `--out`, which must be empty or absent:
     - `proposal.json`: the proposer's id, the attribution record (domain, rule, evidence), the changes, the numbers of §1.2, the run id and the state hash;
     - for a `Proposal`, also `proposed-spec.json`: the run's effective specification with the changes applied by the same code path as `campaign`'s overrides, then validated;
     - for a `GeneratorReport`, `generator-report.json` instead.
   - Nothing else is written, and no process is started.
   - Two calls on one run give byte-identical files.
   - Exit codes follow `RUL-10`'s table. A proposal, a generator report and a `NoProposal` are three distinct codes, chosen in FEEDBACK.
   - MCP exposure is generated from the tree. README section.
4. **The re-gate.** A tier-2 test takes `proposed-spec.json` through every gate a specification passes:
   - `validate` and `solve`;
   - emit, ERC and DRC (with schematic parity);
   - the board crate and `prove`;
   - a campaign of one scenario on that specification, with `expect: pass`, whose `attribute` is `N1`.

   For every `proposals/*/` committed in the tree, the same test also re-runs `propose` from the scenario named in its `proposal.json`, requires byte-identical output, and re-gates its `proposed-spec.json`. A committed proposal therefore cannot drift from the tool or slip past the gates.

## 2. Invariants and fences

- `INV-04`, `INV-05`, `INV-06`, `INV-07`, `RUL-07`, `RUL-16`, `RUL-18`. No `unsafe`.
- **Fence:**
  - `aethelgard-evolve`: the proposer interface and the rules proposer;
  - the CLI: `propose`, its MCP exposure and the README;
  - tests;
  - this work order's FEEDBACK.
- **Read-only:** `aethelgard-chaos`, `aethelgard-ir`, `campaign`, `replay`, `attribute`, `simulate --firmware`, the judge, and every generator.
  - The override path that `campaign` uses is called, not copied. If it is not callable as it stands, extract it as 3100 F7 extracted `checked`, with bytes unchanged.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance: P3 gate clause 2, as tier-2 tests

1. **The 47 kΩ run.**
   - `propose` on `pullup-47k`'s run gives a `Proposal` of exactly one change, `SENSORS` `pull_up_ohms` 2,700.
   - The worked numbers: `R_target` = 47,000 × 240,000 / 3,982,301 ≈ 2,832 Ω, so E12 gives 2.7 kΩ; `Rp_min` ≈ 967 Ω. Record the measured values in FEEDBACK.
   - The diff from the run's effective specification to `proposed-spec.json` is that one field.
2. **The re-gate of §1.4 passes** on that `proposed-spec.json`. Its measured rise is within 2 % of the predicted rise (about 229 ns).
3. **The branch.** The builder commits `propose`'s output for the 47 kΩ run on a branch `proposal/sentinel-01-pullup-47k`, branched from this work order's head:
   - the files go under `proposals/sentinel-01-pullup-47k/`: `proposal.json` and `proposed-spec.json`, written by the tool and never edited;
   - it is pushed by the F7 route, with a **draft** PR titled `PROPOSAL (do not merge): …`, so CI runs the re-gate (§1.4's tier-2 test) on exactly those bytes.

   The reference specification on `main` is not touched, because it was never at fault. The architect reviews the draft PR and closes it; it is never merged.
4. **The other outcomes,** on 3100's seeded runs:
   - every `S1` and `S2` case gives a `GeneratorReport` and no `proposed-spec.json`;
   - every held-low case gives `NoProposal` with the model-slot reason;
   - every pull-up and capacitance case gives a proposal whose re-gate passes;
   - an `indeterminate` run (tier 1, through the chaos writer) gives `NoProposal`.
5. **`propose` twice:** identical bytes. A scenario that is not the run's, or a base whose hash moved, is refused.

§1.4's tests pass in `gates` and `tools`.

## 5. Adversarial pass — full

1. **Make the loop move a gate:** have it propose a ceiling, a capacitance bound or any field other than `pull_up_ohms`, through a forged attribution, a re-sealed run or a crafted scenario.
2. **Make it propose below `Rp_min`,** or propose for a rise that never completed.
3. **Make it propose for a run it should refuse:** `indeterminate`, `none`, a software attribution, or a scenario that is not the run's.
4. **Make two calls differ,** or make `propose` write outside `--out` or start a process.
5. **Make a proposal pass its re-gate for the wrong reason:** for example, one that drops the device so the bus has nothing to time out on.

## 7. Flag-and-stop conditions

- A fact the proposer needs is not in the archive or the solved topology the CLI can verify against it.
- Applying the change cannot go through `campaign`'s override path without changing that path's behaviour.
- The re-gate of the 2.7 kΩ proposal fails, or its rise is more than 2 % from prediction. Report the numbers; do not tune the 0.8.
- A test that can only go green by being weakened.

## 8. Amendment before kickoff (2026-11-12, from the Mythos 5.2 review)

- **The override form (review A8.11).**
  - Three override forms exist: `pull_up_ohms`, `max_capacitance_femtofarads` and `line_held_low`.
  - §1.1's "and nothing else today" means the proposer may emit only `{"bus", "pull_up_ohms"}`. The other two forms describe a fault, never a fix.
  - A test proves the proposer cannot emit either of them.
- **The join (review A8.12).**
  - `Attribution` carries only `EventRef`s, and 3100's bus join is private to `evolve`.
  - Inside this work order's fence, `evolve` exposes a read-only view: for an evidence event, its bus, and for a witness or register event, the solved expectation and the observed reading. The generator report reads from it.
  - `attribute`'s output does not change. Measure it byte for byte.
- **The floor is the gate's (review A2, B2).**
  - `Rp_min` is `aethelgard_ir::electrical::i2c_pull_up_floor_ohms(speed_hz, domain_millivolts)` from `AET-WO-1203`, called by the CLI and handed to the proposer as an integer. `evolve` keeps no copy and gains no `aethelgard-ir` dependency.
  - If `AET-WO-1203` has not merged when you need it, build everything else first, then merge `main` when it lands.
  - The re-gate's `solve` then refuses any proposal below the floor, so the floor is enforced twice from one definition.
- **The re-gate and generation (review A3).** `aethelgard generate` does not exist yet (`AET-WO-3301`). The re-gate of §1.4 generates the board crate and firmware the way the repository's tests do today. Record that in FEEDBACK as a residual that `3301` removes.
- **Wording (`RUL-19`).**
  - `proposal.json` and FEEDBACK say the rise is "computed from declared values". `predicted_rise_ps` is the field name.
  - A proposal is evidence about the specification, not the board.
