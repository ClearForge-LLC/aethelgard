# aethelgard-ir

The specification schema and the solved topology, as two separate types (`RUL-16`), with validation. Every type refuses unknown fields and accepts only a JSON object. Nothing defaults (`INV-06`).

## `Specification`, version 1

Every key is required. `peripheral`, `pin` and `impedance_ohms` may be `null`, but the key may not be missing. Arrays keep the order they were written in; nothing is sorted.

```json
{
  "schema_version": 1,
  "project_name": "sentinel-01",
  "mcu": { "part": "STM32H753ZITx", "supply": "VDD_3V3" },
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
    { "reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 118 },
    { "reference": "U3", "part": "TMP117", "supply": "VDD_3V3", "bus": "SENSORS", "address": 72 }
  ],
  "spi_devices": [{ "reference": "U4", "part": "W25Q64JV", "supply": "VDD_3V3", "bus": "FLASH",
    "cs": { "net": "FLASH_CS", "pin": null } }],
  "net_constraints": [{ "net_name": "I2C_SDA", "max_capacitance_femtofarads": 400000,
    "max_rise_time_nanoseconds": 300, "impedance_ohms": null }]
}
```

Every list except `voltage_domains` may be empty.

## `Specification::validate`

Returns the first refusal, in this order, and in document order within a step. `id()` is what the refusal names; `InvalidIdentifier` names nothing there and carries the JSON path instead (`path()`).

| # | Rule | Refusal (`id()`) |
|---|---|---|
| 1 | `schema_version` is 1 | `UnsupportedSchemaVersion` (none) |
| 2 | Every identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E` (`RUL-14`) | `InvalidIdentifier` (none; `path()`) |
| 3 | `voltage_domains` is not empty | `NoVoltageDomain` (none) |
| 4 | Domains: unique ids, nominal voltage not 0, tolerance in 1..=500 | `DuplicateDomainId`, `ZeroNominalVoltage`, `ToleranceOutOfRange` (the id) |
| 5 | `mcu.supply`, every bus `domain` and every device `supply` names a declared domain | `UnknownDomain` (the name) |
| 6 | Bus ids are unique across both bus lists | `DuplicateBusId` (the id) |
| 7 | An I2C `speed_hz` is 100000, 400000 or 1000000, and `pull_up_ohms` is not 0. An SPI `speed_hz` is not 0 | `UnsupportedI2cSpeed`, `ZeroPullUp`, `ZeroSpiSpeed` (the bus id) |
| 8 | References are unique across both device lists | `DuplicateReference` (the reference) |
| 9 | A device's `bus` names a bus of its own kind | `UnknownBus` (the name) |
| 10 | An I2C address is in `0x08..=0x77` and unique on its bus | `I2cAddressOutOfRange`, `DuplicateI2cAddress` (the device reference) |
| 11 | A net is declared by exactly one signal across every bus and every `cs`, and no signal net has a voltage domain's id | `DuplicateNetName`, `NetShadowsDomain` (the net) |
| 12 | Net constraints: unique `net_name`; each names a declared signal net or a voltage domain id; capacitance, rise time and impedance are not 0 | `DuplicateNetConstraint`, `UnknownNet`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroImpedance` (the net) |

Step 2 walks the identifiers in this order: `project_name`; `mcu.part`, `mcu.supply`; `voltage_domains[i].id`; each I2C bus (`id`, `peripheral` if set, `domain`, then `sda` and `scl`: `net`, `pin` if set); each SPI bus (the same, with `sck`, `mosi`, `miso`); each I2C device (`reference`, `part`, `supply`, `bus`); each SPI device (the same, then `cs.net`, `cs.pin` if set); `net_constraints[i].net_name`. Nothing is trimmed or case-folded, so ids that differ only in whitespace are refused, not matched.

`validate` does not check that a part, peripheral or pin exists, nor that two signals ask for one pin. Both need the pin table, and `AET-WO-1200` refuses them.

## `SolvedTopology`

What the solver derives (`RUL-16`): `schema_version` (1), `specification_sha256` (64 lowercase hex digits) and `assignments`, a `BTreeMap` keyed by net, each value a `PinAssignment` (`pin`, `peripheral`, `signal`, `alternate_function`). A repeated key in `assignments` is refused when parsing. `SolvedTopology::validate` refuses, in order: `UnsupportedSchemaVersion`, `MalformedSpecificationHash`, `InvalidIdentifier` (paths `assignments.<net>`, then `.pin`, `.peripheral`, `.signal`), `AlternateFunctionOutOfRange` (above 15; the net), `DuplicatePin` (the pin). The CLI's `validate` accepts only a `Specification`.
