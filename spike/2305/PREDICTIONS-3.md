# AET-WO-2305: predictions for the re-run after `4240v` merged (main 943f7112)

Written before the runs, after merging main 943f7112. That merge brings spec 19, solved 12, and the generated firmware's VOS2 bring-up (PWR_CR3 written and read back, bounded ACTVOSRDY, VOS Scale 2, bounded VOSRDY) before the clock.

- **Every boot word moves by one Δ, the same on every board.** The bring-up is the same instructions on every board, with each ready flag already set on the platform. I predict Δ is between 0.1 and 5 µs. The settle's 30 ns a spin is unchanged.
- **Recovered still equals undisturbed** to the instruction, on every board.
- **Each bound and probe moves by that Δ.** No verdict moves from the previous re-judge (PREDICTIONS-2): the three waiting boards and Sentinel-03 `droop-bor3` recover; everything else passes as before.
- **Sentinel-01, main 943f7112 against the branch:** records differ only by kind 32 and kind 19's instant, as before. The bench `MANIFEST.json` differs from the one 4240v committed; nothing else does.
- **`campaign_tier2`'s pins move by Δ:** `25_700` → 25,700 + Δ, `125_700` → 125,700 + Δ, `1_357_400` → 1,357,400 + Δ.
