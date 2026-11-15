# AET-WO-4300d3 — The power tree's remaining judgements: hot-plug, the buck's junction, DC bias and the rising brown-out threshold

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300d3-judgements`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Slice C of `AET-WO-4300d`, from the second Mythos 5.2 review (`docs/reviews/2026-11-15-mythos-5-2.md`, A4; response `…/2026-11-15-mythos-5-2.response.md`). It runs after `AET-WO-4300d2` merges (both change `aethelgard-ir`'s power checks). **Cadence: build, with one measured stop (§1.0).**

**Grounds, on `main` (each checked at source by the architect, 2026-11-15):**
- **Hot-plug is a named condition, not a check.** `AET-WO-4300d.FEEDBACK.md:237`: "hot-plugging a pack through long leads rings the input above its DC value; TI (p23, p32) asks for 20–100 µF bulk capacitance in parallel then, and the 38 V absolute maximum is the ceiling".
  - Nothing in `ir/src/power.rs` checks it.
  - The power fixture gives each buck 10 µF plus 220 nF of ceramic and no bulk or clamp.
  - A drone pack is plugged in on every flight. 6S HV is 26.1 V.
- **The buck has no junction check.** `power.rs:1386-1394` compares the board's ambient range with the part row's range. For the LMR33630 that is its junction range, so 85 °C ≤ 125 °C passes with no dissipation term.
  - The LDO has a junction check.
  - `4300d` D1 named the buck's case a condition because TI says RθJA "cannot be used for design", and nothing states that condition today.
- **DC bias is not judged** (`4300d` D6, `power.rs:1639-1643`).
- **The brown-out floor uses only the falling maximum** (`power.rs:843-858`). A rail must also clear the rising maximum to leave reset.
- **The ruled pattern for typical-only figures** (the `4300d` build stop, comment 6893999770): a typical figure is a screen with a named margin, never a guarantee. That is how Isat and CADC are judged.

> **What this is:** the power tree's static checks gain the four judgements a flight controller's supply needs and the review found missing. Each is a check with a citation, or a screen with a named margin where only typical figures exist. None is left as prose.
>
> It is NOT:
> - the battery-sag transient (the later analog work order);
> - a ring simulation of the leads (the ring's size is named, not computed);
> - the MCU's support network (`AET-WO-4300g`).

## 1. Build

0. **Measure first.** Read TI SNVSAN3F's layout and input-capacitor sections, and SNVSB89 (LMR33630) pp. 23 and 32, again for the exact bulk rule. Find whether TI states a θ or Ψ figure intended for design on its EVM, and what condition it states.
   - **STOP** with what you found if the buck's junction cannot be bounded from a held source at all. The ruling then picks between a declared bound and a refusal.
1. **Hot-plug (`BatteryInputUndamped`).** Each battery input that feeds a regulator's VIN carries one of:
   - a declared bulk capacitor in TI's 20–100 µF range, with its rated voltage above the battery's maximum, and its ESR cited because damping is the point;
   - a declared clamp (TVS) whose stand-off voltage is above the battery's maximum and whose clamping voltage at its rated pulse is at or below the regulator's absolute maximum, both cited.

   Otherwise refuse by name. The ring's size is a named condition in the verdict.
2. **The buck's junction (`BuckJunctionOverLimit`).**
   - T_J = T_A,max + P_loss × θ.
   - P_loss comes from a declared loss bound per buck instance, with its `source`: TI's efficiency figure at the operating point, which is typical.
   - θ is the held figure §1.0 names, with its board condition stated.
   - It is a **screen with a named margin**, as Isat is, never a guarantee. The verdict names both figures' conditions.
3. **DC bias (`EffectiveCapacitanceBelowMinimum`).**
   - Each ceramic capacitor row may cite its maker's typical DC-bias figure at the operating voltage. Effective capacitance, with a named margin, is screened against the regulator's stated minimum.
   - Where no figure is held, a declared derating per dielectric class and voltage ratio, with its source, stands in. The verdict names it as declared.
   - This reverses the "not judged" half of `4300d` D6, as ruled in the review response. The rated-value check stays.
4. **The rising brown-out threshold.** The supply floor must clear the selected level's rising maximum as well as its falling maximum, both from DS12117's table. Refuse with `SupplyFloorBelowBorRising`.
5. **Control pins.** Each regulator's EN and PG, and the translator's OE, is declared as either wired to a named net with a stated role, or tied by a stated strap. An unwired control pin is refused (`ControlPinUnwired`).
6. **The fixture's 3.3 V divider:** re-pick it so its centre sits at least 1 % inside the engine's own band, or say in FEEDBACK why the present pair is kept.

## 2. Acceptance

- **The Sentinel-02 power fixture passes every new check** after its declarations are added. Each check fails a fixture made to break it.
- **Every existing board's files and verdicts are byte-identical**, apart from the specification-hash lines a version bump moves.
- **Each new refusal has a tier-1 test.** The CLAUDE.md gates pass. CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`; `RUL-14`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir`: the power checks, the parts rows, the new declarations;
  - tests and fixtures; `ci/`; FEEDBACK.
- **Next free versions:** take them from `main` at kickoff. The second to merge renumbers.
- One PR, left unmerged. Write `~/.cache/aet-4300d3/pr-title-body.md`.

## 5. Adversarial pass, full

1. **An undamped battery input:**
   - bulk rated below the battery's maximum;
   - a TVS whose clamp exceeds 38 V;
   - bulk on the wrong side of the connector.
2. **A hot buck:**
   - a loss bound with no source;
   - an ambient above the part's range;
   - a θ from another package.
3. **A bias lie:** an effective capacitance above the rated value, or a derating above 1.
4. **A floor between the falling and rising maxima.**

## 7. Flag-and-stop conditions

- **§1.0 finds no held source that bounds the buck's junction.**
- **No held capacitor row has a published DC-bias figure,** and the declared derating would be the only source for every capacitor.
