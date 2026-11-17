# AET-WO-1313 — FEEDBACK

Labels: **[measured]** by a run recorded in `spike/1313/`; **[read at source]** from a file or document named beside it; **[inferred]** from arithmetic or reasoning, not measured.

## 1. Predictions (committed before any measured run)

### 1.1 Read at source before predicting

- The placer (`pcb.rs` `place`, `place_connectors`, `layout_of`, at `2b43672`): the MCU at `ORIGIN`; ICs, then passives, in reference order, each at the first free `STEP` ring around its anchor's pad; gaps `GAP_NM` 0.5 mm, `IC_GAP_NM` 1 mm, a reserve of (an anchored part's longest side + 1 mm) for an IC that owns anchored parts; the declared pairs' approach strips kept clear; connectors last, in one row at the bottom, centred under the core, `IC_GAP_NM` apart; the outline is the keep-out boxes' union grown by `MARGIN_NM` 2 mm, snapped to 50 µm. Nothing scores the box.
- V5's connector row (`spike/3400/variants/v5-4l.json`, vendored courtyards): J1 9.46, J2 9.80, J3 9.46, J4 (XT30) 14.30, J5–J7 3 × 11.96, J8 9.46 mm wide; 88.36 mm of courtyard plus 7 × 1 mm gaps = 95.36 mm, plus 2 × 2 mm margin = 99.36 mm before silkscreen widens a keep-out box. V5's width is 100.05 mm [read at source; computed].
- So **V5's width is set by the connector row, not the core** [inferred]. Any option that does not shorten the row (fewer gaps, a second edge) cannot bring the width under 100.00 mm.
- The XT30's courtyard is 15.85 mm deep (−13.60 … +2.25 mm), so it sets the row's depth [read at source].

### 1.2 Predictions [all inferred]

| # | Claim | Predicted |
|---|---|---|
| P1 | The fixture (V5 + ICM-42688-P + BMP581 + ~30 test points + reset switch + BOOT0 on J2, at 4L) under today's placer | width **100.05 mm** (the same row); height **110–120 mm**; every gate passes, `extract` included |
| P2 | Packing efficiency (sum of keep-out boxes / outline area) | V0 **35–45 %**; V5 **25–35 %**; fixture **28–38 %** |
| P3 | The connector row's share of the outline's area (row band from the core's last margin to the edge, full width) | V0 **15–20 %**; V5 and fixture **10–15 %** |
| P4 | The largest empty rectangle inside the outline | V5 and fixture: at least one of **≥ 15 × 15 mm**, beside the bucks or under the MCU's ring |
| P5 | (a) compaction alone | width unchanged (**100.05**, row-bound), height down **10–20 %**: misses 100 × 100 on width alone, by 0.05 mm |
| P6 | (b) bounded placer alone | as (a), within ±3 mm of it in height; also row-bound in width |
| P7 | (c) connectors on two edges alone | width **80–92 mm** (the core sets it); height unchanged or larger (**108–122 mm**); misses on height |
| P8 | (d) smaller gaps alone (the row's 1 mm → 0.5 mm, the core's IC gap kept unless the router proves less) | width **~96.5 mm**; height down **2–5 %**; misses on height |
| P9 | (e) passives on the bottom (arithmetic only) | frees **20–30 %** of the top-side keep-out area; not built |
| P10 | The combination that first fits 100 × 100 at 4L | **(a) + (c)** (or (b) + (c)) at about **85–95 × 85–97 mm**, every gate passing |
| P11 | Router on the compacted fixture at 4L | track length **down 5–15 %** against today's placer; vias within ±15 %; rip-ups up but every net routes |
| P12 | `emit` wall time on the fixture at 4L (release, host) | **20–90 s**; under three minutes for any option |
| P13 | K3 under the recommended method, spec-level renamings (1307's harness, seeds 0–19) of the reference, D1, D3 | with no bound declared, **16/20, 18/20, 15/20 unchanged** (no byte moves by construction); with a bound declared, **within ±3** of those, since the order is still by reference |
| P14 | Decoupling capacitors after compaction | each stays within its anchor's ring (no capacitor moves away from its pin by more than a few millimetres), parity holds |
</content>
</invoke>
