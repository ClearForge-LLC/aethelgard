# AET-WO-4600 — predictions for the last three runs (architect's ruling, 18:06Z)

Written 2026-11-19 ~19:10Z and committed before any of these runs.
- **The ruling:** `prove` runs at `--mem 16G` on any board whose schedule harness is unsplit (no timer, `svd-hal/src/board.rs` L3897), whatever its buses. `run.sh` now keys `PROVE_MEM` on `motor_outputs` being empty.
- **Order:** `driver-tree`, then `logger-spi-only-cap`, then `logger-offboard`, one container at a time, each after `free -g` shows the cap plus 4 GiB available.
- **The CLI binary is unchanged** (sha `f9be51d0…`). The cache's build intermediates were pruned for disk. If `build` must now recompile the firmware's dependencies offline, it is slower; **guess:** it still exits 0.

| Spec | Step | Exit | Verdict / kind | Class | Basis |
|---|---|---|---|---|---|
| `driver-tree` | validate | 0 **guess** | ok, A1's note | — | `power.rs` L3020–3029 |
| `driver-tree` | solve, emit, erc, drc, support, speeds, extract | 0 **guess** | pass; the tree's parts drawn (1315 not merged); `extract` 0 lines | — | |
| `driver-tree` | generate, build, prove (8G, split per timer), simulate | 0 | pass | — | M |
| `driver-tree` | campaign | 2 | `PowerNotSimulated` | (c), owner `2304` | `cli/src/campaign.rs` L380–389 |
| `driver-tree` | replay, attribute | not_run | no archived run | — | |
| `driver-tree` | verify | 0 **guess** | pass | — | |
| `driver-tree` | verify-fab | 3 | `CampaignCoverageShort`: `VDD_5V` (not the MCU's supply), `battery` | (c), owner `2304` | `verify.rs` L1223–1232, L930–944 |
| `driver-tree` | hwdef, pinout | 0 **guess**, 3 | written; `NoSymbolTable` | not a gate | |
| `logger-spi-only-cap` | validate … build | 0 | pass; `extract` now bounds VIB's SCK and MOSI with U40's declared 10 pF | — | Sentinel-03's U5 |
| `logger-spi-only-cap` | prove (16G) | 0 | `every_valid_sequence` passes, peak between 7.7 GB (where 8G was passed) and 13 GB | — | F11 |
| `logger-spi-only-cap` | simulate | 0 | pass, boot word near 39 ms | — | `logger-spi-only` |
| `logger-spi-only-cap` | campaign | 1 | 3/5: `droop-bor3` `brownout_unrecovered`; `spi-absent-u40` unmet (`spi_data_mismatch` beside the two expected kinds) | (b) F1; the second is my scenario's expectation | `logger-spi-only` |
| `logger-spi-only-cap` | verify | 0 | pass | — | |
| `logger-spi-only-cap` | verify-fab | 2 | `UnwitnessedFieldNotAcknowledged` | (a) | |
| `logger-spi-only-cap` | hwdef, pinout | 2, 3 | `ImuOrientationUndeclared`; `NoSymbolTable` | not a gate | |
| `logger-offboard` | validate … build | 0 | pass | — | |
| `logger-offboard` | extract | **1** | fail on `TEMP_SDA` and `TEMP_SCL`. Each line carries J5's declared 65 pF off board, plus three on-board TMP117 pins and the MCU's at 10 pF each, plus copper: about 105–120 pF against the 100 pF bound. My cable condition is too long for the bus as declared | (a): a correct refusal of a bad specification; F8 stands (the probe's TMP117 is still not declarable) | `extract`'s count of off-board capacitance (`ir/src/lib.rs` L1675) |
| `logger-offboard` | prove (16G) | 0 | as L, about 17 min, 8–13 GB | — | F11 |
| `logger-offboard` | simulate | 0 | pass, U10–U12 unmodelled | (d) | |
| `logger-offboard` | campaign | 1 | 5/6: `droop-bor3` `brownout_unrecovered` | (b) F1 | |
| `logger-offboard` | verify | **1** | fail at the `extract` gate | (a) | |
| `logger-offboard` | verify-fab | 2 | `UnwitnessedFieldNotAcknowledged` (checked before any gate) | (a) | |
| `logger-offboard` | hwdef, pinout | 0, 3 | written (TMP117s not emitted); `NoSymbolTable` | not a gate | |

**Resources:** no run past 900‰ of its cap. The 16G proofs peak at 13 GB or less (814‰).
