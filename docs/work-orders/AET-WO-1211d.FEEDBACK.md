# AET-WO-1211d — FEEDBACK: pin devices drawn (DRV8876PWPR, DRV8833PWPR)

**Branch:** `wo/aet-1211d-pin-devices-eda` from `main` `25203c58`; `main` `69c77c96` (docs), then `adb8f61a` (4240c, 1213) merged. **Builder:** Claude (Opus 5.5 orchestrating). Four subagents, each report checked at source before use:
- an Opus subagent authored the DRV8876 symbol and footprint;
- a Sonnet subagent ran the D4 spike;
- a fresh Opus reviewer ran the adversarial pass;
- a second fresh Opus reviewer reviewed D4's table.

Written 2026-11-20.

**Status: READY.** **The answer, in six lines:**
- **D2 (DRV8833) is drawn and passes ERC, DRC (`--spec`, with parity), `check support` and `extract` in the image, every findings list empty** (the amended acceptance, ruled S1). It is the committed D2 with every bridge input on a GPIO output, as ruled 18:45Z.
- **D1 (DRV8876) is refused `PassiveValueUnstated` for CVM2 alone** (ruled S1: SLVSDS7B states no bulk value). It is drawn up to that refusal. In a scratch copy with CVM2 stubbed, its authored symbol and footprint pass ERC, DRC with parity, and support.
- **No existing KiCad byte moved:** the 31 committed specifications, branch against `main` `25203c58`, are byte-identical at every point measured (259 files).
- **Symbols and footprints:**
  - DRV8833: our own symbol (Q3: AISEN and BISEN `passive`), on KiCad 10.0.7's PWP0016C footprint vendored byte for byte;
  - DRV8876: our own symbol and PWP0016A footprint, every pin and pad cited and held to a table in a test.
- **`PinDeviceNotDrawn` is lifted for the two parts and kept for any other.** `KNOWN_PARTS` names them, so `validate`'s `PartNotDrawable` lifts too.
- **4600 D4, measured:** the six device arms written as six table rows and one interpreter emit byte-identically on all 31. Proposed (§4), not built: it is not free.

## 0. How it was made

- **Predictions first:** `spike/1211d/PREDICTIONS.md`, committed at `661ff706` before any run.
- **Rulings** (bus `~/.cache/aet-bus/1211d.md`):
  - S1 (b) and Q1, Q2, 18:18Z;
  - Q3 (a) and the D2 shape, 18:45Z;
  - Q4 and A1/A2 to AET-WO-1211e, 19:06Z.
- **The measurement:** `spike/1211d/measure/sweep.sh` (`validate`, `solve` and `emit` on every committed specification, one binary) and `same.sh` (byte for byte; each command's printed JSON with its own output directory's path normalised).
- **The drawings:** `spike/1211d/draw/` emits at library level (CLI `emit` refuses a pin device until 1211b, ruled Q2). `kit/img.sh` + `kit/judge.sh` run the CLI's four judges in the tools image, under capped-run at 8 GiB with swap 0 read back. `kit/d1-stub.sh` draws D1 past its refusal, in a scratch copy only.
- **The datasheets:** the held SLVSDS7B and SLVSAR1E (`~/.cache/aet-1211/ds`, sha256 in the parts rows). Package pages were rendered and read.

## 1. The null sweep (§1.1)

| point | result |
|---|---|
| after vendoring (`76e8cafe`) | identical, 259 files |
| after the arm (`89b08ec9`) | identical, 259 files |
| after Q3 (`1a7bcf29`) | identical, 259 files |
| at READY (`main` merged) | identical, 259 files (§8) |

N1–N3 hold:
- no committed specification declares a pin device on a board it draws;
- `own_nets` adds nothing for a specification with none;
- `validate`'s output is unchanged on all 31.

The adversarial reviewer re-ran the sweep against its own build of `main`: identical.

## 2. What was built

- **`crates/aethelgard-eda/src/schematic_pin_device.rs`.** One `Arm` per part: symbol, footprint, thermal pad, and the passives the datasheet requires that the row does not hold, each with value, rating and page. Everything else comes from the row (pin numbers and kinds) or the declaration (supply, straps, signals and their pulls, the IPROPI resistor and reservoir, bridges):
  - **straps:** a tie goes straight to its rail (a 10 kΩ strap would select another IMODE level: SLVSDS7B p. 5's bands start at 18.6 kΩ); a declared strap resistor sits on its own net;
  - **declared resistors and the reservoir** come from their power-parts rows (0603). The reservoir is anchored by the MCU's ADC pin, as a battery sense's is;
  - **a `power_in` pin only a capacitor joins** (DRV8833 VINT) is drawn as a rail, so it carries a `PWR_FLAG`;
  - **every numbered row pin ends drawn.** An undeclared open-drain output is a no-connect; anything else is refused;
  - **each required capacitor** is anchored by its first pin end, within 2.0 mm (an engine choice, as the power tree's are). Anchoring CFLY to both CPH and CPL left `U2_CPH` unroutable (§5).
- **New refusals:**
  - `PassiveValueUnstated {reference, part, passive, cite}` (S1);
  - `BridgeOutputNotOnConnector {reference, pin, net}`;
  - `PinDeviceNotDrawable {reference, part, why}`: an undeclared current output, a pin that would float, or a bridge output named for a rail.
  - `PinDeviceNotDrawn` stays for a part with no arm.
- **`schematic.rs`:**
  - claims the pin devices' references before any emitter-numbered one;
  - puts each device ADC input on its MCU pin (a battery sense in `adc_inputs` is still the power tree's);
  - draws the devices before the connectors;
  - adds `Drawing::pins`, a test reader.
- **`aethelgard_ir::names::own_nets`** (Q1): each pin device's passive-pin nets and resistor-strap nets. Nothing else in the IR.
- **`aethelgard-kicad-lib`:** KiCad's PWP0016C footprint, vendored and proved against blob `b4708f1c` at tag 10.0.7; our own DRV8833 and DRV8876 symbols and PWP0016A footprint under `data/aethelgard/`, each re-saved by `kicad-cli … upgrade --force` (a second save changes no byte).
  - KiCad's own `Driver_Motor:DRV8833PWP` (blob `ad2a6eed`) was vendored first and then dropped by Q3: it types AISEN/BISEN `bidirectional`, and ERC refuses that on the pins SLVSAR1E p. 10 grounds.
  - None of KiCad's nine HTSSOP-16-1EP variants at 10.0.7 has PWP0016A's (3.3) × (3.3) mask opening (all nine read).
- **Tests:**
  - `cli/tests/pin_devices_drawn_tier1.rs` (9);
  - `eda/tests/own_driver_parts.rs` (10: both symbols' pins to p. 3 tables; PWP0016A's lands, exposed pad, mask and paste to pp. 45–47; exactly AISEN and BISEN depart from TI's types);
  - the agreement tests (`parts_arms_tier1`, `the_arms_are_the_known_parts`), `strict_fence`, `workspace_shape` and `emit_tier1` updated.

## 3. Decisions, rulings and findings

| # | what | where | status |
|---|---|---|---|
| S1 | DRV8876 CVM2 (VM bulk) has no stated value: SLVSDS7B Table 1 p. 10, p. 3, §9.1 p. 31 | arm | **ruled (b)**: refused by name. §7.1 row: `pin_devices[].bulk {reference, part, source}`, a cited capacitor row rated above VM's maximum, for the next IR WO |
| Q1 | own nets in `own_nets` | IR | **ruled**: accepted, nothing else in the IR |
| Q2 | library-level emit, CLI judges | tests | **ruled**: accepted; CLI `emit` stays refused until 1211b |
| Q3 | KiCad's DRV8833 symbol fails ERC on grounded xISEN | kicad-lib | **ruled (a)**: our own symbol, AISEN/BISEN `passive` (SLVSAR1E p. 10: "If current control is not needed, the xISEN pins should be connected directly to ground"; TI types them `IO`, and this departs from it), no ERC exclusion |
| F-D2 | D2 as committed is refused `PartLimitUndeclared` inside `solve` (SLVSAR1E states no input PWM maximum), so it cannot be drawn at any level | IR | **ruled**: the drawn D2 has GPIO bridge inputs; the committed D2 stays undrawable until the limit is declared from a held source. §7.1 row; no value invented |
| Q4 | DRV8833 bulk: §9.1 p. 14 describes a local bulk in S1's words; TI's own typical application (Fig. 7 p. 12) draws one VM capacitor, C4 10 µF; p. 3 and §10.1 p. 15 state 10 µF minimum | arm | **ruled (19:06Z): the lean.** D2's one 10 µF is the datasheet's stated VM capacitor, so D2 stays drawn. §9.1's system-level bulk is **not judged** (R2). It differs from D1's CVM2 because SLVSAR1E states a value and SLVSDS7B states none |
| A1 | **a DRV8833 nFAULT pulled by a declared resistor to VMOTOR (6.3 V) onto PA1, an FT pin (DS12117: absolute maximum min(VDD + 4.0, 6 V)), validates and passes all four gates** | IR `validate` | **ruled (19:06Z): to AET-WO-1211e**, row R3 |
| A2 | **DRV8876 VREF tied to GND or a low domain validates and passes all four gates.** ITRIP = V_VREF / (A · R) (SLVSDS7B p. 13 Eq. 3): zero at GND, ≈ 0.33 A at 0.5 V against a 1.9 A full scale | IR `validate` | **ruled (19:06Z): to AET-WO-1211e**, row R4 |
| A3 | a bridge output declared on `GND` validated and drew (ERC and DRC failed it, nothing named it) | arm | **fixed**: `PinDeviceNotDrawable`, test, mutant G8 |
| A4 | capacitor ratings are text in the value; every arm capacitor is 0402; a 10 µF 0402's DC bias is unjudged | arm | finding, ruled 18:18Z; VM-rated now reads `>6.3V`, a minimum. Folded into D4 (§4) |
| A5 | no thermal vias or wide copper for VM/OUT/PGND; no gate judges either (SLVSDS7B §10.1 p. 32; SLVSAR1E §10.1.1 p. 15). TI draws (Ø0.2) vias under PWP0016A, below the boards' 300 µm via drill | eda | row R6 |
| A6 | D1's IPROPI reservoir slows current regulation (R2·C1 = 150 µs; SLVSDS7B p. 13: "large capacitor values may slow down the response time") | 1211a fixture | row R7 |
| F1 | an undeclared IPROPI validates; SLVSDS7B p. 13's alternative (IPROPI to GND, VREF above GND) cannot be declared, so the arm refuses it | IR | row R5 |
| F2 | an undeclared nFAULT validates and is drawn open (a no-connect) | — | as designed: the part asks for no pull |
| F3 | the VM capacitors are not counted in `decoupling_picofarads` (`decouples: false`: their value text does not parse as a capacitance), so a droop on VM is judged without them, on the safe side | eda | finding |
| F4 | `ir/src/drawn.rs` L69's comment says no arm draws a pin device until 1211d, which is now stale | IR | outside the fence |
| F5 | IPROPI typed `passive` in the authored DRV8876 symbol, not `output` (p. 13 allows IPROPI tied to GND, and ERC refuses an output on a flagged rail); OUT1/OUT2 `output`, not `power_out` | kicad-lib | decision, cited in the test |

### §7.1 rows (for the roadmap; R1, R3 and R4 go to AET-WO-1211e as ruled 19:06Z)

| row | what | evidence | owner |
|---|---|---|---|
| R1 | `pin_devices[].bulk {reference, part, source}`: a cited capacitor row rated above the VM domain's maximum. **D1 is drawable with it**, and G6 (§5) becomes testable | S1; `d1_is_refused_for_cvm2_alone` | 1211e |
| R2 | the DRV8833's §9.1 system-level bulk (SLVSAR1E p. 14) is not judged: only the stated 10 µF VM capacitor is drawn (Q4). If R1's field lands, a DRV8833 may declare one too | Q4 | beside R1 |
| R3 | **A1: a pull's domain maximum against the MCU pin's tolerated input**, whatever the part row states. The board that passes all four gates: `spike/1211d/adv/nfault_to_vmotor.json` (D2 drawn, nFAULT pulled by R9 10 kΩ to VMOTOR 6 V ± 5 % onto PA1). Its judges are in `adv/judge/nfault_to_vmotor/` | ERC, DRC, support and extract pass, findings `[]` | 1211e |
| R4 | **A2: VREF on GND, or a VREF below the declared full scale with its band**, when IPROPI carries a resistor. The board that passes all four gates: `spike/1211d/adv/d1_vref_gnd.json` (D1, `VREF: {tie: GND}`, drawn with `kit/d1-stub.sh`). Its judges are in `adv/judge/d1_vref_gnd/`. `adv/d1_vref_low.json` (VREF on a 500 mV domain) validates and draws; it was not judged | ERC, DRC, support and extract pass, findings `[]` (with the stub) | 1211e |
| R5 | a tie for a current output (SLVSDS7B p. 13: IPROPI to GND with VREF above GND, regulation off); until then the arm refuses an undeclared IPROPI (F1) | `an_undeclared_nfault_is_open_and_an_undeclared_ipropi_is_refused` | IR |
| R6 | thermal copper: a gate for an exposed pad's thermal connection (vias, plane) and a power net's width (VM, OUT, PGND); the boards' 300 µm via drill against TI's (Ø0.2) | A5 | eda / rules |
| R7 | the IPROPI reservoir against current-regulation response (R2·C1 = 150 µs in D1) | A6 | 1211a fixture / IR |
| R8 | the committed D2 stays undrawable until the DRV8833's input PWM maximum is declared from a held source (ruled 18:45Z); no value invented | F-D2 | IR |
| R9 | capacitor ratings and footprints as rows, not value text (A4); every arm capacitor is 0402 today | A4; §4 | D4's proposal |

## 4. 4600 D4 — measured, proposed

**Measured** (`spike/1211d/d4/RESULTS.md`, `table-interpreter.patch`): the six device arms (BME280, BMP581, TMP117, W25Q64JVxIQ, ICM20948, ICM42688P) written as six table rows and one interpreter give **identical bytes on all 31 committed specifications** (259 files), and `parts_arms_tier1` (every part drawn) passes. **Adversarial review of the table** (asked 18:45Z): a second fresh Opus reviewer ran the patched and the base emitter on 274 inputs the committed specifications do not cover. Evidence: `~/.cache/aet-1211d/d4harness/`, `d4out-*`.
- **Inputs:** every I2C part at every address, off-table addresses, two or four of one part, references and own nets that collide (case variants too), unknown parts and parts on the wrong bus, each SPI part behind the TXU0304 crossing.
- **Result:** full part dumps, refusals (kind, message) and 87 full `emit` runs: **no divergence; 926 files, `diff -r` empty** (re-checked here).
- **The harness can fail:** two planted mutants made 451 files differ.
- **Hazards no input reaches today** (row-authoring risks, for the proposal):
  - an own-net capacitor's name is rebuilt by `format!`, not tied to its pin's own net;
  - an address role on an SPI row is now a run-time error, where it was a compile error;
  - a duplicate row is shadowed silently;
  - the I2C 100 nF decoupler becomes per-row data, so a row can leave it out.

**What a row cannot say as plain data** (each an interpreter feature):
1. the address strap's choice (`strapped`);
2. a strap through a resistor to the supply;
3. an own net closed by a capacitor (ICM20948's REGOUT);
4. translator substitution (`behind`);
5. a stand-in's value and note (W25Q64 on the W25Q32 symbol);
6. ordered passives;
7. pin order is draw order;
8. the bus kind.

**Proposal (not built: not free; +108 lines of rewrite of six arms, against an unchanged byte):**
- **One table of arm rows, in eda beside the symbol library.** Each row carries part, bus, symbol, footprint, pin→role map, required passives (value, rating, footprint, page) and an optional stand-in. The interpreter covers both the bus devices and this work order's pin-device arms (its `Arm` is already that shape).
- **Replace the source-scan test** with a rows-against-`KNOWN_PARTS` test.
- **Carry each required passive as a power-parts row** (rating, dielectric, DC bias, footprint) rather than value text, so A4 is judged.
- **Order constraint:** a row's order is the draw order. Any reorder is a bytes change and needs the sweep.
- **The proposal adds:** a test that each row's own-net capacitor names its own `Own` pin, and one that no part has two rows.
- **What a new part still costs beyond its row** (ICP-10111 has a row and no arm, 4600 F15):
  - its symbol and footprint (`library.rs`, `kicad-lib/src/lib.rs`, `workspace_shape.rs`);
  - `KNOWN_PARTS`, which the CLI's `validate` reads (deriving it from the rows changes `UnknownPart`'s and `PartNotDrawable`'s message bytes unless the rows keep today's order);
  - `aethelgard_ir::names::device_nets`, which restates the rows' own-net names;
  - `parts_arms_tier1`'s `ROWS_WITHOUT_ARM`;
  - for an IMU, `parts.json` `axes`;
  - for a twin-modelled part, `MODEL_SETS` and the twin's `Model`.
  Moving the table into the IR rows would let `device_nets` derive from it. That is a schema-epoch decision.

## 5. Mutants

(see `spike/1211d/RESULTS.md` §M; `mutants/result.json`) **11 of 12 killed.**
- The work order's three (a strap left floating, nFAULT's pull-up missing, a bridge output to no connector) are each killed by a named test.
- **G6 survives** (the pin device's ADC input left off the MCU): only a DRV8876 has an analog output, and D1 is refused at CVM2 before a test can read its MCU pins. It is held by the D1-stub board (MCU pin 13, PF3, on `DRV_IPROPI`) and becomes testable with 1211e (R1), or when CVM2 has a source (ruled 19:06Z).

## 6. Adversarial pass (§5)

A fresh Opus reviewer attacked `99ada513`, then `1a7bcf29` (evidence `~/.cache/aet-1211d/adv/`). Defects: A1–A6 above. Held:
- no byte moved (re-run against its own build of `main`);
- D1 and D2 pin by pin against SLVSDS7B p. 3 and SLVSAR1E pp. 3–4;
- every required capacitor between the right pins with the stated value;
- nSLEEP at reset: analog pin, internal pull-down, FT leakage 0.125 V against VIL 0.5 V; the JTAG pull-up pins excluded by 1211a;
- VINT as a flagged rail is right, and nothing else can join it;
- the PWP0016A footprint against pp. 46–47: every land, the exposed copper, mask and paste;
- two DRV8833s on one board pass all four gates;
- ERC on D2 hides nothing behind the two pre-Q3 warnings.

## 7. Predictions beside results

| # | prediction | result |
|---|---|---|
| N1 | emit byte-identical at three points | **holds** (four points) |
| N2 | no committed specification reaches either arm | **holds** |
| N3 | `own_nets` unchanged with no pin device | **holds** |
| A1 | D2 drawn passes ERC, DRC, support, extract | **holds after Q3** (KiCad's symbol failed ERC; our own passes) and after the D2 shape ruling |
| A2 | D1 refused CVM2 alone; with CVM2 stubbed the rest draws | **holds after one fix**: the first stub draw was `Unroutable` (CFLY's two-pin anchor) |
| A3 | vendored files match their blobs; authored ones canonical under `upgrade --force` | **holds** |
| A4 | each WO mutant killed | **holds** |
| D4a | the six arms as rows, byte-identical | **holds** (259 files) |
| D4b | the features a row cannot hold | **holds**, plus three more (strap-to-supply resistor, own-net capacitor, bus kind) |
| D4c | not free; propose | **holds** |

## 8. Gates, fence, reduction pass

(pending)
