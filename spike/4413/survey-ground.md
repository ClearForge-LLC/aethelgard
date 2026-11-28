# AET-WO-4413 — ground-split design survey (read-only)

Surveyor: an Opus subagent, 2026-11-27. Tree: `~/aethelgard-4310` on `wo/aet-4413-s03-rf-reset-ground` (head `86a1f280`). No repo file edited; no cargo or binary run. Numbers come from the committed `spike/4310/out/s03-extract-3c.json`, the extract behind FEEDBACK's 271.3 / 240.8 mV. **[code]** marks what the code shows. **[inferred]** marks my reasoning. Every line count is an estimate.

## 0. Answer first

1. **The ruled split is necessary, but on today's bounds it is not sufficient.**
   - Suppose the 5 V ports' return and U10's input loop are taken off logic ground perfectly, and the gate charges them nothing. Everything else stays as measured. Then:
     - `VDD_3V3` = 85.7 + 22.3 (`VDD_3V3_RF`'s load) + 10.0 (U13's input) = **118.0 mV against 110**;
     - `VDD_3V3_RF` = 122.7 + 20.1 (`VDD_3V3`'s load) + 9.4 (U11's input) = **152.2 mV against 144**.
   - **Both still fail by about 8 mV.**
2. **U11's and U13's input loops cannot leave logic ground in the present model.**
   - Their PGND pins are the gates' own return pins, looked up on the net `GND` ([code] `supply.rs:1675-1678`).
   - With every buck's input term also at zero, the floors are 108.0 and 142.8 mV. They pass by 2.0 and 1.2 mV, before any reroute moves a bound.
   - Under any certified bound the engine has today, their 10.0 / 9.4 mV terms stay, wherever the tie is (§3).
3. **Today's engine would make a split worse, not better.**
   - A current whose ground pads are not on `GND` is billed at the load's own largest return bound R11 ([code] §1.3).
   - Drawn as a second net, `VDD_3V3` would go to about **334 mV**, and `VDD_3V3_RF` would be unchanged. **An engine change to `supply.rs` is mandatory** for any split to count.
4. **A second lever is needed beside the split.** The largest available ones:
   - `VDD_3V3_RF`'s supply side, 216.2 mΩ × 505 mA = 109.2 of its 122.7 mV;
   - `VDD_3V3`'s worst return pin, R11 = 65.3 mΩ.
   - One of them can be set from the specification today: `net_classes` track widths (§3.3).
5. **The specification cannot express a split today.** `GND` is one fixed net, and there is no zone, region, tie or return-group field (§1.1).
   - The leading option (b) is engine work, not a board edit: an estimated 900–1,850 lines across `aethelgard-ir` and `aethelgard-eda`, with a specification version bump.
   - **This is a scope flag for the architect and Scotty.**

## 1. How ground is modelled, end to end

### 1.1 Specification (`aethelgard-ir`) [code]

- **One ground net, by constant.**
  - `pub const GND: &str = "GND"` is "the emitter's ground symbol, never a declared domain" (`crates/aethelgard-ir/src/lib.rs:165-166`).
  - Any domain id or signal net that folds to `gnd` is refused: `lib.rs:3902` (`DuplicateDomainId`) and `lib.rs:4089` (`NetShadowsDomain`).
  - A pin naming any other ground-like net (say `GND_PWR`) becomes an ordinary signal label (`crates/aethelgard-eda/src/schematic.rs:891-899`, `declared`).
- **Planes.**
  - `board.layers[]` gives each layer a `role` of signal or plane, and a plane's `net` (`board.rs:120-133`).
  - At 4 layers, both inner layers are planes, exactly one of them `GND` and the other a declared domain. The checks: "one plane per net", and `PlaneNet` for any net that is not `GND` or a domain (`board.rs:12-14`, `board.rs:555-569`).
  - Six layers are refused by name (`board.rs:13-14`; `LAYER_COUNTS = [2, 4]` at `board.rs:49`).
  - Sentinel-03 is In1.Cu = `GND`, In2.Cu = `VDD_3V3` (`examples/sentinel-03.json`, `board.layers`).
- **`board.stackup`.** It holds only bounds: copper, plating, temperature and fab tolerances (`board.rs:137-200`). It has no zone, region or keep-out.
- **`net_classes`.** These set track width and via per net, and `GND` may be in a class (`classes.rs:1-30`, `classes.rs:193` refuses `GND` only in a *pair*). Sentinel-03 has `POWER` (500 µm: VBAT, `VDD_5V`, `VDD_3V3_RF`) and `SUPPLY_MCU` (200 µm, 0.75 mm via: `VDD_3V3`); `GND` is on the default rule.
- **No star point, net tie, return group, zone, pour, region or keep-out field exists.** I grepped for zone, pour, plane, net_tie, star, return and keepout across `lib.rs` and `board.rs`.
  - The only keep-outs come from vendored footprints (`route.rs:253-283`, `Keepout`).
  - There are no placement fields: connectors go to edges in rank or reference order (`pcb.rs:827-868`).
- **DS-009 ports.** "A ground pin takes `GND`" (`ds009.rs:71`, `84`). J6, J7 and J8 are declared DS-009 (TELEM, TELEM, I2C), so their pin 6 / pin 4 cannot name another net today.
- **Version.** `SPECIFICATION_SCHEMA_VERSION = 31` (`lib.rs:164`); each bump writes a `versions::CHANGES` row (`versions.rs:56`).

### 1.2 Schematic, placer, router and emitter (`aethelgard-eda`) [code]

- **Every ground connection is `Conn::Rail("GND")`** through `Drawing::ground()` (`schematic.rs:885-888`).
  - Each buck's PGND **and EP** go to it (`schematic_power.rs:785-786`).
  - Its CIN and CIN_HF are anchored between VIN and that PGND (`schematic_power.rs:835`, `886`).
  - The input connector, hot-plug bulk and protection go to it as well.
- **Planes.** `Stack::planes` is read from the specification (`stack.rs:390-400`).
- **Thermal pours.** With any buck declared, `GND` is poured on every outer layer that no plane fills (`stack.rs:500-509`). On Sentinel-03 that is F.Cu and B.Cu `GND`.
- **Zones are the whole outline.** Every zone, plane or pour, is emitted as the outline rectangle (`pcb.rs:2877-2901`). No zone has a region.
- **The router's zone check.**
  - The zones must be exactly the declared ones: "at 4 every plane and nothing else", plus the pours (`route.rs:4651-4654`, `route.rs:4755-4777`).
  - **So a second filled ground net is refused** unless it is declared, and no declaration exists.
- **How ground is joined.** GND pads join In1.Cu by vias. The bucks' thermal pads join by thermal-via arrays (`routed.arrays`).

### 1.3 `extract`'s supply gate [code]

**Which copper it reads.**
- Labels are trusted only once they agree with the copper: `route::connectivity` refuses `NetLabelsDisagree` (`extract.rs:1010`; `route.rs:4651-4660`).
- So two ground nets joined by a **0 Ω part** read as **two nets**.
- A KiCad **net-tie footprint** is refused outright, because `net_tie_pad_groups` waives DRC (`board.rs:178-184`, `strict.rs:478`, `RUL-03` at `docs/architecture.md:548`).
- Copper of the two nets touching is refused by `route::check` (clearance) and by connectivity.

**The return net is the literal `"GND"`.**
- `let gnd = "GND"` (`supply.rs:1676`).
- The return pad is the victim regulator's PGND on `GND`, or `TerminalMissing` (`supply.rs:1677-1678`, via `pad()` at `supply.rs:357-380`).
- `GND` must be a plane, or every return pin is `GroundReturnUnread` (`supply.rs:1755-1768`).

**The copper model (`copper()`, `supply.rs:1-31`, `542-…`).**
- On F.Cu and B.Cu: only the net's tracks, via pads and pads (`supply.rs:7-12`, `744-778`). **The outer `GND` pours are not read** (an inner approximation; leaving copper out only raises R).
- On an inner layer: only a plane *of that net* (`supply.rs:739-744`, `13-19`).
- Plane copper beyond 5 mm (`CROP_NM`) of the terminals' box is cropped (`supply.rs:698-705`).

**How each other current is charged.**
- **Other regulated domains** (`supply.rs:1806-1846`).
  - The entry pads are each consumer's pads on `GND` (`pads_on(board, part, "GND")`, `supply.rs:1825`).
  - A domain whose consumers have no `GND` pad is described as entering "off the board" (`supply.rs:1831-1836`).
- **Other battery-fed regulators' input currents** (`supply.rs:1849-1886`).
  - The entry is that regulator's PGND **on `GND`** (`pad(board, gnd, …).into_iter()`, `supply.rs:1874-1875`).
  - The victim's own regulator is skipped (`supply.rs:1856`).
- **`charge()`** (`supply.rs:2006-2055`).
  - The entry pads are solved farthest-first, six at a time, stopping once one reaches R11 (`2031-2041`).
  - **No entry pads → `unread` → `entry_bound = None` → charged at R11** (`2028-2030`, `2042-2047`).
  - The charge is I · min(R11, entry bound). Where the current *leaves* is never used (the doc at `supply.rs:1388-1396`; condition text at `2117`).

**What a split would do today [code → arithmetic].**
- J6–J8's and U10's ground pads would be off `GND`, so `VDD_5V` and U10's input would be billed at R11 = 65.325 mΩ: 2000 × 65.325 = 130.7 mV, and 1306 × 65.325 = 85.3 mV.
- `VDD_3V3` ≈ 85.7 + 130.7 + 85.3 + 22.3 + 10.0 = **334.0 mV**, assuming R11 and the rest unchanged.
- `VDD_3V3_RF` already bills every other current at its R11 (below), so it is **unchanged at 240.8**.

### 1.4 Today's terms (from `spike/4310/out/s03-extract-3c.json`)

| Victim (return pin) | Own: supply / return worst, mΩ | Load | R11 | Other current, mA | Entry bound, mΩ | Charged at | Charge, mV |
|---|---|---|---|---|---|---|---|
| `VDD_3V3` (U11.1) | 49.0 / 65.3 → 85.7 mV | 750 | 65.3 | `VDD_5V` 2000 (J6–J8) | 42.9 | entry | 85.9 |
| | | | | `VDD_3V3_RF` 505 (U30, U31) | 44.1 | entry | 22.3 |
| | | | | U10 input 1306 | 51.6 | entry | 67.4 |
| | | | | U13 input 249 | 40.3 | entry | 10.0 |
| `VDD_3V3_RF` (U13.1) | **216.2** / 26.8 → 122.7 mV | 505 | **26.8** | `VDD_5V` 2000 | ≥31.5 | **R11** | 53.6 |
| | | | | `VDD_3V3` 750 | ≥37.2 | **R11** | 20.1 |
| | | | | U10 input 1306 | ≥38.9 | **R11** | 35.0 |
| | | | | U11 input 352 | ≥40.3 | **R11** | 9.4 |

**Two structural facts follow from this table.**
- **For `VDD_3V3_RF`, every charge is already at R11.** Moving an aggressor changes nothing until its entry bound falls under 26.8 mΩ.
- **For `VDD_3V3`, every charge is at its entry bound, 40–52 mΩ.**
  - That is a narrow band for pads spread across the board.
  - The MCU's own return pins span 36.7–65.3 mΩ; 1303b measured 37.5–41.9 mΩ across ten pins (`AET-WO-1303b.FEEDBACK.md:36`).
  - **[inferred]** So the bound is dominated by the pad-to-plane constriction at each end (pad, track, worst-case via), not by distance across the plane.

## 2. Options, ranked

### (b) A second ground net joined to `GND` by exactly one tie — LEADING (necessary, not sufficient; §3)

**What goes on it [inferred].** Call it `GND_PWR`.
- J4's battery return, C1 (the hot-plug bulk), and D1/R60's ground return if any.
- U10's PGND **and EP** (EP is AGND, SNVSAN3F Table 6-1, cited at `supply.rs:1658-1659`), and U10's C10, C60, C11, C13 and C14–C17.
- J6–J8's ground pins.
- Logic stays on `GND`: the MCU, U11 and U13 with their caps, U30, U31, U12, the sensors, and J1–J3.

**Why this one [inferred].**
- At DC, Kirchhoff's current law across a single tie is exact. A current whose entry *and* exit are both on `GND_PWR` puts zero net current through the tie, so none flows in `GND` copper.
- The 5 V port loop (J6–J8 → U10 PGND) and U10's input loop (U10 PGND → J4) are both wholly on `GND_PWR`.
- **This is the only option that removes current, rather than moving it, that the gate can *certify*.**

**Expressible today?** No.
- There is no field, `GND` is fixed (§1.1), DS-009 ports require `GND`, and the schematic hard-wires every PGND and EP to `GND`.

**Smallest engine change (estimates).**
1. **`aethelgard-ir`** (keeps INV-01):
   - a declared power-ground block, e.g. `power.ground_split { net, members (references and connector pins), tie { reference, part } }`;
   - validation: the name, members exist, J4 and U10 together, U10's PGND and EP together, and no regulator whose return pin is a gate's on it;
   - `ds009` admitting the declared power ground on a port's ground pin;
   - a held 0 Ω row with its rated current and maximum resistance (INV-06);
   - **a version bump, 31 → 32**, with a `CHANGES` row. A key is added, unlike 4310's 08:55Z no-bump ruling.
   - About 200–350 lines.
2. **`aethelgard-eda` schematic:** a second ground rail, symbol and wiring for the members, and the tie part. About 150–250 lines.
3. **`aethelgard-eda` stack, router, emitter, readers: regions.**
   - **Without them, U10's junction screen refuses.**
   - If EP sits on a net with no plane and no pour, `pour::joined` counts no layer (`pour.rs:285-287`, `min_mm2` = 0 at `pour.rs:376`). `junction_screen` then refuses an area under its figure's smallest (`power.rs:4323-4328`).
   - Today U10 joins In1.Cu 3,382, B.Cu 2,750 and F.Cu 2,082 mm² (from the same JSON). It is the hottest buck, at 1,170 mW.
   - So `GND_PWR` needs a region on In1.Cu and on the outer pours around U10, J4 and J6–J8, with `GND` filling the rest. This also serves U10's hot loop (SNVSAN3F §10.1.1, "the PGND side", already listed as not judged at `loops.rs:170`).
   - The pieces: zone polygons in `pcb.rs:2877-2901`; the declared-zone check at `route.rs:4755-4777`; keeping each net's vias inside its region; `copper()`'s plane branch, which today assumes the outline inset (`supply.rs:822-…`); `pour.rs` per region; and the strict reader's zone polygon.
   - About 400–800 lines.
4. **Placer:** J4, U10's block and J6–J8 placed into that region (today's connector edge order is by reference, `pcb.rs:827-868`). About 100–300 lines.
5. **`supply.rs`, the charge** (about 60–150 lines, plus planted tests).
   - Read `GND_PWR`'s pads from the read board.
   - Count the parts with pads on both nets *from the board*. Exactly one (the declared tie) means a single bridge.
   - A current whose entry and exit pads are both on the far side of it is charged **0**, with a named condition.
   - A current that crosses is charged by its entry on `GND`, as today. One that enters on `GND_PWR` and exits on `GND` enters at the tie pad.
   - Two or more joins: charge at the join pads. **The credit comes from the copper, never from the declaration (INV-07).**
   - **Without this, (b) scores 334 mV (§1.3).**

**Invariants.** INV-01 holds if the new type lives in `aethelgard-ir`. INV-08 holds, because the change is gated on the declared key and every existing board is byte-identical.

**Total:** about 900–1,850 lines. That is one or two engine work orders before the Sentinel-03 board work order.

### (c) A pour split or keep-out on a layer

**Same net, split by a slot with one neck.**
- Physically it is (b). But the gate reads one net and ignores where currents exit.
- The neck raises R(entry, return pin) toward R11, so the `VDD_3V3` charges rise (2 A × up to 65.3 mΩ = 130.7 mV). **Worse.**
- A neck many cells wide is no cut vertex of the gate's cell network, so no exact zero follows.

**Expressible today?** No. There is no region or keep-out field (§1.1), and zones are the whole outline (`pcb.rs:2897`).

**Role.** As a separate net (b) it is needed only as (b)'s region sub-part (point 3 above), never alone.

### (a) Placement only

**Expressible today?** No. There are no placement fields; the placer is deterministic (`pcb.rs` `place`, `connector_edges`).

**What would reduce the charge [code + inferred].** The charge is I · min(R11, max entry R to the *victim's return pin*).
- So it falls only by moving the aggressors' ground pads electrically **toward U11's / U13's PGND**. That is the opposite of good physics: it pulls the 2 A return toward the regulator that references the MCU.
- STOP's fix (i) "connectors beside U10" improves the *true* coupling but **not this bound**, which never reads the exit.

**It cannot pass [inferred from a measured analogue].**
- The tightest pad-to-PGND bound on the board today is U30/U31 to U13 PGND, at 18.1 mΩ.
- `VDD_3V3`: even at 18 mΩ, 2000 + 1306 mA gives about 60 mV, against 24.3 mV of room.
- `VDD_3V3_RF`: charges sit at R11, so placing the aggressors helps nothing above 26.8 mΩ, and (4408 mA × 18.1) is 79.8 mV against 21.3 mV of room.

### (d) A dedicated return layer

**Expressible today?** No.
- Two `GND` planes break "one plane per net", and a `GND_PWR` plane breaks `PlaneNet` (`board.rs:555-569`).
- Six layers are a later work order (`board.rs:13-14`; `stack::kicad_id` knows four layers, `stack.rs:16-23`).

**Replacing In2.Cu (`VDD_3V3`) with a return plane.**
- `VDD_3V3` would go to tracks. Compare `VDD_3V3_RF` on 500 µm tracks: 216 mΩ supply side, against `VDD_3V3`'s 29.9–49.0 mΩ on its plane.
- That alone would blow the 110 mV limit [inferred].

**What it still needs.** As a separate net it still needs all of (b)'s single-tie logic. Ranked last.

### (e) Not a board change, for comparison

- The architect's lean (ii) in FEEDBACK §6 is a certified lower bound (the dual or Dirichlet solve). It would let the gate use the exit (I·(G(a,c) − G(a,t))).
- It is the only lever that removes U11's and U13's input terms (§3.1), and the true coupling as drawn is probably far below the bound.
- Scotty ruled the copper change. I flag that the copper change alone leaves about 8 mV per limit.

## 3. Predicted effect of (b) on both bounds

### 3.1 What vanishes and what stays

Assumptions: (b) as above, with (b).5's charge built, and R11, the own drops and the remaining entry bounds unchanged. That is optimistic: the region and placer changes reroute.

| Term | `VDD_3V3` now → (b) | `VDD_3V3_RF` now → (b) | Why |
|---|---|---|---|
| Own | 85.7 → 85.7 | 122.7 → 122.7 | unchanged copper assumed |
| `VDD_5V` load | 85.9 → **0** | 53.6 → **0** | wholly on `GND_PWR`, single tie |
| U10 input | 67.4 → **0** | 35.0 → **0** | U10 PGND → J4, both on `GND_PWR` |
| Other logic domain's load | 22.3 → 22.3 | 20.1 → 20.1 | MCU and radio share `GND` through U31 and UART `WIFI`; it cannot split |
| Other logic buck's input | U13 10.0 → 10.0 | U11 9.4 → 9.4 | enters `GND` at its PGND, exits at the tie; the bound ignores the exit, so the tie's position changes nothing |
| **Sum / limit** | **271.3 → 118.0 / 110** | **240.8 → 152.2 / 144** | **both fail, by 8.0 and 8.2 mV** |

### 3.2 The conflict in "the bucks' input loops"

- Only U10's input loop can come off. U11's and U13's PGND are the gates' return pins on `GND` (`supply.rs:1677-1678`).
- Putting them on `GND_PWR` would refuse both gates (`TerminalMissing`). Or it would need a cross-net return path through the tie, with `GND_PWR`'s 2 A sharing the tie-to-U11 segment. That moves the problem.
- **No single tie position zeroes both.**
  - A tie at U13's PGND zeroes U13's input physically for `VDD_3V3`, but U11's input then flows U11 → U13 PGND, a real charge on `VDD_3V3_RF`.
  - A tie at U11's PGND does the reverse.
  - Either way, today's bound charges both at their entry.
- With both at zero (which needs (e)), the floors are 108.0 (2.0 mV margin) and 142.8 (1.2 mV margin). Any reroute moves own drops by more than that: 82.3 against 85.7 mV across board revisions in 1303b.

### 3.3 What else must move — measure-only

**`VDD_3V3` needs 8.0 mV more**, any one of:
- R11 from 65.3 to ≤ 54.6 mΩ (750 mA on the own drop);
- `VDD_3V3_RF`'s entry bound from 44.1 to ≤ 28.3 mΩ (the radio cluster's worst ground pad electrically nearer U11's PGND);
- a mix of these.

**`VDD_3V3_RF` needs 8.2 mV more**, either of:
- **R11_RF from 26.8 to ≤ 21.7 mΩ.** Every remaining term rides on it: 505 + 750 + 352 = 1,607 mA.
- **Its supply side from 216.2 to ≤ 200.0 mΩ** (8.2 mV / 505 mA = 16.2 mΩ).

**Specification-expressible levers today:**
- `net_classes`: give `VDD_3V3_RF` its own wider class, split from `POWER`'s 500 µm;
- put `GND` in a class with wider tracks or a larger via.

What fraction of 216 mΩ is track rather than pads and vias is **unknown: measure**.

**Precedent risk.** At 1303b, GND on 0.7 mm vias moved the routing and failed `IMU_SCK` (31.27 > 30 pF) (`AET-WO-1303b.FEEDBACK.md:164`, `:257`).

**Honest bottom line.** With (b) the bounds are plausibly near the limits, but whether either passes can only be measured.
- The prediction above is an upper estimate of the residual at unchanged copper.
- The new placement can move R11 and the own drops either way: the region knocks out GND_PWR's vias from In1's `GND` copper, which lowers plane copper.

## 4. Adversarial: does each option remove the current, or merely move it?

- **(a) Moves it.** The same copper and the same plane; only the bound's R changes. This fails the first question by construction.
- **(b) Removes it, exactly at DC (KCL), but only if:**
  1. there is exactly one on-board join. The gate must count it from the read board: the parts with pads on both nets equal the declared tie, and nothing else.
     - Today's readers already refuse copper contact between nets: `NetLabelsDisagree` and clearance.
     - A capacitor between `GND` and `GND_PWR`, or U10 with EP on `GND` and PGND on `GND_PWR` (its die becomes a second, unrated join), must be counted as a second join and charged at the join pads. That gives no credit.
  2. **there is no off-board join. The gate cannot see this, so it must be a named condition.** On a drone it is false:
     - J5's ESC signal ground (pin 6) returns to battery − through the ESC's power lead.
     - With J4 on `GND_PWR` and J5 on `GND`, the ESC wiring is a second tie.
     - The motor current's lead drop drives a loop current through `GND` from J5 to the tie. Today the same loop runs J5 → `GND` → J4.
     - Mitigations [inferred]: J5's ground on `GND_PWR` (MOTOR signals then cross the split), or a tie placed near J5.
     - Any J6–J8 peripheral grounded elsewhere (a receiver, or a GPS also on a PDB) is the same case. This is FEEDBACK F11's off-board peers, judged by nothing.
- **(c) same net: moves it** (one net; the neck only lengthens the path). **(c) as (b)'s regions:** as (b).
- **(d) same net on two planes: moves it** (the current divides through vias). **(d) separate net:** as (b).

## 5. Physical and electrical soundness [inferred unless cited]

- **A single-point join of power ground to logic ground is standard.** TI's layout guidance for these bucks keeps CIN, PGND and COUT on one solid ground island (SNVSAN3F §10.1, cited at `loops.rs:170` as not judged).
- **U10's EP and AGND must stay on the same net as its PGND.** Otherwise the die is the tie. This is what forces the regions (§2(b).3).
- **U10's hot loop.**
  - If `GND_PWR` had no plane under U10, the switching loop's image current would flow in In1's logic `GND`: AC injection the DC gate never sees.
  - The region on In1.Cu fixes that; a tracks-only `GND_PWR` does not.
- **Battery input path.**
  - J4 −, C1 and the protection's ground side (R60's return) go on `GND_PWR`. F1 and Q1 are high side, so they are unaffected.
  - U11 and U13's input currents (352 + 249 = 601 mA at the 8,917 mV pack minimum) now cross the tie. The bucks' headroom (3b) does not count the tie, `GND_PWR` or the `GND` return copper.
  - About 601 mA × (tie + copper), a few tens of mV, against several volts of headroom. Small, but it should be named.
- **The tie part's rating.** A 0603 0 Ω part (rating unheld) may be marginal for 601 mA DC plus the unbounded ESC loop current. The held row must carry a cited rated current and maximum resistance (INV-06); a 1206 or 2512 jumper is safer.
- **U31 (TXU0304).** It has one GND, so both its sides, and U30 and U13 with them, must stay on logic `GND`. No translator reference crosses the split; I recommend it stays that way.
- **UART and I2C crossing the split.**
  - CRSF and MAVLINK (J6, J7) and `EXT` I2C (J8) are driven from logic `GND`, but the port's ground is `GND_PWR`.
  - The offset is about the 2 A × R(`GND_PWR`, port → tie), up to about 2 A × 43 mΩ ≈ 86 mV, plus U10's ripple. That is the same order as today's offset through shared copper, and inside 3.3 V logic margins.
  - The signals' AC return detours through the tie, so place the tie by the ports and route the signals over it.
  - The line-level and bus judges assume one ground. **The offset is a new unjudged term, to be named.**
- **The ESC and motor return:** §4. Through J5, it is the dominant real-world ground disturbance on a flight controller, and the split does not cure it by itself.

## 6. Rows and flags for the architect

1. **Scope.** (b) is about 900–1,850 lines of engine work across `aethelgard-ir` and `aethelgard-eda`, with a version bump. It is not a Sentinel-03 board edit.
2. **Insufficiency.** On today's bounds, (b) leaves about 118 and 152 mV. A second lever must be ruled: a net-class width or via change (specification-expressible, measure-only), or the dual solve (e).
3. **Wording.** "The bucks' input loops" can apply only to U10 under the present return-pin model.
4. **Off-board ties** (J5's ESC, the DS-009 peripherals) become a named condition of any single-tie credit.
5. **Measure before predicting further.** The first cheap probe is the `net_classes` lever on the board as drawn, since it needs no engine change: a wider `VDD_3V3_RF` class, and `GND` in a class.
