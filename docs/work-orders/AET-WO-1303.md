# AET-WO-1303 — Extraction, full: every bound a P4 gate reads, from the routed board's own copper

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1303-extraction`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build.
**Phase:** reserved since P1 (split from `1302`). Due before `4500` (roadmap §7.1). **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).** It runs after `1307` merges, because `1307` changes routing.

**Grounds, each read on `main`:**
- **Extraction today bounds one quantity on one kind of net:** each I2C line's capacitance, from above (`AET-WO-1305`). `4100` extended it over a near plane at 4 layers. It reads copper through `route::read` and `connectivity`, never through labels (`RUL-03`).
- **P4's gates now need more than that:**
  - **`4300c`:** an Fmax check per signal line, with C rounded up to the table's row. If `4300c` widened the line list, take its result as given.
  - **`4300e1`:** a 90 Ω differential-pair check for USB D+/D−. That needs a bound on impedance, or on its deviation, from the stackup and the routed geometry.
  - **`4300d` and the droop (`2301`):** the regulator-to-MCU supply path. A bound on the copper's resistance turns a declared load into a voltage drop the rail model can use.
- **The bounds must keep `1305`'s discipline:**
  - a rigorous bound with stated conditions, or a refusal by name;
  - every constant bounded from the safe side;
  - a test against an exact case.

> **What this is:** the quantities P4's gates read from a routed board, each bounded rigorously from that board's own copper, with the gates wired to read them.
>
> It is NOT:
> - a field solver;
> - crosstalk or EMI;
> - 6 layers (backlog);
> - signal integrity beyond the bounds named.

## 1. Spike

Probes under `spike/1303/`. **Predictions go into FEEDBACK before any run.**

**S1. The consumers, exactly.** For each gate (`4300c`'s Fmax, `4300e1`'s USB pair, `4300d`'s supply drop, and any other the spike finds), state:
- the quantity and its direction (upper bound, lower bound, or interval);
- the precision the gate needs to be useful on Sentinel-02;
- what the gate does today without it.

**S2. Capacitance on every signal line.** Say whether the I2C bound's machinery bounds any line: SCK, MOSI, MISO, each chip select, UART lines, timer outputs. Measure on the reference and the 4-layer fixture, and name any piece shape it cannot bound. Skip this if `4300c` already did it, and cite it.

**S3. Differential impedance.** Propose a bound for an edge-coupled microstrip pair over a plane, at the declared stackup's dielectric minimum and maximum and its permittivity bounds:
- **The approach:** an interval from closed forms with stated validity ranges (for example Wadell or IPC-2141 forms), or a certified bound. Say which, and why it is honest.
- **The check:** against an exact or reference case, such as a published field-solver table, if one can be cited.
- **Two-layer boards:** if no rigorous bound exists at 2 layers (no reference plane under the pair), say so; the gate refuses USB there.

**S4. Supply-path resistance.** Propose an upper bound on the copper resistance from a regulator's output pad to each MCU supply pin:
- along tracks and through vias, and through a plane if the domain has one;
- with copper resistivity at its maximum temperature and the copper maximum;
- with the via barrel's plating at its minimum.

Name every condition: the plating thickness is a fabricator figure, so cite it or declare it as a condition. Measure on the reference and the 4-layer fixture.

**S5. Where the bounds go.** Propose:
- the `extract` verdict's shape for multiple quantities;
- how `verify` gates each one against its declared limit;
- where the limits are declared: `net_constraints`, or new fields.

`INV-06` applies: no default limit.

**Stop** with the table, a recommendation per question, and the decisions needed.

## 2. Build (after the ruling)

1. Each ruled bound, with its derivation in the module documentation and a test against an exact case.
2. The gates read the bounds.
3. Sentinel-01's existing bounds stay byte-identical, unless the ruling names a change.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-12`, `RUL-14`.
- **Fence:**
  - `aethelgard-eda` (`extract.rs`, and `stack.rs` for its inputs);
  - the CLI's `extract` and `verify`;
  - the specification's new limit fields, if ruled;
  - tests and fixtures;
  - FEEDBACK.
- At the spike: `spike/1303/` and FEEDBACK only.
- One PR, left unmerged. Write `~/.cache/aet-1303/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- A quantity a P4 gate needs has no honest bound from the copper the reader can read. That is a ruling: declare it as a condition, or drop the gate.
