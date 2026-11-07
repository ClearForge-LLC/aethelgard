# AET-WO-1100 — The specification graph: `Specification` and `SolvedTopology`, fail-closed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-0300b` has merged (named in the kickoff; verify live HEAD before branching). This work order uses the shared input cap that 0300b introduces.
**Branch:** `wo/aet-1100-specification`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6, spend by tier), in a fresh session on Furnace.
**Phase:** P1 (One source, two projections). **Phase exit gate:** `docs/roadmap.md` §5 P1. This work order owns no clause outright. It is the input every clause reads, and it keeps the P0 gate's `validate` clause true on the new shape.

**Grounds:**
- `docs/northstar.md` §3, the end state's telemetry probe: one microcontroller, two sensors on I2C, a flash chip on SPI, one rail. §4: `INV-01`, `INV-03` and `INV-06`.
- `docs/architecture.md` §4:
  - `RUL-14`: the identifier set, and at least one voltage domain;
  - `RUL-16`: `Specification` and `SolvedTopology` are separate types, with no `HashMap`;
  - `RUL-13`: the input cap, unchanged.
- `docs/architecture.md` §2.3: the serde positional-array correction. Every new struct goes through `object_only!`.
- The code seams:
  - `crates/aethelgard-ir/src/lib.rs` (`TopologyGraph`, `object_only!`, `IrValidationError` with `kind()` and `id()`);
  - `crates/aethelgard-ir/tests/validate.rs`;
  - `crates/aethelgard-cli/src/main.rs` (`validate`);
  - `crates/aethelgard-cli/tests/validate_cli.rs`;
  - `crates/aethelgard-cli/README.md`;
  - `examples/`.

> **What this is:** the declared specification grows from domains and net constraints to the whole telemetry probe: the microcontroller, its buses, the devices on them, and which pins a person has pinned by hand. `SolvedTopology` is defined as its own type, for `AET-WO-1200` to produce. Everything is fail-closed, as in `AET-WO-0100`.
>
> It is NOT:
> - the microcontroller's pin table, which is `AET-WO-1101`;
> - the solver, which is `AET-WO-1200`;
> - checking that a named part or pin exists. That needs the pin table, so `AET-WO-1200` refuses an unknown part or pin;
> - passives other than the I2C pull-up, which `AET-WO-1300` adds;
> - any emitter.
>
> **Why now:** P0 is closing, and every P1 work order consumes this shape. `RUL-16` must hold before any solver output exists.

**Cadence:** build-only. There is no spike and no STOP for a ruling, except §7. One PR, left unmerged.

## 1. Scope

### 1.1 `Specification` (renames `TopologyGraph`)

The JSON shape, version 1. Every key shown is required. A value shown as `null` may be a string instead; the key itself is never optional (the pattern of `impedance_ohms`):

```json
{
  "schema_version": 1,
  "project_name": "sentinel-01",
  "mcu": { "part": "STM32H753ZITx", "supply": "VDD_3V3" },
  "voltage_domains": [
    { "id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50 }
  ],
  "i2c_buses": [
    {
      "id": "SENSORS", "peripheral": null, "speed_hz": 400000,
      "pull_up_ohms": 2200, "domain": "VDD_3V3",
      "sda": { "net": "I2C_SDA", "pin": null },
      "scl": { "net": "I2C_SCL", "pin": null }
    }
  ],
  "spi_buses": [
    {
      "id": "FLASH", "peripheral": null, "speed_hz": 20000000, "domain": "VDD_3V3",
      "sck": { "net": "SPI_SCK", "pin": null },
      "mosi": { "net": "SPI_MOSI", "pin": null },
      "miso": { "net": "SPI_MISO", "pin": null }
    }
  ],
  "i2c_devices": [
    { "reference": "U2", "part": "BME280", "supply": "VDD_3V3", "bus": "SENSORS", "address": 118 },
    { "reference": "U3", "part": "TMP117", "supply": "VDD_3V3", "bus": "SENSORS", "address": 72 }
  ],
  "spi_devices": [
    { "reference": "U4", "part": "W25Q64JV", "supply": "VDD_3V3", "bus": "FLASH",
      "cs": { "net": "FLASH_CS", "pin": null } }
  ],
  "net_constraints": [
    { "net_name": "I2C_SDA", "max_capacitance_femtofarads": 400000,
      "max_rise_time_nanoseconds": 300, "impedance_ohms": null }
  ]
}
```

- **Rust types:** `Specification`, `Mcu`, `VoltageDomain` (unchanged), `I2cBus`, `SpiBus`, `Signal` (`net`, `pin: Option<String>`), `I2cDevice`, `SpiDevice` and `NetConstraint` (unchanged).
  - Every one of them goes through `object_only!` and `deny_unknown_fields`.
  - Every `Option` field uses `deserialize_with = "Option::deserialize"`, so a missing key is a refusal.
  - Integer widths: `schema_version: u32`, `speed_hz: u32`, `pull_up_ohms: u32`, `address: u8`.
- **Order is content.** The arrays keep document order, so serialization is a function of content. No `HashMap` or `HashSet` appears in any type (`RUL-16`).
- **`RUL-16` holds as follows.** The lists in a `Specification` are not keyed collections in `RUL-16`'s sense. They are what a person wrote, in the order they wrote it. Nothing is sorted, unsorted input is accepted, and duplicates are refused by §1.2.
- **Lists that may be empty.** Every list except `voltage_domains` may be empty: both bus lists, both device lists, and `net_constraints`, as `RUL-14` already rules for the last.

### 1.2 `Specification::validate`

It returns the first refusal, checking in the order below, and in document order within each step. Each refusal is a new or existing `IrValidationError` variant, and `kind()` and `id()` cover every variant.

| # | Rule | Refusal (`id()` returns) |
|---|---|---|
| 1 | `schema_version` is 1 | `UnsupportedSchemaVersion` (none) |
| 2 | Every identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E` (`RUL-14`). The identifiers, and the order they are checked in, are listed below the table | `InvalidIdentifier` (none). It carries the JSON path of the field, such as `i2c_buses[0].sda.net`. It replaces `EmptyProjectName`, `EmptyDomainId` and `EmptyNetName`, whose cases it covers. Comparisons are now exact, with no trimming, because whitespace is refused |
| 3 | `voltage_domains` is not empty | `NoVoltageDomain` (none) |
| 4 | Domains: unique ids, nominal voltage not 0, tolerance in 1..=500 | `DuplicateDomainId`, `ZeroNominalVoltage`, `ToleranceOutOfRange`, as today |
| 5 | `mcu.supply`, every bus `domain` and every device `supply` names a declared domain | `UnknownDomain` (the name) |
| 6 | Bus ids are unique across both bus lists | `DuplicateBusId` (the id) |
| 7 | An I2C `speed_hz` is 100000, 400000 or 1000000, and `pull_up_ohms` is not 0. An SPI `speed_hz` is not 0 | `UnsupportedI2cSpeed`, `ZeroPullUp`, `ZeroSpiSpeed` (the bus id) |
| 8 | References are unique across both device lists | `DuplicateReference` (the reference) |
| 9 | A device's `bus` names a bus of its own kind | `UnknownBus` (the name) |
| 10 | An I2C address is in `0x08..=0x77` and unique on its bus | `I2cAddressOutOfRange`, `DuplicateI2cAddress` (the device reference) |
| 11 | A net is declared by exactly one signal across every bus and every `cs`, and no signal net has the same name as a voltage domain id | `DuplicateNetName`, `NetShadowsDomain` (the net) |
| 12 | Net constraints: unique `net_name`; each names a declared signal net or a voltage domain id; and capacitance, rise time and impedance are not 0, as today | `DuplicateNetConstraint`, `UnknownNet`, `ZeroCapacitance`, `ZeroRiseTime`, `ZeroImpedance` |

**Step 2's identifiers, in check order.** This is §1.1's key order, with each list walked by index:
1. `project_name`;
2. `mcu.part`, then `mcu.supply`;
3. `voltage_domains[i].id`;
4. for each `i2c_buses[i]`: `id`, `peripheral` if set, `domain`, then for `sda` and then `scl`, `net` followed by `pin` if set;
5. for each `spi_buses[i]`: `id`, `peripheral` if set, `domain`, then for `sck`, `mosi` and `miso`, `net` followed by `pin` if set;
6. for each `i2c_devices[i]`: `reference`, `part`, `supply`, `bus`;
7. for each `spi_devices[i]`: `reference`, `part`, `supply`, `bus`, `cs.net`, then `cs.pin` if set;
8. `net_constraints[i].net_name`.

The path is written exactly as shown, for example `spi_devices[0].cs.net`. A `SolvedTopology` path is `assignments.<net>` for the key itself, and `assignments.<net>.pin`, `.peripheral` or `.signal` for a value.

`validate` does **not** check:
- that a part, a peripheral or a pin exists;
- that two signals ask for the same pin.

The solver does both, because both need the pin table (`AET-WO-1200`). That is also where the P1 gate puts the double-assigned pin.

### 1.3 `SolvedTopology` (`RUL-16`)

```rust
pub struct SolvedTopology {
    pub schema_version: u32,               // 1
    pub specification_sha256: String,      // 64 lowercase hex digits
    pub assignments: BTreeMap<String, PinAssignment>, // keyed by net
}
pub struct PinAssignment {
    pub pin: String,                       // e.g. "PB8"
    pub peripheral: String,                // e.g. "I2C1"
    pub signal: String,                    // e.g. "I2C1_SCL"
    pub alternate_function: u8,            // 0..=15
}
```

- Both go through `object_only!`.
- `SolvedTopology::validate` refuses, in this order:
  - a version other than 1: `UnsupportedSchemaVersion`;
  - a hash that is not exactly 64 lowercase hex digits: `MalformedSpecificationHash`;
  - a non-identifier, under step 2's rule and path format: `InvalidIdentifier`;
  - an alternate function above 15: `AlternateFunctionOutOfRange`, naming the net;
  - one pin assigned to two nets: `DuplicatePin`, naming the pin.
- **A repeated key in `assignments` is refused at deserialization.** serde's derived `BTreeMap` keeps the last value of a repeated key and drops the first without a word. So `assignments` uses a `deserialize_with` that refuses any key seen twice, and the CLI then reports it like any other schema error. Test it.
- Nothing in this work order constructs one, outside tests.
- The CLI's `validate` accepts only a `Specification`. A `SolvedTopology` document given to it is refused by `deny_unknown_fields`, with the same exit code and kind as any other unknown field. Test it.

### 1.4 The CLI, the examples and the README

- **`validate`** deserializes a `Specification`. Its report keeps the same fields and its exit codes are unchanged. Counts such as `"net_constraints"` follow the new example, so `valid_example_exits_0` and the README's success line are updated to the new numbers. Do not pad the example to keep an old count. `InputTooLarge` uses the shared cap that `AET-WO-0300b` introduced.
- **Tests that change meaning, and are rewritten rather than deleted:**
  - in `crates/aethelgard-ir/tests/validate.rs`, the three `refuses_empty_*` tests become `InvalidIdentifier` tests;
  - the two `…including_whitespace_variants` duplicate tests become tests that a padded id is an `InvalidIdentifier`;
  - every helper that builds a `TopologyGraph` builds a `Specification`.
- **The README's sentence on "the trimmed offending id"** is rewritten, because nothing is trimmed any more.
- **`examples/sentinel-01-minimal.json`** becomes the §1.1 document. **`examples/invalid-zero-domain.json`** becomes the same document with `nominal_millivolts` 0. Both keep their file names, so the `gates` step that validates them needs no change, and `.github/workflows/` stays untouched.
- **`crates/aethelgard-cli/README.md`:** the `validate` section, and the kinds in the exit-2 row.
- **`crates/aethelgard-ir`** gains a short `README.md` holding the §1.1 shape and the §1.2 table.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06`.** Every missing key, unknown key, positional array, empty list where one is required, and dangling reference is refused with a named kind. Nothing defaults.
- **`INV-01`.** One representation. `aethelgard-ir` still depends on no other member, and gains no dependency: `BTreeMap` is `std`.
- **No `unsafe`.** The workspace-shape tests stay green and unchanged.
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/`, `.github/workflows/`;
  - every crate except `aethelgard-ir` and `aethelgard-cli`.

  In `aethelgard-cli`, these may change:
  - in `src/main.rs`: the `validate` function, the `use aethelgard_ir::…` import, and the `Validate` subcommand's doc comment (which changes `--help`);
  - `tests/validate_cli.rs`;
  - `README.md`.
- One PR, left unmerged. Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. One test per refusal in §1.2 and §1.3, each asserting the kind and `id()`, and the JSON path for `InvalidIdentifier`. Identifier cases include:
   - a Cyrillic `В`;
   - a zero-width space;
   - a leading space;
   - 65 bytes.
2. The serde edge cases: a missing `pin` key, a positional array for each new struct, an unknown field in each new struct, and a duplicate key. Each is refused.
3. Round trip: `Specification` and `SolvedTopology` each serialize and deserialize to an equal value, and re-serializing gives identical bytes.
4. The CLI:
   - both examples behave as the `gates` step expects, exit 0 and exit 2, with the zeroed domain named;
   - a `SolvedTopology` document is refused by `validate`.
5. The workspace gates pass in `gates` and on Furnace, and `tools` stays green. CI is green on the first push. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- The pin table, part catalogue, solver, emitters or board crate.
- New device kinds, other buses (UART, CAN, USB), passives beyond the I2C pull-up, or a second rail's regulator.
- A schema migration tool for version 0 documents. None exist outside `examples/`.

## 5. Adversarial pass — targeted

Every finding goes in FEEDBACK with a severity.

1. **Smuggle a solved field into a `Specification`,** and a specification field into a `SolvedTopology`. Both must be refused.
2. **Find an identifier the rule lets through** that two humans would read as the same, or that KiCad or Rust would refuse as a name.
3. **Find any order dependence:** a `HashSet` iteration that changes which refusal is reported first, or serialization bytes that depend on anything but content.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1100.FEEDBACK.md`, same schema as before. It carries:
- the gates line with CI run URLs;
- what was not built;
- any rule in §1.2 the build showed to be wrong or missing, as decision-needed, not silently changed;
- a promote-or-needs-none pass over `AET-WO-0300b`'s and `AET-WO-0400`'s FEEDBACK. If either file is absent on the base, record "not yet merged" for it and move on.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- A rule in §1.2 that cannot be enforced without the pin table.
- A shape in §1.1 that the reference probe cannot be written in.
- A new dependency, `unsafe`, or a change outside the surfaces in §2.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-1100: §1 is built and every §3 item passes in gates and on Furnace, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1100-specification.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
