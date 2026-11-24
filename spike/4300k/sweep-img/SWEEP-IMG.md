# AET-WO-4300k key-deletion sweep, container gates (SWEEP-IMG.md)

Snapshot 0e924022 (/home/forge/worktrees/aethelgard/4300k-snap), image per container/IMAGE, kicad-cli 10.0.7, via kit/img.sh (8G, swap 0). Variants: every (carrier, key, variant) of the host sweep (results*.json) with carrier `sentinel-03` or `sentinel-03+gated` and validate exit 0, rebuilt with sweep.py's own `gated()` and `apply()` (exec'd from the file): 41 variants + 2 baselines = 43 runs. Per variant: validate, emit, `check erc --schematic`, `check drc --board --spec --schematic`, `check support --schematic --board --spec`. Scripts: gen.py, run.sh, cmp.py; raw outputs out/<id>/, comparison cmp.json.

Comparison: each gate's JSON flattened to (path, value) facts, array elements identified by role/part/pin/net/ic/name; sha256, paths and `*_um` distances dropped (placement); every other number kept (so `judged` counts and list lengths are compared). A variant "judges less" if a fact is gone or a count drops vs its own carrier's baseline. Exit status was not used.

## Baseline (sentinel-03; the gated carrier is identical in all three gates)
- erc: exit 0, verdict pass, `findings: []`; no `ran` array.
- drc: exit 0, verdict pass, findings 0; `"ran":[{"gate":"drc.pairs","off":"diff_pairs"},{"gate":"drc.power","judged":16}]`; power: loops 16, classes 2, conditions 7.
- support: exit 0, verdict pass; roles 19, straps 4, programming {connector J2, nrst true}, conditions 3, supply_source null.

## Answer
One key relaxes a container gate while validate and emit succeed on the same board:

| key | variant | gate | what changed |
|---|---|---|---|
| `net_classes[]` | `[]` (v015) | drc | `drc:0.power.classes#len: 2 -> 0`; gone: `POWER` (nets VBAT, VDD_5V, /U10_SW) and `SUPPLY_MCU` (nets VDD_3V3, /U11_SW). `ran[1].judged` stays 16, the 16 loops are unchanged. erc and support: no change. The emitted board is smaller (37194 -> 36482 lines) and the schematic's reference list is identical. |

Not relaxations:
- `power.sense[]` `[]` (v021): drc `loops 16 -> 15` and `ran[1].judged: 16 -> 15` (loop `C50,U1`, "AN2834 p. 45, 4.4.3: a large external capacitor at the ADC input pin" gone). The emitted schematic also loses C50, R50, R51, #PWR166-168: the sense divider and its reservoir leave the BOM with their judgement (conservative, as in SWEEP.md). erc and support: no change.
- `board.stackup.fab_tolerances` `{}` and `.hole_position_um` / `.hole_size_plus_um` / `.via_ring_um` delete, on both carriers (v003, v004, v006, v008, v030, v031, v033, v035): validate 0, emit exit 2 `FabToleranceUndeclared` (`board.stackup.fab_tolerances.hole_size_plus_um` is undeclared ... it is never defaulted); no board, so erc/drc/support NOT RUN. Refused by name, as SWEEP.md found.

All other 29 variants (including `board.stackup.fab_tolerances` delete, `power.drop_limits[]` delete/empty, `power.loads[]`, `power.conditions.dc_bias_derating[]`, `diff_pairs`-adjacent keys, `test_points[]`, `mcu.reset_switch`, `serial_ports[].*.pin`, `spi_*.pin`, the gated stackup minima): erc, drc and support facts identical to baseline apart from the dropped placement numbers; nothing new either.

List of keys that relax support/erc/drc: `net_classes[]` (empty list; drc, power classes 2 -> 0). Support and erc: none.

Blind spot to know: the drc rule files the gate writes are not in its JSON, so a relaxation that changes only the written rules (not what is reported) would not show here; v015's reported `classes` 2 -> 0 is the visible symptom. Not checked: whether the written .kicad_dru differs for v015.

## Per variant (id, carrier, key, variant: emit rc / erc,drc,support rc / gates that judged less)
- base-sentinel-03 sentinel-03 None None: emit 0 / 0,0,0 / none
- base-gated sentinel-03+gated None None: emit 0 / 0,0,0 / none
- v000 sentinel-03 acknowledge_unwitnessed[] empty: emit 0 / 0,0,0 / none
- v001 sentinel-03 board.max_outline_um delete: emit 0 / 0,0,0 / none
- v002 sentinel-03 board.stackup.fab_tolerances delete: emit 0 / 0,0,0 / none
- v003 sentinel-03 board.stackup.fab_tolerances empty: emit 2 / -,-,- / NOT RUN
- v004 sentinel-03 board.stackup.fab_tolerances.hole_position_um delete: emit 2 / -,-,- / NOT RUN
- v005 sentinel-03 board.stackup.fab_tolerances.hole_size_minus_um delete: emit 0 / 0,0,0 / none
- v006 sentinel-03 board.stackup.fab_tolerances.hole_size_plus_um delete: emit 2 / -,-,- / NOT RUN
- v007 sentinel-03 board.stackup.fab_tolerances.track_width_permille delete: emit 0 / 0,0,0 / none
- v008 sentinel-03 board.stackup.fab_tolerances.via_ring_um delete: emit 2 / -,-,- / NOT RUN
- v009 sentinel-03 connectors[].standard null: emit 0 / 0,0,0 / none
- v010 sentinel-03 i2c_devices[].pin_capacitance null: emit 0 / 0,0,0 / none
- v011 sentinel-03 mcu.reserved_pins[] empty: emit 0 / 0,0,0 / none
- v012 sentinel-03 mcu.reset_switch delete: emit 0 / 0,0,0 / none
- v013 sentinel-03 motor_outputs[].esc null: emit 0 / 0,0,0 / none
- v014 sentinel-03 motor_outputs[].pin null: emit 0 / 0,0,0 / none
- v015 sentinel-03 net_classes[] empty: emit 0 / 0,0,0 / less: drc
- v016 sentinel-03 net_classes[].via null: emit 0 / 0,0,0 / none
- v017 sentinel-03 power.battery.series_milliohms null: emit 0 / 0,0,0 / none
- v018 sentinel-03 power.conditions.dc_bias_derating[] empty: emit 0 / 0,0,0 / none
- v019 sentinel-03 power.loads[] empty: emit 0 / 0,0,0 / none
- v020 sentinel-03 power.loads[].reference delete: emit 0 / 0,0,0 / none
- v021 sentinel-03 power.sense[] empty: emit 0 / 0,0,0 / less: drc
- v022 sentinel-03 serial_ports[].rx.pin null: emit 0 / 0,0,0 / none
- v023 sentinel-03 serial_ports[].tx.pin null: emit 0 / 0,0,0 / none
- v024 sentinel-03 spi_buses[].sck.pin null: emit 0 / 0,0,0 / none
- v025 sentinel-03 spi_devices[].cs.pin null: emit 0 / 0,0,0 / none
- v026 sentinel-03 test_points[] delete: emit 0 / 0,0,0 / none
- v027 sentinel-03 test_points[] empty: emit 0 / 0,0,0 / none
- v028 sentinel-03+gated board.stackup.copper_temperature_max_c delete: emit 0 / 0,0,0 / none
- v029 sentinel-03+gated board.stackup.fab_tolerances delete: emit 0 / 0,0,0 / none
- v030 sentinel-03+gated board.stackup.fab_tolerances empty: emit 2 / -,-,- / NOT RUN
- v031 sentinel-03+gated board.stackup.fab_tolerances.hole_position_um delete: emit 2 / -,-,- / NOT RUN
- v032 sentinel-03+gated board.stackup.fab_tolerances.hole_size_minus_um delete: emit 0 / 0,0,0 / none
- v033 sentinel-03+gated board.stackup.fab_tolerances.hole_size_plus_um delete: emit 2 / -,-,- / NOT RUN
- v034 sentinel-03+gated board.stackup.fab_tolerances.track_width_permille delete: emit 0 / 0,0,0 / none
- v035 sentinel-03+gated board.stackup.fab_tolerances.via_ring_um delete: emit 2 / -,-,- / NOT RUN
- v036 sentinel-03+gated board.stackup.outer_copper_min_um delete: emit 0 / 0,0,0 / none
- v037 sentinel-03+gated board.stackup.plane_copper_min_um delete: emit 0 / 0,0,0 / none
- v038 sentinel-03+gated board.stackup.plating_min_um delete: emit 0 / 0,0,0 / none
- v039 sentinel-03+gated power.drop_limits[] delete: emit 0 / 0,0,0 / none
- v040 sentinel-03+gated power.drop_limits[] empty: emit 0 / 0,0,0 / none
