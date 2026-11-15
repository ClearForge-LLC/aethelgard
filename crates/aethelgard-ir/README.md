# aethelgard-ir

The specification schema and the solved topology, as two separate types (`RUL-16`), with validation. Every type refuses unknown fields and accepts only a JSON object. Nothing defaults (`INV-06`).

## `Specification`, version 4

Version 2 (`AET-WO-4100`) added `board`. Version 3 (`AET-WO-4300c`) requires a net constraint on every SPI bus's `sck` and `mosi`, whose capacitance and rise time choose the pin's output speed, and names parts as the version-2 parts table does (`W25Q64JVxIQ`). Version 4 (`AET-WO-4300a`) requires `mcu.hse_hz` (the crystal's frequency, `0` for none, the only value admitted until a crystal part exists, `4400` D1) and `motor_outputs`: each a `label` (its net), a `pin` (`null` for the solver), a `protocol` (`pwm`, `dshot300`, `dshot600`) and `pwm` (`{rate_hz, pulse_min_us, pulse_max_us}` for PWM, `null` for DShot). An older file is refused by name (`UnsupportedSchemaVersion`); nothing supplies a board or a declaration for it. Every key is required. `peripheral`, `pin`, `impedance_ohms` and a signal layer's `net` may be `null`, but the key may not be missing. Arrays keep the order they were written in; nothing is sorted.

```json
{
  "schema_version": 4,
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
  "spi_devices": [{ "reference": "U4", "part": "W25Q64JVxIQ", "supply": "VDD_3V3", "bus": "FLASH",
    "cs": { "net": "FLASH_CS", "pin": null } }],
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
  }
}
```

Every list except `voltage_domains` may be empty.

**`board`** (`AET-WO-4100`, ruled on PR #120). Micrometres and thousandths. A stackup value is a bound with its source, never a nominal: a minimum dielectric is its source's value less a published tolerance, rounded down; a maximum is rounded up. The engine builds 2 copper layers (both signal) or 4 (`F.Cu`, `In1.Cu`, `In2.Cu`, `B.Cu`: outer layers signal, both inner layers planes, exactly one `GND` and the other a declared domain), through vias only. The rules are honoured: clearance and track width at or above the engine floor (0.2 mm), and the one via the engine builds (0.3 mm drill, 0.6 mm). See `crates/aethelgard-cli/tests/fixtures/four-layer.json` for a 4-layer board on OSH Park's published stackup.

## `Specification::validate`

Returns the first refusal, in this order, and in document order within a step. `id()` is what the refusal names; `InvalidIdentifier` names nothing there and carries the JSON path instead (`path()`).

| # | Rule | Refusal (`id()`) |
|---|---|---|
| 1 | `schema_version` is 3 | `UnsupportedSchemaVersion` (none) |
| 2 | Every identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E`, none of them `/`, `{`, `}`, `$`, `[`, `]`, `"`, `%`, `*`, `,` or `\`, and none holding a date shape (`dddd-dd-dd`) or a time shape (`dd:dd:dd`). The project name is also a crate name (1 to 58 bytes of `a-z`, `0-9`, `-`; a letter first, no trailing or doubled `-`; 58 because `generate` adds `-board`), and a device reference, I2C or SPI, a Renode identifier (a letter, then letters, digits and `_`). One function, `check_identifier(IdentifierKind, &str)`, holds the table, and the error's message names the rule broken (`RUL-14`; `AET-WO-1102`) | `InvalidIdentifier` (none; `path()`) |
| 2a | `mcu.hse_hz` is 0: a crystal needs a crystal part, which no table holds (`4400` D1) | `CrystalNotSupported` (none) |
| 3 | `voltage_domains` is not empty | `NoVoltageDomain` (none) |
| 4 | Domains: unique ids, nominal voltage not 0, tolerance in 1..=500 | `DuplicateDomainId`, `ZeroNominalVoltage`, `ToleranceOutOfRange` (the id) |
| 5 | `mcu.supply`, every bus `domain` and every device `supply` names a declared domain | `UnknownDomain` (the name) |
| 5a | Every bus `domain` equals `mcu.supply`, and every device `supply` equals its bus's `domain` (one rail; level shifting is unscoped). A device naming no bus is left to step 9 | `SupplyMismatch` (the bus id or device reference) |
| 6 | Bus ids are unique across both bus lists | `DuplicateBusId` (the id) |
| 7 | An I2C `speed_hz` is 100000, 400000 or 1000000, and `pull_up_ohms` is not 0. An SPI `speed_hz` is not 0 | `UnsupportedI2cSpeed`, `ZeroPullUp`, `ZeroSpiSpeed` (the bus id) |
| 8 | References are unique across both device lists, compared with case folded | `DuplicateReference` (the reference; the message names the first spelling) |
| 9 | A device's `bus` names a bus of its own kind | `UnknownBus` (the name) |
| 10 | An I2C address is in `0x08..=0x77` and unique on its bus | `I2cAddressOutOfRange`, `DuplicateI2cAddress` (the device reference) |
| 11 | A net is declared by exactly one signal across every bus, every `cs` and every motor output's `label`, and no signal net has a voltage domain's id (compared with case folded),, none differs from `GND` only in case, and voltage domain ids are compared the same way (`DuplicateDomainId`) | `DuplicateNetName`, `NetShadowsDomain` (the net) |
| 12 | Net constraints: unique `net_name`; each names a declared signal net or a voltage domain id; capacitance, rise time and impedance are not 0 | `DuplicateNetConstraint`, `UnknownNet`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroImpedance` (the net) |
| 13 | `board`, in this order: `copper_layers` is 2 or 4; `layers` is KiCad's copper names in order; each role fits (outer signal, at 4 both inner planes, a plane names a net, a signal layer none); each plane net is `GND` or a declared domain, one plane per net, exactly one `GND` at 4; `dielectrics` is the adjacent pairs in order, each minimum above 0; `finished_thickness_um` has `0 < min <= nominal <= max <= 10000` and `min` at least the dielectric minima summed (at 2 layers, the dielectric minimum at most `min` less twice `copper_max_um`, which the notes then imply); `copper_max_um` in 1..=10000, permittivity in 1000..=100000; every `source` 1 to 200 bytes of printable ASCII, no space at either end; clearance and track width at least 200 µm; via drill 300 µm and diameter 600 µm exactly | `UnsupportedLayerCount` (none), `LayerTable`, `LayerRole`, `DielectricTable`, `StackupValue`, `InvalidSource`, `RuleBelowFloor`, `UnsupportedViaRule` (none; `path()`), `PlaneNet` (the net), `PlaneLayout`, `FinishedThickness` (none) |
| 13a | Each motor output in document order: `pwm` an object for PWM and `null` for DShot; a rate above 0; `1 <= pulse_min_us < pulse_max_us`; and `pulse_max_us * rate_hz < 1 000 000`, the longest pulse inside the period | `MotorTimingShape`, `ZeroMotorRate`, `PulseRange`, `PulseExceedsPeriod` (the label) |
| 14 | The embedded parts table parses (`PartsTableInvalid`); then each device's part, I2C devices first (`AET-WO-4300c` §1.5): the bare `W25Q64JV` is refused, naming both ordering codes; a part the table holds sits on its own kind of bus, an I2C device at an address its straps offer, an SPI device on a bus clocked no faster than the part. Each message cites the datasheet. A part the table does not hold is not refused here | `AmbiguousPartName`, `PartOnWrongBus`, `I2cAddressNotOffered`, `SpiClockAbovePart` (the device reference) |

Step 2 walks the identifiers in this order: `project_name`; `mcu.part`, `mcu.supply`; `voltage_domains[i].id`; each I2C bus (`id`, `peripheral` if set, `domain`, then `sda` and `scl`: `net`, `pin` if set); each SPI bus (the same, with `sck`, `mosi`, `miso`); each I2C device (`reference`, `part`, `supply`, `bus`); each SPI device (the same, then `cs.net`, `cs.pin` if set); `net_constraints[i].net_name`; `board.layers[i].net` if set; `motor_outputs[i].label`, then `.pin` if set. Nothing is trimmed or case-folded, so ids that differ only in whitespace are refused, not matched.

`validate` does not check that a part, peripheral or pin exists, nor that two signals ask for one pin. Both need the pin table, so `solve` refuses them (below).

## `SolvedTopology`

What the solver derives (`RUL-16`):
- `schema_version` (5; an older file is refused as `UnsupportedSchemaVersion`, whether or not it has the keys a later version added);
- `specification_sha256` (64 lowercase hex digits);
- `assignments`, a `BTreeMap` keyed by net, holding every bus signal on an alternate function, each value a `PinAssignment` (`pin`, `peripheral`, `signal`, `alternate_function`);
- `gpio`, a `BTreeMap` from net to pin, holding every net driven as a plain GPIO output: today, each SPI device's `cs`. It records no direction, because every entry is an output;
- `i2c_ceilings`, a `BTreeMap` from I2C bus id to that bus's maximum speed in Hz (100,000, 400,000 or 1,000,000). It is empty when there are no I2C buses;
- `i2c_speeds`, a `BTreeMap` from I2C bus id to the speed the specification declared for it (`speed_hz`), under the same keys as `i2c_ceilings` (`AET-WO-1205`). Required, never defaulted;
- `analog_lines`, `i2c_devices` and `spi_devices` (`AET-WO-2200`, `2203`, `4200`);
- `spi_lines` (version 4, `AET-WO-4300c`), a `BTreeMap` keyed by net holding each SPI bus's SCK and MOSI: its `bus`, `signal`, `rate_hz` (the bus's `speed_hz`), the declared `capacitance_femtofarads` and `rise_time_ns`, and `vdd_min_millivolts`, the MCU supply's nominal less its tolerance, rounded down. The generator chooses each pin's output speed from it alone (`RUL-16`).
- `clock` (version 5, `AET-WO-4300a` §1.2), the clock the generated firmware runs on, as a solved fact: the reset clock, HSI at 64 MHz with every prescaler 1 and `TIMPRE` 0 (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.10), so every timer's kernel clock is 64 MHz (Table 57). `validate` refuses any other (`UnsupportedClock`);
- `motor_outputs` (version 5), a `BTreeMap` keyed by label (its net, a key of `assignments` on a `TIMn_CHk` signal): `index`, `protocol`, the declared `pwm`, and the solved `psc`, `arr`, `ccr`. `motor::check_silicon` judges those values against the declared timing on `clock`, with silicon's `ARR + 1` (`4300` D3).

A repeated key in `assignments`, `gpio`, `i2c_ceilings` or `i2c_speeds` is refused when parsing, and every key is required. `SolvedTopology::validate` refuses, in this order:
1. `UnsupportedSchemaVersion`;
2. `MalformedSpecificationHash`;
3. `InvalidIdentifier`: in `assignments`, the paths `assignments.<net>`, then `.pin`, `.peripheral`, `.signal`; then in `gpio`, `gpio.<net>`, then `gpio.<net>.pin`; then `i2c_ceilings.<bus>`, then `i2c_speeds.<bus>`;
4. `AlternateFunctionOutOfRange` (above 15; the net);
5. `InvalidCeiling` (a ceiling other than the three mode speeds; the bus id);
6. `NetAssignedTwice` (a net in both maps; the net);
7. `DuplicatePin` (a pin used twice anywhere across both maps; the pin);
8. the analog lines, then `CeilingNotDerived` (a ceiling that is not what `solve` derives from the bus's own lines: `electrical::ceiling_hz` of the smallest `ceiling_ns` among them; a bus with no line, or a bound slower than every mode, derives nothing; the bus id) (`AET-WO-1204`);
9. the declared speeds (`AET-WO-1205`): `SpeedBusMismatch` (the key sets of `i2c_speeds` and `i2c_ceilings` differ; the stray bus), then per bus `UnsupportedI2cSpeed` (not 100, 400 or 1,000 kHz), `BusTooFast` (above its ceiling), `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit`, each line held to `electrical::i2c_pull_up_floor_ohms` and `i2c_bus_capacitance_limit_femtofarads` at the declared speed, with the names `solve` uses;
10. the devices (`AET-WO-2203`, `4200`), then `SpiLineMismatch` (`spi_lines` is not exactly the solved SCK and MOSI assignments, a line's signal or a zero value, two rates or peripherals on one bus, or an SPI device whose bus has no line; the net or reference);
11. the motor outputs (`AET-WO-4300a`): `UnsupportedClock`, then `MotorOutputMismatch` (an assignment on a timer without an entry, or the reverse; a signal that is not `TIMn_CHk` of `TIMn`; a channel twice; indices not `0..n`; `pwm` not present exactly for PWM; `PSC` or `ARR` past 16 bits, `ARR` 0, `CCR` 0 or above `ARR`; the label), then `TimerShared` (outputs of one timer differing in protocol, PWM rate, `PSC` or `ARR`; the timer);
12. the part limits again, from what the topology carries (`AET-WO-4300c` §4.2): `AmbiguousPartName`, `PartOnWrongBus`, `I2cAddressNotOffered`, and `SpiClockAbovePart` against the bus's `rate_hz` in `spi_lines`.

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
- `PC14` and `PC15` (LSE), so the board keeps its 32 kHz crystal.

`HSE_PINS` (`PH0`, `PH1`) are reserved only beside an HSE crystal. A specification can declare none yet (`mcu.hse_hz` must be 0, `4400` D1), so the solver assigns them like any other pin (`AET-WO-4300a` §1.1). On every committed specification this moved no assignment (measured, `AET-WO-4300a` FEEDBACK P4.1).

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
3. Once every bus is placed, the motor outputs, in document order (`AET-WO-4300a`). Each tries the table's timer channels (`TIMn_CH1` to `TIMn_CH4` of a timer with channels; never a complementary `CHkN`) in the order timer number, channel, pin key, taking a free pin, a channel not taken, and a timer that is free or already carries the output's key: its protocol and, for PWM, its rate (outputs of one timer share its counter). A failure backtracks into the buses, so the search stays complete.
4. Every `cs` goes last, in `spi_devices` document order. Each takes the first free, unreserved pin in key order. This greedy pass loses no solution, because the buses and the motor outputs use a fixed number of pins.

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
| 1 | `Specification::validate` passes, and the hash is 64 lowercase hex digits | `InvalidSpecification` (the inner kind) | none |
| 1a | Every I2C line, SDA then SCL and bus by bus, has a net constraint (nothing is defaulted) | `MissingBusConstraint` (the message names the net) | the bus id |
| | … its `t_max` is at most 1000 ns | `RiseTimeTooSlow` | the bus id |
| | … and its `speed_hz` is at most its ceiling | `BusTooFast` | the bus id |
| 1b | Each bus's `pull_up_ohms` is at least `aethelgard_ir::electrical::i2c_pull_up_floor_ohms(speed_hz, rail)` (UM10204 sink current), the rail being its domain's `nominal_millivolts` | `PullUpBelowSinkFloor` | the bus id |
| | … and each line's `max_capacitance_femtofarads` is at most `i2c_bus_capacitance_limit_femtofarads(speed_hz)`, SDA then SCL | `BusCapacitanceOverLimit` | the net |
| 1c | Every SPI bus's SCK, then MOSI, bus by bus, has a net constraint (`AET-WO-4300c` D2) | `MissingSpiConstraint` (the message names the net) | the bus id |
| 2 | `mcu.part` is the table's part | `PartMismatch` | none |
| 2a | Each motor output's timing fits a timer: a whole-MHz timer clock for PWM's 1 µs tick, a period within a 16-bit `ARR`, the longest pulse inside the period as rounded | `MotorTiming` | the label |
| 3 | Every hand pin is a key of the table (compared as a key, never by case) | `UnknownPin` | the signal |
| | … and is not reserved | `ReservedPin` | the signal |
| | … and no earlier signal holds the same pin | `PinClaimedTwice` | both signals |
| 4 | Every hand peripheral is an offered instance of the bus's kind | `UnknownPeripheral` | the bus |
| | … and no earlier bus holds the same instance | `PeripheralClaimedTwice` | both buses |
| 5 | Each bus alone (reserved pins and every hand pin honoured) can take some instance. The message says why for each instance tried | `NoInstance` | the bus |
| 5a | Each motor output alone, beside every hand pin, has a timer channel (a hand pin with none, or every channel's pin held by hand) | `NoTimerChannel` | the label |
| | … two outputs whose channels all lie on one timer, the same one, share their key | `TimerRateConflict` | both labels |
| | … and together, beside every hand pin and no bus, they can all be placed; else a minimal set, by deletion | `MotorsInfeasible` | the labels in the set |
| 6 | The search places every bus. If it cannot, the result is a minimal set: each bus is dropped in document order if the rest stay infeasible. The set is irreducible, not necessarily the smallest, and costs one search per bus. With motor outputs declared, they stay in every search, and the refusal names the set's buses and every output | `Infeasible`, or `MotorsInfeasible` with motor outputs | the buses in the set (and the outputs) |
| 7 | The node budget is not spent. This refusal names only the bus being placed, by ruling, or `motor_outputs` when the outputs spent it | `BudgetExceeded` | that bus |
| 8 | Every `cs` finds a free pin | `NoFreePin` | the `cs` |

The output passes `SolvedTopology::validate` before it is returned.
