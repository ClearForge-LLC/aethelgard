# AET-WO-2304 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-2304-battery-sag-spike`, from `main` at `cd43b207`. The work order was read from the docs branch (copy at `~/.cache/aet-2304-wo.md` on Furnace); it is not committed here. Prototypes live under `spike/2304/`; no crate, `bridge/`, model, golden, test, workflow or `ci/` changes.

Every figure is labelled **measured**, **read at source** or **inferred**. Measured results come after the predictions below, which were committed before any measured run.

## Predictions — written before any measured run

### Already done before this commit (setup and tool validation, not measurements of a predicted quantity)

- The unmodified CLI, bridge and the restored probe built in the pinned tools image under `capped-run --mem 8G` (`spike/2304/sh/img.sh`).
- `aethelgard generate` and `aethelgard build` accept Sentinel-03 (`crates/aethelgard-cli/tests/fixtures/sentinel-03.json`): ELF sha256 `5756a2e0…ca51`. `simulate` has **not** been run yet.
- **The static chain's mirror of IR (`spike/2304/chain/chain.py selfcheck`) reproduces IR exactly.**
  - IR's own numbers were read from unmodified `aethelgard validate` refusals on scratch copies of the fixture (`spike/2304/chain/headroom_probe_spec.py` makes the headroom one): `RegulatorOutsideDomain` at 1 ‰ tolerance prints the output range, `RegulatorOverloaded` prints the load, and `BuckHeadroomViolated` prints the available µV.
  - U11 3,144,727–3,417,204 µV; U10 4,818,048–5,237,403 µV; loads `VDD_3V3` 748 mA and `VDD_5V` 2,000 mA. U11's headroom at (3,801 mV, 3,000 mA), (3,801, 2,748), (3,801, 2,548) and (3,900, 3,000) is 3,130,245, 3,174,104, 3,208,914 and 3,225,375 µV. Duty is at most 960,900 ppm. All equal.
  - `headroom` is private, so the script **mirrors** it; it does not call it.
- **Read at source before predicting:** sourcing (§1.3) found no maker figure for an ESC's idle current or for a pack's internal resistance. One connector figure is stated: XT30PW-M CR 0.80 mΩ, RC 15 A. The ESC continuous ratings stated are 40 A ×4 (Hobbywing XRotor Micro 40A) and 50 A ×4 (Holybro Tekko32 BigFET). So ESC idle current and pack R run as **labelled sweeps**.

### The model, as designed

**§1.4, static (no Renode, no ngspice).**
- The pack is an ideal source at the declared floor, `cells.min × cell_min_millivolts` = 3 × 3,000 = 9,000 mV, behind one series R: pack, leads, J4 and `VBAT` copper together.
- Each battery-fed buck draws **constant power** from `VBAT`: P = V_nom × I_load / η. η is either 1 (lossless) or the fixture's own declared figure (90 % for U10 and 81 % for U11, from each `junction.loss_milliwatts` source; typical at 36 V, used as labelled).
- The ESC current is a constant current on `VBAT`.
- There is an operating point only while (V_open − R·I_esc)² ≥ 4·R·P; past that the input collapses.
- Below U11's stated minimum operating input (3.8 V) its output is unstated, so the rule counts `VDD_3V3` as in BOR3's band. Above it, in dropout, the rail is IR's `avail`.
- ESC current cases:
  - idle, total of four, **swept** 0, 0.1, 0.2, 0.4, 0.8 and 2.0 A (unsourced);
  - 10 A (`2303`'s spike-declared figure);
  - 160 A and 200 A: the stated continuous ratings ×4, labelled "what an armed firmware could ask it to carry". They are an upper bound, not a draw.
- R points: 0.8 mΩ (J4's stated CR alone), 0.01, 0.05, 0.1, 0.25, 0.5 and 1.0 Ω.

**§1.5, the deck (both candidates, the same deck).**
- `VBAT` holds C1 (47 µF, ESR 250 mΩ, both stated), plus C10 and C20 (10 µF each) and C11 and C21 (0.22 µF each), all at nominal value.
- Pack R: **weak = 1.0 Ω** (fixed here, `2303`'s fault value, for continuity); healthy = 0.05 Ω.
- U10 and U11 are constant-power sinks on `VBAT` at declared η, off below 3.8 V.
- U11 drives `VDD_3V3` through 50 mΩ, toward min(3.3 V, `avail`(V_bat)).
- `VDD_3V3` carries the declared 748 mA (smoothed to zero near 0 V). Its capacitance is 98.81 µF: the emitter's 10.81 µF plus U11's four 22 µF outputs at nominal (§1.8 explains why both).
- BOR3 is at the typical levels `aethelgard-chaos` uses, 2.61 V falling and 2.70 V rising.
- The ESC current reaches its target through a 1 ms first-order lag (`2303`'s; unsourced).
- **(b) Lock-step:** load = I_idle while TIM1 `CEN` is set, 0 otherwise (the WO's wording), at quanta of 100 µs and 10 µs. The judge's DShot stop is left out, as in `2303`.
- **(a) Declared chain:** the same deck, solved once by ngspice with no Renode. The profile steps to the case's current at **40 ms**, a scenario-declared time. It is declared before Sentinel-03's boot-to-`CEN` is measured, so it is not read from the artifact (`INV-07`). Its crossings go through `aethelgard_chaos::power::schedule(3, …)`.
- Cases, each run under both candidates:
  - **I1:** idle 0.4 A total (0.1 A each, a sweep point);
  - **I2:** idle 2.0 A total (0.5 A each, the sweep's high point);
  - **T10:** 10 A.

### Predicted

| id | Quantity | Prediction |
|---|---|---|
| P1 | Sentinel-03's own static draw on the pack at 9.0 V | about 12.5 W lossless, 14.2 W at declared η, so 1.4–1.6 A. **It is larger than any idle ESC case below 1.4 A total.** The board's 2 A `VDD_5V` port budget is the dominant term |
| P2 | What binds first for `VDD_3V3` as R rises at idle | **The constant-power collapse**, not U11's duty limit and not its 3.8 V floor. `avail` at 3.8 V and 748 mA is about 3.52 V, above BOR3. The collapse comes at V_bat = (V_open − R·I_esc)/2 ≥ 4.3 V, which is above 3.8 V |
| P3 | R at which `VDD_3V3` first enters BOR3's band, at idle 0 → 2.0 A total | lossless: about 1.6 Ω at 0 A, falling to about 1.0 Ω at 2.0 A. Declared η: about 1.4 Ω → 0.9 Ω |
| P4 | Same, at full throttle (160–200 A) | about 50–60 mΩ. At 10 A: about 0.40–0.45 Ω |
| P5 | R at which `VDD_5V` leaves its tolerance (4.75 V; `avail` drops below it at V_bat ≈ 5.3 V) | at idle, about 0.15–0.3 Ω below P3's figure, so `VDD_5V` leaves first. V_bat falls below U10's 7.0 V `regulation_vin_min` (band unstated) at about 0.9 Ω at zero ESC current |
| P6 | The decisive margin at the generated firmware's idle draw | At an idle draw ≤ 0.4 A, `VDD_3V3` enters BOR3 only for R ≥ about 1.3 Ω. That is over 1,000× J4's stated 0.8 mΩ, and about 100× the third-party figure for a healthy 850 mAh pack (~12 mΩ, not a maker figure). The firmware can move a crossing only if R lies in the band between P3 with and without the idle current: **about 0.3 Ω wide, at R ≈ 1.3–1.6 Ω** |
| P7 | (b) I1 (0.4 A, 1.0 Ω), 100 µs and 10 µs | **no reset, no reboot loop**. V_bat ≈ 6.7–7.3 V, `VDD_3V3` regulated at 3.3 V, margin to BOR3 > 600 mV |
| P8 | (b) I2 (2.0 A, 1.0 Ω) | **a reboot loop.** Past the collapse at declared η, `CEN` sets the load, `VDD_3V3` falls through BOR3, the reset clears `CEN` and the board recovers. Period = boot-to-`CEN` + a few ms |
| P9 | (b) T10 (10 A, 1.0 Ω) at 10 µs and 100 µs | a reboot loop, as `2303`. Reset 1–3 ms after `CEN`. Period: Sentinel-03's boot-to-`CEN` (**predicted 35–60 ms**; `2303`'s Sentinel-02-shaped board took 39.08 ms) plus that. Resets per second about 1/period. The 100 µs run asserts up to one quantum later, with the same count |
| P10 | (a) I1 | no crossing, no reset: equal to (b) |
| P11 | (a) I2 and T10 | one fall through BOR3 after the 40 ms step. The declared load does not stop at the reset, so V_bat stays collapsed and the reset is **held to the run's end**: one reset, no loop. (a) vs (b): first assertion differs by (40 ms − measured `CEN`) plus the lag; the count is 1 against N |
| P12 | Archive per simulated second, (b) | as `2303` for the same three per-quantum values: samples about 0.50 MB/s at 100 µs and 5.0 MB/s at 10 µs (±15 %); events under 10 KB/s |
| P13 | (a) archive | event-sized: under 1 KB per run beyond the static scenario's, as `2301` D8 |
| P14 | Wall, (b) | DShot runs from `CEN` to the end in I1, so DShot-dominated: 150–250 s per simulated second at either quantum. The loop cases cost less (`2303`: 87 s/s) |
| P15 | Peak RSS | Renode 420–520 MiB; probe under 200 MB for 1 s simulated |
| P16 | `replay_dir` | reproduces the state-sequence hash; two runs of a case are byte-identical |
| P17 | §1.8 `decoupling_picofarads` on `VDD_3V3` | **10,810,000 pF**, a hand count from source by a subagent. U11's four outputs (88 µF nominal) are not counted, so the droop's τ = R·C is about 9× too small on this rail |
| P18 | §1.10 `verify` on Sentinel-03 (no `--fab-out`) | peak RSS 6–14 GiB (`4201`: verify needs 16 G); wall 10–30 min; verdict: the gates it runs today pass |
| P19 | §1.9 | `4400g` moves the TIM1 clock (DShot bit 1,657 ns), kind 26's values (not its widths), and the version numbers. It does not move boot-to-`CEN`, the analog arithmetic, or ngspice memory |

## Results — measured after the predictions (`74c5dbf6`)

**How it ran (measured).**
- Every container ran through `spike/2304/sh/img.sh`, one at a time (flock): `nice -n 10 ~/bin/capped-run`, `--network none`, the worktree read-only, TMPDIR unset.
  - Caps read back inside the scope before each container: `memory.max=8589934592 memory.swap.max=0 pids.max=8192`, or `17179869184` for `verify` (`spike/2304/out/caps.log`).
- Image: `container/IMAGE`, the pinned tools image (Renode 1.17.0, ngspice 44.2).
- The CLI and the bridge were built unmodified from `cd43b207`. The probe was built from `spike/2304/lockstep` (2303's, restored) and `spike/2304/ngspice` (2303's FFI copy, unchanged, outside the workspace).
- **Before the predictions were committed, the mirror's `buck_avail_uv` was evaluated at five points:**
  - U11 at 3.8 V and 748 mA: 3.52 V;
  - U10 at 7.0, 5.2, 5.3 and 5.4 V and 2 A.
  - P2's 3.52 V and P5's 5.3 V came from those calls, so P2 and P5 are not blind predictions.
- **Unpredicted** (added after the predictions, with nothing to hold them against):
  - §1.4's 60/100/127 A rows and its deck-rule column;
  - §1.5's band runs;
  - §1.8's `check_droop` results and the bridge rail comparison;
  - §1.2's pad list from the emitted board.
- **Discarded, not reported as results:** the first three (a) runs, made with a deck error.
  - The enable source was written `dc 1 external`. Measured: ngspice adds the callback's value to the source's `dc` value, so the enable read 2.0 and the bucks drew twice their power.
  - Found by dumping the deck's nodes. Fixed to `dc 0` and re-run. Every reported (a) and (b) run uses the fixed deck.
  - `2303`'s `vctl` was `dc 0`, so it never met this.

### §1.1 The rows, at source

Corrections are marked.

**Provenance:** a read-only subagent gathered the citations, and I spot-checked them. The load-bearing ones I re-read first-hand: `verify.rs` L705, L774, L785–796, L827, L879–944; `crates/aethelgard-cli/README.md` L924; `connectors.rs` L165–168; `power.rs` L1587, L1607, L1642; `power_parts.rs` L302–307, L330–336; `schematic_power.rs` L431–432; `cli/src/campaign.rs` L380–390; and the version constants.

| Row | Item | Classification | At source |
|---|---|---|---|
| L608 | The chain into `--rail` | **dynamic**, the question of §1.5. Confirmed | Nothing chains a regulator into `--rail` today. The droop is a declared trapezoid on the MCU's supply only (`cli/src/campaign.rs` L282–290, `DroopNotMcuSupply`) |
| L695 | A covering tolerated fault moves its quantity past the tolerance | **static**, a coverage rule. Confirmed; **the row is wider than battery sag** | `verify.rs` `stress` L705–; the droop arm L785–796. The row names pull-up bands and capacitance maxima, which are I2C rules, not battery work. Only its droop half touches this order |
| L696 | A serial fault kind | **done by `2302` D5; the row is stale** | `verify.rs` L774 (`SerialRxOpen` → `Target::Serial`, mechanism `OPEN`); L879–880 ("covered only by a detected `serial_rx_open` (AET-WO-2302 D5…)"); L920–927; `crates/aethelgard-cli/README.md` L924. The row says "held or open": only *open* exists, and a held line is `LineHeldLow`, an I2C override treated as intolerable (L827) |
| L697 | The droop's band check against the loaded floor | **static**. Confirmed | `verify.rs` L785–796 compares the declared `floor_millivolts` with `nominal × (1000 − tolerance)/1000`, with no I·R. `chaos/src/campaign.rs` L431–441 `rail_levels` subtracts `drop_microvolts()` (`record.rs` L213–215) from both floor and rest. So the scheduler uses the loaded floor and coverage uses the declared one. Also read: `stress` gives a droop no mechanism, so only a *survived* droop covers a rail |
| L707 | The power fixture's full package | **the goal** | `PowerNotSimulated` at `cli/src/campaign.rs` L380–390 (in `effective`, L372); its test is `campaign_tier1.rs` L2707–2729; `--fab-out` requires `--scenarios` at `main.rs` L124–125. All confirmed |
| L722 | J2's current rating against the battery sag | **static; the wording is wrong** (confirmed) | `4300g` K7 (FEEDBACK L122) is Sentinel-01's J2, a JST GH, against the MCU's draw. On Sentinel-03, J2 is `SM06B-SRSS-TB`, the SWD header (VDD_3V3, BOOT0, NRST, SWDIO, SWCLK, GND). The battery input is `power.input` J4, `XT30PW-M`, 15 A (`ir/src/connectors.rs` L165–168). IR already judges J4 statically: `power_in` (`power.rs` L1587) sums each battery-fed buck's *output* load (L1607) against the rating (L1642). It never sees motor current. The row should read "J4 against the ESC current, if it flows through J4" (§1.2) |
| L723 | A capacitor's dielectric and ESR | **static; half done** | The dielectric is already a field (`power_parts.rs` L302–307: `dielectric`, `dc_bias`). The gap is **ESR on ceramics**: only `BulkCapacitor` has `esr_max_milliohms` (L330–336). U11's outputs (`GRM32ER71A226KE20`, X7R) have no ESR and `dc_bias: null` |
| L799 | The writer passes the reader's 256 MiB | **static**. Confirmed | `MAX_TRACE_BYTES` `archive.rs` L26, checked only on read (L329–334). `TraceWriter::append` (L161–180) checks only `MAX_EVENT_BYTES` (4,096), the replayer's record cap and time order; `commit` (L187–205) checks nothing more |
| L800 | The ESC's supply current as a power-parts row | **data**. Confirmed. §1.3: no maker states the idle figure | `MotorOutput` (`ir/src/motor.rs` L82) is label, pin, protocol, pwm: no ESC, no current. `power_parts.rs` `Kind` (L534–544) has nine kinds and no ESC |
| L801, L802 | DShot stop at run end; lock-step's costs | **only under (b)**. Confirmed | `twin/src/boot.rs` L54 `DSHOT_WINDOW_NS`; §1.5 below |
| L804 | `4500`'s profile; the lean to (b)+(a1) | the question of §1.5 | — |

### §1.2 Where the motor current flows (read at source)

- **Sentinel-03 declares no ESC and no path for its current:**
  - `motor_outputs` are MOTOR1–4 on PE9, PE11, PE13 and PE14, `dshot600`.
  - They leave on **J5, `SM06B-GHS-TB`**: pins 1–4 carry MOTOR1–4, pin 5 has no net, and pin 6 is GND. **J5 carries no `VBAT` and no power pin** (`sentinel-03.json` `connectors`).
  - J4 (`XT30PW-M`) lands on net `VBAT` (`eda/src/schematic_power.rs` L21–23, L322–327; `BATTERY_NET`, `ir/src/power.rs` L82). `VBAT` feeds U10 and U11 and the sense divider R50/R51.
  - Net class `POWER` (500 µm) holds `VBAT` and `VDD_5V`.
  - `power.loads` holds no ESC or motor load.
- **Measured on the emitted board** (`generate`'s `kicad/sentinel-03.kicad_pcb`, sha256 `674a3980…8dba`; pads read by net):
  - `VBAT` reaches J4.2, U10.2/3 and U11.2/3 (VIN and the EN strap), C10.1, C11.1, C20.1, C21.1 and R50.1, and nothing else.
  - J5 carries `/MOTOR1`–`/MOTOR4` on pads 1–4, an unconnected pad 5, and GND (pad 6 and MP).
  - C1, the hot-plug part, is not drawn yet: that is `1315`.
- **The power fixture** (`crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json`) declares **no motor outputs at all**. Its battery input is J1, `XT30PW-M`, with the same C1. So its battery load is the tree's static load, and there is no ESC path to read.
- **Finding F1: the specification cannot say whether ESC current passes through J4 and `VBAT` copper.** The only thing it supports is that ESCs are *not* fed through J5. The schema has no field for it.
  - So the chain's R splits two ways. For the ESC current, R is pack plus leads, plus J4 and `VBAT` copper only if a pad on the board feeds the ESC. For the board's own draw, R is pack plus leads plus J4 plus copper.
  - **L722** (J4 against ESC current) and **`1303b`'s supply gate** rest on the same undeclared fact.
  - §1.4 treats R as one series path that both currents share. That is the worst case. At J4's stated 0.8 mΩ, splitting it changes nothing at the scale of §1.4's answer (inferred from the figures).
- **Sentinel-03's use (read at source):**
  - Roadmap L367: "The power tree, the flight sensors and the witness board go to Sentinel-03, at four layers".
  - Roadmap L375: Sentinel-03 "is built before Sentinel-02, so it proves the sensors Sentinel-02 will fly".
  - `3400` FEEDBACK L169 and L289: J5 is "four TIM1 channels on J5, which can capture DShot from the next board with firmware alone". It is a bench and measurement header.
  - Roadmap L380: "The battery under the motors is Sentinel-02's one firmware-dependent fault".
  - Nothing read says Sentinel-03 will drive ESCs, on the bench or in the air.
  - **So Sentinel-03's campaign battery load is the tree's static load:** U10's 2 A `VDD_5V` budget and U11's 748 mA. That is inferred from the above; Scotty confirms it (D2).

### §1.3 Sourced numbers (`~/.cache/aet-ds/aet-2304/`, each sha256 in its `SHA256SUMS`; never in the repository)

| Quantity | Source (revision, page) | sha256 | Figure, verbatim | Status |
|---|---|---|---|---|
| ESC idle, disarmed (DShot 0, and no signal) | Hobbywing XRotor Micro 40A 4in1 BLHeli-S DShot600 manual, p. 1 | `be8591af…3dc7` | "the current on the ESC is consuming continuously if it's connected to the batteries (even if the ESC is turned off)" | **not stated**: non-zero, no figure |
| ESC idle | Hobbywing XRotor Micro 40A BLHeli_32 4in1 manual (2019-08-28), p. 1 | `86e5487e…f721` | "Long-time contact will cause the battery to completely discharge." | **not stated** |
| ESC idle | Holybro Tekko32 F4 4in1 Mini 45A spec #31098 (©2018) | `eab0a25c…edcc` | — | **not stated**; no continuous rating either |
| ESC idle | Holybro Tekko32 F4 BigFET 4in1 50A spec #31115 (©2018), p. 1 | `c1d63099…bc2f` | — | **not stated** |
| ESC continuous, per channel | Hobbywing XRotor Micro 40A (both manuals), p. 1 | as above | "Cont. Current 40A" (BLHeli-S: 2–5S; BLHeli_32: 3–6S, 20×20) | stated |
| ESC continuous, per channel | Holybro Tekko32 F4 BigFET 50A, p. 1 | `c1d63099…bc2f` | "Continuous Current: 50A x4"; "Burst Current: 60A x4" (4–6S) | stated |
| Pack R, 3S 850 mAh | Tattu R-Line V5.0 850 3S 150C (retailer page) | `51ac31ff…aaef` | "reduce internal resistance" (qualitative); 150C | **not stated** |
| Pack R, 4S 850 mAh | Tattu R-Line 850 4S 95C (gensace.de page) | `4f18c3bd…65ad0` | "lower internal resistance" (qualitative); 95C | **not stated** |
| Pack R, other makers | CNHL, GNB, Ovonic (search; not fetched) | — | no numeric figure found | **not stated**. Third-party bench reviews give about 12–13 mΩ a pack (CNHL 650/850 mAh). They are not a maker figure and are cited only as context |
| Connector CR | AMASS XT30PW-M spec sheet V1.2 (distributor copy), p. 1 | `c57b5a37…f185` | "接触电阻 CR 0.80mΩ"; "额定电流 RC 15A"; "瞬时电流 MC 30A" | stated; no condition, neither max nor typ |
| Connector CR, cable side | XT30U manual (Rotorama, a reseller copy), §1.2 | `a48af4af…2431` | "Contact resistance ≤ 1.2 mΩ" | stated (reseller) |
| U11/U10 UVLO | TI SNVSAN3F rev F, p. 15 §8.3.4 | `3b0920a4…c04a` | "When VCC reaches about 3.7 V … start up. When VCC falls below about 3 V, the device shuts down … roughly represent the input voltage levels" | typical only; p. 7 states "VIN minimum operating input voltage 3.8 V (max)" |

**Gaps (F2):** no maker states an ESC idle current or a pack resistance. Both run as **labelled sweeps** (§1.4). The sourced figures are the ESC continuous ratings, J4's CR and the bucks' UVLO.

### §1.4 The chain, statically (`spike/2304/chain/chain.py`; the run is `spike/2304/out/chain.json`)

**What the script is.**
- **The mirror.** It mirrors IR's arithmetic: `battery_range`, the buck's output range, the loads, `headroom`'s duty-limited `avail`, and the LDO's need.
  - **What `selfcheck` covers:** both bucks' output ranges, both loads, and U11's `avail` at four points. All equal IR's own numbers, read from unmodified `validate` refusals (predictions section).
  - **What it does not cover:**
    - U10's `avail` is the same function with U10's inputs. It was not compared with IR, because `validate` cannot reach it: D-B9's 7.0 V floor refuses first.
    - The power fixture's U12 `need` was not compared with IR either.
- **The spike's model, on top.** IR does not have this; its own note says it counts "no input-path drop". The model:
  - one series R;
  - the bucks as constant power;
  - the ESC as a constant current;
  - a buck below its 3.8 V stated operating minimum counts as "unstated", so in BOR3's band.
- **Board inputs, computed by the mirror from declared maxima** (not a measured draw):
  - `VDD_3V3` 748 mA (the MCU's IDD max of 658 mA plus declared parts);
  - `VDD_5V` 2,000 mA (the DS-009 port budget).
  - That is 12.47 W lossless, or **14.16 W at the declared η**: 1.57 A from 9.0 V. The η figures (90 % and 81 %) are the fixture's 36 V-input typicals, used at 9 V; the lossless column brackets them.
- **One rule, two readings (adversarial M1).**
  - The script counts V_bat below 3.8 V as `VDD_3V3` entering the band: below U11's stated input range, so unstated.
  - The §1.5 deck, built after the predictions, follows the datasheet's typical UVLO instead: off below about 3.0 V, on above 3.7 V (SNVSAN3F §8.3.4), with `VDD_3V3` = `avail` in between. That stays above 2.68 V down to V_bat ≈ 2.93 V.
  - The predictions section says "off below 3.8 V"; the deck as run differs, as stated here.
  - Both readings are tabled. They differ only where the 3.8 V floor binds (2 A and up).
  - The script's `avail`-below-BOR3 column never fires before the floor, so it is not tabled.

**R at which each limit is first reached** (Sentinel-03, V_open 9.0 V, declared η; lossless in brackets where given).
- "Collapse" means there is no operating point.
- The power fixture is within 0.93 % of Sentinel-03 at every entry. Its `VDD_1V8` follows `VDD_3V3` (U12 needs 2.227 V in), so it leaves tolerance at the same R.

| ESC current (total of four) | `VDD_3V3` leaves U11's stated input range (3.8 V) or collapses | what binds | The deck's rule (V_bat < 3.0 V, or collapse) | `VDD_5V` leaves 4.75 V | V_bat below U10's 7.0 V `regulation_vin_min` |
|---|---|---|---|---|---|
| 0 (the board alone) | **1.430 Ω** (1.624) | collapse at V_bat 4.50 V | 1.430 Ω | 1.384 Ω (1.572) | 0.989 Ω (1.123) |
| 0.1 A, sweep | 1.387 Ω (1.568) | collapse | 1.387 Ω | 1.334 Ω | 0.942 Ω |
| 0.2 A, sweep | 1.346 Ω (1.517) | collapse | 1.346 Ω | 1.288 Ω | 0.900 Ω |
| **0.4 A, sweep (I1)** | **1.273 Ω** (1.425) | collapse at 4.25 V | 1.273 Ω | 1.204 Ω | 0.826 Ω |
| 0.8 A, sweep | 1.152 Ω (1.277) | collapse at 4.04 V | 1.152 Ω | 1.065 Ω | 0.709 Ω |
| **2.0 A, sweep (I2)** | **0.908 Ω** (0.985) | 3.8 V floor (collapse at 3.59 V) | 0.910 Ω | 0.791 Ω | 0.497 Ω |
| **10 A (T10, `2303`'s)** | **0.379 Ω** (0.392) | 3.8 V floor | 0.408 Ω | 0.292 Ω | 0.166 Ω |
| 60 A, sweep | 81.6 mΩ | 3.8 V floor | 92.7 mΩ | 58.9 mΩ | 32.2 mΩ |
| 100 A, sweep | 50.1 mΩ | 3.8 V floor | 57.3 mΩ | 36.0 mΩ | 19.6 mΩ |
| 127 A, sweep (an 850 mAh pack at its stated 150C) | 39.8 mΩ | 3.8 V floor | 45.6 mΩ | 28.5 mΩ | 15.5 mΩ |
| 160 A (4 × 40 A rated) | 31.8 mΩ | 3.8 V floor | 36.4 mΩ | 22.7 mΩ | 12.3 mΩ |
| 200 A (4 × 50 A rated) | 25.5 mΩ | 3.8 V floor | 29.3 mΩ | 18.2 mΩ | 9.9 mΩ |

The 160 A and 200 A rows are the ESCs' stated continuous ratings. A 3S 850 mAh pack at its stated 150C supplies about 127 A, so those two rows are a bound that pack does not reach. The 60–127 A rows are a labelled sweep.

**Margins at fixed R, declared η** (V_bat, then the margin to U11's 3.8 V floor):

| R | 0 A | 0.4 A | 2.0 A | 10 A |
|---|---|---|---|---|
| 0.8 mΩ (J4's CR alone) | 8.999 V, +5.20 V | 8.998 V, +5.20 V | 8.997 V, +5.20 V | 8.991 V, +5.19 V |
| 0.05 Ω | 8.921 V, +5.12 V | 8.900 V, +5.10 V | 8.820 V, +5.02 V | 8.416 V, +4.62 V |
| 0.25 Ω | 8.588 V, +4.79 V | 8.483 V, +4.68 V | 8.061 V, +4.26 V | 5.900 V, +2.10 V |
| 1.0 Ω (the weak case) | 6.968 V, +3.17 V | 6.381 V, +2.58 V | **no operating point** | **no operating point** |

While U11 regulates, the script puts `VDD_3V3` at the midpoint of IR's range: 3.281 V, which is 601 mV above BOR3's falling maximum of 2.68 V. That pairs a midpoint rail with IR's worst-case `avail`, which is a bound and is labelled as one. The margin is therefore in R (or in amps), not in volts.

**The answer (§1.4's question).**
- **At idle**, across the swept idle range of 0–2 A, `VDD_3V3` first enters BOR3's band at **0.91–1.43 Ω** (declared η). **What binds is the board's own declared 14 W constant-power load collapsing**, not the ESC: at 0 A it already collapses at 1.43 Ω.
- **At high current** (the 60–127 A sweep) it enters at **40–82 mΩ**; at the rated 160–200 A, at 25–32 mΩ (by the deck's rule, 29–36 mΩ).
- **`VDD_5V`** leaves its tolerance 0.05–0.12 Ω before that at idle, and 7–23 mΩ before at high current. Its regulation band stops being stated (V_bat < 7.0 V) at 0.50–0.99 Ω at idle.

**The decisive measurement.**
- **What the generated firmware can change:** only whether DShot-0 frames are present (`CEN`).
- **What its lever is worth:** an ESC with no signal still draws (Hobbywing: "consuming continuously … even if the ESC is turned off"). So the lever is the difference between DShot-0 idle and no-signal idle, and no maker states either figure.
  - The bands below treat no-signal idle as 0 A, which makes them an **upper bound** on what the firmware can move.
- **Stated in amps, at the WO's weak pack (1.0 Ω):** the firmware moves a crossing **only if its lever is at least 1.474 A** in total, for four ESCs. Below that the board keeps an operating point with `CEN` set, and the verdict is the same either way. The arithmetic: (9 − I)² = 4 · 1.0 Ω · 14.16 W.
  - Measured: I1 (0.4 A) shows no reset under (a) or (b). I2 (2.0 A) loops under (b).
- **Stated in ohms, at a given lever:** the firmware moves a crossing only when the declared R lies between R*(lever) and R*(0) = 1.43 Ω.
  - At 0.4 A the band is **1.27–1.43 Ω**. At 0.8 A it is 1.15–1.43 Ω. At 2.0 A it is 0.91–1.43 Ω, and the WO's 1.0 Ω sits inside it.
  - So the band's width is set by the ESC's unsourced idle figure. **The 0.8 A edge used in an earlier draft was not sourced, and is withdrawn** (adversarial H3).
- **How far the band sits from a credible healthy path R.** There is no maker figure for pack R (F2), so this is inferred.
  - The band's lower edge at 0.4 A is about 106× the third-party ~12 mΩ for a healthy 850 mAh pack.
  - A whole cold path at a low state of charge may be several times higher. That is not quantified here, because it would be an estimate (`INV-06`), so no path ratio is claimed.
  - The ratio to J4's stated 0.8 mΩ alone (1,590×) is a connector ratio, not a path ratio, and is not the margin.
- **The conclusion.**
  - On the generated firmware, a crossing that depends on the firmware needs **both** a declared pack R tens of times a healthy one, **and** an ESC idle difference (DShot-0 against no signal) large enough to reach that R's collapse: ≥ 1.47 A at 1.0 Ω.
  - Neither figure is sourced. A board that drives no ESC (Sentinel-03, §1.2) has no lever at all.

**Each prediction against its result (§1.4):**
- **P1 held:** 14.16 W, 1.57 A (computed from declared maxima).
- **P2 held** up to 0.8 A, where the collapse binds. From 2.0 A up, U11's 3.8 V floor binds instead, which was not predicted.
- **P3 held:** declared 1.43 → 0.91 Ω; lossless 1.62 → 0.98 Ω.
- **P4 missed:**
  - the rated 160–200 A cross at **25–32 mΩ**, not 50–60 mΩ;
  - 10 A crosses at 0.38–0.39 Ω, not 0.40–0.45.
  - Partly this is definition: by the deck's 3.0 V rule the figures are 29–36 mΩ and 0.41 Ω.
- **P5 held in sign, missed in size:**
  - `VDD_5V` leaves first, but by 0.05–0.12 Ω at idle, not 0.15–0.3.
  - Its band becomes unstated at 0.99 Ω (declared) and 1.12 Ω (lossless) at 0 A, against the predicted ~0.9.
- **P6 held in kind, missed in width:** 0.157 Ω wide at 1.273–1.430 Ω, against the predicted ~0.3 Ω at 1.3–1.6 Ω.

### §1.5 The two candidates, run (one board, one fault: the weak pack, 1.0 Ω)

**Setup (measured).**
- Sentinel-03 was generated and built by the unmodified CLI: ELF `5756a2e0…ca51`.
- `aethelgard simulate --for 100ms` gives `pass` (finding kind `device_unmodelled` only), 9.0 s wall (`spike/2304/out/simulate-s03.json`). Its Renode argv was captured from `/proc` (`spike/2304/out/renode-argv.txt`).
- The probe replays the argv's setup, as `2303` did, with the same deviations:
  - the hooks and the capture backends are left out, so there is no DShot stop at the anchor plus 2 ms;
  - the emitted `.repl` stands in for the CLI's `platform-booted.repl`, which is `emit_faulted` with no faults (`cli/src/main.rs` L2055–2080).
- **The firmware's own timing:** the boot word `0x2100B007` and TIM1 `CR1 = 0x81` (`ARPE | CEN`), `CCR1 = 0x28`, at **39.13 ms** (10 µs) and 39.2 ms (100 µs). `2303`'s Sentinel-02-shaped board took 39.08 ms.
- **The deck's boundary-quantised elements:** the ESC command and the reset are quantised under (b), as in `2303`. The bucks' enable (hysteresis 3.7 V on, 3.0 V off; SNVSAN3F §8.3.4, typical) is decided at each quantum boundary in both candidates. At 100 µs it acts up to 100 µs late. That deepens a sag past the collapse far below what 10 µs shows (I2: V_bat 0.46 V against 2.93 V). It is a spike-design artifact, not a property of either candidate; a build puts the enable in the deck.

**The runs** (1 s simulated each; `spike/2304/out/runs/*/run.json`, `actions.txt`, `replay.jsonl`; `stats.txt` holds each net's extremes read from the samples trace alone).

| Case | Candidate | Boot cycles (`CEN` sets) | Resets in 1 s | First reset asserted | `VDD_3V3` at assertion | Cycle period | V_bat min | Wall | Renode RSS start → end | Probe peak |
|---|---|---|---|---|---|---|---|---|---|---|
| I1, 0.4 A | (b) 100 µs (×2) | 1 | **0** | — | — (min 3.2626 V) | — | 6.381 V | 129.6, 125.2 s | 424 → 435 / 439 MiB | 103 MiB |
| I1 | (b) 10 µs | 1 | **0** | — | — (min 3.2626 V) | — | 6.381 V | 205.4 s | 426 → 463 MiB | 137 MiB |
| I1 | (a) | — | **0** | — | — (min 3.2626 V) | — | 6.381 V | 12.2 s | — | 151 MiB |
| I2, 2.0 A | (b) 10 µs | 24 | **24** (one a cycle) | 41.06 ms (1.93 ms after `CEN`) | 2.607 V | 41.07 ms | 2.930 V | 98.2 s | 425 → 452 MiB | 137 MiB |
| I2 | (b) 100 µs | 24 | **24** | 41.20 ms (+140 µs) | 0.915 V (deck artifact, above) | 41.4 ms | 0.456 V | 21.9 s | 427 → 442 MiB | 103 MiB |
| I2 | (a) | — | **4,630** | 41.93 ms (+870 µs) | 2.61 V (interpolated) | — (the enable cycles, F5) | 2.880 V | 12.1 s | — | 188 MiB |
| T10, 10 A | (b) 10 µs | 25 | **150** (six a cycle) | 39.49 ms (0.36 ms after `CEN`) | 2.610 V | 39.89 ms | 2.773 V | 88.8 s | 423 → 447 MiB | 137 MiB |
| T10 | (b) 100 µs | 24 | **48** (two a cycle) | 39.60 ms (+110 µs) | 0.024 V (deck artifact) | 40.3 ms | −1.54 V (ideal source) | 35.6 s | 431 → 446 MiB | 104 MiB |
| T10 | (a) | — | **7, then held to the end** (from 40.71 ms) | 40.36 ms (+870 µs) | 2.61 V (interpolated) | — | −1.0 V (ideal source) | 13.4 s | — | 151 MiB |

- **Several resets within one boot cycle under T10 are the deck, not the firmware.**
  - After the reset clears `CEN`, the ESC current decays through its 1 ms lag.
  - While it decays, V_bat crosses the bucks' enable hysteresis several times, and each crossing renders a reset and a release.
  - How many depends on the quantum (6 at 10 µs, 2 at 100 µs) and on an enable behaviour the LMR33630 does not state (hiccup). **The boot-cycle count is the robust figure: 24–25 at both quanta.**
- **V_bat below 0 V** comes from the ESC modelled as an ideal current source with no compliance, an undeclared behaviour. It occurs only where there is no operating point.
- **How close the 10 µs reference crossings are (adversarial M3; read from `actions.txt`):**
  - I2 asserts at `VDD_3V3` = 2.6066 V against the typical 2.61 V, a margin of 3.4 mV. T10's first assertion is at 2.6099 V, 0.1 mV under.
  - BOR3's falling band is 2.54–2.68 V. At the band's minimum, I2 at 10 µs would not reset, since its rail minimum is 2.6066 V.
  - So the loop's existence at those loads rests on the typical threshold, the deck's 50 mΩ regulator R, and its UVLO hysteresis, as well as on the firmware.
  - The I1 result (no reset, 583 mV clear) does not rest on any of them.
- **Determinism and replay (measured):**
  - The two I1 100 µs runs are byte-identical in all three traces (sha256 in `spike/2304/out/trace-sha256.txt`: samples `b441b681…`, records `4bac55ee…`, events `374e5011…`).
  - `replay_dir` reproduces the state-sequence hash of every trace of every run.
  - The bridge's actions, re-derived from the recorded `VDD_3V3` samples alone, equal the archived `ResetRendered` records: 48 of 48 (I2, both quanta), 96 of 96 (T10, 100 µs), 300 of 300 (T10, 10 µs).
  - (a)'s traces hold no samples, so that check does not apply to them.

**Runs added after the predictions (unpredicted): D1's band at a 0.4 A lever, either side of it** (`spike/2304/sh/band.sh`; (b) at 100 µs, 1 s each).

| Declared R | Static (§1.4, 0.4 A) | (b) lock-step | (a) declared |
|---|---|---|---|
| 1.20 Ω | below the band: an operating point with `CEN` set | **0 resets**; one `CEN` at 39.2 ms; 122.7 s wall | **0 resets**; V_bat 5.336 V: **agree** |
| 1.35 Ω | inside the band (1.27–1.43 Ω): an operating point only without the ESC | **a reboot loop: 24 cycles, 24 resets**, first at 41.0 ms. The firmware moves the crossing | **2,879 chattering resets** (F5): the case (a)'s refusal must catch |
| 1.45 Ω | above R*(0) = 1.43 Ω: no operating point even without the ESC | **never boots**: no `CEN`, 470 resets from 2.4 ms | the events trace passed `MAX_RECORDS`: the writer refused record 65,537 (`TooManyRecords`) and the probe stopped. Refused, as F5 says it should be |

- Measured, so the band is no longer only static.
- Below it, (a) and (b) agree. Inside it, the firmware moves the crossing. Above it, the board cannot run at all.
- (a)'s output inside and above the band is exactly what F5's refusal removes.

**The archive per simulated second (measured, the real `TraceWriter`; stand-in kinds as in `2303`).**

| Shape | 100 µs | 10 µs |
|---|---|---|
| Samples: `VBAT` and `VDD_3V3` as kind 3, `CR1` as kind 0 | 499,914–509,691 B (**0.50–0.51 MB/s**) | 5,000,971–5,096,134 B (**5.0–5.1 MB/s**) |
| Records: three stand-ins a quantum (kinds 17, 17, 9) | 989,290–999,355 B, 30,000 records | **65,536, then refused at 218.46 ms** (as `2303`) |
| Events (changes only) | I1 135 B; I2 4,908 B; T10 9,060 B | I1 157 B; I2 4,930 B; T10 26,755 B |
| (a), events only (crossings as kind 17, resets as kind 24) | I1 82 B (header only); T10 2,167 B; **I2 2,158,030 B, 47,816 events** (F5) | — |

**(a) against (b): the error (a) carries**, in `2303`'s units, with (b) at 10 µs as the reference:

| Case | Late by (first assertion) | Deeper by | Count, (a) against (b) |
|---|---|---|---|
| I1, 0.4 A | — | — | 0 against 0: **no error** |
| I2, 2.0 A | **+870 µs**: the declared 40 ms step against the measured `CEN` at 39.13 ms | (a) 2.88 V against (b) 2.93 V V_bat min | **4,630 against 24**: (a)'s load never stops, so past the collapse its enable cycles; no boot cycles |
| T10, 10 A | **+870 µs** | — | **7 then held to the run's end, against 150 resets in 25 boot cycles**: (a) cannot show the reboot loop (`2303` D1 (ii), confirmed on Sentinel-03) |

(b) at 100 µs against 10 µs: +110 to +140 µs late, the same boot-cycle count. The reset count within a cycle differs (2 against 6), from the deck's quantised enable.

**What `bridge --rail` would have to carry beyond a trapezoid** (read at source, `bridge/src/rail.rs` L17–30): the probe's (a) deck shows the full list.
- The pack: V_open, the series R (the fault), and the bulk C with its ESR and the input ceramics.
- Each battery-fed regulator: input power, or its loads and η; its enable thresholds; and its output model (`avail`) with its output C.
- The rail's load.
- A step profile for the ESC current, with its lag.
- Today's request is one trapezoid (`vertices_ns: [u64; 4]`), a scalar `load_ua`, a scalar `source_mohm` and one `decoupling_picofarads`.
- The CLI's `rail_answer` refuses more than two crossings of a level (`analog.rs` L393–398), so it would refuse (a)'s I2 answer outright.

**Each prediction against its result (§1.5, §1.8, §1.10):**

| id | Predicted | Result | |
|---|---|---|---|
| P7 | I1: no reset; V_bat 6.7–7.3 V; margin > 600 mV | no reset at either quantum; V_bat **6.381 V**; `VDD_3V3` 3.2626 V, **583 mV** above 2.68 V | held for resets; missed for V_bat (0.32 V low) and margin (17 mV short): the deck's declared η and its 50 mΩ regulator R |
| P8 | I2: a reboot loop, period = boot-to-`CEN` + a few ms | 24 cycles, period 41.07 ms = 39.13 + 1.93 ms | held, at a 3.4 mV margin to the typical BOR3 level (above) |
| P9 | T10 (margin 0.1 mV at 10 µs): reboot loop; reset 1–3 ms after `CEN`; boot-to-`CEN` 35–60 ms; same count at 100 µs | loop held; boot-to-`CEN` **39.13 ms** held; reset **0.36 ms** after `CEN`; resets 150 vs 48, cycles 25 vs 24 | missed on the reset time (faster than predicted) and on the reset count across quanta; held on the cycle count |
| P10 | (a) I1 equals (b) | 0 against 0 | held |
| P11 | (a) I2, T10: one fall, held to the end; late by (40 ms − `CEN`) | T10: 7 short resets then held, +870 µs. I2: **4,630** chattering resets | held for T10 in form; **missed for I2** (F5) |
| P12 | samples 0.50 / 5.0 MB/s ±15 %; events < 10 KB/s | 0.50–0.51 / 5.0–5.1 MB/s; events up to 26.8 KB/s (T10, 10 µs) | held for samples; missed for T10's events (the deck's resets within a cycle) |
| P13 | (a) under 1 KB beyond the static | 82 B, 2.2 KB, **2.16 MB** | missed for T10 (2.2 KB) and I2 |
| P14 | (b) I1 150–250 s/s | **125–130 s/s** at 100 µs, 205 s/s at 10 µs | missed low at 100 µs; held at 10 µs |
| P15 | Renode 420–520 MiB; probe < 200 MB | Renode 423–463 MiB; probe 103–188 MiB | held |
| P16 | `replay_dir` reproduces; byte-identical | yes; yes | held |
| P17 | 10,810,000 pF | 10,810,000 pF | held |
### §1.6 The engine's cost, per candidate (read at source on `main` `cd43b207`; versions given after `4400g`)

Today's versions: specification 16 (`ir/src/lib.rs` L81), solved topology 10 (L447), power-parts 4 (`power_parts.rs` L35), parts 10, scenario `[1, 2, 3]` (`chaos/src/campaign.rs` L32), trace 13 (`chaos/src/event.rs` L43). `4400g` takes trace 14, specification 17 and solved 11.

| Cost | (a0) lift `PowerNotSimulated` only | (a) the declared chain, a new fault kind on the battery | (b) lock-step at 100 µs with (a1) per-quantum samples | (d) re-plan at each firmware command |
|---|---|---|---|---|
| Specification | none | **18**: a healthy-pack series R on `Battery` (`ir/src/power.rs` L200–207; D3). An ESC per motor output (`ir/src/motor.rs` L82; `names.rs` L220 and `connectors.rs` L288 iterate it) only if the board drives one, so not for Sentinel-03 (§1.2) | as (a) | as (a) |
| Power-parts | none | **5** for the bucks' enable thresholds (F5, D5); an `esc` kind (`power_parts.rs` `Kind` L534–544, `name()` L547–559; `data/power-parts.json`; consumers in `eda` `stack.rs`, `supply.rs` and `schematic_power.rs`, `ir/src/names.rs`, `hwdef`) is a further bump, which Sentinel-03 does not need | as (a) | as (a) |
| Scenario version | none | **4**: a `battery_sag` override (`chaos/src/record.rs` `Override` L182–, wire L903–913 and L963, `name_ok` L1917; `chaos/src/campaign.rs` `RawDroop` L176, `droop_from` L381, sort L555–565, version gate L639–646) | **4**: the same declaration, plus the load's source (ESC per `CEN`) | as (b) |
| Trace version and kinds | none | **15**: one new kind, the battery-sag declaration (31; `record.rs` L985–1014 numbering, `check_run` ranks L755–780, `check_joins` L2583, `tools_of`). The chain's crossings reuse kind 17 on `VDD_3V3` | **15**: a per-quantum sample kind admitted by `check_archived` (L2893) and `check_joins`, plus (a)'s declaration kind | as (a); plus a record per re-plan (inferred) |
| Reader limits | none | `MAX_RECORDS` untouched: measured 0–45 events a run, but 47,816 in I2's no-operating-point case, which the build must refuse (below). `MAX_TRACE_BYTES` not approached | **L799 (the writer's bound) is needed.** Samples fold to one map entry per net, so `MAX_RECORDS` is never met. Measured 0.51 MB/s at 100 µs: 256 MiB at about 500 s (inferred from the rate). At 10 µs, see §1.5 | none beyond (a) (inferred) |
| `aethelgard-ir` | none | `Battery` R field, validation, the chain's static mirror (if the build keeps `RUL-20`'s rule beside the generator) | as (a) | as (a) |
| `aethelgard-chaos` | none | the override, the record kind, `check_droop`'s sibling (`check_battery_sag`), `check_run`/`check_joins`, scenario 4, trace 15, golden | as (a); plus the sample kind, the writer's bound (L799), and the reader's rules | as (a) |
| `aethelgard-cli` | `campaign.rs` L380–390 and its test (`campaign_tier1.rs` L2707–2729) | (a0), plus `resolve_droop`'s sibling, the bridge request (`analog.rs` `rail()` L280–340), `rail_answer` (≤ 2 crossings a level, L393–398), and `verify.rs` `stress`/`coverage` for `battery` (L705–, L941–944) | (a), plus a per-quantum runner beside `renode.rs`/`run.rs`. The DShot stop moves to run end (`twin/src/boot.rs` L54, `cli/src/anchor.rs`; L801). Renode's two INFO lines a quantum must be filtered (L802) | (a), plus hook-driven segmenting in the twin's runner (inferred) |
| `aethelgard-eda` | **F3: count a regulator's outputs in `decoupling_picofarads`** (`schematic_power.rs` L431–432), or (a0)'s droop runs on a 9×-short rail | as (a0) | as (a0) | as (a0) |
| `bridge/` | none | **`rail.rs` changes shape.** `KEYS` (L17–30) is a closed 12-key list of one trapezoid, `vertices_ns: [u64; 4]`, scalar `load_ua` and `source_mohm`. The chain needs the pack (V, R, bulk C and ESR), each battery-fed regulator (constant power, enable thresholds, `avail` and output C), and a step profile. That means a new request shape and deck writer, and `bridge/tests/rail.rs` | (a)'s deck, plus a lock-step loop: ngspice stepped per quantum beside Renode. The probe holds both in one process; the CLI sandboxes the bridge as its own tool, so a channel is needed (§2.27: the descriptor check; a named FIFO in the run's directory) | as (a) |
| Goldens and committed archives that move | none (one test flips) | `chaos/tests/golden.rs` L55–75 (trace hash), `archive.rs` L344–357 and L575. **`fab/sentinel-01-bench/package/MANIFEST.json` L48 `verdict_sha256`** moves with any verdict line (`cli/src/fab.rs` L694), as at `1605`/`1605c`, so the architect dispatches the package after merge. Spec asserts in `ir/tests/{orientation,serial,board,bench}.rs`, `readme_versions.rs`, every fixture's and example's `schema_version` | as (a) | as (a) |
| Tier-2 tests and CI minutes | +1 (a tree board's campaign with a droop) | +2 to 3 (battery-sag pass, weak-pack fail, coverage). (a)'s solve: 12–13 s per simulated second here (inferred: about 1.3 s for the boot's 100 ms) | + per lock-step scenario: see §1.5's wall. `TIER2_MIN` is 190 (`ci.yml` L381), with 8 shards and a 60 min guard | not measured |

**(a0) is a strict subset of (a)** (read at source). The refusal sits in `effective()`, before any override is applied, so any scenario on a tree board meets it first. (a) must lift it, and (a0) is that lift alone, with no version change.

**(d), named only, not measured.**
- The CLI already places watchpoint hooks on TIM1's `CR1` writes (the captured argv, L115–123), so the command's time is exact.
- But `2302` measured that no Renode call ends a `RunFor` early, and Renode has no cheap rollback. A crossing falling inside a segment already run cannot be undone (`2303` §1.3).
- Under the generated firmware there is one command per boot (`CEN`). So (d) would amount to (a) with its step time taken from the hook. **That is a crossing time read from the artifact under test, an `INV-07` question for the architect** (inferred).

### §1.7 Coverage, per candidate (`verify.rs` L882–946 as it stands; read at source)

**What meets a rail today.** A rail is met only by `Target::Rail(id)` in `covered`:
- `covered` comes from `Seeded::Tolerable` and from `Seeded::Detect` beside a baseline (L895–902);
- `Target::Rail(id)` comes only from a `droop` with `floor_millivolts` below the band, in a scenario expected to pass (L785–796);
- a failing droop is `Seeded::Other`, because `stress` gives it no mechanism;
- `droop` is refused on any rail but the MCU's supply (`DroopNotMcuSupply`, `cli/src/campaign.rs` L282–290).

**The rows.** Sentinel-03's other rows are the ones the rule already has and are not this order's: `CONSOLE`, `CRSF` and `MAVLINK` (each by a detected `serial_rx_open`), I2C `SENSORS` and `EXT`, SPI `FLASH` and `IMU`, and a baseline.

| Row | Today | (a0) | (a) declared chain | (b) lock-step |
|---|---|---|---|---|
| `VDD_3V3` (MCU supply) | refused (`PowerNotSimulated`) | **met** by a survived droop below 3,135 mV. It runs on the emitter's 10.81 µF, which is 9× short (F3) | met as (a0); the chain's own crossings are a second way | as (a) |
| `battery` | unmet (L941–944) | unmet: no kind targets it | **met without relaxing anything**, if `verify.rs` admits the new kind's target. The battery's sag reaches the twin through U11 and BOR, so the twin observes it (resets, or none). That is a rule *addition* in `stress` and `coverage`, not a relaxation | as (a) |
| `VDD_5V` | unmet (L932–936) | unmet | unmet as the rule stands. The chain *computes* `VDD_5V` (U10's `avail`) but **nothing in the twin runs on it** (read at source: no `VDD_5V` in `aethelgard-twin`, `chaos/src`, `bridge/` or `models/`) | unmet: Renode has no 5 V consumer either |
| `VDD_1V8` (power fixture) | unmet | unmet | as `VDD_5V`. It follows `VDD_3V3` (U12 needs 2.227 V), so it leaves tolerance only after BOR3 has reset the MCU (§1.4: same R) | as (a) |

**`VDD_5V`, which the twin cannot observe: the honest options** (each is D4's):
1. **Render the rail's excursion as its consumers' absence.**
   - Read at source: on Sentinel-03, `VDD_5V` feeds only off-board peers through connectors J6 (DS-009 TELEM: `CRSF`), J7 (TELEM: `MAVLINK`) and J8 (DS-009 I2C: `EXT`, no device declared).
   - While the chain holds `VDD_5V` below its band, the CRSF and MAVLINK peers go silent: their RX lines go open, which the existing serial judge **detects**. That is `RUL-17`'s habit (a held line rendered as absence).
   - This is a new mapping, not a relaxation. But how a DS-009 peer behaves below 4.75 V is not declared anywhere, so "absent" is a modelling assumption the architect rules on.
   - `EXT` has no peer and no judge, so a board whose 5 V feeds only an unjudged bus still cannot be met this way.
2. **Judge the rail statically** against its band, from the chain, as a gate verdict rather than a campaign fault. **This changes what `INV-03` counts as the fault campaign covering a rail: Scotty's.**
3. **Leave it unmet.** Any board with a non-MCU rail gets no package, which is today's rule.
4. **Name the rail "unobserved, not verified" in the verdict** (`RUL-19`'s habit) and let the package ship. **A relaxation: Scotty's.**

None is prototyped (§7: the relaxation is not prototyped as if ruled). Option 1 is the only one that meets `VDD_5V` without relaxing the rule, and it needs a ruling on the modelling assumption. **Per §7 this is not a stop:** `battery` is met by (a) without relaxing anything, and `VDD_5V` has one non-relaxing option.

### §1.8 The decoupling (read at source, then measured)

- **Read at source.** `decoupling_picofarads` (`eda/src/schematic.rs` L1118–1147) sums only parts whose owner `decouples`. Two things set it:
  - `Builder::decouple`, the per-device bypasses (L771–779; calls at L1397–1497);
  - the MCU's support capacitors (`schematic_support.rs` L106–108).

  A regulator's `cout` is drawn by `capacitor()` with no owner (`schematic_power.rs` L431–432), so **U11's C24–C27 are not counted**. Their value strings (`"22u GRM…"`, L171–174) would not parse in `picofarads()` (L1150–1166) even if they were.
- **Measured** (`spike/2304/decouple`, which calls the emitter's own function as `resolve_droop` does):
  - Sentinel-03: `{"VDD_3V3":10810000}`, i.e. **10.81 µF**. `VDD_5V` is absent, with nothing counted on it.
  - Power fixture: `{"VDD_1V8":200000,"VDD_3V3":8300000}`.
  - P17 held: 10,810,000 pF.
- **U11's outputs:** 4 × `GRM32ER71A226KE20`, 22 µF nominal, which is 88 µF. With them the rail holds **98.81 µF**, 9.1× the counted figure. At the fixture's 50 % DC-bias rule (`power.conditions.dc_bias_derating`, used only in IR's static checks) it holds 54.81 µF.
- **What `2301`'s droop computes, measured** with the real `aethelgard_chaos::campaign::{check_droop, tau_ns, droop_end_ns}` on `scenarios/motors/dshot600-droop.json`'s droop (2.4 V, 500 mΩ, 100 mA, hold 200 µs):

| Decoupling | τ | 10 τ | the droop's end | `check_droop` |
|---|---|---|---|---|
| 10.81 µF (counted) | 5.405 µs | 54.05 µs | 871.08 µs | `Ok` |
| 54.81 µF (with U11's, 50 % derated) | 27.405 µs | 274.05 µs | 1,091.08 µs | **`DroopRefused`**: "the floor is held 200000 ns, less than 10 τ (274050 ns)" |
| 98.81 µF (with U11's, nominal) | 49.405 µs | 494.05 µs | 1,311.08 µs | **`DroopRefused`** (10 τ = 494050 ns) |

- **The bridge's own rail solve at both figures**, measured: the unmodified `aethelgard-bridge --rail` on the same droop (`spike/2304/sh/rail.sh`, `spike/2304/out/rail/`).

| Decoupling | Hold | VBOR3 falling crossed | VBOR3 rising crossed | Rail minimum |
|---|---|---|---|---|
| 10.81 µF | 200 µs | 119.48 µs | 332.67 µs | 2.350000 V (the loaded floor) |
| 10.81 µF | 600 µs | 119.48 µs | 732.67 µs | 2.350000 V |
| 98.81 µF | 200 µs | 171.68 µs (+52.2 µs) | 354.09 µs (+21.4 µs) | **2.362884 V**: the floor is not reached |
| 98.81 µF | 600 µs | 171.68 µs | 754.67 µs | 2.350004 V |

  Nine times the capacitance moves the BOR3 crossing about 52 µs later, and the reset is still rendered. **So F3's consequence is `check_droop`'s refusal** (the floor is held under 10 τ), not a different reset.
- **Finding F3 · `decoupling_picofarads` leaves out a regulator's output capacitors · `eda/src/schematic_power.rs` L431–432, `schematic.rs` L1118–1147 · high for any tree-board droop · measured.**
  - On a tree board the droop's τ is 9× too small.
  - A droop scenario that passes today is refused once the outputs are counted.
  - So (a0) alone would run `2301`'s droop on a wrong rail model.
- **`1315`** draws the hot-plug part on `VBAT` (WO at `4f074d1f`). Read at source, it does not touch `VDD_3V3`'s decouplers; inferred, it does not move this number.

### §1.9 What `4400g` moves

Read at source on #238 at head `afb91036`, and re-checked at the stop on head `c07c6ebd` (main merged in, plus `4400g` F5's bench package): the same version bumps and file set for everything below. It was **open and unmerged** at every check (`main` stayed `cd43b207`). Every figure here was measured on `main`.

| Figure | Moves under `4400g`? | Why (read at source on #238) |
|---|---|---|
| TIM1's clock, and Renode's DShot cost per ms | **moves** | Every TIM at `timer_kernel_hz`, 64 MHz; DShot600 at 106 ticks is 1,657 ns (FEEDBACK §1.1, `record.rs` `realized_ns`). Inferred: DShot's edge rate falls about 3.9×, so (b)'s wall per simulated second with DShot running should fall. Re-measure on the new `main` |
| Kind 26's bytes | **values move, widths do not** | 424 → 1,657 ns and 639,744 → 2,499,000 ns stay 3-byte and 5-byte CBOR (inferred). The edge counts change |
| Versions | **move** | Trace 13 → 14, specification 16 → 17 (`mcu.foreign_clock`), **and the solved topology 10 → 11, which the WO does not list**. Power-parts (4), parts (10) and scenario `[1,2,3]` are unchanged. A build after `4400g` is trace 15, specification 18, solved 12 if it adds a solved field, power-parts 5 and scenario 4 |
| Files the build would share | **`chaos/src/campaign.rs` yes; `cli/src/campaign.rs` no** (corrects the WO) | 4400g edits `witness_consistent` and deletes `TWIN_CYCLE_NS`. Also shared: `chaos/src/{event,record}.rs`, `ir/src/{lib,motor,solve}.rs`, `chaos/tests/{golden,archive}.rs`, the `SPECIFICATION_SCHEMA_VERSION` asserts in `ir/tests/{orientation,serial,board,bench}.rs`, fixtures, `examples/`, READMEs |
| Boot-to-`CEN` (39.2 ms on Sentinel-03, below) | **does not move** (inferred, confirmed by reading) | Instruction counts; no judge reads the DWT (`no_judge_reads_the_dwt`). 4400g measured nothing about it |
| The analog arithmetic, ngspice memory, the bridge | **do not move** | `chaos/src/power.rs` and `bridge/` are not in #238; `cli/src/analog.rs` changes only a test string |
| The decoupling (§1.8), the coverage rule (§1.7), the chain (§1.4) | **do not move** | `eda`, `verify.rs`, `power_parts.rs` and `ir/src/power.rs` are not in #238 |
| `simulate`'s Renode | **gains a refusal** | #238 refuses any Renode build but `1.17.0+20260907gitf1dd1b4af.aet1605` (`RenodeBuildNotPinned`); the probe's Renode must be that build |

P19 held, with one addition: the solved topology also bumps.

### §1.10 `verify`'s weight on a tree board (measured)

- **The run:** the unmodified CLI's `aethelgard verify --spec crates/aethelgard-cli/tests/fixtures/sentinel-03.json`, with no `--scenarios` and no `--fab-out` (the gates it runs today), once.
  - It ran under `capped-run --mem 16G`, with the caps read back as `memory.max=17179869184 memory.swap.max=0 pids.max=8192` (`spike/2304/sh/verify.sh`).
- **Verdict `pass`, exit 0.** 24 gates: ERC, DRC, support, extract, 18 Kani harnesses (`prove`), build and simulate. Findings `[]`. ELF `5756a2e0…ca51`, the same as `generate`/`build` gave (`spike/2304/out/verify-s03.json`, a summary; the full 381,321 B verdict's sha256 is in it).
- **Wall: 1,075 s (17.9 min).**
- **Peak memory:**
  - the scope's `memory.peak` was **3,651,358,720 B (3.40 GiB), 213 ‰ of the 16 G cap**, also read from the host every 5 s (`spike/2304/out/verify-s03-mem.log`);
  - the largest single process was 3,048,708 KiB (2.91 GiB, `getrusage`).
  - It would have fit 8 G (425 ‰).
- **P18:** predicted a peak of 6–14 GiB and a wall of 10–30 min. **The peak was missed (3.40 GiB, below the band); the wall held (17.9 min); the verdict held (pass).**
- **For the build's tier-2 cost:**
  - one `verify` of Sentinel-03 is about 18 min of one shard's 60-minute guard (8 shards; `TIER2_MIN` 190);
  - a `--fab-out` run adds the campaign's scenarios, each a boot of about 9 s plus (a)'s solve (§1.6);
  - a (b) lock-step scenario adds its wall per simulated second (§1.5).
- **`4400g`** does not move these gates' arithmetic. Its trace and specification bumps move the verdict's hashes (§1.9).

### P19 against its result

| id | Predicted | Result | |
|---|---|---|---|
| P19 | `4400g` moves the TIM1 clock, kind 26's values and the versions, not boot-to-`CEN`, the analog side or ngspice | as predicted, read at source on #238. Also moved: the solved topology version (10 → 11), which was not predicted. `cli/src/campaign.rs` is not touched (§1.9) | held, with one addition |

## Findings (house schema: finding · where · type · recommendation · decision-needed)

- **F1 · The specification cannot say whether ESC current passes through J4 and `VBAT` copper · `sentinel-03.json` (J5 has no power pin), `ir/src/motor.rs` L82, `ir/src/power.rs` L1587–1650 · schema gap · declare it with the ESC row (D2): each ESC names its supply, either `board` (a pad on `VBAT`) or `pack` · D2.**
- **F2 · No maker states an ESC's idle current or a pack's internal resistance · §1.3 · data gap · the ESC row and the pack R come from a measurement with its method stated (Scotty's bench), or the campaign halts by name (`INV-06`) · D2, D3.**
- **F3 · `decoupling_picofarads` leaves out a regulator's output capacitors · `eda/src/schematic_power.rs` L431–432, `schematic.rs` L1118–1147 · model error, measured: 10.81 µF counted against 98.81 µF on Sentinel-03's `VDD_3V3`; `dshot600-droop` is refused (`DroopRefused`) once they count · count them (their parts' capacitance, DC-bias where held), in the first tree-board build · D5.**
- **F4 · The board's own declared maximum load, not the ESC's idle draw, sets the R at which a tree board browns out · §1.4 · computed from declared maxima (14.16 W at the fixture's η; collapse at 1.43 Ω with no ESC at all) · a battery-sag scenario on Sentinel-03 is meaningful with no ESC declared · D1, D5.**
- **F5 · Past the collapse, the chain has no operating point and the deck's buck enable cycles through its UVLO hysteresis · (a) I2: 4,630 resets in 1 s, 47,816 events; the bridge's `rail_answer` refuses more than two crossings a level (`analog.rs` L393–398) · model limit: the LMR33630's behaviour there (hiccup) is not stated · the build's scenario check refuses, by name, a declared R at which the static chain has no operating point **or** puts V_bat below a battery-fed regulator's declared shutdown. The second clause matters above about 4.7 A, where V_bat falls below 3.0 V before the collapse (adversarial M2: at 10 A, 0.408–0.415 Ω; at 160 A, 36.4–46.1 mΩ). This is how `check_droop` refuses an unreachable floor · D3, D5.**
- **F6 · An ngspice `external` source adds the callback's value to its `dc` value · libngspice 44.2 (measured) · tool behaviour · any future deck with an external source writes `dc 0` · none.**
- **F7 · `VDD_5V` has no observer in the twin · `aethelgard-twin`, `chaos`, `bridge/`, `models/` (no `VDD_5V`) · coverage gap · D4 option 1 (its consumers' absence) · D4.**
- **F8 · Roadmap L722 names the wrong connector; L696 is done · `roadmap.md` · stale rows · D6.**
- **F9 · `4400g` also bumps the solved topology (10 → 11); the WO's list omits it; `cli/src/campaign.rs` is not shared with `4400g` · #238 · WO correction · none.**
- **F11 · Below U10's 7.0 V `regulation_vin_min`, `VDD_5V`'s band is unstated before any `VDD_3V3` crossing · §1.4 (0.83–0.99 Ω at idle against ≥ 1.27 Ω) · rule consequence (IR's D-B9) · the build refuses below it, and `battery` is covered by a tolerated sag · D5 (Scotty's, if a brownout is to be admitted below D-B9).**
- **F10 · Under lock-step, a reset within one boot cycle repeats while the ESC current decays · (b) T10: 6 a cycle at 10 µs, 2 at 100 µs · model sensitivity: the count rests on the regulator's undeclared hiccup behaviour and the deck's ESC lag · a build counts boot cycles (`CEN` sets) as its loop measure, and declares the regulator's enable thresholds and the ESC's current rise as data · D2, D5.**

## Decisions — each with a recommendation

**D1 · The model for the campaign: (a). (b) waits for the flight loop.**
- **The margin it rests on (§1.4), in amps:**
  - On the generated firmware the only lever is `CEN` (DShot-0 frames, or none). An ESC with no signal still draws (Hobbywing), so the lever is the DShot-0 idle minus the no-signal idle. No maker states either figure.
  - At the WO's weak pack (1.0 Ω), the firmware moves a crossing **only if that lever is at least 1.47 A in total**. The WO's 0.4 A (I1) gives no reset under either candidate (measured).
  - In ohms, at a 0.4 A lever, the firmware moves a crossing only for a declared R of 1.27–1.43 Ω. For context only, since no maker states a pack R and the WO forbids estimating one: that is about 106× a third-party ~12 mΩ for a healthy pack. **The 1.47 A figure carries the margin**, not this ratio.
  - The band's width is set by the unsourced idle figure: at a 2.0 A lever it is 0.91–1.43 Ω, and 1.0 Ω loops (I2, measured).
- **(a) must refuse where it is wrong (F5).** Past the collapse (or below a regulator's shutdown), (a) gives 4,630 resets against (b)'s 24. With that refusal in place, (a) issues a verdict only where the board keeps an operating point under its declared load, and, under D5's rule, only where every battery-fed rail stays stated.
  - There, on the generated firmware, (a) and (b) agree whenever the declared ESC load includes the idle figure. Measured at I1: 0 against 0 at both quanta. §1.5's band runs measured it either side of the band: at 1.20 Ω they agree, at 1.35 Ω (b) loops and (a) chatters.
  - **That condition is D2's:** a scenario that declares no ESC current on a board that drives ESCs would give (a) a pass where (b) loops. So the ESC row is not optional on such a board.
- **Sentinel-03 drives no declared ESC (§1.2).** It has no lever at all. Its battery load is the tree's declared 14.16 W, and it browns out at 1.43 Ω with no ESC (F4).
- **(b) at 100 µs when the flight loop lands.** An armed firmware moves the load to tens of amps, where the crossing comes at about 40–82 mΩ (the 60–127 A sweep). That is a credible weak path, inferred.
  - There the firmware moves crossings, and (a) misses the loop: T10 gives 7 resets then held, against 25 boot cycles.
  - 100 µs keeps the boot-cycle count (24 against 25), with the first assertion 110–140 µs late. The 10 µs reference crossings are themselves within 0.1–3.4 mV of BOR3's typical level (§1.5), so the loop's existence there rests on typical thresholds.

**D2 · The load's source: a power-parts `esc` kind, named by each motor output.**
- **Its shape:**
  - `supply_cells` (range);
  - `continuous_milliamps` (stated);
  - `idle_milliamps` by state, `no_signal` and `disarmed_dshot0`;
  - `throttle_milliamps`, a table of (throttle per mille, mA at a stated V), optional, where any profile that needs it halts without it;
  - `current_rise_ns` (F10);
  - `supply`, either `pack` or `board`, which answers F1, L722 and `1303b`.
  - Each figure carries its `at` citation, or a measurement record with its method.
- **The specification:** `motor_outputs[].esc` names the part, or declares `none` with a source.
- **Who picks:** the part is `4500`'s and Scotty's. For Sentinel-03, Scotty confirms `none`: J5 is a bench header.
- **Before one is picked:** a battery-sag scenario on a board whose motor outputs name no ESC and declare no `none` **halts by name** (for example `EscUndeclared`, naming the output) (`INV-06`).
- **The sourced figure is missing (F2):** no maker states an idle current. So the idle figures come from a bench measurement with its method stated, or the row cannot be filled.

**D3 · Pack and path R.**
- **Healthy: a specification field, `Battery.series_milliohms {value, source}`** (pack plus leads plus J4), at specification 18 after `4400g`.
  - Not a parts row: the pack is chosen per airframe and is not on the board's bill of materials.
  - No maker states pack R (F2). The source is a measurement (Scotty's charger reading, say) plus J4's stated 0.8 mΩ.
- **The fault: a scenario field, `battery_sag.series_milliohms`** (`INV-07`: the fault is authored apart from the board).
  - The scenario is refused by name when the static chain has no operating point at that R, or puts V_bat below a battery-fed regulator's declared shutdown (F5), as `check_droop` refuses an unreachable floor.

**D4 · Coverage.**
- **`battery`:** met by (a)'s new kind, through a rule addition in `verify.rs` `stress`/`coverage`. It is observed through U11 and BOR. **The architect's** (no relaxation).
- **`VDD_5V`: option 1 recommended.** Render a `VDD_5V` excursion as its consumers' absence: J6 `CRSF` and J7 `MAVLINK` RX open, which the serial judge detects. The modelling assumption (a DS-009 peer below 4.75 V is absent) is **the architect's**.
  - Options 2 (a static gate in place of campaign coverage) and 4 (named unobserved, package ships) **change what `INV-03` counts: Scotty's**.
  - Option 3 (unmet, no package) is today's rule.
- **The fixture's `VDD_1V8`:** it can leave tolerance only after BOR3 has reset the MCU (U12 needs 2.227 V, below BOR3's 2.54 V minimum).
  - Counting it covered by that argument would be a static rule standing in for campaign coverage: **Scotty's**.
  - Otherwise it takes option 1 with its own consumers.
- **`4201` F5** (docs-branch roadmap row L840, owned by "`AET-WO-2304` D4, or the next coverage work order"): `3303`'s rule counts `SENSORS` as covered by passing scenarios with no modelled device on the bus.
  - It is the same principle as option 1's caveat: a target counts as covered only if a judge observes something on it (`EXT` has no peer, and today it is named `no device: nothing to fault`, README L924).
  - **Recommendation: the next coverage work order, not this build.** The battery chain does not touch bus coverage. That order should state the rule once for buses and rails alike.

**D5 · The smallest honest step: (a0) + F3 + (a), built for Sentinel-03 first.**
- **What it is:**
  - lift the refusal;
  - count a regulator's output capacitors in the rail's decoupling (F3);
  - add a `battery_sag` kind: the pack R fault, the tree's declared static load, and an optional declared ESC step profile;
  - solve it once by the bridge's chain deck, with crossings through `power::schedule`;
  - refuse a declared R with no operating point, or below a regulator's declared shutdown (F5); this needs the regulators' enable thresholds as power-parts data;
  - **and refuse a declared R that puts V_bat below any battery-fed buck's `regulation_vin_min`** (F11; IR's own D-B9 rule: below it the band is unstated);
  - add the coverage rule for `battery`.
- **Versions after `4400g`:** specification 18, scenario 4, trace 15 (one new kind: the declaration; crossings reuse kind 17). Power-parts goes to 5 for the buck's enable thresholds (F5: SNVSAN3F §8.3.4 states only typical values, so the build holds the stated 3.8 V operating minimum as the conservative edge unless the architect rules the typicals in). The `esc` kind with `4500` is a further bump. The architect dispatches the bench package afterwards (its `verdict_sha256` moves).
- **What D-B9 leaves the first build (F11):**
  - On Sentinel-03, V_bat falls below U10's 7.0 V `regulation_vin_min` at 0.83 Ω (0.4 A) to 0.99 Ω (0 A). That is before `VDD_3V3` can cross BOR3 (≥ 1.27 Ω).
  - So under that refusal, a `battery_sag` scenario on this tree can only be **tolerated**: the pack sags below its declared 9.0 V floor (L695's principle: the quantity moved past its declared tolerance) and every rail stays stated. **A brownout cannot be shown on Sentinel-03 without leaving `VDD_5V`'s band unstated.**
  - The WO's own weak case (1.0 Ω) would be refused at any ESC current.
  - Admitting a declared R below D-B9, with `VDD_5V` named "unstated" in the verdict, would relax what the verdict covers: **Scotty's**.
  - **Recommendation:** the first build refuses per D-B9 and covers `battery` by a tolerated sag (for example 0.5 Ω, V_bat 8.13 V at 0 A, §1.4). Showing a brownout waits for that ruling, or for a tree whose 5 V band is stated lower.
- **It takes L697** (the loaded floor), because the change is in the same function.
- **It does not need L799.** (a)'s largest trace is 2.2 KB once F5's refusal stands. L799 goes with (b), or with the next chaos order.
- **Sentinel-03's package ships with the chain declared** once D4 is ruled for `VDD_5V` (its own coverage row is separate from F11). The verdict says what it does not claim (`RUL-19`'s habit): "the battery's load is the tree's declared static load; no motor current is commanded, so no reboot loop is shown or excluded".
- **Measured cost:** `verify` on Sentinel-03 (§1.10) is the tier-2 base. (a)'s solve takes about 12 s per simulated second at a 10 µs enable step; the boot's 100 ms is inferred at about 1.2 s.

**D6 · Row hygiene.**
- Close **L696**, done by `2302` D5.
- Reword **L722** to "J4 (XT30PW-M, 15 A) against the ESC current, if the ESC is fed through the board (F1)".
- **L697** and **L695's droop half** ride the build.
- **L695's pull-up and capacitance half** and **L723** (ESR on ceramics) go to a static work order. The chain uses only C1's ESR, which is held.

**D7 · Precede or fold: precede `4500`.**
- The WO cites "the architect's note below" for D7. Neither the cached copy nor the WO on the docs branch at `0883d1d2` holds one (diffed: identical). D7 therefore answers from the evidence alone.
- **For preceding:**
  - Sentinel-03 is first (roadmap L367, L375) and Scotty's fabrication quote waits on it (L419–424).
  - Its build needs no ESC, frame or pack choice (§1.2).
  - Its crates are `ir`, `chaos`, `cli`, `eda` and `bridge/`.
  - `4500`'s ruled profile (`2303` D1 (ii)) *is* (a)'s kind, so `4500` reuses it and adds only the `esc` row and a profile.
- **For folding:**
  - One specification and trace bump instead of two. Each moves goldens and the bench package's `verdict_sha256`, which needs a dispatch (the `1605`/`1605c` lesson).
  - `4500` touches the same campaign files.
- **Recommendation: precede.** Sentinel-03's package is on the critical path, and the second bump is a bounded, known cost.

## What was deliberately not built

- No crate, `bridge/`, model, golden, test, workflow or `ci/` change.
- No relaxation of the coverage rule prototyped (D4, §7).
- No sampling option: (c) is named only (`INV-04`).
- (d) is named, not measured.
- No ESC, frame or pack chosen. No figure was estimated where a maker states none: the ESC idle draw and pack R are labelled sweeps.
- No ArduPilot, and no flight loop.
- The deck's bucks' enable is not put inside the deck (F10, §1.5): it is quantised at the boundaries in both candidates, and labelled.
- No re-run with the CLI's witness logging kept: `2303` measured it changes nothing in the samples trace.
- No re-measure on `4400g`, which did not merge during the spike.

## §7 checked

- **"A measurement needs a crate or `bridge/` change":** none did.
  - The probe owns Renode and libngspice, as `2303`'s did.
  - The decoupling probe calls `aethelgard-eda` and `aethelgard-chaos` as path dependencies, outside the workspace.
  - `git diff --stat cd43b207 -- crates bridge models ci .github Cargo.toml Cargo.lock` is empty.
- **"A run is killed, or passes 900 ‰ of its cap":** none was.
  - Peaks: `verify` 3.40 GiB of 16 GiB (213 ‰).
  - The probe runs: Renode up to 463 MiB plus the probe up to 188 MiB, inside 8 GiB, so under 100 ‰ (inferred from the two processes' figures; the scope's peak was not read for them).
- **"No maker states an ESC's or a pack's figure":** none does (F2). Both are swept and labelled, and D2/D3 say the sourced figure is missing. Not a stop.
- **"The only way any candidate meets the coverage rule is to relax it":** no.
  - `battery` is met by (a) through a rule addition.
  - `VDD_5V` has one non-relaxing option (D4 option 1), which needs the architect's ruling on its modelling assumption.
  - Nothing was prototyped.
- **"A candidate turns out to be sampling":** none. (c) is named only.
- **"`4400g` merges and moves a figure":** it did not merge during the spike (#238 still open at the stop, its head moved from `afb91036` to `c07c6ebd`: main `cd43b207` merged in, and `4400g` F5's bench package added. The same three version bumps; still none of `cli/src/campaign.rs`, `chaos/src/power.rs`, `bridge/`, `power_parts.rs`, `verify.rs` or the emitter. `main` was at `cd43b207` at every check).

## Gates (spike)

- **No crate changed:** the fence diff above is empty.
- **`cargo fmt --all -- --check`:** clean on the workspace.
- **Clippy and `cargo test --workspace` were not run.** The workspace is byte-identical to `main` `cd43b207`, so they would test `main`.
- **The probes** (`spike/2304/lockstep`, `spike/2304/decouple`) are `cargo fmt`-clean and build in the pinned image or on the host.
  - The lockstep probe carries one warning, an unused variable in `2303`'s `splice` mode, kept unchanged.
  - `spike/2304/ngspice` is `2303`'s FFI copied unchanged, not `fmt`-clean, as it was not there.
- **Adversarial pass:** a fresh subagent, read-only, attacked the record. It raised three high, five medium and six low findings, and every one is applied above:
  - **H1, H2:** §1.10 and the rail table were missing; both are now in.
  - **H3:** D1's margin is restated in amps, the unsourced 0.8 A edge is withdrawn, and the honest R framing replaces the 1,590× lead.
  - **M1:** the deck-rule column is added and the predictions' 3.8 V wording is noted.
  - **M2:** F5's refusal is widened.
  - **M3:** the millivolt margins at assertion are stated.
  - **M4:** the band runs were made.
  - **M5:** the mirror claim is narrowed.
  - **Low:** P6's wording, F4's naming, `caps.log`, the quantised elements, the bound label, and the order.

  It confirmed every §1.5 number against `run.json`, `actions.txt` and `stats.txt`, and the §1.4 thresholds by an independent recomputation.
- **Reduction pass (diff): −20 net lines of code · 0 duplicates removed · 3 dead paths removed · 0 protected kept · 2 findings below.**
  - **Removed:** the probe's `DUMP` debug block (its only use was finding F6), and two `chain.py` columns that never fired or were unused. `chain.json` is identical apart from the removed keys, `selfcheck` passes, and a re-run of (a) T10 is byte-identical (`e6193e61…`).
  - **Finding:** the deck's load power and duty (`P_BUCKS_W`, `D`, `I3` in `lockstep/src/main.rs`) restate what `chain.py` derives from the parts table.
  - **Finding:** `decouple`'s droop restates `dshot600-droop.json`.
  - Both are left in the spike; in a build, the deck takes them from IR's one derivation.
- **Evidence:** in `spike/2304/out/`, made by the scripts in `spike/2304/sh/`.
  - The traces (up to 5.1 MB) are not kept; their sha256 is in `spike/2304/out/trace-sha256.txt`.
  - Oversized logs are trimmed, each with its full sha256 beside it.
  - Datasheets are in `~/.cache/aet-ds/aet-2304/` on Furnace, never in the repository.

**STOPPED at §2**, 2026-11-19, for the architect's ruling on D1–D7 (Scotty's where D4 marks it). The PR is a record, to be merged as the record (FEEDBACK plus `spike/2304/`) after the ruling.

