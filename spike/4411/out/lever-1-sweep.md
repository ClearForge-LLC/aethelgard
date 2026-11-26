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
| **R2** | **2.50** | ok | 5 | 121, 54.5 (pin 30 now 42.2) | **16, 49.8**; 31, 46.5 | **118.3, fail** | 1503 | 85.65 × 96.6 | 24 `drill_out_of_range` (U10/U11 thermal vias), the same as the committed USART6 and top-left boards; `--spec` pass |
| R2 | 3.00 / 4.00 | ok (rungs 6 / 2) | – | not extracted | – | – | – | – | – |

**First, a correction to my 05:06Z note.** The ruling's premise came from it, and it was wrong for pin 30. On the 1.54 mm board, pin 30 did not rise because an antipad cut In2:
- Pin 31's GND via took pin 30's own inward escape site at (90.75, 105.75).
- Pin 30 then reached a VDD via shared with pins 32 and 33, at (90.5, 107.125), by about 5 mm of 0.2 mm F.Cu track around the pad toes (read from that board's tracks). That gave 59.7 mΩ.

**What R2 does, read from the emitted 2.5 mm board (`geom.py`):**
- **The rule's own choices:** R2 is my reading of the ruling. It leaves the top and right rows where lever (iv) put them, inward. 2.5 mm is the first swept distance that routes, well past the declared 1.54 mm.
- **VDD:** pin 30 still has no via of its own. It reaches the shared VDD via at (90.625, 107.25), 1.98 mm away, and its bound falls from 59.7 to 42.2 mΩ, so the long loop round the toes is gone.
- **The VDD floor:** the worst is pin 121 at 54.5. That is 54.5 to 54.7 on every board measured: the committed USART6 board, lever (iv) at 1.54 mm, and R2 at 2.5 mm. Pin 121 has no VDD via within 2.6 mm on this board, and its GND neighbour 120's via sits 1.38 mm in front of it.
- **VSS, U1's left row:** the decoupling capacitors sit about 2 mm outside it, with their VDD pads facing U1: C29 and C2 by pins 14 to 17, C3 and C30 by pins 30 to 33 (x = 87.33 mm against the pads' 89.34). Under R2, the left-row GND vias must go outward. As emitted, pin 16's via is 2.48 mm away at (87.875, 96.75), and pin 31's is 2.01 mm away at (87.875, 104.875), outward of pins 28 and 29. At 1.54, 1.76 and 2.0 mm the refusal named pad 16, the first pad refused.
- **The arithmetic:** 0.751 A × (54.5 + 49.8) + 0.5 A × (30.3 + 49.8) = 78.3 + 40.0 = 118.3 mV.

**The pass window:** with VDD at its 54.5 floor, the drop is at most 110 mV only if U1's worst VSS is at most (110 − 0.751 × 54.5 − 0.5 × 30.3) / 1.251 = 43.1 mΩ. Lever (iv) reached 42.4 for pin 31 only by taking pin 30's escape site. So even a directional rule that kept every VSS pin at 42.4 lands about 109 mV, under 1 mV of margin.

**The open term, the module's return (pad 19), not measured:**
- It stays unbounded, so it is charged at U1's worst VSS, as in every run here.
- Its 2 × 2 thermal vias stand at the radio parts row's 1100 µm pitch (`radio-parts.json`, from the datasheet's land pattern). That pitch is not a multiple of the gate's 62.5 µm, and the vias are off the gate's grid, so the gate leaves them out.
- U10's and U11's thermal arrays are off-grid the same way (`extract`'s notes, 28 GND vias).
- **One scratch try:** snapping the four vias by 25 µm or less in the emitted file. `extract` refuses it before measuring: `U30 pad 19's thermal array: a via at (129562000, 36375000) is not at a site the rule's pitch gives`.
- **What measuring it needs:** the array's pitch on a 62.5 µm multiple (1125 µm moves each via 12.5 µm off the land gaps' crossings), and its first via on the gate's grid, which depends on the module's placed origin. Cost not estimated; not built.
- **Sensitivity, arithmetic only:** each 1 mΩ of a pad-19 bound below U1's worst VSS lowers the drop by 0.5 mV.
  - The R2 2.5 mm board would pass at a pad-19 bound of 33 mΩ or less; lever (iv)'s 1.54 mm board at 36 mΩ or less.
  - The module's VDD pad 1 is bounded at 30.3 mΩ.

**Score of the lever (1) prediction (`PREDICTIONS.md`):**
- **Worst VDD:** about 54.6 at pin 121. **Right** (54.5, pin 121).
- **Worst VSS:** about 43 at pin 31, outward. **Wrong:** pin 16 decided at 49.8, and pin 31 was 46.5.
- **The routing risk:** **right.** It was named as outward vias where the neighbouring pins' fanout goes. What it was is narrower: the decoupling capacitors' pads, not the fanout. Pad 16 was refused below 2.5 mm.
- **The drop:** about 110 mV, too close to call. **Wrong by 8 mV:** 118.3.
- **U11 at least 700 mm² and the outline unchanged:** **right** (1503; 85.65 × 96.6).
