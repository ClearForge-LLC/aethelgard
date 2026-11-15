# AET-WO-4300c — FEEDBACK

Builder: Claude Code (Opus 5.5), Furnace, worktree `~/aethelgard-4300c`, branch `wo/aet-4300c-pin-speeds` from `origin/main` at `c621461`. Work order read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #127).

Labels: **[measured]** run here; **[read]** read at source; **[inferred]** reasoned, not run.

## Predictions — written before any measured run

**Already seen before these were written (orientation, not new measurements).** `aethelgard emit` of the reference, to see which nets are routed. `aethelgard extract` on it, whose I2C bounds are pinned in `extract_tier1.rs:133-134` [read]: `I2C_SDA` 46,800 fF and `I2C_SCL` 58,235 fF, each with 30,000 fF of pins (three pins at UM10204's 10 pF, the MCU's included). The emitted reference routes every SPI net: `SPI_SCK` 15 segments and 2 vias, `SPI_MOSI` 13 and 2, `SPI_MISO` 7 and 2, `FLASH_CS` 7 and 2 [measured].

**Read at source, before any run:**
- DS12117 Rev 10 (sha256 `607b9ead…c032`) Tables 52 (rev Y, p. 141–142) and 149 (rev V, p. 245–246) give the same Fmax at every speed, load and band [read, from the text extraction; to be checked against the rendered pages].
- `extract::line_with` takes a `Line` (a net, a bus, the parts on it) and bounds that net's copper. It names no I2C fact except `PIN_FF`, UM10204's `Ci` [read].
- `extract::lines` is I2C-only in three places [read]:
  - it walks `spec.i2c_buses`;
  - it tests pull-ups against the bus's domain;
  - it counts every pin at `PIN_FF`.
- The CLI's `extract` looks each line up in `analog_lines`, which holds no SPI net [read].
- Pin capacitance at source [read]:
  - W25Q64JV Rev J §9.4: `CIN` 6 pF max, `Cout` 8 pF max (note 1: by design or characterisation).
  - ICM-20948 DS-000189 1.6 §3.4 (A.C. electrical characteristics, p. 14): digital inputs (SCLK, SDI, CS) `CI` "< 10" pF, printed in the TYP column. SDO has no figure.
  - DS12117 Rev 10 Table 49 (rev Y, p. 136) and Table 146 (rev V, p. 241): `CIO` 5 pF, **typical only**, no maximum.

| # | Question | Prediction | Basis |
|---|---|---|---|
| P1 | §1.3: does extraction bound an SPI line as it stands? | **Small.** The bound is net-agnostic; only the line list and the pin term change. No passive sits on a reference SPI line, so the pull-up rule is not reached. | inferred from the code |
| P2 | The reference's SPI copper bounds (trace + vias + pads, no pins) | Each between 5 and 20 pF; `SPI_SCK` the largest, `SPI_MISO` and `FLASH_CS` the smallest | inferred: `I2C_SDA`'s copper is 16.8 pF over 19 segments and 4 vias |
| P3 | The reference's SPI lines with the flash's pin (8 pF) added | Each at most 30 pF, so the 30 pF row: `00` gives 12 MHz (< 20 MHz), `01` gives 80 MHz. **`01` is the slowest sufficient speed; §7's third condition does not fire.** | P2 and Table 149 |
| P4 | The I2C lines against the 50 pF ceiling (§1.2) | **The work order's rule refuses I2C lines it requires to pass.** The reference's `I2C_SCL` is 58.2 pF counting the MCU's own pin, and 48.2 pF without it. D1 puts six devices on each bus, 60 pF of pins at 10 pF each before any copper, so **every D1 I2C line is above 50 pF whatever is counted.** | read (the pinned bounds) and inferred (D1) |
| P5 | The 4-layer fixture's I2C lines | Within ±15 % of the reference's 2-layer figures: `I2C_SCL` above 50 pF with the MCU's pin counted | inferred |
| P6 | The ICM-20948 fixture (`spi-imu.json`, 7 MHz) | Its SPI lines carry two device pins and stay at most 50 pF. `00` (12 MHz at 30–50 pF) suffices, so its SPI pins **move to `00`, not `01`.** Its I2C lines are like the reference's. | inferred |
| P7 | D1's SPI lines (one flash, 20 MHz) | Like the reference's: `01` | inferred |

**What P4 means, if it holds.** No reading of §1.2 lets both "above 50 pF the check refuses" and §2's "the Fmax check holds on every line of … D1" stand. The resolution is a ruling, not a fork: a smaller C, an invented row above 50 pF and a quiet open-drain exemption are each a §4.1 case. If the measurement confirms P4, this work order stops with the numbers.
