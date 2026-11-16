# AET-WO-3400 — FEEDBACK

Builder: Claude (strong tier), worktree `~/aethelgard-3400`, branch `wo/aet-3400-bench-board`, cut from `origin/main` `3dcd242`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #183, head `8d84392`) and is not committed here.

Labels used throughout: **[measured]** (run here, output in `spike/3400/results/`), **[read at source]** (a file, a datasheet or a catalogue, cited), **[inferred]** (reasoning, not checked).

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
