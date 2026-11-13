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
  "net_constraints": [
    { "net_name": "I2C_SDA", "max_capacitance_femtofarads": 100000,
      "max_rise_time_nanoseconds": 300, "impedance_ohms": null },
    { "net_name": "I2C_SCL", "max_capacitance_femtofarads": 100000,
      "max_rise_time_nanoseconds": 300, "impedance_ohms": null }
  ]
}
```

Every list except `voltage_domains` may be empty.

## `Specification::validate`

Returns the first refusal, in this order, and in document order within a step. `id()` is what the refusal names; `InvalidIdentifier` names nothing there and carries the JSON path instead (`path()`).

| # | Rule | Refusal (`id()`) |
|---|---|---|
| 1 | `schema_version` is 1 | `UnsupportedSchemaVersion` (none) |
| 2 | Every identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E`, none of them `/`, `{`, `}`, `$`, `[`, `]` or `"`, and none holding a date shape (`dddd-dd-dd`) or a time shape (`dd:dd:dd`). The project name is also a crate name (1 to 58 bytes of `a-z`, `0-9`, `-`; a letter first, no trailing or doubled `-`; 58 because `generate` adds `-board`), and a device reference, I2C or SPI, a Renode identifier (a letter, then letters, digits and `_`). One function, `check_identifier(IdentifierKind, &str)`, holds the table, and the error's message names the rule broken (`RUL-14`; `AET-WO-1102`) | `InvalidIdentifier` (none; `path()`) |
| 3 | `voltage_domains` is not empty | `NoVoltageDomain` (none) |
| 4 | Domains: unique ids, nominal voltage not 0, tolerance in 1..=500 | `DuplicateDomainId`, `ZeroNominalVoltage`, `ToleranceOutOfRange` (the id) |
| 5 | `mcu.supply`, every bus `domain` and every device `supply` names a declared domain | `UnknownDomain` (the name) |
| 5a | Every bus `domain` equals `mcu.supply`, and every device `supply` equals its bus's `domain` (one rail; level shifting is unscoped). A device naming no bus is left to step 9 | `SupplyMismatch` (the bus id or device reference) |
| 6 | Bus ids are unique across both bus lists | `DuplicateBusId` (the id) |
| 7 | An I2C `speed_hz` is 100000, 400000 or 1000000, and `pull_up_ohms` is not 0. An SPI `speed_hz` is not 0 | `UnsupportedI2cSpeed`, `ZeroPullUp`, `ZeroSpiSpeed` (the bus id) |
| 8 | References are unique across both device lists, compared with case folded | `DuplicateReference` (the reference; the message names the first spelling) |
| 9 | A device's `bus` names a bus of its own kind | `UnknownBus` (the name) |
| 10 | An I2C address is in `0x08..=0x77` and unique on its bus | `I2cAddressOutOfRange`, `DuplicateI2cAddress` (the device reference) |
| 11 | A net is declared by exactly one signal across every bus and every `cs`, and no signal net has a voltage domain's id (compared with case folded),, none differs from `GND` only in case, and voltage domain ids are compared the same way (`DuplicateDomainId`) | `DuplicateNetName`, `NetShadowsDomain` (the net) |
| 12 | Net constraints: unique `net_name`; each names a declared signal net or a voltage domain id; capacitance, rise time and impedance are not 0 | `DuplicateNetConstraint`, `UnknownNet`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroImpedance` (the net) |

Step 2 walks the identifiers in this order: `project_name`; `mcu.part`, `mcu.supply`; `voltage_domains[i].id`; each I2C bus (`id`, `peripheral` if set, `domain`, then `sda` and `scl`: `net`, `pin` if set); each SPI bus (the same, with `sck`, `mosi`, `miso`); each I2C device (`reference`, `part`, `supply`, `bus`); each SPI device (the same, then `cs.net`, `cs.pin` if set); `net_constraints[i].net_name`. Nothing is trimmed or case-folded, so ids that differ only in whitespace are refused, not matched.

`validate` does not check that a part, peripheral or pin exists, nor that two signals ask for one pin. Both need the pin table, so `solve` refuses them (below).

## `SolvedTopology`

What the solver derives (`RUL-16`):
- `schema_version` (2; a version-1 file is refused as `UnsupportedSchemaVersion`, whether or not it has `i2c_speeds`);
- `specification_sha256` (64 lowercase hex digits);
- `assignments`, a `BTreeMap` keyed by net, holding every bus signal on an alternate function, each value a `PinAssignment` (`pin`, `peripheral`, `signal`, `alternate_function`);
- `gpio`, a `BTreeMap` from net to pin, holding every net driven as a plain GPIO output: today, each SPI device's `cs`. It records no direction, because every entry is an output;
- `i2c_ceilings`, a `BTreeMap` from I2C bus id to that bus's maximum speed in Hz (100,000, 400,000 or 1,000,000). It is empty when there are no I2C buses;
- `i2c_speeds`, a `BTreeMap` from I2C bus id to the speed the specification declared for it (`speed_hz`), under the same keys as `i2c_ceilings` (`AET-WO-1205`). Required under version 2, never defaulted.

A repeated key in `assignments`, `gpio`, `i2c_ceilings` or `i2c_speeds` is refused when parsing, and every key is required. `SolvedTopology::validate` refuses, in this order:
1. `UnsupportedSchemaVersion`;
2. `MalformedSpecificationHash`;
3. `InvalidIdentifier`: in `assignments`, the paths `assignments.<net>`, then `.pin`, `.peripheral`, `.signal`; then in `gpio`, `gpio.<net>`, then `gpio.<net>.pin`; then `i2c_ceilings.<bus>`, then `i2c_speeds.<bus>`;
4. `AlternateFunctionOutOfRange` (above 15; the net);
5. `InvalidCeiling` (a ceiling other than the three mode speeds; the bus id);
6. `NetAssignedTwice` (a net in both maps; the net);
7. `DuplicatePin` (a pin used twice anywhere across both maps; the pin);
8. the analog lines, then `CeilingNotDerived` (a ceiling that is not what `solve` derives from the bus's own lines: `electrical::ceiling_hz` of the smallest `ceiling_ns` among them; a bus with no line, or a bound slower than every mode, derives nothing; the bus id) (`AET-WO-1204`);
9. the declared speeds (`AET-WO-1205`): `SpeedBusMismatch` (the key sets of `i2c_speeds` and `i2c_ceilings` differ; the stray bus), then per bus `UnsupportedI2cSpeed` (not 100, 400 or 1,000 kHz), `BusTooFast` (above its ceiling), `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit`, each line held to `electrical::i2c_pull_up_floor_ohms` and `i2c_bus_capacitance_limit_femtofarads` at the declared speed, with the names `solve` uses.

A specification's device lists are private to this crate (`RUL-16`, `AET-WO-1204`). Read them through `Specification::declared_i2c_devices` and `declared_spi_devices`; `cli`, `svd-hal` and `twin` carry a `clippy.toml` that disallows both.

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
- `PC14` and `PC15` (LSE) and `PH0` and `PH1` (HSE), so the board keeps its crystals.

**Debugging is assumed to be SWD only.** The JTAG-only pins `PA15`, `PB3` and `PB4` stay assignable, as do USB's `PA11` and `PA12`.

### The model

- Every bus takes one **instance** of its kind. An instance is offered when the table carries each of its kind's signals for it on some pin (`SDA` and `SCL`, or `SCK`, `MOSI` and `MISO`); on the STM32H753ZI these are `I2C1` to `I2C4` and `SPI1` to `SPI6`. No two buses share an instance.
- Every bus signal takes an unreserved pin whose table entry carries `<instance>_<SIGNAL>`, with that entry's alternate function.
- Every SPI device's `cs` takes any unreserved pin, as a plain GPIO output, recorded in `gpio`.
- No pin is used twice. A hand-set `peripheral` or `pin` is fixed.

### I2C ceilings (`AET-WO-1202`)

Before the pin search, each I2C bus gets a ceiling from the rise time its board declares:
1. `t_max` is the smaller `max_rise_time_nanoseconds` of the `net_constraints` entries for the bus's `sda.net` and `scl.net`.
2. The ceiling is the fastest mode whose maximum rise time (NXP UM10204) is at least `t_max`:
   - 1 MHz if `t_max` ≤ 120 ns;
   - 400 kHz if ≤ 300 ns;
   - 100 kHz if ≤ 1000 ns.

Pull-up resistance and capacitance are not read; that physics is the twin's.

### The search order

The order is fixed, and it decides which answer is found:
1. Buses go in document order, I2C before SPI.
2. Within a bus, instances go in ascending name order, then its signals in the order `sda, scl` or `sck, mosi, miso`. Each signal tries pins in the table's key order.
3. Every `cs` goes last, in `spi_devices` document order. Each takes the first free, unreserved pin in key order. This greedy pass loses no solution, because the buses use a fixed number of pins.

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
| 1 | `Specification::validate` passes, and the hash is 64 lowercase hex digits | `InvalidSpecification` (the inner kind) | none |
| 1a | Every I2C line, SDA then SCL and bus by bus, has a net constraint (nothing is defaulted) | `MissingBusConstraint` (the message names the net) | the bus id |
| | … its `t_max` is at most 1000 ns | `RiseTimeTooSlow` | the bus id |
| | … and its `speed_hz` is at most its ceiling | `BusTooFast` | the bus id |
| 1b | Each bus's `pull_up_ohms` is at least `aethelgard_ir::electrical::i2c_pull_up_floor_ohms(speed_hz, rail)` (UM10204 sink current), the rail being its domain's `nominal_millivolts` | `PullUpBelowSinkFloor` | the bus id |
| | … and each line's `max_capacitance_femtofarads` is at most `i2c_bus_capacitance_limit_femtofarads(speed_hz)`, SDA then SCL | `BusCapacitanceOverLimit` | the net |
| 2 | `mcu.part` is the table's part | `PartMismatch` | none |
| 3 | Every hand pin is a key of the table (compared as a key, never by case) | `UnknownPin` | the signal |
| | … and is not reserved | `ReservedPin` | the signal |
| | … and no earlier signal holds the same pin | `PinClaimedTwice` | both signals |
| 4 | Every hand peripheral is an offered instance of the bus's kind | `UnknownPeripheral` | the bus |
| | … and no earlier bus holds the same instance | `PeripheralClaimedTwice` | both buses |
| 5 | Each bus alone (reserved pins and every hand pin honoured) can take some instance. The message says why for each instance tried | `NoInstance` | the bus |
| 6 | The search places every bus. If it cannot, the result is a minimal set: each bus is dropped in document order if the rest stay infeasible. The set is irreducible, not necessarily the smallest, and costs one search per bus | `Infeasible` | the buses in the set |
| 7 | The node budget is not spent. This refusal names only the bus being placed, by ruling | `BudgetExceeded` | that bus |
| 8 | Every `cs` finds a free pin | `NoFreePin` | the `cs` |

The output passes `SolvedTopology::validate` before it is returned.
