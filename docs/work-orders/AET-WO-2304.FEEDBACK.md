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
