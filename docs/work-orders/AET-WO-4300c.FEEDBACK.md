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
  - W25Q64JV Rev J §9.4: `CIN` 6 pF max, `Cout` 8 pF max (note 1: "Tested on sample basis … TA = 25 °C, VCC = 3.0 V", so qualified maxima).
  - ICM-20948 DS-000189 1.6 §3.4 (A.C. electrical characteristics, p. 14): digital inputs (SCLK, SDI, CS) `CI` "< 10" pF, printed in the TYP column. SDO has no capacitance figure, only a test load (`tVD.SDO` at Cload = 20 pF).
  - DS12117 Rev 10 Table 49 (rev Y, p. 137) and Table 146 (rev V, p. 241): `CIO` 5 pF, **typical only**, no maximum.

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
- **The probe** is `spike/4300c/probe.patch`, about 110 lines, applied and then reverted, so nothing in `crates/` changes. It adds `extract::probe_lines` and a test, `probe_4300c.rs`.
  - `probe_lines` is `extract::lines` widened to any list of nets. Every non-passive part on a net is a pin part, and any passive is refused by name.
  - The test emits each acceptance board with `schematic::emit`, then bounds every I2C line with today's `lines` and `bound`, and every SPI line (each bus's SCK, MOSI and MISO, and each device's chip select) with `probe_lines` and `bound`.
- Raw output: `spike/4300c/probe-output.txt`. Reproduce at the stop's commit `139226e`, where both files live (deleted in the build as the ruling's reduction): `git checkout 139226e && git apply spike/4300c/probe.patch && cargo test -p aethelgard-cli --test probe_4300c -- --nocapture`.

### §1.3, measured: the machinery is small; the source of C is not

- **The bound machinery is net-agnostic [measured].** `bound` accepted every SPI line of the three boards that exist, unchanged. No SPI line carries a passive, so the probe's refusal never fired. The pin term is the one I2C-specific part: `PIN_FF` is UM10204's 10 pF.
- **The ICM-20948 fixture has no board [measured].** `aethelgard emit` and `aethelgard generate` on `spi-imu.json` both exit 2 with `UnknownPart` (`U5` is a `ICM20948`, which has no vendored model). Its tier-2 test (`twin_tier2.rs:1436`) calls `generate_with` directly (line 1449), so `generate` never draws it. **Extraction cannot bound a line on a board that is never drawn.** So the small path cannot check one of §2's four boards, and the only route that covers it is the work order's fallback: a declared `net_constraint` per SPI line.

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
  - Three rows print a superscript `v` where note `(5)` belongs: speed `10`'s tr/tf row at 10 pF and 1.62–2.7 V, and speed `11`'s Fmax rows at 50 pF and at 30 pF, both 2.7–3.6 V. The typo is in ST's PDF, not the extraction. Note 5 still governs those rows by their speed.
  - Table 52 (rev Y, pp. 141–142) gives the same Fmax in every cell. The adversary compared both tables with pdfplumber, cell by cell. **The check therefore holds or fails identically on either silicon**, which answers §4.1's third case for these two tables.

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

## STOP: §1.2 against §2, found at the §1.3 measurement

§1.3's own test measured **small**, which on its own would mean "build it", and the reference's SPI lines pass at `01` on the three boards that exist. What blocks the build is that §1.2 and §2 cannot both be met: §1.2 has no exceptions and refuses above 50 pF, while §2 requires the check to hold on four boards. (An earlier draft appealed to §7's second condition; the adversary, finding 12, is right that a refusal by the check this work order names is a named reason, so that appeal is withdrawn.) Four facts, each needing a ruling rather than a builder's fork:

1. **No Table 149 row above 50 pF, and the I2C lines exceed it.**
   - All eight D1 I2C lines are 87.8–119.6 pF, whatever is counted.
   - Each D1 line carries seven device pins, not six: the TMP117s at 0x4A and 0x4B strap ADD0 to SDA or SCL (`schematic.rs:628-631`), so two of them each put two pins on one line. This was confirmed by the adversary (finding 3).
   - Neither part offers a smaller cited figure. The BME280 prints no pin `Ci`. The TMP117's 4 pF is typical, not a maximum.
   - **D1 already fails today's `extract` on `main`** [measured, adversary finding 2, re-run by the builder]. Seven of eight lines exceed the fixture's own declared 100,000 fF; only `SDA1` passes, at 97,767 fF. No test runs `extract` on D1 (`stackups_tier1.rs` only emits it). So these are not "legal lines Table 149 cannot describe": they exceed their own specification.
   - If the MCU's own pin counts, the reference's `I2C_SCL` (58.2 pF) is refused as well.
2. **One acceptance board has no board.** `emit` refuses the ICM-20948 fixture's `U5` as "no vendored model" (`schematic.rs:45`): `aethelgard-kicad-lib` vendors no ICM-20948. A KiCad library on this box does hold `Sensor_Motion:ICM-20948`, but vendoring it is outside this fence. Extraction therefore gives this board no C.
3. **Two parts' pin capacitance is not a cited maximum** [read]:
   - the STM32H753's `CIO` is 5 pF typical, no maximum;
   - the ICM-20948's `CI` is "< 10" pF in the TYP column, for inputs only, with no figure for SDO.

   Only the W25Q64JV gives maxima (`CIN` 6, `Cout` 8 pF), and those are sampled at 25 °C and 3.0 V.
4. **The work order gives a rate only for SCK and I2C.** MOSI, MISO and each chip select need one. MISO is not driven by the MCU at all (finding 8 and D3).

## Decisions needed

- **D1 · I2C lines above 50 pF.**
  - **(a)** The Fmax check judges driven (push-pull) lines only, and I2C is judged by the figure DS12117 gives for it. DS12117 Rev 10 §6.3.32 (p. 188, rev Y; the same text in the rev V chapter), "I2C interface characteristics", says the I2C timing requirements of UM10204 are "guaranteed by design when the I2C peripheral is properly configured" and `i2c_ker_ck` is above Table 93's minimum. Its tr = 0.8473 · Rp · Cload bounds the load in Fast-mode Plus only. That is a cited basis for I2C lines, not a bare exemption (adversary finding 10, re-read by the builder). Table 149's note 3 (tr + tf ≤ 2/3 T, 45–55 % duty) describes a driven edge; an open-drain rise is its pull-up's. `extract` and the campaign keep judging the I2C rise (`RUL-19`). **Recommended.**
    - The cost is that the MCU-driven fall edge is not checked against Table 149. The risk is low: 16.6 ns at 50 pF, against UM10204's 300 ns (inferred).
    - **D1 the fixture fails `extract` either way** (STOP 1). Whether that is fixed in the fixture (its declared bound, or fewer devices per bus) is a separate ruling, not this work order's.
  - **(b)** Keep the rule as written. D1, and any bus with more than about three devices, is refused, and so is the reference unless the MCU's own pin is excluded (D3).
- **D2 · Where C comes from.**
  - **(a)** The work order's fallback: a declared `net_constraint` per SPI line, refused by name when absent. `extract` checks the bound against it, as it does for I2C, using the small widening measured here. `solve` copies it into `SolvedTopology`, so the generator chooses the speed from the topology alone (`RUL-16`). **Recommended.**
    - **Its gap, named** (adversary finding 11): on a board-less fixture such as the ICM-20948's, nothing ever bounds the declared C. That is declared intent rather than a substituted default (`INV-06`), but it is unverified until a board exists.
    - It needs a specification version and a declared value on every fixture's SPI nets.
  - **(b)** The extracted bound alone. `generate::tree` reads the board it has just emitted and passes C to `generate_with`. A board-less fixture is refused, so §2 drops the ICM fixture or the fixture gains a vendored symbol (outside this fence). The firmware then depends on the layout. C is the measured quantity here, not the criterion, so this does not breach `INV-07`.
  - **(c)** Choose at the 50 pF row: `01` for 12–60 MHz and `00` below 12 MHz, with `extract` checking C ≤ 50 pF where a board exists. No specification change is needed, but a board-less fixture's C is assumed, not bounded, and a 12–16 MHz line at 10 pF gets `01` where `00` would do.
- **D3 · Which pins load a line, and in which direction.**
  - **(a)** For an MCU-driven line (SCK, MOSI, a chip select), the load is every other pin on the line plus the copper. Table 149's C is the external load, so the MCU's own pin is not counted. Each device pin counts at its part's cited maximum, from a new `pin_capacitance` value group in `parts.json`; for the W25Q64JV that is 8 pF. **Recommended for driven lines.**
  - **(b)** As extraction does for I2C: every pin, the MCU's included, at 10 pF. The reference's `I2C_SCL` is then refused (D1).
  - **MISO is the device's output, not the MCU's** (adversary finding 8). The SPI is a master (`proofs.rs:321`), so the MCU's MISO pin is an input, and its `OSPEEDR` governs nothing. Table 149 does not apply there; what matters is the device's own drive into its load: W25Q64JV §9.5 characterises at CL 30 pF max, and the ICM-20948 at Cload 20 pF. Counting the MCU's pin in that load needs exactly the typical-only `CIO`. The options are:
    - (i) judge MISO against each driving device's characterised load, with the MCU's `CIO` declared or ruled;
    - (ii) leave MISO's `OSPEEDR` at a fixed policy value, named as having no electrical effect in master mode.
  - **The ICM-20948:** either accept "< 10 pF" as a 10 pF maximum, labelled inferred, or name the part unboundable and leave its lines to a declared constraint (D2 (a)).
- **D4 · The rate of a chip select, and its scope.**
  - **My first recommendation is withdrawn:** a chip select at the bus speed ÷ 8, on the grounds that the firmware toggles it at most once per byte. Nothing enforces that: `set_high` and `set_low` on a chip select are public (`boards/sentinel-01/src/board.rs:229-235`), so firmware may toggle it at loop speed and still build. A rate that rests on the firmware's behaviour is §4.1's "a chip select toggled faster than its bus", and close to `INV-02`. This was the adversary's finding 9, re-read by the builder.
  - **(a)** A chip select is `Role::Output`, not a peripheral pin in §1.2's words, so it is out of this check's scope, and its policy stays `00`. **This needs a scope ruling.**
  - **(b)** A chip select at its bus's speed. The reference's `FLASH_CS` then moves `00` → `01`, a change §2 does not name.
  - **Either way:** the W25Q64JV characterises its inputs at TR/TF ≤ 5 ns (§9.5). Speed `00` gives 13.3–16.6 ns and `01` at 50 pF gives 5.2 ns, so neither meets that measurement condition at 30–50 pF. That is a test condition, not a stated input requirement (read), but the ruling should know it.
  - MOSI at the bus speed gives `01` under any reading.

### Ruling-independent: ready to build on resume

None of these depends on D1–D4. They were not built, because a stop at §1.3 builds nothing past it:

- §1.1, the Fmax table as data (rows `10`/`11` held with note 5, never selectable);
- §1.5, the parts limits at `validate`, with F20's identity shape;
- §1.6, F21's controller names;
- §1.7, F11's ordering-code rename.

## Adversarial pass (full, a fresh Opus subagent, on a frozen copy at `139226e`)

**Verdict:** it found no reading of the work order that proceeds without a ruling. Each finding was spot-checked by the builder before it was adopted above.

| # | Finding | Severity | Label | Disposition |
|---|---|---|---|---|
| 1 | All 24 probe lines re-run byte-identical, and every table value recomputes. The patch is about 110 lines, not 45 | confirmed / nit | measured | the line count is corrected |
| 2 | D1 already fails `extract` on `main`: 7 of 8 lines exceed the declared 100 pF, and no test runs it | major | measured, and re-run by the builder (exit 1) | in STOP 1 and D1 |
| 3 | D1's 7 pins per line are real (TMP117 ADD0 straps to SDA or SCL). No cited maximum brings a line to 50 pF | confirmed | read | in STOP 1; P4's "six" understated it |
| 4 | Tables 52 and 149 are identical cell by cell; the speed-`10` "v" marker is on the tr/tf row | confirmed / imprecision | read | corrected |
| 5 | `CIO` is typical only; Table 49's row is on p. 137 | confirmed / nit | read | corrected |
| 6 | The ICM "< 10" is in the TYP column; SDO has only a 20 pF test load. The W25Q64JV maxima are sampled at 25 °C and 3.0 V | confirmed / omission | read | added |
| 7 | "No KiCad symbol" was loose: the refusal is "no vendored model", and a KiCad library holds `ICM-20948` | minor | read | corrected; vendoring it is outside the fence |
| 8 | MISO is the device's output (the SPI is a master), so Table 149 does not govern it | major | read, inferred | added to D3 |
| 9 | D4(a)'s ÷ 8 rested on firmware behaviour; `set_high` and `set_low` are public | major | read, re-read by the builder | D4(a) withdrawn |
| 10 | DS12117 §6.3.32 gives a cited I2C basis | minor | read, re-read by the builder (Fm+ scope narrowed) | added to D1(a) |
| 11 | D2(a)'s board-less gap was unnamed | minor | inferred | named |
| 12 | §1.3's own test measured small; the stop is §1.2 against §2, and the §7 appeal is a stretch | minor | read | the STOP is reframed and the appeal withdrawn |

## Reduction pass

Nothing in `crates/` changes. `spike/4300c/` (the probe and its output) is the only non-document addition, kept so the ruling can reproduce the numbers. Deleting it after the ruling is the reduction (as `4300` did with `spike/4300/`).

## Gates

- **Gates:** `cargo fmt --all -- --check` passes; `cargo clippy --workspace --all-targets -- -D warnings` exits 0; `cargo test --workspace` passes with no failing test binary [measured, on `139226e`; nothing compiled has changed since].
- **Tier 2:** no tier-2 test is added, so `TIER2_MIN` stays at 96.
- **Containers:** none were started.

---

# BUILD, after the ruling (resumed 2026-11-15, fresh session)

The ruling is on PR #129 (comment 6887905174, copy at `~/.cache/aet-4300c-ruling.md`): D1 (a), D2 (a), D3 (a) with MISO (ii), and D4 (a). It amends the work order where the two differ. `origin/main` at `9dc8861` was merged first (`b3f1563`).

## Forks taken inside the ruling (each for the architect to overturn)

- **F-a · Which SPI lines carry a declaration.** **SCK and MOSI of every SPI bus, and nothing else.** Those are the lines whose toggle rate a peripheral sets (D4) and whose load extraction can bound under D3:
  - **MISO** is out of scope. Its load is the MCU's own input, which has no cited maximum (D3: backlog).
  - **A chip select** is a software-driven GPIO (D4).

  A declaration on either would be a value nothing could confirm, which is why the declared set equals the confirmable set. Solve refuses a missing SCK or MOSI declaration by name (`MissingSpiConstraint`), beside `MissingBusConstraint`.
- **F-b · `max_rise_time_nanoseconds` on an SPI line.** `NetConstraint` requires the key, and this work order does not change the struct.
  - **Its consumer is the Fmax check:** Table 149 note 3's `(tr + tf) ≤ 2/3 T`.
  - The check reads the table's tr/tf maximum at the chosen speed, row and band. It refuses a line whose declared rise time is below that maximum, because the pin cannot be shown to meet the declaration.
  - So the value is read, never defaulted, and never left silently unread.
- **F-c · Where the check's verdict lives.** `aethelgard check speeds --spec F` prints one verdict per specification:
  - every line the board drives or carries, with its `OSPEEDR`;
  - for an in-scope line: the rate, VDD minimum, band, declared C, row, Fmax and tr/tf, with `pass` or a refusal;
  - for an out-of-scope line: its reason (I2C open-drain, DS12117 §6.3.32; MISO an input in master mode; a chip select software-driven, D4).

  The verdict is a pure function of the topology (`RUL-16`), so it reaches the board-less ICM-20948 fixture, which no `emit`, `generate`, `extract` or `verify` can. **The capacitance label** is `declared, confirmed by extract on the routed board` when the specification's design can be drawn. Otherwise it is `declared, UNVERIFIED: no board (<emit's refusal>)`. This is a new CLI subcommand, which is outside the fence's letter ("the CLI's tests and fixtures"); the ruling's "named … in the verdict" needs a verdict that every board reaches.
- **F-d · D1's fixture fix: raise the declared bound, not fewer devices per bus.**
  - `D1` is the 1302 reviewer's 24-device routing board (`architecture.md` §2.21, roadmap). The STM32H753 has four I2C controllers, so fewer devices per bus means fewer devices, which ends what `D1` is for, and 1307's three tests read it.
  - Its eight I2C lines are declared at **150,000 fF**, up from 100,000. Its pull-ups meet that under `1203`'s static rule at its declared 400 kHz:
    - the 2,200 Ω pull-up is above the 967 Ω floor (unchanged);
    - 150 pF is within UM10204's 400 pF Cb;
    - 0.8473 · 2.2 kΩ · 150 pF = 279.6 ns, within its own declared 300 ns rise.
  - **Consequence:** only `d1-24-i2c.json` changes; no file is committed from it.
- **F-e · Versions.**
  - The specification moves 2 → 3, for the required SPI declarations and the part rename (once).
  - The solved topology moves 3 → 4, for the SPI lines it now carries.
  - `parts.json` moves 1 → 2, for `pin_capacitance` and the ordering code.

  `4300a` takes the specification's 4.

## Predictions — written before any measured run of the build

| # | Question | Prediction | Basis |
|---|---|---|---|
| B1 | The reference's SPI bounds under D3 (a): copper plus the flash's 8 pF, the MCU's pin uncounted | `SPI_SCK` 18.5 pF, `SPI_MOSI` 17.2 pF, each within 1 fF of the probe's column B | measured by the probe; the convention is the same |
| B2 | Declared C for SCK and MOSI | 30 pF on the reference, the 4-layer fixture, `D1`, `spi-only*` and the ICM fixture. 50 pF on `D3` (three flashes per bus) and on the solve fixtures | inferred: 30 pF covers B1 with about 60 % headroom |
| B3 | Speeds chosen | Reference, 4-layer, `D1`: SCK and MOSI `01` (20 MHz at the 30 pF row, 2.7–3.6 V: `00` gives 12 MHz, `01` 80 MHz); MISO `00` (was `10`); chip select and I2C `00` (unchanged). **ICM fixture at 7 MHz: SCK and MOSI `00`** (12 MHz ≥ 7 MHz) | Table 149 [read] |
| B4 | VDD band | Every fixture is 3300 mV at 50 ‰, so 3135 mV minimum and the 2.7–3.6 V band. 2700 mV exactly takes the 1.62–2.7 V band (the table prints it in both; the lower is the conservative one) | read |
| B5 | `D1` under extract after F-d | Passes: 8 I2C lines at 97.8–119.6 pF, under 150; `SCK0` and `MOSI0` at 17.5–22.8 pF, under 30 | measured by the probe |
| B6 | The reference's committed files | Change only in: the specification's hash (every file that names it); `OSPEEDR` for SCK and MOSI `10` → `01` and MISO `10` → `00` in `board.rs` and `proofs.rs`; and anything that names the flash's part (`W25Q64JV` → `W25Q64JVxIQ`), if any does. The `.repl` changes only if it names the spec hash or the part | inferred |
| B7 | Tier-2 time added | None. Every new test is tier 1, so `TIER2_MIN` stays 96 | inferred |
| B8 | Verdicts against `main`'s binaries | Every campaign, replay and simulate verdict is unchanged apart from hashes. `OSPEEDR` has no effect in Renode 1.17.0 (the twin models no edge rate) | inferred; cannot be measured until the container question below is ruled |
| B9 | Note 2 (PC13, PC14, PC15 and PI8 at 2 MHz) | Never reached on the LQFP144: PI8 is absent, and PC13–15 carry no SPI signal. It is held as data, and the check applies it to an in-scope pin | read: `STM32H753ZITx.xml` |

## Container caps: the kickoff's premise does not hold on Furnace [measured]

The kickoff runs every KiCad, Renode and tier-2 step under `--memory X --memory-swap X --pids-limit -1`, stating that podman then sets `memory.swap.max` to 0 itself; it says to stop if it does not, and never to write a cgroup file. On Furnace (podman 5.7.0, crun 1.21), read back from each container's own cgroup:

| Flags | `memory.max` | `memory.swap.max` |
|---|---|---|
| `--memory 8g --memory-swap 8g` (four runs, with and without mounts and `--network none`) | 8589934592 | **8589934592** |
| `--memory 8g` | 8589934592 | **17179869184** |
| `--memory 8g --memory-swap 8g --cgroup-conf memory.swap.max=0` | 8589934592 | **8589934592** |

- The 8 GB reading held at every one-second sample over 4 s.
- **The first container was the baseline build.** My runner's guard killed it before its command started, so nothing ran in it. The other five were probes that ran only `sleep`, started after the first reading, to find whether the reading was a race. It was not.
- **4210 measured the same** (its F10, its D8 on PR #131), and `~/.cache/aet-bus` records it. 1307's zero readings came from containers started before its scripts were rewritten at 02:16 (file mtimes, as 4210 read them; inferred, not measured here).
- **The global contract and the kickoff disagree.** The contract's own recipe is to write `0` into the container's `memory.swap.max` after start (Aethelgard F11). The kickoff forbids that write. I have not attempted it.
- **What this blocks:**
  - the baseline against `main`'s binaries (B8);
  - the reference ELF and its re-witnessed identity;
  - any local tier-2 run.

  The host build, the tier-1 tests and the three gates proceed under a capped user scope. CI on GitHub runs tier 2.
- **Without a ruling** that gives a compliant container, this work order ends `STOPPED`: §2's "every verdict is unchanged … against `main`'s binaries" cannot be shown on Furnace.

## Built (commits `501a070` … `2de70f6`)

| Work order item | What was built | Where |
|---|---|---|
| §1.1 Fmax table | **Its home: a pinned data file in `aethelgard-ir`, `data/output-speeds.json`**, with one strict parser, `speeds.rs`, on `parts.json`'s precedent. It is a part fact (the MCU's), read by the generator, the CLI and the tests, so it lives with the other shared data (`INV-01`). It holds 24 rows (4 speeds × 2 bands × 3 loads), each Fmax and tr/tf, and both citations: Table 149 (rev V) and Table 52 (rev Y), which agree in all 48 cells (`~/.cache/aet-4300c/fmax/extract.py` asserts it on the text extraction; the values were checked against the rendered pages 245–246). Rows `10`/`11` carry note 5 and are never selectable. The file also holds note 2's VBAT pins and D1's reason why I2C has no row. The parser refuses by name: a malformed row, a missing citation, a missing or repeated row, note 5 on the wrong speed, and a heavier load given a higher Fmax or a faster edge. | `crates/aethelgard-ir/data/output-speeds.json`, `src/speeds.rs`, `tests/speeds.rs` |
| §1.2 + §1.4 the check and the choice | `OutputSpeeds::choose` picks the slowest of `00`/`01` whose Fmax ≥ the rate and whose tr/tf ≤ the declared rise time (F-b). The band comes from VDD minimum (2.7 V exactly takes the lower band); C is rounded up to the next row. It refuses above 50 pF, outside 1.62–3.6 V, or when only `10`/`11` would serve; nothing is sped up. The generator computes every pin's speed in `Plan::new` (`pin_speeds`): SCK and MOSI from `spi_lines`; I2C, MISO and GPIO outputs at `00`, each with its named reason (D1, D3 (ii), D4). | `svd-hal/src/board.rs` |
| D2 declared C, schema | Specification v3: SCK and MOSI each need a `net_constraint` (`MissingSpiConstraint` in `solve`, beside `MissingBusConstraint`). Solved topology v4: `spi_lines` holds what the check reads (bus, signal, rate, C, rise time, VDD minimum). Its `validate` refuses a forged or missing line (`SpiLineMismatch`) and re-checks every part limit from what it carries (§4.2). | `ir/src/lib.rs`, `solve.rs` |
| D2/D3 extraction | `extract::lines` adds each SPI bus's SCK and MOSI after the I2C lines, which are unchanged. On SCK/MOSI each device pad counts at its part's cited maximum; the MCU pad (the driver) is copper only and is listed as `drivers`; a part with no cited maximum is `UnboundedPinCapacitance`. The CLI judges SPI lines against `spi_lines` and states the SPI pin convention as one extra condition, so `conditions()` and FAB-NOTES are unchanged. | `eda/src/extract.rs`, `cli/src/extract.rs` |
| D3 `pin_capacitance` | Parts table v2: a required `pin_capacitance.max_femtofarads` (null when no maximum is printed) with its own citation. W25Q64JV 8000 fF (§9.4 Cout 8 pF; CIN 6; sampled at 25 °C and 3.0 V, stated). ICM-20948, TMP117 and BME280 are null, each citing why. The W25Q64JV's §9.5 TR/TF 5 ns is recorded as a measurement condition, as ruled. | `ir/data/parts.json`, `parts.rs` |
| §1.5 limits at `validate` | Step 14, each refusal citing the datasheet: `SpiClockAbovePart`, `I2cAddressNotOffered`, `PartOnWrongBus`, and `AmbiguousPartName` for the bare `W25Q64JV`. **F20:** a `SPI.GenericSpiFlash` part's identity must be write `0x9F`, read 3, capacity `0x10`–`0x19` (the twin's own shape). | `ir/src/lib.rs`, `parts.rs` |
| §1.6 F21 | `spi1`, `spi2`, `spi3`, `spi5` and `spi6` join `RESERVED_REFERENCES` (`spi4` was already there from the stock chain). The twin's exact test now derives the emitter's controller names too. | `ir/src/lib.rs`, `twin/tests/reserved_names.rs` |
| §1.7 F11 | `W25Q64JVxIQ` in the table, every fixture, eda's map and the examples. The bare name is refused by name. The schematic `Value` is now the full ordering code, `W25Q64JVSSIQ` (SS is the drawn SOIC-8 208-mil footprint; F-g below). | `parts.json`, `eda/src/schematic.rs` |
| Verdict surface (F-c) | `aethelgard check speeds --spec`, also an MCP tool (`check_speeds`, since tools derive from the CLI tree). | `cli/src/speeds.rs` |
| D1 fixture (F-d) | Raised to 150,000 fF on its eight I2C lines; SCK0/MOSI0 declared. A tier-1 test runs `extract` on D1 at 4 layers and needs every line to pass; it is killed by the old 100,000 fF (7 findings, exit 1) [measured]. | `fixtures/d1-24-i2c.json`, `stackups_tier1.rs` |
| Reduction (ruled) | `spike/4300c/` deleted; the reproduce line points at `139226e`. | — |

### Further forks taken during the build

- **F-f · A part absent from the table is not refused at `validate`.** That is the behaviour before this work order: `emit` refuses an unmodelled part (`UnknownPart`), and the solve fixtures use synthetic parts. Such a part has no limit to enforce, so the check skips it rather than inventing one.
- **F-g · The flash's schematic `Value` became `W25Q64JVSSIQ`.** `W25Q64JVSS` on a BOM can be bought as an IM part, whose JEDEC identity (`EF 70 17`) the firmware would fail, which is exactly F11's hazard. This changes the BOM, schematic and board `Value`, and so the fabrication package's hashes. Nothing of it is committed.
- **F-h · Extraction reads each device's part from the solved topology** (`solved.spi_devices`), not the specification, so `solved_devices.rs`'s RUL-16 pin (schematic.rs is the one eda reader of the declared device lists) still holds unchanged.

### Results, each beside its prediction

| # | Prediction | Result | Label |
|---|---|---|---|
| B1 | Reference SCK 18.5, MOSI 17.2 pF (±1 fF of the probe's column B) | **SCK 18,521 fF** (trace 5920, vias 3038, pads 1563, pins 8000), **MOSI 17,188 fF**. 4-layer: 18,470 / 17,114. D1: 19,183 / 17,486 | measured |
| B2 | Declared 30 pF (50 on D3) | As declared. Every bound is under 30 pF, so extraction confirms all six SCK/MOSI declarations on the three boards that exist | measured |
| B3 | SCK/MOSI `01`, MISO `00`, CS and I2C `00`; ICM fixture `00` | As predicted on all four boards (`check speeds`, `speeds_tier1.rs`) | measured |
| B4 | 3135 mV, upper band; 2.7 V takes the lower | As predicted (`tests/speeds.rs`) | measured |
| B5 | D1 passes extract after F-d | 8 I2C lines 97,767–119,603 fF under 150,000; SCK0/MOSI0 under 30,000 | measured |
| B6 | Reference files change only in the spec hash, the SPI speed fields and the part name | As predicted. The 8 changed files under `boards/sentinel-01/` differ only in the spec-hash line, three `OSPEEDR` writes (PA5 and PA7 `10`→`01`, PA6 `10`→`00`), their three oracle lines and one combined mask, and the doc comment `a W25Q64JV` → `a W25Q64JVxIQ`. The `.repl` changes only in its hash line | measured (`git show e60cd48`) |
| B7 | No tier-2 time; `TIER2_MIN` stays 96 | No tier-2 test was added; `TIER2_MIN` stays 96 | read |
| B8 | Verdicts unchanged against `main`'s binaries | **Not measured: the container question** | — |
| B9 | Note 2 never reached on the LQFP144 | As predicted; held as data and tested (`a_vbat_pin_is_held_to_2_mhz`) | read, measured |

### Refusals shadowed by an earlier one (reduction candidates outside this fence)

§1.5 and F21 now refuse earlier what three later checks refused before:
- eda's `UnsupportedAddress`, for a part in the table;
- the twin's `ModelOffItsBus`;
- the twin's `InvalidDeviceName` for `spi1`–`spi6`.

The tests that asserted them now assert the earlier refusal, each with a comment saying why: `emit_tier1`, `extract_tier1`, `twin platform::tests` and `spi_platform`. The later checks stay as the emitter's own guard. Deleting them is outside this fence (eda beyond the line list, and the twin), so it is a **backlog row**.

### Backlog rows (from the ruling and the build)

- **Vendor the ICM-20948 symbol**, so its fixture has a board and its declared C can be confirmed (ruled D2).
- **The MCU's input capacitance as a maximum**, to judge a device's drive into MISO (ruled D3).
- The three shadowed refusals above.

## Adversarial pass (full; a fresh Opus subagent, on a frozen copy at `2de70f6`)

Run on the host only; no container. The report's text is saved at `~/.cache/aet-4300c/adv-work/REPORT.md`; the subagent's own write of it was refused, and it did not work around that. Its attack inputs are in `~/.cache/aet-4300c/adv-work/`. Each finding was read or re-run by the builder before it was adopted.

| # | Finding | Severity | Label | Disposition |
|---|---|---|---|---|
| 1 | A hand-edited topology could relabel one of a peripheral's lines (`spi_lines.bus`), giving SPI1 two rates. Both the speed choice and the part's clock re-check read the forged one: SCK was emitted at `01` judged at 110 MHz, while the flash's 104 MHz limit was checked against a 1 MHz "GHOST" bus. Library API only. | major | measured | **Fixed.** `validate_spi_lines` now holds one label and one rate per peripheral as well as per label (`SpiLineMismatch`). Test: `a_peripheral_s_lines_share_one_label_and_one_rate`. |
| 2 | A part's `supply_millivolts` had no consumer. 2800 mV ±5 % (2660 mV, under the flash's 2.7 V) and 3600 mV ±5 % (3780 mV) passed everything, and the Fmax check never checked Table 149's 3.6 V top. Pre-existing on `main`. | major / scope | measured | **Fixed, as fork F-i.** The work order's box promises "every part's declared limits are enforced", and supply is one. `validate` step 14 refuses a device whose domain range (nominal ± tolerance; minimum rounded down, maximum up) leaves its part's range (`SupplyOutsidePart`, citing the datasheet). `solve` refuses an SPI bus whose MCU supply leaves 1620–3600 mV (`SupplyOutsideSpeedTable`). Tests: `a_supply_outside_a_part_s_range_is_refused`, `a_supply_above_the_table_s_bands_is_refused`. |
| 3 | Near-miss part names (`W25Q64JVXIQ`, `w25q64jv`) skipped every limit. | minor | measured | **Fixed.** The bare name is matched in any case. A table part named in another case is `PartNameCase`. A part absent from the table is still not refused (F-f); `emit` and `generate` refuse it as `UnknownPart`. |
| 4 | `check speeds` said "extract bounds it" though it computes no bound. Two flashes declared at 30 pF passed `check speeds` while `extract` on their board fails (36,305 fF). A specification with no SPI bus printed `pass` with nothing judged. | minor | measured | **Fixed.** The label now says the value is not bounded here and names `extract` and `verify` as what bounds it. A new `judged` count is 0 when nothing is judged. `verify`'s extract gate is what catches the two-flash case. |
| 5 | Constraints on MISO and chip selects were accepted and never read. | minor | measured | **Fixed.** Refused by name (`UnjudgedSpiConstraint`), so the declared set is the judged set (F-a). |
| 6 | W25Q64JV §9.6's minimum clock slew (0.1 V/ns) is not held as data. | nit | read | **Named, not built.** Every Table 149 row in the 2.7–3.6 V band meets it (13.3 and 16.6 ns at worst, about 0.2 V/ns). A supply under 2.7 V is now refused for the flash (F-i). |
| 7 | eda's `UnsupportedAddress`, the twin's `ModelOffItsBus`, and `spi1`–`spi6` as the emitter's `InvalidDeviceName` are pre-empted by the new earlier refusals. | nit | measured, inferred | **Backlog row** (outside the fence), above. |
| 8 | Stale wording: `extract --help` said I2C only; `check --help` said "KiCad"; `AmbiguousPartName` said "for the IQ and JQ codes"; fR (50 MHz for Read Data `03h`) is not cited; F20 leaves the manufacturer byte free. | nit | read | The help text and the message are **fixed**. fR does not apply, because the firmware sends only `9Fh`. The manufacturer byte is any JEDEC code, so it is left free. |

**What held** [measured or read by the adversary]:
- all 24 rows against Tables 149 and 52, cell by cell, and against the page image;
- rounding at 10,000, 10,001, 30,001 and 50,001 fF;
- the VDD minimum and the band edge (2843 and 2844 mV at 50 ‰);
- the rise-time refusal, and rows `10`/`11` never selected;
- every part limit at `validate`, and the topology re-check under one label;
- specification v2 and v4 refused, and `MissingSpiConstraint`;
- extraction's pin counting (MCU excluded; two flashes give 16 pF);
- the reference's I2C line objects JSON-identical to `main`'s binary;
- D1's 150 pF honest, and its test killing 100 pF;
- the regenerated reference byte-identical, and against `main` only the named changes;
- note 2 unreachable, and the pin capacitances at source;
- no weakened assertion in 19 test diffs, and `ci/` unchanged.

**F-i**, taken in answer to adversarial finding 2: each device's supply range lies within its part's, and an SPI bus's MCU supply within Table 149's 1620–3600 mV. It is in the work order's box ("every part's declared limits"), though not in §1.5's list. The architect may narrow it.

## Mutation (diff-sized: one mutant per guard this work order adds)

There are 28 mutants: the table's choice and parser (7), the parts parser (2), `validate`'s part limits and constraints (8), the topology re-checks (3), `solve` (2), the generator (2), extraction (2), F21, and the `judged` count. `~/.cache/aet-4300c/mut/mutate.py` holds them.
- **Each is killed by a test** [measured].
- Mutant 25, the first version of "eda: typical promoted", was killed only by the compiler. It was rewritten as a behaviour mutant, and a test then killed it.
- After the reduction, mutants 9, 10, 11 and 17 were moved onto the shared helpers and re-killed.

## Reduction pass

Reduction pass (diff): −10 net · 3 duplicates removed · 0 dead paths removed · 2 protected kept · 3 findings below. It was run by a fresh subagent.
- **The duplicates:** the I2C address and wrong-bus check, and the SPI clock check, are each one helper (`check_i2c_address`, `check_spi_clock`), shared by `validate` and the topology re-check; extraction's pad lookup is one `pads_on`.
- **Protected:** the two `belt-and-suspenders:` `UnmodelledPart` refusals in `extract::lines`.
- **Findings:**
  - the `Role` insert repeated in extraction's two loops (folding it only moves it);
  - the twin's `ModelOffItsBus` and eda's `UnsupportedAddress`, outside the fence (backlog, above);
  - the ruled deletion of `spike/4300c/`.
