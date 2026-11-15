# AET-WO-4220b — Sentinel-02's sensors on the board: our own symbols and footprints, and declared pin capacitance

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4220b-board`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. The board slice of `AET-WO-4220`, split at the builder's stop (PR #151, D-B8, ruled in comment 6898233775). It runs after `AET-WO-4300d2` merges. **Cadence: build.**

**Grounds, on `main` after PR #151 (`78f700d`):**
- **The models slice is merged.** The ICM-42688-P and BMP581 are `active` rows in `parts.json` schema 4, modelled in `models/renode/`, and witnessed end to end on the Sentinel-02-shaped fixture.
- **D5, ruled at the spike (comment 6890080737):** our own KiCad symbols. Neither part has a KiCad 10.0.7 symbol, and the ICM's datasheet prints no land pattern.
- **D10, settled by the build's rendered tables:**
  - The ICM-42688-P's `C_I` "< 10" pF sits in the TYP column, with MIN and MAX empty, so it is typical only.
  - The BMP581 prints no pin or input capacitance at all.
  - The spike's ruling says a part with typical-only pin capacitance may declare a stated condition.
- **What blocked it is merged with `4300d2`:**
  - the strict reader's keepouts in vendored footprints (`RUL-03` as amended): the BMP581's land pattern asks for no vias or traces under the part (§8.2, p. 69);
  - power emission;
  - `extract.rs`'s current shape.
- **The inputs are held** in `~/.cache/aet-ds/aet-4220/ds/`:
  - `baro/BMP581-READ.md` §7 and §9 (the rendered land pattern and pin table);
  - `imu/ICM42688P-READ.md` §3 and §4 (pins and package);
  - TDK AN-000393 v2.4 (`imu/an000393/`, sha256 `be0aa48f9f10fc2dcb0e647d8a6509a75636b3f5df0e714388784a53d9bde55b`).

> **What this is:** the two flight sensors are drawn, placed, routed and extracted on a board that every gate judges. Their symbols and footprints are our own, cited pin by pin. Their pin capacitance enters extraction as a declared bound with its condition named, never as a maximum the datasheet does not print.
>
> It is NOT:
> - the FIFO (`AET-WO-4400c`);
> - a second bus per part;
> - Sentinel-02's specification (`AET-WO-4500`).

## 1. Build

1. **Symbols and footprints of our own** in `aethelgard-kicad-lib`, each with its licence (`MIT OR Apache-2.0`, ours) and sha256 (`RUL-11`).
   - Every pin and every pad dimension cites the held page.
   - The BMP581's keepout is the footprint's own, judged by DRC as `RUL-03` now rules.
   - The ICM's land pattern follows AN-000393, cited.
2. **A declared pin capacitance** (D10).
   - A sensor row may carry `pin_capacitance` as either:
     - a cited maximum; or
     - a declared value with its `condition` and `source` (for the ICM: TDK's typical "< 10 pF", declared at 10 pF; for the BMP581: no published figure, declared by the specification with its stated basis).
   - `extract` adds it to each bounded line the pin sits on.
   - The verdict names every declared term as declared, never as a maximum (`RUL-19`; `4300c` D3 is unchanged for parts that print a maximum).
3. **The Sentinel-02-shaped fixture gains a board.** It is emitted, placed, routed and extracted, and DRC passes with parity.
4. **The witness ledger (`AET-WO-4230`), if merged first:** each declared pin capacitance is a ledger row of its own.

## 2. Acceptance

- **The fixture passes ERC, DRC with parity and `extract`** in the image. Its two sensors are drawn from our own library.
- **Every existing board's files and verdicts are byte-identical**, apart from the specification-hash lines a version bump moves.
- **Each new refusal has a tier-1 test.** The CLAUDE.md gates pass. CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`; `RUL-03`, `RUL-11`, `RUL-19`, `RUL-20`.
- **Fence:**
  - `aethelgard-kicad-lib`;
  - `aethelgard-ir`: the `pin_capacitance` field and its parser;
  - `aethelgard-eda`: the emitter's use of the new parts and `extract`'s term;
  - tests and fixtures; `ci/`; FEEDBACK.
- **Next free versions:** take them from `main` at kickoff. The second to merge renumbers.
- One PR, left unmerged. Write `~/.cache/aet-4220b/pr-title-body.md`.

## 5. Adversarial pass, full

1. **A pad off its cited dimension,** a swapped pin, or a missing BMP581 keepout. Each must fail a test or DRC.
2. **A declared capacitance passed off as a maximum,** or a declared term left out of a line's sum.
3. **A trace routed under the BMP581.**

## 7. Flag-and-stop conditions

- **A held page cannot give a pad dimension** the footprint needs.
- **The router cannot respect the BMP581's keepout** on the fixture's board.
