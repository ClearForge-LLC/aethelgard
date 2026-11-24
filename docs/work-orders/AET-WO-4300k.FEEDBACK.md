# AET-WO-4300k — FEEDBACK: Sentinel-03 fabrication readiness (specification 30, power-parts 12)

**Branch:** `wo/aet-4300k-sentinel-03-readiness`. **Base:** `0faaaec3`. **Builder:** Claude (Opus 5.5) as project manager, with subagents (Sonnet 5.5 unless noted) for measurement, datasheet reading, library vendoring, the acknowledgement reasons (Opus: judgement) and tests. Each subagent's report was spot-checked against its artifact before use (§0).

## 0. How it was made

- **Measure, stop, build.**
  - Predictions were committed first (`521dcd06`), then the measurement and the stop after §1.2 (`spike/4300k/RESULTS.md`, `STOP.md`).
  - Ruled 2026-11-24T09:45Z: every lean taken, with (b) OSH Park.
  - Two questions were asked during the build and ruled:
    - Q1 (the fuse), 10:00Z: (a), SF-1206F700-2.
    - Q2 (the thermal copper), 10:30Z: step 1.
- **Subagents, and what was checked of each:**
  - the key-deletion sweep (host), then its image extension for `check support/erc/drc`;
  - the 36-specification blast table;
  - thermal, fabricator and held pages;
  - the datasheet rows (spot-checked in the extracts: VGSS ±20 V, RDS(on) 33 mΩ at −4.5 V, SF-1206F400-2 4.00 A 16 mΩ 0.954 A²s);
  - the KiCad vendoring (every blob SHA checked against GitLab's API);
  - the JST mating sides;
  - the 46 acknowledgement reasons (Opus);
  - the new test files.
- **OSH Park's page** was fetched by me (sha256 `cf14c792…`) to confirm every value the worker read: drill size ±2.5 mil, position 2 mil, annular ring 4 mil, prepreg ±0.0202 mm, core ±0.0991 mm, and an edge keepout of 15 mil, inside the engine's 0.5 mm.
- **A scratch test file was committed by accident.** `crates/aethelgard-cli/tests/zz_scratch_acks.rs` went in with WIP commit `b52c57d5` and was removed in `01c65708`. It is in the branch's history only. A squash at merge removes it.

## 1. The measurement and the stop (§1.1–§1.2)

See `spike/4300k/RESULTS.md` (M0–M12, predictions scored) and `STOP.md` (decisions (a)–(e), D6–D14).

- **No §7 stop on documents:** every held hash matched; the crystal passes AN2867 Rev 25; the XT30 drawing puts the opening away from the tails.
- **Corrections the advisor review made before the push:**
  - the fuse current: the tree's `input_steady_milliamps` is the outputs' sum, not the input;
  - D6 and D13 are one decision;
  - (c) rests on how `--against-fab` compares;
  - the fabricator was not JLCPCB-by-format.

## 2. What was built

- **(1) The readiness gate, `aethelgard_cli::readiness`.**
  - `verify --fab-out` refuses `FabricationNotReady` by the first gap, in this order:
    - `fab_tolerances`, `fabricator`;
    - with `power`: `drop_limits`, `dc_bias_derating`, `loads` and `net_classes`;
    - a hwdef that does not generate;
    - a device floor above the reset (`supervisor::floors_above_reset`);
    - an unacknowledged condition.
  - It is the last refusal before `--against` and every gate, after the campaign's coverage, a stand-in and the ledger. A board refused for one of those keeps that refusal (a sequencing choice, reported 11:4xZ).
  - A stale acknowledgement is refused in every mode (`ConditionAcknowledgementStale`).
- **(R) The re-bind (ruled 11:25Z), `cli/src/rebind.rs`.** `verify --fab-out OUT --rebind COMMITTED` does not judge readiness.
  - The package is derived as `--fab-out` derives it, then compared with the committed one:
    - MANIFEST and FAB-NOTES line by line, where only the binding lines may differ: `specification_sha256`, `topology_sha256`, `verdict_sha256`, each scenario's `sha256` and FAB-NOTES' hash, plus FAB-NOTES' specification line;
    - every other file by `tree::against`, which never follows a symlink, takes regular files only and caps the entries.
  - It is refused `RebindNoPackage` or `RebindOtherBoard` before any gate, and `RebindMoved` after them, naming each moved file. Nothing is written on a refusal.
  - CI's write dispatch re-binds Sentinel-01 (`ci.yml`). Measured in the image: only the binding moved, and `ORDERED.sha256` checks.
- **(2) and (d).**
  - `drop_limits` is required for the MCU's supply.
  - New refusals: `RegulatedDomainUnloaded`, `DoubledPackOverAbsoluteMaximum` (a labelled derivation, D7) and `PeakCurrentOverLimit` (SNVSAN3F p. 14 §8.3.3, p. 21 Eq 4).
  - The A13 false line is fixed (it reads `loads[].reference`). Condition ids come from `Ctx::screen`.
- **(3) `board.stackup.fabricator` with structured cites** (`StackupSourceUncited`, `StackupSourceForeign`). Sentinel-03 declares OSH Park.
- **(4) The connector opening fact** (`connectors::Opening`, cited by page), placed by the placer and judged on the drawn board (`eda::opening`, `extract.openings`). J4 is turned 180°.
- **(a) The thermal pours** (`Stack::pours`), solid joins by Q2 step 1, and the measure (`eda::pour`, `extract.thermal`, `power::junction_screen`).
- **(9) `mcu.reset_supervisor`, `ir::supervisor`:** the TLV803S on NRST, its pull-up and bypass judged (`SupervisorBelowDeviceFloor`, `SupervisorAboveRail`, `SupervisorOutOfRange`).
- **(10) `power.input.protection`:** fuse, P-FET, zener and gate resistor, each a row from a held datasheet (`BatteryInputUnprotected`, `ProtectionPartOutOfRange`, `FuseMeltsOnHotPlug`, `FuseInMotorPath`, `EscUndeclared`). The path's drop is counted in every battery-fed regulator's headroom.
- **(11) `supply_by_read_clock`** on W25Q64JVxIQ, with `Part::supply_min_at`.
- **(T) The ready fixture**, `crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json`, generated by `spike/4300k/build/ready_fixture.py` from `spike/4300k/build/ready-acks.json`.
  - **Tests moved to it:**
    - `fab_tier1`: the run that cannot pass, the tree that differs, the acknowledged reference;
    - `against_fab_tier2` ×2, `verify_47k_tier2` ×2, `verify_2k2_tier2`'s fab-out test, `board_tier2`'s identifier test.
  - **Kept on the reference:** `verify_2k2_tier2`'s binding test (split, no fab flag) and the acknowledged reference under `--against-fab`. No gate's coverage dropped.
  - **The four-layer tier-2 tests (Q3 (a), ruled 12:45Z):** they run on a readied copy of `four-layer.json`, `tests/common/ready.rs::ready()` with the reference's console (USART1, a console connector, the rebased `serial-rx-open-console` scenario); the inner-layer and stackup points stay on a four-layer board. The grown fixture is that copy, built in code.
- **New tests:**
  - `ir/tests/readiness_4300k.rs`: every new refusal at its edges, and each new condition where it holds and only there.
  - `cli/tests/readiness_tier1.rs`.
  - `fab_tier1`'s readiness and re-bind cases, and its CI pin.
  - `against_fab_tier2`'s re-bind (the clean case passed in the image; the moved case reran after the fixture settled, §7).

## 3. The adversarial pass (§5, full)

Two Opus reviewers ran read-only, one per lens. They were given the code and the rulings, not my conclusions. Every finding is in its own bus post (architect.md, 2026-11-24 afternoon).

- **Gates and the re-bind.** No path writes a package that skipped readiness, other than a re-bind.
  - **F1 (built):** the moved-byte refusal had no test at its call site. A tier-2 test now drives it.
  - **F2 (the CI is the control):** the tool cannot tell a committed package from any earlier `--fab-out` of the same board. `fab_tier1::ci_rebinds_only_sentinel_01_s_committed_package` pins `--rebind` in `ci.yml` to one path. `fabrication_ready` is not in `verdict_sha256`, so a package carries no mark that readiness was skipped.
  - **F3 (built):** the committed directory was read with `fs::read`, which follows symlinks, blocks on a FIFO and has no cap. It is now read as `--against-fab` reads one.
  - **F4 (built):** MANIFEST was compared as parsed JSON, so duplicate keys passed. It is now compared line by line.
  - **F5 (built):** the dispatch nested Sentinel-03's package inside Sentinel-01's. Sentinel-03 now has its own artifact.
  - **M1 (ratified 12:45Z, both directions):**
    - Narrower than the ruling: a moved ELF refuses, and FAB-NOTES' tool-version text may not move.
    - Wider: `topology_sha256` may move, since the topology carries the specification's sha.
  - **Low:** a failing `--against-fab` or re-bind verdict now also names readiness "not judged".
- **Power, thermal and board.** The hand checks agree (I²t, current, drop, supervisor margin, peaks, junctions), and 19 datasheet values match their held pages. No wrong verdict on Sentinel-03.
  - **F1, built:** `esc: null` on a protected input is refused (`EscUndeclared`, `INV-06`).
  - **Named, not refused:**
    - F2: the θJA figure's copper (`theta_ja_copper_mismatch`);
    - F3: the 25 °C path resistance;
    - F6 (b): a drive set by the zener;
    - F7: a ceramic with no tolerance;
    - F9: the reset delay unstated under 3,190 mV.
  - **Changed figures:**
    - F4: the input current iterated once, now 1,653 mA and 71 mV;
    - F6 (a): the drive less the fuse's drop;
    - F7: ceramics at their DC-bias tolerance, so the I²t is 2.46 A²s.
  - **F5:** a board above the fuse row's 85 °C is refused.
  - **F8:** TLV803SDBZR is active (held p. 16).
  - **F9:** the pour's clearance is KiCad's measured 0.2025 mm.
- **Earlier corrections from workers' cross-checks:**
  - the 1,640 and 749 mA figures and the 4 mV margin;
  - `Q1` reused as a load reference in `load_references`;
  - the IR power tests' external-part indices, which selected the wrong part after C60 was inserted (now by reference).

## 4. Versions and what moved

- Specification 29 → 30 (`versions::CHANGES` row 30). Power-parts 11 → 12: new kinds `pfet`, `zener`, `fuse`, `supervisor`, `bor_falling_min_millivolts` on the MCU row, and LMR33630's cin minimum enforced (A6).
- `parts.json` stays at 13: `supply_by_read_clock` is optional, and only W25Q64JVxIQ carries it.
- `FAB-NOTES.md` moves with the bump (granted into the fence, ruled at 09:45Z).

## 5. Decisions the architect made, and how they were built

- **(a) Pours drawn, measured (ii-a); Q2 step 1.**
  - On a board with a buck, GND pours go on every outer layer no plane fills (`Stack::pours`).
  - `aethelgard_eda::pour::joined` measures a lower bound per layer:
    - 50 µm cells;
    - other-net copper grown by the clearance, and the edge by 0.5 mm;
    - the fill's 0.25 mm minimum width as an opening;
    - the component reached from the pad or its thermal vias;
    - a nearer-pad split (ties count for neither).
  - `extract.thermal` screens each buck by `power::junction_screen` at the least joined layer.
  - **Measured** on the drawn board (cm², F.Cu / In1.Cu / B.Cu):
    - U10: 20.10 / 38.51 / 36.59, needs 15;
    - U11: 13.14 / 24.77 / 23.82, needs 7.
    - With KiCad's default reliefs on every GND pad, U10 was 13.28 and U11 6.54. That fired the stop, and Q2 ruled step 1.
  - **A deviation from Q2's "keep reliefs" (reported 10:4xZ).**
    - With reliefs on the 0402/0603 passives, KiCad's DRC refused two starved reliefs (C2 and C38, one spoke against two).
    - Those pads are instead not joined to the pour (`zone_connect 0`) and keep their own vias to the GND plane. There is no relief to starve and no tombstoning exposure.
    - No small passive is joined solid, so `solid_gnd_join_small_passive` has nothing to list.
  - **(b) and (c) stay Scotty's options** (ruled 10:30Z): growing the outline toward `max_outline_um`, or lowering U10's 2 A (the DS-009 port budget). Use the numbers above.
  - **4300d2 D2 (a), the keepout's "no fill on its layer", is relaxed for pours, not planes.**
    - KiCad's refill keeps a pour out of U5's keepout, `pour` counts the keepout as no copper, and the DRC gate judges the filled board (it passes).
    - Planes keep the refusal.
  - **Assembly:** Scotty confirms the solid joins with the assembler before the order.
- **(b) OSH Park, declared by `board.stackup.fabricator`.**
  - Bounds cite one of its documents by id, a standard, or a condition by id.
  - Its two unpublished tolerances (track width; and thickness, whose min and max are derived from its published layer tolerances) are handled so: track width is a `{condition}` (`track-width-tolerance`), and thickness cites OSH Park as a derivation.
  - Copper max, permittivity max (IPC-4101 not held), the three copper and plating minima and copper temperature are `{condition}`s.
  - **Scotty confirms the fabricator before any order;** switching it is a data edit.
- **(c) `FabricationNotReady`** is a pre-gate refusal under `--fab-out` only, with no MANIFEST row. Under `--against-fab` the verdict's `fab` report names it "not judged". **The gap:** a later engine that adds a readiness key does not re-judge an existing package until it is re-written.
- **(d) The key list:**
  - `fab_tolerances`, `fabricator`; with `power`: `drop_limits`, `conditions.dc_bias_derating`, `loads` and `net_classes`. The image sweep added `net_classes`, whose emptiness drops DRC's power classes.
  - `diff_pairs` is not on the list; it is named where the USB pins are reserved (`usb_pair_undeclared`).
  - Condition ids come from the code's sites: `Ctx::screen` in `power.rs`, plus `supervisor::check`, `board::conditions`, `clock::conditions`, and the readiness module's spec-read ids.
  - 71 on Sentinel-03, each acknowledged with a reason (`spike/4300k/build/s03_spec.py`).
- **(e) and Q1:**
  - The protection is a fuse SF-1206F700-2, a P-FET DMP6023LSS-13, a 12 V zener MMSZ5242BT1G and a 100 kΩ gate resistor, between J4.2 and `VBAT`, with the sense divider after the FET.
  - The input current, computed from the engine's own losses, is 1,653 mA: 1,640 mA at the pack's 9.0 V minimum, iterated once at 8.93 V (the power review's F4). The input path drops 71 mV, counted in the bucks' headroom.
  - **"About 2×" was amended by Q1:** the hot-plug I²t bound (C·V²/2R_min, the fuse's own minimum the only R) clears only the 7 A part: 2.35 A²s against 3.25 at the Q1 ruling, 2.46 A²s since the ceramics count at their +10 % (F7).
  - Condition `fuse_hot_plug_bound_fuse_only:F1`. The fuse's 32 VDC rating is judged against the steady 17.4 V.
- **DMP6023LSS's "ADVANCE INFORMATION"** is white text in the left margin, present in the text layer of every page and invisible on the rendered page. It is kept as a condition (`datasheet_advance_information:Q1`), as ruled.
- **D6:** the TLV803S's rising maximum is VIT− max (2.99 V) plus its typical hysteresis (40 mV), judged against VDD_3V3's solved minimum less its 110 mV drop limit (3,034 mV, the solved minimum floored to whole mV). The margin is 4 mV, with the condition `supervisor_hysteresis_typical:U12`.
- **D7:** the factor 2 is labelled a derivation in the refusal, the condition and the code.
- **D8:** two input ceramics per buck: C60 and C61, 10.87 µF effective each pair at 17.4 V, worst case.
- **D9:** `board.forward: top`, U5 `x: right`. After J4's turn, IMU_SCK is 21,478 fF and IMU_MOSI 21,268 fF against 30,000.
- **D10:**
  - Sentinel-03's 46 ledger register/class entries are in. Every row is acknowledged.
  - Four held reasons the crystal and the new pins made false were rewritten: USART1.CR1 (FIFOEN set), GPIOB.OTYPER (PB10/PB11 open-drain), RCC.CR ignored-by-model (the HSE bring-up writes it), RCC.D1CCIPR (SPI1/2 on pll1_q).
  - **The reasons, for review:** every acknowledgement is listed in §8, as `examples/sentinel-03.json` holds it.
- **D11:** W25Q64JV's §9.2 rows are held by read clock (`supply_by_read_clock`); at 20 MHz, 2.7 V applies.
- **D12:** the flash floor is refused by `--fab-out` (`FabricationNotReady`, id `U4`) and is otherwise listed by `supervisor::floors_above_reset`. The Sentinel-01 bench row asks: did the board ever run under 2.7 V?
- **D13:** `DropLimitUndeclared` for the MCU's supply; every other regulated domain is `drop_not_judged`.
- **D14:** `sentinel-02-power` is 4S, with a 165 mV VDD_3V3 limit and the same FET, fuse, zener and gate resistor (`spike/4300k/build/s02p_spec.py`).

## 6. Rows (§7.1)

- **Q1 (c):** hold the pack's or the FET's minimum resistance, recompute the hot-plug I²t, and possibly step the fuse down to 4–5 A.
- **The supervisor as the reset floor in the twin:** with a supervisor on NRST, the reset is at its VIT−, not BOR3. The droop scenarios and the 2,680 mV MCU consumer floor still assume BOR3. This is for the twin's work order (D6).
- **(b) and (c) of Q2:** see §5 (a).
- **The supervisor row's board temperature** is not judged (its −40 to 125 °C covers the board's −20 to 85 °C); `supervisor::check` could judge it as `power` judges every power part.
- **1609 F2 closed for new packages:** `fab_tolerances` is a readiness key. Sentinel-01 is untouched, by (c).
- **The fuse's pulse I²t:** SF-1206F's melting I²t is typical and stated around 0.6 ms, while a hot-plug pulse lasts microseconds, and repeated plugs or contact bounce add up. Hold a minimum or pulse-derated figure (beside Q1 (c)).
- **θJA at this board's copper (F2):** the figure is for 2 oz/1 oz, and the board declares 1 oz/½ oz, so U10's 117.8 °C screen has less margin than it shows. A derating needs a ruling or a bench thermal row; until then it is the condition `theta_ja_copper_mismatch`.
- **`ViaRegionsOverlap` fires in no test.** The pours re-routed the two-layer power fixture before 1324 merged, and since 1324 the router keeps a judged net's vias one region apart. Its `supply_tier1` test checks the regions on both nets (taken on the merge), but nothing fires the refusal itself: a planted via either keeps its region apart or is refused for clearance first. A synthetic witness belongs with the router's owner.
- **`fabrication_ready` is not in `verdict_sha256`** (the gate review's F2, ruled 12:45Z as a row): the verdict line is built before the `fab` report, so a package's bytes carry no mark of whether readiness was judged, skipped by a re-bind, or not judged under `--against-fab`. The CI pin test is the control.
- **`ci/colocate.tsv`:** `verify_2k2_tier2` roughly doubled (+~450 s); re-weigh it from CI's own times after the first green run, by the REFIT RULE (ruled 12:45Z).
- **The re-bind's trust root is CI** (the gate review's F2). A tool-side check that the directory is the committed one (git-tracked, under `fab/`) would make the tool itself the control.
- **`--rebind` is on the MCP `verify` tool** (the snapshot blessed): the MCP server runs the same local CLI, so it grants nothing the CLI does not; CI stays the trust root.
- **`with_tools.rs`'s LP1 comment** describes `verify_2k2_tier2` before the split; it is outside this fence.

## 7. Gates, CI, package

- **Reduction pass (diff), by a fresh read-only subagent; every proposal spot-checked before it was applied:**
  - −145 net (commit `f0e5986c`: 141 added, 286 removed).
  - 7 duplicates removed:
    - the pour-zone swap and predicate (`Stack::pour_zones`, `Stack::is_pour`, in place of 7 copies);
    - three identical JST GH openings (one const);
    - `tree::same` on `tree::regular`;
    - `opening.rs`'s second row lookup;
    - one `hex64`;
    - the ready fixture's consts and argument lists in `fab_tier1`;
    - the power tests' five helpers in `ir/tests/common/power_table.rs`.
  - 1 dead path removed: `OnShortfall`, since every row enforces after A6.
  - 2 kept as deliberate:
    - the fuse's 85 °C guard (a review ruling);
    - `pour`'s f64 geometry beside `supply`'s exact integer one (it errs toward blocking).
  - 1 rejected on spot-check: P8. `GmCrit.an2867_margin_hundredths_uncited` is in `TreeVerdict`, ruled at 4240a as information.
  - Findings out of scope:
    - `root()` is re-declared in about 15 CLI test files beside `common::verify::root`;
    - `fab_tier1::rebased_verify` duplicates `common/ready.rs::rebased`;
    - three private `Shape` types (pour, supply, route);
    - `spec`/`refused` copies in 7 more IR test files.
- **Mutations: 41 of 41 killed.** One mutant per guard this work order adds or changes, against the suite it lives in (`~/.cache/aet-4300k/mut/{mutants.py,run.py,results.tsv}`, capped scope).
  - The first sweep left 4 survivors, each closed by a new edge test:
    - the doubled pack at one millivolt of VIN absolute maximum (37,999 mV);
    - Gm_crit exactly at 1.5 mA/V (16 MHz, CL 1 pF, ESR 37,104,895 mΩ, refused, against 37,104,894, which holds);
    - FAB-NOTES' sha line after a line that is not the specification's lead;
    - the pour's nearer-pad tie (the split factored into `pour::nearest` with a unit test).
- **Gates:** PENDING (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (also with `with-tools`), `cargo test --workspace`, after `main` is merged).
- **Tier 2 in the image (16G, swap 0):**
  - The Sentinel-01 re-bind passed every gate. Only the binding moved, and `ORDERED.sha256` checks.
  - `against_fab_tier2` passed 3/3, the re-bind test included (1036 s). `four_layer_fab_tier2` passed 2/2 (453 s).
  - `verify_47k_tier2` passed 2/2, `verify_2k2_tier2` 2/2 and `board_tier2`'s identifier test 1/1, run by the tier-2 worker.
- **Package:** the bootstrap is accepted (12:45Z). `fab/sentinel-03/package` and Sentinel-01's re-bound MANIFEST and FAB-NOTES come from the architect's write dispatch on the final head, committed unchanged. Until then `bench-package` refuses `--against-fab` on Sentinel-03, and `bench_board_tier1` fails on the specification hash.

## 8. Appendix: Sentinel-03's acknowledgements (D10)

Generated from `examples/sentinel-03.json`. There are 76 conditions (`acknowledge_conditions`) and 68 ledger entries (`acknowledge_unwitnessed`).

### Conditions

| id | reason |
|---|---|
| `buck_junction_screened:U10` | TI's Figure 9-3 is typical; U10's 1170 mW is screened at the copper extract measures on this board (>= 15 cm2 needed), and the bench reads its case temperature at 2 A |
| `buck_junction_screened:U11` | TI's Figure 9-3 is typical; U11's 550 mW is screened at the copper extract measures (>= 7 cm2 needed); the bench reads its case temperature |
| `buck_timing_conditions:U10` | tOFF-MIN, RDS(on) and fSW are stated at VIN 12 V; U10 runs 9.0 to 17.4 V, inside the 3S-4S range the headroom and on-time checks use with margin |
| `buck_timing_conditions:U11` | tOFF-MIN, RDS(on) and fSW are stated at VIN 12 V; U11 runs 9.0 to 17.4 V, inside the 3S-4S range the headroom and on-time checks use with margin |
| `bulk_esr_condition:C1` | C1's ESR is stated at 20 C and -40 C only; the board's -20 C floor sits between them, and the doubled-pack bound does not rely on its damping |
| `datasheet_advance_information:Q1` | DS37198 is marked advance information in its text layer; Q1's VDS, VGS and RDS(on) margins are large (60 V vs 34.8 V), re-read when a final sheet is held |
| `derating_declared:X5R+X7R+X7S` | no maker DC-bias curve is held for the output and VCC ceramics; TI's 50 % rule is applied, and the cin ceramics use Murata's own curve |
| `dielectric_accepted:C10` | C10 is X7S (GCM32EC71H106KA03), accepted where TI's example says X7R or better (ruled D-B8); its effective value at 4S is counted from its own curve |
| `dielectric_accepted:C20` | C20 is X7S (GCM32EC71H106KA03), accepted where TI's example says X7R or better (ruled D-B8); its effective value at 4S is counted from its own curve |
| `dielectric_accepted:C60` | C60, U10's second cin (A6, ruled D8), is X7S like C10, accepted under ruling D-B8; the two give 10.87 uF effective at 17.4 V |
| `dielectric_accepted:C61` | C61, U11's second cin (A6, ruled D8), is X7S like C20, accepted under ruling D-B8; the two give 10.87 uF effective at 17.4 V |
| `drop_not_judged:VDD_5V` | VDD_5V feeds only J6.1 and J7.1 off-board (DS-009 ports); no on-board load pin exists for the supply gate (ruled D13) |
| `effective_no_minimum:U10.cout` | U10's output ceramics have no effective minimum in TI's row; their 44 uF is a placeholder derating until Murata's curve for GRM32ER61C226KE20 is held |
| `effective_no_minimum:U11.cout` | U11's output ceramics have no effective minimum in TI's row; their 44 uF is a placeholder derating until Murata's curve for GRM32ER71A226KE20 is held |
| `effective_typical:U10.cin` | Murata's DC-bias chart is typical at 25 C; U10's two cin are counted worst case (curve, -10 % tolerance, -22 % X7S) and clear 10 uF by 8.7 % |
| `effective_typical:U11.cin` | Murata's DC-bias chart is typical at 25 C; U11's two cin are counted worst case (curve, -10 % tolerance, -22 % X7S) and clear 10 uF by 8.7 % |
| `fuse_hot_plug_bound_fuse_only:F1` | F1's I2t bound counts only its own minimum resistance (no FET, copper, connector or pack minimum is held), so 7 A was ruled (Q1 (a)); holding one is a row |
| `hot_plug_doubled_pack:U10` | the ring at U10's VIN is bounded by twice the 17.4 V pack, a derivation (ruled D7), under 38 V; C1's damping only lowers it |
| `hot_plug_doubled_pack:U11` | the ring at U11's VIN is bounded by twice the 17.4 V pack, a derivation (ruled D7), under 38 V; C1's damping only lowers it |
| `hse_drive_level_unmeasured:HSE` | AN2867 Rev 25 Step 4 needs a measured Vpp at OSC_IN; Y1's drive level is a bench row before the order |
| `hse_safety_factor_unmeasured:HSE` | AN2867 Rev 25 Step 3's negative-resistance safety factor needs the board; gmcrit 0.555 mA/V is under the 1.5 mA/V limit; a bench row |
| `inductor_dcr_at_25c:U10` | L1's DCR is stated at 25 C and not raised for 85 C; the headroom check counts it, with the input path's 70 mV, and holds at 9.0 V |
| `inductor_dcr_at_25c:U11` | L2's DCR is stated at 25 C and not raised for 85 C; the headroom check counts it, with the input path's 70 mV, and holds at 9.0 V |
| `inductor_ripple_outside_guidance:U10` | 8.2 uH meets Equation 5 but not Equation 4's 20-40 % ripple over the whole 3S-4S range (ruled D6); the peak current stays under the high-side limit |
| `inductor_ripple_outside_guidance:U11` | 8.2 uH meets Equation 5 but not Equation 4's 20-40 % ripple over the whole 3S-4S range (ruled D6); the peak current stays under the high-side limit |
| `input_path_resistance_at_25c:Q1` | Q1's RDS(on) and F1's resistance are 25 C figures; hotter, the 71 mV drop grows, a small part of the volts the bucks keep above their need at the 9 V pack minimum; a bench row |
| `inrush_bulk_at_rated:C1` | C1 enters the hot-plug I2t at its rated 47 uF (no tolerance in its row); at +20 % the bound is 2.72 A2s, still under F1's 3.25 A2s |
| `inrush_ceramic_at_rated:C11` | C11 is 220 nF of the 91.4 uF the hot-plug bound counts; even 20 % over, it moves that I2t under 0.1 %, far inside F1's margin |
| `inrush_ceramic_at_rated:C21` | C21 is 220 nF of the 91.4 uF the hot-plug bound counts; even 20 % over, it moves that I2t under 0.1 %, far inside F1's margin |
| `isat_typical:L1` | IHLP-5050FD's 16 A saturation is typical only; it is 3.2 times U10's 5.05 A high-side limit maximum (ruled D-B3) |
| `isat_typical:L2` | IHLP-5050FD's 16 A saturation is typical only; it is 3.2 times U11's 5.05 A high-side limit maximum (ruled D-B3) |
| `lifecycle_unread:CRCW0603_F_K` | CRCW0603_F_K: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:DMP6023LSS-13` | DMP6023LSS-13: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GCM188R71H104KA57` | GCM188R71H104KA57: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GCM188R71H224KA64` | GCM188R71H224KA64: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GCM32EC71H106KA03` | GCM32EC71H106KA03: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GRM188R71C105KA12` | GRM188R71C105KA12: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GRM32ER61C226KE20` | GRM32ER61C226KE20: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:GRM32ER71A226KE20` | GRM32ER71A226KE20: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:IHLP5050FDER8R2M01` | IHLP5050FDER8R2M01: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:MMSZ5242BT1G` | MMSZ5242BT1G: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:SF-1206F700-2` | SF-1206F700-2: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:TNPW0603_B_E` | TNPW0603_B_E: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `lifecycle_unread:UCZ1H470MCL1GS` | UCZ1H470MCL1GS: no maker lifecycle page is held; its status is read at order time from the distributor's BOM line (FEEDBACK) |
| `loss_current_gap:U11` | U11's loss is read at 679 mA and its solved load is 749 mA; the 70 mA gap adds little loss at 81 % efficiency, and refusing it needs a ruling (4300d3 F9) |
| `mcu_load_excludes:STM32H753ZITx` | Table 120's run maximum excludes I/O source currents, VDDA and VDD33USB; USB is unused, and each I/O load is a declared power.loads row |
| `offboard_declared:J8.2` | J8's EXT_SCL off-board 20 pF is declared for a 100 mm JST cable and one remote pin; a longer cable or second device is outside the bound |
| `offboard_declared:J8.3` | J8's EXT_SDA off-board 20 pF is declared for a 100 mm JST cable and one remote pin; a longer cable or second device is outside the bound |
| `pin_capacitance_declared:U5` | TDK prints C_I < 10 pF typical only for the ICM-42688-P; U5's pins are counted at 10 pF, and IMU_SCK keeps 8.5 pF margin to its bound |
| `pin_capacitance_declared:U7` | Bosch prints no pin capacitance for the BMP581; U7 is counted at UM10204's 10 pF limit for an I2C pin |
| `plane_not_joined:U10.In2.Cu` | In2.Cu is the VDD_3V3 plane, which U10's GND pad cannot join; the screen counts only F.Cu, In1.Cu and B.Cu |
| `plane_not_joined:U11.In2.Cu` | In2.Cu is the VDD_3V3 plane, which U11's GND pad cannot join; the screen counts only F.Cu, In1.Cu and B.Cu |
| `power_in_rating_underated:J4` | J4 is rated 15 A by AMASS V1.2, with no derating held for 85 C; the branch draws 1.65 A, and no motor current crosses the board |
| `rated_only:U10.cboot` | TI's row states no effective minimum for U10's 0.1 uF boot capacitor; GCM188R71H104KA57 at 50 V rating sees 5 V across it |
| `rated_only:U10.cin_hf` | TI's row states no effective minimum for U10's 0.22 uF HF input capacitor; its bulk cin pair carries the 10 uF effective minimum |
| `rated_only:U10.cvcc` | TI's row states no effective minimum for U10's 1 uF VCC capacitor; GRM188R71C105KA12 at 16 V rating sees VCC, nominally 5 V and above 5.5 V in some conditions (SNVSAN3F pp. 6, 23) |
| `rated_only:U11.cboot` | TI's row states no effective minimum for U11's 0.1 uF boot capacitor; GCM188R71H104KA57 at 50 V rating sees 5 V across it |
| `rated_only:U11.cin_hf` | TI's row states no effective minimum for U11's 0.22 uF HF input capacitor; its bulk cin pair carries the 10 uF effective minimum |
| `rated_only:U11.cvcc` | TI's row states no effective minimum for U11's 1 uF VCC capacitor; GRM188R71C105KA12 at 16 V rating sees VCC, nominally 5 V and above 5.5 V in some conditions (SNVSAN3F pp. 6, 23) |
| `regulation_not_production_tested:U10` | TI states VOUT regulation as a design limit, not production tested; U10's 5 V feeds only the DS-009 ports J6 and J7, off the MCU's rail |
| `regulation_not_production_tested:U11` | TI states VOUT regulation as a design limit, not production tested; VDD_3V3's floor is held by its feedback, its drop limit and U12 |
| `reservoir_cadc_typical:VBAT_SENSE` | CADC is typical 4 pF; C50's 100 nF is 3.05 times the 32.8 nF the single-conversion reservoir rule needs (AN2834 p. 45-46, ruled D-B3) |
| `stackup_condition:copper-max-2oz` | OSH Park states 1 oz outer and 1/2 oz inner copper, nominal; 70 um (2 oz) is the engine's stated upper bound for every layer |
| `stackup_condition:copper-temperature-max` | no fabricator page states the copper's temperature limit; 125 C is the declared ceiling the supply gate's resistivity uses |
| `stackup_condition:outer-copper-min` | OSH Park states 1 oz outer copper, nominal (35 um); 30 um is the declared minimum the supply gate's resistance uses |
| `stackup_condition:permittivity-fr4-max` | IPC-4101's FR-4 maximum is not held; OSH Park's FR408HR is 3.61 nominal, and 5.4 is the declared upper bound for the capacitance bound |
| `stackup_condition:plane-copper-min` | OSH Park states 1/2 oz inner copper, nominal (17.5 um); 15 um is the declared minimum for the GND and VDD_3V3 planes |
| `stackup_condition:plating-min` | OSH Park states via plating 1 mil, nominal (25 um); 18 um is the declared minimum the supply gate's via barrels use |
| `stackup_condition:track-width-tolerance` | OSH Park publishes no track-width tolerance; 20 % is declared for the supply gate's tracks, as JLCPCB states for its own process |
| `supervisor_delay_unstated:U12` | TI states U12's 120-280 ms delay only from 3.19 V, above the rail's 3.034 V worst case; below it the delay is unstated, not absent; a bench row times the release |
| `supervisor_hysteresis_typical:U12` | TLV803S's 40 mV hysteresis is typical only; its release at 3.03 V is 4 mV under VDD_3V3's 3034 mV floor at its pin (ruled D6); a bench row |
| `theta_ja_copper_mismatch:U10` | TI's theta-JA curve is for 2 oz/1 oz copper and the board declares 1 oz/0.5 oz, so U10's 117.8 C screen has less margin than it shows; a bench thermal row (Q2 options stand) |
| `theta_ja_copper_mismatch:U11` | TI's theta-JA curve is for 2 oz/1 oz copper and the board declares 1 oz/0.5 oz, so U11's 104.8 C screen has less margin than it shows; a bench thermal row |
| `usb_pair_undeclared:PA11` | PA11/PA12 are reserved for USB (AET-WO-3400 F5) with no connector drawn on Sentinel-03, so no differential pair exists to judge |
| `vref_is_vdda:VBAT_SENSE` | Sentinel-03 has no separate VREF+; VDDA is VDD_3V3, and the sense's full scale is judged against VDD_3V3's minimum |
| `zener_below_test_current:D1` | D1's VZ is stated at 20 mA; through R60 it carries about 60 uA at 17.4 V, where VZ is lower, so Q1's gate stays under 12.6 V of its 20 V |

### Ledger entries

| register | class | fields | reason |
|---|---|---|---|
| `PWR.CSR1` | silicon-only | 1 | Polled by the bring-up before VOS is raised: the twin's PWR model reads it 1 always (AET-WO-4240v D-3), so no wait is witnessed; bench row B-2 reads PWR_CSR1 after boot. |
| `PWR.D3CR` | silicon-only | 1 | Polled after VOS Scale 2 is written: the twin's PWR model reads it 1 always (AET-WO-4240v D-3), so no wait is witnessed; bench row B-2 reads PWR_D3CR after boot. |
| `FLASH.OPTSR_CUR` | silicon-only | 1 | No model: the twin presents level 3 itself (4230 FEEDBACK section 2 m1). FAB-NOTES programs it by read-modify-write of OPTSR as read; the bench row reads bits 3:2 back as 11. |
| `GPIOA.AFRH` | same-source | 3 | PA8 at AF1 for TIM1_CH1 (MOTOR1), PA9/PA10 at AF7 for USART1: Renode routes PA8 by AFR (4300 FEEDBACK section 0) but the table cannot see its role; USART has no pin. Bench reads AFRH. |
| `GPIOA.AFRL` | same-source | 5 | PA2/PA3 at AF7 for USART2 (CRSF), PA5-PA7 at AF5 for SPI1 (flash): Renode's USART has no pin (4300b FEEDBACK section 0); AFR routes only a timer channel. The bench row reads AFRL. |
| `GPIOA.MODER` | same-source | 9 | PA0 output (FLASH_CS); PA2/PA3, PA5-PA10 alternate (USART2, SPI1, MOTOR1, USART1): BSRR drives a pin whatever MODER says (4200 FEEDBACK); only PA8's TIM1 route reads it. Bench reads MODER. |
| `GPIOA.OSPEEDR` | same-source | 9 | PA5/PA7 medium for SPI1 at 16 MHz; PA0, PA2/PA3, PA6, PA8-PA10 low: no edge rate is modelled, and check speeds judges each fmax with svd-hal's own table, not an ir rule. Bench reads OSPEEDR. |
| `GPIOA.OTYPER` | ignored-by-model | 16 | OTYPER is not stored by Renode (4300b FEEDBACK section 10, measured): written whole, push-pull on every port A pin. The bench row reads it back, as 3302 B2 asked for GPIOB. |
| `GPIOA.PUPDR` | same-source | 15 | Written whole: no pull on PA0-PA7, PA9/PA10, nor PA11/PA12 (kept for USB); debug pulls kept (PA13 up, PA14 down, PA15 up); PA8's is the motor rule's. No pull is modelled. Bench reads PUPDR. |
| `GPIOB.AFRL` | same-source | 2 | PB6/PB7 at AF4 for I2C1: the model routes a pin by AFR only for a timer channel (4300 FEEDBACK section 0). The bench row reads AFRL back; 3302 B2 predicted 0x44 there. |
| `GPIOB.MODER` | same-source | 8 | PB6/PB7, PB10/PB11 alternate for I2C1/I2C2, PB13-PB15 for SPI2, PB12 output (IMU_CS): BSRR drives a pin whatever MODER says (4200 FEEDBACK); no timer pin here. Bench reads MODER. |
| `GPIOB.OSPEEDR` | same-source | 8 | All low: PB6/PB7 I2C1 at 400 kHz, PB10/PB11 I2C2 at 100 kHz, PB12-PB15 SPI2 at 2 MHz. No edge rate is modelled; check speeds judges fmax with svd-hal's own table. Bench reads OSPEEDR. |
| `GPIOB.OTYPER` | ignored-by-model | 16 | OTYPER is not stored by Renode (4300b FEEDBACK section 10): PB6/PB7 (I2C1) and PB10/PB11 (I2C2, EXT) open-drain, the rest push-pull. 3302 B2 reads it on silicon; the bench row compares it. |
| `GPIOB.PUPDR` | same-source | 16 | No pull is modelled. Written whole by split: no internal pull on PB6/PB7 (the bus has 2.2 kOhm), PB4's reset pull-up kept. The bench row reads PUPDR back. |
| `I2C1.CR1` | same-source | 1 | PE is set by the enable, not by silicon::i2c_cr1 (whose word has PE clear); stored, with no reaction claimed. The bench row reads PE as 1; the bus records on the bench need it. |
| `I2C1.CR2` | same-source | 7 | 7-bit addressing, no reload, no PEC, written 0 (AUTOEND set at run time): STM32F7_I2C.cs is not held, no reaction claimed. The bench row reads CR2 back, all but AUTOEND. |
| `I2C1.ICR` | same-source | 9 | Write-1-to-clear (RM0433 Rev 9 section 47.9.8, p. 2015), so no bench row: STOPCF and NACKCF after each transfer. The next record's OK status on the bench (3302 B3) shows the clear took. |
| `RCC.CR` | ignored-by-model | 13 | D2CKRDY: not polled (Q-F), a tag reading 0 (STM32H7_RCC.cs L175). The rest: tags or read-only flags of the RCC_CR words the HSE bring-up writes. Bench rows: HSIKERON, CSIKERON, HSEBYP, D2CKRDY. |
| `RCC.CR` | silicon-only | 3 | HSE bring-up waits on HSERDY (ABM8AIG 16 MHz, 20 ms ruled) and PLL1RDY; Renode sets each with its ON bit (L185, L197). CSSHSEON is set after HSERDY: stored, no detection. Bench reads CR. |
| `RCC.D1CCIPR` | ignored-by-model | 1 | A tag in Renode (STM32H7_RCC.cs L401, reads 0 whatever is written, 4230 FEEDBACK section 2). SPI1 and SPI2 run on pll1_q here (the topology), not per_ck. The bench row reads it. |
| `RCC.D2CCIP1R` | ignored-by-model | 2 | ResetClock reads them at reset before the select write; a tag (STM32H7_RCC.cs L408-410) reads 0 whatever silicon holds. The written SPI123SEL is judged by spi_words. |
| `RCC.D2CCIP2R` | ignored-by-model | 3 | Left at reset (rcc_pclk2/pclk1 kernels the BRR and TIMINGR rules assume); a tag (STM32H7_RCC.cs L420-425) reads 0 whatever silicon holds. The bench row reads each. |
| `SCB.CPACR` | same-source | 2 | The FPU enable in the generated Reset, outside every Kani harness; whether Renode faults a float with CPACR clear is not held. The bench row reads CP10/CP11 as full access. |
| `SPI1.CR2` | ignored-by-model | 1 | A tag (STM32H7_SPI.cs L149), written 0 with TSIZE: no reload is used. The bench row reads it back as 0. |
| `SPI1.IFCR` | ignored-by-model | 7 | Tags in Renode (STM32H7_SPI.cs L258-266), written 0 beside EOTC; write-only, so no bench row. The boot record's flash identity, read on silicon, shows each transfer ended. |
| `SPI1.SR` | ignored-by-model | 1 | MODF is a tag (STM32H7_SPI.cs L238) and MASTER always reads 1: no mode fault in the twin. SSM\|SSI before SPE is proved by Kani; the bench row reads MODF as 0 (3302 saw it 1). |
| `USART1.CR1` | ignored-by-model | 13 | Not stored by Renode's USART (4300b FEEDBACK section 0; M0, M1 measured P4.1): console USART1 on PA9/PA10, 8 data bits, FIFOEN set, no wake-up, interrupts off. The bench row reads CR1 back. |
| `USART1.CR1` | same-source | 8 | Stored with no reaction claimed (4300b FEEDBACK section 0): no parity, oversampling by 16, interrupts off, compared with the topology only. The bench row reads CR1 back. |
| `USART1.CR2` | ignored-by-model | 19 | STOP is the only CR2 field Renode stores (4300b FEEDBACK section 0; RXINV, TXINV measured P4.1): asynchronous, no swap, no inversion, LSB first. The bench row reads CR2 back. |
| `USART1.CR2` | same-source | 1 | One stop bit: stored, no reaction claimed (4300b FEEDBACK section 0), compared with the topology only. The bench row reads CR2.STOP back as 00. |
| `GPIOB.AFRH` | same-source | 5 | PB10/PB11 at AF4 for I2C2 (EXT, J8), PB13-PB15 at AF5 for SPI2 (IMU U5): the model routes a pin by AFR only for a timer channel (4300 FEEDBACK section 0). The bench row reads AFRH. |
| `GPIOC.AFRH` | same-source | 2 | PC10/PC11 at AF7 for USART3 (MAVLINK, J7): Renode's USART has no pin (4300b FEEDBACK section 0), so a wrong AF still carries MAVLINK bytes in the twin. The bench row reads AFRH. |
| `GPIOC.MODER` | same-source | 2 | PC10/PC11 alternate for USART3 (MAVLINK, J7): MODER is read only on a timer route (4300 FEEDBACK section 0) and USART has no pin; the serial pin judge compares the topology. Bench reads MODER. |
| `GPIOC.OSPEEDR` | same-source | 2 | PC10/PC11 low for USART3 at 57600 baud (MAVLINK): no edge rate is modelled; check speeds judges the fmax with svd-hal's own table, not an ir rule. The bench row reads OSPEEDR back. |
| `GPIOC.OTYPER` | ignored-by-model | 16 | OTYPER is not stored by Renode (4300b FEEDBACK section 10, measured): written whole, push-pull on every port C pin, USART3's PC10/PC11 included. The bench row reads it back. |
| `GPIOC.PUPDR` | same-source | 16 | Written whole as 0: no pull on PC10/PC11 (USART3, MAVLINK) nor PC0 (VBAT_SENSE, ADC1 INP10), every other port C pin at reset. No pull is modelled. The bench row reads PUPDR. |
| `GPIOE.AFRH` | same-source | 3 | PE11/PE13/PE14 at AF1 for TIM1_CH2-4 (MOTOR2-4): Renode forwards TIM1 only while AFR picks AF1 (4300 FEEDBACK section 0), but the table cannot see a pin's role. Bench reads AFRH. |
| `GPIOE.MODER` | same-source | 3 | PE11/PE13/PE14 alternate for MOTOR2-4: Renode forwards TIM1's level only while MODER is AF (4300 FEEDBACK section 0); the table cannot see a pin's role, so the weaker class. Bench reads MODER. |
| `GPIOE.OSPEEDR` | same-source | 3 | PE11/PE13/PE14 low for MOTOR2-4 DShot600: no edge rate is modelled; check speeds judges that speed's fmax with svd-hal's own table, not an ir rule. The bench row reads OSPEEDR. |
| `GPIOE.OTYPER` | ignored-by-model | 16 | OTYPER is not stored by Renode (4300b FEEDBACK section 10, measured): written whole, push-pull on every port E pin, MOTOR2-4 on PE11/PE13/PE14 included. Bench row reads it back. |
| `GPIOE.PUPDR` | same-source | 13 | Written whole as 0 on port E but MOTOR2-4 (PE11/13/14, the motor rule's); PE9, MOTOR1 until 4300h D3 moved it to PA8, has no pull now. No pull is modelled. Bench reads PUPDR. |
| `I2C2.CR1` | same-source | 1 | I2C2 runs EXT (PB10/PB11, J8) at 100 kHz: PE is set by the enable, not by silicon::i2c_cr1 (whose word has PE clear); stored, no reaction claimed. The bench row reads PE as 1. |
| `I2C2.CR2` | same-source | 7 | 7-bit, no reload, no PEC: 0 in I2C2's transfer code. STM32F7_I2C.cs is not held, no reaction claimed; EXT (J8) declares no device, so none runs. Bench reads all but AUTOEND. |
| `I2C2.ICR` | same-source | 9 | Write-1-to-clear (RM0433 Rev 9 section 47.9.8): STOPCF\|NACKCF in I2C2's transfer, held by its Kani harness only; no bench row, and EXT (J8) has no device, so no record shows a clear. |
| `RCC.AHB4ENR` | same-source | 2 | GPIOC (USART3, VBAT_SENSE) and GPIOE (MOTOR2-4) clocks: 3302 B2 read Renode's AHB4ENR as 0x3, ports A/B; no held measurement shows ports C/E registered with its RCC. Bench reads AHB4ENR. |
| `RCC.APB1LENR` | same-source | 1 | I2C2's clock for EXT (J8): 3302 B2 read Renode's APB1LENR as 0x00200000, I2C1 only; no held measurement shows i2c2 registered with its RCC, so the gate goes unseen. Bench reads I2C2EN. |
| `RCC.CFGR` | ignored-by-model | 9 | Written whole with SW = PLL1 by the 16 MHz HSE bring-up: SWS read-only, the rest tags (STM32H7_RCC.cs L226-236). MCO1/MCO2 0; PC9 (MCO2) is not set AF. Bench rows read all but SWS. |
| `SPI2.CR2` | ignored-by-model | 1 | SPI2 to U5 (ICM-42688-P) on PB13-PB15: TSER is a tag (STM32H7_SPI.cs L149), written 0 with TSIZE, as no IMU transfer reloads. The bench row reads it back as 0. |
| `SPI2.IFCR` | ignored-by-model | 7 | Tags in Renode (STM32H7_SPI.cs L258-266), written 0 beside EOTC after each U5 transfer; write-only, so no bench row. U5's identity read (0x47 expected), on silicon, shows the transfers ended. |
| `SPI2.SR` | ignored-by-model | 1 | SPI2 masters U5 with PB12 a GPIO select: MODF is a tag (STM32H7_SPI.cs L238) and MASTER reads 1, so no mode fault shows in the twin. Kani proves SSM\|SSI before SPE; bench reads MODF 0. |
| `TIM1.BDTR` | ignored-by-model | 12 | Written as MOE alone (4240b FEEDBACK): no break or dead time on MOTOR1-4. Every BDTR field is a tag (4300a FEEDBACK section 0); toggling MOE moves no observer (2302 A2-4). Bench reads BDTR. |
| `TIM1.CCER` | ignored-by-model | 3 | CC1NE-CC3NE 0: no complementary output. TIM1's CH1N pins include PA7 (SPI_MOSI) and PB13 (IMU_SCK), here at AF5. Tags in Renode (4300a FEEDBACK section 0). Bench reads CCER. |
| `TIM1.CCER` | same-source | 16 | CC1E-CC4E set for MOTOR1-4, CC5/CC6 off, every polarity 0: stored with no reaction claimed; Renode's output path never reads CCxP (4300 FEEDBACK section 0). The bench row reads CCER. |
| `TIM1.CCMR1` | ignored-by-model | 8 | MOTOR1/MOTOR2 (PA8, PE11): OC1PE/OC2PE set, OCxFE/OCxCE 0, all tags; OCxM[3] is reserved in the model (4300a FEEDBACK section 0); the twin lists OCxPE unobservable. Bench reads CCMR1. |
| `TIM1.CCMR1` | same-source | 2 | CC1S/CC2S 00 make channels 1-2 outputs for MOTOR1 (PA8) and MOTOR2 (PE11): stored (STM32_Timer.cs L425-445), no reaction claimed; the image compares the topology. Bench reads CCMR1. |
| `TIM1.CCMR2` | ignored-by-model | 8 | MOTOR3/MOTOR4 (PE13, PE14): OC3PE/OC4PE set, OCxFE/OCxCE 0, all tags; OCxM[3] is reserved in the model (4300a FEEDBACK section 0); the twin lists OCxPE unobservable. Bench reads CCMR2. |
| `TIM1.CCMR2` | same-source | 2 | CC3S/CC4S 00 make channels 3-4 outputs for MOTOR3 (PE13) and MOTOR4 (PE14): stored (STM32_Timer.cs L425-445), no reaction claimed; the image compares the topology. Bench reads CCMR2. |
| `TIM1.CR1` | ignored-by-model | 2 | TIM1 for MOTOR1-4 DShot600: CKD and UIFREMAP written 0, outside the CR1 bits 0-7 Renode stores (CKD a tag; 4300a FEEDBACK section 0), so the twin cannot show them. Bench reads CR1. |
| `TIM1.CR1` | same-source | 5 | ARPE set, edge-aligned up-count (CMS 0, DIR 0), UDIS/URS 0 for MOTOR1-4's DShot600 frames: stored (bits 0-7) with no reaction claimed; the image compares the topology. Bench reads CR1. |
| `TIM1.EGR` | same-source | 8 | EGR carries UG alone for TIM1 (MOTOR1-4): CC1G-CC4G, COMG, TG, BG, B2G written 0, raising no event. Write-only, so no bench row; no held source line shows Renode's handling of them. |
| `USART2.CR1` | ignored-by-model | 13 | CRSF on USART2 (PA2/PA3, J6) at 420000: 8 data bits (M0/M1), FIFOEN set, no wake-up or mute, no DE timing, those interrupts off; none stored by Renode (4300b FEEDBACK section 0). Bench reads CR1. |
| `USART2.CR1` | same-source | 8 | CRSF is 8N1 (validate refuses other framing): PCE/PS 0, OVER8 0 (the column its tolerance check takes), interrupts off; stored, no reaction claimed (4300b FEEDBACK section 0). Bench reads CR1. |
| `USART2.CR2` | ignored-by-model | 19 | CRSF must be non-inverted 8N1 (4300b FEEDBACK section 0): RXINV/TXINV/SWAP are tags (P4.1, measured), so a wrong inversion on PA2/PA3 passes the twin. Only STOP is stored. Bench reads CR2. |
| `USART2.CR2` | same-source | 1 | One stop bit for CRSF 8N1 at 420000 on USART2: stored, no reaction claimed (4300b FEEDBACK section 0), compared with the topology only. The bench row reads CR2.STOP back as 00. |
| `USART3.CR1` | ignored-by-model | 13 | MAVLINK on USART3 (PC10/PC11, J7) at 57600: 8 data bits (M0/M1), FIFOEN set, no wake-up or mute, no DE timing, those interrupts off; none stored by Renode (4300b FEEDBACK section 0). Bench reads CR1. |
| `USART3.CR1` | same-source | 8 | MAVLINK 8N1 at 57600, BRR 1111: PCE/PS 0, OVER8 0 (oversampling by 16), interrupts off; stored with no reaction claimed (4300b FEEDBACK section 0), compared with the topology. Bench reads CR1. |
| `USART3.CR2` | ignored-by-model | 19 | MAVLINK on PC10/PC11: asynchronous, no swap, no inversion, LSB first, all tags or unstored (4300b FEEDBACK section 0; RXINV/TXINV measured P4.1); only STOP is stored. Bench reads CR2. |
| `USART3.CR2` | same-source | 1 | One stop bit for MAVLINK 8N1 at 57600 on USART3 (J7): stored, no reaction claimed (4300b FEEDBACK section 0), compared with the topology only. The bench row reads CR2.STOP back as 00. |
