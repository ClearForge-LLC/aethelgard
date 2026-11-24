# AET-WO-4411 §1.1 — results

Measured 2026-11-24 on Furnace from main `fa12abf4`. Predictions are in `PREDICTIONS.md` (committed `77aa762f`, before these runs), and each is scored here. The tools image is `aethelgard-tools@sha256:1e0ab105…` (`container/IMAGE`), run under `~/bin/capped-run` at 8 GiB with swap 0, read back. The raw outputs are in `out/` and the probe scripts in `tools/`. Datasheet facts, each cited by page, are in `FACTS.md`. The builder spot-checked the peaks, straps, UART pins, hold times and both keep-out figures against the rendered pages.

## Held documents (sha256 in `out/sha256.txt`)

| Document | Version | sha256 |
|---|---|---|
| ESP32-WROOM-32E & WROOM-32UE Datasheet | v2.1 (2026-08-05) | `4c7a345d…f8b8` |
| ESP32-C3-WROOM-02 & WROOM-02U Datasheet | v1.7 (2026-03-05) | `622db737…bfe` |
| ESP32 Hardware Design Guidelines (HDG-32) | "Release master", built Nov 12, 2026 | `707e70a9…614` |
| ESP32-C3 Hardware Design Guidelines (HDG-C3) | "Release master", built Nov 12, 2026 | `c67f1e36…32e` |
| KiCad 10.0.7 `RF_Module.pretty/ESP32-C3-WROOM-02.kicad_mod` | tag 10.0.7 | `2eb3ceb2…16b` |
| KiCad 10.0.7 `RF_Module.pretty/ESP32-WROOM-32E.kicad_mod` | tag 10.0.7 | `b0690fed…3d6` |

All were fetched from the vendors' own URLs (`espressif.com/sites/default/files/documentation/…`, `docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/…`, `gitlab.com/kicad/libraries/kicad-footprints/-/raw/10.0.7/…`). **The guidelines are a rolling "master" build.** Their sha256 holds only the copy fetched today, and the URL will serve other bytes later.

## The keep-out: what each document dimensions (WO §7, third stop)

- **Neither module datasheet dimensions the base-board keep-out.**
  - Each draws the module's own antenna area on its land pattern: 18 × 6.19 mm for the WROOM-32E (Fig. 13, p. 43) and 18 × 6 mm for the C3-WROOM-02 (Fig. 11-1, p. 37). Each marks a "Keepout Zone" with dotted lines on its pin layout (p. 10).
  - For the base board, both send the reader to the guidelines: 32E p. 10 and p. 45 §11.2, "please refer to ESP32 Hardware Design Guidelines > Section Positioning a Module on a Base Board"; C3 p. 10 likewise.
  - **The document that dimensions it is the Hardware Design Guidelines, §1.4.8**, held as above.
- **The guidelines' rule (HDG-32 PDF p. 30–32, printed 27–29; HDG-C3 PDF p. 24–26, printed 21–23):**
  - "It is suggested to place the module's on-board PCB antenna outside the base board, and the feed point of the antenna close to the edge of the base board."
  - Fig. 23 (32) and Fig. 19 (C3) mark two positions as strongly recommended (✓). In both, the antenna area lies wholly outside the board's outline and the module sits at a board corner, with its feed side at the side edge.
  - "If the antenna cannot extend beyond the board edge … cut off the base board on both sides of the antenna and below it."
  - **HDG-32 Fig. 25** (rendered; `FACTS.md` §6) draws the clearance area as board cut away:
    - Min 15 mm beside the antenna, on the side away from the feed;
    - Max 2 mm from the module's feed-side edge to the board edge;
    - Max 1 mm at the antenna's end (the builder's reading of the drawing; the label is ambiguous).
  - **HDG-C3 Fig. 21** draws Min 15 mm on the left, on the right and above the antenna, with the board notched.
  - Both: "A clearance of at least 15 mm is recommended in all directions" (the housing).
- **Whether the clearance applies to inner copper:** no document says. The clearance area is drawn as no board at all, which leaves no copper on any layer.

## Scores

| # | Prediction | Result |
|---|---|---|
| P1 | The S03 hwdef carries `DEFAULT_SERIAL2_PROTOCOL SerialProtocol_MAVLink2` and `_BAUD 57600`, as `#define`s in `hwdef.h` | **Right.** See `out/s03-hwdef-serial-lines.txt` (hwdef.dat lines 36–38; hwdef.h lines 52–54), from 4400h's local build of 4300k's Sentinel-03 (ELF `dbf598d8…`). |
| P2 | `check pinout` with `hwdef.h` judges the port and refuses a wrong protocol or baud by name, with no rebuild | **Right.** On 4400h's topology it gives `ok: true` with `identity: judged` (`out/pinout-4400h-topology-pass.json`). With the port's role set to `gps`, rc 1 and `WrongSerialProtocol SERIAL2 [SerialProtocol_GPS (5) \| SerialProtocol_MAVLink2 (2)]`. A hand-edited baud is refused earlier as `SerialPortMismatch`, because the topology recomputes the BRR. So the baud was planted in main's fixture and re-solved. Against the same ELF, the only finding it adds over the baseline is `WrongSerialBaud SERIAL2 [115200 \| 57600]`. The baseline's own `WrongImuRotation` is there because the ELF was built from 4300k's specification, not main's (`out/pinout-main-fixture*.json`). |
| P3 | The module needs nothing on the ArduPilot side | **Right, if it takes an existing `role: mavlink` port.** The protocol and baud are compiled defaults, judged by `check pinout`, so no defaults file is needed (`4406`) and nothing has to arrive over MAVLink at run time. The module's own end of the link (its firmware's protocol and baud) is not judged. |
| P4 | The strict reader admits KiCad's RF_Module footprints, keepout included | **Wrong.** `aethelgard check drc` refuses both, as KiCad ships them, with `MalformedBoard … footprint[1]/zone[1]/layers[1] names the layer "In3.Cu", which is not a canonical name in the board's layer table`. KiCad's zone lists all 32 copper layers; a 4-layer board has 4. Read but not run: past that refusal, `route/read.rs:325` refuses any footprint zone with a `layers` list ("names more than one layer") and models keepouts on `F.Cu` and `B.Cu` only. |
| P5 | KiCad reports a track or via in the keep-out on F.Cu and on an inner layer, and keeps pours out | **Right.** Under `--refill-zones`, each module reports `items_not_allowed` for a track on F.Cu, on In1.Cu and on B.Cu, and for a via (`out/drc-probes.json`). In the refilled boards, no vertex of the F.Cu, In1.Cu or In2.Cu fill lies inside the keep-out: KiCad carves both planes under the antenna. |
| P6 | A keepout past `Edge.Cuts` raises nothing by itself | **Right.** The base boards' keep-outs cross the top edge; no item names them. With the antenna area wholly off the board (the `overhang` boards), KiCad reports nothing about the zone or the courtyard crossing the edge. |
| P7 | KiCad's zone is not Espressif's figure | **Right, and worse for the C3.** WROOM-32E: KiCad's zone is the antenna area plus 15 mm on three sides (48 × 21.19 mm). That matches HDG-32's "15 mm in all directions" sentence but not Fig. 25's corner rule. C3-WROOM-02: KiCad's zone is the antenna area plus **5 mm** (28 × 11 mm, labelled "5 mm" on Cmts.User), weaker than HDG-C3 Fig. 21's Min 15 on three sides. |
| P8 | Adding the module to Sentinel-03 moves every S03 artifact and undoes 4300k's package | **Not run; it follows from 4300k's CI.** `bench-package-s03` runs `verify --against-fab fab/sentinel-03/package` on every code pull request (4300k branch `87b613f6`, ci.yml:593–640). Any change to `examples/sentinel-03.json` moves its specification hash, so that job is red until the package is written again by a dispatch (37.7 min on CI, run 45428821122; 90-min limit). |
| P9 | A derived example leaves every committed board byte-identical except version lines | **Not run** (needs the key). It is the build's acceptance check. |
| P10 | The peak fits U11 with no regulator change | **Right on main's fixture.** `validate` passes with an extra 345, 500 or 2,253 mA load on `VDD_3V3`, and refuses `RegulatorOverloaded U11` at 2,254 mA (3,001 mA against its 3,000 mA, SNVSAN3F p. 6). So U11 has 2,253 mA spare. **The fuse was not judged** by these runs. A load naming an undeclared reference is refused `LoadNamesNoPart`. |
| P11 | The C3-WROOM-02 is the pick: smaller, 3 straps, no VDD_SDIO strap, peak under 400 mA | **Right on each fact; the pick itself is Scotty's (STOP).** 18 × 20 mm against 18 × 25.5 mm. Three straps (GPIO2, GPIO8, GPIO9) against five. No voltage-setting strap. Peak 345 mA (C3 p. 22, Table 6-4) against 379 mA (32E p. 29, Table 16), both at 100 % duty, 3.3 V. **Not predicted:** placed on the board, the C3's clearance is the larger: 15 mm on both sides and above, against the ESP32's 15 mm on one side at a corner. Hung off the edge, the two are the same. |

## Found, not predicted

- **The footprints' thermal vias fail the minimum hole.** Each footprint carries 12 plated through-hole pads in its ground pad: 0.6 mm pad, **0.2 mm drill**. Every probe board reports 12 `drill_out_of_range` against KiCad's default 0.3 mm minimum. The engine's own via is 0.6 / 0.3 mm (`route.rs:71–72`). As KiCad ships it, the footprint cannot pass DRC under the engine's drill.
- **The C3's silkscreen meets the edge.** Both C3 boards report 2 `silk_edge_clearance`; the 32E reports none.
- **The `overhang` placement clears the keep-out from the board.** Both KiCad zones end exactly at the antenna area's inner edge: 32E y = −6.56, C3 y = −7.1, in footprint coordinates. With the antenna area off the board, no part of either zone lies on it, and no plane is carved.
- **Lifecycle is not stated** in any of the four documents (`FACTS.md` §9).
- **The ESP32's strap hold time differs between documents.** tH is 1 ms in the 32E datasheet (p. 14) and 3 ms in HDG-32 (PDF p. 16). The C3 states 3 ms.
- **ROM boot output on U0TXD at reset, and 5 V tolerance, are stated nowhere** in the four documents.
- **Sentinel-03 already declares `MAVLINK`** (`role: mavlink`, 57600 8N1) on J7, a DS-009 TELEM connector. A module taking that port changes no `serial_ports` line, so its hwdef should not move. That is a prediction for the build, not a measurement.
