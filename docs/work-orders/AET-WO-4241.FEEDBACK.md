# AET-WO-4241 — FEEDBACK (STOPPED at §1.1: the twin's response to an unmodelled read is a failed modelled device's, byte for byte)

**Branch and inputs.**
- Branch `wo/aet-4241-read-every-identity`, from `main` at `d70a99e5`.
- The work order is read from `~/.cache/aet-4241-wo.md`, and the kickoff from `~/.cache/aet-4241-from-architect.md`.

Each statement is labelled:
- **measured:** run on Furnace in the pinned tools image (`container/IMAGE`), under `capped-run --mem 8G`, nice 10;
- **read at source:** file:line at `d70a99e5`;
- **inferred.**

## 0. Status

**Stopped at §1.1 under §7's first condition.** The predictions were committed before every run (`dcee6433`,
`207fe5a8`, `7df1b14b`), and nothing is built on this branch. The selection that §1.2 asks for was measured in a
spike tree only (`spike/4241/sh/instrument.diff`), so `src` is unchanged. The ruling I ask for is in §3.

## 1. The measurement table (§1.1), with the predictions beside it

**Static (`spike/4241/sh/survey.py`, `out/survey.md`; run before the predictions were written).**
- **The specifications:** 42 committed specifications hold 201 devices, and 192 of those devices have a part row
  with an identity.
- **The unmodelled ones:** 78 of the 192 are unmodelled, all **TMP117** (`renode: null`), in 28 specifications.
- **SPI:** TMP117 is the only part in the table with no model, so **no committed specification has an unmodelled
  SPI device**. The SPI rows below use a spike-only lever, `AET4241_UNMODEL` in `model::kind`, in
  `sh/instrument.diff` only.

**Dynamic (measured; `out/runs.md`, raw boot logs in `out/raw/`).**
- "Old" is `main`'s generator, and "new" is the generator with every identity read kept.
- Both were judged by `main`'s judge, instrumented to print the boot's log and the 48 RAM words from the boot word.

| case | prediction | measured |
|---|---|---|
| TMP117 read on I2C (C1–C5, C8) | status `timeout_txis`, value 0; one `Unknown slave at address 72.` | **as predicted**: `0x22020001`, `0x00000000`; one `[WARNING] i2c1: Unknown slave at address 72.` per read (C8: one per address, 72–75, on each of four controllers) |
| …against a modelled device that failed | byte for byte 3305's `wrong-address` U2 | **as predicted**: 3305's U2 sent to 0x75 left `timeout_txis`, 0 and `Unknown slave at address 117.` (`spike/3305/out/records.txt`) |
| SPI read to an unmodelled chip select (C6, C7) | status `ok`, value 0; multiplexer `Tried to transmit` lines | status **as predicted**, `0x22020000`; value **missed**: `0x00ffffff`, the multiplexer's dummy byte `0xFF` three times; lines **as predicted**: `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte`, one per byte |
| …against a modelled SPI device that failed | a mismatch | the same lines a modelled device absent behind its chip select leaves (`4200` D4's `unrouted` evidence); the value is the part's `0xFFFFFF`, a mismatch |
| one `TXIS` wait | 100k–300k instructions (1–3 ms) | **missed: 3.6 ms on the Sentinel-01 family** (C1's boot word 2,662 → 362,757 instructions; C2–C4 alike), **4.8 ms on Sentinel-03 and the `d3` fixture** (C5's TMP117 status written at 486,699; C10, about 480,100 a wait). The 20,000 spins cost what that board's code makes of them |
| 16 waits (C8, C10) | C8: the boot word at 16–48 ms, inside 100 ms; C10 (batch 3): about 58 ms | inside, **both times missed**: C8 at 57.6 ms (5,764,965 instructions), C10 at 76.9 ms (7,699,763). By extrapolation, 21 to 28 unmodelled I2C reads, by board, would not fit in `MAX_VIRTUAL_NS` (inferred) |
| old against new firmware, with a TMP117 | differ | **as predicted**: C1–C5 and C8, each `main.rs` and ELF |
| the `4600` pair and the control (C9, batch 2) | old: with U3 equals without U3; new: they differ; C9 old = new; C9 `pass` with no `device_unmodelled` | **as predicted, all four**: old `2bb1be79…` both (`4600` F19 reproduced); new with U3 `406af077…`, without U3 `2bb1be79…`; C9 `pass`, findings `[]` |
| today's judge, C1/C2 | U4 reads TMP117's slot: `spi_record_invalid`; U2 `absent` `[72]`; `fail` | **as predicted**: `spi_record_invalid` (status `0x22020001` on SPI); U2's `absent` `["0x48"]`; `fail` |
| today's judge, C3 | as C1; U2, U5 `absent` `[72]` | **as predicted** |
| today's judge, C4 | U5's records shift; U5 and U4 fail | **as predicted**: `i2c_timeout`, `i2c_configure_mismatch` and `i2c_data_mismatch` on U5, `spi_identity_mismatch` on U4 |
| today's judge, C5 | U7's records shift and fail; U7 identity `absent` `[72]` | **as predicted**, and every later record too: U4, U5 (`spi_identity_mismatch`, `spi_configure_mismatch`, `spi_record_invalid`) |
| C6 (SPI unmodelled), today's judge | `pass`, `device_unmodelled` ×2 | **as predicted** |
| C7 (every device unmodelled) | `pass` on the boot witnesses alone, `device_unmodelled` ×3 | **as predicted**: `pass`, no record judged |
| C8 (16 TMP117) | `fail`: shifted BME280 records | **as predicted**: `i2c_timeout` on S6, S7, S12, S13, `spi_record_invalid` on S1000; every BME280's `absent` holds 72–75 of its controller |
| C10 (12 unmodelled W25Q64, lever) | `ok`, no SPI wait; `fail` from shifted BME280 records | **as predicted**, but for the time above: 48 `Tried to transmit` lines (12 reads × 4 bytes), no SPI timeout; `fail` (`i2c_timeout` ×4) |

## 2. The stop

§1.1 says: stop and report if the twin's response to an unmodelled read cannot be told apart from a modelled
device that failed. **It cannot.**

- **On I2C.** Renode 1.17.0 models no NACK. An address with no model draws `TXIS` never rising, a `timeout_txis`
  record with value 0, and one `Unknown slave at address N.` line. A modelled device that is absent, or that the
  firmware addresses wrongly, draws the same three, byte for byte (3305's `wrong-address`).
- **On SPI.** The controller completes the transfer whatever is behind the chip select. The record reads `ok` with
  the multiplexer's dummy bytes (`0xFF…`), and the multiplexer logs `Tried to transmit byte … not connected` for
  each byte. A modelled device absent behind its chip select (`4200` D4) draws the same.

**What the judge would need (the design I would build on a ruling):**

1. **The classification comes from the plan, never from the response.**
   - A device's read is "not witnessable" exactly when `aethelgard_twin::model::kind(part)` is `None`, which is
     the parts table's `renode: null`.
   - That uses the specification and the table only, never the firmware (`INV-07`).
   - So the indistinguishability above cannot hide a modelled failure: a modelled device is never classified by
     what it answered.
2. **Each unmodelled identity record gets its own slot, placed by the twin.** The twin places it in the
   firmware's order, from its own copy of the layout.
   - Today `boot::identity` places records from modelled checks alone, so every later record shifts (C1–C5, C8).
   - The record stays out of `checks`, so nothing that reads `checks` (the witness, the stimulus, the probe's
     `checks.first()`, kind-12 trace records, evolve's `i2c_timeout` count) sees it.
3. **The evidence goes to the unmodelled step that drew it.**
   - **I2C:** the unmodelled step's status word gets `3305`'s step hook, so `absent_by_step` gives its
     `Unknown slave` line to it. Without that, the line falls back to every I2C check on the controller (C1–C4),
     or to the next modelled step (C5's U7).
   - **SPI:** the unmodelled device's chip-select window counts as a window, so its multiplexer lines go to no
     modelled step. Today a line inside no modelled window goes to every modelled device's identity step on that
     multiplexer (`renode.rs` L1120–1133).
4. **The named outcome.**
   - Each unmodelled identity read is reported as `identity_not_witnessable` (info), with the part, the reason
     (no Renode 1.17.0 model) and what the record holds, as information only.
   - It replaces `device_unmodelled`, whose "its identity was not read" becomes false, for generated firmware.
     The foreign judge keeps `device_unmodelled`.
   - In `verify`, each read is a `twin_limits` row beside the ledger, class `silicon-only` ("nothing: no model
     exists for it; only the bench measures it"). It is never acknowledged and never blocking, as every twin
     limit is.

## 3. Rulings asked

- **D1. The stop.** May I build to §2's four points, which classify by the plan? Or is a response-level
  distinction required, which would need a model (out of this order's scope)?
- **D2. A record the firmware never wrote.** Suppose an unmodelled device's record still reads `0xFFFFFFFF`, and
  no step hook fired. The twin then witnessed that **the read was not made**, which is a firmware defect, not a
  device judgement.
  - **(a), recommended:** that is `i2c_record_invalid` / `spi_record_invalid` (error), as for a modelled record.
    "Never pass, never fail" then holds only for a read that was made.
  - **(b):** always `identity_not_witnessable`, whatever the record. The `answers`-filter mutant is then killed
    by a generator test only.
- **D3. Time.**
  - Each unmodelled I2C read costs 3.6 to 4.8 ms of virtual time in the twin, which is a `TXIS` wait silicon
    never makes with the part fitted.
  - Every later event moves by it: the boot word, the serial frames' anchor, a droop's recovery bound.
  - §1.4's re-judge will show whether any modelled verdict moves by time alone. If one does, I will stop again
    under §7.
  - A board with 21 to 28 or more unmodelled I2C devices, by board, would not boot inside `MAX_VIRTUAL_NS`
    (inferred, from C8 and C10). Its boot word would be missing, a named failure, never a silent pass. It is
    noted, and not built for.

## 4. Fence and owed

- **Pushed:** `spike/4241/` and this file, nothing else.
- **On a ruling:** the selection in `firmware.rs` (§1.2, measured in `instrument.diff`) and the twin's record
  placement are drafted locally, and are not pushed.
- **Gates:** none apply to a spike-only branch. No `src` changed.
