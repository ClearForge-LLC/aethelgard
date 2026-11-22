# AET-WO-2307 §1.1 — predictions, committed before any run

Base: `main` `adb8f61a`. Written 2026-11-20 before the probe or the CLI ran on either board.

## P1. Sentinel-03, today's refusal (`verify --fab-out`, `scenarios/sentinel-03/*`, scratch output)

- Exit 2, `UnwitnessedFieldNotAcknowledged`, `id` `RCC.AHB4ENR.GPIOCEN`, **280 rows across 35 registers** (`2304b` Q11 and its §2 re-run). Anything else is the §7 stop.
- (register, class) pairs among those 280 rows: **40** (range 35–45). Most registers carry one unwitnessed class among their missing rows; a few (GPIO `MODER`/`AFR` and `TIM1` `CCMR`) may carry two.
- Sentinel-03 already holds 30 entries. Pairs that fall on an existing entry (reported *completed*, reason kept): **about 20**. The rest (about 20) are new, with reason `""`.
- Fields over all pairs: missing fields total 280 (one field per row, no row repeats a (register, field, class)). Completed entries add their existing fields on top.

## P2. Board M, as `4600` built it

- `spike/4600/driver.json` is schema 16 and is refused today as `UnsupportedSchemaVersion` (checked by `validate`, not a measurement). M is held at schema 22 as `crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json` (project `bench-motor-driver`), whose 44 entries / 254 fields later work orders kept current.
- With those 44 entries removed (scratch copy, its droop-bor3 scenario re-based on it): exit 2, `UnwitnessedFieldNotAcknowledged`, **254 rows**, and `entries` are **44 entries / 254 fields**, every one new (reason `""`).
- The committed 44 entries, each `reason` emptied and its fields sorted, **equal `entries` field for field**, in the refusal's order (register, then class).

## P3. Cost

- Each refusal before any gate: under 5 s, no container (it precedes every gate; `4600` measured 1.4 s on M).
