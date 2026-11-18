# AET-WO-1303b — FEEDBACK

Labels: **[measured]** by a run named beside it, its output under `spike/1303b/out/`; **[read at source]** from a file and line named beside it; **[inferred]** from reasoning, neither read nor run.

## 0. Status: STOPPED at §1.1, as the work order requires

- **Coverage is below every pin on every board, with the router's remedies applied [measured]. That is the ruling's own D5 (b) trigger** (comment 6888926396: "If coverage on Sentinel-02's supply pins is still below every pin with the router's remedies applied, (b) the certified discrete flow is ruled in then").
  - VDD_3V3: 0 of 16 pins on the 2-layer power fixture; 10 of 16 on the power fixture at four layers (12 with GND classed); 6 of 16 on Sentinel-03.
  - GND: 0 of 11 on both power fixtures; 10 of 10 on Sentinel-03.
- **Where pins are bounded, the bounds are too loose to gate [measured].**
  - They run 142–892 mΩ, dominated by the plane hops, and each hop's bound is 4.1–87× the exact two-hole sheet (median about 12).
  - Even Sentinel-03's best VDD and best GND pin sum to 313 mΩ: 206 mV at 658 mA, over the 165 mV tolerance.
  - So a gate on (a) alone would fail every board on its bound's looseness, not on its copper.
- **A §7 condition also fires:** the 2-layer ground return has no honest bound from the copper the reader can read. It runs through a `B.Cu` zone that is unfilled in the emitted file (§3).
- **Not built:** the discrete flow, any field, any gate. This branch adds FEEDBACK and the evidence (`spike/1303b/`) only.
- **For the ruling:** D5 (b), the 2-layer ground (§7), the ground terminal, the fields and their schema version, the verdict's shape, the load, the USB pair and the §7.1 rows (§5).

## 1. What was measured, and how

**The boards.** Every specification in the corpus with a regulator, emitted by `main` `002dae07`'s release CLI:
- **`sentinel-02-power`,** the power fixture at two layers, as committed (`crates/aethelgard-ir/tests/fixtures/`);
- **`s024l`,** the power fixture at four layers. No such fixture exists (no test turns `power_tree()` into four layers [read at source, `crates/aethelgard-cli/tests/common/power.rs`]). It is constructed: the committed fixture with Sentinel-03's `copper_layers`, `layers` and `stackup` (`spike/1303b/build-fixtures.py`);
- **`sentinel-03`,** the corpus's 4-layer board with the same regulators.

**The router's remedies, as the specifications declare them [read at source]:**
- Both fixtures already put VDD_3V3 in a `SUPPLY_MCU` class: 0.2 mm tracks and 0.7/0.3 mm vias (`AET-WO-1308`'s via-pad option).
- GND has no class. The `-gnd` variant adds it to the same class (`build-fixtures.py`).
- Wider supply tracks were not tried: 1308 measured that 0.25 mm loses coverage, because it moves the stitching (1308 FEEDBACK B4).

**The terminals (D4):**
- **VDD:** the pad where the regulator's current enters the net. U11 is a buck (LMR33630) with no pad on VDD_3V3, so that is its inductor's output pad, `L2.2`.
  - At DC, that node is the net's highest potential: capacitors carry no current, and every other pad is a sink.
  - So for any point s on the net, V_s − V_pin ≤ V_L2 − V_pin ≤ R(L2, pin) × the net's total current (P4.8). That covers the feedback divider's sense point too (R20) [inferred].
- **GND:** U11's ground pads (U11.1, and the thermal pad U11.9). The choice is a question (§5.3).

**The bound.** `1303`'s closed-form pieces, unchanged in their formulas, with the changes these boards need (`spike/1303b/oracle.diff`, each commented at its site):
- The reader keeps circle and oval pads as copper the plane must clear. The spike's reader refused them, and these boards have connectors.
- Every plane obstacle clears by its own size: d/2 + 0.2025 mm (P4.7's measured fill clearance), rounded up to 0.01 mm, plus 0.125 mm (F9). That covers each foreign via, and every through-hole and NPTH pad of any net. Before, every obstacle cleared as a 0.6 mm via.
- **One via geometry per run.** A run uses the net's vias of one geometry: VDD_3V3 0.7/0.3; GND 0.6/0.3, or 0.7/0.3 in `-gnd`.
  - The other own-net vias are the 0.554/0.254 vias in the regulators' thermal pads.
  - Their copper is not used (Rayleigh), and their drilled holes stay obstacles.
  - A track ending at one is dropped, so no path crosses its hole as a plain point.
- **A pass through a pad keeps off every drilled hole** (any via, any through-hole pad) by w/2 + r_hole + the hole offset.

**The regression check [measured].** The modified oracle, run with the spike's own `run_paths2.py` on the spike's boards, reproduces `spike/1303/s4/paths.out` byte for byte (`out/regress-spike-paths.out`). The changes are neutral where the old boards have none of the new features.

**The conditions:** `spike/1303/s4/conditions.json` as the spike stated them.
- ρ_max is IACS × (1 + α·105) × k 1.181, at 125 °C;
- outer copper ≥ 30 µm, plane copper ≥ 15 µm, plating ≥ 18 µm, all conditions;
- barrel lengths from the declared finished thickness (0.34–2.0 mm at two layers, 1.25–1.851 mm at four).

Each run was made with the hole concentric and at JLCPCB's ±0.075 mm.

## 2. Coverage [measured]

MCU supply pins (U1) bounded, of all, as `concentric / ±0.075 mm`:

| Board | VDD_3V3 as declared | VDD_3V3 with GND classed | GND as declared (0.6/0.3 vias) | GND classed (0.7/0.3) |
|---|---|---|---|---|
| `sentinel-02-power`, 2 layers | **0 / 0 of 16** | 0 / 0 | **0 / 0 of 11** | 0 / 0 |
| `s024l`, power fixture at 4 layers | **10 / 10 of 16** (315–434 / 357–506 mΩ) | 12 / 12 (315–477 / 357–578 mΩ) | **0 / 0 of 11** | 0 / 0 |
| `sentinel-03`, 4 layers | **6 / 5 of 16** (171–697 / 215–509 mΩ) | 5 / 5 (172–262 / 216–333 mΩ) | **10 / 9 of 10** (142–529 / 400–893 mΩ) | 10 / 10 (162–584 / 420–1116 mΩ) |

**Each pin's bound is exactly R ≤ ρ_max·(a/t_out + b/t_plane + c/A(t_p)).** a, b and c are fitted on four thickness sets per path, with worst residual ≤ 4×10⁻¹⁵, and printed per pin. The milliohm figures evaluate that at the conditions above.

**Refused pins, with the piece that refused them** (`out/cov-0.out`; "the search" is the flow-rule Dijkstra, retried after banning each failed piece):

| Board, net | Refused | Piece |
|---|---|---|
| `sentinel-02-power` VDD_3V3 | 16 | **via entry and pad pass.** Every pin is connected (16 of 16 reachable when the flow rules are ignored), but each path the search found failed at a via entry or at a pass through a pad. The bans then exhaust the search: 1 to 40 pad passes, and 1 via entry, per pin |
| `sentinel-02-power` GND | 11 | **no copper the reader reads.** Each pin's stub ends at a stitching via whose other side is the `B.Cu` zone, unfilled in the file. That is §3 |
| `s024l` VDD_3V3 | 6 | via entry (1 to 3 per pin; one pin also 13 pad passes) |
| `s024l` GND | 11 | **pad end at U11.1:** the tube cannot leave U11.1 at any width. U11.9, the thermal pad, has no edge: its own vias are the excluded 0.554/0.254 ones, and no track reaches it. So every GND row rests on one entry |
| `sentinel-03` VDD_3V3 | 10 | via entry (6: 2 to 5 per pin, one also a pad pass); **no plane edge** (4: each pin's stub reaches one via, and that via has no admissible plane hop) |
| `sentinel-03` GND | 0 / 1 | at ±0.075 mm, one pin: no path |

**Looseness, measured on every plane hop the bounded paths use** (`D`, β, the hop's bound, and the exact two-hole sheet (ρ/πt)·acosh(D/2r_d), printed per hop):
- the bound is **4.1–87× the exact value, median 9.4–12.3**, over 84 to 149 hops per run;
- the plane term ρ·b/t_plane is most of each bound. U1.6 on Sentinel-03, for example: 627 of 697 mΩ.

**The drop this implies** (an illustration at 658 mA, not a gate; the load question is §5.5):
- Sentinel-03's best VDD and best GND pin sum to (171 + 142) mΩ → 206 mV;
- its worst pair sums to (697 + 529) mΩ → 806 mV;
- the tolerance is 165 mV.

Whether the board or the bound is at fault, (a) cannot tell: the exact two-hole figures say most of it is the bound.

**Two more facts behind the GND rows:**
- **The In1 barrel is priced at the board's full 1.851 mm.** That is the declared finished maximum: the stackup declares dielectric minima only, so no shorter maximum depth to In1 exists to use.
- **The spike's own pieces still hold.** Every bounded path was recomputed from its own geometry, as in the spike.

## 3. The §7 condition: the 2-layer ground return

At two layers GND is F.Cu stubs and stitching vias into a `B.Cu` zone. The emitted file holds the zone unfilled; the DRC gate refills it in the image [read at source, 1303 P4.7].
- The reader therefore has no copper to bound any GND path through it, and on this board every path goes through it (11 of 11).
- That is the work order's §7: "a quantity the gate needs has no honest bound from the copper the reader can read: stop". The work order names two remedies: declare it a condition, or drop the gate.

**Options, for the ruling (none chosen):**
- **(i) Read the fill.** `extract` reads the zone as KiCad refills it, from the DRC gate's prepared copy in the image. The supply gate would then need KiCad, which `extract` does not today.
- **(ii) Declare it.** At two layers the ground return is a stated, unverified condition, and the gate bounds VDD alone. That under-claims, as D7 says.
- **(iii) Refuse it.** The supply gate refuses a 2-layer board by name (`GroundReturnUnread`), until (i) or a ground plane.

## 4. What the build would move (the goldens)

- **Sentinel-01's bounds and `lines`:** byte-identical. Sentinel-01 has no regulator, so no supply entry is computed.
- **The extract verdict's keys:** `supplies` and `pairs` beside `lines`. No committed file or test pins the verdict's bytes: tests read `verdict["lines"]` by key [read at source, `crates/aethelgard-cli/tests/*.rs`].
- **`verify`** copies extract's `conditions` strings into the bench package's `FAB-NOTES.md` [read at source, `verify.rs` L1590]. So the build must add no condition to a board without a regulator, or that package moves. Campaign hashes cover the twin's states, not extract's output [read at source].
- **The specification's fields: two ways, for the ruling.**
  - **(i) Schema 17, every new key required** (`null` where nothing reads it, as `1308` F1 did). This moves:
    - the 25 specifications at version 16;
    - Sentinel-01-minimal's sha256, in `boards/sentinel-01/` (8 files), `aethelgard-hwdef`'s two fixtures and `aethelgard-svd-hal`'s `serial-four` UI test;
    - Sentinel-01's sha256, in `boards/sentinel-01-bench/` (8 files) and the bench package's `MANIFEST.json` and `FAB-NOTES.md`. The package is rewritten by CI's `bench-package` dispatch.

    No KiCad file or bound moves.
  - **(ii) No version bump:** optional fields, refused by name when a gate reads one and it is absent (`INV-06`), never defaulted. Nothing above moves; the verdict gains its keys only.

## 5. The proposal, for the ruling

1. **D5 (b): rule it in.** Coverage and looseness both show the trigger [measured, §2].
   - The (a) pieces stay as its exact-case tests: bar, annulus, barrel, two-hole sheet and the bend bracket, as the ruling planned.
   - The flow: per-layer sheets with holes, as `1303` S3's stream-function dual, joined by barrels.
   - The discrete flow is not built before the ruling.
2. **The 2-layer ground (§3):** (i), (ii) or (iii).
3. **The ground terminal (D7).** Return current leaves the GND net at the regulator's ground pads (U11.1, U11.9) and at the battery return (the input connector's ground pad, and the input capacitors' grounds). Which is the terminal is a question:
   - the pad the regulator references its feedback to (R21's ground);
   - the regulator's ground pad;
   - every sink with a multi-terminal bound.

   No choice made.
4. **The fields** (§4 (i) or (ii); names and units proposed):
   - `board.stackup.outer_copper_min_um`: `{value, source}`, read by any supply path on an outer layer;
   - `board.stackup.plane_copper_min_um`: `{value, source}`, read by a plane hop;
   - `board.stackup.plating_min_um`: `{value, source}`, read by any barrel;
   - `board.stackup.copper_temperature_max_c`: `{value, source}`. For Sentinel-02 that is 125 °C, the load's Tj (DS12117 Table 120, D6);
   - `power.drop_limits[]`: `{domain, max_millivolts, source}`, the VDD + VSS limit per domain. 165 mV for 3.3 V ±5 %, cited.
   - Optionally `board.stackup.dielectrics[].max_thickness_um`, which would let a barrel to In1 be priced at its own depth instead of the whole board (§2).

   k = 1.181 (IPC-MF-150F §3.8.1.2), IACS and α stay cited constants in the module's documentation, with k on plated copper a stated condition (D6). Each field is refused by name when the gate reads it and it is absent, never defaulted (`RUL-14`: a minimum, never a nominal).
5. **The load is the net's total current** (P4.8 holds for every sink on the net, not one).
   - `power.loads` has no VDD_3V3 entry today [read at source]: its entries are VDD_5V and VDD_1V8.
   - Proposed: the gate reads `power.loads`' entry for the domain, `{domain, milliamps, source}`, refused by name when absent. Its value is the sum of every sink's cited maximum: the MCU's 658 mA (DS12117 Table 120) plus the rest.
6. **The verdict's shape.** `supplies` beside `lines`, one entry per domain:
   - `{domain, source: {reference, pad}, return: {reference, pads}, load_milliamps, limit_millivolts, pins: [...], returns: [...], drop_bound_millivolts, conditions}`;
   - each pin is `{pin, net, r_bound_microohms, coefficients: {a, b, c}}` or `{pin, net, refused: {kind, piece}}`;
   - `drop_bound_millivolts` is (the largest VDD bound + the largest VSS bound) × load, and `null` with a finding when any pin is refused. The MCU ties its VDD and VSS pins inside, so the worst pair is the bound [inferred].
7. **The USB pair (D2).** `pairs` beside `lines`: each conductor of a declared `diff_pairs` entry gets `c_bound_femtofarads` from the existing machinery, against its net's `net_constraints[].max_capacitance_femtofarads` (25,000 for full speed). The mismatch is named in each entry as "unverified: a stated condition of symmetric pair routing (1303 D2)". It is tested on `pair-standin.json`. No new field.
8. **The §7.1 rows:**
   - **A through-hole barrel priced as a via's:** the barrel piece takes the pad's drill as r_d. With (b), a through-hole pad is a barrel like any via.
   - **A relieved pad on a supply path:** refused by name (`ReliefUnpriced`) until its spokes are priced (`1309` D2). Any pad with a thermal-relief zone connection on a bounded path is refused.
   - **A cited current-to-width source:** re-owned. The R bound is this order's quantity; a heating limit per width (IPC-2152) is a different gate, so a backlog row.
   - **A stackup bound below the fabricator's minimum:** the new minima fields validate `minimum ≤ copper_max_um`. `copper_max_um` accepted down to 1 then cannot undercut a declared minimum.

## 6. Record

- **Base:** `main` `002dae07`. Branch: FEEDBACK and `spike/1303b/` only. The build deletes `spike/1303b/` once cited.
- **Oracle:** `spike/1303/s4` at `d5bc8416`, unchanged in the tree. The copy that ran is `~/.cache/aet-1303b/s4`, and its changes are `spike/1303b/oracle.diff`.
- **Host only.** No KiCad run was needed for §1.1: the boards are `emit`'s output, and P4.7's fill measurement stands as the spike's.
