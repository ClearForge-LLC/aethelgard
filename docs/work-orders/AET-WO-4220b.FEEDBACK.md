# AET-WO-4220b — FEEDBACK (build)

**Branch:** `wo/aet-4220b-board` from `origin/main` `36e428b` · **Work order:** `docs/work-orders/AET-WO-4220b.md` on `main` · **Kickoff:** `~/.cache/aet-4220b-kickoff.txt` (released 2026-11-17 with Sentinel-03 as the first consumer, before Sentinel-02) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build, full adversarial pass.

Each answer is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any measured run, and are not edited afterwards.

## 0. Premises re-checked against `main` `36e428b`, and readings taken before the predictions

Every premise in the work order's grounds holds; none is a stop.

- **R1 · The models slice is merged** [read]. `ICM42688P` and `BMP581` are `active` rows in `crates/aethelgard-ir/data/parts.json`, now schema 6 (it was 4 at `78f700d`; `4230`, `4211` and others moved it). Both rows' `pin_capacitance.max_femtofarads` is `null`, with the citation stating the ICM's "< 10" pF sits in the TYP column (DS-000347 Table 4, p. 14) and that Bosch prints no figure (BST-BMP581-DS004-13 Tables 4, 5, 6, 17). The fixture `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` (specification 12) puts U7 BMP581 on I2C `SENSORS` and U5 ICM42688P on SPI `IMU`.
- **R2 · What blocked the board path is merged** [read]: `4300d2` (#149, `dbf049e`), `1310` within `4300g` (#159, `4e821ed`), `1311` (#168). The strict reader, the router (`route/read.rs` `keepout`) and `extract` (`FOOTPRINT`) admit a footprint's keepout zones: each one rectangle on `F.Cu` or `B.Cu`, five flags, no net, several per footprint. A footprint carrying one is never turned (`pcb.rs` ~L990).
- **R3 · The held inputs are intact** [measured]: AN-000393 v2.4's sha256 is `be0aa48f…de55b`, as the grounds state. The 4220 render directory is gone; I re-rendered BMP581 pp. 68–69, DS-000347 pp. 20 and 53–55, and all 28 pages of AN-000393 in the tools image (ghostscript, `~/.cache/aet-4220b/render/`), and read the land patterns from the images.
- **R4 · The D10 ruling's text** (comment 6890080737, read with `gh api`): "the **specification** may declare a pin-capacitance value as a stated condition: a `pin_capacitance.declared` with its reason. Extraction then bounds the line under that named condition, printed in the verdict, rather than refusing. It is never silent and never promoted to 'cited'." D5: our own symbols, "under a separate directory in `aethelgard-kicad-lib` from the vendored KiCad files", `MIT OR Apache-2.0`; "footprints come from KiCad's generic LGA library where one matches the datasheet's land pattern exactly; otherwise they are authored the same way".
- **R5 · No KiCad 10.0.7 footprint matches either land pattern exactly** [measured, the tools image's `/usr/share/kicad/footprints/Package_LGA.pretty`]. LGA-14 3 × 2.5 mm P0.5: `Bosch_LGA-14_3x2.5mm_P0.5mm` has 0.675 × 0.25 mm lands at ±1.2625/±1.0125; `LGA-14_3x2.5mm_P0.5mm_LayoutBorder3x4y` (ST) has 0.625 × 0.35 at ±1.1625/±0.9125. AN-000393 asks for 0.475 × 0.25. LGA-10 2 × 2 mm P0.5: `ST_HLGA-10_2x2mm_P0.5mm_LayoutBorder3x2y` has 0.425 × 0.35; Bosch asks for 0.3 × 0.325. So both are authored. Two independent cross-checks fall out: the ST LGA-14 puts its centres where my arithmetic does (R6), and the ST HLGA-10 numbers its pads in the order I read for the BMP581 (R7).
- **R6 · The ICM-42688-P's land pattern** [read, DS-000347 p. 54 Fig. top and bottom view, p. 55 table; AN-000393 v2.4 §2.2 Fig. 1, p. 6]:
  - Top view: body E = 3.0 (x) by D = 2.5 (y); pin 1 at the left column's top (A1 index); left column 1–4 downward, bottom row 5–7 left to right, right column 8–11 upward, top row 12–14 right to left.
  - Centres: side columns at x = ±1.1625 (the boxed "2X 1.1625"), top and bottom rows at y = ±0.9125 ("2X 0.9125"); pitch e 0.5, D1 1.5 and E1 1.0 BSC, SD 0.25.
  - Arithmetic check [inferred, consistent]: E/2 − 0.10 − L/2 = 1.5 − 0.1 − 0.2375 = 1.1625 and D/2 − 0.10 − L/2 = 1.25 − 0.1 − 0.2375 = 0.9125, with the "(0.10)" edge references and the nominal lead length L 0.475.
  - Lands: AN-000393 Fig. 1: "PCB land length = LGA solder pin length", "PCB land width = LGA solder pin width": 0.475 × 0.25 (L and W nominal, p. 55), long side radial.
  - AN-000393 §2.1: "Avoiding vias, traces, and copper pour under IMU device"; §2.4 Fig. 5: traces leave the pads radially.
- **R7 · The BMP581's land pattern** [read, BST-BMP581-DS004-13 p. 68 Fig. 30 (bottom view) and p. 69 §8.2, Fig. 32]:
  - Fig. 30, bottom view, pin-1 corner top right: top row 8, 9, 10 left to right; right column 1 (upper), 2; bottom row 5, 4, 3 left to right; left column 7 (upper), 6. Mirrored to the top view: left column 1 (upper), 2; bottom row 3, 4, 5 left to right; right column 6 (lower), 7; top row 8, 9, 10 right to left. Counter-clockwise from pin 1, as p. 46's top view gives it (rotated 90°).
  - Centres: the boxed 1.525 (side columns, centre to centre), 1 (outer row pads) and 0.5 (side pads) give x, y = ±0.7625 for the ring and 0.5 pitch.
  - Lands: §8.2 "a land pattern with a size of Footprint +25 µm on each side": package pads 0.25 × 0.275 (6×) and 0.275 × 0.25 (4×), so lands 0.3 × 0.325 and 0.325 × 0.3, as Fig. 32 prints "0.325 × 0.3". Which side is radial is not labelled; it is fixed by arithmetic [inferred, consistent]: 1.525/2 + 0.325/2 = 0.925 = 1.0 − 0.075, Fig. 32's "0.075 ± 0.08" from the body edge; and the package's 1.525/2 + 0.275/2 = 0.9 = 1.0 − 0.1, Fig. 30's "0.1 ± 0.08". The other order would leave 0.175 mm between row pads, below §8.2's "at least 200 µm distance between the pads".
  - §8.2: "We do not recommend vias or traces under the BMP581." Body 2.0 ± 0.1 (Table 3: 1.90 to 2.10).
- **R8 · Fences shared** [read, `~/.cache/aet-bus/WHO.md`]: `names.rs` and the kicad-lib arrays are also 1312's; the kicad-lib `Cargo.toml` is 1801's. Notes sent to both inboxes before any edit.
- **R9 · Two findings for the architect** (not stops):
  - **Q-I versus §1.4.** `4230`'s ruling on Q-I (its FEEDBACK §11 table) made the ledger "register fields only … there are no `PIN_C` rows", after this work order's §1.4 was written; `ledger.rs` is the CLI's, outside this fence. I build no ledger row; `extract`'s own verdict names each declared term. Question Q1 below.
  - **The specification version.** The ruling puts the declaration in the specification, whose version is checked for equality (`lib.rs` L2745). So this work order takes specification 13 from `main`; 1312 takes 13 too, and the second to merge renumbers (both work orders say so).

## 1. Predictions (committed before any measured run)

| # | Prediction | Basis |
|---|---|---|
| P1 | Both footprints, hand-authored and re-saved by `kicad-cli fp upgrade --force`, pass the strict reader, `route::read` and `extract`'s closed lists with **no reader change**. | inferred (R2) |
| P2 | The Sentinel-02-shaped fixture emits; **ERC passes with no finding**; it routes on 2 layers with both keepouts respected; **`check drc --spec --schematic` passes with no finding of any kind** (no `lib_footprint_mismatch`, no `solder_mask_bridge`). | inferred |
| P3 | **`extract` passes** on it. The ICM's two counted pins (SCK and MOSI; MISO and CS are not bounded lines) and the BMP581's two (SDA, SCL) appear as declared terms of 10 000 fF each, never in `pins`. Each bounded line stays under 60 pF. | inferred |
| P4 | **Every existing fixture's emitted files and every gate's verdict are byte-identical** to `main`'s binary, apart from what the version bump moves: no emitted file holds the specification's version, so I predict **no emitted byte moves at all**, only verdict lines that carry a specification sha256. | inferred |
| P5 | The router never needs a change: the interior keepouts sit inside the pad ring, where no grid track fits anyway (pads 0.2–0.25 mm apart at a 0.6 mm track-plus-clearance need). **No §7 stop.** | inferred (R6, R7) |
| P6 | `TIER2_MIN` goes from 160 to **161** (one new tier-2 test: the fixture's board through every gate). Every CI shard stays under 20 minutes. | inferred |
| P7 | A pad moved off its cited dimension, a swapped pin, a removed keepout and a trace under the BMP581 each fail a tier-1 test or DRC; a declared term relabelled as a maximum, or dropped from a line's sum, fails a tier-1 test. | inferred |

## 2. Predictions against measurement

| # | Outcome | Measured |
|---|---|---|
| P1 | **Held.** Both footprints, re-saved by `kicad-cli fp upgrade --force` (KiCad 10.0.7, the tools image), pass the strict reader, `route::read` and `extract` with no reader change. The re-save is deterministic: two runs gave the same sha256. | measured |
| P2 | **Held, with two pin types changed first.** On the first ERC run the BMP581's INT (typed `output`) tied to GND was an error, "Pins of type Output and Power output are connected", and the ICM's INT2/FSYNC (typed `bidirectional`) a warning. D2 retyped them; then ERC has no finding, and DRC by the specification's rules with parity has no finding of any kind. | measured |
| P3 | **Failed as predicted; held after D1.** `extract` passed nothing at first: `IMU_SCK` routed 42.5 mm with 4 vias, and its bound of 31 658 fF exceeded the fixture's declared 30 000 fF. The declared terms themselves were as predicted: 10 000 fF each, in their own term, never in `pins`. After D1, `IMU_SCK` is 26 492 fF and `IMU_MOSI` 26 664 fF (both declared 30 000), and every line passes. | measured |
| P4 | **Held.** Every existing fixture was compared with `main`'s binary at `bdf7ac0` on `main`'s fixture, through `solve`, `emit` (all three files), `generate` (the tree) and `extract`. Every emitted KiCad file, solved topology, generated file and `extract` verdict is byte-identical once each specification's own sha256 is put back. Beyond that: `generate`'s report differs in the topology sha256 and the hashes of the eight files that carry the hash line. `imu-baro-1v8`'s `emit` refusal (`UnknownPart`, an ICP10111) differs in its list of known parts, which now names `BMP581` and `ICM42688P`. The committed `boards/sentinel-01` and three svd-hal goldens moved by their hash lines alone (`AETHELGARD_BLESS=1`); `solve_cli.rs`'s two digests and `order_2203.rs`'s three each return to `main`'s exactly with the old sha256 put back. Harness: `~/.cache/aet-4220b/ident/compare.py`. | measured |
| P5 | **Held.** No router change. The emitted board has no segment or via in any of U5's or U7's keepouts (the adversary's count: 698 segments, 120 vias). | measured |
| P6 | `TIER2_MIN` is 161. CI is in §6. | measured (CI below) |
| P7 | **Held.** 18 mutants, 18 killed (§5). | measured |

## 3. Decisions taken (forks inside the fence)

- **D1 · The fixture's IMU SCK is pinned to PB13 (P3).** The solver put `IMU_SCK` on PA12, across the MCU from MISO and MOSI (PB14, PB15), and its 42.5 mm route exceeded the 30 pF the fixture declares.
  - Raising the declared budget to 50 pF, Table 149's next row, would have passed. It would also keep `OSPEEDR` 00, at 16.6 ns against the 20 ns declared rise (measured). It was rejected: a line's capacitance is a gate value, not a lever (`3200` F1, `INV-07`).
  - The specification's own `sck.pin` is used instead. The bus is SPI2 on PB13/PB14/PB15, and the measured bounds are in P3.
  - The fixture's other users pass on it: `own_models_tier1`, `ledger_tier1`, and in the image `board_tier2`'s new test. The 4220 twin tests are CI's (§6).
- **D2 · Two pin types follow the datasheets' recommended connection, not the pin's widest function (P2).** BMP581 INT is `passive`: Table 28 note (a) allows GND "as long as the IRQ pin is not activated", and the bring-up leaves `INT_CONFIG.int_en` 0. ICM INT2/FSYNC/CLKIN is `input`: "Connect to GND if FSYNC not used". KiCad's own ICM-20948 symbol types FSYNC `input` and its GND-tied RESV `passive`. No interrupt is armed (`RUL-19` (5)).
- **D3 · Keepouts.** Each footprint carries two, read from the held pages:
  - **`F.Cu`, inside the ring of lands** (BMP581 ±0.6 mm; ICM ±0.925 × ±0.675 mm): no track, via or pour. Bosch §8.2: "no vias or traces under the BMP581"; AN-000393 §2.1: "Avoiding vias, traces, and copper pour under IMU device". A land's escape leaves outward, as AN-000393 Fig. 5 draws it. A 0.2 mm track cannot fit between lands 0.2–0.25 mm apart at a 0.2 mm clearance anyway.
  - **`B.Cu`, under the body at its maximum** (BMP581 2.10 mm square, Table 3; ICM 3.1 × 2.6 mm): no track or via, with pour allowed so a two-layer board keeps its ground plane whole. That pour is a reading (adversary H3): §2.1 does not name a layer.
- **D4 · Solder mask and paste follow the board's defaults.** Bosch §8.2 recommends no mask under the sensor, with a 20 µm horizontal clearance; AN-000393 §2.2 recommends none under the IMU, and Fig. 1 a land + 0.1 mm opening; §2.3 a 90 % stencil.
  - A whole-body mask opening exposes lands of different nets in one aperture, which KiCad's solder-mask-bridge check flags unless the footprint carries `allow_soldermask_bridges`, a token `RUL-03` refuses (read, not run).
  - A pad's own `solder_mask_margin` or paste ratio is outside `extract`'s closed pad list (`extract.rs` ~L652).
  - Each land therefore takes the gate's default opening. A fabrication-notes line, or the token for these two footprints, is the architect's (Q3).
- **D5 · Our own files sit apart from KiCad's.** They are under `aethelgard-kicad-lib/data/aethelgard/`, with a README citing each file. Keyed in `FILES` by their path in the gate's library directory (`symbols/Aethelgard_Sensor.kicad_symdir/…`), so `library::tables` needed no change. The crate's `license` becomes `CC-BY-SA-4.0 AND (MIT OR Apache-2.0)`. It now carries `LICENSE-MIT` and `LICENSE-APACHE`, held byte-identical by `package_licences.rs`, and CI's package step requires them (1801 notified, its fence).
- **D6 · The declaration's shape and home.** The D10 ruling puts it in the specification (`pin_capacitance.declared` with its reason).
  - Each I2C and SPI device row requires `pin_capacitance`: `null`, or `{"declared": {femtofarads, condition, source}}` (specification 13).
  - A cited maximum stays the parts table's (`4300c` D3), and a declaration beside one is refused (`PinCapacitanceAlreadyCited`).
  - The range is the table's own, 1 to 100 000 fF. Both texts follow the stackup sources' rule.
  - The fixture declares 10 pF for each sensor. The ICM's value is TDK's typical "< 10" pF. The BMP581's basis is UM10204's `Ci`, the I2C specification Bosch's interface follows, since Bosch prints no figure.
  - An I2C device that declares nothing is counted at UM10204's `Ci` as before; an SPI device with neither a cited maximum nor a declaration is refused as before.
- **D7 · The emitter's wiring**, cited in each arm.
  - **BMP581:** VDDIO and VDD on the supply, each with 100 nF (Fig. 26); SDO strapped by address, through the table's order like the BME280's; CSB to VDDIO; INT to GND.
  - **ICM-42688-P:** 4-wire SPI; 0.1 µF and 2.2 µF on VDD, 10 nF on VDDIO (Fig. 7); pin 7 and FSYNC to GND; RESV 2, 3, 10, 11 and INT1 open.
  - Bosch's 10 Ω inrush resistor (§6.2.5) is for supplies that ramp faster than 10 µs. It is not drawn.

## 4. Questions for the architect

- **Q1 · §1.4 against Q-I.** `4230`'s ruling on Q-I made the ledger "register fields only … there are no `PIN_C` rows", after this work order was written. `ledger.rs` is outside this fence. Built: no row; `extract`'s verdict names each declared term, and the conditions reach `FAB-NOTES.md` through `verify`. Should a declared pin capacitance be a `twin_limits` row (never blocking), a ledger row after all, or stay as built?
- **Q2 · How low may an SPI declaration go (adversary H2)?** A declaration is the specification's statement, printed with its condition, but nothing ties its value to that condition. On one byte-identical board, `IMU_SCK` fails with U5 declared at 10 pF against a 20 pF budget and passes when U5 is declared at 1 fF (measured by the adversary, `~/.cache/aet-4220b/adv/work/ext_flip*.json`). On I2C a declaration is floored at UM10204's `Ci` (A1, built). No SPI floor exists without a source: the parts table holds no typical values. Options:
  - accept as ruled, the condition being the claim;
  - hold the cited typical in `parts.json` and floor a declaration at it (a parts schema bump);
  - refuse a declaration below 10 pF on any bus.
- **Q3 · Mask and paste (D4).** Should these two footprints opt out of the solder-mask-bridge check by name, and the 90 % stencil be stated in `FAB-NOTES.md`? Or leave the defaults?
- **Q4 · `spike/3400/**` stays at specification 12.** It is 3400's merged record, outside this fence. Once this merges, its variants no longer validate on `main`, and 3401 has been told.
- **Q5 · Backlog row.** DS-000347 Table 6 gives SCLK rise and fall at most 16 ns (read note), and the fixtures declare 20 ns rise on SPI lines; no rule holds a device's input-edge maximum. At `OSPEEDR` 00, 30 pF guarantees 13.3 ns (measured, `check speeds`).

## 5. Adversarial pass (full)

Done in two ways: by me, and by a fresh adversary (Opus) given the code and the claims but not my findings, working on a frozen copy (`~/.cache/aet-4220b/adv/work/`, reproductions there).

- **§5.1, a pad off its cited dimension, a swapped pin, a missing keepout.**
  - Mutants M9 (a land 25 µm off), M9b (a land grown 25 µm), M10 (pins 13 and 14 swapped in the symbol) and M11 (the BMP581's front keepout removed) were each applied with the hash re-pinned, as a deliberate wrong edit would be. Each is killed by `eda/tests/own_parts.rs`, whose citation tables are written from the pages, apart from the files.
  - The adversary edited the emitted board by hand: keepouts deleted, moved or allowing tracks, and a land narrowed. `extract` accepts each, because it judges capacitance, not footprint identity. In the image, `check drc --spec --schematic` refuses every one with `lib_footprint_mismatch`, and the unedited board passes (measured, `~/.cache/aet-4220b/drc-adv.sh`). `verify` runs both.
- **§5.2, a declared value passed off as a maximum, or left out of a line's sum.**
  - Mutants M1 (counted among `pins`), M2 (left out of the sum), M3 (an I2C declaration ignored), M4 (preferred over a cited maximum) and M5 (its condition not stated) are each killed.
  - Found and fixed:
    - **A1:** an I2C declaration below UM10204's 10 pF lowered a line's bound below what every undeclared I2C pin is counted at. Now `PinCapacitanceBelowI2cCi`, with the constant held once in `aethelgard-ir` (`extract`'s `PIN_FF` derives from it); killed by M14.
    - **A2:** beside a declared I2C pin, the standing condition still said every I2C pin is at most 10 pF; it now names only the pins it covers. Killed by M15.
    - **H1 (adversary):** the SPI condition still said each device pin is at its cited maximum on lines whose only device pin is declared. Reworded the same way; killed by M16.
  - Open: H2 (Q2).
- **§5.3, a trace under the BMP581.** `board_tier2`'s new test draws U7's own SDA net straight across its centre, on `F.Cu` and then on `B.Cu`. DRC refuses each with `items_not_allowed` on that track (measured in the image). The adversary's tracks inside the keepouts were refused by the router's reader as well ("inside U7's keepout").
- **Refused correctly** (the adversary's list):
  - values out of range, and malformed numbers;
  - a declaration beside a cited maximum;
  - each key-shape trick (missing, `{}`, `{"declared": null}`, unknown fields, duplicate keys at all three levels);
  - texts that are empty, 201 bytes, multi-line or non-ASCII;
  - version 12, with and without the key;
  - a net named `U7_SDO`;
  - each sensor on the wrong bus;
  - a footprint swapped, revalued, renumbered or rotated.
- **Mutation runs, sized to the diff:** 18 mutants (M1–M16, with M6b and M9b), 18 killed, each by its intended assertion. Harness: `~/.cache/aet-4220b/mut/mutants.py`.
  - The first run's M12, M13 and M15 were killed for the wrong reason. `rsync -a` restored M11's mutated footprint with its older mtime, so cargo kept a kicad-lib build of the mutant, and the copy's binary refused its own library.
  - The harness now touches every mutated path after copying. Re-run, each was killed by its own test, and a clean control run passed.

## 6. Gates

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and clippy with `aethelgard-cli/with-tools`: pass (Furnace, measured). `cargo test --workspace`: 119 result lines, none failed (measured).
- Tier 2 in the image: `board_tier2`'s new test passes in 6.5 s (measured). It is weighed at 18.116 s in `ci/test-weights.tsv`, a Furnace estimate times 2.8 as `1310`'s was, until the refit rule measures it. `TIER2_MIN` is 161.
- CI: see the PR.
- Reduction pass (diff): −1 line net · 1 duplicate removed (`own_board_tier1`'s board read) · 0 dead paths removed · 1 protected kept (the citation tables in `own_parts.rs`, which restate the footprints by design: a test derived from the files would be consistency, not judgement, `RUL-20`) · 0 findings below.

## 7. Follow-up: the H2 floor (ruled on #192, Q2; `wo/aet-4220b-floor` from `70d1621`)

The architect's note: `~/.cache/aet-4220b-floor-from-architect.md`.

- **Old behaviour first** [measured, `~/.cache/aet-4220b/floor-red.log`]. Two tests were committed red on `70d1621` (`49dcca9`):
  - `own_board_tier1`'s `a_declaration_below_the_parts_cited_typical_is_refused`: the adversary's `ext_flip` case, the fixture with the IMU budgets at 20 pF. U5 declared at 1 fF passed `extract` with exit 0 on the same board where 10 pF fails.
  - `validate`'s `a_declaration_never_falls_below_the_parts_cited_typical`: a 1 fF declaration validated.
- **Built.**
  - `parts.json` schema 7. Each row's `pin_capacitance` requires `typical_femtofarads`: the typical the datasheet prints, under the row's existing `citations.pin_capacitance`, or `null`. It is 1 fF to the ceiling and never above the row's maximum.
  - `validate` step 14 refuses a declaration below it on any bus: `PinCapacitanceBelowTypical`, naming the device and citing the row.
  - The typical floors a declaration; it is never a bound, and `extract` is unchanged.
  - Both tests are now green, and so is the adversary's own `specs/flip1.json`: refused by `validate` and by `extract` (measured).
  - One mutant (the comparison neutralised) is killed by the floor test.
- **Typicals held, read from each row's own citation text**, which earlier work orders read at source:
  - ICM-42688-P and ICM-20948: C_I "< 10" pF TYP, held as 10 000 fF (the printed figure; "<" makes it a ceiling on the typical, and the floor takes it as written);
  - TMP117: 4 pF typical (6.5).
- **One premise did not hold: the BMP581 prints no typical.** The note says "the BMP581's from its own table", but the held reading (R1, and the row's citation: "tables 4, 5, 6 and 17, pp. 10-13 and 31: no pin or input capacitance is printed") finds no pin capacitance at all. Its typical is `null`. Its declaration is still floored at UM10204's `Ci` by A1, because it is on I2C. The BME280, ICP10111 and W25Q64JV print none either.
- **Coordination.** 4400b (#189) also takes parts schema 7, and the second to merge renumbers; its inbox has the note. The specification version is unchanged at 13.
- **Gates.** fmt, clippy and `cargo test --workspace` pass (measured: 119 result lines, none failed). No tier-2 test changes, so `TIER2_MIN` stays 161.
- **A note on #192's CI (measured), for the shard clause.** Every tools-shard job on #192's head ran 22 to 35 minutes start to end (test steps 19 to 31). `main`'s own last three runs (`bdf7ac0`, `36e428b`, `c7c5ab4`) ran 20 to 34, so the over-20 is `main`'s baseline. This branch adds no tier-2 test. The refit is `ci/shard.sh`'s rule and the architect's (its interim ruling: eight shards, judged by the measurement clause).
