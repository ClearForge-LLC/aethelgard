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

## 2. Decisions taken (forks inside the fence)

To be filled as built.

## 3. Questions for the architect

- **Q1 · §1.4 against Q-I.** The ledger is register fields only, with "no `PIN_C` rows" (4230 Q-I). Should a declared pin capacitance be a `twin_limits` row (never blocking), a ledger row after all, or neither, with `extract`'s verdict as its only record (built)?
