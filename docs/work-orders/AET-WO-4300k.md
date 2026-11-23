# AET-WO-4300k — Sentinel-03 fabrication readiness: a per-board gate over what the engine already holds, and the board fixed until it passes

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300k-sentinel-03-readiness`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before any Sentinel-03 order. Written 2026-11-23. **Cadence:** a measurement, then a build with several stops. A specification bump. It moves Sentinel-03's bytes and creates its package (a dispatch).
**Order: after `4300j`, `4240e` and `4410`, all merged.** This work order and `4410` collide. `4410` widens `SolvedSense` in `aethelgard-ir`, and this work order's crystal makes hwdef refuse Sentinel-03 (`OscillatorNotCited`) until `4410`'s 16 MHz row exists. This gate also needs the hwdef to generate. With `4410` first, this PR can write the package. With it second, it could not.

**Grounds** (verified at `54e16367` unless marked):
- **Review 3** A1, A2, A4, A6, A7, A13, A15 (`fab_tolerances`), B1–B3, C1, and its ledger (`….response.md`). §7.1 rows taken here: L950 (input capacitance), L1122 (4S), L1124 (BOR, reverse polarity, fuse), L1125 (crystal).
- **Scotty, 2026-11-23:** **at most 4S** (the 6S variant is row L1123, not this work order); and **the crystal goes on Sentinel-03** by `4240`'s H1 path (HSE 16 MHz → PLL1, `AET-WO-4240.md:132`). The part `ABM8AIG-16.000MHZ-12-2Z-T3` (`4240` FEEDBACK:25) stays provisional until it is checked against **AN2867 Rev 25**, held on Furnace at `~/.cache/aet-held/st-AN2867-rev25.pdf`, sha256 `cc868fd2b301d422bb4f58c11c8d9f7214037ae4414a2da56ab9c55f78b5d578` (the hash is not checked by the author: check it first). Today's rule cites Rev 10 (`ir/src/clock.rs:241`, `:253`, `:346-382`).
- **The fixture** `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` (schema 28): `power` has no `drop_limits`; `battery.cells` 3–6 at `cell_max_millivolts` 4350; `mcu.hse_hz` 0, `bor_level` 3; the stackup's `dielectrics` cite OSH Park and its `fab_tolerances` JLCPCB; U10's and U11's `junction.copper_square_centimetres` 20, "declared by the fixture"; one `cin` per buck (`GCM32EC71H106KA03`) against `effective_min_picofarads` 10 µF (`ir/data/power-parts.json:407-408`). 17 test files and `scenarios/sentinel-03/` (11 files) name it.
- **A1.** `connector_edges` sets "cable side out" for every `Connector_` part (`eda/src/pcb.rs:758-761`). `place_connectors` puts each courtyard's +y side on the edge (`:1452-1525`; `:1525`). In the XT30 footprint (`kicad-lib/data/kicad-10.0.7/footprints/Connector_AMASS.pretty/AMASS_XT30PW-M_1x02_P2.50mm_Horizontal.kicad_mod`), the pads sit at y 0, the F.Fab body at y −13.1 to −4.1, and the courtyard at −13.6 to 2.25. So the tails are at +y and the body points into the board (measured here). Which end opens is the drawing's: unverified. The tables are `ir/src/connectors.rs:56-62` (JST) and `:127-142`, `:165-199` (XT30). The tests at `pcb.rs:3081`, `:3190`, `:3259` and `:3279` check only that the courtyard is on the edge.
- **A2.** `supplies` returns no gate without `drop_limits` (`eda/src/supply.rs:1343-1356`). The key is optional (`ir/src/power.rs:120-128`; validated at `:863-880`). The gate also needs four stackup minima (`SupplyInputUndeclared`), which `cli/tests/supply_tier1.rs:49-60` adds with "test:" sources. So roadmap L1022's "the gate passes (85.8 mV)" is true of a test copy only.
- **A4:** `power.rs:2663` reads the declared area, `:2696` prints "screened", and `:2709-2715` compares the layer count only.
- **A13:** the sense check (`power.rs:2122-2233`) bounds neither total resistance nor dissipation; the load check (`:1457-1471`) judges `iout` only, and the high-side limit meets only isat (`:1910-1935`); the false line (`:3058-3075`) lists every device on the domain without reading `loads[].reference`.
- **A15:** `RingRule::of` is `None` without `fab_tolerances` (`eda/src/stack.rs:64`). §7.1 L1165 says why that is not a `validate` refusal: Sentinel-01 is ordered and declares none.
- **Hot plug.** `power.rs:2623` prints the ring as "not computed". VIN's absolute maximum is 38 V (`power-parts.json:275-279`, SNVSAN3F p. 6).
- **Gate pattern.** `verify`'s preconditions live at `cli/src/verify.rs:175-235`; `acknowledge_unwitnessed` (`ir/src/lib.rs:209`) is the acknowledgement pattern. CI derives Sentinel-01's package at `.github/workflows/ci.yml:569-571`.

> **What this is:**
> - **(1) B1, the readiness gate.** A `verify --fab-out` precondition that refuses by name: an optional key whose absence turns a gate off or relaxes it (found by a key-deletion sweep); a hwdef that does not generate (`aethelgard_hwdef::generate` on the solved topology); a "screened, not guaranteed" condition the specification does not acknowledge by a stable id.
> - **(2) A2 and A15.** `drop_limits` is required wherever `power` is declared (`RUL-14`, amended 2026-11-17). The readiness gate requires `fab_tolerances` for any new package. **Sentinel-01 is never back-filled** (the ledger's A15 row).
> - **(3) A7.** One fabricator that every stackup bound, minimum and tolerance cites, or a refusal when their sources differ. The supply gate's four minima come from it.
> - **(4) A1 and B3.** The mating side and the cable exit become a per-footprint fact in the connector and power-in tables, cited by page. Placement reads the fact; the "+y is the cable" convention no longer decides. A connector whose opening faces the board is refused.
> - **(5) A4.** The copper joined to U10's and U11's thermal pads is measured from the drawn board, or the pours are drawn (ruled at the stop). The declared number is then no longer what the screen rests on.
> - **(6) The board itself.**
>   - **4S:** `cells.max` 4. The doubled pack is judged against each VIN absolute maximum, refused with no clamp declared: 4 × 4350 = 17,400 mV, doubled 34,800 mV, under 38 V. Roadmap L1122's 33.6 V assumes 4.2 V cells. The factor 2 is an undamped LC step: cite it, or label it a derivation.
>   - **A6:** enough input ceramics per buck to meet the 10 µF effective. Three per buck is about 12.3 µF (inferred).
>   - **The crystal:** declared as `hse-crystal-16.json` declares it, checked against AN2867 Rev 25.
> - **(7) A13.** The sense divider bounded in total resistance and in dissipation in its package, each bound cited. A regulated rail with no load refused, or named in the verdict (measure which). A buck's sourced-against-solved current gap printed (refusing it needs a ruling on `4300d3` F9). Peak inductor current, `iout` plus half the ripple at VIN's maximum, against `high_side_limit_milliamps`' minimum. The false "not in its load" line fixed.
> - **(8) B2.** Sentinel-03 becomes `examples/sentinel-03.json` (`git mv`; the paths that name it follow). It gets `fab/sentinel-03/package/` by dispatch, a `BENCH.md` bound to `MANIFEST.json`'s ELF hash (`3308`'s binding), and a CI `--against-fab` step beside `ci.yml:569-571`.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> **Questions for Scotty, not decided here:** BOR level 3's falling threshold (2.54–2.68 V, the review's figure) below the flash's 2.7 V floor; and no reverse-polarity protection or fuse. Cite both at source (DS12117's BOR table; W25Q64JV's VCC minimum). The gate lists them as unacknowledged conditions until he rules.

## 1. Build

1. **Measure first.** Commit predictions in `spike/4300k/`, then record:
   - the `validate`/`solve`/`emit`/`extract`/`verify` baseline on Sentinel-03, with every condition it prints, and on every committed specification;
   - the key-deletion sweep, per gate;
   - each thermal pad's joined copper per layer, and TI's θJA figure and its board, at source (the sub-reviewer's 43 °C/W is unverified);
   - the XT30 drawing (`~/.cache/aet-ds/aet-2304/amass-xt30pw-m/xt30pw-m-spec.pdf`): its sha256 against `AMASS_XT30PW_M_V12_SHA256` (`connectors.rs:157`), and the page that shows the opening relative to the tails;
   - AN2867 Rev 25's gain margin, CL and ESR, by page, against the ABM8AIG row, and every place Rev 25 differs from Rev 10's text in `clock.rs`;
   - which fabricator can source every bound;
   - which bytes move, by board. **Predicted:** Sentinel-03's topology and board; every committed specification's version literal; `fab/sentinel-01-bench/package/MANIFEST.json`'s hash lines (the ELF: measure); no file `ORDERED.sha256` lists; no other board's pin or copper.
2. **Stop with the measurement and these decisions:** (a) the thermal copper, measured or pours drawn; (b) the fabricator, and how its single source is declared; (c) how Sentinel-01's `--against-fab` stays byte-identical under the new precondition without back-filling it; (d) the gate's key list and condition ids; (e) Scotty's two questions.
3. **Build (1)–(7) to the ruling,** with goldens and scenarios moved by their generators.
4. **(8):** promote the specification, request the dispatch, commit the package byte for byte, and add `BENCH.md` and the CI step. Measure the step's CI time.
5. **Re-run the review's mutations** (A13's five, J4 rotated): each is refused by name.

## 2. Acceptance

- **`verify --fab-out` writes `fab/sentinel-03/package/` from `examples/sentinel-03.json`** with every clause met. It refuses a deleted gate key, a hwdef refusal, and an unacknowledged condition, each with its test.
- **J4 opens off the board** by its table fact. Every connector and power-in row states its mating side with a page. A planted wrong rotation is refused.
- **4S, the crystal, A6 and A13** each hold with their tests. A copy with `cells.max` 6 is refused for want of a clamp.
- **No committed board other than Sentinel-03 moves a pin.** `ORDERED.sha256`'s test stays green.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `crates/aethelgard-ir` (power, connectors, `clock.rs`'s citation, validate, versions, held data, README, tests); `crates/aethelgard-eda` (connector placement, the mating fact, the thermal measure or pours); `cli/src/verify.rs` (the precondition), and the tests and scenarios that name Sentinel-03; `examples/sentinel-03.json`; `fab/sentinel-03/` and `fab/sentinel-01-bench/package/MANIFEST.json`'s hash lines, by dispatch only; `ci.yml`'s one new step; `spike/4300k/` and FEEDBACK.
- **Not:** hwdef and pincheck (`4410`'s); svd-hal (Sentinel-03's firmware moves by its generator only); the twin, chaos; the ran-or-not table (`1609`); the clamp, the TVS, 6S (L1123); a BOR or input-protection design; anything above `ORDERED.sha256`'s first ruling block; roadmap L1022's sentence (the architect's edit: FEEDBACK only).
- **Running beside you:** the backlog in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4300k/pr-title-body.md`.

## 5. Adversarial pass, full

- **A gate turned off quietly:** a key deleted, set to `[]` or `null`, or renamed; a condition acknowledged that never occurs, or one named under another reference.
- **A connector rotated 90° or 180°,** a JST row that regresses, or an opening on an edge shared with a mounting hole.
- **A declared fabricator with one bound citing another,** and a source string that merely contains the maker's name.
- **Each bound at its edges:** the divider, the peak current at VIN's maximum and minimum, the doubled pack at exactly 38 V.
- **A citation that does not say what is claimed:** read AN2867 and the AMASS drawing in the PDF, never an extract.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement and decisions (a)–(e).
- **The crystal fails AN2867 Rev 25** on gain margin, CL or ESR, or the PDF's hash differs. Stop with the page and the numbers.
- **The XT30 drawing does not show the opening, or puts it on the tails' side** (it would contradict the placement fix). Stop with the page.
- **Any committed board other than Sentinel-03 moves a pin, or an `ORDERED.sha256` byte would move.** Stop with the diff.
- **Scotty's two questions are unruled at step 4.** Stop before the dispatch.
- **A held page does not support a bound you need** (the divider, the factor 2). Stop, and name the document a ruling would need.
