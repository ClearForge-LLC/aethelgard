# AET-WO-1219 — predictions

Written and committed 2026-11-22, with `RULES.md` and before any file under `crates/aethelgard-ir` is opened.

**What had been seen when these were written.** Nothing here is blind beyond this list:
- the work order, and its examples: AF numbers, register names, DS12117 FT/TT, `io.analog_pads`, VOS ranges, RCC clock names, `mcu.part` strings, HSE/MCO;
- `README.md` and `docs/northstar.md`;
- the architecture's outline and `RUL-08`;
- the roadmap's audit rows, and the Grok response §2.5;
- the file names and line counts of `crates/aethelgard-ir` (`clock.rs`, `vos.rs`, `speeds.rs`, `ds009.rs`, `silicon.rs`, `pin_parts.rs`, `power_parts.rs`, `data/parts.json`, `data/power-parts.json`, …; about 30,000 source lines);
- the builders' memory index, which names specification 27, HSE, VOS, the DS12117 support network and the pin devices.

A search of the box found no RP2350 datasheet. The RP2350 prediction is therefore known, not predicted.

| # | Quantity | Prediction |
|---|---|---|
| P1 | K entries (serde fields reachable from the specification) | 320, range 200 to 450 |
| P2 | K by class | V 72 %, N 12 %, S 16 %, D 0 (by rule 1) |
| P3 | K flagged `?` (tie-break a) | 5 to 15 |
| P4 | R entries (refusal kinds) | 180, range 100 to 280 |
| P5 | R by class | V 75 %, N 10 %, S 15 % |
| P6 | H tables | 10, range 5 to 16: D 6, S 4. The S tables are the pin table (FT/TT), the AF map, VOS, and the speed grades |
| P7 | Where S concentrates | `mcu.*`, the clock tree (HSE/MCO/kernel clocks), VOS, the pin speed grades, the pin structure (`io.analog_pads`, FT/TT), AF numbers |
| P8 | S keys by version | over half introduced in versions 18 to 27 (the P4 flight-controller work), the rest mostly in P1's versions |
| P9 | Keys missing from every committed specification (source minus data) | 15 to 40 optional keys, and at least one enum variant no specification uses |
| P10 | The lean | 1 to 4 S keys named to change before the epoch. Two reasons: an ST value set sits at a top-level or cross-peripheral key rather than under an MCU-scoped block, or a refusal kind's string is an ST register name. The rest are left as additive |
| P11 | RP2350 column | "not held" in every cell (known, see above) |
