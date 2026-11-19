# AET-WO-3304 — FEEDBACK (bench preparation; STOPPED at §2)

**Branch and inputs.**
- Branch `wo/aet-3304-bench-sensors`, from `main` at `c3f8ddbb`.
- The work order is read from `~/.cache/aet-3304-wo.md`, the architect's copy.
- The bench sheet is `AET-WO-3304.bench.md`. Every prediction is there, written before any bench; this file says where each came from and what the twin cannot say.
- **No crate, model, golden, test, workflow, `ci/` or `container/` file changed.** The diff is the sheet, this file, and one specification, `examples/sentinel-02-bench.json`.

Each statement is labelled **measured** (run on Furnace in the pinned image), **read at source**, or **inferred**.

## 0. Status: STOPPED at §2, as the work order ends

Nothing was run on silicon; the bench is Scotty's. The sheet is ready to take to t9o777.

## 1. The firmware: a bench specification, not Sentinel-02's ELF

### 1.1 Sentinel-02 as solved cannot reach the ICM on a default Nucleo

`aethelgard solve` on `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` (**measured**) gives the pins below. Their on-board use is **read at source** in UM2407 Rev 6 (sha256 `b493e14f…a4cd`):
- the default bridge states come from the manual's bold rows (Tables 10 and 14, pp. 29–33);
- an independent reader, given only the manual and the pin list, agreed on every pin.

| Net | Pin | Header | On the Nucleo by default | Kind | Reversible? |
|---|---|---|---|---|---|
| I2C_SCL / I2C_SDA | PB6 / PB7 | CN10 14 / 16 | not on the VCP (SB9, SB18, SB34, SB66 OFF) | none | — |
| IMU_SCK | PB13 | CN7 5 (only with JP6 OFF), CN12 30 | Ethernet RMII TXD1 through **JP6 ON** (the least certain default: the bold row names it mainly as the OFF case) | load [inferred from the signal name] (the PHY's input) | jumper |
| IMU_MISO | PB14 | CN12 28 only | **LD3**, the red user LED, with no bridge named | load on MISO | **no bridge** |
| IMU_MOSI | PB15 | CN7 3, CN12 26 | nothing | none | — |
| **IMU_CS** | **PA1** | CN11 30 only | **the PHY's 50 MHz RMII reference clock**, through SB57 ON | **driver** | solder bridge SB57 |
| SPI_SCK / MISO | PA5 / PA6 | CN7 10 / 12 | nothing (LD1 is on PB0) | none | — |
| **SPI_MOSI** (flash) | **PA7** | CN12 15 (D11 is PB5) | **the PHY's RMII CRS_DV**, through SB31 ON | [inferred from the signal name: the manual gives no direction] **driver**, fought by MOSI | solder bridge SB31 |
| FLASH_CS | PA0 | CN10 29 | nothing (B1 is on PC13) | none | — |
| CONSOLE TX / RX | PA9 / PA10 | CN12 21 / 33 | USB OTG_FS VBUS and ID, through SB23 and SB24 ON | load; VBUS is 5 V when a cable is in CN13 | solder bridges |

So Sentinel-02's ELF reaches both sensors only after:
- opening SB57: the ICM's chip select is otherwise a 50 MHz clock;
- lifting JP6;
- accepting LD3 on MISO.

The same ELF also clocks its flash bus into the PHY's CRS_DV output on PA7 (SB31), now that `4230` made SPI1 run. That is contention on the bench's only development board, for a part the bench does not have.

**The deciding reason is the predictions.** Sentinel-02's firmware also reads U2 (BME280), U3 (TMP117) and U4 (W25Q64), none of which is on the bench:
- U4's absence the twin can render (`spi_device_absent`).
- **An absent I2C device beside a present one, it cannot** (**read at source**, `aethelgard-chaos` `campaign.rs`). Its only I2C absence is the whole bus (`analog_rise_exceeds_ceiling` or `line_held_low`, rendered as the bus's devices absent, `AET-WO-2101` S6).

So every U2 and U3 row, and the bus state U7's read inherits after two NACKs, would be a row the twin cannot predict. That is §5's "prediction that could not fail" in another form.

### 1.2 The bench specification, and why `examples/`

`examples/sentinel-02-bench.json` is Sentinel-02's own U7 (BMP581) and U5 (ICM-42688-P) rows, unchanged: part, address `0x46`, declared pin capacitance and orientation. They are pinned to header pins with nothing on them:

| Change from `sentinel-02-own.json` | Why |
|---|---|
| I2C1 pinned on PB6/PB7 | `3302` measured this line's capacitance (55 pF, the figure §1.2 of the work order asks for) and decoded the generated firmware's traffic on these header pins |
| The IMU bus on SPI1: SCK PA5, MISO PA6, MOSI PB5, CS PD14 | The Arduino SPI header (D13, D12, D11, D10), with nothing on any pin by default (D11 is PB5: SB33 OFF, SB35 ON) |
| U2, U3, U4, the FLASH bus, its constraints, the console port, the 5 V and 1V8 domains removed | Not on the bench (§1.1). With no serial port, the firmware has no echo waits |
| `mcu.bor_level` 0 | This Nucleo's measured `BOR_LEV` (`3302` B7). The twin presents the declared level as record word 1, so the sheet's brown-out row is the board's, not Sentinel-02's 3 |
| `accept_lifecycle` emptied | It named only the removed flash |
| `project_name` `sentinel-02-bench` | — |

**Why `examples/` and not `boards/`:**
- `boards/` holds generated board crates, enumerated by `workspace_shape.rs`. Committing one is generated code that a test guards, which is outside the fence.
- `examples/` holds specifications. Four tests already walk it and hold every specification there to their rules:
  - `silicon_rules.rs` (every solved controller under the independent rules);
  - `ledger_tier1.rs` (every specification that solves has a ledger);
  - `hwdef_tier1.rs` (the board crate's pin speeds equal the stored ones);
  - `silicon_words.rs`.
- So the new file is solved and judged like any other board, and no test names it. Nothing in the engine names it either.

**The solve** (**measured**):

| Net | Pin and function |
|---|---|
| I2C_SCL, I2C_SDA | PB6, PB7: I2C1, AF4 |
| IMU_SCK, IMU_MISO, IMU_MOSI | PA5, PA6, PB5: SPI1, AF5 |
| IMU_CS | PD14, GPIO |

- **SPI1:** kernel `per_ck` (HSI, 64 MHz), `MBR` 4, so **2.000 MHz**, at most 2.030 MHz at HSI's fastest. That is under the work order's 4 MHz for the analyzer: 6 samples per half-period at 24 MSa/s.
- **I2C1:** `TIMINGR` `0x10C7123C` at 400 kHz.

## 2. The ELF and every prediction's source run

**The ELF:** `sentinel-02-bench.elf`, sha256 `375fd31a28f4442b796110f0df82d31a9e133ce4a12f2a982fbb923cfe369c02` (**measured**).
- Built by `main`'s CLI (`cargo build --locked -p aethelgard-cli` in the image at `c3f8ddbb`), with `generate --spec` then `build --crate`.
- Built twice, at `/cache/runs/gen1` and `/cache/runs/deep/path/gen2`, giving one sha.
- The image is `aethelgard-tools@sha256:e5fc7ddb…1b58`, run under `capped-run` and `--network none`.
- Kept at `~/.cache/aet-3304/bench/` with `SHA256SUMS`.

| Run | What | Result (**measured**) | Feeds |
|---|---|---|---|
| S1 | `simulate --firmware … --for 100ms` on the emitted `.repl` (`051883e2…`) | `pass`. Boot word at 39,039,070 ns virtual; 22 masked register rows equal; U7 identity `0x50`, configure, data; U5 identity `0x47`, configure 1–3, data; every model witness line present; peak RSS 587 MiB | B2's register image; B5's records, MOSI frames and the transaction order |
| S2 | `campaign`, 7 scenarios (`~/.cache/aet-3304/bench/scen/`) | below; the rise of each is in the sheet's B3 | B2's nothing-wired records; B3's rise table |
| S3 | `verify` with three of S2's scenarios | §3 | INV-03's status of the ELF |

**S2, scenario by scenario.**
- Every `expect` was written before the run, from 0.8473·R·C.
- **Five matched.** Two matched in verdict but not in findings: `sda-held-low` also raised `i2c_timeout`, and `spi-absent-u5` also raised `spi_data_mismatch`. My expected finding lists were incomplete there; the verdicts were right.

| Scenario | Verdict | U7 records | U5 records |
|---|---|---|---|
| `pullup-2200-60pf` | pass | OK `0x50`, OK, OK | OK `0x47`, OK ×3, OK |
| `pullup-3300-60pf` | pass | as above | as above |
| `pullup-4700-60pf` | pass | as above | as above |
| `pullup-10000-60pf` | fail: `analog_rise_exceeds_ceiling`, `i2c_timeout` | TXIS ×3 | as above |
| `unwired-47k` (100 pF) | fail, as above | TXIS ×3 | as above |
| `sda-held-low` | fail: `i2c_line_held_low`, `i2c_timeout` | TXIS ×3 | as above |
| `spi-absent-u5` | fail: `spi_identity_mismatch`, `spi_transaction_unwitnessed`, `spi_data_mismatch` | as 2.2 kΩ | OK with `0xff` (the mux's dummy byte) |

**The decoder lines** are **read at source**, not run:
- libsigrokdecode 0.5.3's `i2c/pd.py` (sha256 `bd28a545…`) and `spi/pd.py` (`57d988e8…`);
- `sigrok-cli` 0.7.2's `decode.c`;
- libsigrokdecode's `instance.c`, for the instance names `i2c-1` and `spi-1`.

No `sigrok-cli` runs on Furnace, so the bench's first row checks the installed decoders' hashes.

## 3. `verify` on the bench specification (S3)

**Measured:** **`verify` passes on this exact pair**, so `INV-03` holds for the bench ELF.
- **Run:** `verify --spec examples/sentinel-02-bench.json --scenarios baseline-2k2 pullup-2200-60pf unwired-47k`, in the image under a 16 GiB cap. 479 s, 9.1 GiB peak.
- **The verdict is `pass`,** with `elf_sha256` `375fd31a…9c02` (the bench ELF) and `topology_sha256` `c370e89f…`. It is at `~/.cache/aet-3304/bench/verify-verdict.json`, sha256 `fef8a33c…6b99`.
- **Every gate passes:** ERC, DRC, support, extract, the six Kani harnesses (`every_valid_sequence`, the I2C1 and SPI1 transfers, the SPI1 frame refusal, `reset_clock`, `flash_option_status`), build, simulate, the campaign, and `attribute` on `unwired-47k`.

**The ledger:** 268 rows.

| Class | Rows |
|---|---|
| independent-rule | 65 |
| observed | 53 |
| same-source | 85 |
| ignored-by-model | 64 |
| silicon-only | 1 |

- 150 rows are unacknowledged (the specification's `acknowledge_unwitnessed` is `[]`, as Sentinel-02's own is). So no fabrication package can be written from it, and none is wanted.
- The sheet's sources follow these classes.

**Two earlier runs, for the record.**
- **The first was OOM-killed at an 8 GiB cap** after 343 s, with no output. F3 in §7.
- **The second failed only its campaign gate.** It used a 10 kΩ-at-60 pF scenario, which seeds two overrides and so counts for nothing under `3303` D1 (F4). The fourth scenario of that run, `baseline-2k2`, was added there because verify credits a detected fault only beside a passing no-fault baseline (`verify.rs`).

## 4. What the twin cannot predict, said plainly

- **The repeated start.** Renode's I2C model is transaction-level, and its witness `read 0x01 -> 0x50` is the same for Sr and for P then S. The sheet's `Start repeat` lines are the generator's word, by RM0433's `START` after `TC`. The bench is the first judge of it.
- **Every data byte.** The twin's BMP581 and ICM data are their stimuli (25 °C, 101,325 Pa; 29 °C, 0 g, +1 g). The sheet predicts the framing and the status only.
- **The ICM's MISO byte during a command or write byte.** DS-000347 does not state it, and the twin does not witness it.
- **A floating MISO with no ICM wired.** The twin's `0xff` is Renode's mux convention.
- **The I2C bus with nothing wired.** The twin's absence is a model rendering (TXIS), not a waveform. `3302` measured TXIS on this board with this driver code, so the row is a repeat, labelled so.
- **Any time on silicon.**
  - The twin is instruction-level at 100 MIPS.
  - The SCL period's lower bound is the independent rule's (`silicon.rs`), and the SPI clock band is the solver's arithmetic.
  - The 20 ms wait is the generator's lower bound only. Silicon's cycles per spin, with flash wait states and no I-cache at reset, are not modelled; the wait may be several times 20 ms [inferred].
  - No gap between transactions is pinned.
- **`4400g`.** This firmware has no timer and no USART, so no prediction moves when it merges. The emitted `.repl`'s hash (S1's input) may change; the ELF and every row do not.
- **`4220c` B1–B4** are FIFO questions. This firmware reads the sense registers (`0x1D` to `0x22`) and never the FIFO, so no row depends on them. The data's byte order depends on `SENSOR_DATA_ENDIAN`, which DS-000347 documents (p. 74–75).
- **The rise at the bench.** It assumes the BMP581 breakout's own pull-ups form an RC line with 60 pF. The BMP581's pins (≤ 10 pF declared) and any level shifter are not in it; the sheet says a shifted bus is not comparable.

## 5. Adversarial pass (targeted, §5)

The pass ran in three parts:
- an independent reader checked the pins against UM2407 alone;
- an independent reviewer checked the finished sheet against the evidence and the decoder sources, through §5's three lenses;
- I made a pass of my own.

Every finding of the reviewer's that I adopted was checked at source first. **Fixed in the sheet:**

| # | Lens | Finding | Fix |
|---|---|---|---|
| A1 | decoder | **The capture's trigger would have broken the first transaction.** `--wait-trigger` stores no sample before the trigger (`sigrok-cli` `session.c`, **read at source**), and the decoder takes sample 0 as the previous pin state (libsigrokdecode `instance.c`). So a capture triggered on SDA's first fall cannot see the first START, and the identity read's `Start repeat` prints as `Start`: a false disagreement | Untriggered 4 s capture, with the reset within about a second; the bus idles high at sample 0 |
| A2 | decoder (mine) | **sigrok's SPI decoder samples on the rising edge in mode 0 and mode 3 alike.** A bus in mode 3 decodes to the same bytes, and the SPI lines would call it agreement | The sheet says so; the mode is judged by the SCK-idle row and B2's `CFG2` row |
| A3 | could not fail | The GPIO `MODER` and `AFRL` rows were tagged "twin". The ledger classes every such field same-source: the twin reads back what was written | Retagged **generator**; AHB4ENR split (GPIOA and B observed, GPIOD same-source) |
| A4 | could not fail | The unwritten registers (AHB3/1/2, APB3, APB1H, APB4, SysTick, ISER0–4) expect Renode's reset value, not a cited document | Tagged **twin (Renode reset)**: a difference there is with the model's reset value, not the firmware |
| A5 | could not fail | `TIMINGR`, and `CR2`/`TXDR` after the transfer, are the words the firmware wrote. Only a read-back can fail | Tagged **generator, read back**. `TIMINGR`'s rule holds the fields by inequalities, not the word |
| A6 | could not fail | The SCL row's ≥ 2.5247 µs floor cannot realistically fail | Added an expected band, 2.65–2.90 µs at R near 2 kΩ, derived from `3302`'s measured excess and labelled **repeat-derived**. The floor stays as the rule's |
| A7 | could not fail | The rise rows gave picoseconds with no band, and "C implied" restates the bench | ±15 % (`3302`'s C scatter plus the scope's resolution). The falsifiable claim is the side of 300 ns: crossover 5.90 kΩ at 60 pF, 4.7 kΩ at 20 % margin |
| A8 | could not fail | The 10 kΩ row pre-explained both outcomes | It now commits to the twin's TXIS; a `0x50` is a disagreement with its size |
| A9 | disclosure | The sheet cited S2 without saying that two scenarios matched in verdict only | One line in the sheet |
| A10 | header pin | Table 11's D0 route also names SB12, the ST-LINK TX's bridge to PD9 (ON). The manual does not exclude a driver on CN10 pin 16 | A meter step before power: no low resistance from CN10 pin 14 or 16 to GND or 3V3 |
| A11 | markdown | A blank line had split B6's table | Joined |

**Checked and right** (the reviewer recomputed each):
- every decoder string and its order: `Write` before `Address write: 46`; `Start repeat` only with no STOP since the last START; the `i2c-1:` and `spi-1:` instance names;
- the `-A` row and class names;
- the record addresses and word layout against `main.rs`'s `record` and its 20 words;
- the masks and values;
- the settle spins, the SCL floor, the SPI band;
- every file hash.

**Header pins:** no bench pin is driven by anything on the Nucleo. Both independent readers agree.

**Left as is:** the SPI clock band, 1.948–2.030 MHz, is HSI's datasheet tolerance (about 4 %), not a sharper prediction. The sheet labels it the solver's rule.

**A correction to `3302`'s record, not mine to make.** `3302`'s B0 results say PB6 is "routable to the ST-LINK virtual COM port by SB34/SB66". In Table 11, PB6's VCP route is SB9 and SB18; SB34 and SB66 are PB7's. It is a protected record, so it is reported here only.

## 6. Reduction pass

Diff-sized, over the three files.
- **Removed:** the rise column of §2's scenario table in this file, a second copy of the sheet's B3 table (−7 cells of the same numbers). Each figure now has one home, the sheet.
- **Kept:**
  - §1.2's change table: the specification's only explanation; the specification has no comments.
  - Sentinel-02's unchanged rows in the specification (declared pin capacitance, orientation, the IMU nets' constraints): `extract`, the placer and the support gate read them.
- **Nothing labelled belt-and-suspenders.**
- No suite or mutation applies to prose and one specification; the gates in §8 ran with the specification present.

## 7. For the architect

No ruling blocks the bench. These are findings and questions.

**Q1. Which breakouts?** The BMP581's pull-ups, its SDO strap and any level shifter decide B3 and B4's rows. The sheet asks the bench to read them, and says what a fixed `0x47` strap gives (NACK; TXIS). If you know the boards, they can be named before the bench.

**F1. The twin cannot render one absent I2C device beside a present one.** Its only I2C absence is the whole bus (`AET-WO-2101` S6). This is what ruled out Sentinel-02's own ELF here (§1.1). A per-device I2C absence, the I2C twin of `spi_device_absent`, would let a board with parts not yet in hand be benched. Backlog, by the scope test: it advances what the system is for.

**F2. The twin claims no time on the wire.** It claims no SCL period, no SPI clock and no gap between transactions. The sheet's timing rows are the independent rule's bounds and a repeat-derived band. Modelling the H7's clock synchronisation, `3302`'s second scar, is where a twin prediction of the SCL period would come from. Already a roadmap row; noted, not new.

**F3. `verify` on this specification needs more than 8 GiB here.**
- **Measured:** OOM-killed at an 8 GiB cap after 343 s; 8.8 and 9.1 GiB peaks at a 16 GiB cap, 414 to 479 s.
- The Kani proofs are the likely part [inferred].
- `capped-run`'s default of 8G will not hold it. CI's runners have 16 GB.

**F4. `verify`'s campaign gate needs its own scenario set.**
- A scenario that seeds two overrides and expects a fail (here 10 kΩ at 60 pF) is `Other` under `3303` D1, and fails the gate.
- So the sheet's prediction scenarios and the verify set differ. That is by the rule, and §3 runs the set the rule admits.
- A note only; no change asked.

**F5. Sentinel-02 on the Nucleo, if it is ever wanted.** The route is in §1.1's table:
- open SB57 (PA1);
- lift JP6 (PB13);
- accept LD3 on MISO;
- open SB31 so the flash's MOSI does not fight the PHY on PA7.

Then, with U2, U3 and U4 absent, accept rows the twin cannot predict until F1 exists. Not recommended.

## 8. Gates

Run on the host, at `c3f8ddbb` plus this diff (**measured**):

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --no-fail-fast`, with `target/` inside the worktree as CI has it | **2,258 passed, 0 failed** |

**The four tests that walk `examples/` pass with the new specification in it:**

| Test | Passed |
|---|---|
| `silicon_rules` | 1 |
| `ledger_tier1` | 12 |
| `hwdef_tier1` | 5 |
| `silicon_words` | 6 |

**A first run failed once, from my environment.** With `CARGO_TARGET_DIR` outside the worktree, `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named` writes its scenario under that directory and draws `BaseOutsideRoot`. With `target/` in the tree, as CI's is, it passes. The same trap as `3100`'s, and not this diff's.

The tier-2 tests were not run on the host. This diff touches no code and no golden; CI runs them.

**Reduction pass (diff):** −7 cells net · 1 duplicate removed (FEEDBACK §2's rise column) · 0 dead paths · 0 protected kept · 0 findings.
