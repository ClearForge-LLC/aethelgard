| Prediction | Result | Measured |
|---|---|---|
| P1.1 | held | identity window: 4 U5 lines, equal to the 4 `model.rs` expects |
| P1.2 | held | 3 Noisy lines as predicted; the value only in `Write at bank #0 to 0x6 (PowerManagement1): value 0x1` (Debug) and the second `Received byte`. Order: Preparing, Received 0x6, Debug, Warning, Received 0x1 |
| P1.3 | held | U5 warns 1 time(s) in the run, inside the configuration window: `Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).` |
| P1.4 | held | 14 Noisy lines (predicted 14); bytes ['0x0', '0x0', '0x0', '0x0', '0x0', '0x0']; registers 0x2D–0x32 named |
| P1.5 | held | with `DefaultAccelerationZ 1`: AccelerationXOutHigh 0x0, AccelerationXOutLow 0x0, AccelerationYOutHigh 0x0, AccelerationYOutLow 0x0, AccelerationZOutHigh 0x40, AccelerationZOutLow 0x0 |
| P1.6 | failed in part | U5's lines outside its steps: {'Finishing transmission, going idle': 6}, by the edge that preceded each: {'GPIO #1 True': 4, 'GPIO #0 False': 1, 'GPIO #0 True': 1}. On every edge of any select, the multiplexer finishes every deselected device (`SPIMultiplexer.cs` `UpdateChipSelectState`), so U5 also goes idle on U4's edges. U5 warns only P1.3's line, but **U4 warns 8 times** in a correct run (`Transmission finished in the unexpected state: RecognizeOperation`, `GenericSpiFlash.cs` L95): it is finished while idle on edges of either select |
| P1.7 | held | 3 runs: kept lines and reads byte-identical: True |
| P2.1 | held | 2 U5 windows: the forged one holds 0 U5 lines; `GPIO #1` edges and U5's idle line are logged; no `Preparing for write`. Records still claim `0x601`: 0x00000601 |
| P2.2 | held | 2 U5 windows; the configuration step's 3 lines present; no read of 0x2D; record 4 claims 0x22020000/0x00000000 |
| P3.1 | held | 3 runs, 46 transfer lines each: 0 outside their own select's window or inside another's. The two selects' windows never overlap |
| P3.2 | held for the transmit lines; a second kind found | `Tried to transmit … to device 0x0, but it's not connected`: {'#0': 12} (inside `GPIO #0` windows only). But the multiplexer also logs `Tried to finish transmission to device 0x0, but it's not connected` (Warning) on every edge: {'none': 8, '#1': 3} by the window open when logged. The judge's `unrouted_line` takes only `Tried to transmit byte` (`renode.rs` L529), so these do not reach `E2` today |
| P3.3 | consistent (inferred) | P2.1's forged window is empty: forged edges add an empty window and move no transfer line. Windows attribute; they do not witness |
| P4.1 | held | `IRQ IsSet` after the run: False; `INT_STATUS_1` read: 0x00000000. The `INT_ENABLE_1` write logs `IRQ unset` (Debug) and an `Interrupts: … Raw Data Ready False …` Noisy line |
| P4.2 | held | after the firmware writes `INT_STATUS_1` = 1: `IRQ set` (Debug), `IsSet` True, status read 0x00000001 |
| P6.1 | held | identity 0x000000BD, data 0x00000000; device lines: ['[DEBUG] i2c1.baro: Writing 0x90 to register 0x20']; Noisy: 0; device warnings 0 |
| P6.2 | held for identity; failed in part for data | identity 0x00000838 (`00 08` + CRC `0x38`). Debug lines: 4, one per command and one per read, bytes in the line. **The measurement read also logs 2 Noisy lines** (`A=… B=… C=…`, `p_raw=…`), so a data step is visible at Noisy, but not its identity. Device warnings 0 |
| P6.3 | held but for the data step's count | Noisy lines per step: [4, 4, 7] (predicted 4, 4, 5: the 2-byte read logs `Write`, `Preparing`, `Reading`, then a `Read value` and an `Auto-incrementing` line per byte, 7). Values decimal (`Read value 85`, `67`, `79`); register hex 8 digits. Device warnings 0. 3 runs identical: True |
| P6.4 | held (read) | none of the three barometer models has a GPIO; no barometer case logged an IRQ line |

Step sizes in the correct ICM run (log bodies without the host timestamp; the judge's events are built from these):

| Step | U5 lines (all levels) | Noisy | bytes, all | bytes, Noisy |
|---|---|---|---|---|
| identity | 4 | 4 | 261 | 261 |
| configuration write | 5 | 3 | 394 | 195 |
| data read, 6 bytes | 14 | 14 | 1006 | 1006 |

Longest Noisy line of the data read: 87 bytes; mean 71.9.

Every [WARNING] of the correct ICM run (`imu-script`, run 0), by source:

- 1 × `(no source): Tried to finish transmission to device 0x0, but it's not connected`
- 2 × `(no source): Tried to finish transmission to device 0x1, but it's not connected`
- 1 × `gpioPortB: Unhandled write to offset 0x4. Unhandled bits: [6] when writing value 0x40. Tags: OT6 (0x1).`
- 1 × `gpioPortB: Unhandled write to offset 0x4. Unhandled bits: [7] when writing value 0x80. Tags: OT7 (0x1).`
- 1 × `i2c1: Unhandled write to offset 0x10. Unhandled bits: [0-1, 4-6, 8, 13, 21-22] when writing value 0x602173. Tags: SCLL (0x73), SCLH (0x21), SCLDEL (0x6).`
- 1 × `i2c1: Unhandled write to offset 0x1C. Unhandled bits: [4] when writing value 0x30. Tags: NACKCF (0x1).`
- 8 × `spi1.spi1mux.u4: Transmission finished in the unexpected state: RecognizeOperation`
- 1 × `spi1.spi1mux.u5: Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).`
