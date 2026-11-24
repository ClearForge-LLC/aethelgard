# AET-WO-4411 — STOP after §1.1 (and §7's third condition)

2026-11-24, the 4411 builder, from main `fa12abf4`. Evidence: `RESULTS.md` (each prediction scored), `FACTS.md` (each datasheet fact cited by page, spot-checked), `out/`, `tools/`. Nothing outside `spike/4411/` is changed.

**Two stops fire together.**
- **§1.1, as planned:** the measurement is done, and each decision below is Scotty's or the architect's.
- **§7, third condition: the module datasheet does not dimension the base-board keep-out.**
  - Each datasheet dimensions only the antenna area on the module (18 × 6 mm for the C3, p. 37; 18 × 6.19 mm for the 32E, p. 43).
  - Each refers the reader to Espressif's *Hardware Design Guidelines*, §1.4.8 "Positioning a Module on a Base Board". **That is the document that dimensions it** (HDG-C3 Fig. 21; HDG-32 Fig. 25).
  - The guidelines are published only as a rolling "Release master" build. The copies fetched today are held by sha256 (`RESULTS.md`) and staged in `~/.cache/aet-held/incoming/aet-4411/`. The same URL will serve other bytes later.

## The answers §1.1 asks for

| Question | Answer |
|---|---|
| **Module** | **Lean: ESP32-C3-WROOM-02** (PCB antenna). Its u.FL variant is **ESP32-C3-WROOM-02U**, in the same datasheet (v1.7, sha256 `622db737…cfdabe`); KiCad 10.0.7 ships a footprint and a symbol for each. Against the ESP32-WROOM-32E/32UE (v2.1, `4c7a345d…f8b8`): it is smaller (18 × 20 mm against 18 × 25.5 mm), has three straps against five, no voltage-setting strap (the 32E's IO12 must be low at power-up, p. 39), and a lower TX peak (345 mA against 379 mA). Both modules need a 0.5 A supply. Placed on the board, the C3 needs the larger clearance (15 mm on three sides against the ESP32's corner); hung off the edge (D2), the two need the same. |
| **Facts** | `FACTS.md`, summarised in the key below. |
| **ArduPilot** | **The hwdef declares it today; no defaults file and no run-time MAVLink are needed.** Sentinel-03's `MAVLINK` port (`role: mavlink`) emits `define DEFAULT_SERIAL2_PROTOCOL SerialProtocol_MAVLink2` and `DEFAULT_SERIAL2_BAUD 57600`. `check pinout --hwdef-h` judges both from the ELF's `var_info`: a planted role is refused as `WrongSerialProtocol SERIAL2`, and a planted baud as `WrongSerialBaud SERIAL2 [115200 \| 57600]`, with no rebuild. The module's own end of the link (its firmware's protocol and baud) is not judged. |
| **Board** | **Lean: a derived `examples/sentinel-03-wifi.json`** (D6). |

## Decisions to rule

- **D1, the module.** (a) ESP32-C3-WROOM-02, with 02U named **[lean]**; or (b) ESP32-WROOM-32E, with 32UE named.

- **D2, where the antenna goes.** The guidelines *suggest*; they do not require.
  - **(a) Hung off the edge [lean].** The antenna area lies wholly outside the outline, with the module at a board corner and its feed side at the side edge (HDG Fig. 19/23, the two ✓ positions).
    - Measured: both KiCad zones end exactly at the antenna area's inner edge, so with this placement **no keep-out lies on the board**, no plane is carved, and KiCad reports nothing for the zone or the courtyard crossing the edge.
    - Cost: the placer keeps every courtyard `MARGIN_NM` (2 mm) inside the edge (`pcb.rs:58–67`), so this one part needs a named exception there.
    - The DRC keep-out check then bites only when the module is misplaced inward, which is exactly the planted "module away from the edge".
  - **(b) On the board with the board cut away.** This cannot be drawn: `route::Board.outline` is a `Rect`.
  - **(c) On the board, uncut**, inside a 15 mm keep-out. Espressif marks it not recommended. KiCad carves both planes under the antenna (measured), so `extract`'s near-plane bound and the supply drop bound would need to learn a carved plane.

- **D3, the held source for the edge rule.** Accept HDG-C3 §1.4.8 (Fig. 19 and Fig. 21), held by the sha256 fetched 2026-11-24, as the cited source. Say where held copies live in the repo, if anywhere (they are staged in `~/.cache/aet-held/incoming/aet-4411/`).

- **D4, the footprint. KiCad's cannot be vendored byte for byte and pass. Measured, four reasons:**
  1. The keepout zone lists all 32 copper layers, and the gate refuses `In3.Cu` on a 4-layer board (`MalformedBoard`).
  2. `route/read.rs:325` refuses any keepout with a `layers` list and models only F.Cu and B.Cu.
  3. Its 12 thermal vias are 0.6 mm pads with **0.2 mm drills**, against the engine's 0.6 / 0.3 mm via. All 12 fail the 0.3 mm minimum hole.
  4. The C3's zone is 5 mm, against the guidelines' 15 mm.

  The options:
  - **(a) Our own footprint [lean],** under `data/aethelgard/`, derived from KiCad's (so its licence stays CC-BY-SA 4.0, stated as `4220b`'s README states its own):
    - KiCad's pads and courtyard;
    - no in-footprint vias: the engine emits the ground pad's via array at its own via, as `1309` did for the buck;
    - the antenna area drawn as single-layer keepout zones, as `4220b`'s sensors are.
  - **(b) Vendor KiCad's byte for byte and teach the readers multi-layer zones.** Reason 3 still fails, so (b) alone cannot pass.

  **Either way, the gate judges the antenna area and edge rule from cited part data, never from the footprint's own zone** (`INV-07`: a contributor's 5 mm would otherwise become the pass criterion). The symbol, KiCad's `RF_Module:ESP32-C3-WROOM-02`, is vendored byte for byte; it is not yet run through ERC.

- **D5, what the supply gate counts.** The datasheet gives two figures:
  - **500 mA**, "current delivered by external power supply, min" (p. 21, Table 6-2), a recommended operating condition;
  - **345 mA** peak TX (p. 22, Table 6-4), one measurement at 3.3 V and 25 °C with 100 % duty.

  **Lean: count 500 mA on the module's rail** (the stated requirement, which bounds the peak), and carry 345 mA as `tx_peak_milliamps_typical`, never as a bound (`RUL-19`). A specification cannot declare less: the figure comes from the part row, not the key. That closes the adversarial "average declared where the peak is given".

  Measured on 4300k's Sentinel-03 (specification 30, its binary at `87b613f6`): `validate` passes with +500 mA on `VDD_3V3`. On main's fixture, U11 has 2,253 mA spare. **The 110 mV `VDD_3V3` drop limit is judged on the routed board by `verify` and was not run.**

- **D6, which board carries it.**
  - **(a) A derived `examples/sentinel-03-wifi.json` [lean].**
    - Sentinel-03 is untouched, so 4300k's order is not delayed.
    - It depends on #390 merging, which brings specification 30 and `examples/sentinel-03.json` to main. This build waits for it, and bumps to 31.
    - Cost:
      - one example, plus its goldens;
      - `emit` takes 15 s on Furnace (main's S03; RSS 0.55 GB);
      - ERC and DRC in tier 2, a few minutes;
      - a package job only if ruled. If it is, about the size of `bench-package-s03`: 37.7 min on CI before its 45-min cancel, 90-min limit, 937 s on Furnace (4300k's figures), on every code pull request.
  - **(b) Sentinel-03 itself.**
    - Every S03 artifact moves.
    - `bench-package-s03`'s `--against-fab` is red on the next code PR until a dispatch rewrites the package, and 4300k's readiness, FAB-NOTES and ORDER must be redone before the order.
    - The 110 mV drop limit has to be re-judged with the module's 500 mA.

- **D7, which port.**
  - **(a) The module takes `MAVLINK`, and J7 (DS-009 TELEM) goes [lean].** Keeping both would put two drivers on `MAVLINK_RX`. `serial_ports` does not change, so the hwdef and the ArduPilot ELF should be byte-identical to S03's (to be shown in the build).
  - **(b) A fourth port, say `WIFI`, keeping J7 for a companion computer.** This adds a `SERIAL3` line, moves the hwdef, and needs an ArduPilot build of the new board. `ardupilot-image.yml` is outside this fence.

- **D8, lifecycle.** None of the four documents states a lifecycle. The parts table's lifecycle needs a held source: an Espressif product-status export, as Scotty exported Winbond's. Until one is held, the row has no lifecycle, and a check that needs one refuses by name (`INV-06`).

## The key, as proposed (specification 31, after #390)

```json
"radio_modules": [{
  "reference": "U20",
  "part": "ESP32-C3-WROOM-02-N4",
  "serial_port": "MAVLINK",
  "supply": "VDD_3V3",
  "enable": { "pull": { "reference": "R30", "part": "…", "ohms": 10000, "to": "VDD_3V3" },
              "delay": { "reference": "C30", "part": "…" } },
  "straps": { "GPIO2": { "resistor": { "reference": "R31", "part": "…", "ohms": 10000, "to": "VDD_3V3" } },
              "GPIO8": { "resistor": { "reference": "R32", "part": "…", "ohms": 10000, "to": "VDD_3V3" } },
              "GPIO9": "open" },
  "decoupling": [ { "reference": "C31", "part": "…" }, { "reference": "C32", "part": "…" } ],
  "antenna": { "corner": "top_right" }
}]
```

- **`part`** names a new busless row kind in the parts table, `radio_module`. It reuses `pin_parts`' pin kinds (supply, ground, enable input, strap) and adds two:
  - `uart_tx` / `uart_rx`, with the module's levels;
  - module data, each value cited:
    - supply range 3.0–3.6 V (p. 21);
    - supply requirement 500 mA (D5);
    - `tx_peak_milliamps_typical` 345;
    - EN: VIL_nRST ≤ 0.25·VDD (p. 22); tSTBL and tRST ≥ 50 µs (p. 15, Table 4-6); RC 10 kΩ / 1 µF advised (p. 33);
    - each strap's internal pull and its SPI-boot level:
      - GPIO2 = 1, floating by default, and the datasheet recommends a pull-up;
      - GPIO8 any;
      - GPIO9 = 1, with a weak internal pull-up;
      - hold time tH 3 ms;
    - U0TXD on pin 12 (GPIO21) and U0RXD on pin 11 (GPIO20), with their VIH/VIL/VOH/VOL (p. 22);
    - the antenna area's rectangle in footprint coordinates, and the edge rule (D2, D3);
    - the footprint and the symbol;
    - `firmware: not judged`.
- **`straps`** maps each strap pin to `open` (the module's own pull holds it) or to a tie or resistor, with the same `Strap` shape `pin_devices` uses. It is judged against the row's required SPI-boot level at reset. A strap the row says floats needs a declared level.
- **Passives the key brings onto the board:** EN's RC, the strap pull-ups, and the decoupling. HDG-C3 §1.3 asks for 10 µF at the module and an ESD diode at the main power entrance; the exact values go into `FACTS.md` before the build.
- **The solver** gives the port its USART pins as it does today. The module's U0TXD goes to the port's RX net and its U0RXD to the port's TX net. The module must sit on the MCU's own domain: 5 V tolerance is stated nowhere.
- **The verdict** gains one line: `U20 ESP32-C3-WROOM-02-N4: firmware not judged; radio not simulated`.

### Refusals and the planted failures (each with a test and a mutant)

| Planted | Refused as |
|---|---|
| A pour or track in the keep-out (the module moved inward) | `check drc`: KiCad's `items_not_allowed`, on any copper layer (measured: F.Cu, In1.Cu, B.Cu, a via), named by the gate as the module's keep-out |
| The module away from the edge | `RadioAntennaOnBoard`: the antenna area is not wholly outside the outline at the declared corner |
| A strap at the wrong level (GPIO9 tied low: download boot) | `RadioStrapWrongLevel`, and `RadioStrapUndriven` for a floating strap left `open` |
| A rail short of the requirement | `RegulatorOverloaded`, counting the row's 500 mA |
| The UART on pins the solver did not give it | The topology's existing `SerialPortMismatch`, plus `RadioPortUndeclared` for a `serial_port` no port declares |
| The module on a domain outside 3.0–3.6 V | `RadioSupplyOutOfRange` |

## What the build would not do

- No antenna or RF judgement, and no module firmware. The datasheet says "UART0 is used to download firmware and log output" (p. 33), so ArduPilot's MAVLink port will receive whatever the module logs at boot. Its content and baud are stated nowhere, and stay unjudged; the verdict line names the firmware as not judged.
- No twin (4400i).
- No long-range radio.
- No second MCU.
