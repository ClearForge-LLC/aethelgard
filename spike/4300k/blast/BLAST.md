# AET-WO-4300k blast radius (measured at 521dcd06, debug binary)

Scratch outputs: `/home/forge/.cache/aet-4300k/blast/<name>/{validate,solve,emit,extract}.{out,err,rc}`. Scripts: `part1.sh`, `part2.py`, `dropmeasure.py`, `mkmd.py` in this directory.

## Part 1: host baseline, 36 committed specifications

rc = exit codes validate/solve/emit/extract. Conditions = `conditions` array count printed by extract (identical across boards except where noted); extract `ran` rows: lines=judged count, supply/oscillator/pairs = `off` key when not declared.

| spec | rc v/s/e/x | refusal kind | extract verdict | conditions | lines judged | findings | emit notes | lifecycle msgs | gates off |
|---|---|---|---|---|---|---|---|---|---|
| connectors | 0/0/0/0 | - | pass | 8 | 6 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| d1-24-i2c | 0/0/0/0 | - | pass | 8 | 10 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| d1-drv8876 | 0/0/2/2 | emit: DrawsNoBoard drv8876-ph-en | - | 0 |  | 0 | 0 | 0 |  |
| d2-drv8833 | 2/2/2/2 | validate: PartLimitUndeclared U2 | - | 0 |  | 0 | 0 | 0 |  |
| d3-24-i2c-12-spi | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 8 | 16 | 16 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| driver-judges-nothing | 0/0/0/0 | - | pass | 6 | 0 | 0 | 0 | 0 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| four-layer | 0/0/0/0 | - | pass | 8 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| four-layer-power | 0/0/0/0 | - | pass | 8 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| four-port | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 8 | 4 | 2 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| hse-crystal-16 | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 10 | 6 | 2 | 0 | 1 | power.drop_limits,diff_pairs |
| hse-external-8 | 0/0/2/2 | emit: DrawsNoBoard hse-external-8 | - | 0 |  | 0 | 0 | 1 |  |
| i2c-spi-bench | 2/2/2/2 | validate: InvalidIdentifier | - | 0 |  | 0 | 0 | 0 |  |
| imu-baro-1v8 | 0/0/2/2 | emit: DrawsNoBoard imu-baro-1v8 | - | 0 |  | 0 | 0 | 2 |  |
| invalid-zero-domain | 2/2/2/2 | validate: ZeroNominalVoltage VDD_3V3 | - | 0 |  | 0 | 0 | 0 |  |
| logger-4600 | 0/0/0/0 | - | pass | 7 | 6 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| motors-dshot600 | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| motors-pwm | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| pair-standin | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal |
| sentinel-01 | 0/0/0/0 | - | pass | 14 | 6 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-01-bench | 0/0/0/0 | - | pass | 11 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-01-ext-bmp581 | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 15 | 6 | 1 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-01-minimal | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-02-bench | 0/0/0/0 | - | pass | 9 | 4 | 0 | 0 | 0 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-02-bench-hse8 | 0/0/2/2 | emit: DrawsNoBoard sentinel-02-bench-hse8 | - | 0 |  | 0 | 0 | 0 |  |
| sentinel-02-own | 0/0/0/0 | - | pass | 9 | 6 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-02-power | 0/0/0/2 | extract: TranslatorPinUnbounded | - | 0 |  | 0 | 2 | 2 |  |
| sentinel-02-shaped | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 9 | 6 | 1 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| sentinel-03 | 0/0/0/0 | - | pass | 15 | 8 | 0 | 2 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| serial-dshot-9600 | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| serial-four | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 8 | 4 | 2 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| solve-infeasible-shared | 0/2/2/2 | solve: Infeasible | - | 0 |  | 0 | 0 | 0 |  |
| solve-pigeonhole-spi | 0/2/2/2 | solve: Infeasible | - | 0 |  | 0 | 0 | 1 |  |
| solve-stress | 0/0/0/1 | extract: verdict fail (exit 1) | fail | 7 | 16 | 8 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| solve-thrash-pins | 0/2/2/2 | solve: Infeasible | - | 0 |  | 0 | 0 | 1 |  |
| spi-only | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |
| two-bme | 0/0/0/0 | - | pass | 7 | 4 | 0 | 0 | 1 | power.drop_limits,mcu.hse.crystal,diff_pairs |

Condition templates printed by extract (count of boards): see the sentinel-03 list below; the others are subsets plus per-board `TPn` test-pad lines, the BMP581/ICM42688P declared-capacitance lines, and the oscillator line (hse-crystal-16 only).

### sentinel-03 extract `conditions` (verbatim, 15)

- scope: the bare board's copper and each I2C pin's Ci; part bodies, solder and terminations above the pads are not included
- laminate and solder mask relative permittivity at most 5.4 (the IPC-4101 FR-4 maximum, specified at 1 MHz)
- finished copper at most 70 um thick; finished board from 1.25 mm to 1.851 mm thick
- each dielectric at least its declared minimum: F.Cu to In1.Cu 0.179 mm, In1.Cu to In2.Cu 0.891 mm, In2.Cu to B.Cu 0.179 mm
- zone fill keeps the board's clearance (the DRC gate refills and judges it)
- every I2C pin not counted at a declared value at most UM10204's Ci of 10 pF; no multiplexer or switch on the line
- neighbouring nets are static while the line rises
- on SCK and MOSI, each device pin not counted at a declared value at its part's cited maximum pin capacitance (parts table); the MCU's own pin drives the line and is not counted (DS12117 Table 149's C is the external load)
- beyond the board, each connector pin on a line adds the capacitance its specification declares for the mated contact, the cable and the remote pins, as declared with its source (the line's `offboard`); nothing here measures it
- U7 (BMP581): each of its pins on a bounded line counted at 10000 fF, a value the specification declares, not a maximum the datasheet prints; the bound holds only under its condition: each of U7's pins on SDA and SCL at most UM10204's Ci of 10 pF, the I2C specification its interface follows; Bosch prints no pin capacitance, so the bound holds only if the part meets that limit (source: Bosch BST-BMP581-DS004-13 v1.13: no pin capacitance in Tables 4, 5, 6, 17 (pp. 10-13, 31), I2C per NXP UM10204 (p. 33); UM10204 rev. 7.0 Table 10, p. 43: Ci 10 pF maximum)
- U5 (ICM42688P): each of its pins on a bounded line counted at 10000 fF, a value the specification declares, not a maximum the datasheet prints; the bound holds only under its condition: each of U5's pins on SCK and MOSI at most TDK's typical C_I of < 10 pF; the datasheet prints no maximum, so the bound holds only if the part's pins do not exceed that typical (source: TDK InvenSense DS-000347 v1.9, 3.3.2 Table 4, p. 14: C_I < 10 pF for FSYNC, SCLK, SDI and CS in the TYP column, MIN and MAX empty (AET-WO-4220 D10))
- `TP1` is a test pad on `I2C_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance
- `TP2` is a test pad on `I2C_SCL`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance
- `TP3` is a test pad on `EXT_SDA`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance
- `TP4` is a test pad on `EXT_SCL`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance

extract `ran`: `[{"gate": "extract.lines", "judged": 8}, {"gate": "extract.supply", "off": "power.drop_limits"}, {"gate": "extract.oscillator", "off": "mcu.hse.crystal"}, {"gate": "extract.pairs", "off": "diff_pairs"}]`; `findings`: `[]`; verdict `pass`.

emit `notes` (verbatim):

- via ring: U10 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill
- via ring: U11 pad 9's thermal vias raised from 0.554 to 0.7 mm, the pad that keeps the declared ring round their 0.254 mm drill

emit `capacitance_ladder`: `{"bounded_first": false, "bounded_via_cost": 96, "judged_by": "extract, run separately on the emitted board (RUL-20)", "rung": 2, "smallest_margin": {"c_bound_femtofarads": 27152, "declared_femtofarads": 30000, "net": "IMU_MOSI"}}`

validate/solve/emit/extract each print the same one `lifecycle` message (W25Q64JVxIQ NRND, acknowledged in accept_lifecycle); validate prints no "screened"/"not computed" text. Full outputs: `sentinel-03/*.out`.

### `generate hwdef` on sentinel-03

`aethelgard solve --file sentinel-03.json > topology.json` (rc 0), then `aethelgard generate hwdef --topology topology.json --out hwdef.dat`: rc 2, stdout `{"ok":false,"error":{"kind":"ImuOrientationUndeclared","id":"U5","message":"`U5` (ICM42688P) is an IMU with no declared orientation; ArduPilot's IMU line needs its rotation (AET-WO-4400d)"}}`. No hwdef.dat written.

## Part 2: blast radius per rule

Only two of 36 specifications declare `power`: sentinel-03 and sentinel-02-power. Neither declares `power.drop_limits`. No specification declares any supply stackup minimum (outer_copper_min_um, plane_copper_min_um, plating_min_um, copper_temperature_max_c).

| spec | A power / drop_limits | B fab_tolerances; fabricator(s) in stackup sources | C battery doubled vs VIN abs max | D bor_level / MCU-rail devices vs falling min | E power.input | F loads per regulated rail |
|---|---|---|---|---|---|---|
| i2c-spi-bench | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U7 BMP581 1710, U5 ICM42688P 1710; REFUSED: U7(1710>1580),U5(1710>1580) | - | - |
| connectors | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| d1-24-i2c | no power | no fab_tolerances; OSH Park | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices S0 BME280 1710, S1 BME280 1710, S2 TMP117 1800, S3 TMP117 1800, S4 TMP117 1800, S5 TMP117 1800, S6 BME280 1710, S7 BME280 1710, S8 TMP117 1 ... REFUSED: S0(1710>1580),S1(1710>1580),S2(1800>1580),S3(1800>1580),S4(1800>1580),S5(1800>1580),S6(1710>1580),S7(1710>1580),S8(1800>1580),S9(1800>1580),S10(1800>1580),S11(1800>1580),S12(1710>1580),S13(17... | - | - |
| d3-24-i2c-12-spi | no power | no fab_tolerances; OSH Park | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices S0 BME280 1710, S1 BME280 1710, S2 TMP117 1800, S3 TMP117 1800, S4 TMP117 1800, S5 TMP117 1800, S6 BME280 1710, S7 BME280 1710, S8 TMP117 1 ... REFUSED: S0(1710>1580),S1(1710>1580),S2(1800>1580),S3(1800>1580),S4(1800>1580),S5(1800>1580),S6(1710>1580),S7(1710>1580),S8(1800>1580),S9(1800>1580),S10(1800>1580),S11(1800>1580),S12(1710>1580),S13(17... | - | - |
| driver-judges-nothing | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices none; ok | - | - |
| four-layer-power | no power | no fab_tolerances; OSH Park | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| four-layer | no power | no fab_tolerances; OSH Park | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| four-port | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| hse-crystal-16 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U7 BMP581 1710, U4 W25Q64JVxIQ 2700, U5 ICM42688P 1710; REFUSED: U4(2700>2540) | - | - |
| hse-external-8 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U7 BMP581 1710, U4 W25Q64JVxIQ 2700, U5 ICM42688P 1710; REFUSED: U4(2700>2540) | - | - |
| imu-baro-1v8 | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_1V8; devices U2 BME280 1710, U6 ICP10111 1710, U5 ICM20948 1710, U7 ICM20948 1710; REFUSED: U2(1710>1580),U6(1710>1580),U5(1710>1580),U7(1710>1580) | - | - |
| logger-4600 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U10 TMP117 1800, U11 TMP117 1800, U12 TMP117 1800, U13 TMP117 1800, U20 BMP581 1710, U30 W25Q64JVxIQ 2700; REFUSED: U30(2700>2540) | - | - |
| motors-dshot600 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| motors-pwm | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| pair-standin | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| sentinel-01-bench | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| sentinel-02-own | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U3 TMP117 1800, U7 BMP581 1710, U4 W25Q64JVxIQ 2700, U5 ICM42688P 1710; REFUSED: U4(2700>2540) | - | - |
| sentinel-02-shaped | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U7 BMP581 1710, U4 W25Q64JVxIQ 2700, U5 ICM42688P 1710; REFUSED: U4(2700>2540) | - | - |
| sentinel-03 | power / NO drop_limits | fab_tolerances; JLCPCB/OSH Park (MULTI) | pack max 6 x 4350 = 26100 mV, doubled 52200 mV; VIN abs max U10 LMR33630ADDA 38000, U11 LMR33630ADDA 38000; REFUSED: U10,U11 | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U3 TMP117 1800, U7 BMP581 1710, U4 W25Q64JVxIQ 2700, U5 ICM42688P 1710; REFUSED: U4(2700>2540) | power.input J4 XT30PW-M hot_plug 1 | VDD_5V: 1 loads (2000 mA), VDD_3V3: 13 loads (89 mA) |
| serial-dshot-9600 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| serial-four | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| solve-infeasible-shared | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U100 TMP117 1800, U101 TMP117 1800, U102 TMP117 1800, U103 TMP117 1800; REFUSED: U100(1800>1580),U101(1800>1580),U102(1800>1580),U103(1800>1580) | - | - |
| solve-pigeonhole-spi | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U200 W25Q64JVxIQ 2700, U201 W25Q64JVxIQ 2700, U202 W25Q64JVxIQ 2700, U203 W25Q64JVxIQ 2700, U204 W25Q64JVxIQ 2700, U205 W25Q64JVxIQ 2700, U ... REFUSED: U200(2700>1580),U201(2700>1580),U202(2700>1580),U203(2700>1580),U204(2700>1580),U205(2700>1580),U206(2700>1580)... | - | - |
| solve-stress | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U100 TMP117 1800, U101 TMP117 1800, U102 TMP117 1800, U103 TMP117 1800, U200 W25Q64JVxIQ 2700, U201 W25Q64JVxIQ 2700, U202 W25Q64JVxIQ 2700 ... REFUSED: U100(1800>1580),U101(1800>1580),U102(1800>1580),U103(1800>1580),U200(2700>1580),U201(2700>1580),U202(2700>1580),U203(2700>1580),U204(2700>1580),U205(2700>1580),U206(2700>1580),U207(2700>1580)... | - | - |
| solve-thrash-pins | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U200 W25Q64JVxIQ 2700, U201 W25Q64JVxIQ 2700, U202 W25Q64JVxIQ 2700; REFUSED: U200(2700>1580),U201(2700>1580),U202(2700>1580) | - | - |
| spi-only | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| two-bme | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U5 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U5(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| d1-drv8876 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices none; ok | - | - |
| d2-drv8833 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices none; ok | - | - |
| sentinel-02-power | power / NO drop_limits | fab_tolerances; JLCPCB | pack max 6 x 4350 = 26100 mV, doubled 52200 mV; VIN abs max U10 LMR33630ADDA 38000, U11 LMR33630ADDA 38000, U12 TLV75518PDBV 6000; REFUSED: U10,U11 | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | power.input J1 XT30PW-M hot_plug 1 | VDD_5V: 1 loads (2000 mA), VDD_3V3: 11 loads (37 mA), VDD_1V8: 1 loads (20 mA) |
| invalid-zero-domain | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U2(1710>1580),U3(1800>1580),U4(2700>1580) | - | - |
| sentinel-01-ext-bmp581 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U5 BMP581 1710, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| sentinel-01-minimal | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| sentinel-01 | no power | no fab_tolerances; no fabricator named | - | bor_level 3 (falling min 2540 mV); MCU rail VDD_3V3; devices U2 BME280 1710, U3 TMP117 1800, U4 W25Q64JVxIQ 2700; REFUSED: U4(2700>2540) | - | - |
| sentinel-02-bench-hse8 | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U7 BMP581 1710, U5 ICM42688P 1710; REFUSED: U7(1710>1580),U5(1710>1580) | - | - |
| sentinel-02-bench | no power | no fab_tolerances; no fabricator named | - | bor_level 0 (falling min 1580 mV); MCU rail VDD_3V3; devices U7 BMP581 1710, U5 ICM42688P 1710; REFUSED: U7(1710>1580),U5(1710>1580) | - | - |

## Part 2 (A) measurement: drop on the emitted board, drop limit 5 % of nominal

Method (`dropmeasure.py`): in-memory copy of each power specification with supply_tier1's four "test:" stackup minima (outer_copper_min_um 30, plane_copper_min_um 15, plating_min_um 18, copper_temperature_max_c 125) and ONE `power.drop_limits` row at a time, 5 % of the domain's nominal (VDD_1V8 90 mV, VDD_3V3 165 mV, VDD_5V 250 mV); `emit`, then `extract`. Outputs: `/home/forge/.cache/aet-4300k/blast/drop/`.

| spec | domain, limit | result |
|---|---|---|
| sentinel-03 | VDD_3V3, 165 mV | extract rc 0, verdict pass, `extract.supply` judged 1: `drop_bound_microvolts` 90032 (90.0 mV) at load U1 `load_milliamps` 747; 16 supply pins 34898-62614 microohm, 10 return pins 38641-57910 microohm; source L2 pad 2, return U11 pad 1 |
| sentinel-03 | VDD_5V, 250 mV | extract rc 2 `SupplyTerminalMissing`: "the load U1 has no pin on `VDD_5V`, so the drop limit on `VDD_5V` judges nothing" |
| sentinel-02-power | VDD_1V8 / VDD_3V3 / VDD_5V | extract rc 2 `TranslatorPinUnbounded` for all three, before the supply gate ("translator U13 on `IMU_SCK` ... no stated maximum (SCES935A p. 9, Cio)"); unmeasurable here. docs/roadmap.md:1024 records 206-544 mV at 658 mA for this 2-layer power fixture, which exceeds 165 mV |

So "a drop limit per regulated domain" cannot be judged: only the MCU's rail (VDD_3V3) has a load pin the gate can use; limits on VDD_5V / VDD_1V8 are refused (5V) or unreachable (1V8 behind the translator refusal).

## Per rule: specifications newly refused or forced to edit

- **A. drop_limits required when power declared**: 2 of 36 force an edit: sentinel-03, sentinel-02-power. If limits were declared: sentinel-03 passes on VDD_3V3 (90 mV vs 165 mV; also needs the four stackup minima added, none of the 36 declares them); VDD_5V limit unjudgeable; sentinel-02-power cannot be measured (TranslatorPinUnbounded) and its recorded 2-layer drop (206-544 mV) would fail 165 mV.
- **B. fab_tolerances / single fabricator**: 2 declare `fab_tolerances` (sentinel-03, sentinel-02-power). Stackup `source` strings cite more than one fabricator in 1: sentinel-03 (OSH Park for dielectrics and finished thickness, JLCPCB for the five fab_tolerances). sentinel-02-power cites JLCPCB tolerances over "AET-WO-1305" 2-layer sources. OSH Park only: d1-24-i2c, d3-24-i2c-12-spi, four-layer, four-layer-power. i2c-spi-bench (template) has empty sources. All others cite no fabricator ("AET-WO-1305 conditions", IPC-4101). No specification declares supply stackup minima.
- **C. doubled pack <= every VIN abs max**: both power boards are 6S x 4350 mV = 26100 mV, doubled 52200 mV > LMR33630ADDA VIN abs max 38000 mV (power-parts.json:275-279): sentinel-03 (U10, U11) and sentinel-02-power (U10, U11) refused; neither declares a clamp. 2 of 36. sentinel-02-power's U12 (TLV75518PDBV, abs max 6000 mV) takes VDD_3V3, not the pack.
- **D. every supply-bounded device VCC min <= reset falling-threshold min, no supervisor**: `mcu.bor_level` is a required key (0 to 3; lib.rs:365-375), never absent; 0 = RM0433's factory default (POR/PDR only, 1.58 V falling min). DS12117 Rev 10 Table 16 p. 107, falling-edge minima: BOR0/POR/PDR 1.58 V, BOR1 1.95 V, BOR2 2.25 V, BOR3 2.54 V (maxima 1.68, 2.06, 2.37, 2.68). Device VCC min from parts.json `supply_millivolts.min` (citations.supply): W25Q64JV 2700 (Winbond rev J 9.2), TMP117 1800 (TI SNOSD82D 6.3), BME280 / BMP581 / ICM42688P / ICM20948 / ICP10111 1710. Result: 33 of 36 refused; 3 pass (driver-judges-nothing, d1-drv8876, d2-drv8833: no devices on the rail).
  - bor_level 3 (14 refused, all solely W25Q64JV 2700 > 2540): hse-crystal-16, hse-external-8, logger-4600, motors-dshot600, sentinel-01-bench, sentinel-02-own, sentinel-02-shaped, sentinel-03, serial-dshot-9600, spi-only, sentinel-02-power, sentinel-01-ext-bmp581, sentinel-01-minimal, **sentinel-01 (examples/sentinel-01.json: bor_level 3, U4 W25Q64JVxIQ 2700 > 2540, refused)**.
  - bor_level 0 (19 refused; any device with VCC min above 1580 mV): i2c-spi-bench, connectors, d1-24-i2c, d3-24-i2c-12-spi, four-layer-power, four-layer, four-port, imu-baro-1v8, motors-pwm, pair-standin, serial-four, solve-infeasible-shared, solve-pigeonhole-spi, solve-stress, solve-thrash-pins, two-bme, invalid-zero-domain, sentinel-02-bench-hse8, sentinel-02-bench.
- **E. reversed input refused for want of protection**: only the 2 power boards declare `power.input` (sentinel-03 J4 XT30PW-M, sentinel-02-power J1 XT30PW-M, each with one hot_plug cap C1); `input` has no reverse-protection field in either. Both would be forced to change; the other 34 are untouched.
- **F. regulated rail with no load**: none. sentinel-03: VDD_5V 1 load (2000 mA), VDD_3V3 13 loads (89 mA). sentinel-02-power: VDD_5V 1 (2000 mA), VDD_3V3 11 (37 mA), VDD_1V8 1 (20 mA). 0 refused.
