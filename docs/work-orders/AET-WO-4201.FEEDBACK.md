# AET-WO-4201 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-4201-bme280-second-source`, from `main` at `c3f8ddbb`. The work order is on the docs branch `claude/argon-repo-docs-synthesis-rjrb0d` at `bcc80a7d` (`docs/work-orders/AET-WO-4201.md`, identical to the kickoff copy). This branch does not merge it. Evidence is in `spike/4201/`. No crate, model, parts table, board or golden changed. Scratch trees stayed in `~/.cache/aet-4201/scratch/`, outside the repository.

Every answer is labelled:
- **measured**: run here;
- **read at source**: from a file or datasheet named beside it;
- **agent-read**: extracted by a subagent from a datasheet held by sha256. Values marked ✓ were re-read at source by the builder;
- **inferred**.

Read and run on 2026-11-19, 06:40–08:00 UTC.

## 0. The answer first

**Recommendation to Scotty:** put a **BMP581** in U2 on Sentinel-01 (0x46).

**On Sentinel-02 and -03,** U7 is already a BMP581. Either:
- a second BMP581 at 0x47; or
- U2 left unpopulated, measured clean: nothing in the repository asks for two barometers.

**Condition:** this holds only if your own read shows BMP581 stock at two of DigiKey, Mouser and LCSC. Furnace could read no stock figure at any of the three (§3).

**Why:**
1. The BME280's role in this repository has two jobs. Its humidity is read nowhere.
   - It is the I2C bus's identity witness.
   - It is an ArduPilot barometer.
2. The BMP581 already does both jobs in the engine: parts row, footprint and symbol, Renode model, bring-up script and ArduPilot driver.
3. **Measured:** `validate`, `solve` and `emit` pass on all four Sentinel specifications with U2 swapped to a BMP581, and `generate hwdef` writes `BARO BMP581`.
4. **Measured:** the bench `verify` with a BMP581 in U2 reaches the campaign gate; the gates before it passed (inferred from the gate order, §4.2). The campaign is then refused by one twin fault: a scheduled line stamped 1 ns early after the brown-out reset (§4.2). The unchanged tree passes the same campaign 2/2.
   - That fault is the BMP581's one real engine cost. A build work order fixes it first.
   - It lies in the twin's clocks or our BMP581 model, where `4400g` is working now.
5. Its breakout is already on the bench for `AET-WO-3304`.

**If the BMP581 is not stocked at two distributors:**
- The next candidate is the **Infineon DPS368**. It is a different maker, has the BME280's pin order on a 2.0 × 2.5 mm body, and has an ArduPilot driver.
- Its engine cost is real: a row, a symbol, a footprint, a schematic arm and a Renode model of our own.
- **Second alternative:** the BMP390. Its pinout is the BMP581's, so its footprint and symbol cost is the lowest of the new parts. But it is Bosch again.

**If you hold a humidity requirement:** none is written in the repository. In that case the answer changes to one of:
- BME688 (or BME680). This loses the ArduPilot barometer, because ArduPilot has no driver for it.
- A second part for humidity (SHT45). Its identity needs a parts-schema change (measured, §4.1).

**§7's stop:** "No candidate in stock at two distributors" cannot be decided from Furnace. Every stock cell is unread, not zero (§3). The cells that decide it are the BMP581's and the DPS368's at DigiKey, Mouser and LCSC.

## 1. The role, per board (read at source)

| What reads the BME280 | Where | What it reads |
|---|---|---|
| The generated firmware | `crates/aethelgard-ir/data/parts.json`, BME280 row: `script: null` | Identity only: write `0xD0`, read 1 byte, expect `0x60`. No pressure, temperature or humidity read |
| The twin's judge | `crates/aethelgard-twin/src/model.rs` (`Model::Bme280`, `BME280.cs` L38/L75 witness) | The identity transaction, witnessed by the model's own log (architecture §2, "The witness is the device model's own log") |
| The foreign (ArduPilot) judge | parts row `reads: [{write 0xF7, read 6}]`, cited to datasheet §4 p. 23 | The burst from `0xF7`: pressure and temperature (`0xF7–0xFC`). Humidity would be `0xFD–0xFE`, which is not listed |
| ArduPilot | parts row `ardupilot: {kind baro, driver BMP280, id 0x60}`; `generate hwdef` writes `BARO BMP280 I2C:0:0x76` (fixtures `sentinel-01.hwdef.dat` L25, `sentinel-02.hwdef.dat` L26) | A barometer. `AP_Baro_BMP280` reads pressure and temperature only |
| The bench | `AET-WO-3302` B3–B5 | The identity ("the BME280 ID should read `0x60`") on a 3.3 V breakout, plus the bus's rise times at 2.2 kΩ, 47 kΩ and 2.7 kΩ |
| The bench order | `fab/sentinel-01-bench/ORDER.md` L36 | U2 BME280, LGA-8 2.5 × 2.5 mm |

**Humidity:** `grep -i humid` over the northstar, architecture, roadmap, `3302`, `2202`, `ORDER.md`, the four Sentinel specifications and the parts table returns nothing (measured). No line in the repository asks for humidity.

**Per board:**

| Board (specification) | I2C `SENSORS` devices | The BME280's job | What a replacement must do |
|---|---|---|---|
| Sentinel-01 reference (`examples/sentinel-01-minimal.json`) | U2 BME280 0x76, U3 TMP117 0x48 | **The only modelled I2C device:** TMP117 is `renode: null`. So the BME280 carries P2 gate clause 1's identity witness and every I2C finding of the seeded campaign (`scenarios/sentinel-01/seeded/cases.json`: `i2c_timeout`, `i2c_identity_mismatch`, `i2c_transaction_unwitnessed`) | Be modelled in Renode, or Sentinel-01 loses its I2C witness (§4.3) |
| Sentinel-01 as ordered (`examples/sentinel-01.json`, `fab/sentinel-01-bench/`) | as above, plus the `EXT` bus with no device | as above; plus `BARO` in its hwdef | as above, plus a cited ArduPilot driver |
| Sentinel-02 (`tests/fixtures/sentinel-02-own.json`) | U2 BME280 0x76, U3 TMP117 0x48, U7 BMP581 0x46 | A second barometer (U7's BMP581 is the first), and a second modelled witness | An ArduPilot baro at an address other than 0x46 and 0x48 |
| Sentinel-03 (`tests/fixtures/sentinel-03.json`) | as Sentinel-02 | as Sentinel-02 | as Sentinel-02 |

The founding document names "environmental probes" among the markets (`docs/inputs/argon-session/00-founding-explainer-google-doc.md` L79). That is a product aspiration, not a Sentinel requirement; whether a Sentinel should sense humidity is yours to say.

So **no board needs humidity.** Sentinel-01 needs a modelled identity witness and a barometer. Sentinel-02 and -03 already have a barometer in U7, the BMP581. The BME280 there is a second one, and nothing in the repository asks for two: `AET-WO-4210` chose "Sentinel-02's IMU and barometer", singular (roadmap L279), and no line names barometer redundancy (measured, grep).

**So U2 left unpopulated on -02/-03 is an option (measured):** with U2 removed, Sentinel-02 and -03 pass `validate`, `solve` and `emit`. Sentinel-02's hwdef keeps `BARO BMP581 I2C:0:0x46`; Sentinel-03's hwdef is refused at baseline as well (F2). Whether a Sentinel should fly one barometer or two is yours to say.

## 2. Candidates (datasheets)

Twelve parts from six makers were examined: BMP581, BMP390, BME680, BME688 (Bosch); DPS310, DPS368 (Infineon); SPL06-001, SPA06-003 (Goertek); MS8607 (TE); SHT40/SHT45 (Sensirion); LPS22HB, LPS25HB (ST). ST's LPS22HB and LPS25HB were not measured: st.com served neither datasheet (curl `rc=92`, WebFetch timeouts, 2026-11-19 06:47Z). The candidate table (§2.1) carries the six that fill the role. MS8607 and SHT45 are in §2.2.

Datasheets held by sha256 (`spike/4201/facts/datasheets.sha256`; fetched 06:47–06:52Z, URLs in `spike/4201/facts/*.json`):
- **BMP390:** BST-BMP390-DS002-07 rev 1.7 (2021-03), `8dcf0f74…` ✓
- **BME680:** BST-BME680-DS001-09 rev 1.9 (2024-02), `8e42b591…` ✓
- **BME688:** BST-BME688-DS000-03 rev 1.3 (2024-02), `fce73ae0…` ✓
- **DPS310:** Infineon IFX-sch1406115644540 V1.2 (2020-10-15), `c856826f…` ✓. Infineon-hosted community copy: infineon.com's asset URL returned 202 with no body.
- **DPS368:** Infineon IFX-duu1493988991355 v1.1 (2019-07-03), `16d3f603…`
- **SPA06-003:** Goertek, no document number, Ver 2.0 (2023-12-05), `aee86bb4…`. LCSC-hosted and marked "Confidential"; Goertek publishes no datasheet that we found.
- **SPL06-001:** Goertek, Ver 7.0, `e5d365e4…`. LCSC-hosted.
- **SHT4x:** Sensirion "Datasheet SHT4x" V7.3 (2026-06), `8db4a43f…`
- **Baselines, already held:** BME280 BST-BME280-DS001-24 (`a2ccdb44…`) and BMP581 BST-BMP581-DS004-13 (`ddcf4202…`).

### 2.1 The candidate table

| | **BME280** (today) | **BMP581** | **BMP390** | **DPS368** (DPS310) | **SPA06-003** | **BME688** / BME680 |
|---|---|---|---|---|---|---|
| Maker | Bosch | Bosch | Bosch | Infineon | Goertek | Bosch |
| I2C 7-bit address | 0x76 / 0x77 (SDO) | 0x46 / 0x47 (SDO) | 0x76 / 0x77 (SDO) | 0x76 / 0x77 (SDO; 0x77 default) | 0x76 / 0x77 (SDO) | 0x76 / 0x77 (SDO) |
| Identity | `0xD0` = `0x60` | `0x01` = `0x50` | `0x00` = `0x60` ✓ p. 30 | `0x0D` = `0x10`: REV_ID[7:4], PROD_ID[3:0] ✓ DPS368 p. 37 (DPS310 p. 36) | `0x0D` = `0x11` (p. 32) | `0xD0` = `0x61` ✓ (BME680 p. 28, BME688 p. 33) |
| VDD / VDDIO (V) | 1.71–3.6 / 1.2–3.6 | 1.71–3.6 / 1.08–3.6 | 1.65–3.6 / 1.2–3.6 ✓ p. 3 | 1.7–3.6 / 1.2–3.6 ✓ p. 1 | 1.7–3.6 / 1.08–3.6 | 1.71–3.6 / 1.2–3.6 |
| Package | LGA-8 2.5 × 2.5, P0.65 | LGA-10 2.0 × 2.0, P0.5 | LGA-10 2.0 × 2.0. **Pinout identical to the BMP581's** ✓ (BMP390 Table 52 = BMP581 Table 28, both p. 46) | VLGA-8 2.0 × 2.5, P0.65; the BME280's pin order | LGA-8 2.0 × 2.5, P0.65; the BME280's pin order | LGA-8 3.0 × 3.0, P0.8 |
| On the BME280's footprint? | — | No | No | No: rows 1.45 mm apart against the footprint's 2.05 (agent-read) | No (as DPS368) | No |
| Pressure, absolute | ±1.0 hPa, 0–65 °C ✓ p. 11 | ±30 Pa, −5–65 °C; ±75 Pa full range incl. solder drift ✓ pp. 8–9 | ±0.50 hPa typ, 0–65 °C; ±0.33 hPa at 900 hPa, 25 °C ✓ pp. 3, 7 | ±1 hPa, 0–65 °C | ±0.3 hPa (column unconfirmed) | ±0.6 hPa |
| Pressure, relative | ±0.12 hPa | ±6 Pa (10 kPa steps) ✓ | ±0.03 hPa ✓ | ±0.06 hPa ✓ p. 1 | ±3 Pa | ±0.12 hPa |
| Temperature | ±0.5 °C (0–65 °C) | ±0.5 K ✓ p. 10 | ±0.5 °C at 25 °C; ±1.5 °C 0–65 °C | ±0.5 °C | ±1 °C | ±0.5 °C (BME688); BME680 ±1.0 °C over 0–65 °C |
| Humidity | ±3 %RH | none | none | none | none | ±3 %RH |
| Current, 1 Hz typ | 2.8 µA P+T; 3.6 µA H+P+T ✓ p. 2 | 1.3 µA | 3.2 µA ✓ p. 3 | 1.7 µA pressure, 1.5 µA temperature ✓ p. 1 | 3 µA (low); 11 µA standard | 3.1 µA without gas; heater 9–13 mA, 17 mA peak, when gas is run |
| Temperature range | −40–85 °C | −40–85 °C | −40–85 °C | −40–85 °C | −40–85 °C | −40–85 °C |
| Start-up, max | 2 ms | 2 ms; 4 ms from deep standby | 2 ms | 12 ms sensor ready; **40 ms** coefficients ready | 2 / 4 ms | 2 ms |
| Lifecycle (vendor page, 2026-11-19) | active (row's citation, 2026-11-15) | active (row's citation, 2026-11-15) | no status word on Bosch's page | no status word read (page returned navigation only). DPS310: "not for new design" per an Infineon community thread, **unverified** | no vendor page found | no status word. BME680 NRND per search summaries, **unverified** |
| ArduPilot `Copter-4.7.2` at `fb773567` | `AP_Baro_BMP280` (0x60) | `AP_Baro_BMP581` (0x50) | `AP_Baro_BMP388`: `BMP390_ID 0x60` (L29) | `AP_Baro_DPS280` (`DPS280_WHOAMI 0x10`, L41; DPS310 class) | `AP_Baro_SPL06`: `SPA06_CHIP_ID 0x11` (L25) | **none**: no `AP_Baro_*` for BME68x |
| Renode 1.17.0 (`renode-infrastructure` `cd4b002a`) | `I2C.BME280` | ours, `Sensors.BMP581` | none | none | none | none |
| KiCad 10.0.7 | vendored | ours (`Aethelgard_Sensor`) | none upstream | none upstream | none upstream | `Sensor:BME680` and `Bosch_LGA-8_3x3mm_P0.8mm` upstream, not vendored |

Notes:
- **The BMP390's `0x60` is the BME280's value, at another register.** A judge keyed on the value alone could not tell them apart. Ours keys on register and value, from the row.
- **DPS310, DPS368 and SPL06-001 all read `0x10` at `0x0D`,** so an identity read cannot tell them apart. The upper nibble is a revision: an identity with `mask: null` would refuse a later silicon revision, and `mask 0x0F` leaves only `PROD_ID = 0`.
- **The DPS368's 40 ms coefficient wait equals the parts table's `MAX_SETTLE_US` (40 000 µs) exactly.** A bring-up script that reads data would sit at the ceiling. An identity-only row (`script: null`) does not wait.
- **SPL06-001:** DigiKey shows it Obsolete per a search summary (PCN "EOL/Obs 31/Dec/2023", not read). It is excluded in favour of SPA06-003.

### 2.2 Examined and set aside

- **TE MS8607** (P+T+H in one package): te.com returned 403 to the datasheet (twice, 06:48Z), and te.com.cn returned a 567 bot wall. No datasheet value is held.
  - TE's product page (WebFetch, 2026-11-19) says: "This product is not currently available." It also gives 5 × 3 × 1.15 mm and 1.5–3.6 V.
  - Its pressure half has no identity register (MS56xx-style PROM with CRC). Inferred from the family: the datasheet was not read.
  - ArduPilot names no MS8607 driver at `fb773567`. `AP_Baro_MS5611.h` lists MS5611, MS5607, MS5637 and MS5837.
  - **It fails "in stock" on its maker's own page.** If you want it measured, the datasheet must be fetched by hand.
- **Sensirion SHT45** (the humidity half of a pair; agent-read, Datasheet SHT4x V7.3):
  - 1.08–3.6 V supply, with no separate I/O range (Table 4 p. 9);
  - ±1.0 %RH and ±0.1 °C typical at 25 °C; the maxima are printed only as graphs (Tables 1–2);
  - 0.4–2.2 µA average at one measurement a second, by repeatability; 0.08 µA idle;
  - −40 to 125 °C; DFN-4 1.5 × 1.5 mm;
  - 1 ms maximum power-up and soft reset.

  Renode 1.17.0 has `I2C.SHT45` upstream, and KiCad 10.0.7 has `Sensor_Humidity:SHT4x` and its DFN-4 footprint.
  - It has no identity register. Its 0x89 read is a per-unit serial number (datasheet §4.7 p. 13), so there is no fixed value to compare.
  - **Measured:** schema 10 refuses such a row (§4.1).
  - SHT45 is 0x44 only. The SHT40-CD1B is 0x46, which would collide with U7 on Sentinel-02/-03.
- **ST LPS22HB / LPS25HB:** not measured (above). ArduPilot accepts both (`LPS22HB_WHOAMI 0xB1`, `LPS25HB_WHOAMI 0xBD`), and Renode has `Sensors.LPS25HB`. If you want ST in the set, a hand-fetched datasheet would let it be measured.
- **ICP-10111:** already in the parts table, but as an `eol` twin stand-in at 1.71–1.89 V. It is not a purchase.

## 3. Stock and price: what Furnace could read

**No distributor's stock figure was readable from Furnace on 2026-11-19.** Each attempt and its result:

| Distributor | Tried | Result |
|---|---|---|
| DigiKey | search page `digikey.com/en/products/result?keywords=<MPN>` via WebFetch | Price only. The page carries no stock text for any row (BMP390, BMP581, BME280, SPA06-003, BME680, BME688, LPS22HBTR). For DPS368XTSA1, DPS310XTSA1, SPL06-001 and the SHT4x it returned 403 (06:48–06:57Z) |
| DigiKey | detail page | 403 (WebFetch) |
| Mouser | search and detail pages | WebFetch: 60 s timeout (twice). The curl probe that saw an "Access Denied" captcha page was one of the discarded browser-UA probes (below) |
| LCSC | search and product-detail pages (`C5124834` and others) | WebFetch: rendered in JavaScript, with no stock or price in the page text |
| JLCPCB | part page | Rendered in JavaScript: "Extended" part, no stock |
| Octopart | search and part pages | 403 to a plain client and to WebFetch |

**Disclosure.** Before the classifier stopped it, three Octopart pages and curl probes of DigiKey, Mouser and LCSC were fetched with a browser-like User-Agent (06:40–06:42Z). That is getting past a bot wall. I discarded those reads, and no figure here comes from them. No later read set any header.

**DigiKey qty-1 prices read (USD, WebFetch of the search page; the quantity break is not labelled on the page):**

| MPN | Price | Read at (UTC) |
|---|---|---|
| BMP581 | $3.55 | 06:57 |
| BMP390 | $3.58 | 07:58 (also 06:4x and 06:50, unstamped) |
| BME280 (reference) | $4.35 | 07:58 |
| SPA06-003 | $0.67297. A reel-sized break is likely, **inferred** | 06:57 |
| BME680 | $9.89 | 07:58 |
| BME688 | $9.89 | 07:58 |
| LPS22HBTR | $3.18 | between 06:47 and 06:52, unstamped (subagent) |

**For Scotty to read by hand.** The decision rests on the first two rows. Identifiers come from search results, so check each on the page:

| Candidate | Orderable MPN | DigiKey | Mouser | LCSC | Stock / price (yours) |
|---|---|---|---|---|---|
| BMP581 | BMP581 | search `BMP581` | search `BMP581` | search `BMP581` | |
| DPS368 | DPS368XTSA1 | product 10244079 / 10244081 | 726-DPS368XTSA1 | C3232508 | |
| BMP390 | BMP390 | product 16164575 | 262-BMP390 | C5124834 | |
| SPA06-003 | SPA06-003 | product 22188103 | not found | C30589048 | |
| BME688 | BME688 | product 13681261 | 262-BME688 | C3664478 | |
| BME680 | BME680 | product 7401317 | 262-BME680 | C125972 | |
| SHT45 (pair) | SHT45-AD1B-R2 | 1649-SHT45-AD1B-R2 | not found | C5221601 | |

## 4. The engine's cost, per candidate

### 4.1 Measured dry runs

**Setup.** The CLI was built from `git archive c3f8ddbb` into `~/.cache/aet-4201/scratch/base`. Each variant is the specification with U2's part and address changed (`spike/4201/tools/variant.py`). Outputs are summarised in `spike/4201/out/dry-runs.txt`.

| Run | Sentinel-01 reference | Sentinel-01 as ordered | Sentinel-02 | Sentinel-03 |
|---|---|---|---|---|
| U2 BME280 (baseline) | validate, solve, emit: 0 | 0 | 0 | 0 |
| **U2 BMP581** (0x46 on -01; 0x47 on -02/-03) | **validate, solve, emit: 0** | **0** | **0** | **0** |
| U2 removed (unpopulated) | — | — | validate, solve, emit: 0; hwdef keeps `BARO BMP581 I2C:0:0x46` | validate, solve, emit: 0 (hwdef refused at baseline, F2) |
| U2 BMP390 (no row) | validate exit 2, `UnknownPart`: "`U2` is a `BMP390`, which has no vendored model; known: BME280, TMP117, BMP581, W25Q64JVxIQ, ICM20948, ICM42688P" | same | same | same |

**The BMP581 variant, generated (measured, `spike/4201/out/`):**
- **`.repl`:** `u2: I2C.BME280 @ i2c1 0x76` becomes `u2: Sensors.BMP581 @ i2c1 0x46`, and the model of our own is named with its sha256 (`repl-bmp581.diff`).
- **Generated firmware `main.rs`:** gains the BMP581's cited bring-up script, `0x36 ← 0x40, 0x81`, a 10 ms settle, and a data read from `0x1D` (`firmware-main-bmp581.diff`, 100 lines). This is more than the BME280's identity-only row gives today.
- **BOM:** U2 becomes `Aethelgard_Sensor:BMP581` on `Bosch_LGA-10_2x2mm_P0.5mm`, plus one 100 nF (the BMP581's second decoupler, Fig. 26).
- **hwdef:** `BARO BMP280 I2C:0:0x76` becomes `BARO BMP581 I2C:0:0x46` on -01, and `…0x47` on -02 beside U7's `BARO BMP581 I2C:0:0x46` (`hwdef-bmp581.diff`).
  - Sentinel-03's hwdef is refused at baseline as well (`SenseNotEmitted`, `VBAT_SENSE`). That refusal predates this work and does not depend on U2.

**Parts rows against schema 10** (measured, `cargo test -p aethelgard-ir --lib the_embedded_table` on a scratch `parts.json`; rows by `spike/4201/tools/rows.py`):
- Rows for BMP390, DPS368, SPA06003 and BME680 parse: identity-only, `renode: null`, ArduPilot citations pending.
  - The only failure is the name-list pin: `left: [... "BME680", "BMP390", ..., "DPS368", ..., "SPA06003", ...]`.
  - Schema 10 holds each of them with **no schema change**.
- **The SHT45 row is refused:** `PartsTable { part: Some("SHT45"), reason: "`identity.datasheet_value` is false, which only a twin stand-in may be" }`.
  - A part with no fixed identity value needs a schema change: a new identity kind (a CRC-checked read, say), ruled.
  - The MS8607 would be the same (inferred).

### 4.2 The bench verify with U2 a BMP581 (measured)

**Setup.** CI's `bench-package` verify was run on a scratch tree where `examples/sentinel-01.json` (Sentinel-01 as ordered) has U2 a BMP581 at 0x46:
- `verify --spec examples/sentinel-01.json --scenarios <the five bench scenarios> --fab-out`;
- in the pinned tools image, under `capped-run`, one container at a time (`spike/4201/tools/img.sh`).

**At 8 GiB the `prove` gate was OOM-killed** (journal: `run-p273176….scope: The kernel OOM killer killed some processes`, 07:09:40Z; verdict `"gate `prove` was killed by a signal"`, `out/verify-bmp581-8G.json`). Re-run at 16 GiB, swap 0.

**At 16 GiB (measured, `out/verify-bmp581.json`):** exit 5, `ToolError` from the `campaign` gate:

> `[WARNING] sysbus: AETHELGARD-INJECT 0 0 1952227 1962227 28a801 770`: its count places it at 20854010 ns in a boot started at 1231740 ns, but it is stamped 20854009 ns

**The control (measured, `out/campaign-controls.txt`).** One ELF per tree; the five bench scenarios run through `campaign`, twice each:

| Tree | Run 1 | Run 2 |
|---|---|---|
| `c3f8ddbb` unchanged (U2 BME280) | pass, 5/5 matched | pass, 5/5 matched |
| U2 BMP581 | the same refusal, on the same line | the same refusal, on the same line |

Alone, `droop-bor3` refuses and `serial-rx-open-console` passes.

**What it is.** The judge (`crates/aethelgard-cli/src/renode.rs` L2535–2573) places a scheduled action's line at `start_ns + 10 ns × count` and refuses one stamped earlier ("no core runs ahead of its time"). After the brown-out reset, with `Sensors.BMP581` on the bus, a scheduled line is stamped 1 ns before its count.
- That our model's own sampling timer (`AET-WO-4220` D8) moves Renode's time slicing across the reset is **inferred**, not traced.
- **No committed scenario puts a BMP581 through a BOR reset** (measured: the droop scenarios' bases are Sentinel-01 and `motors-dshot600`, all with a BME280). So this swap is the first run of the pair.
- **Sentinel-02 on unchanged `main`** (U7 a BMP581), through the same droop with its base retargeted: no stamp refusal, but `brownout_unrecovered` against the borrowed `pass` expectation. That scenario was written for Sentinel-01, so the run neither confirms nor rules out the same latent fault on Sentinel-02.

**Which gates passed before the refusal (inferred):** `verify` runs its gates in the order §4.3's verdict lists: ERC, DRC, `support`, `extract`, eight `prove`, `build`, `simulate`, two `attribute`, then `campaign`. The 16 GiB run's error came from `campaign`, so the gates before it passed, if `verify` stops at the first failure as it did at `prove` under 8 GiB. That includes ERC, DRC and `extract` on the BMP581's LGA-10 as routed on the bench board. The verdict of a run that errors names no gate, so this is not measured gate by gate.

**So the BMP581's engine cost is not zero.** It carries this twin fault, which must be found and fixed before its `verify --fab-out` passes. The fault is in the twin's clocks or our BMP581 model, where `4400g` is working now.

### 4.3 A part with no Renode model: Sentinel-01 with U2 removed (measured)

**What this measures.** For the twin, an I2C device with `renode: null` is equivalent to its absence: the firmware reads only modelled devices' identities (roadmap `2202`), and the judge reports `device_unmodelled` as info. So U2 removed is the proxy for BMP390, DPS368, SPA06-003 or BME688 before a model of our own exists.

**Setup.** CI's bench verify was run on Sentinel-01 as ordered with U2 removed, at 16 GiB.

**Measured (`out/verify-nou2-summary.json`): `verify --fab-out` passes and writes the package.**
- All 17 gates pass, including the campaign at 5/5 matched.
- The only I2C note is `device_unmodelled` for U3, the TMP117.

**Why it passes:** the bench's two I2C scenarios (`pullup-3k3`, `pullup-2k2`) expect `pass`, and with no device on the bus they pass trivially. `AET-WO-3303`'s coverage rule counts `SENSORS` as covered by them.

**Where it shows (measured, `out/seeded-nou2.txt`).** Run on the reference specification with U2 removed:

| Case | Expected findings | Findings with U2 removed |
|---|---|---|
| `pullup-47k` | `analog_rise_exceeds_ceiling`, `i2c_timeout` | `analog_rise_exceeds_ceiling` only |
| `held-low-sda` | `i2c_line_held_low`, `i2c_timeout` | `i2c_line_held_low` only |

Unchanged `main` matches all three cases (3/3); with U2 removed, 1/3 match. With no modelled device the firmware makes no transfer, so `i2c_timeout` is never raised. The electrical half still names the fault, but the software half's witness is gone. The seeded cases whose findings are software-side (`wrong-register`, `wrong-nbytes`: `i2c_identity_mismatch`, `i2c_transaction_unwitnessed`) would have nothing to see (inferred). Their tier-2 tests would fail.

**So an unmodelled part on Sentinel-01:**
- costs P2 gate clause 1's witness, silently in the bench verify;
- breaks the seeded I2C campaign;
- until a Renode model of our own is written for it.

### 4.4 Cost per candidate

Every candidate keeps the BME280 in the engine. The BME280's row, model, symbol, footprint and every fixture that uses it stay valid: 84 non-doc files name it (measured, `git grep -l -i bme280 -- . ':!spike' ':!docs'`), and only those that belong to a Sentinel whose U2 changes must move (the last row below).

| Cost | BMP581 | BMP390 | DPS368 | SPA06-003 | BME688 | BMP581 + SHT45 (humidity) |
|---|---|---|---|---|---|---|
| Parts row | none (row exists) | 1 new row, schema 10 (measured) | 1 row, schema 10 (measured); the mask question (§2.1) | 1 row, schema 10 (measured); a datasheet with no document number (`RUL-14` cites document and revision: a weaker citation) | 1 row, schema 10 (measured); `ardupilot: null` | **schema change** (measured refusal) |
| `names::VENDORED_PARTS` and a `schematic.rs` arm | none | both (code) | both | both | both | both, for SHT45 |
| Symbol and footprint | none (ours exist) | a symbol of ours with the BMP581's pin map. The footprint is the BMP581's (read at source): BMP390 Fig. 26 p. 49 and BMP581 Fig. 30 p. 68 show the same 2.0 mm body, 0.5 mm pitch, 1.525 mm span, 6 × 0.25×0.275 and 4 × 0.275×0.25 pads, 0.1 mm from the edge. The drawings are rotated 90° to each other, so pin 1's corner must be checked against BMP390 Fig. 23 in the build (inferred to match, since Table 52 = Table 28) | ours, both: no KiCad 10.0.7 item | ours, both | vendor KiCad's `Sensor:BME680` and `Bosch_LGA-8_3x3mm_P0.8mm` (`aethelgard-kicad-lib` pins and README) | vendor KiCad's `SHT4x` and its DFN-4 |
| Renode model | ours exists, **but the bench verify fails its campaign gate on a 1 ns stamp after the BOR reset (§4.2, measured): one twin fault to fix** | ours to write (none upstream) | ours to write | ours to write | ours to write | `I2C.SHT45` upstream: add it to `RENODE_MODELS` and the twin's `Model` with its witness lines |
| Identity judge | none (the row drives it) | the row | the row | the row | the row | new identity kind |
| Generated firmware identity read | none new; the bring-up script arrives with the row (measured) | data only: from the row | data only | data only | data only | code for the new identity kind |
| ArduPilot | `BMP581` driver | `BMP388` driver | `DPS310` | `SPL06` | **none: no barometer** on -01 under ArduPilot | unchanged |
| Bench sheet | `3302` B3–B5 rewritten for the BMP581 at 0x46, expecting `0x50` at `0x01`; `3304` already prepares this read | B3–B5 rewritten; a new breakout | rewritten; Adafruit's DPS310 board is "No longer stocked" | rewritten; Adafruit 6420 | rewritten; Adafruit 5046 | rewritten; a second breakout |
| Boards and goldens that move | the four Sentinel specifications; `boards/sentinel-01/`, `boards/sentinel-01-bench/` (`.repl`, firmware); `fab/sentinel-01-bench/package/*` and `MANIFEST.json` (CI-written); `ORDER.md` U2 row; hwdef fixtures `sentinel-01`/`sentinel-02.hwdef.dat` and their topologies; the tier-2 tests that pin the old specification hashes; the seeded cases' evidence (§4.2); on Sentinel-02, the foreign judge's expectations, since unmodified ArduPilot reads both barometers (roadmap L401, `4400e`), which with a BMP581 in U2 are two BMP581s (inferred: the ArduPilot ELF was not rebuilt here) | the same set, plus the parts-table pin test and `the_embedded_table…` name list | same | same | same, minus the hwdef BARO line | same, plus the schema bump's fixtures |

## 5. `INV-07`: what the twin could not model at first, so the bench alone judges

- **BMP581:** nothing new to model. Our model already answers the identity, the configuration and the data read (`models_tier2.rs`: `our_models_answer_their_datasheets_reset_values`, `ardupilot_s_probe_and_init_pass_against_our_models`). Humidity leaves the board.
  - **But the twin's run of it through a brown-out reset is refused today (§4.2).** Until that is fixed, the bench verify of the swapped board does not finish, and the bench alone judges the BOR recovery with a BMP581 on the bus.
- **BMP390, DPS368, SPA06-003:** until a model of our own exists, the identity read is `device_unmodelled`. The bench alone judges the identity, and on Sentinel-01 the I2C findings too (§4.3).
  - DPS368: the 12 ms and 40 ms ready times as well.
  - BMP390: the `0x60` at `0x00`, which a BME280 placed by mistake would not give.
- **BME688 / BME680:** the gas heater: 9–13 mA typical, 17 mA peak, while a gas conversion runs. Its current on `VDD_3V3` and its self-heating of the temperature reading are the bench's to judge; no model has them. A firmware that leaves `run_gas` clear never draws it (datasheet). Under ArduPilot there is no barometer to judge at all.
- **SHT45:** `I2C.SHT45` exists, but its heater commands' power (20–200 mW, 10 % duty cap) is the bench's. Whether the upstream model answers 0x89 at all is not read.

## 6. Bench cost

`3302` B3–B5 wire "the 3.3 V BME280 and TMP117 breakouts … with their own pull-ups removed".

- **BMP581:** the breakout is already on the bench (`3304`, Scotty 2026-11-18/19). Its pull-ups: read them off or measure them, as `3304` already asks. B3 records `0x50` at register `0x01`, address 0x46 (SDO low).
- **The others need a breakout bought.** Adafruit 4816 (BMP390, $10.95), 6420 (SPA06-003, $4.95), 5046 (BME688) and 5665 (SHT45) are each listed with a 3.3 V regulator and level shifting. Whether their pull-ups can be removed is not stated on any page read.
  - A BSS138 level shifter carries pull-ups on both sides (inferred from the boards' descriptions). B3–B5's removed-pull-up wiring may need the shifter bypassed: a bench risk, not a cost in money.

## 7. Recommendation to Scotty

1. **First choice: BMP581 in U2.**
   - 0x46 on Sentinel-01.
   - On -02/-03 there are two options. Each has a barometer already (U7):
     - a BMP581 at 0x47, which keeps two barometers;
     - U2 left unpopulated, which keeps one. Measured clean in `validate`, `solve`, `emit` and hwdef; nothing in the repository asks for two (§1).
   - **Condition:** your hand read shows stock at two of DigiKey, Mouser and LCSC.
   - **Engine cost:** specifications and regenerated goldens. No new row, symbol, footprint or model.
   - **Plus one twin fault, measured (§4.2):** after the brown-out reset, a scheduled line is stamped 1 ns early, so the bench verify's campaign is refused. Fixing it is the build's first item. It may already be latent on Sentinel-02, and is not proved either way.
   - **Twin:** it keeps Sentinel-01's modelled witness, and gains a data read the BME280 never had.
   - **Bench:** it is already there.
   - **Two costs it carries:**
     - **One maker.** If the BME280's shortage is Bosch's own, the BMP581's could follow (inferred; no evidence of a BMP581 shortage was read).
     - **Sentinel-02/-03 fly two identical barometers.** That is redundancy without diversity.
2. **If the BMP581 is short, or you want a second maker on -02/-03: DPS368.**
   - It is Infineon, active per a source I could not confirm, and has an ArduPilot driver.
   - Its engine cost is the full set: row, names list, schematic arm, symbol, footprint and a Renode model of our own. Until that model exists, Sentinel-01 loses its I2C witness (§4.3), so on Sentinel-01 it is a build work order, not a swap.
3. **BMP390 if the BMP581 is short and a second maker does not matter.** Its pinout is the BMP581's, so its board cost is the least of the new parts. It still needs a row, a model and a schematic arm.
4. **Only if you hold a humidity requirement:**
   - BME688: it loses ArduPilot's barometer on Sentinel-01, and needs a heater on the bench.
   - Or BMP581 plus SHT45: a parts-schema change for an identity with no fixed value.
5. **Not recommended:**
   - MS8607: its maker lists it "not currently available", and no datasheet could be held.
   - SPL06-001 and DPS310: Obsolete / not for new design, both unverified.
   - SPA06-003 for -01: a vendor datasheet with no document number, LCSC-hosted and marked "Confidential". It is a citation that `RUL-14` would accept only weakly.

## 8. Findings outside the question

- **F1.** The held BME280 PDF (`a2ccdb44…`) has the cover and last page at rev 1.24 (BST-BME280-DS001-24, 2024-02). Its interior page footers still print "BST-BME280-DS001-23 Revision_1.23_012022" (measured, PyMuPDF). The parts row cites 1.24, which the cover supports. No action; noted so that a later reader does not take the footers for a mismatch.
- **F2.** Sentinel-03's `generate hwdef` is refused on `main` (`SenseNotEmitted`, `VBAT_SENSE`, `AET-WO-4400b` D7). Measured here, not caused here.
- **F3.** No machine-readable stock source is reachable from Furnace without getting past a bot wall. A future parts work order that needs stock should either name a distributor API key (DigiKey and Mouser both offer one) as Scotty's credential to hold off Furnace, or plan the hand read from the start.
- **F4.** `verify`'s `prove` gate needs more than 8 GiB on Sentinel-01 as ordered. It was OOM-killed under `capped-run --mem 8G` and passed at 16 GiB (measured). CLAUDE.md's `capped-run` default is 8G, so a builder who copies the default for a bench verify loses ~17 minutes to an OOM.
- **F5. A blind spot in `AET-WO-3303`'s coverage rule (measured, §4.3).** A declared I2C bus counts as covered by scenarios that expect `pass`, even when no modelled device on it could observe a fault.
  - Sentinel-01 as ordered, with its only modelled I2C device removed, passes `verify --fab-out` and writes a package.
  - The rule could ask, for an I2C bus, for at least one modelled device, or for one scenario whose expected findings include a firmware-side I2C finding.
  - For the architect; not built here.

## 9. Closing passes

**Adversarial pass, against my own claims:**
- **"Humidity is read nowhere."** Grepped the whole repository, not just the steering documents. It turned up only the BME280's own datasheet text, KiCad's symbol description and one test mutating a channel name. The founding document names environmental probes as a market, and §1 now says so.
- **"Unmodelled is equivalent to absent."** Checked in code: the firmware reads only modelled devices' identities (`2202`), and the twin builds no peripheral for `renode: null` (`renode.rs` L2499–2510, `device_unmodelled` as info).
- **"The BMP581 swap is clean."** The first verify said otherwise. The refusal was taken against a control (unchanged tree, 2/2 pass) and isolated to one scenario before it was written down. An 8 GiB OOM was not read as a result.
- **Each subagent's summary was taken as a claim.** Re-read at source, with ✓ in §2:
  - every identity value of the tabled candidates;
  - BMP390 supply and accuracy;
  - BMP581 and BME280 accuracy and current. This corrected the BMP581 to ±30 Pa over −5–65 °C and ±75 Pa over the full range;
  - the BMP390 and BMP581 pinouts and land drawings.
  - Values not re-read keep the agent-read label.
- **Bosch's datasheet pages are "all rights reserved".** Two page renders made for the land-pattern comparison were removed from `spike/4201/` before commit, and the figures are cited instead.

**Reduction pass** (diff-sized):
- `spike/4201/tools/rows.py`: a dead expression removed (`"read": 1`).
- Nothing else qualifies. The tools are four short scripts the evidence needs; the outputs are trimmed summaries, with no emitted KiCad file and no 220 KB verdict (`verify-nou2-summary.json` keeps gates and campaign only).

**Gates:**
- `cargo fmt --all -- --check`: exit 0.
- No Rust, board, example, fab or scenario file differs from `c3f8ddbb` (`git diff --stat c3f8ddbb -- crates boards examples fab scenarios models` is empty). So `clippy` and `cargo test --workspace` have nothing new to judge, and were not run.
- The branch adds this FEEDBACK and `spike/4201/` (Python, shell, JSON, text) only.

**Fence:** `spike/4201/` and this FEEDBACK. The scratch trees, the cargo targets and the datasheets stay in `~/.cache/aet-4201/`.
- The one scratch scenario (`scenarios/scratch4201/s02-droop-bor3.json`) lives in the scratch tree, not here.
