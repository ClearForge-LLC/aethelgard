# AET-WO-1308 FEEDBACK — router: per-net widths, a via-pad option, coupled pairs, rotation

**Builder:** Claude (strong tier), worktree `~/aethelgard-1308`, branch `wo/aet-1308-router-classes` from `main` `d52c051`.
**Cadence:** build, with one measured stop (§1.4: STOP only if the class machinery breaks an existing verdict).

§1 and §2 were committed before any measured run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source in this repository at `d52c051`.
- **[measured]**: run. Host cargo runs under a capped user scope; KiCad runs in the pinned image through `~/bin/capped-run`.
- **[inferred]**: neither; reasoning from the two.

## 1. Readings the work order leaves open (forks taken; each is a question for the architect)

- **F1. The schema bump against "Sentinel-01's committed files stay byte-identical".** §1.1 bumps `schema_version` once. Every key is required (`INV-06`, the `board` precedent), so `examples/sentinel-01-minimal.json` gains `"net_classes": []` and `"diff_pairs": []` and version 4. Its sha256 changes. **[read]** that hash is written into the 8 files of `boards/sentinel-01/` (one provenance line each) and into `aethelgard-twin`'s `REFERENCE` test literal, as `4300c`'s bump did. **Reading taken:** the KiCad files, the BOM and every verdict stay byte-identical; the board crate changes on its hash line only. The number is 4, the next free on `main`; `origin/wo/aet-4300f-build` also takes 4, so the second to merge renumbers (WHO.md's rule), and 4300f is told.
- **F2. "Clearance at the larger of the two classes."** §1.1 gives a class a width, an optional via and nets, and no clearance. **Reading taken:** every class's clearance is `board.rules.clearance_um`, so the larger of two is that value. No clearance field is added. A per-class clearance would be a field the work order does not list.
- **F3. "A via whose ring is under the rules".** `board.rules` declares a drill and a diameter (fixed at 300/600 µm) and no ring. **Reading taken:** a class via's ring, (pad − drill) / 2, must be at least the rules' own, (600 − 300) / 2 = 150 µm; its drill at least `board.rules.via_drill_um`; both at most the board bound. Pad and drill come together or not at all.
- **F4. A pair is its nets' class.** A pair declares a width, so a net in a pair and in a class is "a net in two classes" and is refused by name, as is a net in two pairs. A pair's gap is at least the declared clearance and its width at least the declared track width. Its two nets are distinct, exist, and are not a rail, a plane or ground.
- **F5. What "constant gap" exempts.** The coupled run holds the gap exactly. At each end each conductor has an uncoupled breakout from its pad to the run; the verdict reports each end's uncoupled length. A pair whose conductors would have to cross, or whose centre-to-centre spacing (gap + width) is not a whole number of the router's 0.125 mm grid pitches, cannot be coupled and is refused by name.
- **F6. The length bound.** §1.3 says "a length difference within a stated bound" and declares no field. **Reading taken:** an engine constant, stated in the verdict, not a USB figure. USB full speed needs symmetry for its mismatch condition (`1303` D2), not a skew number.
- **F7. The work order's example class cannot leave the LQFP.** **[read]** LQFP-144 pads are 0.3 mm wide at 0.5 mm pitch. A track centred on a pad keeps clearance s from the neighbour pad only if w ≤ 2·(0.5 − 0.15 − s) = 0.3 mm at s = 0.2 mm; the router's grid margin (`EPS_NM` 10 µm) makes its own limit 0.28 mm. A 0.5 mm `power` class is therefore exactly what §5.3 refuses, on the MCU's supply pins. **Reading taken:** the 4-layer fixture's `power` class is 0.25 mm with a 0.7 mm via pad on a 0.3 mm drill. A neck-down at the pad (class width away from the part, narrower at it) is a question, not built.
- **F8. Rotation without a connector.** No connector footprint is on `main` (`4300f` vendors them). The per-edge test uses an asymmetric stand-in footprint already vendored, rotated 0/90/180/270, one per edge. The placer is unchanged; `pcb` gains the rotation a placement carries, always 0 from today's placer.
- **F9. The pair fixture's nets.** **[read]** USB nets do not exist on `main` (`4300e1`). The pair fixture declares a stand-in pair on two existing two-pad nets between the same two parts, named in the fixture as a stand-in for USB D+/D−. Adding a USB peripheral is outside the fence.

## 2. Predictions (committed before any measured run)

- **P1. Identity.** Every existing fixture's emitted `.kicad_sch`, `.bom.csv` and `.kicad_pcb` is byte-identical to `main`'s binary's; so are `extract`'s verdicts and the gate's rule files for a specification without classes. The only committed-file changes outside the new tests and fixtures are F1's hash lines.
- **P2. The LQFP.** A class wider than 0.28 mm on an MCU pad is refused by the router naming the first such pad ("U1 pad N has no node clear to start a track"); 0.25 mm routes.
- **P3. §1.4 coverage** (`rbound2.py`'s measurement, parameterised for the class), on the 4-layer fixture with `power` = 0.25 mm and 0.7/0.3 mm vias on `VDD_3V3` and `GND`: concentric, 13 to 16 of 16 MCU supply pins (was 11); at ±0.075 mm, 6 to 10 of 16 (was 2). **Not every pin**, so the numbers go to `1303` D5 (b). The ring rises from 0.15 to 0.2 mm, which at the worst offset is 0.125 mm against 0.075 mm.
- **P4. KiCad judges per-net widths from the gate's rules.** A `.kicad_dru` rule conditioned on the net name with `track_width` min = max = the class width reports a track narrower and one wider than its class (both kinds); `via_diameter` and `annular_width` minima report a smaller class via. A track whose label lies is caught first by `route::read`/`check`/`connectivity`, which join copper by geometry.
- **P5. Rotation in KiCad's file.** A footprint's `(at x y θ)` rotates its pads' local positions; each pad's own `(at … angle)` is absolute (θ plus the library angle); positive θ is counter-clockwise on screen with Y down, so a local (x, y) lands at origin + (x cos θ + y sin θ, −x sin θ + y cos θ).
- **P6. The pair.** The stand-in pair routes coupled on F.Cu with 0 vias per conductor, its length difference within the bound; breaking it (asymmetric vias, a gap that widens, a detour) fails the pair judge by name.
- **P7. CI.** Every shard stays under 20 minutes; `TIER2_MIN` rises by exactly the tier-2 tests added (two predicted: classes-and-pair DRC, rotation DRC).

# Build

## B1. KiCad, read in the pinned image (P4, P5) [measured]

Probes under `~/.cache/aet-1308/{rot,p4}`, each through `~/bin/capped-run` (caps read back: `memory.max` 8 GiB, `memory.swap.max` 0, `pids.max` 8192).
- **P5 holds.** The reference's U4 (SOIC-8) at `(at 82.55 108.4 θ)`:
  - KiCad's IPC-D-356 export puts pad 1 (library `(-3.5875, -1.905)`) at (80.645, 111.989) at 90° and (84.455, 104.813) at 270°: origin plus `(x cos θ + y sin θ, −x sin θ + y cos θ)`.
  - A pad's own angle in the file is absolute: a pad left at 0 in a 90° footprint exports unrotated (`R000`), and one written at 90 exports turned.
  - A text's angle is absolute too: an SVG of F.SilkS shows the reference horizontal (1.62 × 1.05 mm) at angle 0 inside a 90° footprint, and vertical at 90.
  - `kicad-cli pcb upgrade` writes back exactly the angles given.
- **P4 holds.** A gate-style `.kicad_dru` rule keyed on `A.NetName`, placed after the board minimum, wins for that net. It reported:
  - every 0.2 mm VDD_3V3 track against a 0.25 mm class (`min width 0.2500 mm`);
  - one track widened to 0.3 mm (`max width 0.2500 mm`);
  - every 0.6 mm via against a 0.7 mm class, with its 0.15 mm ring (`via_diameter`, `annular_width`).

  With every VDD track at 0.25 mm and every via at 0.7 mm, no width or via finding remained.

## B2. What was built

- **Specification v4** (subagent, spot-checked; `9681ede`, `e09fe38`): `net_classes` and `diff_pairs`, both required. Each refusal is its own kind, with a tier-1 test in `crates/aethelgard-ir/tests/classes.rs`: `NetInTwoClasses` (a pair counts as its nets' class), `ClassUnknownNet`, `ClassNoNets`, `ClassBelowMinimum`, `ClassAboveMaximum`, `ViaRingUnderRules`, `PairNets`, `PairOnRail`, `DuplicateClassName`, plus `ClassNetQuoted` (mine: RUL-14 admits `'`, which would break the gate's KiCad expressions, so it is refused rather than escaped by guess).
- **`stack::Rule`** per net: its class's width and via, its pair's width, or the board's.
- **The router:** one set of occupancy maps per distinct rule, each reach computed at that rule. A board with one rule therefore runs today's arithmetic, which P1 measures. Exact judges (`check`, `connectivity`, `zone_core`) take each item's radius from its net's rule.
- **`route::read`** refuses a track or via other than its net's rule, naming the class or pair.
- **The pair router** (`route_pair`) lays each pair first, whole, on F.Cu with no via, and its copper is fixed. The search runs over (node, direction, steps since the last turn), with turns as macros, so both conductors keep legs of at least a pitch. Breakouts are ordinary single-net searches.
  - The start and goal require the run to head away from (A) and towards (B) the pads, with the negative pad strictly on the run's right. Without that, the first fixture's breakouts had to cross (measured).
  - A net stopped only by a pair's fixed copper goes on to the wider search instead of failing at the windowed soft search. That control-flow change is gated on a pair existing.
- **The pair judge** (`route::pairs`) works from copper alone:
  - one path per conductor, pad to pad;
  - the same layers and the same via count;
  - one coupled run whose centre spacing is exactly gap + width, bridged only at a corner;
  - uncoupled at most `PAIR_BREAKOUT_NM` (4 mm) at each end;
  - length difference at most `PAIR_SKEW_NM` (2 mm).

  It runs at the end of `route` and in `check drc --spec` for a specification with pairs. That verdict gains `pairs`; the refusal kinds are `PairNotCoupled`, `BoardUnreadable`, `NetLabelsDisagree` and `ClearanceViolated`.
- **The gate's rules** (`rules.rs`): per class and pair, `track_width` min = max = the width; for a declared via, `via_diameter` and `hole_size` exact and `annular_width` minimum (P4's shape). With no class the file is byte-identical (unit test).
- **Rotation:**
  - `route::rotate` and `route::angle`;
  - `route::part` reads a footprint at 0/90/180/270;
  - `pcb` writes a placement's rotation, turning each pad's, property's and `fp_text`'s own angle with it, and `geometry_at` rotates the keep-out boxes;
  - `pcb::write_moved` and `schematic::emit_moved` place named parts at a given origin and rotation (4300f's hook); the placer still places at 0.
- **`extract`** bounds a line at its own rule's width and via (`Physics::laid_by`). Before this, a classed SPI line would have been bounded at 0.2 mm, unsoundly.
- **`FAB-NOTES.md`** gains one line per class and pair, and qualifies "every hole … 0.6 mm pad" when a class declares a via. Neither changes without classes.

## B3. P1, identity [measured]

`~/.cache/aet-1308/bin/emit-all.sh` ran `main`'s release binary (`d52c051`, v3 fixtures) and this branch's (v4 fixtures) over every specification in `examples/` and the CLI fixtures:
- **14 of 14** have identical exit codes and identical `.kicad_sch`, `.bom.csv` and `.kicad_pcb` sha256: 6 emitted, 8 refused as before.
- `extract` on the 6 boards differs only in the input path and `specification_sha256`.
- `boards/sentinel-01/` changes only on its 8 hash lines (F1). The subagent regenerated it with `generate`, and the result equalled the edited tree.

## B4. §1.4, the supply coverage [measured]

`spike/1303/s4/run_paths2.py`, copied to `~/.cache/aet-1308/s4` and parameterised (board path and W from the environment), in a scratch venv with numpy 2.5.4 and scipy 1.18.1 (offline, from uv's cache). The class conditions set `via_pad_mm` 0.7 and `antipad_radius_mm` 0.685: 0.35 + 0.2005, rounded up to 0.56, plus the 0.125 opening, derived as 1303 derived 0.635. Leaving 0.635 for 0.7 mm foreign vias would have been optimistic.

| 4-layer fixture, 16 MCU supply pins | concentric | ±0.075 mm | best-stand-in range |
|---|---|---|---|
| `main` (0.2 mm, 0.6/0.3 vias; 1307's routes) | **16/16** | **16/16** | 38–160 / 80–284 mΩ |
| via only (0.2 mm, 0.7/0.3) | **16/16** | **16/16** | 23–77 / 65–121 mΩ |
| width only (0.25 mm, 0.6/0.3) | 5/16 | 5/16 | 38–58 / 80–116 mΩ |
| `power` fixture (0.25 mm, 0.7/0.3) | 5/16 | 5/16 | 37–58 / 77–98 mΩ |

- **1303's 11/16 and 2/16 were measured on the pre-1307 board.** On today's routes the closed form already covers every pin at both offsets. The 0.7 mm pad keeps 16/16 and lowers the worst bound at ±0.075 mm from 284 to 121 mΩ.
- **The 0.25 mm width loses 11 pins.** It is not the tube: W = 0.2 mm on the same copper (admissible, since the tube lies inside the copper) gives the same 5/16. The wider stubs shift 9 VDD and 15 GND stitching vias by one pitch, and the closed-form plane pieces then find no admissible path for those pins.
- **Reading for `1303` D5 (b):** the closed form holds every pin with the via-pad remedy. Its coverage is fragile to a one-pitch move of the stitching, though, which is the case for the certified discrete flow. That is the architect's call. Not a STOP: no existing verdict changed.

## B5. Fixtures [measured]

- **`four-layer-power.json`:** `four-layer.json` with `power` (VDD_3V3 and GND, 0.25 mm, 0.7/0.3 vias). It has 244 segments, 109 of them classed, and 71 vias. Every class track is 0.25 mm, every class via 0.7/0.3, everything else as before. ERC and DRC with parity are clean, with no finding (tier 2).
- **`pair-standin.json`:** the reference with SPI on SPI6 (SCK PG13, MOSI PG14, MISO PG12) and FLASH_CS declared on PG15, so U4 is placed by the pins it shares with the pair. `USB_STANDIN` is [SPI_SCK, SPI_MOSI], gap 300 µm and width 200 µm, a stand-in for USB D+/D− (F9). The verdict's `pairs`:
  - lengths 14.625 and 13.125 mm, a difference of 1.5 mm (bound 2 mm);
  - 0 vias each, F.Cu only;
  - uncoupled 0.875 and 0.375 mm at the flash end, 0 at the MCU.
  - ERC and DRC with parity are clean.
- **Without CS pinned,** U4 sits by PA0 on the far side of the MCU. The run is then 38 mm, wraps the MCU, and blocks U4_HOLD.
- **Reversed polarity** is refused by name: `pair USB_STANDIN cannot be coupled` (tier 1).
- **The starve case (§5.3):** `power` at 0.3 mm is refused as `GND`: U1 pad 138 has no node clear to start a track.

## B6. Rotation, one part on each edge [measured]

The reference was emitted with U2 at the left edge at 90°, U3 at the top at 0°, U4 at the right at 270° and R1 at the bottom at 180°. It routes and reads back. U4's pad 1 lands at its origin plus (1.905, −3.5875), and the file carries `(at -3.5875 -1.905 270)` on it. ERC and DRC with parity are clean (tier 2).
- **First attempt:** text left upright while the footprint turned put the reference on the footprint's own turned silkscreen (`silk_overlap`). Turning text with the footprint fixed it.
- **U3 (WSON-6) at 180°** still gives `silk_overlap`: the reference against the pin-1 triangle, about 0.065 mm apart in the library. **[inferred]** KiCad draws a 180° footprint reference upright, so different glyph strokes meet the triangle. The test puts R1 at 180° instead. This is placement quality, not a rotation error, and it is relevant to 4300f's edge placement.

## B7. Predictions against measurement

- **P1** held.
- **P2** held at the pad: 0.3 mm is refused naming U1 pad 138, and 0.25 mm routes.
- **P3 missed, both ways.**
  - The baseline was already 16/16, not 11 and 2.
  - The `power` class gave 5/16, not 13–16 and 6–10: the width moves the stitching.
  - The via-only class gave 16/16 with lower bounds.
- **P4** held.
- **P5** held, and found that text angles are absolute too.
- **P6** held: 0 vias each and a 1.5 mm difference. The pair needed CS pinned, and one polarity.
- **P7:** pending CI.

## B8. Questions for the architect

1. **F1:** the schema bump changes `boards/sentinel-01`'s 8 hash lines and the twin's `REFERENCE`. This reads "byte-identical" as the KiCad files and the verdicts. 4300f also takes v4, so the second to merge renumbers.
2. **F2:** classes carry no clearance. Is a per-class clearance field wanted?
3. **F6:** the bounds were chosen before any measurement: `PAIR_SKEW_NM` 2 mm and `PAIR_BREAKOUT_NM` 4 mm. Should they be specification fields?
4. **F7:** 0.28 mm is the widest class at the LQFP. Is a neck-down at the pad wanted, so a supply class can be wider away from the part?
5. **Pairs are F.Cu only, with no via.** A pair via needs a spacing at which two vias clear each other. At 0.5 mm spacing they cannot, so via pairs are refused by construction.
6. **The subagent's judgement:** `campaign_tier1::a_solved_topology_is_not_a_base` now refuses as `SchemaViolation`, not `UnsupportedSchemaVersion`. Both documents are version 4 now. The refusal still comes before any run.
7. **`1303` D5 (b):** decide from B4.
