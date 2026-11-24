# AET-WO-4300k key-deletion sweep (SWEEP.md)

Measured on commit 521dcd06 (branch `wo/aet-4300k-sentinel-03-readiness`), host only, no container, no KiCad. Nothing under crates/, examples/, fab/, docs/ or scenarios/ was edited.

## Answer first

Five keys relax a judgement on a board that is physically the same (same BOM) while `validate` exits 0. All five are visible in the output of a command that already runs on the host:

| key (JSON path) | absence | what the gate says without it | type | gate |
|---|---|---|---|---|
| `power.drop_limits` (Option<Vec<DropLimit>>; delete and `[]` accepted, `null` refused) | extract's supply gate is off | `extract:0.ran[1]`: `{"gate":"extract.supply","judged":1}` becomes `{"gate":"extract.supply","off":"power.drop_limits"}`; the whole `supplies[VDD_3V3]` row (drop_bound_microvolts 90032 against limit_millivolts 165) is gone | crates/aethelgard-ir/src/power.rs:128 | crates/aethelgard-eda/src/supply.rs:1342-1356 |
| `board.stackup.fab_tolerances` (Option<FabTolerances>; delete accepted, `null` and `{}` refused/emit-refused) | the via ring rule is None: emit writes no `capacitance_ladder`, and raises no thermal via | emit loses `"capacitance_ladder"` and `"notes": ["via ring: U10 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill", ...]`; extract's own findings print `"the router's capacitance ladder runs only when `board.stackup.fab_tolerances` is declared, and this specification declares none, so its lines were routed once with no regard to their bounds"` only when a line then fails | crates/aethelgard-ir/src/board.rs:184 | crates/aethelgard-eda/src/stack.rs:64 (`RingRule::of`), schematic.rs:957 (`ladder_runs`) |
| `diff_pairs` (required key, `[]` accepted) | extract.pairs off | `ran[3]`: `{"gate":"extract.pairs","judged":2}` becomes `{"gate":"extract.pairs","off":"diff_pairs"}`; `pairs[USB_STANDIN]` and its `"mismatch": "unverified: ..."` row are gone | crates/aethelgard-ir/src/lib.rs:197 | crates/aethelgard-cli/src/extract.rs:101, 552 |
| `power.conditions.dc_bias_derating` (required key, `[]` accepted) | `cout` is judged at rated capacitance | `solve:0.power.conditions`: `"U10: `cout` would hold 44000000 pF effective at 5250 mV under DC bias (...)"` is replaced by `"U10: `cout` is judged at rated capacitance; its row states no effective minimum under DC bias"` (same for U11) | crates/aethelgard-ir/src/power.rs:147 | crates/aethelgard-ir/src/power.rs:2108 |
| `power.loads` (required key, `[]` accepted) | each regulator's junction screen runs with no load | `"U10: its junction is screened, not guaranteed: 1170 mW of loss at 2000 mA (...)"` becomes `"... 1170 mW of loss at 0 mA (...)"`; `power.domains.VDD_5V.load_milliamps` 2000 -> 0 | crates/aethelgard-ir/src/power.rs:116 | crates/aethelgard-ir/src/power.rs:2696 |

(The first two are the known positives from the brief; the detector found both with no tuning. The last three are new: each is a required key that accepts an empty list, so strictly they are "empty list relaxes", not "optional key absent".)

The detector sees the two known positives, so it is not broken: `power.drop_limits` rows below, and `board.stackup.fab_tolerances` rows.

## What the engine already prints

`extract` already reports a gate that is off: `ran[]` entries of the form `{"gate":"extract.supply","off":"power.drop_limits"}`, `{"gate":"extract.oscillator","off":"mcu.hse.crystal"}`, `{"gate":"extract.pairs","off":"diff_pairs"}`. On sentinel-03 (no drop_limits, no diff_pairs, a non-crystal HSE) all three are `off` in the baseline today. `emit` prints nothing when the ring rule is absent (it omits `capacitance_ladder` and `notes`; there is no `off` line), and `solve` prints nothing at all for the two power cases except the changed `conditions` line. So the supply, pairs and oscillator gates self-report; the ring rule, derating and loads do not.

## Method

1. Keys enumerated from the IR source, not by hand: `spike/4300k/sweep/enumerate.py` parses every `struct` in crates/aethelgard-ir/src, walks the type graph from `Specification`, and emits each field that is `Option<..>`, `serde(default)`, `skip_serializing_if`, a `Vec`/set (may be empty) or a map, with JSON path, struct, type, attributes, file and line. 108 entries (`/home/forge/.cache/aet-4300k/sweep/keys.json`). Two walker artifacts: `power.conditions.dc_bias_derating[].dielectrics[].between[]` and `.max_thickness_um` are not real keys (that `dielectrics` is a list of dielectric-class strings; the walker resolved the name `Dielectric` to board.rs's struct).
2. Carriers. Primary: crates/aethelgard-cli/tests/fixtures/sentinel-03.json. A key sentinel-03 does not declare (non-null, non-empty) takes the first committed specification from `bash spike/4300k/universe.sh` that does. With `--all-carriers` (pass 2) up to four carriers per key. In-memory copies (not committed): `sentinel-03+gated` is sentinel-03 plus supply_tier1.rs:49-60's four stackup minima (outer_copper_min_um 30, plane_copper_min_um 15, plating_min_um 18, copper_temperature_max_c 125, each with a "test:" source) and `power.drop_limits` VDD_3V3 165 mV; it is the carrier for the five supply-gate keys and all `fab_tolerances` keys. Synthetic carriers for keys no committed specification gives a value: `syn:s03+impedance_ohms=50`, `syn:s03+dielectric max_thickness_um=min+40` (and a gated copy), `syn:s03+serial peripheral,negotiated` (CRSF baud set to 921600, which validate accepts as negotiated).
3. Variants per key: `delete`, `null`, `empty` (`[]` for a list, `{}` for an object; n/a for a scalar), `rename` (key + `_x`). Applied to every element a path's `[]` crosses.
4. Detector: for each variant `validate` exit and error kind; if exit 0, `solve`, `emit --out`, `extract --board <emitted pcb> --spec`, `check speeds`. Each JSON output is flattened to (command, path, value) facts (sha256 and file paths dropped, array elements identified by net/domain/name/id), and compared with the same carrier's own baseline: a fact gone, new, or changed. Numeric changes are collapsed (they follow placement). Class is then assigned by reading each diff (table.py, MANUAL).
5. Run in a capped scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`), 6 workers, with a release build of the CLI in a separate target dir (`CARGO_TARGET_DIR=/home/forge/.cache/aet-4300k/sweep/target-rel`), because a gated `extract` takes 5 min 22 s even in release. The release build is the same source (`cargo build --offline --release -p aethelgard-cli`); checked on sentinel-03: emitted board sha256 da09ab13bd83... is identical, and `solve`, `extract` and `check speeds` outputs are identical apart from the `input` path.

## Commands

```
export PATH=$HOME/.cargo/bin:$PATH
cd /home/forge/aethelgard-4300k
python3 -I spike/4300k/sweep/enumerate.py > /home/forge/.cache/aet-4300k/sweep/keys.json
bash spike/4300k/universe.sh > /home/forge/.cache/aet-4300k/sweep/universe.tsv
export XDG_RUNTIME_DIR=/run/user/$(id -u) AET_BIN=/home/forge/.cache/aet-4300k/sweep/target-rel/release/aethelgard
systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0 --expand-environment=no python3 -I spike/4300k/sweep/sweep.py                        # pass 1: results.json (copied to results-pass1.json)
... python3 -I spike/4300k/sweep/sweep.py --all-carriers --paths '<keys that showed no change>' --out results-pass2   # pass 2
... python3 -I spike/4300k/sweep/sweep.py --paths 'serial_ports[].peripheral,serial_ports[].negotiated' --out results-pass3
... python3 -I spike/4300k/sweep/sweep.py --paths 'board.stackup.dielectrics[].max_thickness_um' --out results-pass4
python3 -I spike/4300k/sweep/table.py > table.md          # the table below
python3 -I spike/4300k/sweep/show.py '<path>' [variant] [results file]   # the full diff of one key
python3 -I spike/4300k/sweep/bomdiff.py                   # BOM before/after for the keys whose absence removes a declared thing
```
Per-variant commands are exactly: `aethelgard validate --file F`; `solve --file F`; `emit --file F --out DIR`; `extract --board DIR/<project>.kicad_pcb --spec F`; `check speeds --spec F`.

## Not run, and why

- `check erc`, `check drc`, `check support`: need `kicad-cli`, not on the host. Nothing here says whether `check support` (the 4300g support network) reads any of these keys differently. `check pinout` needs an ELF; `generate`, `verify`, `build`, `prove`, `simulate`, `campaign` start the sandbox or take 20 s+ each (`generate` took 19.9 s on one run) and were not swept. `mcu.foreign_clock`'s reader is `simulate --foreign`, not run.
- Never non-null in any committed specification and not synthesised: `power.regulators[].controls[].wire`, `power.crossings[].controls[].wire` (needs a wired net: `Wire {net, role}` and exactly one of `strap`/`wire` null), `pin_devices[].signals[].pin`, `pin_devices[].analog[].pin`, `pin_devices[].analog[].channel` (all solver-freedom keys; `pin` null means the solver chooses, which cannot remove a judgement, but I did not run it), and `pin_devices[].part_limits.input_pwm_max_hz` (the one partial case: for DRV8876PWPR validate refuses a declaration with `PartLimitDeclaredTwice` because the part row states 100000 Hz, p. 4; for a part whose row is silent it is required: `motor_outputs[].pin` null on d2-drv8833 gives `PartLimitUndeclared`; so validate already forces it exactly once).
- `check speeds` costs about 15-19 s per call even in release; it was run on every variant validate accepted.

## Surprising

- Two board.stackup.fab_tolerances fields, `hole_size_minus_um` and `track_width_permille`, are read only by the supply gate (supply.rs:1384, 1386). On sentinel-03 their deletion changes nothing; on the gated carrier extract refuses with `SupplyInputUndeclared`. So they are conservative, but only because `drop_limits` is declared; a readiness rule that wants them must say "when drop_limits".
- `board.stackup.fab_tolerances` partially declared (one of `hole_position_um`, `hole_size_plus_um`, `via_ring_um` missing) passes `validate` and is refused later, by `emit` (FabToleranceUndeclared). Wholly absent passes everything. `null` and `{}`: `null` is refused by validate (SchemaViolation), `{}` is refused by emit.
- `power.drop_limits` and `board.stackup.fab_tolerances` accept `delete` but refuse `null` (the `present` deserialiser), so "absent" and "explicit null" are different today.
- `power.loads` and `power.sense` and `diff_pairs` and `dc_bias_derating` are required keys, yet validate accepts `[]`, so INV-06 "required as a key" does not stop an empty list from relaxing a bound.
- Every key whose `null` is refused by validate (sda/scl/mosi/miso pins in i2c-spi-bench, `connectors[].pins[].net`, `.offboard`, `pin_devices[].bulk`, `.temperature_celsius`, `regulators[].junction`, `.feedback`, `sense[].reservoir`, `spi_devices[].pin_capacitance`) is already conservative: refused by name.
- 403 of the table's rows are refused by validate. Every rename is refused (deny_unknown_fields), and every delete of a required key is a `SchemaViolation` (missing field).
- sentinel-02-power's baseline `extract` already refuses (TranslatorPinUnbounded), so extract cannot witness anything on that carrier.

## Class legend

`relaxes`: validate exit 0 and a judgement, condition line or bound is removed or weakened on the same board (same BOM). `conservative`: absence removes a part and its judgement together (BOM diff), makes a bound stricter, or is refused downstream. `refused`: validate exits 2 (kind named). `refused-later`: validate exits 0, a later command refuses by name (the command and kind are in the cell). `no change`: no fact of any host command differs except placement-driven numbers. `n/a`: a scalar has no `{}` / `[]` form. `untested`: no carrier.

## Table (one row per key path, carrier, variant)

| key path | carrier | variant | validate | class | what changed (quoted) | type |
|---|---|---|---|---|---|---|
| `mcu.hse` | hse-crystal-16 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:373 |
| `mcu.hse` | hse-crystal-16 | null | exit 0 [exit extract=1] | conservative | removes the crystal (BOM loses Y1, C20, C21, C26-C29; adds bypass caps); the oscillator gate goes off with its subject; clock falls to HSI: gone `extract:0.ran[2].judged`: 2<br>gone `extract:0.oscillator[U1_OSC_IN].crystal`: "Y1"<br>chg `solve:0.clock.source`: "PLL1" / "HSI" | lib.rs:373 |
| `mcu.hse` | hse-crystal-16 | empty | exit 2 | refused | validate exit 2 InvalidJson | lib.rs:373 |
| `mcu.hse` | hse-crystal-16 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:373 |
| `mcu.supply_source` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:383 |
| `mcu.supply_source` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:383 |
| `mcu.supply_source` | i2c-spi-bench | empty | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:383 |
| `mcu.supply_source` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:383 |
| `mcu.reset_switch` | sentinel-03 | delete | exit 0 | conservative | removes SW1 from the BOM; no judgement removed (placement only): chg `emit:0.capacitance_ladder.bounded_first`: false / true | lib.rs:387 |
| `mcu.reset_switch` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:387 |
| `mcu.reset_switch` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:387 |
| `mcu.reset_switch` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:387 |
| `mcu.foreign_clock` | sentinel-02-own | delete | exit 0 | conservative | solve drops the foreign_clock block; the reader is simulate --foreign, which was not run: new `solve:0.foreign_clock`: null | lib.rs:396 |
| `mcu.foreign_clock` | sentinel-02-own | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:396 |
| `mcu.foreign_clock` | sentinel-02-own | empty | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:396 |
| `mcu.foreign_clock` | sentinel-02-own | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:396 |
| `mcu.foreign_clock.usarts{}` | sentinel-02-own | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:658 |
| `mcu.foreign_clock.usarts{}` | sentinel-02-own | null | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:658 |
| `mcu.foreign_clock.usarts{}` | sentinel-02-own | empty | exit 2 | refused | validate exit 2 ForeignClockUsarts | motor.rs:658 |
| `mcu.foreign_clock.usarts{}` | sentinel-02-own | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:658 |
| `mcu.reserved_pins[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:401 |
| `mcu.reserved_pins[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:401 |
| `mcu.reserved_pins[]` | sentinel-03 | empty | exit 0 | no change |  | lib.rs:401 |
| `mcu.reserved_pins[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:401 |
| `voltage_domains[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:178 |
| `voltage_domains[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:178 |
| `voltage_domains[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 NoVoltageDomain | lib.rs:178 |
| `voltage_domains[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:178 |
| `i2c_buses[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:179 |
| `i2c_buses[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:179 |
| `i2c_buses[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 UnknownBus | lib.rs:179 |
| `i2c_buses[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:179 |
| `i2c_buses[].peripheral` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:472 |
| `i2c_buses[].peripheral` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:472 |
| `i2c_buses[].peripheral` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:472 |
| `i2c_buses[].peripheral` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:472 |
| `i2c_buses[].sda.pin` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `i2c_buses[].sda.pin` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:464 |
| `i2c_buses[].sda.pin` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `i2c_buses[].sda.pin` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `i2c_buses[].scl.pin` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `i2c_buses[].scl.pin` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:464 |
| `i2c_buses[].scl.pin` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `i2c_buses[].scl.pin` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:180 |
| `spi_buses[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:180 |
| `spi_buses[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 UnknownBus | lib.rs:180 |
| `spi_buses[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:180 |
| `spi_buses[].peripheral` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:485 |
| `spi_buses[].peripheral` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:485 |
| `spi_buses[].peripheral` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:485 |
| `spi_buses[].peripheral` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:485 |
| `spi_buses[].sck.pin` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-03 | null | exit 0 | no change | solver freedom: a pinned pin becomes the solver's choice; may move the board (on pair-standin emit then says Unroutable, on sentinel-02-own a line fails); no judgement removed | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].mosi.pin` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].mosi.pin` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:464 |
| `spi_buses[].mosi.pin` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].mosi.pin` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].miso.pin` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].miso.pin` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:464 |
| `spi_buses[].miso.pin` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].miso.pin` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `i2c_devices[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:184 |
| `i2c_devices[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:184 |
| `i2c_devices[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 LoadNamesNoPart | lib.rs:184 |
| `i2c_devices[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:184 |
| `i2c_devices[].pin_capacitance` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:504 |
| `i2c_devices[].pin_capacitance` | sentinel-03 | null | exit 0 | conservative | STRICTER: the declared 10000 fF per pin becomes the UM10204 10 pF default, terms.pins 20000 -> 30000: rem `extract:0.conditions[*]`: "U7 (BMP581): each of its pins on a bounded line counted at 10000 fF, a value the specification declares, not a maximum the datasheet prints; the bound holds only under its condition: each of U7's pins on SDA and SCL at most UM102 / "every I2C pin not counted at a declared value at most UM10204's Ci of 10 pF; no multiplexer or switch on the line"<br>gone `extract:0.lines[I2C_SCL].declared_pins[*]`: "U7.2" | lib.rs:504 |
| `i2c_devices[].pin_capacitance` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:504 |
| `i2c_devices[].pin_capacitance` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:504 |
| `spi_devices[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:187 |
| `spi_devices[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:187 |
| `spi_devices[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 LifecycleAcceptanceUnused | lib.rs:187 |
| `spi_devices[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:187 |
| `spi_devices[].cs.pin` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_devices[].cs.pin` | sentinel-03 | null | exit 0 | no change | solver chooses the pin (IMU_CS PB12 -> PA1); no judgement removed: chg `solve:0.gpio.IMU_CS.pin`: "PB12" / "PA1" | lib.rs:464 |
| `spi_devices[].cs.pin` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_devices[].cs.pin` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_devices[].pin_capacitance` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:517 |
| `spi_devices[].pin_capacitance` | sentinel-03 | null | exit 2 | refused | validate exit 2 UnboundedPinCapacitance | lib.rs:517 |
| `spi_devices[].pin_capacitance` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:517 |
| `spi_devices[].pin_capacitance` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:517 |
| `spi_devices[].orientation` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:527 |
| `spi_devices[].orientation` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:527 |
| `spi_devices[].orientation` | i2c-spi-bench | empty | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:527 |
| `spi_devices[].orientation` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:527 |
| `connectors[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:190 |
| `connectors[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:190 |
| `connectors[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SerialPortNotAtConnector | lib.rs:190 |
| `connectors[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:190 |
| `connectors[].standard` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | sentinel-03 | null | exit 0 | no change | no output differs on the host commands | connectors.rs:209 |
| `connectors[].standard` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:209 |
| `connectors[].standard` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].pins[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:210 |
| `connectors[].pins[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:210 |
| `connectors[].pins[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ConnectorPinMissing | connectors.rs:210 |
| `connectors[].pins[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:210 |
| `connectors[].pins[].net` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:220 |
| `connectors[].pins[].net` | sentinel-03 | null | exit 2 | refused | validate exit 2 ConnectorSupplyContradictsStandard | connectors.rs:220 |
| `connectors[].pins[].net` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:220 |
| `connectors[].pins[].net` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:220 |
| `connectors[].pins[].offboard` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:222 |
| `connectors[].pins[].offboard` | sentinel-03 | null | exit 2 | refused | validate exit 2 ConnectorBudgetMissing | connectors.rs:222 |
| `connectors[].pins[].offboard` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:222 |
| `connectors[].pins[].offboard` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:222 |
| `net_constraints[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:191 |
| `net_constraints[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:191 |
| `net_constraints[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 MissingBusConstraint | lib.rs:191 |
| `net_constraints[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:191 |
| `board.layers[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:66 |
| `board.layers[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:66 |
| `board.layers[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 LayerTable | board.rs:66 |
| `board.layers[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:66 |
| `board.layers[].net` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:125 |
| `board.layers[].net` | sentinel-03 | null | exit 2 | refused | validate exit 2 LayerRole | board.rs:125 |
| `board.layers[].net` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | board.rs:125 |
| `board.layers[].net` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:125 |
| `board.stackup.dielectrics[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:140 |
| `board.stackup.dielectrics[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:140 |
| `board.stackup.dielectrics[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 DielectricTable | board.rs:140 |
| `board.stackup.dielectrics[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:140 |
| `board.stackup.dielectrics[].between[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:244 |
| `board.stackup.dielectrics[].between[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:244 |
| `board.stackup.dielectrics[].between[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 DielectricTable | board.rs:244 |
| `board.stackup.dielectrics[].between[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:244 |
| `board.stackup.outer_copper_min_um` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.outer_copper_min_um`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:156 |
| `board.stackup.outer_copper_min_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:156 |
| `board.stackup.outer_copper_min_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:156 |
| `board.stackup.outer_copper_min_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:156 |
| `board.stackup.plane_copper_min_um` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.plane_copper_min_um`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:162 |
| `board.stackup.plane_copper_min_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:162 |
| `board.stackup.plane_copper_min_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:162 |
| `board.stackup.plane_copper_min_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:162 |
| `board.stackup.plating_min_um` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.plating_min_um`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:168 |
| `board.stackup.plating_min_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:168 |
| `board.stackup.plating_min_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:168 |
| `board.stackup.plating_min_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:168 |
| `board.stackup.copper_temperature_max_c` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.copper_temperature_max_c`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:174 |
| `board.stackup.copper_temperature_max_c` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:174 |
| `board.stackup.copper_temperature_max_c` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:174 |
| `board.stackup.copper_temperature_max_c` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:174 |
| `board.stackup.fab_tolerances` | sentinel-03 | delete | exit 0 | relaxes | RingRule::of is None: emit writes no capacitance_ladder and raises no thermal via; the board and BOM are the same: gone `emit:0.capacitance_ladder.rung`: 2<br>gone `emit:0.notes[*]`: "via ring: U10 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill" / "via ring: U11 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill"<br>gone `emit:0.capacitance_ladder.judged_by`: "extract, run separately on the emitted board (RUL-20)" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03 | empty | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03 | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_position_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03 | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03 | delete | exit 0 | no change | read only by the supply gate (supply.rs:1384): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract) | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03 | delete | exit 0 | no change | read only by the supply gate (supply.rs:1386): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract) | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03 | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.via_ring_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.max_outline_um` | sentinel-03 | delete | exit 0 | no change | no gate I ran reads it; absence is an unbounded outline (ladder numbers move with placement). Ruled optional in PR #191 Q1: chg `emit:0.capacitance_ladder.bounded_first`: false / true | board.rs:80 |
| `board.max_outline_um` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:80 |
| `board.max_outline_um` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:80 |
| `board.max_outline_um` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:80 |
| `board.forward` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 InvalidIdentifier | board.rs:89 |
| `board.forward` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidJson | board.rs:89 |
| `board.forward` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | board.rs:89 |
| `board.forward` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:89 |
| `board.drawn` | hse-external-8 | delete | exit 0 [exit emit=2] | conservative | absent is true, the strict case: emit refuses the twin stand-in by another rule (HseSourceNotDrawn): chg `emit:0.error.kind`: "DrawsNoBoard" / "HseSourceNotDrawn" | board.rs:98 |
| `board.drawn` | hse-external-8 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:98 |
| `board.drawn` | hse-external-8 | empty | - | n/a | scalar: {} / [] does not apply | board.rs:98 |
| `board.drawn` | hse-external-8 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:98 |
| `net_classes[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:195 |
| `net_classes[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:195 |
| `net_classes[]` | sentinel-03 | empty | exit 0 | no change | no judgement removed on host; routing and the ladder rung move: chg `emit:0.capacitance_ladder.bounded_first`: false / true | lib.rs:195 |
| `net_classes[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:195 |
| `net_classes[].via` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].via` | sentinel-03 | null | exit 0 | no change | geometry only | classes.rs:60 |
| `net_classes[].via` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | classes.rs:60 |
| `net_classes[].via` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].nets[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:61 |
| `net_classes[].nets[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:61 |
| `net_classes[].nets[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ClassNoNets | classes.rs:61 |
| `net_classes[].nets[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:61 |
| `diff_pairs[]` | pair-standin | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:197 |
| `diff_pairs[]` | pair-standin | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:197 |
| `diff_pairs[]` | pair-standin | empty | exit 0 | relaxes | extract.pairs goes off and the pair rows go; BOM identical; the same nets are still extracted as lines: gone `extract:0.ran[3].judged`: 2<br>gone `extract:0.pairs[USB_STANDIN].mismatch`: "unverified: the conductors' capacitance mismatch is a stated condition of the router's symmetric pair routing (AET-WO-1303 D2), not bounded"<br>gone `extract:0.pairs[USB_STANDIN].name`: "USB_STANDIN" | lib.rs:197 |
| `diff_pairs[]` | pair-standin | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:197 |
| `diff_pairs[].nets[]` | pair-standin | delete | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:76 |
| `diff_pairs[].nets[]` | pair-standin | null | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:76 |
| `diff_pairs[].nets[]` | pair-standin | empty | exit 2 | refused | validate exit 2 PairNets | classes.rs:76 |
| `diff_pairs[].nets[]` | pair-standin | rename | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:76 |
| `motor_outputs[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:199 |
| `motor_outputs[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:199 |
| `motor_outputs[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ConnectorNetUnknown | lib.rs:199 |
| `motor_outputs[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:199 |
| `motor_outputs[].pin` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | sentinel-03 | null | exit 0 | no change | solver freedom; pins move, no judgement removed | motor.rs:170 |
| `motor_outputs[].pin` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | motor.rs:170 |
| `motor_outputs[].pin` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pwm` | driver-judges-nothing | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:173 |
| `motor_outputs[].pwm` | driver-judges-nothing | null | exit 2 | refused | validate exit 2 MotorTimingShape | motor.rs:173 |
| `motor_outputs[].pwm` | driver-judges-nothing | empty | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:173 |
| `motor_outputs[].pwm` | driver-judges-nothing | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:173 |
| `motor_outputs[].esc` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:175 |
| `motor_outputs[].esc` | sentinel-03 | null | exit 0 | no change |  | motor.rs:175 |
| `motor_outputs[].esc` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:175 |
| `motor_outputs[].esc` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:175 |
| `power` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:203 |
| `power` | sentinel-03 | null | exit 2 | refused | validate exit 2 ClassUnknownNet | lib.rs:203 |
| `power` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:203 |
| `power` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:203 |
| `power.conditions.dc_bias_derating[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:147 |
| `power.conditions.dc_bias_derating[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:147 |
| `power.conditions.dc_bias_derating[]` | sentinel-03 | empty | exit 0 | relaxes | the cout bound is judged at rated, not derated, capacitance; BOM identical: rem `solve:0.power.conditions[*]`: "U10: `cout` would hold 44000000 pF effective at 5250 mV under DC bias (GRM32ER61C226KE20: 11000000 pF, 500 per mille of its rating kept at 329 per mille of its rated voltage, by the declared derating, a placeholder until its make / "U11: `cout` would hold 44000000 pF effective at 3465 mV under DC bias (GRM32ER71A226KE20: 11000000 pF, 500 per mille of its rating kept at 347 per mille of its rated voltage, by the declared derating, a placeholder until its make | power.rs:147 |
| `power.conditions.dc_bias_derating[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:147 |
| `power.conditions.dc_bias_derating[].dielectrics[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:156 |
| `power.conditions.dc_bias_derating[].dielectrics[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:156 |
| `power.conditions.dc_bias_derating[].dielectrics[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 PowerValueInvalid | power.rs:156 |
| `power.conditions.dc_bias_derating[].dielectrics[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:156 |
| `power.conditions.dc_bias_derating[].dielectrics[].between[]` | - | - | - | untested | no committed specification declares a value for it | : |
| `power.conditions.dc_bias_derating[].dielectrics[].max_thickness_um` | - | - | - | untested | no committed specification declares a value for it | : |
| `power.input.hot_plug[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:178 |
| `power.input.hot_plug[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:178 |
| `power.input.hot_plug[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 BatteryInputUndamped | power.rs:178 |
| `power.input.hot_plug[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:178 |
| `power.battery.series_milliohms` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-03 | null | exit 0 | no change | no output differs on the host commands | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.regulators[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:115 |
| `power.regulators[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:115 |
| `power.regulators[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 DomainWithoutSource | power.rs:115 |
| `power.regulators[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:115 |
| `power.regulators[].feedback` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:242 |
| `power.regulators[].feedback` | sentinel-03 | null | exit 2 | refused | validate exit 2 ExternalPartMissing | power.rs:242 |
| `power.regulators[].feedback` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:242 |
| `power.regulators[].feedback` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:242 |
| `power.regulators[].external[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:243 |
| `power.regulators[].external[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:243 |
| `power.regulators[].external[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ExternalPartMissing | power.rs:243 |
| `power.regulators[].external[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:243 |
| `power.regulators[].junction` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:247 |
| `power.regulators[].junction` | sentinel-03 | null | exit 2 | refused | validate exit 2 BuckJunctionOverLimit | power.rs:247 |
| `power.regulators[].junction` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:247 |
| `power.regulators[].junction` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:247 |
| `power.regulators[].controls[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:249 |
| `power.regulators[].controls[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:249 |
| `power.regulators[].controls[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ControlPinUnwired | power.rs:249 |
| `power.regulators[].controls[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:249 |
| `power.regulators[].controls[].strap` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:269 |
| `power.regulators[].controls[].strap` | sentinel-03 | null | exit 2 | refused | validate exit 2 PowerValueInvalid | power.rs:269 |
| `power.regulators[].controls[].strap` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | power.rs:269 |
| `power.regulators[].controls[].strap` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:269 |
| `power.regulators[].controls[].wire` | - | - | - | untested | no committed specification declares a value for it | : |
| `power.loads[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:116 |
| `power.loads[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:116 |
| `power.loads[]` | sentinel-03 | empty | exit 0 | relaxes | regulators judged with no load: loss at 2000 mA -> 0 mA; required key, [] accepted; BOM identical: rem `solve:0.power.conditions[*]`: "U10: its junction is screened, not guaranteed: 1170 mW of loss at 2000 mA (declared: SNVSAN3F p. 27, Figure 9-7, typical at TA 25 C: 90 % on the 36 V curve at 2 A; P_loss = 5.237 V x 2 A x (1/0.90 - 1) = 1164 mW, rounded up) at \ / "U11: its junction is screened, not guaranteed: 550 mW of loss at 747 mA (declared: SNVSAN3F p. 27, Figure 9-8, typical at TA 25 C: 81 % on the 36 V curve at 0.68 A; P_loss = 3.417 V x 679 mA x (1/0.81 - 1) = 544 mW, rounded up) a | power.rs:116 |
| `power.loads[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:116 |
| `power.loads[].reference` | sentinel-03 | delete | exit 0 | no change |  | power.rs:321 |
| `power.loads[].reference` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:321 |
| `power.loads[].reference` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | power.rs:321 |
| `power.loads[].reference` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:321 |
| `power.sense[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:117 |
| `power.sense[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:117 |
| `power.sense[]` | sentinel-03 | empty | exit 0 | conservative | removes the VBAT_SENSE network (BOM loses R50, R51, C50) and its ADC judgement with it: gone `solve:0.adc_inputs.VBAT_SENSE.pin`: "PC0"<br>gone `solve:0.power.domains.VDD_3V3.consumers[VREF+ (VBAT_SENSE full scale)].floor_millivolts`: 2536 | power.rs:117 |
| `power.sense[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:117 |
| `power.sense[].reservoir` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:334 |
| `power.sense[].reservoir` | sentinel-03 | null | exit 2 | refused | validate exit 2 SenseSourceImpedance | power.rs:334 |
| `power.sense[].reservoir` | sentinel-03 | empty | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:334 |
| `power.sense[].reservoir` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:334 |
| `power.crossings[]` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:118 |
| `power.crossings[]` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:118 |
| `power.crossings[]` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SupplyMismatch | power.rs:118 |
| `power.crossings[]` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:118 |
| `power.crossings[].external[]` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:359 |
| `power.crossings[].external[]` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:359 |
| `power.crossings[].external[]` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 ExternalPartMissing | power.rs:359 |
| `power.crossings[].external[]` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:359 |
| `power.crossings[].controls[]` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:361 |
| `power.crossings[].controls[]` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:361 |
| `power.crossings[].controls[]` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 ControlPinUnwired | power.rs:361 |
| `power.crossings[].controls[]` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:361 |
| `power.crossings[].controls[].strap` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:269 |
| `power.crossings[].controls[].strap` | sentinel-02-power | null | exit 2 | refused | validate exit 2 PowerValueInvalid | power.rs:269 |
| `power.crossings[].controls[].strap` | sentinel-02-power | empty | - | n/a | scalar: {} / [] does not apply | power.rs:269 |
| `power.crossings[].controls[].strap` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:269 |
| `power.crossings[].controls[].wire` | - | - | - | untested | no committed specification declares a value for it | : |
| `power.drop_limits[]` | sentinel-03+gated | delete | exit 0 | relaxes | the supply gate is off: ran[1] judged 1 -> off power.drop_limits: gone `extract:0.ran[1].judged`: 1<br>gone `extract:0.supplies[VDD_3V3].domain`: "VDD_3V3"<br>gone `extract:0.supplies[VDD_3V3].drop_bound_microvolts`: 90032 | power.rs:128 |
| `power.drop_limits[]` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:128 |
| `power.drop_limits[]` | sentinel-03+gated | empty | exit 0 | relaxes | same as delete; [] is accepted and means none: gone `extract:0.ran[1].judged`: 1<br>gone `extract:0.supplies[VDD_3V3].domain`: "VDD_3V3"<br>gone `extract:0.supplies[VDD_3V3].drop_bound_microvolts`: 90032 | power.rs:128 |
| `power.drop_limits[]` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:128 |
| `accept_lifecycle[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:208 |
| `accept_lifecycle[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:208 |
| `accept_lifecycle[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 LifecycleNotAccepted | lib.rs:208 |
| `accept_lifecycle[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:208 |
| `serial_ports[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:210 |
| `serial_ports[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:210 |
| `serial_ports[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 ConnectorNetUnknown | lib.rs:210 |
| `serial_ports[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:210 |
| `serial_ports[].tx.pin` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `serial_ports[].tx.pin` | sentinel-03 | null | exit 0 | no change |  | lib.rs:464 |
| `serial_ports[].tx.pin` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `serial_ports[].tx.pin` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `serial_ports[].rx.pin` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `serial_ports[].rx.pin` | sentinel-03 | null | exit 0 | no change |  | lib.rs:464 |
| `serial_ports[].rx.pin` | sentinel-03 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `serial_ports[].rx.pin` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `acknowledge_unwitnessed[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | sentinel-03 | empty | exit 0 | no change |  | lib.rs:217 |
| `acknowledge_unwitnessed[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[].fields[]` | sentinel-03 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:259 |
| `acknowledge_unwitnessed[].fields[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:259 |
| `acknowledge_unwitnessed[].fields[]` | sentinel-03 | empty | exit 2 | refused | validate exit 2 UnwitnessedFieldsEmpty | lib.rs:259 |
| `acknowledge_unwitnessed[].fields[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:259 |
| `test_points[]` | sentinel-03 | delete | exit 0 | conservative | removes the pads; the probe-load condition lines go with them (named in the RUL-14 amendment): rem `extract:0.conditions[*]`: "`TP1` is a test pad on `I2C_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance" / "`TP2` is a test pad on `I2C_SCL`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance" | lib.rs:223 |
| `test_points[]` | sentinel-03 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:223 |
| `test_points[]` | sentinel-03 | empty | exit 0 | conservative | same as delete: rem `extract:0.conditions[*]`: "`TP1` is a test pad on `I2C_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance" / "`TP2` is a test pad on `I2C_SCL`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance" | lib.rs:223 |
| `test_points[]` | sentinel-03 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:223 |
| `pin_devices[]` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 ConnectorNetUnknown | lib.rs:231 |
| `pin_devices[]` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:231 |
| `pin_devices[]` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 ConnectorNetUnknown | lib.rs:231 |
| `pin_devices[]` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:231 |
| `pin_devices[].supplies{}` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:87 |
| `pin_devices[].supplies{}` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:87 |
| `pin_devices[].supplies{}` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 SupplyPinUndeclared | pin_devices.rs:87 |
| `pin_devices[].supplies{}` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:87 |
| `pin_devices[].straps{}` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:90 |
| `pin_devices[].straps{}` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:90 |
| `pin_devices[].straps{}` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 StrapUndeclared | pin_devices.rs:90 |
| `pin_devices[].straps{}` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:90 |
| `pin_devices[].signals[]` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:91 |
| `pin_devices[].signals[]` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:91 |
| `pin_devices[].signals[]` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 DevicePinUnbound | pin_devices.rs:91 |
| `pin_devices[].signals[]` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:91 |
| `pin_devices[].signals[].pin` | - | - | - | untested | no committed specification declares a value for it | : |
| `pin_devices[].signals[].initial` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:147 |
| `pin_devices[].signals[].initial` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:147 |
| `pin_devices[].signals[].initial` | d1-drv8876 | empty | - | n/a | scalar: {} / [] does not apply | pin_devices.rs:147 |
| `pin_devices[].signals[].initial` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:147 |
| `pin_devices[].signals[].active` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:149 |
| `pin_devices[].signals[].active` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:149 |
| `pin_devices[].signals[].active` | d1-drv8876 | empty | - | n/a | scalar: {} / [] does not apply | pin_devices.rs:149 |
| `pin_devices[].signals[].active` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:149 |
| `pin_devices[].signals[].pull` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:151 |
| `pin_devices[].signals[].pull` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:151 |
| `pin_devices[].signals[].pull` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:151 |
| `pin_devices[].signals[].pull` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:151 |
| `pin_devices[].signals[].pull.mcu` | d2-drv8833 | delete | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:160 |
| `pin_devices[].signals[].pull.mcu` | d2-drv8833 | null | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:160 |
| `pin_devices[].signals[].pull.mcu` | d2-drv8833 | empty | - | n/a | scalar: {} / [] does not apply | pin_devices.rs:160 |
| `pin_devices[].signals[].pull.mcu` | d2-drv8833 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:160 |
| `pin_devices[].signals[].pull.resistor` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:162 |
| `pin_devices[].signals[].pull.resistor` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SignalShape | pin_devices.rs:162 |
| `pin_devices[].signals[].pull.resistor` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:162 |
| `pin_devices[].signals[].pull.resistor` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:162 |
| `pin_devices[].analog[]` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:92 |
| `pin_devices[].analog[]` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:92 |
| `pin_devices[].analog[]` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 TripSenseUnbound | pin_devices.rs:92 |
| `pin_devices[].analog[]` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:92 |
| `pin_devices[].analog[].pin` | - | - | - | untested | no committed specification declares a value for it | : |
| `pin_devices[].analog[].channel` | - | - | - | untested | no committed specification declares a value for it | : |
| `pin_devices[].bridges[]` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:93 |
| `pin_devices[].bridges[]` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:93 |
| `pin_devices[].bridges[]` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 ConnectorNetUnknown | pin_devices.rs:93 |
| `pin_devices[].bridges[]` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:93 |
| `pin_devices[].bridges[].inputs{}` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:232 |
| `pin_devices[].bridges[].inputs{}` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:232 |
| `pin_devices[].bridges[].inputs{}` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 BridgeChannelUnknown | pin_devices.rs:232 |
| `pin_devices[].bridges[].inputs{}` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:232 |
| `pin_devices[].bridges[].outputs{}` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:234 |
| `pin_devices[].bridges[].outputs{}` | d1-drv8876 | null | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:234 |
| `pin_devices[].bridges[].outputs{}` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 ConnectorNetUnknown | pin_devices.rs:234 |
| `pin_devices[].bridges[].outputs{}` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:234 |
| `pin_devices[].bulk` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:102 |
| `pin_devices[].bulk` | d1-drv8876 | null | exit 2 | refused | validate exit 2 BulkUndeclared | pin_devices.rs:102 |
| `pin_devices[].bulk` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:102 |
| `pin_devices[].bulk` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:102 |
| `pin_devices[].temperature_celsius` | d1-drv8876 | delete | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:107 |
| `pin_devices[].temperature_celsius` | d1-drv8876 | null | exit 2 | refused | validate exit 2 TemperatureUndeclared | pin_devices.rs:107 |
| `pin_devices[].temperature_celsius` | d1-drv8876 | empty | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:107 |
| `pin_devices[].temperature_celsius` | d1-drv8876 | rename | exit 2 | refused | validate exit 2 SchemaViolation | pin_devices.rs:107 |
| `spi_buses[].sck.pin` | i2c-spi-bench | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | i2c-spi-bench | null | exit 2 | refused | validate exit 2 InvalidIdentifier | lib.rs:464 |
| `spi_buses[].sck.pin` | i2c-spi-bench | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].sck.pin` | i2c-spi-bench | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | pair-standin | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | pair-standin | null | exit 0 [exit emit=2] | refused-later | emit Unroutable: new `emit:0.error.message`: "the router cannot complete net `/SPI_SCK`: pair `USB_STANDIN` cannot be coupled: no run on F.Cu holds both conductors 300000 nm apart" | lib.rs:464 |
| `spi_buses[].sck.pin` | pair-standin | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].sck.pin` | pair-standin | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-02-own | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-02-own | null | exit 0 [exit extract=1] | no change | solver freedom: a pinned pin becomes the solver's choice; may move the board (on pair-standin emit then says Unroutable, on sentinel-02-own a line fails); no judgement removed | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-02-own | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:464 |
| `spi_buses[].sck.pin` | sentinel-02-own | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:464 |
| `connectors[].standard` | connectors | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | connectors | null | exit 0 | no change | no output differs on the host commands | connectors.rs:209 |
| `connectors[].standard` | connectors | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:209 |
| `connectors[].standard` | connectors | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | four-port | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | four-port | null | exit 0 [exit extract=1] | no change | no output differs on the host commands | connectors.rs:209 |
| `connectors[].standard` | four-port | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:209 |
| `connectors[].standard` | four-port | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | hse-crystal-16 | delete | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `connectors[].standard` | hse-crystal-16 | null | exit 0 [exit extract=1] | no change | no output differs on the host commands | connectors.rs:209 |
| `connectors[].standard` | hse-crystal-16 | empty | - | n/a | scalar: {} / [] does not apply | connectors.rs:209 |
| `connectors[].standard` | hse-crystal-16 | rename | exit 2 | refused | validate exit 2 SchemaViolation | connectors.rs:209 |
| `net_constraints[].impedance_ohms` | syn:s03+impedance_ohms=50 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:563 |
| `net_constraints[].impedance_ohms` | syn:s03+impedance_ohms=50 | null | exit 0 | no change | declared 50 vs null: no output of validate, solve, emit, extract or check speeds differs; no host gate reads it | lib.rs:563 |
| `net_constraints[].impedance_ohms` | syn:s03+impedance_ohms=50 | empty | - | n/a | scalar: {} / [] does not apply | lib.rs:563 |
| `net_constraints[].impedance_ohms` | syn:s03+impedance_ohms=50 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:563 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:s03+dielectric max_thickness_um=min+40 | delete | exit 0 | no change | doc (board.rs:249-253): absent prices the via barrel at the finished maximum less the other layers' minima, i.e. a longer barrel, stricter; supplies[VDD_3V3].drop_bound_microvolts identical on the gated carrier | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:s03+dielectric max_thickness_um=min+40 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:s03+dielectric max_thickness_um=min+40 | empty | - | n/a | scalar: {} / [] does not apply | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:s03+dielectric max_thickness_um=min+40 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:255 |
| `board.stackup.fab_tolerances` | sentinel-02-power | delete | exit 0 [exit extract=2] | relaxes | same; emit loses the ladder, its unjudged TranslatorPinUnbounded entry and the via-ring notes (extract refuses TranslatorPinUnbounded on this baseline, so extract is not a witness here): gone `emit:0.notes[*]`: "via ring: U10 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill" / "via ring: U11 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill"<br>gone `emit:0.capacitance_ladder.judged_by`: "extract, run separately on the emitted board (RUL-20)" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-02-power | empty | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03+gated | delete | exit 0 [exit extract=2] | relaxes | emit relaxes as on sentinel-03; with drop_limits declared the supply gate then refuses (extract SupplyInputUndeclared), so only the emit-side relaxation survives: gone `emit:0.notes[*]`: "via ring: U10 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill" / "via ring: U11 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill"<br>new `extract:0.error.message`: "the supply gate reads `board.stackup.fab_tolerances.hole_position_um`, and the specification does not declare it: it is never defaulted (RUL-14)"<br>gone `emit:0.capacitance_ladder.judged_by`: "extract, run separately on the emitted board (RUL-20)" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03+gated | empty | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:184 |
| `board.stackup.fab_tolerances` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:184 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-02-power | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_position_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03+gated | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_position_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_position_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:196 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-02-power | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03+gated | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_plus_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:203 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-02-power | delete | exit 0 [exit extract=2] | no change | read only by the supply gate (supply.rs:1384): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract) | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.fab_tolerances.hole_size_minus_um`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.hole_size_minus_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:210 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-02-power | delete | exit 0 [exit extract=2] | no change | read only by the supply gate (supply.rs:1386): no change unless power.drop_limits is declared; with it, see the gated rows (refused at extract) | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03+gated | delete | exit 0 [exit extract=2] | refused-later | extract SupplyInputUndeclared: new `extract:0.error.message`: "the supply gate reads `board.stackup.fab_tolerances.track_width_permille`, and the specification does not declare it: it is never defaulted (RUL-14)" | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.track_width_permille` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:219 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-02-power | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.via_ring_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03+gated | delete | exit 0 [exit emit=2] | refused-later | emit FabToleranceUndeclared: new `emit:0.error.message`: "`board.stackup.fab_tolerances.via_ring_um` is undeclared, and the via ring rule needs it wherever `board.stackup.fab_tolerances` is declared; it is never defaulted" | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03+gated | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03+gated | empty | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `board.stackup.fab_tolerances.via_ring_um` | sentinel-03+gated | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:227 |
| `net_classes[].via` | four-layer-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].via` | four-layer-power | null | exit 0 | no change | geometry only | classes.rs:60 |
| `net_classes[].via` | four-layer-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].via` | four-layer-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].via` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `net_classes[].via` | sentinel-02-power | null | exit 0 [exit extract=2] | no change | geometry only | classes.rs:60 |
| `net_classes[].via` | sentinel-02-power | empty | - | n/a | scalar: {} / [] does not apply | classes.rs:60 |
| `net_classes[].via` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | classes.rs:60 |
| `motor_outputs[].pin` | hse-crystal-16 | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | hse-crystal-16 | null | exit 0 [exit extract=1] | no change | solver freedom; pins move, no judgement removed | motor.rs:170 |
| `motor_outputs[].pin` | hse-crystal-16 | empty | - | n/a | scalar: {} / [] does not apply | motor.rs:170 |
| `motor_outputs[].pin` | hse-crystal-16 | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | hse-external-8 | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | hse-external-8 | null | exit 0 [exit emit=2] | no change | solver freedom; pins move, no judgement removed | motor.rs:170 |
| `motor_outputs[].pin` | hse-external-8 | empty | - | n/a | scalar: {} / [] does not apply | motor.rs:170 |
| `motor_outputs[].pin` | hse-external-8 | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | d2-drv8833 | delete | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `motor_outputs[].pin` | d2-drv8833 | null | exit 2 | refused | validate exit 2 PartLimitUndeclared | motor.rs:170 |
| `motor_outputs[].pin` | d2-drv8833 | empty | - | n/a | scalar: {} / [] does not apply | motor.rs:170 |
| `motor_outputs[].pin` | d2-drv8833 | rename | exit 2 | refused | validate exit 2 SchemaViolation | motor.rs:170 |
| `power.battery.series_milliohms` | sentinel-02-power | delete | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-02-power | null | exit 0 [exit extract=2] | no change | no output differs on the host commands | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-02-power | empty | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.battery.series_milliohms` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:214 |
| `power.loads[].reference` | sentinel-02-power | delete | exit 0 [exit extract=2] | no change |  | power.rs:321 |
| `power.loads[].reference` | sentinel-02-power | null | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:321 |
| `power.loads[].reference` | sentinel-02-power | empty | - | n/a | scalar: {} / [] does not apply | power.rs:321 |
| `power.loads[].reference` | sentinel-02-power | rename | exit 2 | refused | validate exit 2 SchemaViolation | power.rs:321 |
| `serial_ports[].peripheral` | syn:s03+serial peripheral,negotiated | delete | exit 2 | refused | validate exit 2 SchemaViolation | serial.rs:135 |
| `serial_ports[].peripheral` | syn:s03+serial peripheral,negotiated | null | exit 0 | no change | solver freedom; no change | serial.rs:135 |
| `serial_ports[].peripheral` | syn:s03+serial peripheral,negotiated | empty | - | n/a | scalar: {} / [] does not apply | serial.rs:135 |
| `serial_ports[].peripheral` | syn:s03+serial peripheral,negotiated | rename | exit 2 | refused | validate exit 2 SchemaViolation | serial.rs:135 |
| `serial_ports[].negotiated` | syn:s03+serial peripheral,negotiated | delete | exit 2 | refused | validate exit 2 SchemaViolation | serial.rs:141 |
| `serial_ports[].negotiated` | syn:s03+serial peripheral,negotiated | null | exit 2 | refused | validate exit 2 SerialFramingContradictsRole | serial.rs:141 |
| `serial_ports[].negotiated` | syn:s03+serial peripheral,negotiated | empty | - | n/a | scalar: {} / [] does not apply | serial.rs:141 |
| `serial_ports[].negotiated` | syn:s03+serial peripheral,negotiated | rename | exit 2 | refused | validate exit 2 SchemaViolation | serial.rs:141 |
| `acknowledge_unwitnessed[]` | driver-judges-nothing | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | driver-judges-nothing | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | driver-judges-nothing | empty | exit 0 | no change |  | lib.rs:217 |
| `acknowledge_unwitnessed[]` | driver-judges-nothing | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | four-layer | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | four-layer | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | four-layer | empty | exit 0 | no change |  | lib.rs:217 |
| `acknowledge_unwitnessed[]` | four-layer | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | logger-4600 | delete | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | logger-4600 | null | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `acknowledge_unwitnessed[]` | logger-4600 | empty | exit 0 | no change |  | lib.rs:217 |
| `acknowledge_unwitnessed[]` | logger-4600 | rename | exit 2 | refused | validate exit 2 SchemaViolation | lib.rs:217 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:gated+dielectric max_thickness_um=min+40 | delete | exit 0 | no change | doc (board.rs:249-253): absent prices the via barrel at the finished maximum less the other layers' minima, i.e. a longer barrel, stricter; supplies[VDD_3V3].drop_bound_microvolts identical on the gated carrier | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:gated+dielectric max_thickness_um=min+40 | null | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:gated+dielectric max_thickness_um=min+40 | empty | - | n/a | scalar: {} / [] does not apply | board.rs:255 |
| `board.stackup.dielectrics[].max_thickness_um` | syn:gated+dielectric max_thickness_um=min+40 | rename | exit 2 | refused | validate exit 2 SchemaViolation | board.rs:255 |
