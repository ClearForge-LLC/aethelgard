# AET-WO-3400 — FEEDBACK

Builder: Claude (strong tier), worktree `~/aethelgard-3400`, branch `wo/aet-3400-bench-board`, cut from `origin/main` `3dcd242`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #183, head `8d84392`) and is not committed here.

Labels used throughout: **[measured]** (run here, output in `spike/3400/results/`), **[read at source]** (a file, a datasheet or a catalogue, cited), **[inferred]** (reasoning, not checked).

## 0. Status: STOPPED at S3 for the architect's and Scotty's ruling

- **Everything Specification 12 can say routes on two layers** [measured]. V0 to V7 each pass `validate`, `generate`, `emit`, `check erc` and `check drc --spec` (exit 0, `findings == []`), plus DRC with schematic parity and `check support`.
  - **The largest variant passing the work order's five commands is V7:** 79 parts, 100.05 × 118.95 mm. It adds the LDO and `main`'s 1.8 V crossing with an ICM-20948 as a density proxy for F2. Its `extract` refuses the translator's pins (`TranslatorPinUnbounded`), as `main`'s own tier-2 test expects.
  - **The largest passing every gate, `extract` included, is V6:** V5 plus the LDO, 71 parts, 100.05 × 107.60 mm.
  - **The recommended board is V5:** 68 parts, the same outline. It has no LDO, because Sentinel-02 has no 1.8 V domain.
  - **Renamings:** every variant routes to its own copper under the 20 renamings, 20/20.
- **Not measured: the loaded board as it would be ordered** (Sentinel-03 under §10.4): V5 or V6 plus E0–E2, `4220b` and the witness's port and USB. Main cannot express those rows, so no variant holds them. V7 (a 0.4 mm QFN on its own bus behind a translator) is the nearest proxy, and it routes at 2L and 4L. **The board recommended for ordering now, (a) = V0, is measured:** it passes every gate at 2L and 4L.
- **Most of the test features cannot be expressed** [read at source, measured by probe]. Test points, jumpers, shunts, a BOOT0 header, a reset switch, input capture, `MCO`, USB, FDCAN and the flight sensors' footprints each need an engine work order (§7).
- **One gap on `main` matters for ordering Sentinel-01** (F1): the emitter does not draw `power.input.hot_plug`, the bulk capacitor that `BatteryInputUndamped` credits. It also gives that capacitor's reference, `C1`, to a 100 nF decoupler. This is a known backlog row (4300d3 D5, roadmap §7.1 L635, "before `4500`"). With Sentinel-01 first, its deadline moves before Sentinel-01's order.
- **Amended (§0, Scotty 22:30 UTC), measured in §10:**
  - Every variant also routes and passes the five commands at four layers, with the same outline, 14–31 % less track and SNVSAN3F item 5 no longer flagged.
  - The witness (W1–W6) needs a capture-port role and USB FS. Both fit V5's free pins (port G, PA11/PA12). ULPI and RMII would need SPI1 and CRSF re-pinned.
  - I2C can only be sniffed by the analyzer path (RM0433 chapter 47 has no monitor mode).
  - The DMA sample rate is not in ST's documents; it has to be measured on the Nucleo.
  - **Path: (a) the lean probe now (US$33 for six at OSH Park, no engine work), then the loaded board as Sentinel-03 at four layers, (c)** (§10.4).
- **Recommendation, short (as amended):**
  - Order the lean probe (V0, `main`'s reference) now.
  - Build the loaded board as **Sentinel-03 at four layers**:
    - V5's content;
    - the witness's capture port and USB FS;
    - E0 (the hot-plug draw), E1 (test points), E2 (BOOT0 and reset);
    - `4220b` unheld, with Sentinel-03 as its consumer before Sentinel-02.
  - J1 (the witness) is the job; J2 is not taken.
  - §7 (written before the amendment) holds the loaded board's tiers; §10.4 holds the path.
- **Gates** (`spike/3400/sources/gates.txt`). `main` has since moved to `b29bdec` (4403 D5: `container/`, `.github/` and docs only; no crate, example or Cargo file). It is merged, with no rerun. Measured on `d324028`, the merge of `main` at `ecc8d01`: `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0; `cargo test --workspace` 0, with 1947 passed and 0 failed. Earlier, on `53bb75b`: the same, with 1942 passed.

## 1. Predictions (committed before any variant was run)

Written after reading `main` and before running any CLI command on any specification. The facts in §1.1 were read at source, so they are not predictions; §1.2 to §1.4 are.

### 1.1 Read at source before predicting

- The emitter draws SPI devices `W25Q64JVxIQ` and `ICM20948` only (`crates/aethelgard-eda/src/schematic.rs` ~L848–L891); any other SPI part is `UnknownPart`. So `ICM42688P` and `BMP581` (F2) cannot be emitted on `main`; their board path is `AET-WO-4220b` (held).
- Specification 12 has no field for USB, FDCAN, a test point, a jumper, a solder bridge, a DNP position, a button, a current shunt or sense amplifier, a timer input capture, `MCO`, or a second pull-up value per bus (`crates/aethelgard-ir/README.md` §Specification). `power.sense` exists, but it is the battery's voltage divider, not a current sense.
- The emitter names `NRST`, `SWDIO` and `SWCLK` for itself (`names::own_nets`), and `connectors` may carry a domain, `GND` or a declared signal net. A pin with `net: null` is a no-connect.
- The vendored KiCad subset (`aethelgard-kicad-lib/data/kicad-10.0.7`) holds the footprints the emitter uses today: LQFP-144, SOIC-8, WSON-6, LGA-8 (Bosch 2.5 mm), QFN-24 (InvenSense 3 mm), TSSOP-14, HSOP-8, SOT-23-5, 0402/0603/1210 passives, IHLP-5050, XT30PW-M, JST GH 4/6/10 and JST SH 6.

### 1.2 S1 classes, assigned before measuring

Each *must* cites the prediction it would test on silicon. A row whose "why" cannot cite one is not a *must* (§5 of the work order).

| # | Class | The prediction it tests (cited) |
|---|---|---|
| T1 selectable pull-ups | **must** | `RUL-19` (1): the campaign's rise time is computed from the declared pull-up and the declared capacitance; `extract` (`1305`) bounds the copper. Sentinel-01 is ordered to falsify the extractor (roadmap §3). One pull-up value measures one point; a sweep measures the slope that the bound predicts. Also northstar §2's second divergence (the H7's 1.7 V clock-high threshold): visible only with a weak pull-up |
| T2 capacitance coupon | extra | Same bound, with a known added load; T1 plus an off-board load at a connector covers most of it |
| T3 current sense | extra | `4300d`/`4300d3` declared loads and the buck's junction screen; a bench supply's current reading covers the total |
| T4 test points | **must** | No bench row on Sentinel-01's copper can be read without a probe point on its line (`3302`'s bench rows were read on the Nucleo's headers) |
| T5 fault jumpers | extra | `2301`'s fault classes and chaos's static faults (`E2`, `S3`) against the same fault on silicon; valuable for `INV-05` attribution, not needed to order Sentinel-02 |
| T6 adjustable MCU rail | extra | `2301`'s droop model and `bor_level` (`4300d3` SupplyFloorBelowBorRising); the Nucleo can be fed from a bench supply too [inferred, to be checked against UM2407] |
| T7 loopbacks | extra | `4300a`/`4300b` virtual-time witnesses against silicon; the same timing is measurable on the Nucleo, so this is the board witnessing itself, not new information |
| T8 programming paths | **must** for SWD (already on the board); extra for USART boot with a BOOT0 jumper and a reset button; defer for USB DFU (no USB) | `3301`'s programming on our copper; `4300g`'s `NoProgrammingPath` refusal |
| F1 power tree | **must** | `4300d`/`4300d3`/`4300d2`: the bucks' static checks, the loop rules, hot-plug damping and the support gate with a regulator as the MCU's source. Sentinel-02 carries this tree; the Nucleo cannot test it |
| F2 IMU and barometer | **must** | `4220`'s own C# models and `4210`'s witnessed scripts against silicon, and `1310`'s sub-grid escape on copper; Sentinel-02 flies these parts |
| F3 motor outputs | extra | `4300a`'s DShot timing; measurable on the Nucleo's headers |
| F4 CRSF input | extra | `4300b`'s framing and the HSI tolerance; measurable on the Nucleo |
| F5 USB | defer | Not on `main` (`4300e1` unwritten); no specification field |
| F6 SPI flash | **must** (kept) | The first scar's fix (`4230`: `SSM`/`SSI` before `SPE`) on our own copper |
| J1 witness fixture | extra (the later job) | Needs input capture and an I2C/SPI target role, neither in the specification |
| J2 DroneCAN node | defer | Needs FDCAN and a transceiver, neither in the specification |
| J3 Klipper/grblHAL/Zephyr | note only | — |

### 1.3 The variants and their predicted exits

The variants are built by `spike/3400/make_variants.py` from today's Sentinel-01, grafting only new sections, so V0's `net_constraints` and `board.rules` are kept unchanged in every variant.

- **V0:** `examples/sentinel-01-minimal.json`, byte for byte.
- **V1 (must, as far as specification 12 expresses it):** V0 plus F1: the XT30 input, the hot-plug capacitor, the two LMR33630 bucks (5 V and 3.3 V), the battery sense, and the classes they need. The 1.8 V LDO and the TXU0304 crossing are left out, because Sentinel-02 has no 1.8 V domain (roadmap §5, `4500`) and no candidate device sits behind them. `mcu.supply_source` becomes `null`, because a regulator then outputs the MCU's supply. T1, T4, T8's extras and F2 are left out: no field, or no emitter arm.
- **V2 (must and extra):** V1 plus four DShot600 outputs to a JST GH connector (F3), a CRSF port to a JST GH connector (F4), and a DS-009 TELEM port as the console's sibling, so that J1 has a serial line out.
- **V3 (everything expressible):** V2 plus a second I2C bus on a DS-009 I2C connector at a different pull-up and with an off-board capacitance declared: the part of T1 and T2 that specification 12 can say, and the one free-standing coupon socket.

| Variant | `validate` | `generate` | `emit` | `check erc` | `check drc --spec` | Confidence |
|---|---|---|---|---|---|---|
| V0 | 0 | 0 | 0 | 0, no finding | 0, no finding | high: `boards/sentinel-01` is regenerated in CI |
| V1 | 0 | 0 | 0 | 0 | 0, with the buck's loop rules passing | medium (70 %): the full power fixture routes at 2 layers on `main` (`board_tier2.rs` L1244); Sentinel-01's 2-layer board adds the flash, sensors and three connectors |
| V2 | 0 | 0 | 0 | 0 | 0 | medium-low (55 %): eight more nets to connectors on 2 layers |
| V3 | 0, unless a bus with no device is refused | 0 | 0 or `Unroutable` | 0 | 0 | low (40 %) |

- **Acknowledgements:** V0's `acknowledge_unwitnessed` names fields of GPIOA, GPIOB, SPI1, USART1, I2C1 and RCC. Adding rows adds ledger rows but moves no solved pin of V0's nets, I predict, so `validate` does not refuse them as stale (`UnwitnessedAcknowledgementStale` fires on a named field the ledger lacks, not on a row nobody names). If the solve moves a pin, the entry goes stale and the variant gets the ledger's current list, recorded as a named difference from V0.
- **Board outline:** V0 is the emitter's rectangle as committed. I predict V1 grows by at least the buck pair's area (each buck with its IHLP-5050 inductor is about 15 × 15 mm with its loop parts), V2 by one or two connectors.
- **Renaming check:** the largest passing variant routes to its own copper under all 20 renamings (`1307`'s ruled order), 20/20, provided the harness renames every name the board's `Stack` keys (classes, joins, thermal arrays), not only `planes`. If the harness leaves those unrenamed, a failure is the harness's.
- **SNVSAN3F at two layers:** `check drc`'s `power.conditions` names the inner ground plane as not met (the tier-2 test looks for "item 5"), and the bench can compare instead: switching-node ringing and output ripple with a 20 MHz-limited probe, radiated noise on the I2C lines while the buck switches, and the buck's case temperature at the declared load.

### 1.4 Cost, predicted

- Fabrication of a 2-layer board of the order of 60 × 50 mm, five pieces, from a low-cost fab: about US$5–30 before shipping, assembly extra [inferred, to be sourced].
- The MCU dominates the BOM: STM32H753ZIT6 about US$15–20 at one piece [inferred, to be sourced].

### 1.5 Predictions for the amendment (WO §0, Scotty 2026-11-16 22:30 UTC), committed before any four-layer run or W measurement

**S2 at four layers.** Each variant's `board` section is replaced by `crates/aethelgard-cli/tests/fixtures/four-layer.json`'s (S-P-P-S on OSH Park's stackup: In1.Cu `GND`, In2.Cu `VDD_3V3`). Nothing else changes.
- **The five commands:** V0-4L to V7-4L exit 0 with no finding (75 %). A refusal, if any, comes from the plane's net or a class whose net is now a plane (`ClassNetAbsent` or a router refusal), not from routing space.
- **The outline is the same as at two layers.** The placer does not read the layer count [inferred]. There are fewer vias to ground, since the ground stitching is the plane.
- **`extract` moves the other way.** Every line's bound rises, because the elliptical cap over a near plane adds capacitance (`4100`: the 4-layer fixture's I2C lines were 45.9 and 48.8 pF, against 43.3 pF here at two layers).
  - I2C stays under 100 pF.
  - SPI_SCK and SPI_MOSI (declared 30 pF) fail on at least one of V5–V7-4L (55 %).
  - The 2-layer pass does not carry over to four layers for free.
- **SNVSAN3F item 5:** the "not met" condition disappears at four layers, or turns into a judged condition (70 %).
- **Renaming:** 20/20 on the largest 4-layer variant.

**W rows** (each will be read at source in RM0433 Rev 9, DS12117 Rev 10, the KiCad 10.0.7 library or the sigrok wiki, or labelled inferred):
- **W1, the probe header:**
  - 16 channels in 2 × 8, with a ground per pair;
  - a direction-controlled translator per 8 channels, 1.65–5.5 V on the target side (e.g. SN74LXC8T245);
  - series resistors and an ESD array.
  - The specification cannot express it: no GPIO-capture role exists, and a connector pin cannot carry an undeclared net.
- **W2, the logic-analyzer capture:**
  - Rate: timer-paced DMA from `GPIOx_IDR` reaches 10–25 MS/s, the same for 8 and 16 channels, because one read takes either width. It is limited by the bus path from DMA1/2 (D2) to GPIO (AHB4, D3). Labelled inferred unless a cited figure is found; the Nucleo measurement is Scotty's.
  - Depth: AXI SRAM is 512 KB, so 512 k samples at 8 channels and 256 k at 16.
- **W3, the host link:**
  - USB FS: on chip, PA11/PA12, about 1 MB/s bulk in practice [inferred].
  - USB HS through ULPI: an external PHY (USB3300/USB3320 class) on 12 pins with a 60 MHz clock.
  - Ethernet RMII: a LAN8742A-class PHY on 9 pins plus MDIO/MDC, with a 50 MHz reference.
  - **Pin collisions with V5:** ULPI_CK (PA5) and RMII_CRS_DV (PA7) collide with SPI1; ETH_MDIO (PA2) with CRSF_TX; ULPI data on PB10/PB11 with the EXT I2C bus.
  - **Layers:** ULPI and RMII want a reference plane, so either pushes toward four layers.
- **W4, protocol capture:**
  - SPI as a receive-only slave, USART receive, and timer input capture are each available.
  - RM0433 describes no listen-only or monitor mode for I2C: a slave ACKs its address, and the peripheral cannot observe without taking part. So I2C goes through the analyzer path.
- **W5, stimulus:** I2C target mode, an SPI slave, a CRSF source by USART TX, and a DShot reader by input capture plus DMA. Each exists in RM0433. None is a specification role today except the serial port's TX.
- **W6, the host protocol:** SUMP, as sigrok's `openbench-logic-sniffer` driver speaks it, so PulseView's decoders work. Its channel and depth limits are those the driver supports.

**S4, cost:**
- (a) the lean probe, V0 at 2L;
- (b) the loaded board at 2L, V6 (largest passing every gate);
- (c) V6-4L.

**Calendar to the order** [inferred]:
- (a) a day. V0 has no power tree, so E0 does not apply.
- (b) one to two weeks: E0, E1, E2, `4220b`, and the witness rows' engine work orders (a capture-port role, a host link).
- (c) as (b), plus whatever four layers adds to `extract`'s declarations.

| Configuration | OSH Park | JLCPCB / PCBWay | Assembly |
|---|---|---|---|
| (a) V0 | about US$16.6 for 3 at $5/sq in (2L) | per-size price only through an interactive quote, so it goes on Scotty's list (90 %) | — |
| (b) V6 2L | about US$83 for 3 | as (a) | JLCPCB's published fees as read; the BOM dominates, with the MCU the largest line |
| (c) V6 4L | about US$167 for 3 at $10/sq in (4L, rate to be read) | as (a) | as (b) |

## 2. S1: every candidate, every field

Pins are after the solve (V5, `results/v5/0-solve.json`) [measured]. Board area is the change in the emitter's outline between variants [measured], or a footprint's courtyard from KiCad 10.0.7 in the pinned image (`spike/3400/sources/courtyards.txt`) [read at source]. KiCad libraries are CC-BY-SA 4.0 with the design exception (vendored `LICENSE.md`) [read at source]. Prices: digikey.sg, q1/q10 at cut tape, read 2026-11-16. The page says "All prices are in SGD" where marked **SGD**; **SGD?** means the page showed `$` with the region set to Singapore/SGD. No figure was converted to USD; that would be an estimate. "unknown" means no public page gave a price: LCSC and JLCPCB render prices in script, DigiKey.com returned 403, Mouser timed out. No login was used; nothing was bought.

### 2.1 Test features

| # | What it tells us (cited) | Specification 12? Smallest generic addition | Parts: rows, KiCad 10.0.7 symbol / footprint, production | Cost; area; pins | Class |
|---|---|---|---|---|---|
| T1 selectable pull-ups | `RUL-19` (1): the campaign's rise time from the declared pull-up and capacitance, against the scope. `extract` (`1305`) bounds the copper; Sentinel-01 is ordered to falsify the extractor (roadmap §3). Northstar §2's second divergence (the H7's ~1.7 V clock-high threshold) needs a weak pull-up | **Partly, today:** a second bus at another operating point. V5's `EXT` bus is 10 kΩ at 100 kHz, declared ≤ 1000 ns, bound 56.5/59.3 pF [measured]; V0's bus is 2.2 kΩ at 400 kHz. **Not:** a choice of value on one bus. Addition: **`jumpers`**, a solder jumper between two declared nets with a default state, with alternate pull-ups behind it, judged by the campaign once per state (§7 E3) | Existing rows: CRCW0603 (`power-parts.json`), the emitter's 0402 pull-ups. Jumper footprint: `Jumper:SolderJumper-3_P1.3mm_Open_RoundedPad1.0x1.5mm` and symbol `Jumper:SolderJumper_3_Open`, in the library. Production: Vishay's family page gives no status (research A) | Resistors: unknown price. Area: 12.6 mm² per 3-pad jumper (courtyard); the EXT port cost +11.5 cm² of outline (V5 against V2). Pins: 2 (PB10/PB11, I2C2) | **must** (the jumper form); the EXT bus is carried now |
| T2 capacitance coupon | The same bound and `RiseTimeTooSlow` against a known added load | **Partly, today:** V5's J8 (DS-009 I2C) declares 20 pF off-board, so a coupon of up to 20 pF plugs in within the declaration. A larger coupon needs a larger declaration. **On-board coupon:** the same `jumpers` addition, plus a capacitor row the extractor counts as `UnmodelledPart` today | C0G 0603 (Murata GRM1885C1H): no row; lifecycle and price unknown (research A) | Unknown; 4.3 mm² per 0603 plus 8.2 mm² per jumper; 0 pins | extra |
| T3 current sense | `4300d`/`4300d3`'s declared loads: V5's solve gives 658 mA on VDD_3V3 and 2000 mA on VDD_5V (declared), and the buck junction screen. A shunt measures each rail | **No.** Addition: `power.shunts` (a sense resistor in series between a regulator's output and its domain, with two Kelvin test points), optionally an INA226 as an I2C device | INA226AIDGSR: TI **ACTIVE** (ti.com/product/INA226, research A), symbol `Sensor_Energy:INA226` in the library; no `parts.json` row and no twin model (the ledger would acknowledge it). Shunt: no row | INA226 **SGD** 4.38 / 3.273 (digikey.sg, my read). Shunt unknown. Area: 10.3 mm² per 1206 shunt; VSSOP-10 not measured. Pins: 0 (on an I2C bus) | extra |
| T4 test points | Every bench row on our copper. `3302`'s rows were read on the Nucleo's headers; Sentinel-01's bus lines and rails have no probe point | **No.** A connector pin is the only way to reach a net: J3's VDD_3V3 and the DS-009 ports do it for some nets. Addition: **`test_points: [{net}]`**, any declared net, domain or `GND`, drawn as a pad and routed like any pin (§7 E1) | `Connector:TestPoint` and `TestPoint:TestPoint_Pad_D1.0mm` (no part); Keystone 5000 loop: footprint in the library, no digikey.sg listing found | Pad: no BOM cost; 0.7 mm² courtyard each. Keystone: unknown. Pins: 0 | **must** |
| T5 fault jumpers | `2301`'s faults and chaos's static faults (`E2` device absent, `S3` cs-never) against the same fault on silicon (`INV-05`) | **No.** The same `jumpers` addition: open a line, or a weak pull-down by jumper | Solder jumpers (no part); 2.54 mm headers: `PinHeader_1x02/1x03`, maker part not chosen, price unknown | 8.2–30.5 mm² each; 0 pins | extra |
| T6 adjustable MCU rail | `2301`'s droop model, `bor_level` (`4300d3` SupplyFloorBelowBorRising), `1208`'s MCU range | **Partly, today:** J3 pin 1 is on VDD_3V3 in every variant, so a bench supply can feed the rail there. Without a link to open, it fights U11's output. `mcu.supply_source` takes one source (`4300g` D3). Addition: a `jumpers` entry between the regulator's output and the MCU domain, with the supply source judged in each state | Header 1x02 plus shunt: maker part not chosen, unknown | 21.6 mm²; 0 pins | extra |
| T7 loopbacks | `4300a`/`4300b`'s virtual-time witnesses against silicon, the board witnessing itself | **No.** TX to RX needs `jumpers`; a motor output to a timer input needs a **capture input** role; MCO needs `mcu.clock_out`. Both MCO pins are free in V5 (PA8 `RCC_MCO_1`, PC9 `RCC_MCO_2`; pin table) | No parts beyond jumpers and test points | 0 BOM; pins: 1 capture, 1 MCO | extra |
| T8 programming | `3301`'s programming on our copper; `4300g`'s `NoProgrammingPath` | **SWD with connect-under-reset, today:** probe `p-nrst` puts NRST on J2 pin 3 and passes every gate [measured]; V5 carries it. **USART boot:** the console is held on PA9/PA10 (USART1) by `pin` in V2–V5. [inferred: AN2606 lists USART1 PA9/PA10 among the H74x/75x bootloader's interfaces; the PDF timed out twice and was not read.] **BOOT0 to a header: refused** (`ConnectorNetUnknown`, probe `t8-boot0-on-swd`), because BOOT0 is not one of the emitter's own nets as NRST is. Addition: BOOT0 joins `names::own_nets`, and an optional reset switch joins the MCU support row (§7 E2). **USB DFU:** defer with F5 | PTS810SJM250SMTRLFS: digikey.sg "Active" (distributor); footprint `SW_SPST_PTS810`, symbol `Switch:SW_Push`. Tag-Connect `TC2030-IDC-NL`: footprint in the library | PTS810 **SGD?** 0.80 (q10 not listed); 21.1 mm². Pins: NRST and BOOT0 are dedicated | **must** for SWD and NRST (carried in V5); extra for BOOT0 and reset |

### 2.2 The flight controller's subsystems

| # | What it tells us | Specification 12? | Parts | Cost; area; pins | Class |
|---|---|---|---|---|---|
| F1 power tree | `4300d`/`4300d3`/`4300d2`: the bucks' static checks, the 15 loop rules, hot-plug damping, and the support gate with a regulator as the MCU's source. Sentinel-02 carries this tree | **Yes.** The bucks: V1–V7. The LDO (TLV75518, VDD_1V8, 20 mA declared): V6, every gate passed, and V7. **But** `power.input.hot_plug` is not drawn (F1, §6) | `power-parts.json` rows exist (LMR33630ADDA, IHLP5050FDER8R2M01, GCM32EC71H106KA03, …), with vendored KiCad symbols and footprints (`LMR33630ADDA`, `Texas_HSOP-8-1EP`, `L_Vishay_IHLP-5050`, `AMASS_XT30PW-M`). LMR33630: TI **ACTIVE** at family level (research B) | LMR33630ADDAR **SGD** 3.96 / 2.956; IHLP5050FDER8R2M01 **SGD** 2.92 / 2.403; GCM32EC71H106KA03K **SGD?** 1.37; UCZ1H470MCL1GS **SGD** 1.92 / 1.233; XT30PW-M unknown (no listing). Area +44.9 cm² of outline (V1 against V0). Pins: 1 (PF3, ADC3_INP5, VBAT_SENSE) | **must** |
| F2 IMU and barometer | `4220`'s C# models and `4210`'s witnessed scripts against silicon; `1310`'s sub-grid escape and `4220b`'s own footprints on copper | **No.** The specification names the parts (`parts.json` rows exist); the emitter refuses them: its SPI arms are `W25Q64JVxIQ` and `ICM20948` only. This is `4220b` | Neither part has a KiCad 10.0.7 symbol (Sensor_Motion, Sensor_Pressure; research B; `4220b` grounds). An LGA-14 3 × 2.5 mm footprint exists (11.5 mm²); none for the BMP581's LGA-10. ICM-42688-P: TDK **"Production"**. BMP581: Bosch's page states no status | ICM-42688-P **SGD?** 6.67; BMP581 **SGD?** 4.87 (q10 not shown). Pins: 2 chip selects, plus one SPI bus if not shared | **must**; needs `4220b` |
| F3 motor outputs | `4300a`'s DShot timing witnessed in virtual time, against the scope | **Yes** (V2–V5). Solved free, the motors took PA8 (MCO1's only pin) and PA11 (OTG_FS DM); V4/V5 pin them to PE9/PE11/PE13/PE14 (TIM1_CH1–4) | SM06B-GHS-TB, an existing row | **SGD** 0.67 / 0.571; motors, CRSF and MAVLINK together +29.8 cm² (V2 against V1). Pins: 4 | extra |
| F4 CRSF input | `4300b`'s framing at 420 000 baud and HSI's tolerance (`SerialBaudTolerance`), against silicon | **Yes** (V2–V5): USART2 PA2/PA3 on a DS-009 TELEM GH-6 | SM06B-GHS-TB | as F3; pins 2 | extra |
| F5 USB | `4300e1` (pins, the 48 MHz clock, VBUS) | **No**: no field; `4300e1` is unwritten. OTG_FS exists only on PA11/PA12 (pin table), kept free in V4/V5 | `USB_C_Receptacle_GCT_USB4105-xx-A` and `USBLC6-2SC6` are in the library. USB4105-GF-A **SGD?** 1.10; USBLC6-2SC6 **SGD?** 0.74; lifecycle unknown (GCT 403, ST timeout) | 95.1 + 14.5 mm²; pins 2 (+VBUS sense) | defer |
| F6 SPI flash | The first scar's fix on our copper: `SSM`/`SSI` before `SPE` (`4230`); the ledger's MODF row reads 0 on the bench | **Yes** (every variant) | W25Q64JVxIQ: NRND, acknowledged (`4211`). Research B could not re-read Winbond's catalogue (a sign-in page) | **SGD?** 6.01. Pins 4 (PA5–PA7, PA0) | **must** (kept) |

### 2.3 A job after bring-up

| # | What it adds | Specification 12? | Parts | Cost; pins | Class |
|---|---|---|---|---|---|
| J1 witness fixture | Bench rows for later boards, run by a board | **Partly.** CRSF out: the CRSF port's TX (PA2) exists. Reading DShot: J5's pins are TIM1_CH1–4, which can capture [read at source], so firmware can read DShot there with no respin, but the specification has no capture role. Standing in for a sensor needs I2C/SPI **target** roles, which are not in the specification | No extra parts: J5, J6, J8 | 0 BOM; uses J5 and J6 | extra, recommended (§7) |
| J2 DroneCAN node | A sensor node on Sentinel-02's CAN bus, running AP_Periph | **No**: no FDCAN field and no transceiver row. FDCAN1 PD0/PD1 and FDCAN2 PB5/PB13 are free in V5 (pin table and solve). AP_Periph has H743 targets at Copter-4.7.2 (`CubeOrange-periph`, `MatekH743-periph`: `MCU STM32H7xx STM32H743xx`, `env AP_PERIPH 1`; research A, GitHub) | TCAN334DCNR: TI **ACTIVE**, "3.3V single supply", classic CAN at 1 Mbps (DroneCAN's rate) (ti.com/product/TCAN334); symbol `Interface_CAN_LIN:TCAN334` and footprint `SOT-23-8` in the library. CAN-FD transceivers (TCAN1462-Q1) need 5 V VCC | TCAN334DCNR **SGD** 3.90 / 2.905 (backorder); SOT-23-8 14.4 mm² + GH-4 60.5 mm². Pins 2 | defer |
| J3 Klipper, grblHAL, Zephyr | Note only | — | Klipper `src/stm32/Kconfig`: `MACH_STM32H743`, `MACH_STM32H723`. grblHAL Drivers: Nucleo-H743ZI, WeAct MiniSTM32H743. Zephyr: `nucleo_h743zi`, `nucleo_h723zg` (research A, URLs there). V5 solves 19 of the pin XML's 114 I/O pins and SWD holds PA13/PA14, leaving 93 free (count corrected after an XML-parser re-check), so each would find pins. **Prediction, not fact:** an H743 build runs on the H753ZI (the H753 is the H743 with crypto, `RUL-08`); unverified | — | — | note |

### 2.4 Rows added by the spike (the work order's "add any you find")

| # | What it tells us | Specification 12? | Parts | Cost; area; pins | Class |
|---|---|---|---|---|---|
| A1 MAVLink TELEM port (J7, DS-009 TELEM) | `4300b`'s MAVLink framing and HSI tolerance on a second USART. It is J1's second serial line out, and a companion computer's port (roadmap §5 P4) | **Yes** (V2–V7): USART3 on PC10/PC11 in V5 | SM06B-GHS-TB, an existing row | **SGD** 0.67 / 0.571; pins 2 | extra (carried) |
| A2 external SPI port (J9) | An SPI bus off the board: J1 driving the next board's SPI as a master, and later as a target | **Yes**, but not within its declared bound (V3, V4) | SM06B-GHS-TB | Pins 3; `extract` 54.0/56.1 pF against 50 pF declared when pinned to SPI4 on PE2/PE5/PE6 (V4) | extra; dropped from V5 (Q2) |
| A3 the 1.8 V crossing and IMU (proxy) | No new prediction. It is the only stand-in on `main` for F2's density: a 3 × 3 mm QFN at 0.4 mm pitch on its own bus, behind a translator (`4300d2`, `1310`) | **Yes** (V7); `extract` refuses `TranslatorPinUnbounded`, as on `main` | `main`'s fixture rows (ICM20948 EOL, acknowledged; TXU0304) | +11.35 mm of outline height (V7 against V6); pins 4 | not a candidate: a measurement aid |

## 3. S2: the variants, measured

Built by `spike/3400/make_variants.py` and run by `spike/3400/run.sh <v>`. Every step's stdout, exit and stderr are in `spike/3400/results/<v>/`. Host: release CLI from `3dcd242`, then from the merge of `ecc8d01` (below). KiCad steps ran the same binary inside the pinned tools image (`container/IMAGE`, `c69ff9bc…`) under `~/bin/capped-run`. Caps were read back as memory.max 8 GiB, memory.swap.max 0 and pids.max 8192 (each `*.stderr`). Inside rootless podman the sandbox reports `uid_boundary: false` (`user_namespace`), as in every local tier-2 run.

| Variant | Adds (on top of the previous) | validate | generate | emit | erc | drc --spec | drc + parity | support | extract | Outline (mm) | Parts | Router: segments / track / vias / rip-ups |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| V0 | today's Sentinel-01, byte for byte | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 44.10 × 48.50 | 34 | 549 / 597 mm / 101 / 18 |
| V1 | F1: XT30, two LMR33630, battery sense; `supply_source` null | 0 | 0 | 0 | 0 | 0, 15 loop rules pass | 0 | 0 | 0 | 61.75 × 107.40 | 62 | 879 / 1093 mm / 141 / 90 |
| V2 | F3 (4 × DShot600, J5), F4 (CRSF, J6), MAVLink (J7); console pinned PA9/PA10 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 89.50 × 107.40 | 65 | 1037 / 1555 mm / 196 / 88 |
| V3 | a second I2C bus (10 kΩ, 100 kHz) on DS-009 I2C J8; an external SPI port J9 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 113.10 × 107.60 | 69 | 1144 / 1962 mm / 237 / 44 |
| V4 | V3 with pins kept free: motors PE9/11/13/14, J9's SPI on PE2/PE5/PE6 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **1**: XEXT_SCK 54.0 pF, XEXT_MOSI 56.1 pF > 50 pF declared (~100 mm of track) | 113.10 × 107.60 | 69 | 1092 / 2002 mm / 208 / 92 |
| **V5** (recommended) | V4 without J9; NRST on J2 pin 3 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **100.05 × 107.60** | 68 | 1020 / 1743 mm / 189 / 41 |
| V6 (largest, every gate) | V5 + the LDO U12 (TLV75518) to VDD_1V8, 20 mA declared | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.05 × 107.60 | 71 | 1109 / 1722 mm / 214 / 31 |
| V7 (largest, the five) | V6 + `main`'s crossing U13 (TXU0304) and ICM-20948 U5 on SPI `IMU` (F2 proxy) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **2**: `TranslatorPinUnbounded` (SCES935A gives no maximum Cio), as `main`'s `board_tier2.rs` L1244 expects | 100.05 × 118.95 | 79 | 1511 / 2467 mm / 302 / 74 |

- `findings == []` in `check erc`, `check drc --spec`, DRC with parity and `check support` for V0–V7 and `p-nrst` [measured, asserted by script]. V3's J9 and V2's MAVLink port were not among the predicted definitions; they are §2.4's A2 and A1.
- `generate`'s `kicad/` equals `emit`'s three files byte for byte for V0–V7. `generate` of V0 reproduces `boards/sentinel-01/**` (`diff -rq` empty) [measured].
- The router's numbers come from `route::route` on each emitted board (`9-rename.txt`, first two lines). Each unrenamed reroute equals the emitted copper.
- The variants V6 and V7 were added after the advisor's review, to measure F1's LDO and the nearest F2 proxy; V0–V5's files were unchanged by the rebuild.
- **Rerun after `main` moved to `ecc8d01` (AET-WO-1208: validate's MCU rail rule, emit validates first).** Merged, the CLI rebuilt, every variant and probe rerun. Every exit and every verdict JSON is byte-identical to the run on `3dcd242`; only the stderr's scope names differ. So the emitted boards, and with them the renaming results, stand (`results/*/3-emit.json` hashes compared).
- **First runs, kept in `spike/3400/sources/first-runs/`:**
  - With the motors solved free, the solver moved the console from PA9/PA10 to PB14/PB15, because TIM1 took PA8–PA11. That left the bootloader's USART1 pins and made V0's GPIOA acknowledgements stale (`validate` exit 2, `UnwitnessedAcknowledgementStale`).
  - Pinning the console where V0 solved it fixed both, except PUPD8 and PUPD11: motor pins are pulled down from the first GPIO initialisation (`4230`'s ruling), so they leave the same-source row. Those two names were removed from V0's entry; nothing new is acknowledged.
  - Unpinned, V3's SPI_MOSI was bounded at 32.6 pF against 30 pF (`extract` exit 1).
  - **`generate` and `emit` accepted the stale V2 that `validate` refused** (the acknowledgement check is the CLI's `validate`'s and `verify`'s, by the README). This is noted, not claimed as a defect.
- **No variant fails for want of a field**, because the rows that need one were never put in. Each is recorded in §2 with its addition, and the two probes measure the edge: NRST on a connector passes, and BOOT0 on a connector is refused.
- **Predictions scored:** V0, V1 and V2 as predicted. V3 better than predicted (40 %): it passes the five, and passes `extract` once the console is pinned. Outline growth as predicted, but larger: V1 adds 44.9 cm², not one 15 × 15 mm block per buck. The emitter's placer lays the tree out at 61.75 × 107.40 mm. The acknowledgement prediction was wrong for V2 (the solve moved pins), as the predictions allowed.

## 4. The renaming check (1307), on the largest passing variant

- **V7 (the largest) and V6: 20/20** renamings route to their own copper (`results/v7/9-rename.txt`, `results/v6/9-rename.txt`). V0–V5: 20/20 each.
- **Harness, named.** `spike/3400/rename/` copies `escape_tier1.rs`'s `mix`, `hash`, `renamed` and `named_back` (seeds 0..20, the same assertion), with two extensions:
  1. `renamed` also renames the stack's classes, pairs, through-hole joins and thermal arrays by the same maps. 1307's boards declare none of these.
  2. `named_back` maps `Routed.arrays` (`AET-WO-1309`) back by reference and net. It postdates 1307's copy.
- **Without extension 2, every power variant scored 0/20.** Every seed routed, with the unrenamed rip-up count, and the segments, vias and zones were set-equal, but the arrays kept their renamed references. That is the harness's, not the router's.
- **Negative control:** with the classes left unrenamed (`AET_3400_SKIP_CLASSES=1`), V3 scores 0/20, with 452 segments differing (`results/v3/9-rename-negative-control.txt`). So the comparison catches a real dependence.
- **For the architect:** 1307's guard, extended to a board with thermal arrays, would need extension 2 or it fails falsely (Q4).

## 5. SNVSAN3F at two layers: not met, and what the bench measures instead

`check drc --spec` names it itself on every power variant: "LAYOUT: not met: SNVSAN3F p. 33, 10.1 item 5 asks for a ground plane on an inner layer; this board has two copper layers". It also names the items it does not judge (the input loop's area, the SW node's copper, FB routing, PGND) [measured, `results/v5/5-drc.json`]. Its 15 loop rules pass (C/L/R distances to their pins, 1.25–4.63 mm against 2–5 mm engine bounds).

The bench comparison against Sentinel-02's four layers, each at the declared 2658 mA steady input [inferred method]:

1. **Switching-node ringing:** overshoot and ring frequency on U10/U11's SW pin, with a short ground spring at 20 MHz and at full bandwidth.
2. **Output ripple** on VDD_3V3 and VDD_5V at the regulator, and at the MCU's farthest VDD pin.
3. **Coupled noise on I2C/SPI** while the buck switches: the quiet-line amplitude on I2C_SDA and SPI_MOSI, both in V5's power-variant placement.
4. **Thermal:** U10/U11 case temperature at load, against the junction screen's declared 20 cm² of copper. The screen's input is "declared by the fixture, not measured on the drawn board".
5. **Radiated:** a near-field probe sweep over the bucks. Comparative only; no limit is claimed.

These need test points on SW, the rails and the bus lines (T4). Four layers have to beat Sentinel-01 on (1) to (3) by a margin we can state. Without that, the four-layer ruling for Sentinel-02 rests on TI's word alone (`4300d2` D4).

## 6. Adversarial pass (targeted, on the measurements)

- **A variant that passes because a constraint was dropped:** none. `spike/3400/relaxations.py` diffs each variant against V0 (`results/relaxations.txt`): no net constraint, board, rule, bus, device or lifecycle is removed or loosened. The named changes, none of them relaxations:
  - the console's pins pinned to V0's own solve;
  - `mcu.supply_source` from J3 to `null`, because U11 outputs VDD_3V3 (`4300g` D3). J3's VDD_3V3 pin stays, and with the buck that is a back-feed point (T6);
  - PUPD8 and PUPD11 removed from one acknowledgement, because the ledger moved them.
  - **V7 only:** one acceptance added, `accept_lifecycle` for the ICM-20948 (`eol`), carried with the proxy from `main`'s fixture ("A twin stand-in for the IMU, never bought"). The script flags it. It is an acceptance, so V7 is a density proxy and never a board to order.
  - What the power tree brings is declared, not measured, and carries over from `sentinel-02-power.json`: the junction's 20 cm² and 550 mW, the 2000 mA 5 V load ("DS-009 port budget for Sentinel-02"), and the board temperature range. For Sentinel-01 the 5 V load is generous. None of it is relaxed.
- **A *must* row whose "why" cites no prediction:** none demoted. T1, T4, T8 (SWD), F1, F2 and F6 each cite one (§1.2, §2). T8's USART boot and reset rest on AN2606, which was not read, so they stay *extra*.
- **A cost with no source:** every figure carries its URL in `spike/3400/sources/research-A.md` and `research-B.md` (the research subagents' files, copied verbatim), or in `sources/digikey-sg-reads.md` (my reads: INA226, TCAN334, PTS810, USBLC6, USB4105, TMP117). Research A's AP_Periph claim was re-checked in the pinned ArduPilot image (`sources/ap-periph.txt`), and its TCAN334 lifecycle on ti.com. That image pin was `a4818a04`. `main` re-pinned it to `7b7ebed3` with `b29bdec` (4403 D5, merged after the check); the `Copter-4.7.2` tag is unchanged by that work order, which mirrors sources and does not move the tag [inferred from its title]. Not re-run. OSH Park's rate and TMP117's price were re-read and match. Prices that could not be read say "unknown". The q1 sum in §7 is SGD, labelled.
- **F1. The declared hot-plug capacitor is not on the board, and its reference is reused.**
  - On `main`, `power.input.hot_plug` (C1, UCZ1H470MCL1GS) satisfies `BatteryInputUndamped` in `ir`, but nothing in `aethelgard-eda` draws it (no `hot_plug` in `eda/src`).
  - The emitter numbers its own decouplers from C1, so the bill of materials' C1 is a 100 nF 0402. Measured on V1–V5 and on `main`'s `sentinel-02-power.json` (no `UCZ1H470` in schematic, board or bill of materials).
  - Known as backlog (4300d3 D5; roadmap §7.1 L635, "before `4500`"; 1206 FEEDBACK A2 notes the emitter never draws it). New here:
    1. the deadline is now Sentinel-01's order, because it carries the tree;
    2. a declared reference collides with an emitted part of another value, and nothing refuses it.

## 7. S3: the decision table

### 7.1 Three tiers

| Tier | Rows | What it takes |
|---|---|---|
| **1. Carry now, no engine change** (V5, measured) | F1 power tree; F6 flash; the two sensors; F3 four DShot on J5 (PE9–PE14, TIM1); F4 CRSF on J6; MAVLink on J7; console on J1 held at PA9/PA10 (USART1 boot); T1/T2's second operating point (EXT bus at 10 kΩ / 100 kHz on DS-009 I2C J8, the off-board coupon socket); T6's J3 rail tap; T8's SWD with NRST on J2. Pins kept free for later: PA8, PC9 (MCO), PA11/PA12 (USB), PD0/PD1 (FDCAN1) | A specification in the next board work order. No version number |
| **2. Engine work orders, before ordering** | E1 test points (T4); E2 BOOT0 on a header plus a reset switch (T8); E0 the hot-plug draw and its reference (F1, §6); **`4220b` unheld** (F2) | Below |
| *T1's place* | T1 is a *must* for what it measures: the rise time at more than one pull-up, including a weak one. V5's EXT bus (10 kΩ, 100 kHz) beside SENSORS (2.2 kΩ, 400 kHz) measures two points on our copper, the weak one included, and so meets that *must* in tier 1. **Reclassified after measuring:** the jumper form (a sweep on one bus) becomes *extra* and goes to tier 3 with E3. If the architect rules that one bus must be swept, E3 moves to tier 2, before ordering, with its own spike (Q5) | |
| **3. Defer, with pins reserved** | E3 jumpers and alternates (T1 full, T2 on-board, T5, T6's link, T7's TX–RX); E4 current sense (T3); E5 capture input and `MCO` (T7, J1); F5 USB (`4300e1`); J2 DroneCAN | Each its own work order. Sentinel-01's V5 pins leave them room |

### 7.2 The engine work orders it implies

| Id (proposed) | What | Size [inferred] | Unblocks |
|---|---|---|---|
| E0 | The emitter draws `power.input.hot_plug` and reads `controls` (4300d3 D5), and refuses a declared reference it would reuse | S | F1 is honest on copper: the damping the check credits is on the board |
| E1 | `test_points: [{net}]`, any declared net, domain or `GND`; a pad drawn, placed, routed and judged by parity. Symbol `Connector:TestPoint`, footprint `TestPoint_Pad_D1.0mm` vendored | S–M (emitter, placer, router targets, parity; one specification version) | Every bench row on Sentinel-01, the §5 comparison, and every later board |
| E2 | `BOOT0` in `names::own_nets`, so a connector pin may carry it over its strap; an optional reset switch in the MCU support row (`SW_SPST_PTS810`) | S | USART boot by a jumper, a user's path without a probe (`3301`) |
| `4220b` | As written (our own symbols and footprints; declared pin capacitance) | L (as written) | F2: the flight IMU and barometer on our copper. Sentinel-01 becomes its **second consumer, so the hold's reason ("only consumer Sentinel-02") no longer holds** |
| E3 | `jumpers`: a solder jumper between two declared nets with a default state; parts behind it; the campaign and `extract` judge each state | M–L (`INV-01` and `INV-07`: one specification, several electrical states) | T1 (full), T2, T5, T6, T7's TX–RX |
| E4 | `power.shunts` and an INA226 row (`parts.json`, with a twin model or an acknowledged ledger row) | M | T3 |
| E5 | A capture-input role solved to a timer channel, and `mcu.clock_out` | M | T7, J1's DShot reading |
| `4300e1` | USB static, as roadmap §5 | M–L | F5 |

**Recommended before the loaded board is ordered (Sentinel-03 under the amendment, §10.4): E0, E1, E2 and `4220b`.** The lean probe (a) needs none of them. E0 is a correctness gap on the board as specified. E1 is the instrument without which the board measures nothing. E2 is small. `4220b` puts the flight sensors on our copper first. E3–E5 make a richer bench board, but none is needed to order Sentinel-02.

### 7.3 The later job

- **J1, the witness fixture: yes.** What it needs on the board is already in V5:
  - CRSF TX on J6;
  - four TIM1 channels on J5, which can capture DShot from the next board with firmware alone;
  - a DS-009 I2C port (J8) and the DS-009 TELEM ports;
  - MCO pins free.

  Its firmware is ours (`RUL-12`), and the specification needs E5 (a capture role) to express the reading side. Standing in for a sensor (I2C/SPI target) is a later engine change, not a copper change, if J8 and a spare SPI port are on the board. V5 has J8 but not J9. J9 is the one row I dropped, for `extract`'s 50 pF bound (V4). If the architect wants an SPI target port, either declare J9's bound from its measured 56 pF with its source, or accept the longer lead. Q2.
- **J2, the DroneCAN node: not on (a); a cheap candidate for Sentinel-03** (restated after the amendment). On a board drawn later, the respin argument is gone. It needs an `fdcan`/transceiver specification addition (M) and costs:
  - a TCAN334DCNR, SGD 3.90 / 2.905, SOT-23-8 at 14.4 mm²;
  - a 120 Ω termination and a GH-4 (60.5 mm², SGD 0.53).

  FDCAN1 stays free on PD0/PD1 or PB8/PB9 (pin XML). That is why the capture port is port G (Q7).

### 7.4 The cost of the recommended variant (V5), superseded by §10.3 (the amendment's three configurations, at OSH Park's 2L and 4L rates)

- **BOM, priced at q1:** SGD 70.79 for the 17 priced rows of V5's 68 (digikey.sg, 2026-11-16; mixed **SGD** and **SGD?**, research B):
  - STM32H753ZIT6 29.15; BME280 5.97; TMP117AIDRVR 8.63; W25Q64JVSSIQ 6.01;
  - 2 × LMR33630ADDAR 7.92; 2 × IHLP5050FDER8R2M01 5.84; 2 × GCM32EC71H106KA03K 2.74;
  - 3 × SM04B-GHS-TB 1.59; 3 × SM06B-GHS-TB 2.01; SM06B-SRSS-TB 0.93.
- **Unknown:** the XT30PW-M and 50 passives (0402/0603/1210 MLCCs and resistors, the Murata and Vishay rows). E0 adds the UCZ1H470MCL1GS (**SGD** 1.92). `4220b` adds the ICM-42688-P (**SGD?** 6.67) and BMP581 (**SGD?** 4.87). E1's pads cost nothing; E2's switch is **SGD?** 0.80. No USD figure is claimed.
- **Fabrication:** OSH Park's 2-layer service is "$5 per square inch for 3 copies" (1.6 mm, ENIG; oshpark.com, re-read 2026-11-16). V5's outline is 16.69 in², so **US$83.43 for three** (computed from the page's rate). V0 would be US$16.58. JLCPCB and PCBWay give per-size prices only through an interactive quote: unknown.
- **Assembly** (JLCPCB's public page, as read by research B, not re-read): Economic setup $8.18, stencil $1.53, $0.0016 per SMT joint, $3.07 extended-part feeder fee.
- **The outline is the placer's, not a minimum:** 34 parts take 21.4 cm² (V0); 68 take 107.7 cm² (V5). A board of more than 100 mm on a side may leave a low-cost fab's cheapest size class [inferred, not sourced]. Q3.

## 8. Questions for the architect

- **Q1.** Move E0 (4300d3 D5, the hot-plug draw) from "before `4500`" to "before Sentinel-01 is ordered", and add the reference-collision refusal to it?
- **Q2.** J9, the external SPI port: drop it (V5, as recommended), or keep it with its bound declared from the measured 56 pF?
- **Q3.** Is V5's 100 × 108 mm acceptable? Or does a placement-density work order come before the order? V0's density was 1.6 parts/cm²; V5's is 0.63.
- **Q4.** 1307's renaming guard: should it gain this spike's two extensions (classes/pairs/joins/thermal renamed; `Routed.arrays` named back) once a power board joins its fixtures?
- **Q5.** E3 (`jumpers`) carries an `INV-01`/`INV-07` question: one specification, several electrical states, each judged. It wants its own spike before a build.

## 9. Rerun, evidence, reduction

- **Rerun from `main`:**
  1. `cargo build --release -p aethelgard-cli`
  2. `python3 -I spike/3400/make_variants.py`
  3. `spike/3400/run.sh v0` … `v7`, and `p-nrst`
  4. `CARGO_TARGET_DIR=~/.cache/aet-3400/target cargo run --release --manifest-path spike/3400/rename/Cargo.toml -- spike/3400/variants/v5.json spike/3400/results/v5/emit/s01-v5.kicad_pcb`
  5. `python3 -I spike/3400/relaxations.py`

  The emitted `gen/` and `emit/` trees are not committed (12 MB); `run.sh` regenerates them byte for byte.
- **Evidence:** `spike/3400/sources/` holds the research files, the digikey.sg reads, the KiCad 10.0.7 listing and courtyards from the pinned image, the AP_Periph check, the first runs and the gates. AN2606: both downloads and one fetch failed, so nothing was kept.
- **Reduction pass** (diff-sized):
  - the probe directory keeps only the BOOT0 refusal's input and output;
  - the NRST probe lives in `variants/p-nrst.json`;
  - the diff-printing branch of the rename probe runs only on a failing seed 0;
  - nothing else is removable without losing a rerun path.

## 10. The amendment (WO §0, Scotty 2026-11-16 22:30 UTC): the witness board, four layers, the cost

Predictions are §1.5, committed as `8ee7c93` before any run below. Sources are in `spike/3400/sources/`:
- `research-W-silicon.md`: RM0433 Rev 9, DS12117 Rev 10, ES0392 Rev 16 and the pin XML, quoted with pages;
- `research-W-parts.md`, `research-S4.md`: web, dated 2026-11-16;
- `kicad-w-parts.txt`: the pinned image's KiCad 10.0.7;
- `digikey-sg-reads.md`: my spot-checks. Each matched.

### 10.1 S1, the witness rows

| # | What it settles (cited) | Specification 12? | Parts: KiCad 10.0.7, lifecycle, price (digikey.sg, 2026-11-16) | Pins on V5; conflicts | Class |
|---|---|---|---|---|---|
| W1 probe header | 16 channels, a ground per pair (a 2 × 12 header, or 2 × 10 with a ground per four), direction-controlled translation per 8 channels, input ESD | **No.** There is no GPIO or capture-port role, and a connector cannot carry an undeclared net (`ConnectorNetUnknown`, §2.1 T8). Addition: a `capture_ports` role (a port's pins solved together, with a translator per byte, its direction pin and its target-side domain), plus translator rows in a parts table | **SN74LXC8T245PWR:** both ports 1.1–5.5 V, so 1.8, 3.3 and 5 V targets are covered (TI ACTIVE); **SGD** 2.41 / 1.772 (re-read); no KiCad symbol (`SN74LVC8T245`, 1.65–5.5 V, is in `Logic_LevelTranslator`; the pinout match is [inferred]); TSSOP-24 footprint in the library. SN74AVC8T245 stops at 3.6 V, so not for 5 V targets. TXS0108E (auto-direction): 1.2 Mbps open-drain, by TI's page (research-W-parts). **TPD8E003DQDR:** TI ACTIVE, **SGD** 2.47 / 1.565; no KiCad symbol. 2 × 10 header (Würth 61302021121): **SGD?** 1.82 / 1.55 | **16 pins of one port** (W2). On V5, ports D and G are fully bonded on LQFP-144 and unused by the solve [read at source: pin XML and `results/v5/0-solve.json`]. Port G is preferred: port D holds FDCAN1's PD0/PD1. A port-wide capture wants all 16 pins of one port, which also rules out port A and port B. **Pins:** 16, plus each translator's DIR (2) and OE (2, or strapped). **Area** (courtyards, `courtyards-w.txt`): 2 × TSSOP-24 at 66.7 mm²; a 2 × 10 header at 160.8 mm² (2 × 12: 191.7 mm²). TPD8E003's DQD package has no KiCad footprint | **extra**; needed for J1 (the job). It tests no Sentinel-02 prediction, so it is not a *must* |
| W2 analyzer capture | Timer update → DMAMUX1 → DMA1/2, peripheral `GPIOx_IDR` to memory (RM0433 §17.3, Tables 124/125: TIMx_UP inputs; §2.1.6: DMA1/2 reach AHB4 and every SRAM but the TCMs). IDR may be read by byte or half-word (RM0433 §11.3.1, L20852 checked), so 8 and 16 channels cost one transfer per sample either way | Firmware, plus W1's role | — | — | **extra**, needed for J1 |
| ↳ rate | **Not derivable from ST's documents** [read at source: no throughput figure in chapters 15–17 or DS12117]. The ceiling is one sample per AHB clock (HCLK ≤ 240 MHz at VOS0, rev V); the realised rate depends on the D2-to-D3 AHB path and arbitration. **It has to be measured on the Nucleo: Scotty's bench.** My 10–25 MS/s prediction stays unverified. **Searched for a cited third-party figure (2026-11-16, one bounded search): none for an H7 timer-DMA capture.** Only CPU-toggle figures ("16Mhz", "8 MHz") and one blog's unsourced "10-50 MHz" (ampheo.com) turned up. An F401 SUMP project gives a 32 K buffer but no rate (stm32duino.com/viewtopic.php?p=702). None of these is cited as a rate | | | | |
| ↳ depth | AXI SRAM 512 KiB → **524,288 samples at 8 channels, 262,144 at 16** [read at source: RM0433 Table 7; computed]. SRAM1–3 add 288 KiB, contiguous. NDT is at most 65,535, so a deep buffer needs circular or double-buffer mode (§15.2). ES0392 2.2.10: an AXI SRAM read corruption on rev Y/W silicon, with a workaround | | | | |
| W3 host link | **USB FS** on chip: PA11/PA12 (OTG_HS2's FS PHY), 12 Mbit/s, free on V5. **USB HS** needs an external ULPI PHY: DS12117 §3.41 "an external PHY device connected to the ULPI is required". Its 12 signals each have **one** pin on LQFP-144, and four collide with V5 (CK PA5 = SPI1_SCK; D0 PA3 = CRSF_RX; D3/D4 PB10/PB11 = EXT I2C2). **Ethernet RMII** at 100 Mbit/s: 9 signals plus MDIO/MDC. CRS_DV (PA7 = SPI1_MOSI) and MDIO (PA2 = CRSF_TX) have no alternate pin; TX_EN/TXD0/TXD1 move to PG11/PG13/PG12. MII is unavailable on this package (ES0392 2.2.27) | **No** (`4300e1` unwritten) | USB FS: USB4105-GF-A **SGD?** 1.10, USBLC6-2SC6 **SGD?** 0.74 (both footprints in the library). ULPI: USB3300-EZK **SGD** 2.37 (KiCad symbol); USB3320C-EZK **SGD** 2.98 (no symbol). RMII: LAN8742A-CZ **SGD** 2.10 (`Interface_Ethernet:LAN8742A`); Würth 7499010211A MagJack **SGD** 11.96 / 11.761 (`Connector:RJ45_Wuerth_7499010211A`). PHY crystals: unsourced. Microchip's pages returned 403. **Area** (`courtyards-w.txt`): FS 95.1 + 14.5 mm²; ULPI QFN-32 39.2 mm² (USB3300) or QFN-24 27.0 mm² (USB3320C); RMII QFN-24 27.0 mm² + the MagJack at 420.7 mm² | FS: 2. ULPI: 12, needing SPI1, CRSF_RX and I2C2 to move. RMII: 11, needing SPI1 and CRSF_TX to move | FS: **extra**, needed for J1; HS and RMII: **defer** |
| ↳ streaming | FS at 12 Mbit/s. In practice about 1 MB/s bulk [inferred, not cited]. That suits SUMP's capture-then-upload model (W6), not real-time streaming of 16 channels above about 0.5 MS/s [inferred]. ULPI and RMII would want a reference plane (§10.2's four layers) [inferred] | | | | |
| W4 protocol capture | SPI as a receive-only slave (`COMM = 10`, RM0433 §50.4.5: "MISO output is disabled"). Two instances sniff MOSI and MISO, one with `IOSWP` [inferred]. USART receiver only (`RE` without `TE`, §48). Timer input capture with DMA. **I2C: RM0433 chapter 47 has no monitor, listen or sniff mode** (checked, L76288–78755). A target that matches its address ACKs it and, unless `NOSTRETCH`, stretches SCL (§47.4.8), so an I2C peripheral cannot observe without taking part. **I2C is sniffed through W2's path** | No (needs capture and target roles) | Uses W1 or J5–J8 | SPI slave pins on a spare SPI | **extra** (firmware plus E5 and target roles) |
| W5 stimulus | I2C target up to Fm+ 1 MHz; `NOSTRETCH` forbids `SBC` and puts the data deadlines before the ninth SCL. ES0392 2.19.8: OVR is not set on underrun. SPI slave: receiver ≤ 100 MHz, **transmitter ≤ 29–31 MHz** (DS12117 Table 194, rev V). CRSF source: the serial port's TX (expressible today). DShot reader: input capture at 4.17 ns per tick at 240 MHz, about 400 ticks per DShot600 bit [computed] | Partly: the CRSF TX only | — | J5 (TIM1, capture-capable), J6, J8 | **extra** |
| W6 host protocol | **SUMP, as sigrok's `ols` driver speaks it, over USB CDC.** sigrok.org documents the command set (reset 0x00, run 0x01, ID 0x02, XON/XOFF; 5-byte 0x80–0x82, 0xc0–0xce) for up to 32 channels, so 16 fit. The driver ships in stock PulseView 0.4.2. sigrok-pico's protocol is in neither PulseView 0.4.2 nor sigrok-cli 0.7.2 (research-W-parts). The "1ALS" ID reply and the metadata tokens are to be checked in libsigrok's source before building | Firmware only | — | — | **extra**, needed for J1 |

**What the witness needs on the board now, so it is not a respin.** A 16-pin port kept whole (port G), USB FS on PA11/PA12, J5's TIM1 pins and J8, all of which V5 already leaves free. ULPI or RMII would mean re-pinning SPI1 and CRSF first. That is a pinning change, not measured here.

### 10.2 S2 at four layers [measured]

The `-4l` variants are each 2L variant with `board` replaced by `four-layer.json`'s (S-P-P-S, In1.Cu `GND`, In2.Cu `VDD_3V3`). The relaxation diff names that `board` change and nothing else.

| Variant | five commands | parity | support | extract | Outline | Router: segments / track / vias / rip-ups | Renamings |
|---|---|---|---|---|---|---|---|
| V0-4L | 0 ×5, no finding | 0 | 0 | 0 | 44.10 × 48.50 (same) | 393 / 414 mm / 104 / 1 (2L: 549 / 597 / 101 / 18) | 20/20 |
| V1-4L | 0 ×5 | 0 | 0 | 0 | 61.75 × 107.40 | 635 / 815 mm / 143 / 1 | 20/20 |
| V2-4L | 0 ×5 | 0 | 0 | 0 | 89.50 × 107.40 | 797 / 1282 mm / 200 / 2 | 20/20 |
| V3-4L | 0 ×5 | 0 | 0 | 0 | 113.10 × 107.60 | 889 / 1675 mm / 217 / 4 | 20/20 |
| V4-4L | 0 ×5 | 0 | 0 | **1**: XEXT 53.3/57.8 pF > 50 | 113.10 × 107.60 | 874 / 1715 mm / 219 / 4 | 20/20 |
| V5-4L | 0 ×5 | 0 | 0 | 0 | 100.05 × 107.60 | 863 / 1448 mm / 213 / 2 | 20/20 |
| V6-4L | 0 ×5 | 0 | 0 | 0 | 100.05 × 107.60 | 870 / 1445 mm / 206 / 18 | 20/20 |
| V7-4L | 0 ×5 | 0 | 0 | **2**: `TranslatorPinUnbounded` | 100.05 × 118.95 | 1156 / 2084 mm / 286 / 55 | 20/20 |

- **Adversarial (§5 of the WO) at four layers:** each `-4l` variant's `board.rules` equal V0's (200/200/300/600 µm). Only `copper_layers`, `layers` and `stackup` differ, so none passes by a looser rule (`results/relaxations.txt`).
- **Every loaded variant routes at both layer counts,** so the amended §7 clause (report a loaded board that will not route at 2L) did not fire.
- **Four layers cut track by 14–31 % (V0 31 %; the loaded variants 14–25 %) and rip-ups to almost none,** with the same outline (the placer does not read the layer count, as predicted).
- **SNVSAN3F item 5's "not met" is gone** from every 4L power variant's `power.conditions`.
- **`extract` hardly moves:** I2C bounds within 4.5 pF of 2L (both ways), SPI unchanged or lower. **My prediction was wrong** (bounds rise, SPI fails at 4L): the plane's cap is offset by shorter routes.
- `findings == []` in every gate for every 4L variant.

### 10.3 S4, the cost of three configurations

Every price is a page read on 2026-11-16 or arithmetic on one. A blank in a figure is on Scotty's list (§10.5).
- **(a)** V0 at 2L, the lean probe.
- **(b)** V6 at 2L, the largest variant passing every gate: the WO's "loaded board". **Not V7:** V7's IMU is a twin stand-in (EOL, "never bought"), so it would be priced for a part nobody orders.
- **(b) and (c) are a floor.** V6 lacks the hot-plug capacitor (E0), the flight sensors (`4220b`), the test pads (E1), the reset switch (E2) and the witness parts. Each adds parts, and may add outline and so fabrication cost. They are listed in `sources/bom-per-board.md`.
- **(c)** V6-4L.

| | (a) lean, 2L | (b) loaded, 2L | (c) loaded, 4L |
|---|---|---|---|
| Outline, area [measured] | 44.10 × 48.50 mm, 21.4 cm² (3.315 in²) | 100.05 × 107.60 mm, 107.7 cm² (16.687 in²) | the same as (b) |
| **OSH Park**, rate read at source (2L $5, 4L $10 per in² per set of 3); computed. 5 boards = 2 sets (6); 10 = 4 sets (12), sets of 3 assumed | **US$33.15 / US$66.31** | **US$166.87 / US$333.75** | **US$333.75 / US$667.49** |
| Second fab (tried: JLCPCB, PCBWay, Aisler 404/redirect loop, PCBgogo price page 404, Eurocircuits calculator 404, Seeed Fusion quote by email only) | unknown: interactive quote only (Scotty's list) | unknown, as (a); PCBWay's published $5/10 pcs promo is ≤ 100 × 100 mm, which (b) exceeds | unknown, as (a) |
| **Assembly, JLCPCB Economic** (read at source) | $8.18 setup + $1.53 stencil per order; 241 pads × $0.0016 = $0.39 per board [computed; pads as joints, inferred]; extended-part fees $3.07 each, at most 12 unique part types; the basic/extended split is on Scotty's list | $8.18 + $1.53; 356 pads × $0.0016 = $0.57 per board. Through-hole: the XT30's 4 TH pads (2 pins and 2 unnumbered mechanical pads, read in the emitted board). At Standard's hand rate, $0.0164 per joint × 4, plus $3.58 per order (Economic's rate not read). Extended-part fees: $3.07 each, at most 29 unique types | as (b); Economic's eligibility at 4L not stated (Scotty's list) |
| **BOM per board, priced lines.** Line by line, with each unit price, currency as shown, URL and date: `sources/bom-per-board.md`. Generic 0402 MPNs are the spike's picks. The MCU line was re-read: SGD? 29.15, one tier, status not shown | **SGD 52.89** for 28 of 34 rows: MCU 29.15, BME280 5.97, TMP117 8.63, W25Q64 6.01, 2 × SM04B-GHS 1.06, SM06B-SRSS 0.93, 17 × 100 nF 0402 0.58, 3 × 10 k 0402 0.42, 4.7 µF 0402 0.14. Unknown: 2 × 1 µF and 2 × 2.2 µF 0402 (obsolete or not found), 2 × 2.2 k 0402 (not searched) | **SGD 95.86** for 54 of 71 rows: (a)'s lines plus 2 × LMR33630 7.92, 2 × IHLP5050 5.84, 2 × GCM32EC71H106 2.74, TLV75518 0.58, 1 more SM04B and 3 × SM06B-GHS 2.54, 4 × GRM32ER61C226 10.32 and 4 × GRM32ER71A226 11.48 (both **"Not For New Designs"**), 3 × GCM188 100 nF 0.81, 2 × GCM188 220 nF 0.46, 2 more 10 k 0402. Unknown: 4 × GRM188R71C105KA12 (**obsolete** at digikey.sg; a `power-parts.json` row), 4 × TNPW0603 values, 2 × CRCW0603, 1 µF/2.2 µF/2.2 k 0402s, XT30PW-M | as (b) |
| Witness add-on (W1 16-channel, W3 USB FS), not in any variant | — | SGD 13.42: 2 × SN74LXC8T245 4.82, 2 × TPD8E003 4.94, header **SGD?** 1.82, USB4105 **SGD?** 1.10, USBLC6 **SGD?** 0.74; series resistors unpriced | as (b) |
| Engine work orders before ordering [sizes inferred] | **none.** V0 is `main`'s reference, and `verify --fab-out` (P3) emits its package. E1 (test points, S–M) is optional and would make it a better probe | E0 hot-plug draw (S); E1 test points (S–M); E2 BOOT0/reset (S); `4220b` (L, as written); W1 capture-port role and translator rows (M); W3 USB FS, `4300e1` (M–L); W4/W5 capture and target roles (E5 and more, M–L); the W2/W6 firmware (Rust, J1's application, L) | as (b); four layers add no engine work [measured: every gate passes] |
| **Calendar to the order** [inferred from this project's cadence: several work orders merged on 2026-11-16 alone] | **0 engine days;** the order is Scotty's purchase | **about 1–2 weeks** for E0–E2 and `4220b` with two builders in parallel. The witness's own work orders (capture role, USB FS, firmware) take about 2–4 weeks more, but they need not precede the order if W1's port and USB FS are on the copper | as (b) |

**Not summed across currencies.** Fab and assembly are USD; parts are SGD; no conversion is claimed.

### 10.4 S3, the path: recommend (a) now, and the loaded board as Sentinel-03 at four layers, (c)

| Path | Teaches before Sentinel-02 | Does not teach | When [inferred, except fab days read at OSH Park] |
|---|---|---|---|
| **(a)** V0 now; the loaded board later as Sentinel-03 | On our own copper, as emitted: <ul><li>programming by SWD;</li><li>boot and the register image read back over SWD (3302's bench rows, now on our board);</li><li>`BOR_LEV`;</li><li>SPI1 with the `MODF` fix and the flash's JEDEC identity;</li><li>I2C to both sensors;</li><li>the router's and support gate's output working.</li></ul>**The I2C rise time against `extract` needs hand-probing the 0402 pull-up pads:** V0 has no test points (T4), and no NRST on J2 (no connect-under-reset). It never builds a power tree. Its scar unholds `4400b`, `4400c`, `4500` and `4220b`, which the work order holds "until Sentinel-01's scar" | The buck and power tree; the flight sensors; the witness; **and the 2L buck against 4L comparison of §5.** On this path the 4-layer ruling for Sentinel-02 rests on TI's SNVSAN3F and on Sentinel-03's own 4L measurement, not on a measured 2L/4L contrast | Order: now (0 engine days). Fab: 9–12 days (OSH Park 2L). Sentinel-03: after E0–E2, `4220b` and the witness's port and USB (about 1–2 weeks of engine work), then 9–14 days (OSH Park 4L) |
| **(b)** V6 at 2L | The power tree on our copper at 2L, which is the §5 comparison point; the flight sensors after `4220b`; the witness | The buck in Sentinel-02's own stackup class: SNVSAN3F item 5 is "not met" here. ULPI and RMII are poor fits without a plane [inferred] | Order: after 1–2 weeks of engine work; fab 9–12 days; US$166.87 per 6 |
| **(c)** V6 at 4L | The power tree, sensors and witness in Sentinel-02's stackup class; every gate measured passing at 4L, SNVSAN3F item 5 no longer flagged | As (a), it has no 2L buck to compare. Its first copper waits for the engine work | Order: as (b); fab 9–14 days; US$333.75 per 6 |

**Recommendation: (a) now, then (c) as Sentinel-03.**
- **(a) is first copper in under two weeks for US$33**, with no engine change. What it does test is exactly roadmap §3's reason for Sentinel-01 first. Its scar releases the four held work orders.
- **(c) over (b)** for the loaded board: it teaches the power tree where Sentinel-02 will run it.
- **What the recommendation gives up is §5's 2L-against-4L buck comparison.** If Scotty wants that contrast as evidence for Sentinel-02's four layers, the cheapest way to buy it is a few (b) boards beside (c) at Sentinel-03's order, US$166.87 per six. (b) as the only loaded board is dominated by (c).
- **If only one board is wanted, take (c).** It delays first copper by the engine work.

### 10.5 For Scotty to fetch (pages Furnace could not use)

1. **Second-fab quotes for (a), (b) and (c) at 5 and 10 boards:** JLCPCB https://cart.jlcpcb.com/quote, PCBWay https://www.pcbway.com/orderonline.aspx, Aisler (404 and redirect loop from here), Eurocircuits (calculator 404), PCBgogo (price page 404) or Seeed Fusion (quotes by email, 2 working days).
2. **OSH Park:** whether Prototype copies come in other than sets of 3; area rounding; copper weight.
3. **JLCPCB assembly:** Economic eligibility by size and layers; whether $3.07 is per unique extended part; basic or extended for each BOM line (https://jlcpcb.com/parts).
4. **Parts:**
   - AMASS XT30PW-M (LCSC);
   - live replacements and prices for GRM188R71C105KA12 (obsolete; a cited `power-parts.json` row), GRM155R61A105KE15D (obsolete), a 2.2 µF and a 2.2 kΩ 0402;
   - TNPW0603 at 102 k, 25.5 k, 105 k and 46.4 k; CRCW0603 at 10 k and 100 k.
5. **Maker lifecycles behind sign-in or 403:**
   - Winbond W25Q64JV (already held by Scotty's 4211 evidence);
   - Microchip USB3300/USB3320C and LAN8742A, with their crystal and reference-clock needs;
   - ST AN2606 (the bootloader's pins), STM32H753ZI, USBLC6-2SC6;
   - GCT USB4105.
6. **The W2 sample rate:** a timer-paced DMA from `GPIOD->IDR` on the Nucleo at 8 and 16 bits, raising the rate until samples drop. No ST document gives it.
7. **Murata's own lifecycle** for GRM32ER71A226KE20 and GRM32ER61C226KE20 (digikey.sg says "Not For New Designs"); both are `power-parts.json` rows.

### 10.6 Added questions

- **Q6.** The path: (a) now and Sentinel-03 at four layers (recommended), or (c) only?
- **Q7.** The probe header: 16 channels on 2 × 12 with a ground per pair, or on 2 × 10 with a ground per four? And which port for the 16 inputs? **Port G is recommended:** port D carries FDCAN1's PD0/PD1, kept free for J2. Port G costs RMII's alternate TX pins PG11–PG14; PB11–PB13 remain.
- **Q8.** Distributors list two lifecycle states for `power-parts.json` rows (GRM188R71C105KA12 obsolete; GRM32ER*226KE20 "Not For New Designs"), and nothing in the engine judges a power part's lifecycle. Should `4210`'s `accept_lifecycle` extend to the power parts table? That would be its own work order.
