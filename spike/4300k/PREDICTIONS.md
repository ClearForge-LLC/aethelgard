# AET-WO-4300k — predictions, written before any run (§1.1)

Written 2026-11-24 on `main` `0faaaec3`, before any `validate`, `solve`, `emit`, `extract` or `verify` ran on this branch. The CLI was built, nothing was run with it.

**Grounds:** code read on `0faaaec3`: `verify.rs::preconditions` (`:183-224`), `eda::supply::supplies` (`supply.rs:1342-1356`), `eda::stack::RingRule::of` (`stack.rs:64`), `ir::connectors` (`POWER_IN`, `:127-199`), `power.rs`'s thermal screen (`:2631-2715`) and hot-plug line (`:2623`), `clock.rs`'s AN2867 Rev 10 citations (`:279`, `:291`, `:420-426`); the fixture `crates/aethelgard-cli/tests/fixtures/sentinel-03.json`, read by a one-line script.

**Not blind:**
- P1 is a count (`spike/4410/universe.sh`, copied as `universe.sh`).
- The three held PDFs' sha256 were checked before writing (they gate a §7 stop, and the WO asks for that check first): the XT30 V1.2 drawing `c57b5a37…` equals `AMASS_XT30PW_M_V12_SHA256` (`connectors.rs:157`); AN2867 Rev 25 `cc868fd2…` equals the WO's; TLV803 SBVS157E `1645cfe1…` equals the WO's prefix.
- The fixture's `motor_outputs[].esc` text was read: "J5 is a bench header … ESCs … are fed from the pack, not through the board's VBAT copper (Scotty, 2026-11-19, confirmed)".

## The universe

- **P1.** 36 committed current specifications.

## Sentinel-03's baseline (host, then the image)

- **P2.** `validate`, `solve` and `emit` pass. `emit` prints the conditions `power` names today, at least: each buck's "junction is screened, not guaranteed" (U10, U11); the hot-plug "ring's size is not computed" line on J4; the typical-only isat screen and the DC-bias "typical, not guaranteed" cin line for each buck; the "not in its load" line (A13) for at least one device.
- **P3.** `extract` passes, and judges no supply (`drop_limits` absent): no supply gate line at all.
- **P4.** `verify` (image) runs every gate, and no precondition is met only because a gate did not run: today's preconditions are the four of `verify.rs:206-223`, none of which reads a key's presence. I predict verify **passes** apart from campaign coverage, low confidence; a hwdef is not one of its gates.
- **P5.** `generate hwdef` on Sentinel-03 is refused `ImuOrientationUndeclared` (U5): `4410` D3 left both keys to this WO.

## The key-deletion sweep (per gate)

- **P6.** Deleting each of these turns a gate off or relaxes it with `validate` still passing:
  - `power.drop_limits` — already absent (supply gate off);
  - `board.stackup.fab_tolerances` — the via-ring rule off (`RingRule::of` → `None`);
  - `power.input.hot_plug` — the hot-plug line gone (an empty list may also be accepted);
  - `power.regulators[].junction` — the thermal screen off, if the key is optional;
  - `power.conditions.dc_bias_derating` — the effective-capacitance screen off or worse;
  - `connectors[].pins[].offboard` — `extract`'s off-board term gone;
  - `net_constraints` — `extract`'s bounds gone (empty is valid by `RUL-14`);
  - `power.sense`, `power.crossings`, `test_points` — conservative (they remove parts), not gates;
  - `board.max_outline_um` — the outline bound off.
- **P7.** Deleting `mcu.reset_switch`, `acknowledge_unwitnessed` or `accept_lifecycle` makes the board stricter or adds a part, never looser: not on the list.
- **P8.** At least one deletion I have not listed relaxes a gate. Low confidence on which.

## Thermal copper (A4)

- **P9.** On the drawn board, the copper joined to U10's and U11's thermal pads is far under the declared 20 cm² per layer: under 2 cm² on F.Cu and nothing joined on B.Cu beyond vias, the plane layers giving In1.Cu (GND) its whole pour if the pad is GND.
- **P10.** TI's θJA in SNVSAN3F is the JEDEC figure for the DDA package (about 42 °C/W on a JEDEC high-K board); the "43 °C/W" of the sub-reviewer is near it but from a different table or board. Low confidence.

## The XT30 drawing (A1)

- **P11.** The V1.2 sheet's drawing shows the mating opening on the side away from the solder tails, i.e. at −y in the KiCad footprint, which `place_connectors` puts inside the board. So J4 opens into the board today, and the fix rotates it 180°. No §7 stop.

## AN2867 Rev 25 (the crystal)

- **P12.** The ABM8AIG-16 row passes Rev 25: gain margin above 5 (Rev 25 still asks 5), CL 18 pF met by 18 pF + 18 pF caps with a stray term, ESR at most 50 Ω at 16 MHz. No §7 stop.
- **P13.** Rev 25 differs from Rev 10 in section numbering at least (`§3.3`, `§3.4` cited in `clock.rs` move), and it states `gm_crit` with the same formula. Low confidence on whether the stray-capacitance guidance changed.

## Fabricator

- **P14.** No one fabricator holds every bound today: the dielectrics and finished thickness cite OSH Park, `fab_tolerances` JLCPCB, `copper_max_um` the engine's own condition and permittivity IPC-4101. JLCPCB publishes a 4-layer stackup (JLC04161H-7628 and similar) with dielectric thicknesses, hole and ring tolerances and copper weights, so **JLCPCB can source every bound** except the permittivity maximum (it publishes a nominal Dk). OSH Park publishes no hole or ring tolerance.

## Input protection (10)

- **P15.** No ESC current flows through Sentinel-03's copper: J4's VBAT reaches only the two bucks' inputs, the sense divider and C1; J5 carries no VBAT. So the whole board is the flight controller's own supply branch, and the fuse and FET sit between J4 and that branch.
- **P16.** The branch's maximum input current, from the power tree's numbers, is under 2.5 A: the 5 V rail's 2 A at 90 % plus the 3.3 V rail's ~0.1 A, at the 3S floor of 9 V, about 1.3 A; so a fuse near 2.5–3 A.

## Which bytes move, by board

- **P17.** Sentinel-03's topology and board; every committed specification's version literal (36 files, by the bump); `fab/sentinel-01-bench/package/MANIFEST.json`'s hash lines. The ELF moves too, low confidence: its provenance names the specification's hash, and the version literal moves that hash. No file `ORDERED.sha256` lists moves (they are Gerbers and the drill file, which carry no version). No other board's pin or copper moves.
