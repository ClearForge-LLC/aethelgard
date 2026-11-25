# AET-WO-4411 §1.1 — predictions, committed before the runs they predict

Written 2026-11-24 by the 4411 builder, from main `fa12abf4`, before any `aethelgard` run below. Each one is scored in `RESULTS.md`, right or wrong, and never edited after the run.

Already read before writing (so not predicted): KiCad 10.0.7's `RF_Module` library holds footprints and symbols for ESP32-WROOM-32E/32UE and ESP32-C3-WROOM-02/02U, each PCB-antenna footprint with one keepout zone; both module datasheets send the base-board keep-out to Espressif's *Hardware Design Guidelines* ("Positioning a Module on a Base Board"); `crates/aethelgard-hwdef` emits `define DEFAULT_SERIALn_PROTOCOL` and `_BAUD` per `serial_ports` role; `aethelgard-pincheck`'s identity judge has `WrongSerialProtocol` and `WrongSerialBaud`.

## The ArduPilot port (WO §1.1, third bullet)

- **P1.** Sentinel-03's emitted hwdef (4300k's `examples/sentinel-03.json`, as 4400h built it) carries `define DEFAULT_SERIAL2_PROTOCOL SerialProtocol_MAVLink2` and `define DEFAULT_SERIAL2_BAUD 57600` for the `MAVLINK` port, and the generated `hwdef.h` carries both as `#define`s. So the protocol and baud are compiled defaults; no defaults file is needed (`4406`).
- **P2.** `aethelgard check pinout` on that ELF **with** its `hwdef.h` judges the port: it passes on the declared topology, and refuses by name (`WrongSerialProtocol`) a topology whose `MAVLINK` role is changed to another protocol, and (`WrongSerialBaud`) one whose baud is changed — with no ArduPilot rebuild.
- **P3.** Nothing in the module key is needed on the ArduPilot side: the module is a peer on an existing `role: mavlink` port.

## The footprint under the gate's readers (no emission yet: a hand-placed probe board)

- **P4.** The strict footprint reader admits `RF_Module:ESP32-C3-WROOM-02` and `ESP32-WROOM-32E` as KiCad wrote them, keepout included (the `4300d2` D2 note). If it refuses, the refusal names the zone's 32-layer list (`F.Cu B.Cu In1.Cu … In30.Cu`).
- **P5.** `kicad-cli pcb drc --refill-zones`, under the gate's rules, on a 4-layer probe board with the module and a track through the antenna keep-out on `F.Cu`, reports the track; a via there is reported; a pour there is not filled into the keep-out. A track on `In1.Cu` under the keep-out is reported too (the zone's layer list covers inner layers).
- **P6.** A keepout zone that extends past `Edge.Cuts` raises no DRC item by itself.
- **P7.** KiCad's footprint keep-out is not Espressif's Fig. "Keepout Zone for … Module's Antenna": KiCad draws a box around the antenna on every side; Espressif draws the antenna at a board corner with the board cut away beside it.

## Which board carries it

- **P8.** Adding the module to Sentinel-03 moves every Sentinel-03 artifact: specification hash, solved topology, hwdef, schematic, board, BOM, package and verdict. It undoes 4300k's ready package and its `--fab-out` run (37.7 min on CI).
- **P9.** A derived `examples/sentinel-03-wifi.json` leaves every committed board byte-identical, except where a specification-version bump moves the lines that name the version (the same set every version bump has moved).
- **P10.** The module's transmit peak on `VDD_3V3` fits U11 (LMR33630, 3 A) with no regulator change, and the 7 A input fuse does not move.

## The module

- **P11.** ESP32-C3-WROOM-02 is the pick over ESP32-WROOM-32E: smaller (18 × 20 mm against 18 × 25.5 mm), three strapping pins against five, and a single 3.3 V flash voltage with no VDD_SDIO strap. Its transmit peak is under 400 mA at 3.3 V.

## Lever (iv) prediction, committed before any routing (2026-11-25T04:02:13Z)

A declared VSS escape distance on Sentinel-03 (each U1 VSS pin's nearest GND via within 1.29 mm, the distance measured at pin 31 on the swapped-connector layout, where its bound was about 37 mOhm), with the WIFI port on USART6 PG14/PG9:
- U1's worst VSS bound falls from about 62.5 to about 37 mOhm; the worst VDD bound stays about 54.7 mOhm.
- The drop under ruled 1 (a') is about 0.751 A x (54.7 + 37) + 0.5 A x (30.3 + 37) = 68.9 + 33.7 = about 103 mV, under 110 (pad 19 unbounded: its return charged at U1's worst, then 37).
- U11's F.Cu copper stays at least 700 mm2 (1573 before the rule); the outline stays 85.65 x 96.6 mm.
- Every board that does not declare the rule emits byte for byte as before.

## Lever (1) prediction, directional GND escapes, committed before any routing (2026-11-25T13:08:48Z)

Ruled 2026-11-25T13:05Z. Each of U1's GND escape vias within the declared 1.54 mm of its pad's centre, its antipad (via radius plus the board's clearance) wholly in the half-plane through the pad's centre facing away from the plane's feed (L2 pad 2), and wholly in the half-plane facing away from the centroid of U1's nearest VDD_3V3 pads (no constraint when that centroid is the pad's centre, as at pin 31 between VDD pins 30 and 32). Measured from the 1.54 mm board: pin 31's GND via sat inward at (90.75, 105.75), in front of VDD pin 30, whose own escape was pushed to a shared via 1.80 mm away; pin 30 went to 59.7 mOhm.
- **Worst VDD:** about 54.6 mOhm, pin 121 (the top row, nearest the feed, where the half-plane rule still sends pin 120's via inward). Pin 30 recovers to below it.
- **Worst VSS:** about 43 mOhm, pin 31, its via now on the outward side at about the same distance (42.4 inward on the 1.54 mm board; the In1 path from there to U11 adds well under 1 mOhm at this pitch).
- **Drop under 1 (a'):** 0.751 A x (54.6 + 43) + 0.5 A x (30.3 + 43) = 73.3 + 36.7 = **about 110.0 mV**. Every 1 mOhm of VSS moves it 1.25 mV, so I cannot call pass or fail from this arithmetic: within about 1.5 mV of the limit either way.
- **Risk to routing:** outward vias sit where the neighbouring pins' fanout goes; pins whose neighbours are signals (16, 38, 51, 61, 83, 94, 107, 120, 130) may be refused, or the capacitance ladder may fall a rung. U11's F.Cu stays at least 700 mm2 (1503 on the 1.54 mm board); the outline stays 85.65 x 96.6 mm.
- Pad 19 (the module's return) stays unbounded (its thermal vias are off the gate's grid), so its return is charged at U1's worst VSS, as in the ruling's arithmetic.

### Lever (1), first reading refused at pad 130; the second reading, committed before its run (2026-11-25T13:11:30Z)

- **First reading (run, refused):** the antipad wholly in the half-plane facing away from the feed itself. Pad 130 (the top row, VDD 131 beside it, the feed up and right) is left 3 grid nodes and the emit refuses it: `cannot take U1 pad 130 to a via`. That reading was stricter than the ruling: the path the ruling names runs between the feed and *those* VDD pins, and for the top row it arrives from outside the package, so an inward via does not cut it.
- **What moved pin 30 on the 1.54 mm board (read from its tracks, not predicted):** not pin 31's antipad. Pin 31's GND via took pin 30's own inward escape at (90.75, 105.75), so pin 30 reaches a shared VDD via at (90.5, 107.125) by about 5 mm of 0.2 mm F.Cu track around the pad toes. My 05:06Z "antipads" explanation was wrong for pin 30.
- **Second reading (the one measured):** the pad's long axis has two ends; the end facing the feed is the path's. The via's antipad lies wholly beyond the pad's centre on the other end (left and bottom rows outward, top and right rows inward), and the via is not nearer the centroid of the nearest VDD pads (projection at most 0). Within 1.54 mm.
- **Prediction unchanged:** worst VDD about 54.6 (pin 121), worst VSS about 43 (pin 31, outward), drop about 110 mV, too close to call; the outward vias on the left and bottom rows are the routing risk.

## Option 1 prediction, the module on its own regulator, committed before any emission (2026-11-25T20:14:05Z)

Ruled 2026-11-25T20:10Z. The design measured: a third LMR33630ADDA (U13), fed from the battery like U11, its own output `VDD_3V3_RF` (no plane: In2.Cu stays `VDD_3V3`). Its block copies U11's (R30/R31 105 k / 46.4 k, C30/C63 cin, C31 cin_hf, C32 cboot, C33 cvcc, C34-C37 cout, L3), and U30, C91/C92, R90 and R91 move to the new rail. The part was chosen before measuring:
- **The LDO is out on its datasheet:** the only held LDO family (TLV755P, SBVS320D) is rated 500 mA (p. 4, 5.3), which leaves no margin over the module's 500 mA. From `VDD_5V` it loses (5.237 - 3.3) V x 0.5 A = 0.97 W, at the DBV's θJA of 231.1 °C/W (p. 5, 5.4), a rise of 224 °C, against a junction limit of 125 °C.
- **The LMR33630 is the one buck the library holds with a θJA-versus-copper figure,** which the junction screen needs. At 0.5 A, Figure 9-8 (p. 27) reads 76.7-78.7 % on the 36 V curve, taken as 76 % (U11's convention): P_loss = 3.417 V x 500 mA x (1/0.76 - 1) = 540 mW. At the figure's smallest area, 7 cm² (θJA 42 °C/W), the junction is 85 + 22.7 = 107.7 °C, under 125.
- **Fed from the battery,** not `VDD_5V`. U10's 2000 mA is DS-009's port budget, and taking 0.37 A more from it would re-judge U10. From the battery, U10 does not move, and the module's current reaches the battery through the same fuse as before.

Predictions:
- **U1's drop:** about **88 mV** (0.751 A x (54.7 + 62.5)), the module's term gone, within 80-95 mV if U13's block moves the escapes. It passes 110.
- **The module rail's drop:** the limit is floor(3,144,727 µV) - 3.0 V = **144 mV**.
  - The supply side is tracks, at most about 60 mOhm. Pad 19 stays unbounded, so the return is charged as in 1 (a') at U1's worst VSS, about 62.5 mOhm.
  - Drop at most about 0.5 A x (60 + 62.5) = **61 mV**. It passes.
  - The gate as built refuses the rail by name before judging it: `SupplyTerminalMissing`, since U1 has no pin on `VDD_3V3_RF`. The scratch measurement needs a scratch patch that makes U30 the load.
- **U13:** 540 mW. It needs **at least 700 mm²** per joined layer, the figure's smallest point. U10 and U11 stay at least 700 mm² each (U11 1503 before).
- **U11's loss row:** its load returns to about 751 mA, so U11's row is re-read there (Figure 9-8, about 83 %, about 530 mW). U11's copper need does not change (7 cm², the figure's smallest point).
- **The outline: does not hold.** A third buck block with at least 7 cm² of its own pour grows 85.65 x 96.6 mm by about 10-20 mm on one side.
- **DRC:** 36 `drill_out_of_range` (U13's thermal array adds 12 to U10's and U11's 24), nothing else new; `check drc --spec` passes.
- **Sentinel-01 emits byte for byte as before.**
