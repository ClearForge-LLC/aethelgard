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

## Results — after the predictions

### Where it ran

- Host, `cargo test` in a capped user scope (`MemoryMax=24G`, `MemorySwapMax=0`). No KiCad, Renode or tier-2 step ran: `emit` and `extract` are host Rust, and no container was started.
- **The probe** is `spike/4300c/probe.patch`, about 45 lines, applied and then reverted, so nothing in `crates/` changes. It adds `extract::probe_lines` and a test, `probe_4300c.rs`.
  - `probe_lines` is `extract::lines` widened to any list of nets. Every non-passive part on a net is a pin part, and any passive is refused by name.
  - The test emits each acceptance board with `schematic::emit`, then bounds every I2C line with today's `lines` and `bound`, and every SPI line (each bus's SCK, MOSI and MISO, and each device's chip select) with `probe_lines` and `bound`.
- Raw output: `spike/4300c/probe-output.txt`. Reproduce with `git apply spike/4300c/probe.patch && cargo test -p aethelgard-cli --test probe_4300c -- --nocapture`.

### §1.3, measured: the machinery is small; the source of C is not

- **The bound machinery is net-agnostic [measured].** `bound` accepted every SPI line of the three boards that exist, unchanged. No SPI line carries a passive, so the probe's refusal never fired. The pin term is the one I2C-specific part: `PIN_FF` is UM10204's 10 pF.
- **The ICM-20948 fixture has no board [measured].** `aethelgard emit` and `aethelgard generate` on `spi-imu.json` both exit 2 with `UnknownPart` (`U5` is a `ICM20948`, which has no vendored model). Its tier-2 test, `twin_tier2.rs:1436`, calls `generate_with` directly, so `generate` never draws it. **Extraction cannot bound a line on a board that is never drawn.** So the small path cannot check one of §2's four boards, and the only route that covers it is the work order's fallback: a declared `net_constraint` per SPI line.

### The bounds, in pF (fF ÷ 1000)

- **Copper** = trace + vias + pads, from the probe [measured].
- **A** = extraction as it stands: every pin at 10 pF, the MCU's own included [measured].
- **B** = copper + every pin but the MCU's, each at its cited maximum: 10 pF for an I2C pin (UM10204); 8 pF for the flash, the larger of `CIN` 6 and `Cout` 8 [measured copper; pin values read].
- **Row** is C rounded up to 10, 30 or 50 pF, and **speed** is the slowest whose Fmax meets the rate. VDD min is 3135 mV (3300 − 5 %), the 2.7–3.6 V band. Rate: the bus speed (SPI 20 MHz, I2C 400 kHz).

| Board | Line | Copper | A | A: row → speed | B | B: row → speed |
|---|---|---|---|---|---|---|
| reference | `I2C_SDA` | 16.8 | 46.8 | 50 → `00` | 36.8 | 50 → `00` |
| reference | `I2C_SCL` | 28.2 | **58.2** | **> 50: refused** | 48.2 | 50 → `00` |
| reference | `SPI_SCK` | 10.5 | 30.5 | 50 → `01` | 18.5 | 30 → `01` |
| reference | `SPI_MOSI` | 9.2 | 29.2 | 30 → `01` | 17.2 | 30 → `01` |
| reference | `SPI_MISO` | 12.0 | 32.0 | 50 → `01` | 20.0 | 30 → `01` |
| reference | `FLASH_CS` | 8.9 | 28.9 | 30 → `01`* | 16.9 | 30 → `01`* |
| 4-layer | `I2C_SDA` | 15.9 | 45.9 | 50 → `00` | 35.9 | 50 → `00` |
| 4-layer | `I2C_SCL` | 18.8 | 48.8 | 50 → `00` | 38.8 | 50 → `00` |
| 4-layer | SPI (4 lines) | 8.9–12.0 | 28.9–32.0 | 30/50 → `01` | 16.9–20.0 | 30 → `01` |
| D1 | `SDA0`…`SCL3` (8 lines) | 17.8–39.6 | **97.8–119.6** | **> 50: refused, all 8** | **87.8–109.6** | **> 50: refused, all 8** |
| D1 | `SCK0`, `MOSI0`, `MISO0`, `CS0_0` | 9.5–14.8 | 29.5–34.8 | 30/50 → `01` | 17.5–22.8 | 30 → `01` |
| ICM-20948 fixture | every line | **no board** | — | — | — | — |

\* A chip select at its bus's rate. At `00` it would need its rate at or below 12 MHz (D4).

- **DS12117 Rev 10 Table 149 (rev V), pp. 245–246**, checked against the rendered pages [read]. Every value in `4300` FEEDBACK S5 is right.
  - Three rows print a superscript `v` where note `(5)` belongs: speed `10` at 10 pF and 1.62–2.7 V, and speed `11` at 50 pF and at 30 pF, both 2.7–3.6 V. The typo is in ST's PDF, not the extraction. Note 5 still governs those rows by their speed.
  - Table 52 (rev Y, pp. 141–142) gives the same Fmax in every cell, read from the text extraction. **The check therefore holds or fails identically on either silicon**, which answers §4.1's third case for these two tables.

### Each prediction beside its result

| # | Prediction | Result | Label | Verdict |
|---|---|---|---|---|
| P1 | Small; no passive on an SPI line | The bound runs unchanged on every SPI line, and no passive is present. But one of the four boards has no board to extract. | measured | right about the machinery; missed the board-less fixture |
| P2 | SPI copper 5–20 pF; SCK largest; MISO and CS smallest | 8.9–12.0 pF on the reference. **MISO is the largest** (12.0 pF: the longest route, 23.25 mm over 7 segments) and CS the smallest. | measured | range right, order wrong |
| P3 | ≤ 30 pF with the flash's pin; `01` the slowest sufficient | B: 16.9–20.0 pF, the 30 pF row, `01` (80 MHz ≥ 20 MHz; `00` gives 12). A: 28.9–32.0 pF, still `01` (60 MHz) | measured | as predicted; §7's third condition does not fire |
| P4 | The rule refuses I2C lines §2 needs to pass | The reference's `I2C_SCL` is 58.2 pF under A, refused; 48.2 pF under B. **All eight D1 I2C lines are 87.8–119.6 pF, refused under either.** | measured | as predicted |
| P5 | 4-layer within ±15 % of the reference; `I2C_SCL` above 50 pF under A | `I2C_SCL` is 48.8 pF under A, **16 % below** the reference's 58.2, and **passes**. Its route is shorter (29.4 mm against 43.5) with 4 vias against 7. | measured | wrong |
| P6 | ICM fixture: SPI ≤ 50 pF, so `00` at 7 MHz | No board exists, so there is no C at all | measured | wrong premise |
| P7 | D1's SPI: `01` | `01` under both A and B | measured | as predicted |

## STOP (§1.3, and §2's acceptance)

The SPI half of §1.3 measured **small**, and on the three boards that exist the reference's SPI lines pass at `01`. Four facts still block the build; each needs a ruling, not a builder's fork:

1. **No row above 50 pF, and the I2C lines exceed it.** §1.2 refuses above 50 pF, but §2 requires the check to hold on every D1 line. All eight D1 I2C lines are 87.8–119.6 pF whatever is counted. UM10204 allows 400 pF on a Fast-mode bus, so these are legal I2C lines that Table 149 has no figure for. If the MCU's own pin counts, the reference's `I2C_SCL` (58.2 pF) is refused too, and that is §7's second condition (a reference verdict changes for a reason the work order does not name).
2. **One acceptance board has no board.** The ICM-20948 fixture cannot be emitted (no KiCad symbol for `ICM20948`), so extraction gives it no C. The small path checks three of §2's four boards.
3. **Two parts' pin capacitance is not a cited maximum** [read]:
   - the STM32H753's `CIO` is 5 pF typical, no maximum (Tables 49 and 146);
   - the ICM-20948's `CI` is "< 10" pF, printed in the TYP column, for inputs only. SDO, the IMU's MISO driver, has no figure.

   Only the W25Q64JV gives maxima (`CIN` 6, `Cout` 8 pF).
4. **The work order gives a rate only for SCK and I2C.** MOSI, MISO and each chip select need one, and the chip select's decides whether it leaves `00`. §2 allows only the `10` → `01` change on the SPI pins.

## Decisions needed

- **D1 · I2C lines above 50 pF.**
  - **(a)** The Fmax check judges push-pull lines only. Note 3 defines Fmax by (tr + tf) ≤ 2/3 T and 45–55 % duty, which is a property of a driven edge. An open-drain line's rise is its pull-up's, and the rise is already judged: `extract` checks the bound against the declared capacitance, and the campaign checks the rise time (`RUL-19`). I2C keeps `00` by policy, named as the one exception. **Recommended.**
  - **(b)** Keep the rule. D1 and every bus with more than about three devices are refused, and the reference only if the MCU's pin is excluded (D3).
- **D2 · Where C comes from.**
  - **(a)** The fallback: a declared `net_constraint` per SPI line, refused by name when absent. `extract` checks the bound against it, as for I2C, using the small widening measured here. `solve` copies it into `SolvedTopology`, so the generator chooses the speed from the topology alone (`RUL-16`), and the board-less ICM fixture is checked at its declared C. It needs a specification version and a declared value on every fixture's SPI nets. **Recommended.**
  - **(b)** The extracted bound alone. `generate::tree` reads the board it has just emitted and passes C to `generate_with`. A board-less fixture is refused, so §2 drops the ICM fixture or it gains a symbol (outside this fence). The firmware then depends on the layout.
  - **(c)** Choose at the 50 pF row (`01` for 12–60 MHz, `00` below 12 MHz), with `extract` checking C ≤ 50 pF where a board exists. This needs no specification change, but a board-less fixture's C is assumed, not bounded (`INV-06`), and a 12–16 MHz line at 10 pF gets `01` where `00` would do.
- **D3 · Which pins load a line.**
  - **(a)** The load on an MCU pin is every other pin on the line plus the copper. Table 149's C is the external load, so the MCU's own pin is never counted, and its typical-only `CIO` is never needed. Each device pin counts at its part's cited maximum, a new `pin_capacitance` value group in `parts.json`; W25Q64JV 8 pF. **Recommended.**
  - **(b)** As extraction does for I2C: every pin, the MCU's included, at 10 pF. The reference's `I2C_SCL` is then refused (D1).
  - **Separately, the ICM-20948:** accept "< 10 pF" as a 10 pF maximum and apply it to SDO, labelled inferred, or name the part unboundable and leave its lines to a declared constraint (D2 (a)).
- **D4 · The rates of MOSI, MISO and a chip select.**
  - **(a)** MOSI and MISO at the bus speed, which is conservative: a data line alternating every bit toggles at half of it. A chip select at the bus speed ÷ 8, since the firmware toggles it at most once per byte. The chip select then stays at `00` (2.5 MHz against 12 MHz). **Recommended.**
  - **(b)** Every SPI line at the bus speed. The reference's `FLASH_CS` then moves `00` → `01`, which is a change §2 does not name.

### Ruling-independent: ready to build on resume

None of these depends on D1–D4. They were not built, because a stop at §1.3 builds nothing past it:

- §1.1, the Fmax table as data (rows `10`/`11` held with note 5, never selectable);
- §1.5, the parts limits at `validate`, with F20's identity shape;
- §1.6, F21's controller names;
- §1.7, F11's ordering-code rename.
