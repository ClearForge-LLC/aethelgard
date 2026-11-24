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
  - **The reasons, for review:** PENDING (listed at the close from the specification).
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
- **`ViaRegionsOverlap` has no witness.** The pours re-routed the two-layer power fixture, so 4410 Q12's PC0 overlap no longer occurs, and a planted via either keeps its region apart or is refused for clearance first. This is for the router work order.
- **`fabrication_ready` is not in `verdict_sha256`** (the gate review's F2, ruled 12:45Z as a row): the verdict line is built before the `fab` report, so a package's bytes carry no mark of whether readiness was judged, skipped by a re-bind, or not judged under `--against-fab`. The CI pin test is the control.
- **`ci/colocate.tsv`:** `verify_2k2_tier2` roughly doubled (+~450 s); re-weigh it from CI's own times after the first green run, by the REFIT RULE (ruled 12:45Z).
- **The re-bind's trust root is CI** (the gate review's F2). A tool-side check that the directory is the committed one (git-tracked, under `fab/`) would make the tool itself the control.
- **`--rebind` is on the MCP `verify` tool** (the snapshot blessed): the MCP server runs the same local CLI, so it grants nothing the CLI does not; CI stays the trust root.
- **`with_tools.rs`'s LP1 comment** describes `verify_2k2_tier2` before the split; it is outside this fence.

## 7. Gates, CI, package

PENDING.
