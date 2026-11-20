# AET-WO-2305 §1.1 predictions, written before the runs

Committed before `sh/run-all.sh` runs. One trial came first: `s01min` (Sentinel-01 minimal: its undisturbed boot, `droop-bor3`, `droop-tolerated`), run once to learn the log's format. Its numbers were seen, so the rows marked *seen* are not predictions.

**How each column is read** (`sh/analyze.py`):
- the release, from kind 18;
- the boot word's instant, from the `AETHELGARD-BOOTWORD` hook's instruction count after the last release, which is exact (start + count × 10 ns);
- the first identity line's instant, from Renode's stamp on the first check's first expected line. The stamp is good to one quantum only (stamp ≤ the instant < stamp + 100 µs);
- the witness's lines at the probe, from kind 19, and the first device's lines up to the boot word, from the log;
- the verdict.

| Run | Release (ns) | Boot word after the boot's start | Witness lines at the probe | First device's lines to the boot word | Verdict |
|---|---|---|---|---|---|
| S-01 minimal `droop-bor3` | *seen:* 1,231,690 | *seen:* 25.7 µs | *seen:* 2 (U2's `Write D0`, `Read 60`: one step) | *seen:* 2 | *seen:* pass |
| S-01 minimal `droop-tolerated` | none (no BOR crossing) | — | — | — | *seen:* pass |
| S-01 bench `droop-bor3` | 1.22–1.24 ms | 25–40 µs | 2 | 2 | pass |
| `motors-dshot600` `dshot600-droop` (droop at 100 µs) | 0.32–0.34 ms | 25–60 µs | 2 | 2 | pass |
| `four-layer-droop` | ~1.23 ms | 25–40 µs | 2 | 2 | pass |
| `spi-only-droop-bor3` | ~1.23 ms | 20–40 µs | 2 (U3's identity) | 2 | pass |
| Sentinel-03 `droop-tolerated` | none | — | — | — | pass |
| Sentinel-02-shaped `s02-droop` (`2303` F11) | 0.32–0.34 ms | 39.0–39.5 ms (1.95 × the ICM-42688-P's 20 ms, plus the reads) | 2 | 4–8 (BME280 identity, configuration, data) | **fail, `brownout_unrecovered`** (boot word unset at +100 µs) |
| `4600`'s logger `droop-bor3` | ~1.23 ms | 19.5–19.7 ms (1.95 × the BMP581's 10 ms) | 2 (U10's identity) | 4–8 | **fail, `brownout_unrecovered`** |
| `sentinel-01-ext-bmp581` `droop-bor3` | ~1.23 ms | 19.5–19.7 ms | 2 | 4–8 | **fail, `brownout_unrecovered`** |

**Undisturbed boots.** Each board's boot word, counted from 0, equals its post-release boot word counted from the boot's start, to the instruction. The reset re-applies the cold state, so the boot is the same program from the same state.

**Time base.** The twin's settle is 1.95 ± 0.01 × the declared settle on every board that has one: ext-bmp581 and the logger at 10 ms, the Sentinel-02 shape at 20 ms. The spin's code, HSI's fastest clock and Renode's 100 MIPS are the same on every board, so I predict a constant. The non-settle part of the boot is board-dependent and in the tens of µs.

**Verdicts on `main`.** Sentinel-01's runs and the four other committed droops pass. The three waiting boards fail by `brownout_unrecovered` at the fixed instant.
