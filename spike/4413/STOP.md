# AET-WO-4413 part (1): STOP

2026-11-27, branch `wo/aet-4413-s03-rf-reset-ground`. Predictions: `PREDICTIONS.md` (committed `db0bc89d` before any probe). Scored: `RESULTS.md`. Surveys: `survey-reset-rail.md`, `survey-ground.md`.

**In one paragraph.**
- **The reset can be built**, with a newly held part, TPS3839G33.
- **The radio rail can be raised** to make room for it, with every consumer's maximum held.
- **VDD_3V3 can pass** on today's copper if its rail is raised the same way, by 0.25 mV.
- **VDD_3V3_RF cannot pass by anything this work order's fence allows.**
  - The split is not expressible, and the gate's credit for it is a change to `4310`'s rule.
  - Even with both, the arithmetic leaves it 6 mV over.
  - The one spec lever, a wider class, cannot be routed at U31's TSSOP pad.

Two of §7's stops apply: the split cannot meet both limits, and the work needs engine changes outside the fence.

## The table

| | Today | Reset + both rails raised (in the fence) | Plus the split, with the gate's single-join credit (engine) | Plus the RF supply path halved (router) |
|---|---|---|---|---|
| Readiness | refused: U30, U31.B | **pass** (predicted: U30 held by TPS3839G33; U31.B by its OE, D3) | pass | pass |
| VDD_3V3 bound / limit | 271.3 / 110, fail | **271.7 / 272, pass by 0.25** (measured) | about 118 / 272 | about 118 / 272 |
| VDD_3V3_RF bound / limit | 240.8 / 144, fail | **241.2 / 146, fail by 95** (measured) | about 152 / 146, fail by 6 (arithmetic) | about 98 / 146 (arithmetic: supply side 216 → 108 mΩ) |
| Fence | — | `examples/sentinel-03.json`, parts data, `aethelgard-ir` (one field) | `aethelgard-ir`, schematic, pour, placer, and `4310`'s gate | the router (neck-down at a fine-pitch pad) or the placer (U13 beside U30) |

## Decisions to rule

**D1. The radio rail's monitor: TPS3839G33DBZR.**
- Held 2026-11-27 under `main`'s held-copy rule: `~/.cache/aet-held/ti-tps3839-sbvs193d.pdf`, SBVS193D, sha256 `a1894023…`, `SOURCE.txt` beside it.
- VIT− 3.003 / 3.08 / 3.126 V at −40 to 85 °C (p. 7, §7.5), inside the board's −20 to 85 °C. Push-pull active-low RESET (p. 4), SOT-23.
- Its minimum of 3,003 mV is at or above U30's 3,000.
- Its release (3,126 + 31 mV typical) is a screen with a bench row, as TLV803S's is on the MCU rail.
- It is placed at U30 and senses the rail at U30's pin.
- No other held part fits: TLV803S (2.87 V), TLV803's other variants, and TPS3840 (guaranteed 175–225 mV hysteresis).
- **Lean: accept.**

**D2. The field.**
- A list `power.rail_monitors[]`, each `{reference, part, rail, drives: [{reference, pin}]}`, in `aethelgard-ir`.
- `supervisor::judged` counts a part on `rail` held only when the monitor drives that part's enable, so a monitor credits only what its output holds (§5's first question).
- `mcu.reset_supervisor` stays as is. The emitter draws the part and wires RESET to each driven pin, replacing U30's EN RC pull-up.
- Specification version 31 → 32.
- **Lean: this shape.** The other shape, a second `reset_supervisor`, cannot name what it drives.

**D3. U31.B.**
- (a) Drive U31's `OE` from the same RESET, so U31.B is held. OE is VCCA-referenced; RESET's high is the radio rail, 3,303–3,589 mV, inside OE's absolute maximum, to be confirmed at SCES935A p. 5 in the build.
- (b) Leave `OE` strapped, and U31.B stays named.
- **Lean: (a).** U30 off with U31.B live is the case the rule names.

**D4. The radio rail.**
- U13's feedback becomes 100k / 41.2k (TNPW0603 B), solved 3,303.185–3,589.075 mV.
- The domain becomes 3,430 mV ± 4.9 %. ± 5 % is refused, `DomainOutsideConsumer`, by U31's 3.6 V rows.
- The drop limit becomes 146 mV, "the solved minimum less TPS3839G33's release, VIT− max 3,126 mV plus 31 mV typical".
- **Lean: accept.**

**D5. VDD_3V3.**
- (a) Raise U11 the same way, with the domain at 3,430 mV ± 4.9 %. Its limit, by its own rule (the solved minimum less TLV803S's 3,030 mV release, strictly above), becomes 272 mV. Measured on today's copper: 271.747 mV, a pass by 0.25 mV.
- (b) Keep it as it is, and rely on a later split.
- **Lean: (a).**
  - The limit's derivation is the ruled one (`4300k` D6), only its set point moves. Whether that counts as a limit re-derived toward the answer is §5's fourth question, and yours.
  - The 0.25 mV is a certified upper bound's margin, real but thin: any routing change can take it. A split would make it about 154 mV.

**D6. VDD_3V3_RF's ground: not buildable inside this fence.**
- (i) An engine work order (or two) first:
  - a second ground net with one join;
  - its regions on In1 and the outer layers;
  - placer clustering;
  - the gate's credit for a current whose source and sink are both past the single join, counted from the board as read.
  - About 900–1,850 lines (survey §2). It changes `4310`'s gate, so it needs its own ruling.
  - Even then, VDD_3V3_RF is predicted 6 mV over, so it also needs:
- (ii) A second lever for the RF supply path, today 216 mΩ, 109 of its 123 mV: a neck-down at a fine-pitch pad in the router, so the class can be wider elsewhere, or a placement rule that puts U13 beside U30.
- (iii) Review the 505 mA load: U30's "IVDD current delivered by external power supply, 0.5 A min" (v1.7 p. 21) is a supply-capability figure, not a measured draw. This is a question only. Changing it would re-derive toward the answer, and I don't lean to it.
- **Lean:**
  - Split this work order. **4413 now:** D1–D5, the reset, both rails and readiness, with Sentinel-03 still refused by `extract` for VDD_3V3_RF alone, by name.
  - **An engine work order** for (i) and (ii), then a follow-up board work order.
  - The package waits on the engine work order.
  - `board_tier2`'s Sentinel-03 test keeps pinning the one refusal until then.

## The cost

- `emit` 97 s, `extract` 952 s on the raised-rail board (Furnace).
- `bench-package-s03` refuses at readiness in 6 min today.
- With D1–D5 built, it passes readiness and fails at `extract` (VDD_3V3_RF), so it is not green until D6.

## Not done (part (1) only)

- No crate, scenario or package changed.
- The two new datasheets are held. A parts row for TPS3839G33 waits on D1.
