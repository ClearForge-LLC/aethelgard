# AET-WO-4411 lever (1), directional GND escapes: scratch spike (never committed into a crate)

Ruled 2026-11-25T13:05Z. Patch: `spike/4411/escape/lever1-scratch.patch` (against `route.rs` at `5557314e`; reverted after the runs).
In `route_ground`, each of U1's GND pads gets its own via within D of its centre, with no joining of other copper and no early exit (lever (iv)'s spike), and the via must lie on an allowed side.
Sentinel-03 with U30 on USART6 PG14/PG9, `rank_apart` placement, the gate at 62.5 µm. Emit, then `extract`; DRC in the image (`kit/img.sh`, `capped-run` 8 GiB).

**Readings of "the side away from the nearest VDD pins and from the plane's path between the feed (L2 pad 2) and those pins":**
- **R1:** the via's antipad (via radius plus the board's clearance, 0.56 mm) lies wholly in the half-plane through the pad's centre facing away from the feed itself, and away from the centroid of U1's nearest VDD_3V3 pads.
- **R2:** the pad's long axis has two ends, and the end facing the feed is the path's. The antipad lies wholly beyond the pad's centre at the other end, so the left and bottom rows escape outward and the top and right rows inward. The via is also not nearer the centroid of the nearest VDD pads (projection at most 0).

| run | D (mm) | emit | rung | worst VDD (pin, mΩ) | worst VSS (pin, mΩ) | drop (≤ 110 mV) | U11 F.Cu (≥ 700) | outline | DRC / `--spec` |
|---|---|---|---|---|---|---|---|---|---|
| lever (iv), no direction (for reference) | 1.54 | ok | 5 | 30, 59.7 | 31, 42.4 | 113.0 | 1503 | 85.65 × 96.6 | – |
| R1 | 1.54 | Unroutable: `cannot take U1 pad 130 to a via` (3 nodes allowed) | – | – | – | – | – | – | – |
| R2 | 1.54 | Unroutable: `cannot take U1 pad 16 to a via` | – | – | – | – | – | – | – |
| R2 | 1.76 | Unroutable, pad 16 | – | – | – | – | – | – | – |
| R2 | 2.00 | Unroutable, pad 16 | – | – | – | – | – | – | – |
| **R2** | **2.50** | ok | 5 | 121, 54.5 (pin 30 now 42.2) | **16, 49.8**; 31, 46.5 | **118.3, fail** | 1503 | 85.65 × 96.6 | the same 24 `drill_out_of_range` (U10/U11 thermal vias) as every S03 board; `--spec` pass |
| R2 | 3.00 / 4.00 | ok (rungs 6 / 2) | – | not extracted | – | – | – | – | – |

**Why the drop fails:**
- **VDD recovers, as the rule intends:** pin 30 goes from 59.7 to 42.2 mΩ, and the worst is pin 121 again at 54.5.
- **VSS rises on U1's left row:** the decoupling capacitors sit about 2 mm outside it, their VDD pads facing U1: C29 and C2 by pins 14–17, C3 and C30 by pins 30–33 (x = 87.33 mm against the pads' 89.34). Between their pads there is no room for a 0.72 mm via within 2.0 mm of pins 16 or 31. Pin 16's via lands 2.38–2.48 mm away, and pin 31's 2.0 mm away (outward of pins 28–29).
- **The arithmetic:** 0.751 A × (54.5 + 49.8) + 0.5 A × (30.3 + 49.8) = 78.3 + 40.0 = 118.3 mV.

**The module's return (pad 19), not measured:**
- It stays unbounded, so it is charged at U1's worst VSS, as in every run here.
- Its four thermal vias form a 1.1 mm array. That pitch is not a multiple of the gate's 62.5 µm, so the gate leaves the vias out as off-grid.
- One scratch try snapped them by 25 µm or less in the emitted file. `extract` refuses that before measuring: `U30 pad 19's thermal array: a via at (129562000, 36375000) is not at a site the rule's pitch gives`.
- So pad 19 can't be measured without changing the footprint's array pitch and the module's placement grid.
- **Sensitivity, arithmetic only:** each 1 mΩ of a pad-19 bound below U1's worst VSS lowers the drop by 0.5 mV. The R2 2.5 mm board would pass with a pad-19 bound of 33 mΩ or less, and lever (iv)'s 1.54 mm board with 36 mΩ or less. The module's VDD pad 1 is bounded at 30.3 mΩ.
