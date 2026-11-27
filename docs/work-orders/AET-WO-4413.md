# AET-WO-4413 — Sentinel-03's radio rail held in reset, and the 5 V return off the MCU's ground

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `wo/aet-4310-power-coverage` (PR #414), named in the kickoff. This work order is stacked: its PR targets `4310`'s branch, so `4310`'s rules prove the fix, and #414 then merges green with it.
**Branch:** `wo/aet-4413-s03-rf-reset-ground`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4310` builder, who measured both findings), with a full adversarial pass.
**Phase:** P4, slice A's follow-up (Sentinel-03), written 2026-11-27. **Cadence:** a measurement and a stop, then the build.

**Grounds.**
- **Scotty's ruling, 2026-11-27 12:34 ET, on `4310`'s two Sentinel-03 findings: change the board now.** In his words: "I agree, board change now to make it a stable and sure fire fix, plus also adding the reset no matter what." Not chosen: a lower bound first, or acceptance by name.
- **The radio rail (`4310` (2), ruled D1 (B) 2026-11-27):**
  - U30 (ESP32-C3-WROOM-02-N4, 3.0 V minimum, v1.7 p. 21 Table 6-2) and U31's B side sit on `VDD_3V3_RF`, which no monitor holds.
  - U30's `EN` has an RC only (R90 10 kΩ, C90). U31's `OE` is strapped to VCCA.
  - U13's PG falls at 90 % of FB at minimum (SNVSAW1E p. 7 §7.5): 2830.3 mV at the rail's regulated minimum of 3144.7 mV, which is under 3.0 V.
  - The rail's minimum at U30's pin is 3000.7 mV, so no supervisor fits as drawn.
- **The ground (`4310` (3c), the maximum-principle bound, certified upper bounds on the routed copper):**
  - VDD_3V3 at most 271.3 mV against 110.
    - Its own current: 85.7.
    - VDD_5V's 2000 mA at J6, J7 and J8's GND: 85.9.
    - U10's input current at its PGND: 67.4.
    - VDD_3V3_RF: 22.3.
    - U13's input: 10.0.
  - VDD_3V3_RF at most 240.8 mV against 144.
- `INV-03`, `INV-05`, `INV-06`, `INV-07`, `INV-08`; `RUL-20`.

> **What this is.**
> - **(1) Measure first, predictions committed in `spike/4413/`.**
>   - **The reset:** which held supervisor can hold U30's `EN` (and U31's `OE`, if gating it is how U31.B gets a monitor) at or above 3.0 V at its worst-case threshold. That means a threshold minimum ≥ 3000 mV, and a release that is reliable below the rail's minimum at the pin.
>   - **The rail:** the set point U13 needs to make that room, held against every `VDD_3V3_RF` consumer's maximum (U30's 3.6 V, U31, the pull-ups), with each figure from a held datasheet by page.
>   - **The ground:** what splitting the 5 V ports' return (and the bucks' input loops) off the MCU's return path takes. Can the specification express it today? Through placement, the pour, a net class, or a return group? If not, what is the smallest engine change? Give the predicted bound for both domains after the split.
>   - **The cost:** Sentinel-03's `generate` time and `bench-package-s03`'s time.
>   - **Stop** with the options and a lean for each.
> - **(2) The reset, built.**
>   - A supervisor on U30's `EN`, monitoring `VDD_3V3_RF`, mandatory.
>   - It is drawn and placed, judged by `4310`'s readiness as that rail's monitor, and cited by page. U31's B side gets its monitor the same way, or is shown to need none, by `4310`'s rule.
>   - If the specification needs a field for it, it is added in `aethelgard-ir` (`INV-01`). The version bump is named.
> - **(3) The rail, raised** to the set point (1) found, with every consumer's maximum held. `drop_limits`' VDD_3V3_RF row is re-derived from the new minimum, with its source text.
> - **(4) The ground split,** as (1) is ruled, so that both drop limits pass under `4310`'s bound on the regenerated copper. A margin computed from the wrong number is the adversarial pass's first question.
> - **(5) The package, regenerated** by the architect's write dispatch on your head, and committed unchanged. Update `fab/sentinel-03/BENCH.md` for the new parts and set point. Add a bench row for the supervisor's measured threshold.
>
> It is NOT:
> - a softer rule, or a change to `4310`'s rules;
> - the certified lower bound (the dual solve). That is a later check, never a gate;
> - any other board's design;
> - Sentinel-02.

## 1. Build

1. **Measure (1).** Stop with the table, the options and your leans.
2. **Build (2)–(5) to the ruling,** on your branch, and merge `4310`'s branch when it moves. Name every byte that moves, and why.

## 2. Acceptance

- **Sentinel-03 passes `4310`'s readiness and both drop limits** on the regenerated board. `verify --fab-out` writes its package, and `bench-package-s03` is green, inside `1919`'s 45 min.
- **Planted cases:**
  - the supervisor removed: readiness names U30 again;
  - a threshold below 3.0 V: refused by name;
  - the rail set above a consumer's maximum: refused by name;
  - the split undone: the bound fails again.
  
  Each plant has a mutant per branch.
- **No other committed board moves a byte**, except where (1)'s ruling names a field.
- **`fab/sentinel-01-bench/ORDERED.sha256` holds** for all ten files.
- **Sentinel-03's outline stays inside its declared 100 × 100 mm bound.**
- **CI is green** on this branch and on #414 after the merge into it.

## 3. Fences

- **Fence:**
  - `examples/sentinel-03.json` and its fixtures;
  - the parts data for the new parts;
  - the generator, placer and router changes that (1)'s ruling names;
  - `aethelgard-ir`, for a field (1) shows is needed;
  - `fab/sentinel-03/`;
  - `spike/4413/`;
  - `docs/work-orders/AET-WO-4413.*`.
- **Not:**
  - `4310`'s rules;
  - other boards;
  - `ci/` beyond weights;
  - the steering documents.
- One PR, targeting `wo/aet-4310-power-coverage`, left unmerged. Write `~/.cache/aet-4413/pr-title-body.md`.

## 5. Adversarial pass, full

- **A monitor that isn't one:** a supervisor whose output does not hold the consumer it is credited for, or whose threshold is read at typical rather than worst case.
- **A raised rail that breaks a consumer:** any part on `VDD_3V3_RF` above its maximum at the rail's top.
- **A split that moves the current rather than removing it:** the same return reaching the MCU's reference through another path.
- **A drop limit re-derived toward the answer.**

## 7. Flag-and-stop conditions

- **After (1),** with the table.
- **No held supervisor fits** the window, even with the rail raised.
- **The split cannot meet both limits** inside the outline bound.
- **A byte `ORDERED.sha256` binds moves,** or any other board moves.
- **A figure the work needs is held nowhere.**
