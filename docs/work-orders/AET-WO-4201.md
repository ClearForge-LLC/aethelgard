# AET-WO-4201 — Spike: a second source for the BME280's role, measured against what the engine needs from a part

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4201-bme280-second-source`.
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4 parts (roadmap §7.1, the BME280 supply row, owner "Scotty; the next parts work order"). Written 2026-11-19. **Cadence: spike, ending in a stop. The part choice is Scotty's.**

**Grounds:**
- **The BME280 cannot be bought.** Every authorized US distributor showed 0 on 2026-11-17 (DigiKey, Mouser, Arrow, Newark, LCSC), with a 16-week lead time.
- **Sentinel-01, -02 and -03 all carry it:** environmental sensing over I2C, with identity 0x60 at 0x76/0x77.
- **The engine carries it end to end:**
  - its parts data (`aethelgard-ir` parts table, schema 10);
  - footprint and symbol;
  - the Renode model;
  - the identity judge;
  - the bench row `3302` B3–B5;
  - the generated firmware's identity read (`2202`).

  A second part costs each of these, and that cost is part of the answer.

> **What this is:** the candidates measured, so Scotty can choose. Each candidate is:
> - a part, or a pair of parts, that fills the BME280's role on the bus and in the airframe;
> - in stock today;
> - with every engine cost stated.
>
> It is NOT:
> - the choice. That is Scotty's;
> - a change to any crate, model, parts table, board or golden;
> - a stocking plan for the BME280 itself. Only if no candidate passes is that the recommendation.

## 1. Spike

1. **The role, from the repository.** State what each Sentinel uses the BME280 for: pressure, humidity, temperature, or only its identity as a bus witness. Cite the specification, the parts table, and what the generated firmware reads.
   - Sentinel-02/-03 also carry a BMP581 for pressure. If the BME280's job there is humidity and temperature only, the candidate set changes. Say which job each board needs.
2. **Candidates.** At least three, from at least two manufacturers.
   - Bosch BME680 and BME688.
   - A pressure-plus-humidity pair, for example a Bosch BMP390 or a second BMP581, with a Sensirion SHT4x.
   - TE MS8607.
   - Any part you find with the same role.
   - For each, cite the datasheet with document number and revision. Record:
     - the I2C address options;
     - the identity register and value;
     - supply and I/O voltage;
     - the package, and whether it is footprint-compatible with the BME280's LGA-8 2.5 × 2.5 mm;
     - accuracy against the BME280's;
     - current;
     - temperature range;
     - lifecycle.
3. **Stock, measured, with date and source.**
   - Read DigiKey, Mouser and LCSC stock and unit price for each candidate, at the time you read it.
   - If Furnace cannot reach a distributor, say which ones. Scotty reads those by hand; do not guess a figure.
4. **The engine's cost, per candidate:**
   - the parts-data rows and schema impact;
   - footprint and symbol (KiCad library or ours);
   - a Renode model: does one exist upstream, or what would ours need?
   - the identity judge's row;
   - the generated firmware's identity read (a new bring-up script);
   - the bench sheet's rows;
   - every board and golden that would move.
   - **Measure, don't guess,** wherever a dry run can: for example, `solve` a scratch copy of Sentinel-01 with the candidate's parts row, in a scratch tree, never committed.
5. **`INV-07`.** For each candidate, say what the twin could not model at first (a gas sensor's heater, say), and so what the bench alone would judge.

## 2. Stop

**STOP** with FEEDBACK holding:
- the role per board;
- the candidate table: datasheet, stock and price with date and source, engine cost and bench cost;
- a recommendation with its reason, phrased as a recommendation to Scotty.

Push an `AET-WO-4201 STOPPED:` commit and open a PR. It is merged as the record after Scotty's choice. Write `~/.cache/aet-4201/pr-title-body.md`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-14` (no nominal as a minimum; every value cited).
- **Fence:** `spike/4201/` and FEEDBACK only. Scratch trees stay outside the repository.
- **Running beside you:** `4400g`, `1910` and `3304`. You share no file with them. One container of yours at a time, under `capped-run`.
- One PR.

## 7. Flag-and-stop conditions

- §2 is the stop.
- **No candidate is in stock at two distributors:** stop with the table. The recommendation then becomes a stocking plan.
