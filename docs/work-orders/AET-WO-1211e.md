# AET-WO-1211e — Pin devices validated against the MCU and the part: a pull no MCU pin tolerates, a VREF that chops at nothing, and a declared bulk capacitor

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211e-pin-device-validate`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before the baseline. Written 2026-11-20. **Cadence:** a measurement, then a build with stops.

**Grounds** (`1211d` S1 and its adversarial pass, 2026-11-20 18:57Z, ruled 19:06Z):
- **A1, MCU damage.** D2 with a DRV8833's nFAULT pulled by a declared 10 kΩ to `VMOTOR` validates, solves, emits and passes all four eda gates.
  - It puts 6.3 V on PA1, an FT pin. DS12117 rev 10's absolute maximum for FT inputs is min(VDD + 4.0 V, 6 V).
  - The DRV8876 twin is refused only because its row carries `pull_up_max_mv`. Nothing checks the MCU side of a pull.
- **A2, would not run.** A DRV8876 with VREF tied to `GND`, or to a domain too low for its declared full scale, validates and draws.
  - SLVSDS7B p. 13, Eq. 3: ITRIP = V_VREF / (A_IPROPI · R_IPROPI).
  - VREF at GND chops at zero current. 0.5 V gives about 0.33 A against D1's declared 1.9 A full scale.
- **S1, `bulk`.** The DRV8876's VM bulk capacitor (CVM2) has no stated value: SLVSDS7B §9.1 leaves it to system testing. The arm refuses `PassiveValueUnstated`, so D1 cannot be drawn.
  - Roadmap §7.1, the `1211d` S1 row, names the field: `pin_devices[].bulk: {reference, part, source}`, a cited capacitor row rated above the VM domain's maximum.
  - The DRV8833's single 10 µF is its datasheet's stated VM capacitor (the 19:06Z ruling on Q4); it needs no `bulk`.
- Read `1211d`'s evidence under `~/.cache/aet-1211d/adv/` (on Furnace) and its FEEDBACK rows, once it merges, before you write a test.

> **What this is:** three rules in `aethelgard-ir`, each refused by name at `validate`, and one specification field.
> - **A1:** every pull a specification declares on a net that reaches an MCU pin is judged against that pin's tolerated input.
>   - The tolerance is the pin's I/O structure (FT, TT, …) from DS12117's pin table, and that structure's absolute maximum, with VDD taken at the MCU domain's declared maximum and the 6 V cap applied.
>   - Each structure and limit is cited by table and page. A pin whose structure is not held is refused by name, never assumed.
> - **A2:** a part whose row states a reference-sets-trip rule (the DRV8876's VREF) is refused when the reference is on `GND`, and when VREF_min / (A_IPROPI · R_IPROPI,max) is below the declared full scale at its band's worst edge.
> - **`bulk`:** `pin_devices[].bulk: {reference, part, source}`, where `part` is a capacitor row rated above the VM domain's maximum.
>   - It is required where the part row says its VM bulk is unstated (the DRV8876). Elsewhere it is refused, unless the row allows one.
>   - `source` is the designer's statement of why that value. No default exists.
>   - The D1 test fixture declares one, its source saying plainly that it is a fixture's stand-in and not a design.
>
> It is NOT:
> - eda. The arm that draws `bulk` (lifting `PassiveValueUnstated` for D1) is a follow-up after `1211d` merges. Name it in FEEDBACK.
> - the generator or the twin;
> - a value for CVM2 from any source but the specification's own declaration.

## 1. Build

1. **Measure first,** with predictions committed before the runs, in `spike/1211e/`:
   - Every committed specification and test fixture: each declared pull on an MCU net, its domain maximum, the pin's structure and its tolerated input.
   - **Prediction:** none exceeds.
   - Every reference-sets-trip part and its VREF.
   - **Stop and report** if a committed specification would be refused by any rule.
2. **The data:** the MCU pin structures and their limits, held where the MCU's pin data lives today, each row cited. Say where that is, and why there.
3. **The rules** in `validate`, each with its kind name in the IR README. Name the A1 kind (for example `PullAboveMcuTolerance`) and the A2 kinds in FEEDBACK.
4. **`bulk`** in the schema, `validate` and the IR README. Specification version: the next one free when you merge (24 if `4240b` merged at 23), by `spike/4240a/kit/renumber.py`, under the first-to-merge rule.
5. **Tests:**
   - `1211d`'s A1 board (nFAULT → R → `VMOTOR`) refused by name;
   - the same pull to `3V3` passes;
   - a TT pin pulled to 5 V refused;
   - A2's GND and 0.5 V cases refused;
   - a VREF meeting the full scale passes;
   - D1 without `bulk` refused, and with it passing;
   - `bulk` on a DRV8833 refused;
   - a `bulk` part rated at or below the domain's maximum refused.
   - **Mutants:** one per rule and per guard. A test kills each.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No committed specification is refused, or the stop was ruled.
- **Every committed specification's emitted bytes are unchanged** except version and sha lines, by a null sweep as `1211a` ran it.
- **The CLAUDE.md gates pass, and CI is green.** If the specification bump moves the bench package's sha lines, queue for a dispatch.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (rules, data, schema, README) and its tests;
  - the specification version literal and the files the kit renumbers;
  - the sha-bearing goldens, by their generators;
  - `spike/1211e/` and FEEDBACK.
- **Not:**
  - `aethelgard-eda` (the `bulk` arm waits for `1211d`);
  - the generator, the twin, or chaos;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `4240b` (IR clock, specification 23, in the package queue);
  - `1211b` (generator, solved 16);
  - `1211d` (eda, still drawing D2);
  - `1913` (CI weights);
  - `2307` (CLI ledger refusal).
- Merge `main` before READY. One PR, left unmerged. Write `~/.cache/aet-1211e/pr-title-body.md`.

## 5. Adversarial pass, full

- **A pull that reaches an MCU pin by a path the rule does not walk:** through a connector, a second device, a strap, or `power.sense`.
- **A VDD taken at its minimum** where its maximum belongs, or the reverse.
- **An unpowered MCU with a powered motor domain.** Report what DS12117 states for that case. Do not invent a limit.
- **A `bulk` whose rating or part row passes only by unit or rounding.**

## 7. Flag-and-stop conditions

- **A committed specification would be refused.** Stop and report it.
- **An MCU pin's structure or limit has no held source.** Stop and report it; do not assume FT.
- **DS12117 states a separate unpowered-pin limit that a declared power sequence reaches.** Stop and report it, with the page.
