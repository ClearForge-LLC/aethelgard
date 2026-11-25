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
