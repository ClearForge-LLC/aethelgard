# AET-WO-4411 lever (iv), scratch spike (never committed into a crate)

`route_ground`: each of U1's GND pads to its own via within D of its centre (no joining other copper,
no early exit). Sentinel-03 with U30 on USART6 PG14/PG9, `rank_apart` placement. Emit, then extract.

| D (mm) | emit | rung | worst VDD (pin, mΩ) | worst VSS (pin, mΩ) | drop (≤ 110 mV) | U11 F.Cu (≥ 700) | outline |
|---|---|---|---|---|---|---|---|
| none (USART6 as committed) | ok | 5 | 121, 54.7 | 31, 62.5 | 134.3 | 1573 | 85.65 × 96.6 |
| 1.29 (ruled start) | Unroutable: "cannot take U1 pad 107 to a via" | – | – | – | – | – | – |
| 1.54 | ok | 5 | 30, 59.7 | 31, 42.4 | 113.0 | 1503 | 85.65 × 96.6 |
| 1.64 | ok (same board as 1.54) | 5 | 30, 59.7 | 31, 42.4 | 113.0 | 1503 | 85.65 × 96.6 |
| 1.76 | ok | 3 | 17, 58.4 | 31, 41.9 | 111.4 | 667 | 85.65 × 96.6 |

Before the spike, the VSS bounds against the nearest GND via on the boards on disk: 1.2875 mm gives
35–37 mΩ, 1.63 mm about 39, 3.38 mm about 62 (every pin, three layouts).
