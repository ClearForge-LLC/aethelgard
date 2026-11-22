| board | validate / solve / emit | paths | by class | stated (worst at VDD < 1 V, pin, source) |
|---|---|---|---|---|
| `crates/aethelgard-cli/tests/fixtures/connectors.json` | 0 / 0 / 0 | 7 | connector 7 | none |
| `crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` | 0 / 0 / 2 (emit:DrawsNoBoard) | 6 | connector 5, pin_device_current_output 1 | 4683 mV on PF3 (FT_ha), pin_device_current_output, VMOTOR |
| `crates/aethelgard-ir/tests/fixtures/d2-drv8833.json` | 2 / 2 / skip (PartLimitUndeclared) | 5 | connector 5 | none |
| `crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json` | 0 / 0 / 0 | 8 | connector 8 | none |
| `crates/aethelgard-cli/tests/fixtures/four-layer.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/four-layer-power.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json` | 0 / 0 / 0 | 10 | connector 10 | none |
| `crates/aethelgard-cli/tests/fixtures/hse-external-8.json` | 0 / 0 / 2 (emit:DrawsNoBoard) | 11 | connector 10, foreign_clock 1 | none |
| `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json` | 0 / 0 / 2 (emit:DrawsNoBoard) | 2 | connector 2 | none |
| `examples/invalid-zero-domain.json` | 2 / 2 / skip (ZeroNominalVoltage) | 0 | - | none |
| `crates/aethelgard-cli/tests/fixtures/logger-4600.json` | 0 / 0 / 0 | 4 | connector 4 | none |
| `crates/aethelgard-cli/tests/fixtures/motors-dshot600.json` | 0 / 0 / 0 | 6 | connector 6 | none |
| `crates/aethelgard-cli/tests/fixtures/motors-pwm.json` | 0 / 0 / 0 | 6 | connector 6 | none |
| `crates/aethelgard-cli/tests/fixtures/pair-standin.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `examples/sentinel-01.json` | 0 / 0 / 0 | 20 | connector 8, test_point 12 | none |
| `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json` | 0 / 0 / 0 | 16 | connector 6, test_point 10 | none |
| `examples/sentinel-01-ext-bmp581.json` | 0 / 0 / 0 | 20 | connector 8, test_point 12 | none |
| `examples/sentinel-01-minimal.json` | 0 / 0 / 0 | 4 | connector 4 | none |
| `examples/sentinel-02-bench.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `examples/sentinel-02-bench-hse8.json` | 0 / 0 / 2 (emit:DrawsNoBoard) | 3 | connector 2, foreign_clock 1 | none |
| `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` | 0 / 0 / 0 | 4 | connector 4 | none |
| `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` | 0 / 0 / 0 | 3 | battery_divider 1, connector 2 | 2536 mV on PF3 (FT_ha), battery_divider, battery |
| `crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json` | 0 / 0 / 0 | 10 | connector 10 | none |
| `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` | 0 / 0 / 0 | 40 | battery_divider 1, connector 16, esc 4, test_point 19 | 2536 mV on PF3 (FT_ha), battery_divider, battery |
| `crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json` | 0 / 0 / 0 | 8 | connector 8 | none |
| `crates/aethelgard-cli/tests/fixtures/serial-four.json` | 0 / 0 / 0 | 12 | connector 12 | none |
| `crates/aethelgard-cli/tests/fixtures/solve/infeasible-shared.json` | 0 / 2 / skip | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/solve/pigeonhole-spi.json` | 0 / 2 / skip | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/solve/stress.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/solve/thrash-pins.json` | 0 / 2 / skip | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/spi-only.json` | 0 / 0 / 0 | 2 | connector 2 | none |
| `crates/aethelgard-cli/tests/fixtures/two-bme.json` | 0 / 0 / 0 | 2 | connector 2 | none |
