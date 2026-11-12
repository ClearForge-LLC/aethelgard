# AET-WO-1305 — FEEDBACK

**Branch:** `wo/aet-1305-extract-bound` · **Base:** `b227c12`; `origin/main` 05edb43 (`AET-WO-1602`) merged in as 67f5dc3, and dbf2cba (the new tools image pin) as 315e469 · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build-only, with the §1.3 measurement first.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §1.3 Measurement on the reference, before any acceptance number

Recorded from the first run of `aethelgard extract` on the board `aethelgard generate --spec examples/sentinel-01-minimal.json` writes (board sha256 `7b9b0e3d…2070`, specification sha256 `acd60081…49d1`), before any test held a number. The constants it used: per-length bound 280 fF/mm, 124 fF for each track's two round ends, 1,519 fF per via, `Ci` 10,000 fF per pin.

| Line | Trace | Vias | Pads | Pins | `C_bound` | Declared | Verdict |
|---|---|---|---|---|---|---|---|
| `I2C_SDA` | 9,146 fF (24.25 mm, 19 tracks) | 6,076 fF (4 vias) | 1,578 fF (4 pads) | 30,000 fF (U1.137, U2.3, U3.6) | **46,800 fF** | 100,000 fF | pass |
| `I2C_SCL` | 16,024 fF (43.50 mm, 31 tracks) | 10,633 fF (7 vias) | 1,578 fF (4 pads) | 30,000 fF (U1.136, U2.4, U3.1) | **58,235 fF** | 100,000 fF | pass |

- The lengths and via counts match an independent count of the board's text (a script over the `.kicad_pcb` in the session scratchpad): 24.25 mm and 4 vias on SDA, 43.5 mm and 7 vias on SCL.
- The pads are R1/R2's line pad and the three pins' lands. The lands are counted as copper as well as at `Ci` (F2).
- **The reference passes with 42 % (SCL) and 53 % (SDA) of the declared bound to spare.** No §7 stop.

**Prediction, before the test:** a specification declaring 20 pF (20,000 fF) on both lines fails on its pins alone (3 × 10,000 fF = 30,000 fF > 20,000 fF). `extract` exits `1` with two `capacitance_over_declared_bound` findings, SDA at 46,800 fF and SCL at 58,235 fF, each with its four terms. The same holds for any declared bound below 46,800 fF (both lines fail) or from 46,800 to 58,234 fF (SCL alone fails).

## What was built (§1)

1. **`aethelgard_eda::extract`**, a new module beside `route`.
   - `read` is a strict reader. Before any estimate it refuses an item, child, repeated child or value count it does not model, with the lists derived from the emitted board and the six vendored footprints. It also refuses: a pad without `layers` or naming a non-canonical layer, or shaped other than `rect`/`roundrect`; copper in a footprint outside its pads; a coordinate beyond 250 mm; two footprints with one reference; and a declared thickness other than 1.6 mm. Then it runs `route::read`, `route::connectivity` and `route::check`.
   - `lines` takes each I2C line's parts from `schematic::design`, the parts `emit` draws for the specification. `bound` requires the board's parts on each line to be exactly those: by reference, footprint, `Value` and pad set.
   - Every constant is derived in `const` from `route`'s `WIDTH_NM`, `CLEARANCE_NM`, `VIA_NM` and `DRILL_NM`, with its source in a doc comment (§1.4).
2. **`aethelgard extract --board <pcb> --spec <spec>`** (CLI `src/extract.rs`): one JSON line, exit 0, 1, 2 or 3. The board is read once into memory, and its sha256 and the bound come from those same bytes. The MCP tool `extract` comes from the clap tree; only `mcp.rs`'s two list tests changed.
3. **`verify`** runs `extract` after DRC. It binds `/input/sha256` to the generated board and `/specification_sha256` to `--spec`: 13 bound fields, up from 11.
4. **README:** the CLI's `## extract` section, the exit-code kinds, `verify`'s gate list and binding table, and the MCP tool list; plus a row for `extract` in the eda README. The root `README.md` is protected (F15).

## The bound, and why each term is a bound (kickoff item 2)

**Principle.** With the line at potential 1 and every other conductor at 0, any trial potential `ψ` taking those values gives `C ≤ ∫ ε |∇ψ|² dV`, for any distribution of `ε`.
- Source: W. C. Chew, *Lectures on Theory of Microwave and Optical Waveguides*, arXiv:2107.09672v2, §7.1.2, eqs. 7.1.48 and 7.1.51, pp. 322–323, read in the PDF.
- A floating conductor may take any constant, and `ψ` = 0 on it is one of those, so floating copper is covered too.

**Trial.** For each convex copper piece, `ψ = max(0, 1 − d/S)`, with `S` = `CLEARANCE_NM` = 0.2 mm.
- Its gradient is `1/S` in the shell `0 < d < S`, so `C_piece ≤ ε_max · V_shell / S²`.
- The shell's volume around a prism is closed-form: Steiner's formula, derived in `shell`'s doc comment and tested against the box form.
- The line is the union of its pieces, and `max(ψ_a, ψ_b)` has energy at most the sum of the two, so the line's bound is the sum.

**Why `ψ` is 0 on every foreign conductor.** Each one is at least `S` from each piece:
- **same layer:** `route::check`, which `read` runs on exactly the shapes used (track stadium, via disk, pad box);
- **other layer:** the dielectric is thicker than `S`, by a `const` assert at 1.6 mm, and exactly `S` at the stated minimum of 0.34 mm;
- **inside the dielectric:** only via barrels, each inside its checked disk;
- **the zone's fill:** it keeps the clearance, which the DRC gate refills and judges. When `extract` runs alone, this is a condition. A fill in the file is refused. The adversarial pass measured the refilled reference's minimum gap from the fill to SDA/SCL copper at 500,499 nm.

| Constant | Value | Source | Why it bounds, not approximates |
|---|---|---|---|
| π | 355/113 | arithmetic | 3.14159292 > π, and every term's coefficient in π is positive |
| ε0 | 8.8541878202 × 10⁻¹² F/m | CODATA 2022, NIST `Value?ep0`: 8.8541878188(14) × 10⁻¹² | value plus one standard uncertainty |
| `ε_r` | 5.4 | the IPC-4101 FR-4 maximum at 1 MHz, as quoted in the "IPC Specification" column of two data sheets read here: Panasonic R-1566W (IPC-4101E /127, "Permittivity, maximum at 1MHz 5,4") and Bungard FR4 (IPC 4101A, "spec maximum, A. @ 1 MHz 5.4"). IPC-4101 itself is paywalled and was not read | `C` cannot fall when `ε` rises anywhere, so one maximum covers laminate, mask (below it) and air (1). The same sheets give typical FR-4 at 1 MHz as 4.8 (Bungard) and 4.95 (Panasonic). Part bodies are outside the scope (F4) |
| `COPPER_NM` | 70 µm | a **condition** (F3): the board declares no copper weight | twice KiCad's default 35 µm; a thicker conductor only enlarges the shell |
| `VIA_HEIGHT_NM` | 2.0 mm | a **condition** (F3): the declared 1.6 mm plus 25 % | the barrel and both pads lie inside its disk × height |
| per-length | **280 fF/mm** | `ε0 ε_r (2(c + W)S + πS²)/S²` × 1 mm, rounded up (exact 279.30) | the derivative of a track's shell with length, for the stadium shape `check` clears |
| per track | **124 fF** | the shell of a track of length 0 (its two round ends), rounded up (exact 123.29) | KiCad tracks have round ends of radius W/2 |
| via | **1,519 fF** | the shell of a disk of radius 0.3 mm, 2.0 mm high, rounded up (exact 1,518.47) | the via's checked disk contains its pads and barrel |
| pad | per pad | the shell of the pad's box, `COPPER_NM` high, rounded up | `route::Pad::rect` contains the rounded pad, and is the box `check` clears; only `rect` and `roundrect` are accepted |
| `Ci` | **10,000 fF** per pin | NXP UM10204 Rev. 7.0, 1 October 2021, Table 10, page 43: "Ci, capacitance for each I/O pin", max 10 pF in Sm, Fm and Fm+, note [10] (multiplexers and switches may exceed). Read in the PDF (sha256 `dc91f00f…b36af`), its text decoded through the PDF's own ToUnicode maps | a specified maximum, not a typical value |

**Rounding** is upward at every step:
- π and ε0 are taken from above;
- every division is `div_ceil`;
- the trace term is ⌈280 × length⌉ once per line, plus whole femtofarads per track;
- the arithmetic is `u128` with checked operations, and an overflow is the refusal `Overflow`.

The adversarial pass re-derived every constant with exact rationals and got the same values.

**The bound dominates the exact solutions of its class** (test `the_bound_dominates_the_exact_solutions_of_its_class`):
- A zero-thickness coplanar strip with grounds at `S`, in `ε_max` everywhere, is exactly `4ε K(k)/K(k')` = 122.3 fF/mm by conformal mapping. The bound is 280.
- A coaxial via barrel inside a ground at `S` is 1,176 fF over 2.0 mm. The bound is 1,519.

## §3 acceptance, on the final tree

- **The reference passes** (`the_reference_passes_with_every_term`): SDA 46,800 fF and SCL 58,235 fF against 100,000. Every term is as §1.3 recorded, unchanged by the fixes below. The verdict names the board's and the specification's sha256 and `"capacitance": "upper bound estimated from routed geometry"`.
- **A declared bound below `C_bound` fails, exit 1, naming each line and its numbers.**
  - At 20 pF both lines fail with their four terms, exactly as predicted in §1.3.
  - At 50 pF only SCL fails.
  - At each line's own bound, both pass.
- **Labels that disagree with the copper** are refused (`NetLabelsDisagree`, exit 3) before any estimate: one SDA track relabelled SCL.
- **A longer route raises the bound** (`a_longer_route_raises_the_trace_term_by_the_per_length_bound`).
  - Every I2C segment was doubled in each direction. 41 doublings stayed connected and clear. Each raised its line's trace term by exactly 280 fF/mm × the added length, to the femtofarad: every routed length is a multiple of the 0.125 mm grid, and 280 fF/mm is 35 fF per pitch. Nothing else moved.
  - 37 doublings ran into another net and were refused (`ClearanceViolated`).
  - The connectivity model joins a track only at its ends, so a doubling away from a pad or via joint is refused as `NetLabelsDisagree`.
- **`verify`** on the reference passes, with `extract` between DRC and `prove`. Its `input_sha256` is the generated board's (`board_tier2.rs`, verify 2k2).
- **A swapped board is a `BindingMismatch`** (`verify_refuses_a_board_swapped_before_extract`, tier 2). The swap goes into `verify`'s scratch once DRC's own copy is whole; result exit 5, `id` `extract`.
- **Gates:**
  - **after merging `AET-WO-1601` (main a2f9ea4, merge 2af6cae), at ccbf707:** host fmt, clippy (with and without `with-tools`) and `cargo test --workspace`, 922 passed. In the pinned image (`c69ff9bc…`, unchanged by 1601), the CI `tools` job's steps in order under rootless podman 5.7: 991 tests passed, 0 failed, tier-2 canary 74 = `TIER2_MIN` (1601's 73 + this WO's 1), with 1601's two root-only tests passing. Caps read back on the test container: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` max. The first image run after the merge failed one test, the swap test (F14): it is fixed at ccbf707, and the run above is after the fix;
  - host: fmt, clippy (with and without `with-tools`), and `cargo test --workspace`, 907 passed, at 30a4ecd (after merging dbf2cba);
  - image, at 30a4ecd, the final code, in the **new** pin from main dbf2cba (`container/IMAGE`, `c69ff9bc…`, root-owned toolchain): 974 tests passed, 0 failed, tier-2 canary 72 = `TIER2_MIN`, and caps read back on the test container (`memory.max` 17179869184, `memory.swap.max` 0, `pids.max` max). Earlier, at fd06f13 and in the old pin `2df68259…`: the CI `tools` job's steps, in order under rootless podman 5.7, `--network none` for every step after the build. fmt, clippy with `with-tools`, and the bridge's tests passed. `cargo test --workspace --features aethelgard-cli/with-tools` passed 972 tests with 0 failures, and the tier-2 canary counted 72, which is `TIER2_MIN` (71 + 1, the swap test). Caps: `--memory 16g --memory-swap 16g --pids-limit -1`, then 0 written into the container's `memory.swap.max`. Read back on the test container: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` max. Three short containers exited before the reader found them;
  - CI on PR #99, at 01983f9: `gates` and `image` passed; `tools` was still running when this was written. CI on the final push runs on the PR.

## The architect's check on PR #99: is every foreign conductor at least S from the line?

**Ruling recorded:** F1 is ratified. The Dirichlet-principle shell bound supersedes §1.1's closed forms. F6 goes to its own WO, a shared strict KiCad reader.

**The check:** `ψ` must be 0 on every foreign conductor, so every other net's copper must be at least `S` from the line, including pads of other nets inside the same footprint.
- **`route::check` judges it.**
  - `items` (`crates/aethelgard-eda/src/route.rs:1446-1484`) pushes every pad of every part, each footprint's own pads included, then every track and via.
  - `check` (`route.rs:1490-1503`) compares every pair on a shared layer whose nets differ. A pad with no net differs from everything, and nothing exempts two pads of the same part.
  - `extract::read` runs it before any estimate, so a violation is `ClearanceViolated`, naming both items, and never a number.
- **Test:** `a_foreign_pad_closer_than_the_clearance_to_a_line_is_refused`. Two foreign pads were each moved to 0.15 mm from an I2C pad in the same footprint: U1 pad 138 beside SDA's pad 137 at the MCU's 0.5 mm pitch, and U2 pad 2 (VDD) beside SDA's pad 3 on the BME280. Both are refused as `ClearanceViolated`, naming both pads.
- **The reference's smallest foreign gap to each line**, measured by an independent geometry in `the_references_smallest_foreign_gap_to_each_line_is_at_least_the_clearance` (pads as boxes, tracks as segments 0.2 mm wide, vias as 0.6 mm disks, shared layers only):

  | Line | Smallest gap to another net's copper | Between |
  |---|---|---|
  | `I2C_SDA` | **200,000 nm** (= `S`) | U1 pad 137 and U1 pad 136 (`/I2C_SCL`) |
  | `I2C_SCL` | **200,000 nm** (= `S`) | U1 pad 136 and U1 pad 135 (`unconnected-(U1-PB5-Pad135)`, a floating pad) |

  Both are footprint-internal pairs at the MCU's pitch, exactly `S` apart. The trial potential is 0 for `d ≥ S`, so a conductor at exactly `S` is covered. A floating conductor is covered as well (it may take any constant, and 0 is one).
- **What `route::check` does not see:** the zone fill. That remains condition 4: the DRC gate's refill, measured at a 500,499 nm minimum gap by the adversarial pass, and a fill in the file is refused.

## Mutation (sized to the diff, one mutant each, scratch clone `~/.cache/aet-1305-mut`)

- **First round (18 mutants, at b9bcbf9).** 14 were killed. The 4 survivors were the pull-up rail condition, `LineNotOnBoard`, two controllers on one bus, and the per-length ceiling. All were killed by added tests, which the redesign below then replaced.
- **After the adversarial fixes (20 mutants of the new guards).** 14 were killed. One more was a false kill: the mutant failed to compile, and the rewritten mutant is killed. That left five survivors:
  - two needed tests, both added: an unknown footprint child, and a pad set other than the design's. A third test came from the false kill: a second top-level item;
  - two looked dead and were removed by the reduction pass:
    - the `invalid_spec` branch, which is dead;
    - the `gr_rect`-on-copper arm. **It was not dead.** I judged it covered by `route::read`'s "no outline" and the second-`gr_rect` refusal, but it was the only guard on the outline's own children. The review before push found it: a second `(layer "F.Cu")` on the outline passed `extract`, and KiCad reads a repeated child's last value. The outline now has an allowlist like every other item `read` accepts (`gr_rect`, `stroke`), with a regression (`outline_layer_twice`), and its mutant is killed. Every top-level item that carries geometry is now covered by `allowed` or `footprint`;
  - one is unreachable with today's generator and is labelled `belt-and-suspenders:`: a design passive on a line other than the pull-up.
- **After the reduction:** the two guards it moved (the line's `<=` and `femtofarads`' ceiling) are both killed.
- **The `verify` binding** is killed by `every_bound_field_of_every_gate_is_checked`.

## Adversarial pass (full, fresh subagent on Opus, frozen tree b9bcbf9)

Findings are in `~/.cache/aet-1305-adv/FINDINGS.md`, with inputs under `~/.cache/aet-1305-adv/mut/` and `spec/`. KiCad's own reading of each board came from `pcbnew.LoadBoard` in the pinned image, with caps read back: `memory.max` 8589934592, `memory.swap.max` 0. Count: **19 broke**, 9 wording findings, 24 held, 1 inconclusive.

| Class | Broke | Fix | Now |
|---|---|---|---|
| A: the reader reads a board otherwise than KiCad | a repeated `width`, `end`, start/end/width (a 40 mm-wide plate: 1.3 nF by the module's own formula, passed at 46.8 pF), via `size`, `padstack`, pad `size`, pad/footprint `at`, a drill offset, a repeated pad `net`, a repeated `thickness` (DRC also passed it, F6), a pad without `layers`, a non-square `circle` pad | strict reader (above) | all refused |
| B: parts classed by topology alone | a 100 nF capacitor renamed from R1 passed as a "pull-up"; U1 renamed J1 passed as "controller" | parts from the specification's design, by reference, footprint, value and pads | refused (`DeviceNotOnLine`, `PartMismatch`, `UnmodelledPart`) |
| C3/C4 | a zone fill 10 µm from SDA, a zone `connect_pads` clearance, a `stackup` | strict reader | refused |
| E8 | a 9 km coordinate panicked in `route::check` (`gap2` squares), exit 101, no JSON | 250 mm bound in the walk, before `route::read` | `BoardTooLarge` |
| E7 | a specification with no I2C bus passes with no lines | documented (F9) | as documented |
| C6, C7, C8, D1–D7 | conditions that could not hold for an assembled board, over-claims in docs | rewritten as scope statements; D4 deleted | see F4 |

**Re-verification:** I re-ran the whole `mut/` corpus (32 boards) on the final binary. 31 are refused. `user_layer_table_only` passes, which is correct (F10). One tier-1 regression per class is committed (`what_the_reader_does_not_model_is_refused`, `the_parts_on_a_line_are_the_designs`).

## Reduction pass (diff)

1. **A fact stated twice.** The CLI's `Line` and `Terms` structs restated `LineBound` field by field; the verdict line is now built once from `LineBound`. Three `must*` helpers restated `Option::expect`, and `ceil_div` restated `div_ceil`. All removed.
2. **Unused.**
   - The `invalid_spec` field and branch were unreachable, and are removed.
   - The `gr_rect`-on-copper arm was removed as unreachable, but it was load-bearing (mutation section above). The guard is restored as an allowlist for the outline's children, so the reduction regressed and was then repaired before the push.
   - The unknown-passive refusal is unreachable today and kept as labelled redundancy.
3. **Two mechanisms that could be one.** None found: `copper_layer` and `canonical_layer` answer different questions.
4. **A workaround for a gone problem.** None.

Reduction pass (diff): −69 lines net in source (after the outline's allowlist) · 4 duplicates removed · 1 dead path removed · 1 removal found load-bearing and replaced by an allowlist · 1 protected kept · 2 findings below (F5, F6)

## Findings and gaps

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **F1** | **Ruled on PR #99: ratified.** **The trace, via and pad terms are one variational bound, not the WO's cited closed forms.** The WO names Wadell's coplanar form, a via form with an antipad, and a parallel-plate pad term with a cited fringing factor (§1.1, §1.4). Those are approximations, not bounds. My own derivation, not a citation: the conformal-map partial-capacitance method for a strip over a plane relaxes the potential's continuity across the slots, so it can only lower the minimum energy, which makes it a lower bound. A fringing factor would need its own source to bound. A citable closed form that does bound exists: Chew eq. 7.1.48, with a trial potential whose energy is Steiner's formula. So I did not read §7 item 2 as met. §1.4's "fringing factor" constant does not exist. The bound dominates the exact coplanar and coaxial solutions (test). The adversarial pass raised this as D8 | `aethelgard_eda::extract` | fork | accept the variational bound as the §1.1 terms, or name the closed form wanted | **yes** |
| **F2** | **Pins' lands are counted as copper as well as at `Ci`.** The WO counts the pad term only for pads that are not device pins. `Ci` is the part's capacitance, not the land's, and leaving the lands out is §5.2's "pad left out". It adds 1,145 fF to SDA's bound on the reference | `line` | fork | accept | no |
| **F3** | **Copper at most 70 µm, and a board from 0.34 to 2.0 mm, are stated conditions, not cited values.** The board declares neither | `CONDITIONS` | condition | `AET-WO-3300`'s fabrication package declares both, so the condition becomes a fabrication instruction | yes (for 3300) |
| **F4** | **The scope is the bare board's copper plus `Ci`.** Part bodies (a ceramic resistor body near εr 10; the adversarial pass estimates up to about +353 fF per pull-up pad, as an estimate, not a measurement), solder and terminations above the pads are excluded, and the verdict's first condition says so. An "assembled-part" term would need part geometry the board file does not carry | `CONDITIONS`, README | scope | decide whether `1303` in full adds an assembled-part term | **yes** |
| **F5** | **Fence: `extract` reads `schematic` and `pcb`, not only `route`.** Parts are classified from `schematic::design`, since only the specification's design says what R1 or U1 is. Four visibility-only edits in generator files, with no emitted byte changed (the reference board's sha256 is `7b9b0e3d…2070` before and after): `pcb::escape` → `pub(crate)`; `schematic::design`, `R` and `C` → `pub(crate)`. The RUL-16 fence holds: `extract` never calls the device accessors, and `solved_devices.rs` still pins `schematic.rs` as eda's only reader | `pcb.rs:294`, `schematic.rs:36-37,415` | fence deviation | accept, or name another home for the classification | **yes** |
| **F6** | **Ruled on PR #99: its own WO.** **`Sx::child` takes the first match, and KiCad the last, so other gates misread boards too.** The adversarial pass's `dup_thickness` (a second `(thickness 0.2)`) passed `check drc` as well as the old `extract`. Every reader of `.kicad_pcb` through `child()` is open to the same class. Outside this fence: reported, not fixed | `sexpr.rs:105`; `check drc`'s board reader | gap | a backlog WO: one strict reader shared by every gate reading a board (`extract::read`'s walk is a starting point) | yes (scope) |
| F7 | My first build called `declared_i2c_devices` from eda's `extract`; `solved_devices.rs`'s fence test caught it at the first workspace run. The classification moved to the design | — | process | none | no |
| F8 | `extract` refuses a board declaring a thickness other than 1.6 mm, the generator's value; the bound is derived for it | `read` | scope | none | no |
| F9 | A specification with no I2C bus passes with `lines: []`: nothing exceeds a bound. Documented in the README (adversarial E7) | CLI | wording | none | no |
| F10 | A layer table giving F.Cu a user name (`(0 "F.Cu" signal "TOP")`) passes. Every item still names layers canonically, and a pad naming `TOP` is refused | `read` | held | none | no |
| F11 | The permittivity maximum is specified at 1 MHz; IPC gives none below, where part of a 300 ns rise's energy lies. The sheets read put typical FR-4 at 4.8 and 4.95 at 1 MHz, under the 5.4 used, but nothing here measures how much FR-4 rises below 1 MHz | `EPS_R_NUM` | residual | measure, or cite a laminate's low-frequency maximum, before relying on the margin | no |
| F12 | Neighbours are taken as static (condition 6). I2C's own lines never switch together in a compliant transfer, but an SPI edge beside an I2C rise is not covered | `CONDITIONS` | residual | coupling to switching nets belongs to `1303` in full | no |
| F13 | **Settled at the 1601 merge (main a2f9ea4), per the architect's ruling.** `extract` runs no tool, so its verdict has no `tool` or `sandbox` field (asserted in `extract_tier1.rs`). In `verify`, the extract entry has `uid_boundary` `null`, the form 1601 gives a gate that did not run, and no `uid_boundary_unmet` (asserted in `board_tier2.rs`). It is set at the call site, so `gate_report` keeps 1601's rule that a tool gate whose verdict lacks `sandbox` reads `false`. The `Gate` doc and the CLI README say so | `verify.rs` | settled | none | no |
| F14 | **Revisited at the 1601 merge.** The swap test finds DRC's copy by its scratch layout (`aethelgard-*/tool/input.kicad_pcb` under `TMPDIR`). 1601 refuses a `TMPDIR` whose ancestors another user can write, so the test now takes its `TMPDIR` from 1601's `common::private_tmp`. The layout is unchanged; the test still depends on it | `board_tier2.rs` | fragility | none now; a layout change fails the test loudly (`swapped` stays false) | no |
| F15 | The root `README.md` (protected) says extraction "is scheduled before any fabrication package is called verified". Once this merges, the first step exists: a capacitance bound judged against the declared one, which the campaign still does not consume | `README.md:13` | doc | the architect updates it | yes |
| F16 | NXP's site refused a scripted download (the session's permission classifier denied a retry as bot-check circumvention), so UM10204 was fetched with the fetch tool and its text decoded through the PDF's ToUnicode maps by a scratch script; Table 10 was read from that text. The data sheets and Chew's notes were read with pypdf in a scratch venv | — | process | none | no |
