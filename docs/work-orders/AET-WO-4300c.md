# AET-WO-4300c — Pin speeds a datasheet supports, and the parts table's limits enforced

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300c-pin-speeds`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P4, the first piece of `AET-WO-4300`'s split (ruled on PR #123). **Cadence: build, with one measured stop (§1.3).**

**Grounds, each read on `main` at `c621461`:**
- **Speeds `10` and `11` assume the I/O compensation cell** (DS12117 Rev 10, Table 149, note 5). Renode 1.17.0 does not model `SYSCFG_CCCSR`, and a firmware that waits on `READY` hangs in the twin (measured, `4300` spike S5). The generator emits `10` for SPI and never enables the cell. **Ruled (`4300` D1):** speeds `10`/`11` are refused, and SPI moves to `01` (60 MHz at 50 pF, VDD ≥ 2.7 V).
- **No line is checked against its speed** (`4200` D6, moved here). Table 149 gives Fmax per `OSPEEDR`, load and VDD band. Fmax is defined by (tr + tf) ≤ 2/3 T, skew ≤ T/20 and duty 45–55 % (note 3).
- **The parts table's limits have no consumer** (`4200` F19, measured):
  - an SPI bus declared at 200 MHz with the W25Q64JV (104 MHz maximum) passes `validate`, `solve` and `generate`;
  - a part on the wrong kind of bus passes `validate` and `solve`, and is refused only by eda with a contradictory message.
- **Three smaller `4200` findings land here,** because this work order owns the parts table's consumers:
  - **F20:** a malformed flash identity reaches `generate`; the parser should check a model's identity shape.
  - **F21:** the SPI controllers' own names (`spi1` … `spi6`) are not reserved.
  - **F11:** the W25Q64JV's JEDEC ID depends on its ordering code (`EF 40 17` for IQ/JQ, `EF 70 17` for IM/JM), and the table does not say which it holds.

> **What this is:** every generated pin's speed is one the datasheet gives a figure for, every line's toggle rate is checked against that figure, and every part's declared limits are enforced before generation.
>
> It is NOT:
> - motor outputs or DShot (`4300a`);
> - UARTs (`4300b`);
> - power parts (`4300d`);
> - USB (`4300e1`/`e2`);
> - full parasitic extraction (`1303`).

## 1. Build

1. **The Fmax table as data.**
   - **The data:** DS12117 Rev 10 Table 149 (rev V silicon) and Table 52 (rev Y), every `OSPEEDR` row at 10, 30 and 50 pF in both VDD bands. Each value is cited to the table and the document's sha256 (`~/.cache/aet-2301/ds/`).
   - **Its home:** decide it, and say why in FEEDBACK. `parts.json`'s precedent: a pinned data file with one strict parser, refusals by name, and every value cited.
   - **Rows `10`/`11`** are held as data, with their note-5 condition, and are never selectable.
2. **The check.** For every generated line on a peripheral pin, `Fmax(OSPEEDR, VDD_min, C) ≥` the line's required toggle rate:
   - **SPI SCK:** the bus's declared `speed_hz`.
   - **I2C SCL/SDA:** the declared speed. Trivially met at `00`, but checked so the rule has no exceptions.
   - **VDD:** the supplying domain's declared minimum (nominal minus tolerance), which selects the VDD band.
   - **C:** the line's capacitance, rounded **up** to the next table row (10, 30, 50 pF). Above 50 pF the check refuses by name.
3. **Measure, then STOP if the answer is large: where C comes from for SPI lines.**
   - **What exists:** extraction (`aethelgard_eda::extract`) bounds I2C lines today.
   - **What to measure:** whether its machinery bounds an SPI line (SCK, MOSI, MISO, each chip select) as it stands, or needs only the line list widened. Then compute the reference's SPI line bounds.
   - **Small** (the bound machinery is net-agnostic and only the line list changes): build it, and the check reads extraction's bound, plus each receiver's input capacitance from the parts table, adding the field if needed, with its citation.
   - **Large:** stop with the numbers. The fallback is a declared `net_constraint` per SPI line, refused by name when absent, which is a ruling.
4. **The generator emits the slowest `OSPEEDR` whose Fmax meets the line's rate.** That is `00` where it suffices and `01` otherwise; never `10`/`11`. A line whose rate needs `10`/`11` is refused by name, never sped up. The reference's SPI pins move from `10` to `01`.
5. **The parts table's limits enforced at `validate`:**
   - each SPI device's `max_clock_hz` against its bus's `speed_hz`;
   - each I2C device's address against the table's options;
   - each device's bus kind against its part's.

   Each is refused by name with the part and the citation. **F20:** the parser checks a modelled part's identity shape (for `SPI.GenericSpiFlash`: command `0x9F`, 3 bytes, a valid capacity code).
6. **F21:** the controller names `spi1`–`spi6` (and any other the platform declares and the reserved list lacks) join `RESERVED_REFERENCES`. Keep its test exact.
7. **F11:** the table's flash entry is named for its ordering code (for example `W25Q64JVxIQ`), with the IM/JM identity noted in its citation. Migrate every reference to the part name: the specification, fixtures and eda's symbol map. A specification naming the bare `W25Q64JV` is refused by name, with the message naming both ordering codes.

## 2. Acceptance

- **Every generated `OSPEEDR` is `00` or `01`,** and the Fmax check holds on every line of the reference, the 4-layer fixture, `D1` and the ICM-20948 fixture.
- **The reference's committed generated files change only in the SPI pins' speed field,** plus each named consequence: the board crate, the ELF, the platform hashes, the identity re-witnessed. Every verdict is unchanged; prove it as `4200` did, against `main`'s binaries.
- **Each new refusal has a tier-1 test:** too fast for the part, too fast for the pin, the wrong bus kind, a bad address, a malformed flash identity, a reserved controller name, a bare ordering code.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes. `main`'s slowest is 18.4 min: if you add tier-2 time, refit by `ci/shard.sh`'s rule. `TIER2_MIN` exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02`, `INV-06`, `INV-07`; `RUL-12`, `RUL-14`, `RUL-16`.
- **Fence:**
  - `aethelgard-ir` (the table, validation, reserved names);
  - `aethelgard-svd-hal` (the speed policy);
  - `aethelgard-eda` (only where extraction's line list must widen, and the part name map);
  - the CLI's tests and fixtures;
  - `examples/`, `boards/sentinel-01/**` (regenerated);
  - `ci/`;
  - this work order's FEEDBACK.
- **Another builder on Furnace:** `1307` (`~/aethelgard-1307`, the router, `route.rs`). If you widen extraction, you touch `extract.rs`, which 1307 does not, but merge `origin/main` whenever 1307 lands.
- One PR, left unmerged. Write `~/.cache/aet-4300c/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Adversarial pass, full

1. **A line that passes the check but should not:**
   - a capacitance rounded down;
   - VDD taken from the nominal, not the minimum;
   - a rev Y board judged on rev V's table, or the reverse (say which silicon the reference is, and why the check holds on both);
   - a chip select toggled faster than its bus.
2. **A part limit that slips:** a bus speed changed after `validate`, or a device moved between buses. The solved topology must carry what the check read (`RUL-16`).
3. **A table that lies:** a malformed row, a missing citation, a row `10` made selectable. Each must be refused by name.

## 7. Flag-and-stop conditions

- SPI lines cannot be bounded without a large extraction change (§1.3).
- Any verdict on the reference changes for a reason this work order does not name.
- The reference's SPI line at `01` fails the Fmax check at its bounded C.
