# aethelgard-ir

The specification schema and the solved topology, as two separate types (`RUL-16`), with validation. Every type refuses unknown fields and accepts only a JSON object. Nothing defaults (`INV-06`).

## `Specification`, version 18

Version 2 (`AET-WO-4100`) added `board`. Version 3 (`AET-WO-4300c`) requires a net constraint on every SPI bus's `sck` and `mosi`, whose capacitance and rise time choose the pin's output speed, and names parts as the version-2 parts table does (`W25Q64JVxIQ`). Version 4 (`AET-WO-4300f`) adds `connectors`. Version 5 (`AET-WO-1308`) requires `net_classes` and `diff_pairs`; an empty array declares none. Version 6 (`AET-WO-4300a`) requires `mcu.hse_hz` (the crystal's frequency, `0` for none, the only value admitted until a crystal part exists, `4400` D1) and `motor_outputs`: each a `label` (its net), a `pin` (`null` for the solver), a `protocol` (`pwm`, `dshot300`, `dshot600`) and `pwm` (`{rate_hz, pulse_min_us, pulse_max_us}` for PWM, `null` for DShot). Version 7 (`AET-WO-4300d`) requires `mcu.bor_level` (0 to 3) and `power`, whose value `null` declares no power tree. Version 8 (`AET-WO-4210`) requires `accept_lifecycle`: each `eol` or `nrnd` part the board uses, as `{part, status, statement}` with a non-empty statement; `validate` refuses an unacknowledged one by name (`LifecycleNotAccepted`, `LifecycleStatusMismatch`), and an empty array acknowledges none. Version 9 (`AET-WO-4300b`) requires `serial_ports` (below); an empty array declares none. Version 10 (`AET-WO-4300d3`) adds to `power`: `conditions.dc_bias_derating` (each `{dielectrics, ratio_max_permille, retained_permille, source}`), `input.hot_plug` (each `{reference, part, side}`, `side` `board` or `pack`), and on each regulator `junction` (`{copper_square_centimetres, loss_milliwatts}`, each `{value, source}`; `null` for an LDO) and `controls`, as on each crossing (each `{pin, strap, wire}`, exactly one of `strap` and `wire` `null`; `wire` is `{net, role}`). Version 11 (`AET-WO-4300g`, renumbered from 9) requires `mcu.supply_source`: `{connector, pin}`, the declared connector pin the supply enters by, or `null` when a regulator of `power` outputs the supply. Version 12 (`AET-WO-4230`, renumbered from 10, then from 11, by the same rule) requires `peer_deviation_ppm` and `peer_deviation_source` on every serial port, and `acknowledge_unwitnessed` (below; an empty array acknowledges none). Version 13 (`AET-WO-4220b`) requires `pin_capacitance` on every I2C and SPI device (below). Version 14 (`AET-WO-1312`, renumbered from 13 by the same rule) adds `test_points` (each `{net, reference}`: a pad on a net a connector pin may name, step 15a) and `mcu.reset_switch` (a push-button from `NRST` to `GND`); both may be absent, as that work order states, which declares no test point and no switch, and neither is written back when empty or `false`. Version 15 (`AET-WO-1313`) adds `board.max_outline_um` (`{width, height}`, micrometres as the rest of `board`, each 1 to 1 000 000; `OutlineBound` otherwise), optional by the precedent ruled on PR #191 (Q1): absent is today's placer, it is not written back, and an explicit `null` is refused as `test_points` and `mcu.reset_switch` refuse it. A declared bound that cannot hold the parts at all (one keep-out box, or their sum, inside the bound less the placer's 2 mm margin) is refused by name, `OutlineTooSmall`, before any placement: by `emit` always, and by the CLI's `validate` when the specification solves; one that does not solve is left to `solve` and `emit` to refuse by their own names (ruled F3). A bound the placer does not reach is `emit`'s `OutlineUnreachable`. Version 16 (`AET-WO-4400d`, ruled on PR #207) adds `board.forward`, the edge the vehicle's nose points to as the board file draws it seen from F.Cu (`top`, the edge at the least y, `right`, `bottom` or `left`), and an SPI device's `orientation`, `{x, z}`: where the chip's own +X and +Z point in the board frame, each `forward`, `aft`, `right`, `left`, `up` or `down` (Y is Z × X). **The board frame is the engine's constant (ruled D3):** x points to `board.forward`, y to its right, and z down, through F.Cu to B.Cu, so the board is mounted F.Cu up; a board mounted any other way is ArduPilot's `AHRS_ORIENTATION`, a parameter outside this engine. Both keys are optional by the same precedent and an explicit `null` is refused. An undeclared IMU is placed as before and refused by `generate hwdef` (`ImuOrientationUndeclared`); a declared one whose footprint cannot be turned to it is the placer's `ImuRotationUnplaceable`. Version 17 (`AET-WO-4400g` §1.2, Q1 ruled on PR #229) adds `mcu.foreign_clock`, the kernel clocks a foreign firmware programs, each `{value, source}` in Hz: `apb1_timers_hz` and `apb2_timers_hz` (RM0433 Rev 9 Table 57's `rcc_timx_ker_ck` and `rcc_timy_ker_ck`) and `usarts`, one entry per USART the platform carries, keyed `USART1`, `USART2`, `USART3`, `UART4`, `UART5`, `USART6`, `UART7` and `UART8`. It is optional by the same precedent and an explicit `null` is refused. `simulate --foreign` refuses a specification without it by name, and a run of the generated firmware refuses one with it (`INV-06`); the twin builds every timer and USART at it, and the foreign judge checks the timers against the RCC registers at the run's end. Version 18 (`AET-WO-2304b` §1.2) requires `power.battery.series_milliohms`, the healthy battery path's series resistance (the pack, its leads, the input connector and the `VBAT` copper) as `{value, source}` in mΩ with a non-zero value, or `null` (undeclared), and `esc` on every motor output: `{"none": {"source"}}` (no ESC on the board's path, and why) or `null` (undeclared). Both keys are required, because their absence would relax the battery-sag check; `validate` judges nothing else with either, and a battery sag halts by name on a `null` (`SeriesResistanceUndeclared`, `EscUndeclared`). A file of another version is refused by name (`UnsupportedSchemaVersion`); nothing supplies a board, a declaration, a connector list, a class list or a motor output list for it. Every key is required, but `test_points` and `mcu.reset_switch` (version 14), `board.max_outline_um` (version 15), `board.forward` and an SPI device's `orientation` (version 16), and `mcu.foreign_clock` (version 17). `peripheral`, `pin`, `impedance_ohms` and a signal layer's `net` may be `null`, but the key may not be missing. Arrays keep the order they were written in; nothing is sorted.

```json
{
  "schema_version": 18,
  "project_name": "sentinel-01",
  "mcu": { "part": "STM32H753ZITx", "supply": "VDD_3V3", "hse_hz": 0, "bor_level": 3,
           "supply_source": { "connector": "J2", "pin": "1" } },
  "voltage_domains": [{ "id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50 }],
  "i2c_buses": [{
    "id": "SENSORS", "peripheral": null, "speed_hz": 400000, "pull_up_ohms": 2200, "domain": "VDD_3V3",
    "sda": { "net": "I2C_SDA", "pin": null }, "scl": { "net": "I2C_SCL", "pin": null }
  }],
  "spi_buses": [{
    "id": "FLASH", "peripheral": null, "speed_hz": 20000000, "domain": "VDD_3V3",
    "sck": { "net": "SPI_SCK", "pin": null }, "mosi": { "net": "SPI_MOSI", "pin": null },
    "miso": { "net": "SPI_MISO", "pin": null }
  }],
  "i2c_devices": [
    { "reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 118,
      "pin_capacitance": null },
    { "reference": "U3", "part": "TMP117", "supply": "VDD_3V3", "bus": "SENSORS", "address": 72,
      "pin_capacitance": null }
  ],
  "spi_devices": [{ "reference": "U4", "part": "W25Q64JVxIQ", "supply": "VDD_3V3", "bus": "FLASH",
    "cs": { "net": "FLASH_CS", "pin": null }, "pin_capacitance": null }],
  "connectors": [
    { "reference": "J1", "part": "SM06B-SRSS-TB", "standard": null, "pins": [
      { "pin": "1", "net": "VDD_3V3", "offboard": null }, { "pin": "2", "net": null, "offboard": null },
      { "pin": "3", "net": null, "offboard": null }, { "pin": "4", "net": "SWDIO", "offboard": null },
      { "pin": "5", "net": "SWCLK", "offboard": null }, { "pin": "6", "net": "GND", "offboard": null }] },
    { "reference": "J2", "part": "SM04B-GHS-TB", "standard": null, "pins": [
      { "pin": "1", "net": "VDD_3V3", "offboard": null }, { "pin": "2", "net": null, "offboard": null },
      { "pin": "3", "net": null, "offboard": null }, { "pin": "4", "net": "GND", "offboard": null }] }
  ],
  "net_constraints": [
    { "net_name": "I2C_SDA", "max_capacitance_femtofarads": 100000,
      "max_rise_time_nanoseconds": 300, "impedance_ohms": null },
    { "net_name": "I2C_SCL", "max_capacitance_femtofarads": 100000,
      "max_rise_time_nanoseconds": 300, "impedance_ohms": null },
    { "net_name": "SPI_SCK", "max_capacitance_femtofarads": 30000,
      "max_rise_time_nanoseconds": 20, "impedance_ohms": null },
    { "net_name": "SPI_MOSI", "max_capacitance_femtofarads": 30000,
      "max_rise_time_nanoseconds": 20, "impedance_ohms": null }
  ],
  "board": {
    "copper_layers": 2,
    "layers": [
      { "name": "F.Cu", "role": "signal", "net": null },
      { "name": "B.Cu", "role": "signal", "net": null }
    ],
    "stackup": {
      "finished_thickness_um": { "nominal": 1600, "min": 340, "max": 2000, "source": "…" },
      "dielectrics": [{ "between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "…" }],
      "copper_max_um": { "value": 70, "source": "…" },
      "relative_permittivity_max_permille": { "value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz" }
    },
    "rules": { "clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600 }
  },
  "net_classes": [
    { "name": "power", "track_width_um": 250, "via": { "diameter_um": 700, "drill_um": 300 }, "nets": ["VDD_3V3", "GND"] }
  ],
  "diff_pairs": [
    { "name": "USB", "nets": ["SPI_SCK", "SPI_MOSI"], "gap_um": 300, "width_um": 200 }
  ],
  "power": null
}
```

Every list except `voltage_domains` may be empty.

**`connectors`** (`AET-WO-4300f`, ruled on PR #134). Where a net leaves the board. Each connector names its `reference`, its `part` (a row of `connectors::PARTS`: JST GH `SM04B-GHS-TB`, `SM06B-GHS-TB`, `SM10B-GHS-TB` and JST SH `SM06B-SRSS-TB`, each cited by its catalogue's sha256), its `standard` (a row of `ds009::PORTS` by name, or `null` for an explicit map), and every signal pin, `1` to N, once:

```json
{ "reference": "J1", "part": "SM04B-GHS-TB", "standard": "DS-009 I2C", "pins": [
  { "pin": "1", "net": "VDD_5V", "offboard": null },
  { "pin": "2", "net": "I2C_SCL", "offboard": { "capacitance_femtofarads": 30000, "source": "condition: …" } },
  { "pin": "3", "net": "I2C_SDA", "offboard": { "capacitance_femtofarads": 30000, "source": "condition: …" } },
  { "pin": "4", "net": "GND", "offboard": null } ] }
```

- A pin's `net` is a declared signal net, a voltage domain or `GND`, or `null` for a pin left unconnected. A connector cannot invent a net.
- A pin on an I2C or SPI line (a bus line or a chip select) declares its `offboard` capacitance, with its source: the mated contact, the cable and the remote pins, at most. Extraction adds it to the board's copper on every line it bounds (`RUL-19` (1)). Any other pin's `offboard` is `null`.
- The mounting pads are not declared. The emitter puts them on `GND`.

Each I2C and SPI device carries `pin_capacitance` (version 13, `AET-WO-4220b`): `null`, or a declaration for a part whose datasheet prints its pin capacitance as a typical only, or not at all (the `AET-WO-4220` D10 ruling):

```json
"pin_capacitance": { "declared": { "femtofarads": 10000,
  "condition": "each pin at most TDK's typical C_I of < 10 pF, which no maximum bounds",
  "source": "TDK DS-000347 v1.9 Table 4, p. 14: C_I < 10 pF in the TYP column" } }
```

- `extract` counts the declared value for each of the device's pins on every line it bounds: an SPI device's on SCK and MOSI, an I2C device's on SDA and SCL in place of UM10204's `Ci`, never below it (`PinCapacitanceBelowI2cCi`). The verdict lists each such pin as declared, with its condition and source, and states that the bound holds only under it; a declared value is never named a maximum (`RUL-19`).
- A part whose parts-table row cites a maximum is counted at that maximum (`AET-WO-4300c` D3), and a declaration for it is refused. A declaration below the typical the part's row cites is refused on any bus (`PinCapacitanceBelowTypical`): the ICM-42688-P's and ICM-20948's C_I "< 10" pF TYP, held as 10 pF, and the TMP117's 4 pF. An SPI device whose part cites none and declares none is refused by `extract` (`UnboundedPinCapacitance`); an I2C device without one is counted at UM10204's `Ci`, as before.
- A `standard` holds the connector to DS-009's pinout for that port. DS-009 rev 0.1.0 is cited by its only commit `48dd6d1d` and its sha256, never vendored, and its mark is used nowhere (ruled D2, provisionally). The ports are `DS-009 TELEM`, `DS-009 GPS`, `DS-009 GPS FULL`, `DS-009 I2C` and `DS-009 DEBUG MINI`.
  - A supply pin must be a domain of the voltage DS-009 states: +5 V for the external ports (ruled D5), and the microcontroller's supply for the debug port.
  - A `TX (OUT)` pin takes a serial port's `tx` net and an `RX (IN)` pin its `rx` net, both of one port: DS-009's tables are the autopilot's side, and the peripheral crosses the lines (Telemetry Port). So `DS-009 TELEM`, `DS-009 GPS` (whose I2C pins take one bus's lines) and `DS-009 I2C` validate.
  - A role this schema cannot yet declare (a safety switch, a LED, a buzzer, SWD) takes no net, so `DS-009 GPS FULL` and `DS-009 DEBUG MINI` are refused until the work order that declares it. CTS and RTS may be `null`, as DS-009 lets them float.
  - Everything that judges a standard is `ds009.rs`. Explicit maps only (ruling (c)) would be that module, the field and its four refusals deleted.

A connector is in the `Specification` only. Nothing solves it, so the twin, which reads only the `SolvedTopology`, never sees one.

**`power`** (`AET-WO-4300d`, ruled on PR #137). The battery, the power-in connector (`connectors::POWER_IN`), each regulator with its feedback divider and external parts, declared loads, the battery sense and the bus crossings. Each part is a row of the power parts table, `data/power-parts.json`, read by `power_parts::parse_power_parts`; every value there is cited to a page of a document held by sha256. The shape and the order of the checks are `power.rs`'s module documentation. Every refusal is `IrValidationError::Power`, whose `kind()` names the check. A specification that declares a tree is refused by the emitter (`PowerNotEmitted`) until the emission work order lands.
**`serial_ports`** (`AET-WO-4300b`). Each a USART or UART with a TX and an RX line, the solver choosing what is `null`:

```json
{ "label": "CONSOLE", "peripheral": null,
  "tx": { "net": "CONSOLE_TX", "pin": null }, "rx": { "net": "CONSOLE_RX", "pin": null },
  "role": "console",
  "framing": { "baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false },
  "negotiated": null, "peer_deviation_ppm": 5000, "peer_deviation_source": "declared" }
```

- `peripheral` is `null` or one of `serial::PERIPHERALS` exactly as written: `USART1`, `USART2`, `USART3`, `UART4`, `UART5`, `USART6`, `UART7`, `UART8`. `LPUART1`, in any case, is refused by its own name: its enable bit is in `RCC_APB4ENR`, which the generated firmware does not write, and its baud generator is not a USART's.
- `role` is `console`, `mavlink`, `gps`, `rc_crsf` or `rc_sbus`. The framing is declared, never inferred from it. `rc_crsf` must be 8N1, not inverted, at 416 666 baud or above (TBS's CRSF specification, dual-wire, held by sha256, `serial::CRSF_SOURCE`), and at a baud of the cited set `serial::CRSF_BAUDS` (ruled on PR #152): 416 666 (TBS) or 420 000 (the default of ExpressLRS receivers and Betaflight, `serial::CRSF_ELRS_SOURCE`). Any other baud is refused (432 000 among them) unless the port declares it negotiated. SBUS has no vendor-published source, so nothing is refused on its behalf.
- `negotiated` is required: `null`, or `{ "source": "…" }` on an `rc_crsf` port whose baud is a rate the two ends negotiate (CRSF frame 0x70, "CRSF Protocol Speed Proposal"), with a source for the rate held to the stackup sources' rule (1 to 200 bytes of printable ASCII, no space at either end). It is refused on any other role (`SerialNegotiatedOffCrsf`), with any other source, empty included (`InvalidSource`, naming `serial_ports[i].negotiated.source`, never the value), and at a baud of `serial::CRSF_BAUDS` (`SerialNegotiatedAtCitedBaud`). `solve` copies it to the solved port.
- `peer_deviation_ppm` is required (`u32`): the deviation of the remote transmitter's baud clock from the declared baud, a declaration. The check for silicon (`serial::check_silicon`) sums the quantization error, HSI's worst-case deviation in the direction that adds (DS12117 Rev 10 Table 36 rev Y / Table 133 rev V: +14,984 / −25,894 ppm over voltage and junction temperature −40 °C to TJmax) and this, and requires the sum strictly below RM0433 Rev 9's Table 406/407 tolerance; `solve` refuses a port that fails it (`SerialBaudTolerance`, naming each term), and the finding (`SiliconFraming`) reports them. An `rc_crsf` port declares at least `serial::crsf_cited_spread_ppm()`, the spread of the two bauds `CRSF_BAUDS` cites (8,002 ppm); a lower figure is refused (`PeerDeviationBelowCited`). The fixtures declare 5,000 ppm for a console, MAVLink, GPS and SBUS peer, a declaration with no datasheet behind it.
- `peer_deviation_source` is required: `cited` or `declared` (`AET-WO-4230` R2, the architect's ruling on Q-A). `cited` stands only where the repository holds the citation: an `rc_crsf` port whose `peer_deviation_ppm` is exactly the cited spread; any other `cited` is refused (`PeerDeviationNotCited`). Every other figure is `declared`, and `verify`'s verdict names it.
- `framing`: `baud` above 0; `data_bits` 7 to 9, and with a parity bit a word of 7 to 9 bits (`USART_CR1.M`); `parity` `none`, `even` or `odd`; `stop_bits` 1 or 2; `inverted` sets both `TXINV` and `RXINV`.
- The TX and RX nets are signal nets like any other: each ends at a connector pin, which declares no `offboard` (D8 is I2C and SPI only), and takes no net constraint (no check reads one).

**`acknowledge_unwitnessed`** (`AET-WO-4230` R1, the architect's ruling on PR #161, comment 6911424608). Each register's fields that the witness ledger classes `ignored-by-model`, `same-source` or `silicon-only`, acknowledged per register and class:

```json
{ "register": "SPI1.CFG2", "class": "ignored-by-model", "fields": ["CPHA", "CPOL"],
  "reason": "Renode stores neither (a tag): the bench reads them back" }
```

`register` is the ledger's name for it (a peripheral and a register joined by one `.`, each an upper-case letter then upper-case letters, digits or `_`); `class` one of the three; `fields` the exact, non-empty list without repeats; `reason` held to the stackup sources' rule (1 to 200 bytes of printable ASCII, no space at either end). At most one entry per register and class. `validate` judges only this shape; the CLI's `validate` refuses an entry naming a field the ledger lacks for that register and class (`UnwitnessedAcknowledgementStale`), and `verify --fab-out` a ledger row of those classes no entry names (`UnwitnessedFieldNotAcknowledged`). `Specification::unwitnessed_acknowledgement(register, class)` reads an entry, typed by `UnwitnessedClass`.

**`net_classes`, `diff_pairs`** (`AET-WO-1308` §1.1, §1.3; FEEDBACK F2 to F4). Micrometres. A class gives its nets a track width and, optionally, a via (`via` is a required key, `null` for the rules' via); a pair gives two nets, `[positive, negative]`, a gap and a width. Every class's clearance is `board.rules.clearance_um`, so none is declared (F2).

**`board`** (`AET-WO-4100`, ruled on PR #120). Micrometres and thousandths. A stackup value is a bound with its source, never a nominal: a minimum dielectric is its source's value less a published tolerance, rounded down; a maximum is rounded up. The engine builds 2 copper layers (both signal) or 4 (`F.Cu`, `In1.Cu`, `In2.Cu`, `B.Cu`: outer layers signal, both inner layers planes, exactly one `GND` and the other a declared domain), through vias only. The rules are honoured: clearance and track width at or above the engine floor (0.2 mm), and the one via the engine builds (0.3 mm drill, 0.6 mm). See `crates/aethelgard-cli/tests/fixtures/four-layer.json` for a 4-layer board on OSH Park's published stackup.

## `Specification::validate`

Returns the first refusal, in this order, and in document order within a step. `id()` is what the refusal names; `InvalidIdentifier` names nothing there and carries the JSON path instead (`path()`).

| # | Rule | Refusal (`id()`) |
|---|---|---|
| 1 | `schema_version` is 18 | `UnsupportedSchemaVersion` (none) |
| 2 | Every identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E`, none of them `/`, `{`, `}`, `$`, `[`, `]`, `"`, `%`, `*`, `,` or `\`, and none holding a date shape (`dddd-dd-dd`) or a time shape (`dd:dd:dd`). The project name is also a crate name (1 to 58 bytes of `a-z`, `0-9`, `-`; a letter first, no trailing or doubled `-`; 58 because `generate` adds `-board`) whose stem is no Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`, in any case: `generate` names files after it, `AET-WO-1102` A7, `AET-WO-1206`), and a device or connector reference a Renode identifier (a letter, then letters, digits and `_`). One function, `check_identifier(IdentifierKind, &str)`, holds the table, and the error's message names the rule broken (`RUL-14`; `AET-WO-1102`) | `InvalidIdentifier` (none; `path()`) |
| 2a | `mcu.hse_hz` is 0: a crystal needs a crystal part, which no table holds (`4400` D1) | `CrystalNotSupported` (none) |
| 2b | `mcu.bor_level` is 0 to 3 | `BorLevelOutOfRange` (none) |
| 2c | where `mcu.foreign_clock` is declared (`AET-WO-4400g`): each `source` 1 to 200 bytes of printable ASCII, no space at either end; no value 0; `usarts` names exactly the eight USARTs; and the USARTs one kernel clock mux selects for are declared at one clock (RM0433 Rev 9 §8.7.21: `USART16SEL` for USART1 and USART6, `USART234578SEL` for the other six) | `InvalidSource` (none; `path()`), `ForeignClockZero`, `ForeignClockUsarts`, `ForeignClockMuxSplit` (none) |
| 3 | `voltage_domains` is not empty | `NoVoltageDomain` (none) |
| 4 | Domains: unique ids, nominal voltage not 0, tolerance in 1..=500 | `DuplicateDomainId`, `ZeroNominalVoltage`, `ToleranceOutOfRange` (the id) |
| 5 | `mcu.supply`, every bus `domain` and every device `supply` names a declared domain | `UnknownDomain` (the name) |
| 5a | Every bus `domain` equals `mcu.supply`, and every device `supply` equals its bus's `domain`, or the `b_domain` of a crossing `power` declares on its bus (`AET-WO-4300d` D9). A device naming no bus is left to step 9 | `SupplyMismatch` (the bus id or device reference) |
| 6 | Bus ids are unique across both bus lists | `DuplicateBusId` (the id) |
| 7 | An I2C `speed_hz` is 100000, 400000 or 1000000, and `pull_up_ohms` is not 0. An SPI `speed_hz` is not 0 | `UnsupportedI2cSpeed`, `ZeroPullUp`, `ZeroSpiSpeed` (the bus id) |
| 8 | References are unique across both device lists and the connectors, compared with case folded | `DuplicateReference` (the reference; the message names the first spelling) |
| 9 | A device's `bus` names a bus of its own kind | `UnknownBus` (the name) |
| 10 | An I2C address is in `0x08..=0x77` and unique on its bus | `I2cAddressOutOfRange`, `DuplicateI2cAddress` (the device reference) |
| 11 | A net is declared by exactly one signal across every bus, every `cs`, every serial port's `tx` and `rx`, and every motor output's `label`, and no signal net has a voltage domain's id (compared with case folded), none differs from `GND` only in case, no motor output's `label` and no serial net is `GND` in any case (each leaves the board as a signal), and voltage domain ids are compared the same way (`DuplicateDomainId`). The MCU's debug nets `SWDIO`, `SWCLK` and `NRST` are the engine's (`AET-WO-4300g`): a signal net spelling one in any case is a duplicate, and a domain spelling one shadows it | `DuplicateNetName`, `NetShadowsDomain` (the net) |
| 12 | Net constraints: unique `net_name`; each names a declared signal net or a voltage domain id; capacitance, rise time and impedance are not 0 | `DuplicateNetConstraint`, `UnknownNet`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroImpedance` (the net) |
| 13 | `board`, in this order: `copper_layers` is 2 or 4; `layers` is KiCad's copper names in order; each role fits (outer signal, at 4 both inner planes, a plane names a net, a signal layer none); each plane net is `GND` or a declared domain, one plane per net, exactly one `GND` at 4; `dielectrics` is the adjacent pairs in order, each minimum above 0; `finished_thickness_um` has `0 < min <= nominal <= max <= 10000` and `min` at least the dielectric minima summed (at 2 layers, the dielectric minimum at most `min` less twice `copper_max_um`, which the notes then imply); `copper_max_um` in 1..=10000, permittivity in 1000..=100000; where declared (`AET-WO-1303b`, each optional), `outer_copper_min_um` and `plane_copper_min_um` in 1..=10000 and at most `copper_max_um`, `plating_min_um` in 1..=10000, `copper_temperature_max_c` in 1..=300, a dielectric's `max_thickness_um` in 1..=10000 and at least its minimum, and `fab_tolerances`' `hole_position_um`, `hole_size_plus_um` and `hole_size_minus_um` in 0..=1000, `track_width_permille` in 0..=999 and `via_ring_um` (`AET-WO-1314`) in 0..=1000, each where declared; every `source` 1 to 200 bytes of printable ASCII, no space at either end; clearance and track width at least 200 µm; via drill 300 µm and diameter 600 µm exactly; then each SPI device's `orientation` in document order (version 16): `x` and `z` at right angles, and `board.forward` declared | `ImuAxesNotPerpendicular`, `ForwardUndeclared` (the device reference); `UnsupportedLayerCount` (none), `LayerTable`, `LayerRole`, `DielectricTable`, `StackupValue`, `InvalidSource`, `RuleBelowFloor`, `UnsupportedViaRule` (none; `path()`), `PlaneNet` (the net), `PlaneLayout`, `FinishedThickness` (none) |
| 13a | `net_classes` and `diff_pairs`, classes first, each in file order: a name is an identifier and unique across classes and pairs together; a class names at least one net; a pair names exactly two distinct nets, neither a voltage domain nor `GND`; every net is a declared signal net, a voltage domain id or `GND`, and holds neither `'` nor `?` (the gate names it in KiCad rule expressions, where `'` ends the quote and `?` is a wildcard); a net is in at most one class or pair (a pair is its nets' class); a class width and a pair width are at least the larger of 200 µm and `board.rules.track_width_um`, and at most 10000 µm; a class via has a drill at least `board.rules.via_drill_um`, a pad at most 10000 µm, and a ring `(diameter - drill) / 2` at least the rules' own `(via_diameter_um - via_drill_um) / 2` (150 µm); a pair gap is at least `board.rules.clearance_um` and at most 10000 µm | `InvalidIdentifier` (none; `path()`), `DuplicateClassName`, `ClassNoNets`, `PairNets`, `PairOnRail`, `ClassUnknownNet` (the net), `ClassNetNotLiteral` (the net), `NetInTwoClasses` (the net), `ClassBelowMinimum`, `ClassAboveMaximum` (none; `path()`), `ViaRingUnderRules` (the class) |
| 13b | Each motor output in document order: `pwm` an object for PWM and `null` for DShot; a rate above 0; `1 <= pulse_min_us < pulse_max_us`; and `pulse_max_us * rate_hz < 1 000 000`, the longest pulse inside the period | `MotorTimingShape`, `ZeroMotorRate`, `PulseRange`, `PulseExceedsPeriod` (the label) |
| 13c | Each serial port in document order (`AET-WO-4300b`): its label unique with case folded; its `peripheral` not `LPUART1` in any case, one of `serial::PERIPHERALS` as written, and not an earlier port's; its framing's `baud` above 0, `data_bits` 7 to 9, its word (data and parity) 7 to 9 bits, `stop_bits` 1 or 2 (the field and the rule in the message); a `negotiated` declaration only on `rc_crsf`, with a source by the stackup sources' rule, at a baud `serial::CRSF_BAUDS` does not cite; no contradiction of the role's published framing (`rc_crsf` only; the field, what is required and the citation); an `rc_crsf` port's `peer_deviation_ppm` at least the cited spread; a `peer_deviation_source` of `cited` only on an `rc_crsf` port at exactly the cited spread | `DuplicateSerialLabel`, `SerialPeripheralLpuart`, `UnknownSerialPeripheral`, `SerialPeripheralTwice`, `SerialFramingShape`, `SerialNegotiatedOffCrsf`, `SerialNegotiatedAtCitedBaud`, `SerialFramingContradictsRole`, `PeerDeviationBelowCited`, `PeerDeviationNotCited` (the label); `InvalidSource` (the source's path) |
| 13d | Each `acknowledge_unwitnessed` entry in document order (`AET-WO-4230` R1): its register of the ledger's shape; its class one of the three; its fields not empty and none twice; its reason by the stackup sources' rule; no earlier entry for the same register and class | `UnwitnessedRegisterMalformed`, `UnwitnessedClassUnknown`, `UnwitnessedFieldsEmpty`, `UnwitnessedFieldTwice`, `UnwitnessedAcknowledgedTwice` (the register); `InvalidSource` (`acknowledge_unwitnessed[i].reason`) |
| 14 | First, a net constraint no check reads: on an SPI bus's `miso` or a `cs` (`UnjudgedSpiConstraint`), on a motor output (`UnjudgedMotorConstraint`), on a serial port's `tx` or `rx` (`UnjudgedSerialConstraint`; the net). Then the embedded parts table parses (`PartsTableInvalid`); then each device's part, I2C devices first (`AET-WO-4300c` §1.5): the bare `W25Q64JV` is refused, naming both ordering codes; a part the table holds sits on its own kind of bus, an I2C device at an address its straps offer, an SPI device on a bus clocked no faster than the part, and whose row's `modes` hold the one mode the generated firmware drives (`parts::DRIVEN_SPI_MODE`, mode 0: `svd-hal` writes `CFG2` with `CPOL` and `CPHA` 0; `AET-WO-1207` §1.5). Each message cites the datasheet. A part the table does not hold is refused, I2C devices first, before any limit, by the emitter's kind, its message naming the table's parts (`AET-WO-1206` §1.8, `AET-WO-1316`). Then each declared pin capacitance, I2C devices first (`AET-WO-4220b`): 1 to `parts::MAX_PIN_FEMTOFARADS` (100 000) fF; on an I2C device at least UM10204's `Ci`, `electrical::I2C_PIN_CI_FEMTOFARADS` (10 000 fF), which every undeclared I2C pin is counted at; never for a part whose row cites a maximum; and, on any bus, at least the typical the part's row cites (`parts.json` schema 7, `pin_capacitance.typical_femtofarads`, ruled on #192 Q2). Then each SPI device that declares an `orientation` (version 16): its part's row has `ardupilot.kind` `imu`, and cites `axes` and `ardupilot.frame` (`parts.json` schema 9) | `OrientationOnNonImu`, `ImuAxesNotCited` (the device reference); `UnknownPart` (`PartWithoutRow`), `AmbiguousPartName`, `PartOnWrongBus`, `I2cAddressNotOffered`, `SpiClockAbovePart`, `SpiModeNotDriven`, `PinCapacitanceOutOfRange`, `PinCapacitanceBelowI2cCi`, `PinCapacitanceAlreadyCited`, `PinCapacitanceBelowTypical` (the device reference) |
| 15 | `connectors` (`AET-WO-4300f`), each in document order: its reference is not `U`, `R` or `C` and a number, in any case (the emitter's own parts); its part is a row of the table; its pins are `1` to N, each once (a pin `07` is not `7`); then pin by pin, in document order, its net is declared and, on an I2C or SPI line (a bus line or a chip select), it has a nonzero `offboard` of at most `MAX_OFFBOARD_FEMTOFARADS` (1 µF), which no other pin has (a motor output's or serial port's pin included); then, if it names a standard, the port exists, has the part's series and pin count, and each pin in order holds its role | `UnknownConnectorPart`, `ConnectorPinUnknown`, `ConnectorPinRepeated`, `ConnectorPinMissing`, `ConnectorNetUnknown`, `ConnectorReferenceReserved`, `ConnectorBudgetMissing`, `ConnectorBudgetTooLarge`, `ConnectorBudgetNotApplicable`, `UnknownConnectorStandard`, `ConnectorPartContradictsStandard`, `ConnectorSupplyContradictsStandard`, `ConnectorPinContradictsStandard` (the reference) |
| 15a | `test_points` (`AET-WO-1312`), each in document order: its net is one a connector pin may name (a declared signal, a voltage domain, `GND`, `SWDIO`, `SWCLK`, `NRST` or `BOOT0`; `TestPointNetUnknown`); its reference is outside the emitter's numbering (`U`, `R`, `C` or `SW` and a number, in any case) and no other reference of the specification has its KiCad key (`TestPointReferenceTaken`); no net but `GND` carries two (`TestPointNetRepeated`; `GND` may carry any number, `AET-WO-3402` §1.1). |
| 16 | `power` (`AET-WO-4300d`), in the order of `power.rs`'s module documentation: the tree, the parts, the board temperature, the thermal-via rule, the control pins, each regulator's output against its domain, each domain against its consumers, the brown-out level's rising maximum, the battery, the hot-plug parts, the loads (and each `drop_limits` entry, `AET-WO-1303b`: a declared domain, 1 to 100000 mV, once per domain), headroom and timing, each buck's junction screen, the external parts with their effective capacitance under DC bias, power-in, the sense and the crossings. Each refusal cites its datasheet page | `PowerPartsTableInvalid` (the part), the `power::Refusal` names (the reference, domain or net) |
| 17 | A programming path (`AET-WO-4300g` §1.4): one declared connector carries `SWDIO`, `SWCLK`, `GND` and `mcu.supply` on its pins. NRST may reach it and is not required (DS-009's debug ports carry none) | `NoProgrammingPath` (the supply) |
| 18 | One declared supply source (`AET-WO-4300g` §1.4): with `mcu.supply_source` `null`, a regulator of `power` outputs `mcu.supply`; with it set, it names a declared connector's listed pin on `mcu.supply`, not on the programming connector; a pin named while a regulator outputs the supply too is two sources | `SupplyHasNoSource`, `SupplyHasTwoSources` (the supply) |
| 19 | Every I2C line, SDA then SCL and bus by bus, has a net constraint: its rise time bounds the bus's speed, and nothing is defaulted (`AET-WO-4100` F1, `AET-WO-1206` §1.2). Before, `solve` refused it | `MissingBusConstraint` (the bus id; `claimants()` the bus, as `solve` printed it) |
| 20 | The names `generate` refuses (`names.rs`, `AET-WO-1206` §1.3), each by `generate`'s kind, `id` and message: a reference (devices, connectors, the power tree's) starting `#`, KiCad's virtual-symbol prefix; a reference KiCad would take for one the emitter numbers, spelled another way: the MCU's `U<n>` (the first the specification does not spell), or any `R<n>` or `C<n>` from 1 (KiCad's key folds case and drops leading zeros, `reference_key`); then a net or domain the emitter names for itself (`own_nets`: `GND`, `VBAT` beside a power tree, the MCU's VCAPs, the parts' straps and filters, a buck's and a crossing's nets) in any case, or one starting `unconnected-` (an own net so starting, from a power reference, included); then two of the specification's references with one KiCad key (`J1` and `J01`; case alone is step 8's); then a device named for a peripheral instance a solve can use (`peripheral_instances`: `GPIOA`–`GPIOK`, `I2C1`–`I2C4`, `SPI1`–`SPI6`, `serial::PERIPHERALS`, `TIM<n>` of `motor::TIMERS`), in any case | `DuplicateReference` (`EmitterReference`; the pair, `U1 and u1`), `ReservedName` (the name, or the pair `U1_VCAP1 and u1_vcap1`), `InvalidDeviceName` (none, as the twin prints none) |
| 20a | Each `power.loads` entry with a `reference`, in document order (`names::loads`; `AET-WO-1209` §1.2, `AET-WO-1209c`): it names a part the board has (one the specification declares, the MCU's `U<n>`, the reset switch, or an `R<n>` or `C<n>` the emitter numbers); then it names neither the MCU nor a regulator of `power.regulators`, whose current a row already counts: the MCU's table row, or the regulator's own input | `LoadNamesNoPart`, `LoadNamesCountedPart` (the reference; `error.path`; the counting row in the message) |
| 21 | What `solve` refused from the specification and the reset clock alone (`AET-WO-1207` §1.2, `AET-WO-1208` §1.3; `1206` F3, `1207` F1), in the order `solve` judged it, by its kind, `id`, claimant and message: each I2C bus's worse line (the larger `max_rise_time_nanoseconds` of SDA and SCL) at most 1000 ns, the slowest I2C mode's rise time, and its `speed_hz` at most the ceiling that line sets; then each I2C bus's `pull_up_ohms` at least `electrical::i2c_pull_up_floor_ohms(speed_hz, rail)` at its domain's maximum (`AET-WO-4230` §1.5, review A8), and each line's capacitance at most `i2c_bus_capacitance_limit_femtofarads(speed_hz)`, SDA then SCL; then each SPI bus, in document order, on an MCU supply inside DS12117 Table 149's bands, 1620 to 3600 mV across its whole tolerance, and with a net constraint on its SCK then its MOSI, the load its output speed is chosen for (`AET-WO-4300c` D2); then each motor output's timing on the reset clock's one timer clock (a whole-MHz clock for PWM's 1 µs tick, a period within a 16-bit `ARR`, the longest pulse inside the period); then each serial port's `BRR` (16 to `0xFFFF`) and `serial::check_silicon` on the reset clock's one USART/UART kernel clock. Before, `solve` refused each | `RiseTimeTooSlow`, `BusTooFast`, `PullUpBelowSinkFloor`, `SupplyOutsideSpeedTable`, `MissingSpiConstraint` (the bus id), `BusCapacitanceOverLimit` (the net), `MotorTiming` (the label), `SerialBaudOutOfRange`, `SerialBaudTolerance` (the port's label); `claimants()` the same, as `solve` printed it |
| 22 | With no power tree, the MCU's supply domain by step 16's own consumer rule (`AET-WO-1208` §1.2, F1/F5 as ruled on PR #182): its declared range (`minimum_millivolts`, `maximum_millivolts`) inside the power-parts row's `kind.mcu.vdd_millivolts` (DS12117 Rev 10 Table 111, p. 209, VDD 1.62 to 3.6 V), its minimum above the floor, the larger of that minimum and the declared `bor_level`'s VBOR falling maximum (Table 115). With a tree, step 16 has judged it. After step 21, so `SupplyOutsideSpeedTable` beside an SPI bus keeps its name. A part with no MCU row is the emitter's | `DomainOutsideConsumer` (the domain; the message names the MCU and cites both rows) |

Step 2 walks the identifiers in this order: `project_name`; `mcu.part`, `mcu.supply`, then `mcu.supply_source`'s `connector` and `pin` if set; `voltage_domains[i].id`; each I2C bus (`id`, `peripheral` if set, `domain`, then `sda` and `scl`: `net`, `pin` if set); each SPI bus (the same, with `sck`, `mosi`, `miso`); each I2C device (`reference`, `part`, `supply`, `bus`, then `pin_capacitance.declared`'s `condition` and `source` if set, each held to the board's source rule, `InvalidSource`); each SPI device (the same, then `cs.net`, `cs.pin` if set, then the declaration's two texts); `net_constraints[i].net_name`; `board.layers[i].net` if set; `motor_outputs[i].label`, then `.pin` if set; `serial_ports[i].label`, `.peripheral` if set, then `.tx` and `.rx` (`net`, `pin` if set); each connector (`reference`, `part`, then each pin's `pin`, `net` if set and `offboard.source` if set, the last held to the board's source rule, `InvalidSource`). A connector's `standard` names a table row and may hold a space, so step 15 checks it against the table instead. Nothing is trimmed or case-folded, so ids that differ only in whitespace are refused, not matched.

`validate` does not check that a peripheral or pin exists, nor that two signals ask for one pin. Both need the pin table, so `solve` refuses them (below). A part it does check (step 14), against the parts table.

## `SolvedTopology`

What the solver derives (`RUL-16`):
- `schema_version` (11; an older file is refused as `UnsupportedSchemaVersion`, whether or not it has the keys a later version added);
- `specification_sha256` (64 lowercase hex digits);
- `assignments`, a `BTreeMap` keyed by net, holding every bus signal on an alternate function, each value a `PinAssignment` (`pin`, `peripheral`, `signal`, `alternate_function`);
- `gpio`, a `BTreeMap` from net to pin, holding every net driven as a plain GPIO output: today, each SPI device's `cs`. It records no direction, because every entry is an output;
- `i2c_ceilings`, a `BTreeMap` from I2C bus id to that bus's maximum speed in Hz (100,000, 400,000 or 1,000,000). It is empty when there are no I2C buses;
- `i2c_speeds`, a `BTreeMap` from I2C bus id to the speed the specification declared for it (`speed_hz`), under the same keys as `i2c_ceilings` (`AET-WO-1205`). Required, never defaulted;
- `analog_lines`, `i2c_devices` and `spi_devices` (`AET-WO-2200`, `2203`, `4200`); an SPI device carries its declared `orientation` (version 10, `AET-WO-4400d`), absent when none is declared, so that `generate hwdef` and `check pinout` read it without the specification (`RUL-16`);
- `spi_lines` (version 4, `AET-WO-4300c`), a `BTreeMap` keyed by net holding each SPI bus's SCK and MOSI: its `bus`, `signal`, `rate_hz` (the bus's `speed_hz`), the declared `capacitance_femtofarads` and `rise_time_ns`, and `vdd_min_millivolts`, the MCU supply's nominal less its tolerance, rounded down. The generator chooses each pin's output speed from it alone (`RUL-16`).
- `clock` (version 5, `AET-WO-4300a` §1.2), the clock the generated firmware runs on, as a solved fact: the reset clock, HSI at 64 MHz with every prescaler 1 and `TIMPRE` 0 (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.10), so every timer's kernel clock is 64 MHz (Table 57). `validate` refuses any other (`UnsupportedClock`);
- `motor_outputs` (version 5), a `BTreeMap` keyed by label (its net, a key of `assignments` on a `TIMn_CHk` signal): `index`, `protocol`, the declared `pwm`, and the solved `psc`, `arr`, `ccr`. `motor::check_silicon` judges those values against the declared timing on `clock`, with silicon's `ARR + 1` (`4300` D3);
- `power` (version 6, `AET-WO-4300d`), `null` or the solved tree: per domain its regulator, the regulator's guaranteed output at the source pad, the source and return pads (a buck's RFBT pad 1 and RFBB pad 2), its load and each consumer's floor; the battery-side steady and fault currents; every named condition the checks rest on.
- `serial_ports` (version 7, `AET-WO-4300b`), a `BTreeMap` keyed by label: `index`, `peripheral`, the `tx` and `rx` nets (keys of `assignments` on `<peripheral>_TX` and `<peripheral>_RX`), the declared `role`, `framing` and `peer_deviation_ppm`, and the solved `kernel_hz`, `brr`, `actual_millibaud` and `error_ppm`. `serial_ports_in_order` gives them in the specification's order with both assignments. The kernel clock at reset is `rcc_pclk2` for USART1 and USART6 and `rcc_pclk1` for the rest (`RCC_D2CCIP2R` resets to 0, RM0433 Rev 9 §8.7.21), 64 MHz on the reset clock. `serial::check_silicon` judges `BRR` and the framing on `clock` against RM0433's receiver tolerance (Tables 406 and 407, `OVER8` = 0, `ONEBIT` = 0, as data in `serial::TOLERANCE`).
- `spi_controllers` (version 8, `AET-WO-4230`), a `BTreeMap` keyed by SPI peripheral, one per SPI peripheral of `assignments`: `kernel` (`"per_ck"` for SPI1–3, `SPI123SEL` = `100`, `per_ck` being `hsi_ker_ck` at `CKPERSEL`'s reset `00`; `"hsi_ker_ck"` for SPI4/5 and SPI6, `SPI45SEL` / `SPI6SEL` = `011`; RM0433 Rev 9 §8.7.19, §8.7.20, §8.7.22), `kernel_hz`, `declared_hz` (the lowest `speed_hz` of the buses on it), `mbr` (the least with the kernel at HSI's fastest, +14,984 ppm rounded up, divided by `2^(mbr+1)` at or below `declared_hz`; DS12117 Rev 10 Table 36 / Table 133), `achieved_hz` (the nominal kernel so divided) and `achieved_max_hz` (the fastest so divided, what `check speeds` judges), each rounded down;
- `i2c_timings` (version 8), a `BTreeMap` keyed by I2C peripheral, one per I2C peripheral of `assignments`: `kernel_hz` (`rcc_pclk1` for I2C1–3, `rcc_pclk4` for I2C4, both selects at reset), `cr1` (the whole `CR1` the bring-up writes, `PE` clear: 0, the analog filter on, `DNF` 0) and `timingr`, keyed `Standard100k`, `Fast400k`, `FastPlus1M`, each synthesized by RM0433 Rev 9 §47.4.9 with every minimum met at HSI's fastest (`solve::synthesize_timingr`'s documentation);
- `bor_level` (version 8), `mcu.bor_level`, copied so the twin presents it without reading the specification (`RUL-16`).
- `mcu` (version 9, `AET-WO-4400b`), `mcu.part`, copied so a generator names the MCU without reading the specification (`RUL-16`);
- `pin_speeds` (version 9, `AET-WO-4400b` D2), a `BTreeMap` keyed by net, holding exactly the nets of `assignments` and `gpio`: each pin's `OSPEEDR`, `0` or `1`. `solve` chooses it with `speeds::OutputSpeeds::choose` for each line of `spi_lines`, refusing a line no selectable speed serves (`RateAbovePinSpeed` and its siblings), and gives `0` to every other pin (`AET-WO-4300c` D1, D3, D4). Generators and the pinout check read it; neither chooses (`RUL-20`). `validate` refuses a stray or missing net, or a value above `1` (`PinSpeedMismatch`).
- `foreign_clock` (version 11, `AET-WO-4400g`), `null`, or `mcu.foreign_clock`'s values copied by `solve`: `apb1_timers_hz`, `apb2_timers_hz` and `usarts`, keyed as declared, so the twin builds the platform's timers and USARTs at them without reading the specification (`RUL-16`);

`silicon` holds the independent rules for the version-8 values (`RUL-20`): `check_spi`, `check_i2c` and `check_timingr` judge them by their own RM0433 facts, never by `solve`'s code. `check_brr` judges a USART's `BRR` by `2 · |BRR · baud − kernel| ≤ baud` and `16 ≤ BRR ≤ 0xFFFF` in exact integers, sharing nothing with `serial::solve_brr` (`AET-WO-4230` R3, Q-H). `spi_words` gives the bring-up's whole SPI words, `CFG2` = `MASTER | SSM | AFCNTR` on every master (R4, Q-D, RM0433 Rev 9 §50.11.4 p. 2221), and `motor_output_pupd` a motor-output pin's `PUPDR` field, `10` pull-down (R5, Q-G). For the pinout check (`AET-WO-4400b` D3), `implied_pupd` gives the `PUPDR` field the topology implies for a pin (the motor pull-down, else none: no internal pull is declared, and an I2C line's pull-up is external), and `implied_open_drain` whether it is open-drain (exactly the keys of `analog_lines`). No generator calls either.

A repeated key in `assignments`, `gpio`, `i2c_ceilings` or `i2c_speeds` is refused when parsing, and every key is required. `SolvedTopology::validate` refuses, in this order:
1. `UnsupportedSchemaVersion`;
2. `MalformedSpecificationHash`;
3. `InvalidIdentifier`: in `assignments`, the paths `assignments.<net>`, then `.pin`, `.peripheral`, `.signal`; then in `gpio`, `gpio.<net>`, then `gpio.<net>.pin`; then `i2c_ceilings.<bus>`, then `i2c_speeds.<bus>`;
4. `AlternateFunctionOutOfRange` (above 15; the net);
5. `InvalidCeiling` (a ceiling other than the three mode speeds; the bus id);
6. `NetAssignedTwice` (a net in both maps; the net);
7. `DuplicatePin` (a pin used twice anywhere across both maps; the pin);
8. the analog lines, then `CeilingNotDerived` (a ceiling that is not what `solve` derives from the bus's own lines: `electrical::ceiling_hz` of the largest `ceiling_ns` among them, the worse line (`AET-WO-1206` §1.7); a bus with no line, or a bound slower than every mode, derives nothing; the bus id) (`AET-WO-1204`);
9. the declared speeds (`AET-WO-1205`): `SpeedBusMismatch` (the key sets of `i2c_speeds` and `i2c_ceilings` differ; the stray bus), then per bus `UnsupportedI2cSpeed` (not 100, 400 or 1,000 kHz), `BusTooFast` (above its ceiling), `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit`, each line held to `electrical::i2c_pull_up_floor_ohms` and `i2c_bus_capacitance_limit_femtofarads` at the declared speed, with the names `solve` uses;
10. the devices (`AET-WO-2203`, `4200`), then `SpiLineMismatch` (`spi_lines` is not exactly the solved SCK and MOSI assignments, a line's signal or a zero value, two rates or peripherals on one bus, or an SPI device whose bus has no line; the net or reference);
11. the motor outputs (`AET-WO-4300a`): `UnsupportedClock`, then `MotorOutputMismatch` (an assignment on a timer without an entry, or the reverse; a signal that is not `TIMn_CHk` of `TIMn`; a channel twice; indices not `0..n`; `pwm` not present exactly for PWM; `PSC` or `ARR` past 16 bits, `ARR` 0, `CCR` 0 or not below `ARR`; the label), then `TimerShared` (outputs of one timer differing in protocol, PWM rate, `PSC` or `ARR`; the timer);
12. the serial ports (`AET-WO-4300b`): `SerialPortMismatch` (an assignment on a USART, UART or LPUART that is not a port's `tx` or `rx` on the port's own peripheral, the net; or a port whose peripheral is not one of `serial::PERIPHERALS`, whose nets are not on `<peripheral>_TX` and `_RX`, that shares a net or a peripheral with another, whose `index` is out of `0..n` or repeated, whose framing's shape is refused or contradicts its role, whose `negotiated` declaration is refused, or whose `kernel_hz`, `brr`, `actual_millibaud` or `error_ppm` is not what `serial` recomputes from `clock` and the framing, the label);
13. the controllers (`AET-WO-4230`): `BorLevelOutOfRange` (`bor_level` above 3); where `foreign_clock` is present, `ForeignClockZero`, `ForeignClockUsarts` and `ForeignClockMuxSplit`, as rule 2c (`AET-WO-4400g`); `SpiControllerMismatch` (the keys of `spi_controllers` are not exactly the SPI peripherals of `assignments`, or an entry is not what `solve` derives from `clock` at the lowest `rate_hz` of its buses' `spi_lines`; the peripheral); `I2cTimingMismatch` (the same for `i2c_timings` and the I2C peripherals, with `cr1` and the three `TIMINGR` words; the peripheral);
14. the part limits again, from what the topology carries (`AET-WO-4300c` §4.2): `AmbiguousPartName`, `PartOnWrongBus`, `I2cAddressNotOffered`, and `SpiClockAbovePart` against the bus's `rate_hz` in `spi_lines`, then `SpiModeNotDriven`.

A specification's device lists and connectors are private to this crate (`RUL-16`, `AET-WO-1204`). Read them through `Specification::declared_i2c_devices`, `declared_spi_devices` and `declared_connectors`; `cli`, `svd-hal` and `twin` carry a `clippy.toml` that disallows all three.

The CLI's `validate` accepts only a `Specification`.

## `solve`

```rust
pub fn solve(spec: &Specification, table: &PinTable, specification_sha256: &str)
    -> Result<SolvedTopology, SolveError>
```

A pure function: its output depends only on the specification, the table and the hash (`INV-01`). The caller hashes the specification's bytes as it read them (`INV-03`); the solver never sees them. Every collection is a `BTreeMap`, `BTreeSet` or `Vec`, and the search's bound counts nodes, never time, so two solves give the same bytes on any host. `solve_with_limit` takes the node budget as an argument, and exists so tests can reach `BudgetExceeded`.

### Reserved pins

`RESERVED_PINS` are never assigned, and a hand pin on one refuses with `ReservedPin`:
- `PA13` (SWDIO) and `PA14` (SWCLK), so the board can still be programmed and debugged;
- `PC14` and `PC15` (LSE), so the board keeps its 32 kHz crystal.

The HSE pins (`PH0`, `PH1`) would be reserved only beside an HSE crystal. A specification can declare none yet (`mcu.hse_hz` must be 0, `4400` D1), so the solver assigns them like any other pin (`AET-WO-4300a` §1.1). On every committed specification this moved no assignment (measured, `AET-WO-4300a` FEEDBACK P4.1).

**Debugging is assumed to be SWD only.** The JTAG-only pins `PA15`, `PB3` and `PB4` stay assignable, as do USB's `PA11` and `PA12`.

### The model

- Every bus takes one **instance** of its kind. An instance is offered when the table carries each of its kind's signals for it on some pin (`SDA` and `SCL`, or `SCK`, `MOSI` and `MISO`); on the STM32H753ZI these are `I2C1` to `I2C4` and `SPI1` to `SPI6`. No two buses share an instance.
- Every bus signal takes an unreserved pin whose table entry carries `<instance>_<SIGNAL>`, with that entry's alternate function.
- Every SPI device's `cs` takes any unreserved pin, as a plain GPIO output, recorded in `gpio`.
- No pin is used twice. A hand-set `peripheral` or `pin` is fixed.

### I2C ceilings (`AET-WO-1202`)

Before the pin search, each I2C bus gets a ceiling from the rise time its board declares:
1. `t_max` is the larger `max_rise_time_nanoseconds` of the `net_constraints` entries for the bus's `sda.net` and `scl.net`, the worse line (`AET-WO-1206` §1.7).
2. The ceiling is the fastest mode whose maximum rise time (NXP UM10204) is at least `t_max`:
   - 1 MHz if `t_max` ≤ 120 ns;
   - 400 kHz if ≤ 300 ns;
   - 100 kHz if ≤ 1000 ns.

Pull-up resistance and capacitance are not read; that physics is the twin's.

### The search order

The order is fixed, and it decides which answer is found:
1. Buses go in document order, I2C before SPI.
2. Within a bus, instances go in ascending name order, then its signals in the order `sda, scl` or `sck, mosi, miso`. Each signal tries pins in the table's key order.
3. Once every bus is placed, the motor outputs, in document order (`AET-WO-4300a`). Each tries the table's timer channels (`TIMn_CH1` to `TIMn_CH4` of a timer with channels; never a complementary `CHkN`) in the order timer number, channel, pin key, taking a free pin, a channel not taken, and a timer that is free or already carries the output's key: its protocol and, for PWM, its rate (outputs of one timer share its counter). A failure backtracks into the buses, so the search stays complete.
4. Once every motor output is, the serial ports, in document order (`AET-WO-4300b`). Each tries its declared peripheral, or each of `serial::PERIPHERALS` the table offers in that order, not taken by another port; on it, a TX pin then an RX pin, each in key order, its hand pin or a free one, never one pin for both. A failure backtracks into the motor outputs and the buses, so the search stays complete; on a specification without serial ports this step places nothing and spends no node, so no committed solve moved.
5. Every `cs` goes last, in `spi_devices` document order. Each takes the first free, unreserved pin in key order. This greedy pass loses no solution, because the buses, the motor outputs and the serial ports use a fixed number of pins.

Each serial port's `BRR` is `serial::solve_brr` on its peripheral's kernel clock: the nearest whole `USARTDIV`, half up, oversampling by 16, from 16 to `0xFFFF`. Its error is then at most `1 / (2 BRR)`: 3.125 % at `BRR` 16 (Table 406, whose tightest row is 3.41 %), 2.94 % from 17 on (Table 407, tightest 3.03 %). The quantization alone therefore never fails the tolerance (`serial::within`), but the check for silicon judges the worst case (`AET-WO-4230` §1.8): the quantization error plus HSI's deviation (up to 25,894 ppm) plus the port's `peer_deviation_ppm`, strictly below the row, so `SerialBaudTolerance` refuses a declaration whose sum reaches it (the small `BRR`s, whose rounding error is largest, and any large declared peer deviation).

Each motor output's `PSC`, `ARR` and `CCR` (`motor::solve_timing`) depend on its protocol and the timer clock alone, which is 64 MHz on every timer of the reset clock: PWM takes a 1 µs tick (`PSC` 63), `ARR` the nearest whole tick count to `1 s / rate` less 1 (silicon counts `ARR + 1`), and `CCR` the idle pulse, `pulse_min_us`; DShot takes `PSC` 0, the bit the nearest whole count of 64 MHz ticks to `1 / bit rate` (107 at 600 kbit/s), and `CCR` the nearest whole count to 3/8 of it (a `0` bit, ArduPilot `Copter-4.7.2` `AP_HAL/RCOutput.h` L406–408).

Names sort in **byte order**: `PA0, PA1, PA10, …, PA15, PA2`. So `SPI2_SCK` takes `PA12` before `PA9`. Instance names sort correctly only while they stay single-digit.

The search backtracks. Before it starts, and again after each bus is placed, every bus still to place must pass three checks:
- it still has an instance whose every signal has a free pin (forward checking);
- the remaining buses can take distinct instances (instance matching);
- their signals can take distinct free pins, each drawn from the union of its candidates over the bus's still-placeable instances (pin matching).

Each check answers only yes or no, so none can change which answer is found. Together they turn the infeasible specifications the `AET-WO-1200` spike found exponential into refusals that take well under a millisecond. Neither matching check alone covers both shapes.

They are not complete. When two instances need the same single pin for one signal, both checks pass, and an infeasible specification can still spend the whole budget (`AET-WO-1200` FEEDBACK A1).

The search tries at most `NODE_LIMIT` (10,000,000) values across the whole solve, including the minimal-set searches.

### Refusals (`SolveError`), in check order

Each check runs in document order: bus signals, then every `cs`. `kind()` is the variant name, except for `InvalidSpecification`, which reports the validation refusal's own kind. `id()` is the single pin, peripheral, bus or part refused, or `None`. `claimants()` lists who is in conflict:
- for pin refusals, the signal's JSON path, such as `i2c_buses[0].sda` or `spi_devices[0].cs`;
- for peripheral refusals, the bus's path, such as `i2c_buses[0]`;
- for the refusals about whole buses, the bus ids.

| # | Check | Refusal | `claimants()` |
|---|---|---|---|
| 1 | `Specification::validate` passes (its step 19 refuses an I2C line with no net constraint, `MissingBusConstraint`; its step 21 what `solve` refused from the specification and the reset clock alone, `AET-WO-1207` §1.2 and `AET-WO-1208` §1.3; its step 22 an MCU supply outside the MCU row), and the hash is 64 lowercase hex digits | `InvalidSpecification` (the inner kind) | none, except the refusals it shares with `solve`: `MissingBusConstraint`, `RiseTimeTooSlow`, `BusTooFast`, `PullUpBelowSinkFloor`, `SupplyOutsideSpeedTable`, `MissingSpiConstraint` (the bus id), `BusCapacitanceOverLimit` (the net), `MotorTiming` (the label), `SerialBaudOutOfRange`, `SerialBaudTolerance` (the port's label) |
| 2 | `mcu.part` is the table's part | `PartMismatch` | none |
| 3 | Every hand pin (buses, `cs`, motor outputs, then serial ports' `tx` and `rx`) is a key of the table (compared as a key, never by case) | `UnknownPin` | the signal |
| | … and is not reserved | `ReservedPin` | the signal |
| | … and no earlier signal holds the same pin | `PinClaimedTwice` | both signals |
| 4 | Every hand peripheral is an offered instance of the bus's kind | `UnknownPeripheral` | the bus |
| | … and no earlier bus holds the same instance | `PeripheralClaimedTwice` | both buses |
| | … and every serial port's is offered by the table (`validate` refused the rest) | `UnknownPeripheral` | the port's path, `serial_ports[i]` |
| 5 | Each bus alone (reserved pins and every hand pin honoured) can take some instance. The message says why for each instance tried | `NoInstance` | the bus |
| 5a | Each motor output alone, beside every hand pin, has a timer channel (a hand pin with none, or every channel's pin held by hand) | `NoTimerChannel` | the label |
| | … two outputs whose channels all lie on one timer, the same one, share their key | `TimerRateConflict` | both labels |
| | … and together, beside every hand pin and no bus, they can all be placed; else a minimal set, by deletion | `MotorsInfeasible` | the labels in the set |
| 5b | Each serial port alone, beside every hand pin, has a peripheral with a TX and an RX pin. A fixed TX and RX that each carry some peripheral it may use, but never one carrying both, are named apart | `SerialPinsSplit`, else `NoSerialPeripheral` (the message says why for each peripheral) | the label |
| | … and together, beside every hand pin and no bus or motor output, they can all be placed; else a minimal set, by deletion | `SerialPortsInfeasible` | the labels in the set |
| 6 | The search places every bus. If it cannot, the result is a minimal set: each bus is dropped in document order if the rest stay infeasible. The set is irreducible, not necessarily the smallest, and costs one search per bus. With motor outputs or serial ports declared, they stay in every search, and the refusal names the set's buses, every output and every port | `Infeasible`; `MotorsInfeasible` with motor outputs and no serial port; `SerialPortsInfeasible` with serial ports | the buses in the set (then the outputs, then the ports) |
| 7 | The node budget is not spent. This refusal names only the bus being placed, by ruling, or `motor_outputs` or `serial_ports` when those spent it | `BudgetExceeded` | that bus |
| 7a | Each serial port, in document order: its peripheral has a kernel clock on `clock` (its `BRR` and `serial::check_silicon` are `validate`'s step 21, `AET-WO-1208` §1.3) | `SerialClock` | the label |
| 7b | Each SPI bus, in document order: its controller has a kernel clock on `clock` (`per_ck` or `hsi_ker_ck` from HSI) | `SpiKernelClockUnavailable` | the bus id |
| | … and its controller's lowest declared speed is at least the kernel at HSI's fastest divided by 256 (`MBR` 111). The verdict is the specification's (every SPI kernel is HSI on the reset clock), but the message names the controller the search chose, so it stays here (`AET-WO-1208` FEEDBACK) | `SpiSpeedBelowKernelRange` | the bus id |
| 8 | Every `cs` finds a free pin | `NoFreePin` | the `cs` |

The output passes `SolvedTopology::validate` before it is returned.
