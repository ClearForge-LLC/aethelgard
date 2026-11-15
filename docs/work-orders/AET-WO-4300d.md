# AET-WO-4300d — Power parts: from battery to the MCU's supply pins, checked statically

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300d-power`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build.
**Phase:** P4, the fourth piece of `AET-WO-4300`'s split (PR #123). **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**
- The spike runs now and writes nothing outside `spike/4300d/`.
- The build waits for `4300f` (connectors: power comes in through one) and `1308` (per-net widths).

**Grounds, on `main`:**
- **The `4300` spike's S4** (`AET-WO-4300.FEEDBACK.md`, read):
  - Buck, LDO and sense parts become parts-table rows.
  - The checks are static: the declared battery against the buck's VIN range and UVLO, dropout, VOUT against the domain, and the sense full scale against the ADC pin's range.
  - **ADC battery monitoring is out,** because the twin's ADC value is the judge's own number.
  - The battery-sag chain into `--rail` is a later work order.
- **The `1303` ruling** (PR #133, D4, D7): the supply-path resistance bound needs a regulator **output pad** as its source. There is no supply gate until a board has one. The drop is checked on VDD and VSS together.
- **The `4210` spike** (PR #131, D7): the ICM-20948's VDDIO is 1.71–1.95 V. A 1.8 V I/O domain, or a level translator, is a declared part or a refusal.
- **DS12117 Rev 10** (held): Table 120's 658 mA maximum at Tj 125 °C is the MCU's declared load (`1303` D4). The supply range is 1.62–3.6 V, and the BOR levels are in the datasheet.
- **Furnace cannot reach st.com.** TI and Analog Devices hosts were not tried. Fetch every datasheet and hold it by sha256; list any refusal for Scotty, who fetches.

> **What this is:** the power tree of a flight controller as declared parts: battery in, then a buck, then an LDO or a second rail, then each voltage domain. Every part's datasheet limits are checked statically against the declared battery and loads, and the regulator's output pad becomes the source that `1303`'s supply bound measures from.
>
> It is NOT:
> - ADC battery monitoring in the twin;
> - the battery-sag transient (a later work order feeding `--rail`);
> - an ESC or power-distribution board;
> - the buck's switching-loop layout beyond the datasheet's stated layout rules (named, checked where the gates can).

## 1. Spike

Probes under `spike/4300d/`. **Predictions go into FEEDBACK before any run.**

**S1. The parts, by evidence.** For each candidate, record:
- its datasheet, held by sha256;
- whether it is in production (the vendor page);
- whether a KiCad 10.0.7 symbol and footprint ship in the tools image, with their licence (`RUL-11`);
- the fields a static check needs.

Recommend, with a reason:
- **one buck** for a 2S–6S battery to 5 V, or to 3.3 V directly: VIN, VOUT, IOUT, UVLO, switching frequency, and the required external parts (inductor, capacitors with their stated ranges);
- **one 3.3 V LDO or second buck,** with dropout at load, its stability's output-capacitor range, and PSRR if stated;
- **the 1.8 V question:** an LDO for a 1.8 V I/O domain plus a translator, or a refusal. `4210` D7 depends on it;
- **one battery voltage-sense divider** (resistor tolerance as declared parts) and, optionally, **one current-sense part.** The sense is checked statically only.

**S2. What the specification gains.** Propose `power`:
- the battery (cells, chemistry, per-cell minimum and maximum, with a source for each);
- each regulator part, its input net, its output domain;
- each domain's declared load, with its source (the MCU's from DS12117 Table 120).

Say how the solver turns this into a tree, and what it refuses: a domain with no source, a loop, or a load above a regulator's IOUT.

**S3. The static checks.** For each, say the data it reads and the refusal it gives:
- the battery's range against VIN and UVLO;
- the dropout at the declared load;
- VOUT's tolerance against the domain's declared range and the MCU's 1.62–3.6 V;
- the BOR margin;
- the sense full scale against the ADC pin's range;
- every required external part present, with its value inside the datasheet's stated range.

**S4. The board.** Read what the emitter, placer and router would need:
- the regulators' symbols and footprints;
- the buck's external parts;
- the datasheet's layout rules: the switching loop, the feedback trace and thermal pads.

Say which layout rules a gate can check (for example the input-capacitor distance) and which stay named conditions. Measure nothing on the router here: it is `1307`'s now and `1308`'s next.

**S5. The output pad.** Confirm how `1303`'s supply bound would find the regulator's output pad from the topology, and name what it needs.

**Stop** with:
- the table;
- the recommended parts;
- the specification's shape;
- the checks;
- the decisions needed.

## 2. Build (after the ruling; expected, and the ruling confirms)

1. **The ruled parts in `parts.json`,** cited, with symbols and footprints vendored where the licence allows.
2. **`power` in the specification and the topology,** and the static checks at `validate` and `solve`, each refused by name with its citation.
3. **The emitter places and routes the power tree** with `1308`'s classes. Every gate passes on a test fixture.
4. **Sentinel-02's power tree is ready** for `4500`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-11`, `RUL-14`, `RUL-16`, `RUL-19`.
- **The spike fence:** `spike/4300d/` and FEEDBACK only. Datasheets go in `~/.cache/aet-4300d/ds/`.
- **Other builders on Furnace:** read `~/.cache/aet-bus/WHO.md`. Never touch their worktrees. One container of yours at a time.
- One PR, left unmerged. Write `~/.cache/aet-4300d/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- **No candidate buck has a fetchable datasheet** and a KiCad footprint under a vendorable licence. Stop with the URLs, for Scotty.
- **The 1.8 V question has no part that fits** without a level translator the board cannot place. That is a ruling.
