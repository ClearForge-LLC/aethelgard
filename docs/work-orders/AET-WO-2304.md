# AET-WO-2304 — Spike: the battery under the motors, measured: a declared current profile through the power tree, or per-quantum lock-step, and the smallest honest step that lets a board with a power tree run its campaign and get a fabrication package

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`cd43b207` when written; verify live HEAD).
**Branch:** `wo/aet-2304-battery-sag-spike`.
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4 (roadmap §7.1, every row owned by "the battery-sag work order"). Written 2026-11-19. **Cadence: spike, ending in a stop.** The model is the architect's to rule. Anything that weakens a gate's meaning is Scotty's.

**Grounds:**
- **The hard block.** `campaign` refuses any specification with a power tree, `PowerNotSimulated`.
  - The refusal is in `crates/aethelgard-cli/src/campaign.rs` L377–390, inside `effective` (L372). It is not in `aethelgard-chaos`. Its test is `crates/aethelgard-cli/tests/campaign_tier1.rs` L2707–2729.
  - `verify --fab-out` requires `--scenarios` (`crates/aethelgard-cli/src/main.rs` L124–128, `INV-03`).
  - So neither Sentinel-03 (`crates/aethelgard-cli/tests/fixtures/sentinel-03.json`) nor the Sentinel-02 power fixture (`crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json`) can get a package. `4300d2` D5 (c) accepted `fab` proved below `verify` until this work order (roadmap §7.1 L707).
- **Lifting the refusal is not enough.** The coverage rule adds `rail \`battery\` (no fault kind droops it)` whenever a tree is declared. It also marks every rail other than the MCU's supply unmet (`crates/aethelgard-cli/src/verify.rs` L930–944; `3303` ruling (b)). Sentinel-03 also declares `VDD_5V`, and the power fixture declares `VDD_5V` and `VDD_1V8`.
- **Sentinel-03 comes first.** It carries the power tree, the flight sensors and four motor outputs (PE9–PE14, `dshot600`). It is built before Sentinel-02 (roadmap L367, L375). Scotty's fabrication quote waits on it (L419–424).
- **`2303`'s ruling** (#200, comment 6943762720; architecture §2.26 L439–453, `OQ-03` L678):
  - **D1:** (ii) for `4500`, a declared current profile through `2301`'s droop machinery. (i), lock-step, belongs to this work order. Lock-step waits for a firmware whose commands move the load, and for a declared ESC supply current.
  - **D2:** not chosen. The architect leans to (b) 100 µs plus (a1) per-quantum samples under a trace version bump, with the writer enforcing the bound. (c) is sampling, and Scotty's alone.
  - **D4:** the ESC is a power-parts row, with idle and per-throttle supply current, named by each motor output. Absent means halt.
  - **D5:** under lock-step only, the DShot stop moves to the run's end.
- **The generated firmware commands no throttle.** DShot is "a continuous zero-throttle pattern, with no DMA" (`crates/aethelgard-svd-hal/README.md` L64). A DShot 0 frame is a disarmed motor. So under the generated firmware, the only load the firmware moves is an ESC's idle draw while `CEN` is set. `2303`'s 10 A was spike-declared (F9).
- **The static chain already exists in `aethelgard-ir`:**
  - `battery_range` (`power.rs` L609);
  - `battery_inputs` (L1305);
  - `headroom` (L1448), with a buck's `D ≤ 1 − tOFF-MIN·fSW` and its RDS(on) and DCR drops;
  - the LDO's `dropout_max_millivolts` (`power_parts.rs` L200–202).

  It is a rule that shares no code with the generator (`RUL-20` (1)). The battery has no internal-resistance field (`power.rs` L200–207).
- **The droop path today:**
  - a trapezoid on the MCU's supply only (`DroopNotMcuSupply`, `cli/src/campaign.rs` L285);
  - the scenario declares its source R and load (`chaos/src/record.rs` L199–216);
  - it is solved once by `aethelgard-bridge --rail` (`bridge/src/rail.rs` L17–30; `cli/src/analog.rs` L280);
  - its decoupling is the emitter's `decouples` parts on the rail (`cli/src/campaign.rs` L305–308; `eda/src/schematic.rs` L1118–1147).

> **What this is:**
> - every row this work order owns, classified at source;
> - the battery chain under the motors, measured on Sentinel-03 under two candidate models, with sourced loads;
> - each candidate's full engine cost;
> - the smallest honest step to a `verify --fab-out` package for a board with a power tree, put to the architect as D-decisions.
>
> It is NOT:
> - the build. Every prototype lives in `spike/2304/` and in scratch trees outside the repository;
> - a change to any crate, `bridge/`, model, golden, test, workflow or `ci/`;
> - the choice of Sentinel-02's ESC, frame or pack. Those are `4500`'s and Scotty's;
> - a sampling proposal. Option (c) is named only (`INV-04`);
> - ArduPilot in the campaign, or the flight loop (roadmap L429).

## 1. Spike

Every figure is labelled **measured**, **read at source** or **inferred**. As in `2303`, **commit the predictions before any measured run.** A run added later is reported as unpredicted.

1. **The rows, at source.** Classify each row as **dynamic** (it needs the chain or lock-step), **static** (a rule or a data field), or **done or stale**. Cite the code for each. The architect's reading is below. Confirm or correct it.

   | Row | Item | Architect's reading |
   |---|---|---|
   | L608 | The chain: battery, buck/LDO dropout, VDD, into `--rail` | dynamic: the question of §1.5 |
   | L695 | A covering tolerated fault moves its quantity past the declared tolerance | static: a coverage rule in `verify.rs` `stress` (L705–) |
   | L696 | A serial fault kind | **done** by `2302` D5: `serial_rx_open` covers a port (`verify.rs` L878–880; `crates/aethelgard-cli/README.md` L924). The row looks stale |
   | L697 | The droop's band check against the loaded floor | static. The site is `verify.rs` L785–796, which compares the declared `floor_millivolts`. The scheduler's `rail_levels` already subtracts I·R (`chaos/src/campaign.rs` L431–440) |
   | L707 | The power fixture's full package | the goal |
   | L722 | J2's current rating against the battery sag | static, and the wording looks wrong. `4300g` K7 is about Sentinel-01's J2 (JST GH) against the MCU's draw. Sentinel-03's battery input is J4, `XT30PW-M`, rated 15 A (`ir/src/connectors.rs` L165–168). It matters only if motor current flows through J4 (§1.2) |
   | L723 | A capacitor's dielectric and ESR | static. A ceramic row has a dielectric but no ESR (`power_parts.rs` L302–306). Only `BulkCapacitor` has ESR (L330–336). The gap is ESR on ceramics and support capacitors (`4300g` K8: VCAP below 100 mΩ) |
   | L799 | The writer passes the reader's 256 MiB | static. The next chaos order owns it, and any dynamic trace needs it first |
   | L800 | The ESC's supply current as a power-parts row | data (§1.3) |
   | L801, L802 | The DShot stop at the run's end; lock-step's costs | only under candidate (b) |
   | L804 | A declared profile for `4500`; the lean to (b)+(a1) | the question of §1.5 |

2. **Where the motor current flows, at source.**
   - Does it pass through the board's battery input (J4) and its `VBAT` copper? Or does it share only the pack and its leads, with the ESC fed from the pack directly?
   - Read Sentinel-03's specification and emitted netlist, and the power fixture's. If the specification cannot say, that is a finding: the chain's R depends on it, and so do L722 and the `1303b` supply gate.
   - **Sentinel-03's use.** Read `3400`/`3401` and the roadmap. Will Sentinel-03 drive ESCs at all, on the bench or in the air? If it never will, say so: its campaign's battery load is then the tree's static load.
3. **Sourced numbers, each with document, revision, page and sha256.** Hold them as `aet-ds` holds datasheets. Never in the repository.
   - **ESC supply current.** Take at least two ESCs that fit a ducted 3″ frame on a 3S–4S pack (4-in-1, BLHeli_S/BLHeli_32/AM32 class). Record idle (disarmed: DShot 0, and no signal) and per-throttle current, if stated.
   - **Pack resistance.** A 3S and a 4S LiPo of 450–850 mAh. Record the internal resistance as the maker states it (DC or 1 kHz AC, and at what state of charge).
   - **Connector resistance,** if AMASS states one for the XT30PW-M.
   - **If a maker does not state a figure, say so.** Do not estimate one (`INV-06`). Run §1.4 as a sweep over that quantity instead, labelled as a sweep.
4. **The chain, statically.** Use a spike script with no Renode. It must call or mirror IR's `headroom` arithmetic, and say which.
   - **Boards:** Sentinel-03 (U10 and U11 from the battery) and the power fixture (adding U12, the LDO from `VDD_3V3`).
   - **Load cases:** for each sourced or swept ESC current (idle × 4; full throttle × 4 as an upper bound labelled "what an armed firmware could draw") and pack R, compute:
     - V_bat at `cells.min × cell_min_millivolts`, less I·R;
     - each regulator's output;
     - the margin to `VBOR3`'s falling band (2.54–2.68 V, `chaos/src/power.rs` L51–72);
     - the margin to each buck's `regulation_vin_min`.
   - **The answer:** the pack-plus-path R at which `VDD_3V3` first enters BOR3's band, at idle current and at full throttle. Do the same for `VDD_5V` leaving its tolerance.
   - **This is the decisive measurement.** If, at the generated firmware's idle draw, no credible R moves `VDD_3V3` within a stated margin of BOR3, then the load's dependence on the firmware cannot change a campaign verdict on the generated firmware. Say what margin you used and why.
5. **The two candidates, run.** One board (Sentinel-03, generated and built by the unmodified CLI) and one fault (a high pack R, the "weak" case).
   - **(a) Declared chain.** A current profile is declared in the scenario: steps at stated times, with magnitudes from §1.3. It is solved once before the boot through battery, regulator and `VDD_3V3`, and its crossings are scheduled by `2301`'s rule (`aethelgard_chaos::power::schedule`).
     - Prototype the deck under `spike/2304/`, with `2303`'s FFI copy outside the workspace.
     - Record what `bridge --rail`'s request would have to carry beyond a trapezoid (`bridge/src/rail.rs` L17–30).
   - **(b) Lock-step.** Use `2303`'s probe (`spike/2303/lockstep`, `spike/2303/ngspice`), restored under `spike/2304/`, with the same deck.
     - The load is ESC idle × 4 while TIM1 `CEN` is set. Run at 100 µs and at 10 µs.
     - Also run `2303`'s spike-declared 10 A once, on Sentinel-03, for continuity with §2.26.
   - **Measure, for each case:**
     - crossings, resets and their times;
     - whether a reboot loop occurs;
     - archive bytes and records per run, with the real `TraceWriter` (stand-in kinds as in `2303`, labelled);
     - wall time;
     - peak RSS of Renode and of the probe;
     - whether `replay_dir` reproduces the hash.
   - **(a) against (b):** the difference in each crossing's time and the reset count, at sourced loads and at 10 A. That is the error (a) carries, in the units `2303` used (late by, deeper by, count).
6. **The engine's cost, per candidate.** Name file and line for each, and give versions as numbers after `4400g` (§1.9).

   | Candidate | What it is |
   |---|---|
   | **(a0)** | Lift `PowerNotSimulated` only. `2301`'s MCU-supply droop runs on a tree board. No new fault kind |
   | **(a)** | The declared chain as a new fault kind on the battery |
   | **(b)** | Lock-step at 100 µs with (a1) per-quantum samples |
   | **(d)** | Re-plan at each firmware command (`2303` §1.3), named. Measure it only if it is cheap |

   For each, the cost of:
   - the specification schema: a battery and path R field, and motor outputs naming an ESC;
   - the power-parts schema: an `esc` kind;
   - the scenario version (`SCENARIO_VERSIONS`, `chaos/src/campaign.rs` L32);
   - the trace version (`TRACE_VERSION`, `chaos/src/event.rs` L43) and new record kinds;
   - the reader's rules (`check_archived`, `check_joins`), `MAX_RECORDS` (`replay.rs` L74) and `MAX_TRACE_BYTES` (`archive.rs` L26, L329; L799);
   - crates touched: `ir`, `chaos`, `cli`, `eda`, `bridge/`;
   - goldens and committed archives that move. A trace bump moved the bench package's `verdict_sha256` at `1605`/`1605c`;
   - tier-2 tests and CI minutes.

   For (a), say whether (a0) is a strict subset of it.
7. **Coverage, per candidate.** Using `verify.rs` L882–946 as it stands, say which rows a Sentinel-03 campaign would meet, and which stay unmet: `battery`, `VDD_5V`, and on the fixture `VDD_1V8`.
   - A rail the twin has no observer for is the hard case: nothing in Renode runs on `VDD_5V`. Name the honest options.
   - Each option that relaxes the rule is a D for the architect, or for Scotty if it changes what `INV-03` counts. Never a workaround.
8. **The decoupling.** Say whether `decoupling_picofarads` counts U11's output capacitors on `VDD_3V3` for Sentinel-03 (read at source, then the number measured). Then give what `2301`'s droop computes with and without them. A tree board's rail model is wrong if they are left out.
9. **What `4400g` moves.** It is open as #238 and touches `aethelgard-twin`, `aethelgard-chaos` (trace 13 → 14, replay, campaign) and `aethelgard-ir` (specification 16 → 17, `mcu.foreign_clock`).
   - **Measure on `main`,** at the SHA the kickoff names.
   - For each figure, say whether `4400g` would move it.
   - **The architect expects these to move:**
     - TIM1 at 64 MHz instead of 250 MHz, with periods realized in whole ns. So Renode's DShot cost per ms, and so (b)'s wall per simulated second;
     - kind 26's bytes;
     - every version number in the cost table (the build's bump becomes trace 14 → 15 and specification 17 → 18);
     - the files the build would share with `4400g` (`cli/src/campaign.rs`, `chaos/src/campaign.rs`).
   - **The architect expects these not to move** (inferred; confirm):
     - the boot's time to `CEN`, which depends on instruction counts (no DWT read in `aethelgard-svd-hal` or `aethelgard-twin`);
     - the analog arithmetic;
     - ngspice's memory.
   - **If `4400g` merges mid-spike,** do not rebase measured runs. Re-measure only the figures named as moving, on the new `main`, and give both.
10. **`verify`'s weight on a tree board.**
    - Run `aethelgard verify` on Sentinel-03 (the gates it runs today, no `--fab-out`) once, under `capped-run --mem 16G`. Record its peak RSS and wall time.
    - The build's tier-2 cost rests on this. `verify` can need more than 8 GiB.

## 2. Stop

**STOP** with FEEDBACK holding:
- §1.1's table, confirmed or corrected;
- §1.2's flow and Sentinel-03's use;
- §1.3's sourced table, with any gaps named;
- §1.4's margins;
- §1.5's predictions beside its results;
- §1.6's cost table, §1.7's coverage table, §1.8, §1.9's list of movers, and §1.10;
- findings in the house schema (`finding · where · type · recommendation · decision-needed`);
- what was deliberately not built.

**Decisions to put, each with a recommendation:**
- **D1 · The model for the campaign: (a) or (b).**
  - Recommend (a) if §1.4–§1.5 show that, at sourced idle draw, the generated firmware cannot move a crossing. (b) then waits for the flight loop, where an armed ArduPilot moves the load.
  - Recommend (b) at 100 µs if the firmware can move a crossing.
  - State the margin the recommendation rests on.
- **D2 · The load's source.** `2303` D4 rules a power-parts ESC row. Recommend its shape. Say who picks the part (`4500`/Scotty), and what a campaign does before one is picked: halt by name, as `INV-06` requires.
- **D3 · Pack and path R.** For the healthy case: a specification field (`Battery`, sourced; a schema bump) or a parts row. For the fault: a scenario field (`INV-07`: the fault is authored apart).
- **D4 · Coverage for `battery` and for rails the twin cannot observe.** Say what meets each row. Mark any option that changes what "verified" counts as Scotty's.
- **D5 · The smallest honest step:** which candidate, or combination, the first build is, and its versions after `4400g`.
  - Say whether it takes L799 (the writer's bound) and L697.
  - Say whether it ships Sentinel-03's package with the chain declared and the reboot loop named as not shown (`RUL-19`'s habit: say what the verdict does not claim).
- **D6 · Row hygiene.** L696 closed as done by `2302`, L722 reworded, and whether L695, L697 and L723 go to a static work order or ride the build.
- **D7 · Precede or fold.** Does this spike's build precede `AET-WO-4500`, or fold into it? Give the evidence both ways (see the architect's note below). Recommend one.

Push an `AET-WO-2304 STOPPED:` commit and open one PR. As with `2303`, it is merged as the record (FEEDBACK plus `spike/2304/`) after the ruling. Write `~/.cache/aet-2304/pr-title-body.md`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-04`, `INV-05`, `INV-06`, `INV-07`; `RUL-05`, `RUL-06`, `RUL-12`, `RUL-16`, `RUL-17`, `RUL-19`, `RUL-20`.
- **Fence:**
  - `spike/2304/` and `docs/work-orders/AET-WO-2304.FEEDBACK.md` only.
  - Read anything. Restore `2303`'s probe by copying it, and keep its FFI outside the workspace, as `2303` did.
  - Scratch trees, traces and datasheets stay outside the repository. Quote each trace's sha256 and the script that remakes it.
  - **No change** to any crate, `bridge/`, `models/`, golden, test, `.github/` or `ci/`. `git diff --stat <base> -- crates bridge models ci .github Cargo.toml Cargo.lock` is empty at the stop.
- **Running beside you:**
  - `AET-WO-4400g` (#238: twin, chaos, ir, the CLI's campaign);
  - `AET-WO-1910` (#233) and `1911`, which own CI;
  - `AET-WO-1315` (the emitter draws the hot-plug parts on tree boards: `aethelgard-eda`). Your §1.8 reads the emitter's decoupling on main. Say if `1315` would move it;
  - `AET-WO-4240` (a spike: the HSE clock tree).

  You share no file with them.
- **Containers:**
  - One container of yours at a time, under `capped-run`, with `--network none` for runs.
  - Use `--mem 8G` for the probe, as `2303` did, and `--mem 16G` for any `verify`.
  - Read the caps back inside the scope before each run (`2303`'s `caps.log`).
- On Furnace: no `sudo`, no system packages, no credentials. Push with `gh-write`.
- One PR, left unmerged.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **A measurement needs a crate or `bridge/` change** that cannot be made under `spike/2304/`. Stop and say what it needs.
- **A run is killed, or passes 900‰ of its cap.** Stop with the numbers. A cap is never raised here.
- **No maker states an ESC's or a pack's figure.** Do not estimate one. Sweep it and label the sweep. D2/D3 then say the sourced figure is missing. This is not a stop for the whole spike.
- **The only way any candidate meets the coverage rule is to relax it.** Stop at D4. Do not prototype the relaxation as if it were ruled.
- **A candidate turns out to be sampling under `INV-04`.** Name it, and never recommend it. It is Scotty's.
- **`4400g` merges and moves a figure your recommendation rests on, beyond its stated band.** Re-measure it on the new `main` before the stop, or stop with both figures.
