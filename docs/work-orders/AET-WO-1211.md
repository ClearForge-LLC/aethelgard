# AET-WO-1211 — A driver IC as a specification: a device with no bus, GPIO and ADC signals, and a PWM output's declared idle, bridge pair and resolution (design spike, stop for the schema ruling)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211-driver-ic-spike`.
**Author:** Claude (architect) · **Builder:** strong tier. No adversarial pass: nothing is built.
**Phase:** P4, before the baseline (`4600` D7). Written 2026-11-20. **Cadence:** a design spike that stops for a ruling. The builds follow it, in work orders written from the ruling.

**Grounds** (`4600` F2, F6, F10 and D7, ruled on #245; roadmap §7.1):
- **F6:** the specification cannot declare a device without a bus, a plain GPIO signal, or an ADC input.
  - So a driver IC cannot be declared: its sleep and fault pins and its current sense have no field (`ir/src/lib.rs` about L102–152, L337–388).
  - `4600` §2 lists the real parts that need it, each with its datasheet:
    - DRV8833 (AIN/BIN, nSLEEP, nFAULT open drain, xISEN shunts);
    - DRV8876 (EN/IN1, PH/IN2, nSLEEP, nFAULT, IPROPI at 1000 µA/A);
    - DRV8871 (IN1/IN2/ILIM only);
    - INA180A1 (a current-sense amplifier).
- **F2:** every PWM output idles at its `pulse_min_us`, and `validate` refuses 0 (`ir/src/motor.rs` about L66–70, L259–262).
  - An H-bridge cannot coast.
  - Two IN/IN inputs on one timer brake-chop at 2 % from boot.
  - A PH/EN bridge drives at 2 %.
  - A servo idles at its end stop.
  - Every gate passes it.
- **F10:** PWM ticks in whole microseconds at any rate, so 20 kHz has 50 steps.
- **D7:** F2 and F6 change what a motor output and a device are. Later additive fields cannot undo that cleanly, so they land before the P4 baseline.

> **What this is:** a proposal, measured against today's engine, for:
> - **(a)** a device kind with no bus, bound to a part row;
> - **(b)** declared GPIO signals, each an output or an input. An output has a declared initial level. An input has a pull and an active level. Each binds to a device pin;
> - **(c)** declared ADC inputs: channel, divider or transfer, range, and the device pin it reads;
> - **(d)** per PWM output: a declared `idle` (low, high, or a pulse in µs, 0 allowed) separate from its range, and a declared `resolution`, from which `PSC` is derived;
> - **(e)** a declared bridge pair, never driven high together. The rule is the firmware's, Kani's and the twin's.
>
> For each, the spike states what every projection would do:
> - `validate` and its refusals;
> - the solver (pins, `PSC`/`ARR`, ADC channels);
> - the generator (init order, idle before `CEN`, the pair's interlock);
> - Kani's oracle;
> - the twin (what Renode models, read at source; what it cannot witness);
> - the ledger;
> - the eda emitter (an arm per part, or `4600` D4's per-part data);
> - `verify` (what each new gate judges, and what it judges nothing on, by `2306`'s rule).
>
> It is NOT a build. No crate, specification or workflow changes. `spike/1211/` and FEEDBACK only.

## 1. The spike

1. **Read first,** with every citation held by sha256 in `~/.cache/aet-1211/ds/`:
   - DRV8876 SLVSDS7B and DRV8833 SLVSAR1E: pins, logic levels, nSLEEP timing, nFAULT open-drain and its pull-up, IPROPI's transfer and range;
   - RM0433: the GPIO output modes, ADC1–3's channels on H753's pins, and the ADC's input range and sampling time;
   - Renode `cd4b002a`: what `STM32_GPIOPort` and the H7's ADC model do (read at source; quote the lines).
2. **Two fixtures, written as they would be declared** (in `spike/1211/`, never committed as specifications):
   - **D1, a DRV8876 driver:** EN/IN1 and PH/IN2 on one timer (PH/EN mode), nSLEEP, nFAULT, and IPROPI on an ADC channel through its resistor.
   - **D2, a DRV8833 driver:** two IN/IN bridges on one timer each, nSLEEP, nFAULT, and no current sense.
   - Each with a servo output, so `idle` is shown on a non-bridge output too.
3. **The schema proposal.** One JSON shape for (a)–(e), with every field's meaning, unit, refusal and citation. Give two shapes where a real choice exists, with a lean. In particular:
   - **Are GPIO and ADC signals fields of the device, or of the MCU?** The device owns its pins. The MCU owns its peripheral.
   - **Is a bridge pair a property of two outputs, or a device kind** (`bridge`) that owns two outputs?
   - **Is `idle` required on every PWM output?** No default substitutes (`INV-06`). That forces every committed specification to state its outputs' idle: count them, and show the migration.
   - **Does `resolution` replace whole-µs ticks** for every PWM output, and which solved words move?
4. **Measure what moves.** For every committed specification under the proposed schema (a script run on today's files, never committed to them):
   - which fields each file must gain;
   - which solved words would move: `PSC`, `ARR` and `CCR` under (d);
   - which emitted bytes would move.
   - **Prediction:** with `idle` = `pulse_min_us` and `resolution` = 1 µs stated explicitly, no solved word and no byte moves. Show it, or name what moves.
5. **The twin.**
   - **GPIO:** can the twin witness a declared initial level before `CEN`, and an input's pull? Can a scenario drive nFAULT low? Read at source, then measure with one hand-written Renode script on the stock platform, in the pinned image.
   - **ADC:** does Renode's H7 ADC return a value a script can set? If not, say what can never be witnessed, and propose the rule-only judgement.
6. **The split.** Propose the build work orders that follow, each sized for one builder, in order, with its fence and the specification version it takes. `1210` holds 21, and `4240b` may take the next. **Lean:**
   - one IR/schema work order;
   - one generator and Kani work order;
   - one twin work order;
   - an eda arm per part.

## 2. Acceptance

- **The proposal** is in FEEDBACK §2: the schema, a lean per choice, and the refusals.
- **The two fixtures** are written to it, with every number cited.
- **The measurement of what moves** is in §3, with the predictions beside it.
- **The twin's reach** is in §4, read at source and measured.
- **The proposed split** is in §5.
- **The CLAUDE.md gates pass** on the branch: nothing outside `spike/` and FEEDBACK changes.

## 3. Invariants and fences

- **Invariants:** `INV-01` (the shared types stay in `aethelgard-ir` when built), `INV-06` (no default for `idle`), `INV-07`.
- **Fence:** `spike/1211/` and `docs/work-orders/AET-WO-1211.FEEDBACK.md`. No crate, specification, `boards/`, `fab/`, workflow or `ci/` change.
- **Running beside you:** every running builder. This spike reads `main`. Merge nothing.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged, as the record. Write `~/.cache/aet-1211/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **This work order always stops at §2** for the architect's schema ruling. Ask on the bus.
- **A cited number cannot be read from a held maker's document.** Name it. Do not use a distributor's copy as the citation.
