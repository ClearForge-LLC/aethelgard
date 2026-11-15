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
