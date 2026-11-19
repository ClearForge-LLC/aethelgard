# AET-WO-4600 — predictions for the runs after the stop was lifted

Written 2026-11-19 ~17:25Z, after the architect's ruling (bus, 17:08Z), and committed before any of these runs. The ruling:
- the stop is lifted;
- `prove` on a board with an I2C bus runs at `--mem 16G`;
- M's stage-2 `verify --fab-out` runs, and so does every run on the not-run list;
- a run past 900‰ of 16G is still a stop.

`spike/4600/PREDICTIONS.md` (`e57f7cb8`) stays as committed. What it predicted for these rows still stands where this file does not correct it.

Since the first batch, three things changed in `run.sh` and `sh/step.sh`:
- `prove` at 16G when the specification has an I2C bus;
- an in-scope watch that kills a step past 900‰ of `memory.max` (exit `killed-900`);
- a wait for `free -g` available ≥ cap + 4 GiB before each container.

These predictions build on what the first batch measured: F1 (a BOR droop fails on any board whose first scripted device waits), F11 (unsplit schedule harness), and the exit-code miss (the acknowledgement refusal exits 2).

| Spec | Step | Exit | Verdict / kind | Class | Basis |
|---|---|---|---|---|---|
| M stage 2 (acknowledgements written) | validate | 0 | ok | — | the reasons now pass `ir/src/board.rs` L575–585 |
| M stage 2 | verify-fab | 0 | pass, package written; coverage met with no output faulted; manifest sha256 quoted only | **(b)** F3 | M's stage-1 gates all passed; 41 entries cover the 250 rows |
| L stage 1 | verify-fab | 2 | `UnwitnessedFieldNotAcknowledged`, 188 rows (coverage passes first: droop-bor3 counts as declared) | (a) | `verify.rs` L1223–1246 |
| L | hwdef | 0 **guess** | written; no IMU line (L has none); TMP117 not emitted (`ardupilot` null); BMP581 a baro line; W25Q64 dataflash | not a gate | `parts.json` `ardupilot` |
| L | pinout | 3 | `NoSymbolTable` | not a gate | as M, `pincheck/src/elf.rs` L127 |
| L stage 2 | validate | 0 | ok | — | |
| L stage 2 | verify-fab | **1** | fail: the campaign gate fails on `droop-bor3` (`brownout_unrecovered`, expectation unmet); no package | **(b)** F1 | first batch's campaign |
| `logger-1s-u50` | validate, solve, emit, speeds, generate, verify | 2 | `BatteryOutsideRegulatorInput` (the cell's 3,000 mV under the buck's 3,800 mV VIN) | (a) + (c) | `ir/src/power.rs` L1316–1340 |
| `logger-i2c-only` | validate … simulate | 0 | pass; prove at 16G, `every_valid_sequence` unsplit, peak 8–13 GB | — | as L |
| `logger-i2c-only` | campaign | 1 | 4/5 met; `droop-bor3` `brownout_unrecovered` (the BMP581 is still there) | (b) F1 | |
| `logger-i2c-only` | verify | 0 | pass | — | |
| `logger-i2c-only` | verify-fab | 2 | `UnwitnessedFieldNotAcknowledged` | (a) | variants stop at stage 1 |
| `logger-i2c-only` | hwdef · pinout | 0 · 3 | written · `NoSymbolTable` | not a gate | |
| `logger-spi-only` | validate … build | 0 | pass | — | |
| `logger-spi-only` | prove | 0 **guess**, at 8G (no I2C bus) | the unsplit schedule over two SPI buses. **Risk:** if it needs more than 8G it is killed, and that is a stop again | — | F11 |
| `logger-spi-only` | simulate | 0 | pass; U40 (ICM42688P) and U30 witnessed; boot word after the ICM's 20 ms settle | — | |
| `logger-spi-only` | campaign | 1 | `droop-bor3` `brownout_unrecovered` (the ICM's settle; F1 on a board with no I2C read, the witness falling back to SPI); `spi-absent-u40` fails as expected, **guess**: the same two kinds as the flash's | (b) F1 | |
| `logger-spi-only` | verify · verify-fab | 0 · 2 | pass · `UnwitnessedFieldNotAcknowledged` | — · (a) | |
| `logger-spi-only` | hwdef | 2 **guess** | `ImuOrientationUndeclared` (U40 declares no orientation) | not a gate | `hwdef/src/lib.rs` L149–151 |
| `logger-spi-only` | pinout | 3 | `NoSymbolTable` | not a gate | |
| `logger-offboard` | validate … simulate | 0 | pass; `extract` counts J5's 65 pF on TEMP's lines and stays under 100 pF (**guess**) | (c) F8 | |
| `logger-offboard` | campaign | 1 | `droop-bor3` `brownout_unrecovered` | (b) F1 | |
| `logger-offboard` | verify · verify-fab · hwdef · pinout | 0 · 2 · 0 · 3 | as `logger-i2c-only` | | |
| `driver-tree` | validate | 0 **guess** | ok, with A1's note (device currents not in the load) | — | `power.rs` L3020–3029 |
| `driver-tree` | solve … simulate | 0 **guess** | pass; prove at 8G, split per timer | — | |
| `driver-tree` | campaign | 2 | `PowerNotSimulated` | (c), owner `2304` | `cli/src/campaign.rs` L380–389 |
| `driver-tree` | replay · attribute | not_run | no archived run | — | |
| `driver-tree` | verify | 0 **guess** | pass | — | |
| `driver-tree` | verify-fab | 3 | `CampaignCoverageShort`: `VDD_5V` (not the MCU's supply), `battery` | (c), owner `2304` | `verify.rs` L1223–1232, L930–944 |
| `driver-tree` | hwdef · pinout | 0 **guess** · 3 | written · `NoSymbolTable` | not a gate | |

**Resources (guess):**
- I2C boards: `prove` 8–13 GB, `verify` 12–14 GB, `verify-fab` about the same plus the campaign. None past 900‰ of 16G (14.7 GiB).
- `logger-spi-only`'s 8G `prove` is the one at risk.
