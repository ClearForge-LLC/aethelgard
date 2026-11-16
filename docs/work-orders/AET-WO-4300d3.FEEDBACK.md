# AET-WO-4300d3 — FEEDBACK (build, with one measured stop: §1.0, a held source for the buck's junction)

**Branch:** `wo/aet-4300d3-judgements` from `origin/main` `dbf049e` · **Work order:** `docs/work-orders/AET-WO-4300d3.md` · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, with subagents · **Cadence:** build, with a full adversarial pass.

Each answer is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any new check was run. No prediction is edited after that commit. The only runs before it were:
- ghostscript text extraction and page renders of held datasheets, in the pinned tools image through `~/bin/capped-run`;
- the byte-identity baseline: `main`'s binary on every example and fixture (`~/.cache/aet-4300d3/base/`, 91 files across 19 specifications).

## 0. Readings taken before the predictions

### §1.0, the measured stop: the buck's junction [read]
- **SNVSB89 is not an LMR33630 document.** `ti.com/lit/pdf/snvsb89` serves SNVSB89A: the LM25180-Q1 PSR flyback datasheet (Nov 2018, rev. July 2019; sha256 `4cad6041…`). The pp. 23 and 32 material the work order means is in SNVSAN3F, the LMR33630 datasheet already held (sha256 `3b0920a4…`).
  - p. 23, 9.2.2.6: "it is desirable to use an electrolytic capacitor on the input in parallel with the ceramics… The moderate ESR of this capacitor can help damp any ringing on the input supply caused by the long power leads."
  - p. 32, Power Supply Recommendations: "an aluminum or tantalum input capacitor in parallel with the ceramics… A value in the range of 20 µF to 100 µF is usually sufficient to provide input damping." The same page says a snap-back (thyristor-type) TVS "is not recommended".
  - p. 6, 7.1: VIN to PGND absolute maximum is 38 V.
- **TI states no θ or Ψ for design.**
  - p. 7, 7.4: RθJA 42.9 °C/W (DDA) is "only valid for comparison… can not be used for design purposes" (JESD 51-7, simulated 4-layer JEDEC board). ψJB is 13.8 °C/W and ψJT 4.3 °C/W, as characterization parameters.
  - p. 24, 9.2.2.11, Equation 11: T_J = T_A + P_loss · RθJA, where "the effective RθJA of the device and PCB combination" is "more difficult to estimate".
  - p. 25, **Figure 9-3, "Typical RθJA versus Copper Area for a Four-Layer Board and the HSOIC (DDA) Package"**: "the copper area given in the graph is for each layer; the top and bottom layers are 2 oz. copper each, while the inner layers are 1 oz… for illustration purposes only." Read from the 300 dpi render, in °C/W at cm² per layer: 41.7 at 6.5 (where the curve starts), about 35.5 at 10, 29.5 at 15, 27.7 at 20, 26.7 at 30, 26.1 at 40, 25.3 at 50, and 24.5 at 58 (where it ends).
  - Figure 9-4 is the RNX (VQFN) package, a different curve.
- **The EVM guide (SNVU573A, held) states no θ, Ψ or thermal figure** [measured: no match for thermal, °C/W, θ or junction in its text].
- **Efficiency** (p. 27, Figures 9-7 and 9-8; DDA, 400 kHz, TA 25 °C, typical). Read from the render:
  - 5 V out at 2 A: 92.5 % at 24 V in, 90.5 % at 36 V in.
  - 3.3 V out at 0.68 A: about 85 % at 24 V in, 82 % at 36 V in.
- **So a held source exists (Figure 9-3), typical and conditioned. §1.0 does not stop.** The fixture's board is outside that figure's condition (D1).

### Hot-plug parts [read; fetched by a subagent, spot-checked at source]
- **Bulk capacitor: Nichicon UCZ** (catalog "UCZ Chip Type, High Reliability", Jun. 2024; sha256 `7ecc9608…`, `~/.cache/aet-ds/`).
  - Row `UCZ1H470M□□1GS`: 47 µF ±20 %, 50 V, 8×10 mm, -40 to +125 °C.
  - ESR maximum is 0.25 Ω at 20 °C and 100 kHz, and 3.50 Ω at -40 °C. The ESR at the board's -20 °C floor is not printed.
  - Caveats: the full part string `UCZ1H470MCL1GS` is inferred from the catalog's numbering example. No lifecycle status could be read (Nichicon's product page is a script-only app). The catalog carries no document number.
- **TVS: Littelfuse SMBJ** (06/03/20 revision; Littelfuse's own site refused the download, so the copy is from a TI E2E mirror).
  - **No SMBJ row has a stand-off above 26.1 V and a clamp at or below 38 V.**
    - SMBJ26A: VRWM 26.0 V, VC 42.1 V.
    - SMBJ28A: VRWM 28.0 V, VC 45.4 V.
  - The clamp path is therefore proved with a test-only table, not a pinned row.

### DC bias [read]
- **No held capacitor document gives a DC-bias figure** [measured]. Murata's reference sheets give only "Temperature Characteristics… No bias". Murata's chart for GCM32EC71H106KA03 is being fetched by the operator. So **§7's second stop condition is true today**: the declared derating is the only source for every capacitor.
- **The derating sources held:**
  - SBVS320D (TLV755P) p. 15: "Generally, derate ceramic capacitors by 50%". Its p. 3 note 1 (Table 4-1; corrected at close from "p. 4") adds: "Throughout this document the nominal derating on these capacitors is 50%. Make sure the effective capacitance at the pin is greater than 0.47µF". So **TLV755's 0.47 µF is an effective minimum, read at source.**
  - SNVSAN3F p. 22, in the 5 V design example: "a 10% bias de-rating" for 16 V 1210 ceramics at 5 V. **The two sources disagree** (10 % against 50 %). That disagreement is a finding, not something to average.
- **SNVSAN3F's minima.**
  - p. 19 Note: "the effective value of capacitance is defined as the actual capacitance under D.C. bias and temperature… ensure that the minimum value of effective capacitance is provided."
  - p. 23: "A minimum of 10 µF of ceramic capacitance is required on the input". This sentence does not say "effective".
  - p. 20, Table 9-2 lists "COUT (RATED CAPACITANCE)" 4 × 22 µF and "CIN + CHF 10 µF + 220 nF". The EVM uses one 10 µF 50 V part at up to 36 V.
  - **Whether the 10 µF minimum is effective is inferred, not read (D2).**

### The brown-out threshold [read]
- DS12117 p. 212, Table 115 gives VBOR rising maximum as 1.71, 2.15, 2.47 and 2.78 V for levels 0 to 3 (falling maximum 1.68, 2.06, 2.37, 2.68 V). Rising BOR0 carries note 1.

### Control pins [read]
- SNVSAN3F p. 5, pin table:
  - EN: "Do not float."
  - PG: "Open drain power-good flag output… Can be left open when not used."
- The emitter ties a buck's EN and an LDO's EN to their input, leaves PG unconnected, and ties OE to VCCA (`eda/src/schematic_power.rs:402-403`, `:481`, `:531`).

### The 3.3 V divider [measured, `main`'s solve]
- R20/R21 = 105 k / 46.4 k gives a guaranteed output of 3144.7 to 3417.2 mV. The domain is 3135 to 3465 mV.
- That leaves 9.7 mV (0.29 %) of margin at the low edge and 47.8 mV (1.45 %) at the high edge.

## 1. Predictions (committed before any new check runs)

| id | prediction | label |
|---|---|---|
| P1 | **Junction (U10, 5 V, 2000 mA).** P_out ≤ 5.237 V × 2 A = 10.47 W. η = 90 % (the 36 V curve at 2 A, which brackets 26.1 V from below) gives P_loss ≈ 1.16 W. At Figure 9-3's 20 cm² point (28.0 °C/W rounded up) the screen gives T_J ≈ 85 + 32.8 = 117.8 °C, 7 K under 125 °C. At 10 cm² (36 °C/W) it is about 127 °C, which is refused. | inferred |
| P2 | **Junction (U11, 3.3 V, 679 mA).** P_out ≤ 2.32 W. η = 81 % gives P_loss ≈ 0.55 W, so T_J ≈ 100 °C at 20 cm². | inferred |
| P3 | **DC bias.** At the declared 50 % derating, the TLV755's cin and cout each hold 0.50 µF effective against 0.47 µF, a margin of 1.06 times. If the buck's 10 µF were effective (D2), U10's and U11's single 10 µF 50 V cin would hold 5 µF and be refused. | inferred |
| P4 | **BOR rising.** VDD_3V3's minimum of 3135 mV clears level 3's 2780 mV rising maximum by 355 mV, so the fixture passes. A domain floor between 2680 and 2780 mV is refused `SupplyFloorBelowBorRising`. | inferred |
| P5 | **Hot-plug.** One UCZ 47 µF 50 V part on the board side of J1 passes for U10 and U11. Each of these is refused `BatteryInputUndamped`: a rating at or below 26.1 V, 19 µF, a pack-side part, a test-table TVS clamping at 45.4 V, and no part at all. | inferred |
| P6 | **Byte identity.** Every board without a power tree is byte-identical apart from specification-hash lines. Boards with a power tree are byte-identical too, because their declarations are not drawn. The verdicts of power-tree specifications change by their new conditions only. | inferred |
| P7 | **Tier 2** gains no test, so `TIER2_MIN` stays at 129. The fixture's existing tier-2 tests keep passing with the declarations added. | inferred |
| P8 | **Size:** 1,200–2,000 lines, almost all in `aethelgard-ir` (power.rs, power_parts.rs, the table, tests). | inferred |

## 2. Decisions for the architect (open)

- **D1 · The fixture's board is outside Figure 9-3's condition.** `sentinel-02-power.json` declares 2 copper layers. The only held θ is for a four-layer 2/1/1/2 oz board.
  - **Default taken** (it keeps every board byte-identical): use the figure and name in the verdict that the board is outside its condition, with the layer count.
  - Options:
    - (a) the default;
    - (b) refuse below four layers, and make the fixture four-layer, which changes its tier-2 board;
    - (c) a declared θ with its source, as §1.0's stop offered.
- **D2 · Is SNVSAN3F's 10 µF input minimum effective or rated?** Reading it as effective is inferred, not read (see §0).
  - **Default taken:** no effective minimum on the buck's cin and cout rows; the rated checks stay. The verdict names the shortfall each would have under the declared derating.
  - Options:
    - (a) the default;
    - (b) effective. Then the fixture fails at its single 10 µF (P3), and each buck needs a second or third 10 µF, which changes the power boards;
    - (c) effective, with the screen waiting on Murata's chart.
- **D3 · PG left open.** §1.5 offers "wired" or "tied by a strap"; the emitter leaves PG open.
  - **Default taken:** `open` is a strap a row may allow, by citation (SNVSAN3F p. 5: "Can be left open when not used"). EN may not be open.
- **D4 · The 3.3 V divider is kept.** 1 % inside at both edges needs 66 mV of slack, and the band leaves 57.5 mV.
  - A balanced pair would put each edge about 0.87 % inside.
  - Any new pair changes the power boards' BOM and value strings, which §2's byte identity forbids.
  - **Open:** re-pick the pair if the architect prefers margin to byte identity.
- **D5 · Declared hot-plug parts and control straps are not drawn.** `eda` is outside this fence. So the emitted board lacks the bulk capacitor this tree now requires, and the straps are drawn by the emitter's own rule.
  - Named as a condition in the verdict.
  - Proposed backlog row: the emitter draws `power.input.hot_plug` and reads `controls`. Owner: 4300g or the next eda work order.

## 3. The build, measured against the predictions

| id | outcome | label |
|---|---|---|
| P1 | **Held.** U10 screens at 117760 m°C on an 85 °C board, 7240 mK under 125 °C: 1170 mW × 28.0 °C/W at the 20 cm² point. At 10 cm² it reaches 127.1 °C and is refused. The limit is exact: 1428 mW passes and 1429 mW is refused. | measured |
| P2 | **Held.** U11 screens at 100.4 °C (550 mW × 28.0 °C/W). | measured |
| P3 | **Held.** The LDO's cin and cout hold 0.500 µF effective against 0.47 µF, a margin of 1.06 times. At 470 per mille kept, each is refused (the minimum is strict, "greater than"); at 471 per mille they pass. With D2's other reading pinned through a table copy, U10's single 10 µF cin is refused at 5 µF. | measured |
| P4 | **Held.** 3135 mV clears VBOR3's 2780 mV rising maximum. With the rising maximum moved to 3135 mV in a table copy, the fixture is refused `SupplyFloorBelowBorRising`; at 3134 mV it passes. | measured |
| P5 | **Held, with one addition:** three bulk parts (141 µF) are refused for exceeding TI's 100 µF. | measured |
| P6 | **Held.** `emit`, `solve` and `check speeds` were run with `main`'s binary and with this branch's, over all 19 specifications (91 files). Every board, schematic, BOM and verdict is byte-identical once specification hashes and the output directory's name are masked. The one other change is the power fixture's `solve`: the same topology, with its conditions replaced (15 removed, 25 added). | measured |
| P7 | TIER2_MIN stays 129: no tier-2 test was added. See §6 for CI. | measured |
| P8 | **About 1,860 lines in `aethelgard-ir`,** most of them tests and table data. Beyond `ir`, only lines the version bump moved. | measured |

### What the version bump moved [measured]
Specification 9 changed every specification's sha256. With the old hash put back, each pinned digest re-taken below equals its old value exactly, and no generator changed.
- `boards/sentinel-01`: regenerated by `aethelgard generate`. Eight files changed, one hash line each.
- `solve_cli`'s two digests.
- `order_2203`'s three digests. The boot image did not move.
- The twin's and pincheck's reference topologies, `verify_2k2_tier2`'s prefix, the svd-hal UI fixtures (re-blessed, hash-only), and `cli/README.md`'s example line.
- **Root `README.md` was not touched** (protected). Its quickstart pins the specification at `1080652` with its own image digest, so it stays self-consistent. Re-pinning after merge is the architect's (`roadmap.md` §6).

### Overlaps
- `4300g` also bumps `power-parts.json` to schema 2. Whoever merges second takes 3. Both sides were told on the bus.
- `4300g` reports that, with PDR_ON tied to VDD, DS12117 p. 22 note 6 needs VDD above 1.71 V. No check here reads the MCU's VDD minimum except the existing floor, max(VDD min, VBOR falling); at `bor_level` 3, VBOR governs. This is `4300g`'s to rule.

## 4. Adversarial pass (a fresh Opus subagent, on `e42228d`)

The reviewer worked from a copy, given the code and the work order but not this file's conclusions. Probes and repro specifications are in `~/.cache/aet-4300d3/adv/`. Nothing panicked, and every new key is required. Each finding was checked at source before it was adopted.

| id | sev | finding | disposition |
|---|---|---|---|
| F1–F4 | high | A control pin wired to `GND`, a rail or the battery passed: a strap refused by name got through as a wire. PG on the 26.1 V pack passes SNVSAN3F p. 6's 22 V PG maximum. No condition named the declared wire. | **Fixed.** A wire to a domain, `GND`, `VBAT` or `battery` (case-folded) is refused `ControlPinUnwired`: it is a strap, which the row judges. Every other wire is named in the verdict, with this condition: "neither the net's driver nor its level is judged". A one-ended (floating) net cannot be detected without a netlist. That is D5's backlog row. |
| F5 | high | Where a maker's DC-bias figure stops below the working voltage, the declared derating could exceed it. | **Fixed.** The derating is capped at the maker's last point. |
| F6 | med | Overlapping derating bands gave a verdict that hung on their order. | **Fixed.** The shape refuses two bands that share a dielectric at one ratio, and a narrower band keeping less than a wider one. |
| F7 | med | A row could omit a control pin and so switch the check off. | **Fixed.** Each row names exactly its kind's control pins: EN and PG on a buck, EN on an LDO, OE on a translator. |
| F8 | med | Three 47 µF parts (141 µF) are refused. TI says "20 µF to 100 µF is usually sufficient", which is not a maximum. | **Kept**, as the work order's letter ("in TI's 20–100 µF range") reads. **Question D6.** |
| F9 | med | A declared loss of 1 mW passes: the loss is not tied to the load. | **By design**, a declared bound. The verdict names the declaration and the load it was taken at. Backlog: a floor from the switches' conduction loss at the load. |
| F10 | low | The TLV755 EN strap cited p. 3, which does not license tying EN to IN. | **Fixed.** It now cites p. 11, 6.3.2: "if shutdown capability is not required, connect EN to IN" [read]. |
| F11 | low | The θ curve's package check is a suffix match, so `A` passes. | **Kept.** TI's part numbers end with the package code. Backlog: compare against the row's footprint. |
| F12 | low | A VIN absolute maximum below the recommended maximum was accepted. | **Fixed** for the buck and the LDO. A rising BOR equal to its falling one stays accepted, as consistent. |
| F13 | low | A bulk rated 26.101 V passes on a 26.1 V pack. | The work order's letter ("above the battery's maximum"). **Question D6.** |
| F14 | low | Rated capacitance is used before its tolerance. A battery input feeding only an LDO has no damping range. | **Named in the verdict** ("from rated capacitance before its tolerance"). The LDO-only case is named as its row stating no range. |
| F15 | process | §7's second condition is true, and the build went on. | **Accepted.** The kickoff said to build on the declared derating until Murata's chart lands. This build stops on it (§6). |

## 5. Mutation run (diff-sized)

A Sonnet subagent ran one mutant per new guard, each against `aethelgard-ir --test power`: 49 mutants plus 6 "guard dropped" mutants.
- **The first run left 19 survivors.** Twelve were boundary mutants with no equality case. Seven were real gaps:
  - the TVS stand-off guard was shadowed by its clamp in every case;
  - the junction's round-up;
  - the maker-point choice;
  - the smallest-band choice;
  - an effective minimum with no working voltage;
  - two parser bounds.
- `the_new_guards_hold_at_their_edges` pins each one.
- **Re-run in full on `e8f5f34`**, after the adversarial fixes and the reduction: the 55 above plus one mutant per guard added for F1, F4, F5, F6 (two), F7 and F12 (buck and LDO separately). **69 killed, none survived** [measured, `~/.cache/aet-4300d3/evidence/mut/e8f5f34-log.txt`]. One mutant had no target: 11a's pin-role check was replaced by F7's exact-set check, whose own mutant is killed.

## 6. Gates, reduction, close

- **Gates on `e8f5f34`** (the last code change; later commits touch FEEDBACK, one test comment and `ci/` only) [measured, Furnace]:
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  - `cargo test --workspace --no-fail-fast`: 106 binaries, 0 failures.
  - `bash ci/test-shard.sh`: 87 passes.
- **CI** [measured]:
  - `e42228d`: all green. Tools shards 12.6 to 18.4 minutes, gates 11.3. `tier2_tests_passed=129 floor=129`.
  - `e8f5f34`: all green, tier 2 at 129 = floor. **Shard 0 ran 21.5 minutes**, past the refit rule's 20; the others ran 13.9 to 18.0. Its tier-2 content is the same as `e42228d`'s 18.4 minutes: this branch adds no test of 0.5 s or more. The planner's weights predate `4300d2`, `3303` and `4220`, so their heavy tests weigh the 0.05 s default.
  - **Refit:** step 1 ran here as a throwaway (`9ac2774`, 4300b's `f28eb59` harness unchanged, run 43769041867), reverted exactly by `1e4acb0`. The architect then ruled (bus, 22:55): `4300b` owns the refit of `main`'s tests, and this PR takes 4300b's weights after it merges, then refits only if a shard still passes 20 minutes. So no weights change is committed here. The measurement is kept for comparison: `~/.cache/aet-4300d3/refit/weights.rows`, 339 tests of 0.5 s or more, planned at 894 s per shard. Its colocated maxima were sent to `4300b`.
  - TIER2_MIN stays 129, exact on both runs.
- **Reduction pass (diff): −5 net · 3 duplicates removed · 1 dead path removed · 0 protected kept · 2 findings below.**
  - Removed: three copies of the case-naming assertion, now one helper, and the `ground` strap, which no row allows.
  - Finding: `power.rs`'s `dielectric()` names repeat the serde names of `Dielectric`, the house pattern `Role::name` follows. Kept.
  - Finding: `effective()` builds its explanation strings even when nothing is judged. Left, as cheap.
- **`tRSTTEMPO`** (the review response sends this row here) [read]: DS12117 p. 212, Table 115 gives "Reset temporization after BOR0 released" with Conditions "-", Min "-", **Typ 377**, Max "-" µs, and note 1 ("guaranteed by design"). It is a typical figure: nothing in the table bounds it. Its user is `aethelgard-chaos/src/power.rs`, outside this fence. Backlog row for the owner of the twin's power model: name it as typical wherever it is used.
- **Questions for the architect, beyond §2:**
  - **D6:** TI's 20–100 µF as a hard range (F8), and the bulk's rating against the DC maximum only, not the ring (F13).
  - **D7:** `4300g`'s 1.71 V PDR_ON floor (§3 Overlaps).

**Stop.** Three reasons, each named:
1. §7's second flag-and-stop condition holds at close: no held capacitor row has a published DC-bias figure, so the declared derating is the only source for every capacitor. Murata's chart for GCM32EC71H106KA03 has not landed in `~/.cache/aet-ds/` or the inbox. When the chart lands, a figure is one row's `dc_bias` points.
2. D1–D3 are unruled. A ruling on any of them changes one data field or one branch.
3. Under the 22:55 ruling, the shard time waits on `4300b`'s refit. When this PR's head is under 20 minutes on every shard, that is variance, not a fix.

Everything else in §1 is built, judged, adversarially reviewed, mutation-tested and reduced.

## 7. Rulings applied (PR #158, comment 6902904831)

- **§7, declared derating at the conservative 50 %.**
  - The shape now refuses a declared derating that keeps more than 500 per mille of a class-2 ceramic (X5R, X7R, X7S), citing SBVS320D p. 15 and the ruling. A C0G band may keep up to 1000.
  - The fixture declares 500. Murata's chart replaces the derating for its row when the chart lands: one row's `dc_bias` points, used before the derating.
- **D2 (c), effective, named non-blocking until a figure is held.**
  - LMR33630's `cin` row now holds a 10 µF effective minimum, cited to SNVSAN3F p. 23 and p. 19, with `on_declared: report`.
  - When only the declared derating decides it, a shortfall is a named condition giving the row and the shortfall in µF. For the fixture: "U10: `cin` holds 5.000 µF effective at 26100 mV under the declared derating, 5.000 µF short of the 10.000 µF minimum … non-blocking until a maker's DC-bias figure is held for its parts, when it is enforced (ruled D2 (c))", and the same for U11.
  - Once any of the role's parts has a maker figure, including one held only below the working voltage, it refuses `EffectiveCapacitanceBelowMinimum`.
  - TLV755's 0.47 µF stays `on_declared: enforce`, because its own datasheet states the 50 % it is judged at.
  - A ratio no figure or derating covers still refuses: a missing figure is not a shortfall.
- **D1 (a), D3, D4:** accepted as built, with no change.
- **D5: backlog, required before `4500`.** The emitter draws `power.input.hot_plug` and reads `controls`; Sentinel-02's board must carry its bulk capacitor. Owner: the next eda work order after `4300g`. The verdict's D5 condition stays.
- **Backlog row (ruled):** effective input capacitance enforced when the chart arrives, and the fixture's input capacitance re-picked then. Owner: the next power work order.
- **TVS:** binds the 6S fixture only. Sentinel-02 is ruled 3S–4S (`4500` declares its own).
- **Mutation of the ruling's guards** (one mutant each, `aethelgard-ir --test power`):
  - Mutants: report regardless of mode; the declared flag never set; a maker point, or the capped last point, counted as declared; the ceiling at 501; the class-2 test inverted.
  - The first run left one survivor: the capped last point. A test now pins it. **6 of 6 killed** [measured, `~/.cache/aet-4300d3/evidence/mut/rulings-log.txt`].
- **Shard time, as ruled at 22:55:** when `4300b` merges, merge `main` and take its `ci/test-weights.tsv`. Re-measure only if a shard still passes 20 minutes.

## 8. Merge of `main` after `4300b`, and READY

- **Merged `origin/main` at `25640c5`** (4300b) in `a3c43ab`, by merge commit.
  - **Specification renumbered 9 → 10.** 4300b took 9 (`serial_ports`); this work order merges before `4300g`, which takes 11. Power-parts schema stays 2.
  - `ci/test-weights.tsv`, `ci/colocate.tsv` and `TIER2_MIN` (155) are `main`'s exactly: `git diff origin/main -- ci/ .github/` is empty. This branch adds no test of 0.5 s or more, so it adds no weight row.
- **What the renumber moved** [measured]: specification hashes only.
  - `boards/sentinel-01` was regenerated: 8 files, hash lines only.
  - The reference's hash was replaced in the twin's, Renode's and the CLI README's pinned topologies, and in `verify_2k2_tier2`'s prefix.
  - The svd-hal UI fixtures were re-blessed (`motors-pwm`, `serial-four`), hash-only.
  - `solve_cli`'s two digests and `order_2203`'s three were re-taken. With `main`'s hash put back, each equals `main`'s exactly; the boot image did not move.
  - Main's pincheck topology still carries an older hash, which no test reads. It is left as `main` has it.
- **Gates on `a3c43ab`** [measured, Furnace]:
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  - `cargo test --workspace --no-fail-fast`: 110 binaries, 0 failures.
- **The plan** [measured]: `ci/shard.sh assign` on this branch's 1892 workspace tests with `main`'s weights plans 1046 s per shard. That is the figure 4300b's accepted run `bcc4135` printed (43796790248), so this branch leaves the plan as `main` has it.
- **CI before the merge:** `73b59e1` was green, shards 13.3 to 16.9 minutes, tier 2 129 = floor.
- **CI on this head:** read per shard, as ruled on #152 (comment 6906540406). The final line is printed only once it is green with no shard over 20 minutes on both the run and its one re-run.

**READY.**
- Every ruling of comment 6902904831 is applied (§7).
- The full adversarial pass, the mutation runs and the reduction pass are done (§4–§6, §7).
- The backlog rows are named: D5 (the emitter draws hot-plug parts and reads controls, before `4500`); effective input capacitance enforced when Murata's chart arrives; `tRSTTEMPO` named as typical; F9 and F11.
