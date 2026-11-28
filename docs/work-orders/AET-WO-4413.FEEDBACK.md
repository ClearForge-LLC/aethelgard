# AET-WO-4413 FEEDBACK: Sentinel-03's radio rail held in reset (the ground split follows)

Builder: the 4310 builder, 2026-11-27. Branch `wo/aet-4413-s03-rf-reset-ground` from `86a1f280` (4310's READY head). The PR targets `wo/aet-4310-power-coverage`.

**The one-paragraph status.**
- Built as ruled at 19:18Z (D1–D4 and D6 as leaned, D5 declined) and 21:31Z (A1 (i), G1). U32, a TPS3839G33DBZR, sits on `VDD_3V3_RF` and drives U30's `EN` and U31's `OE`, and U13 raises the rail to 3,303–3,589 mV.
- **Sentinel-03 is refused twice, as ruled:**
  - **Readiness names U30 and U31.B:** "monitored by U32, but U32 does not sense at U30's pin; the drop between is not bounded" (and at U31's). The credit I first built rested on a placement I had claimed and not built (A1, my second stop). It is reverted, and the binding goes to the engine work order.
  - **`extract` refuses both drop limits,** each named with its figure: VDD_3V3 at most 286.7 mV against 110, and VDD_3V3_RF at most 220.8 mV against 145. The ground split that would pass them is the same engine work order (ruled D6).
- Its committed package is withdrawn. It was last valid at `ec440f71`.

## 1. Part (1): the measurement and its stop

- Predictions were committed (`db0bc89d`) before any probe ran, and are scored in `spike/4413/RESULTS.md`. The stop with the table and D1–D6 is `spike/4413/STOP.md` (PR #417 at `56b84349`).
- TPS3839 (SBVS193D) and TPS3840 (SNVSB03E) were newly held under `main`'s held-copy rule, each at `~/.cache/aet-held/` with its `SOURCE.txt`.

## 2. What was built (part (2) on)

- **The field, `power.rail_monitors[]`** (specification 31 → 32, ruled D2): `{reference, part, rail, bypass}`.
  - A part names its monitor: a radio module's `enable.monitor`, in place of its RC, or a translator's `OE` control's `monitor`, in place of its strap.
  - `supervisor::check` judges each monitor: its row push-pull, its bypass a capacitor, its rail not the MCU's, every part naming it on its rail, at least one, its VDD range covering the rail, its release strictly under the rail's minimum at its pin, and its delay at least a driven module's settling time.
  - Since 21:31Z A1 (i), the floor rule credits a monitor only for its own pins: every part it drives is named.
  - The README's version-32 sentence and its row are written.
- **The parts** (power-parts 15, ruled D1):
  - TPS3839G33DBZR, cited to SBVS193D, held, with its `SOURCE.txt`: VIT− 3,003 / 3,126 mV, Vhys 31 mV typical, VDD 0.9–6.5 V, a minimum delay of 120 ms.
  - Every supervisor gains its `output` (open-drain with its pull-up, or push-pull) and `delay_min_microseconds`. A push-pull part on `NRST` is refused by name.
  - The `TPS3839` symbol is vendored, registered in its four places.
- **Sentinel-03** (ruled D3):
  - U32 and C95 are drawn; R90 and C90 are gone.
  - U13 is at 100k/41.2k, with the domain at 3,430 mV ± 4.9 %. ± 5 % is refused, `DomainOutsideConsumer`, by U31's 3.6 V rows.
  - VDD_3V3_RF's limit is 145 mV, with its source text.
  - U13's loss is re-derived at 3.589 V: 512 mW. The branch now carries 1,918 mA.
  - The loads and acknowledgements follow.
  - `CrossingOeUnheld` admits a monitor on VCCB.
- **G1, named:** `radio_supply_peak_unbounded:U30`. The solved top is 10.9 mV under U30's 3.6 V, from a static solve. Its acknowledgement cites Table 6-1 for the absolute maximum, and bench row B-11 reads the peak.
- **The package and CI** (ruled D6):
  - `fab/sentinel-03/package/` is removed. BENCH.md says so at its top, adds B-10 (U32's thresholds) and B-11, and brings B-1 and B-8 to the new figures.
  - `bench-package-s03` asserts both refusals: a plain `verify` failing by exactly the two drop limits, and `--fab-out` refused by readiness, U30 and U31.B named, nothing written.
  - `board_tier2`'s test is renamed `the_sentinel_03_fixture_is_refused_by_its_two_drop_limits`, with its weight row.
- **The version bump's derived bytes:**
  - `boards/sentinel-01` and `-bench`;
  - the hwdef fixtures (each topology, `.hwdef.dat` and `.defaults.parm`) and the svd-hal UI boards;
  - 4300j's two spike specifications, which tests read;
  - the solve digests;
  - the CLI README's example line.

  Each moved by its specification sha alone (`spike/4413/out/inv08.txt`).
- **Sentinel-01-bench's package:** from the architect's write dispatch, run 46079993133 on `361fd0dd`, committed unchanged.
  - FAB-NOTES.md and MANIFEST.json move by the version-32 specification's sha, its topology's sha and the verdict.
  - Every other file is byte-identical, and the ten files `ORDERED.sha256` binds pass `sha256sum -c` against the artifact, 10 OK.

## 3. Deviations, named

- **VDD_3V3_RF's limit is 145 mV, not the 146 the ruling wrote.** `SupervisorAboveRail` holds the release strictly under the rail's minimum at the monitor's pin. 3,303 mV (the solved minimum, whole millivolts) less 146 is 3,157, which equals U32's release (3,126 + 31). 145 is the largest limit that passes, as 272 rather than 273 was in the stop's D5 probe.
- **U31's `OE` follows U32's reset on VCCB, and is no longer strapped to VCCA.** SCES935A §9.1 (Overview) allows either reference ("OE can be referenced to either VCCA or VCCB"). Pulled low, it holds the outputs high-impedance through power-up and power-down, and `OE` is rated to 6.5 V (p. 5, §7.1). `CrossingOeUnheld` now admits a monitor on the module's rail, and still refuses one on any other rail.
- **U30's enable RC (R90, C90) is removed, not kept beside the monitor.** The monitor's delay of at least 120 ms (SBVS193D p. 8) is judged against the module's tSTBL in the RC's place. A 1 µF capacitor on a push-pull output would charge through it past RESET's ±10 mA absolute maximum (p. 6, §7.1).
- **VDD_3V3's own term rose about 21 mV** (85.7 → about 107 mV), because the regenerated placement moved with U32 added and R90 and C90 gone. Its bound is 286.7 mV, against 271.3 before. It was refused before and is refused now; the ground split is what passes it.
- **A1, my own error, found by the build's adversarial pass.** My part (1) stop told the architect that U32 "is placed at U30 and senses the rail at U30's pin". Nothing I built did that: the placer put U32 83 mm from U30, and the floor rule credited it on 3 mV with no term for the drop between. I stopped, and the credit is reverted (21:31Z A1 (i)).
- **The plant "a threshold below 3.0 V: refused by name"** holds trivially since the revert: every part a monitor drives is named, whatever its threshold. The floor plant I built for the credit is retired with it, along with its four mutants (R-M1 to R-M4).
- **The G1 condition's text is generic:** the margin from the solved top, from a static solve. The radio parts row has no field saying a maximum is also absolute, so that citation (Table 6-1) is in Sentinel-03's acknowledgement.

## 4. Gates and proof

- **Mutants:** 18 of 18 killed (`spike/4413/mutants/p2.out`), one per guard this work order adds; four credit mutants retired with the revert. The OE-only check in `check_rail_monitors` is unreachable today and is labelled `belt-and-suspenders:` at the site.
- **`INV-08`** (`spike/4413/out/inv08.txt`): every other board is byte-identical in `emit`, and in `solve` but for its specification sha. Sentinel-03's three files move, and the 40 specifications move by their version line. `ORDERED.sha256`: see §3.
- **Tier 2 in the pinned image:**
  - `board_tier2`'s, `twin_tier2`'s and `campaign_tier2`'s Sentinel-03 tests pass;
  - Sentinel-03's `verify` gives exactly the asserted refusal, 24 min on Furnace.
- **Plan and route:** `ci/shard.sh plan` in the image, guard ok (3,613 names); the route guard passes.
- **Tier 1 on `361fd0dd`:** 3,368 passed, 6 failed.
  - `bench_board_tier1`'s committed-package check failed by design until Sentinel-01-bench's write (§2); it is settled now.
  - The other five pinned what this work order moves, and are fixed after that head and re-run green:
    - `radio_tier1`: U30's EN on `/U32_RESET`;
    - `supply_tier1`, two tests: the load list without R90, and the via-pad plant on 4411's rail with U31 removed;
    - `via_ring_tier1`: the ladder's EXT_SDA margin re-measured, 76,862 fF;
    - `order_2203`: three texts move by their specification sha line alone, shown by `spike/4413/order/order_back.rs` (each hashes to the digest before with the version-31 sha put back), and the boot image did not move.
- **The dispatch run's own reds, each by name:**
  - `gates` and tools shard 5: the committed-package check above.
  - Shards 0, 2, 6 and 14: the four tests just listed, already fixed.
  - `bench-package-s03`: its first check wrote its verdict to `target/`, which the runner cannot write. The plain `verify` never ran, so neither did `prove`, and the job's ceiling check (`--require-peak`) found no Kani peak. Both checks now write to `$RUNNER_TEMP`. The plain `verify` runs `prove` again, so the ceiling stays and judges a real peak; the `--fab-out` run proves nothing.
  - `ardupilot-image`, `generated (sentinel-03)`: I regenerated Sentinel-03's hwdef fixtures before adding the G1 acknowledgement, which moved its spec sha again, and `generate hwdef` would not overwrite an existing output, so the stale file stayed. Both are fixed by regenerating on the final spec.
- **Both CI checks proven on real runs:**
  - the plain `verify` in the tools image on `361fd0dd` (1,485 s on Furnace), refused by exactly the two pinned findings;
  - `--fab-out` on the host, refused by readiness naming U30 and U31.B.

## 5. Adversarial and reduction

- **The full adversarial pass** (`spike/4413/ADVERSARIAL.md`):
  - A1 led to the second stop and the revert, and G1 is named.
  - The rest is fixed: the version bump's missing bytes, the stale texts, B-8's instruction to short C90, and the CI pin re-measured.
  - Named, not fixed:
    - G2, VCCA up before U32 releases OE;
    - G3, the hold judged in readiness, not `validate`;
    - G4, U32's transient behaviour, which is not modelled;
    - G5, `rail_minimum`'s no-limit fallback, which has no test;
    - G7, `bench-package-s03`'s CI time: one full Sentinel-03 `verify` (1,485 s on Furnace) and a `--fab-out` that stops at readiness, under the job's 90-minute timeout; its CI time on this head is not yet measured;
    - G8, an OE typo is reported as `CrossingOeUnheld` (it fails safe).
- **Reduction pass (diff):** −8 lines net (11 lines became 3). One dead path removed: `judged_against`, whose only caller was the retired floor plant. One protected kept: the labelled OE-only guard. One finding below.
  - **The finding:** `supervisor::held` and `check_rail_monitors` walk the same parts that name a monitor (radio enables, translator OEs). One walk could serve both, but the engine work order rewrites the credit around a monitor bound at its part, so it is left to that work order.

## 6. Rows

- The ground split and the RF supply path lever: the next engine work order (ruled D6).
- `power::rail_consumers` lists no supervisor or translator, so a campaign droop of `VDD_3V3_RF` does not reach U32. The twin models neither U30 nor U31, so no run resets U30 from it.
- U32's output wire to U30's `EN` is routed at the default class, with no loop rule, and its distance is not bounded.
