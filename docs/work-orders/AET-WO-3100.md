# AET-WO-3100 — Attribution: electrical or software, from the archive alone, with the evidence named (`INV-05`)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3100-attribution`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a **full** adversarial pass (`docs/roadmap.md` §6). The roadmap names attribution as the place where a bug produces a *confident wrong answer* (§3).
**Phase:** P3. It owns the P3 exit gate's first clause: "On a seeded set of at least twenty failing traces with known cause, attribution names the correct half for every one, and names the evidence span in each record (`INV-05`)."

**Grounds:**
- **`AET-WO-2300`, merged (PR #74).** Every campaign run is an archive run directory of integer-only CBOR events, and `replay` re-derives its state-sequence hash with no solver, no Renode and no bridge. The kinds this work order reads are in `crates/aethelgard-chaos/src/record.rs`:
  - kind 7, `RiseMeasured` (with `ceiling_ps`, `complete`, `exceeds`);
  - kind 8, `LineStuckHigh`;
  - kind 9, `BusFault` (with its cause, such as `scenario.line_held_low`);
  - kind 10, `BootObserved`;
  - kind 11, `RegisterObserved`;
  - kind 12, `I2cCheck`, whose `address`, `register` and `expected` come from the solved topology and the part table, never from the firmware (`INV-07`), and whose `status` is what the firmware wrote;
  - kind 13, `I2cWitness`, the controller's own `CR2` and `TXDR` after the run;
  - kind 14, `Verdict`.
- **`architecture.md` §2.16:** Renode's I2C controller never signals NACK. An absent address leaves TXIS unset, so the firmware times out. **A wrong address in the firmware and a 47 kΩ pull-up therefore produce the same symptom, `i2c_timeout` at `timeout_txis`.** Telling them apart is the whole job.
- **`crates/aethelgard-evolve`** exists as a stub: "Attribution from traces and mutation proposals; never merges anything or bypasses a gate."
- **`INV-05`** (attribution precedes mutation), **`INV-04`** (replay from the trace), **`INV-06`** (fail closed), **`INV-07`** (expectations independent of the artifact).

> **What this is:**
> - `aethelgard_evolve::attribute`, a pure function from one archived run to one attribution record;
> - `aethelgard attribute --archive <run dir>`, which prints that record;
> - a seeded set of at least twenty failing runs with known cause, and a tier-2 test that attribution names every one correctly.
>
> It is NOT:
> - a mutation or a proposal (`AET-WO-3200`);
> - a change to `campaign`, `replay`, `simulate --firmware`, the judge, or any verdict;
> - a change to the firmware generator. Seeded software faults are test fixtures (§1.4).

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **The attribution record.** `attribute(run) -> Attribution`, deterministic, with:
   - `domain`: `electrical`, `software`, `none` (the run passed), or `indeterminate`;
   - `rule`: the id of the one rule in §1.2 that decided it;
   - `evidence`: the archive event indices, with their kinds, that prove the cause;
   - `symptoms`: the event indices of the failures the cause explains;
   - `masked`: software evidence on a bus whose electrical fault makes it unreliable (§1.2 E-rules), listed and never used;
   - `run_id` and the state-sequence hash, as `replay` computes them.

   Every failing finding in the run's verdict is either a symptom the record explains or a reason for `indeterminate`. Nothing is left unexplained in silence.
2. **The rules: closed, ordered, and each requiring positive evidence.**
   - **Electrical (E):** a bus with a `RiseMeasured` that `exceeds`, a `LineStuckHigh`, or a `BusFault` is electrically faulted. A symptom on that bus is attributed `electrical`, with those events as evidence. Software evidence on the same bus goes to `masked`, because the physical layer comes first: a firmware's view of a broken bus is not evidence about the firmware.
   - **Software (S):** a symptom is attributed `software` only when **both** of these hold:
     - its bus has a `RiseMeasured` for every line, each `complete` and within its ceiling, and no `BusFault` or `LineStuckHigh` (the electrical layer is *measured* healthy);
     - there is positive software evidence, such as:
       - the witness's `CR2.SADD` or `NBYTES`, or `TXDR`'s register byte, differs from the `I2cCheck`'s solved address or register;
       - a `RegisterObserved` mismatch on that peripheral's clock gate, GPIO mode or alternate function;
       - a boot failure (`BootObserved` with no boot word, or `pc` outside the firmware).

     `CR2`'s field positions come from the same source the judge uses (the ST header through `aethelgard-svd-hal`), never a literal.
   - **`indeterminate`** is everything else. In particular:
     - no rise was measured for a line on the symptom's bus;
     - the only fault is on a different bus from the symptom;
     - the electrical layer is healthy, but no software evidence exists.

     **Software is never concluded by eliminating electrical, nor electrical by eliminating software.**
   - **Not echoing the judge.** The verdict's finding kinds select which symptoms need explaining. They are never themselves evidence.
3. **`aethelgard attribute --archive <run dir>`.**
   - It first applies exactly `replay`'s checks, and refuses whatever `replay` refuses, with the same error and exit code.
   - It reads nothing outside the run directory and starts no process. MCP exposure is generated from the tree, as always. README section.
   - It prints one JSON line, the record of §1.1.
   - **Exit codes:** 0 when the domain is `electrical`, `software` or `none`; 1 when `indeterminate`.
   - Two calls on one run give byte-identical output.
4. **The seeded set.**
   - **Electrical cases are scenarios,** under `scenarios/sentinel-01/seeded/`, in 2300's format, unchanged. Each is run against the unmutated generated firmware. At least:
     - pull-ups of 3.9 kΩ (just over the 300 ns ceiling: about 330 ns at the reference's 100 pF), 4.7 kΩ, 10 kΩ, 47 kΩ and 100 kΩ;
     - capacitance raised past the ceiling with 2.2 kΩ, twice;
     - `line_held_low` on `sda` and on `scl`.
   - **Software cases are firmware mutants** of the generated firmware, built by the test in its scratch directory. Each is a named, exact-once text patch, which refuses unless it applies exactly once, so a generator change fails the test loudly and never silently un-seeds a case. They run with the `pullup-2k2` scenario. At least:
     - a wrong device address (`0x77` for `0x76`);
     - a wrong identity register;
     - a wrong `NBYTES`;
     - the I2C clock enable skipped;
     - a wrong alternate function on SCL;
     - no boot word.
   - **The cases that test the trap:**
     - the wrong-address mutant with a 3.3 kΩ pull-up (about 280 ns, inside the ceiling) must be `software`;
     - the wrong-address mutant with 47 kΩ must be `electrical`, with the software evidence in `masked`;
     - 3.3 kΩ with the unmutated firmware must be `none`.
   - **The manifest:** `scenarios/sentinel-01/seeded/cases.json` lists each case: scenario, mutant or none, true domain, and expected rule. The scenario format does not change.
   - **The count:** at least twenty failing cases, at least eight in each domain.
   - **The measured rise** of every pull-up and capacitance case is recorded in FEEDBACK, not assumed from the numbers above.

## 2. Invariants and fences

- `INV-04`, `INV-05`, `INV-06`, `INV-07`, `RUL-06`, `RUL-13`, `RUL-16`. No `unsafe`.
- **Fence:**
  - `aethelgard-evolve`: attribution;
  - the CLI: `attribute`, its MCP exposure, and the README;
  - `scenarios/sentinel-01/seeded/`;
  - tests;
  - this work order's FEEDBACK.
- **Read-only:** `aethelgard-chaos`, `campaign`, `replay`, `simulate --firmware`, the judge, and the firmware generator.
  - If attribution needs a fact the archive does not carry, that is a §7 stop. Do not add a record kind on your own.
  - `evolve` depends on `aethelgard-chaos` and `aethelgard-svd-hal` as it needs. It never names `Specification` (`RUL-16`).
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance: P3 gate clause 1, as tier-2 tests

1. **Every seeded failing case:**
   - the campaign runs it and its own `expect` matches;
   - `attribute` on its run names the manifest's domain and rule;
   - `attribute` names at least one evidence event, every index in range and of a kind its rule allows.
2. **The three trap cases** of §1.4 are attributed as stated.
3. **`attribute` twice on each run:** identical bytes. A one-byte change to the run is refused as `replay` refuses it.
4. **Tier 1, no Renode,** on archives written through the `aethelgard-chaos` writer:
   - each `indeterminate` path of §1.2;
   - a healthy bus with no software evidence;
   - a fault on another bus.

§1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — full

1. **Make attribution name the wrong half with confidence.**
   - Drop, reorder or duplicate the analog events of a well-formed run, re-sealed with the chaos writer.
   - Fault one bus and fail another.
   - Make the witness agree with a wrong address.
2. **Make it echo the judge:** change only the verdict's finding kinds in a re-sealed run, and see whether the domain follows.
3. **Make it conclude by elimination:** remove all evidence of one half, and see whether it names the other.
4. **Make it reach outside the run directory,** or start a process.
5. **Make a mutant un-seed silently:** change the generator's output so a patch no longer applies, and see whether a case still "passes".

## 7. Flag-and-stop conditions

- Attribution needs a fact the archive does not carry, so a record kind or a campaign change would be needed. Report what and why; do not add it.
- A seeded case cannot be made to fail the way its manifest says without changing the firmware generator or the judge.
- A test that can only go green by being weakened.

## 8. Amendment — ruling on S1 (2026-11-09)

The first build stopped at §7 before writing code, correctly. The archive cannot join a symptom (kinds 12 and 13 name a device and a controller) to a bus (kinds 7 to 9 name a bus). The ruling is the builder's option (a), as one new record kind inside this work order. The full text is in this work order's FEEDBACK.

- **Kind 15, `I2cBusSolved`,** written by `campaign` once per solved I2C bus, directly after kind 6.
  - It carries `bus`, `peripheral` and `ceiling_ps`.
  - `lines`: each line's signal, net, pin and alternate function.
  - `devices`: each device's reference and address.

  All of it comes from the `SolvedTopology` (`INV-07`, `RUL-16`).
- **Run rules** in `check_run`, so `replay` and `attribute` both enforce them:
  - exactly one kind 15 per bus;
  - every `I2cCheck` and `I2cWitness` resolves to exactly one bus, at its address and controller;
  - every analog net is a line of its bus.

  A dropped or forged join is refused, never attributed. No rule may infer the bus from there being only one.
- **The trace header goes from version 2 to 3,** and a version-2 trace is refused. The goldens are re-pinned, with the old and new values in FEEDBACK.
- **§2's fence widens to exactly this:**
  - `aethelgard-chaos`: kind 15, its rules and the version;
  - `campaign`: writing kind 15.

  `replay`'s logic, `simulate --firmware` (byte-identical, measured), the judge and the firmware generator stay read-only.
- **§3.4's "a fault on another bus"** is a two-bus tier-1 archive. It is `indeterminate` *because the fault is elsewhere*.
- **`OQ-03` is re-measured** with kind 15.
