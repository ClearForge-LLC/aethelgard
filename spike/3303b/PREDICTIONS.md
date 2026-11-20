# AET-WO-3303b predictions (written before any after-run)

Written 2026-11-19 on `bba3e60c` (the split), before any Kani run, `verify` or tier-2 test on split output. Committed alone. The after-runs read the tree of the commit that adds this file (`git archive`, `~/.cache/aet-3303b/after-tree/SHA`).

**Method.** Each row is one container under `~/bin/capped-run --mem 16G`, `nice -n 10`, with one container of mine at a time (`sh/img.sh`, `sh/step.sh` from `AET-WO-4600`). Wall time runs from the step's start to its end, so it includes cargo-kani's compile of the crate. Peak is the scope's `memory.peak`, page cache included, so it is an upper bound. Every row is n = 1 on a shared Furnace: wall times move ±30 % with the other builders' load, and peaks do not.

**What informs the predictions.**
- The before-run on `main` 3f989435: `boards/sentinel-01`'s `every_valid_sequence` took 314.3 s and 7.43 GB.
- `AET-WO-4600` F11 on the 4600 boards: 110.7 s and 4.62 GB for `logger-i2c-only`; 390.4 s and 10.65 GB for `logger-spi-only-cap`.
- `AET-WO-3303`'s motor parts: `i2c1` took 12–13 s, `spi1` 36–37 s and `tim1` 44 s.
- `AET-WO-4600`'s driver: `tim1` and `tim2` peaked at 2.49 GB; its `gpio` part took 2.9 s and 0.22 GB.

## 1. The parts each board now emits (read from the generator's output, not proved)

| Board | Before | After |
|---|---|---|
| `boards/sentinel-01` (`sentinel-01-minimal.json`) | `every_valid_sequence` | `_i2c1`, `_spi1` (PA0 is SPI1's chip select, so there is no `gpio` part) |
| `boards/sentinel-01-bench` (`sentinel-01.json`) | `every_valid_sequence` | `_i2c1`, `_i2c2`, `_spi1` |
| `logger-i2c-only` (4600, re-pinned) | `every_valid_sequence` | `_i2c1`, `_i2c2`, `_gpio` (an output no unit owns) |
| `logger-spi-only-cap` (4600, re-pinned) | `every_valid_sequence` | `_spi1`, `_spi2` (two SPI parts that each schedule the one kernel selection) |

Every other harness is unchanged. Its text is byte-identical on both committed boards, function by function. Predicted: the same exit as before, the time within ±30 %, the peak within ±10 %.

## 2. Each part, predicted

| Board | Part | Exit | Time (range) | Peak (range) |
|---|---|---|---|---|
| sentinel-01 | `_i2c1` | 0 | 15 s (8–45) | 1.0 GB (0.5–2.0) |
| sentinel-01 | `_spi1` | 0 | 45 s (20–120) | 2.0 GB (1.0–3.5) |
| sentinel-01-bench | `_i2c1` | 0 | 15 s (8–45) | 1.0 GB (0.5–2.0) |
| sentinel-01-bench | `_i2c2` | 0 | 15 s (8–45) | 1.0 GB (0.5–2.0) |
| sentinel-01-bench | `_spi1` | 0 | 45 s (20–120) | 2.0 GB (1.0–3.5) |
| logger-i2c-only | `_i2c1` | 0 | 15 s (8–45) | 1.0 GB (0.5–2.0) |
| logger-i2c-only | `_i2c2` | 0 | 15 s (8–45) | 1.0 GB (0.5–2.0) |
| logger-i2c-only | `_gpio` | 0 | 4 s (2–15) | 0.3 GB (0.1–1.0) |
| logger-spi-only-cap | `_spi1` | 0 | 45 s (20–120) | 2.0 GB (1.0–3.5) |
| logger-spi-only-cap | `_spi2` | 0 | 45 s (20–120) | 2.0 GB (1.0–3.5) |

**Totals for the schedule (the sum of the parts' times; the largest part's peak).**

| Board | Before | After |
|---|---|---|
| sentinel-01 | 314 s, 7.43 GB (measured) | about 60 s, 2.0 GB |
| sentinel-01-bench | not yet measured: predicted 350–700 s and 8–12 GB, more than Sentinel-01 for the second I2C | about 75 s, 2.0 GB |
| logger-i2c-only | 4600's 110.7 s and 4.62 GB: predicted again within ±30 % and ±10 % | about 35 s, 1.0 GB |
| logger-spi-only-cap | 4600's 390 s and 10.65 GB: predicted again within ±30 % and ±10 % | about 90 s, 2.0 GB |

**No part's peak exceeds 8 GiB.** Predicted largest peak among the parts: under 3.5 GB, on an SPI part.

## 3. The old name, and `verify`

- `prove --harness proofs::every_valid_sequence` on a split crate: refused, nonzero exit and verdict not `pass`, because no harness has that name (`kani.rs`'s `no_harness` and `harness_not_found` paths). I do not predict which of the two fires.
- `verify --spec` with no scenarios:

| Board | Before | After |
|---|---|---|
| sentinel-01 | pass | pass; a prove gate per harness, both parts among them, and no `every_valid_sequence` gate |
| sentinel-01-bench | pass | pass; the same, with three parts |
| logger-i2c-only | pass (4600: 152.8 s, 4.77 GB) | pass, peak under 3.5 GB |
| logger-spi-only-cap | exit 1 at `extract` (4600 F18: `LOG_SCK` 37.5 pF against 30 pF; 462 s, 10.68 GB), not a regression | exit 1 at `extract`, unchanged, peak under 3.5 GB |

- After, each `verify` is shorter by about the whole harness's time less the parts' sum. Its peak is the largest of its gates; the routing and Kani gates are predicted under 3.5 GB.

## 4. The cap (§1.5)

Predicted from the after-numbers: `prove` fits `--mem 4G` on every board without a timer, and `verify` fits the 8 GiB default with margin. The recommendation will be the measured largest peak plus a margin, rounded to a cap a caller can pass. It is not this table.

## 5. What would be a stop (§7), restated

- A part fails where the whole passed (the before rows), or the whole fails where every part passes.
- `check_parts` refuses one of these four boards. The generator refuses then, so `generate` and the re-bless would fail. Neither did (§1), so this is already measured not to fire.
- A part above 8 GiB is not a stop. It would be recorded and would set §4.
