# AET-WO-1315 — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read]** at the source named beside it; **[inferred]** from arithmetic or reasoning.

## Status: READY (D1 ruled (a) by the architect, 2026-11-19 16:30Z)

- **Everything in §1 is built.** The hot-plug bulk capacitor is drawn, placed and judged by the loop gate. Each control pin is drawn from its declaration. The hot-plug reference is claimed. The verdict no longer says "not drawn". `Stack::of` refuses a partial tolerance block at every reader.
- **Every board without a power tree is byte-identical to `main`.** That covers 34 of the 36 specifications, Sentinel-01 among them; Sentinel-01's package is covered in §3.2.
- **The two power boards pass every gate inside their bounds** [measured, tier 2 in the image].
- **D1, ruled (a): C1 stays anchored to U10.** Drawing it where §1.3 says reshapes Sentinel-03's placement, from 85.25 × 80.95 to **96.50 × 69.35 mm**. The cost is recorded as F6. An anchored C1 is judged by a loop rule (9.852 mm against 10.0); an unanchored one would sit about 17 mm away, judged by nothing. The pass branch of the size samples is pinned at 1 character (§3.4).
- **`main` `47ec18f6` (1910) is merged in.** The one conflict was `power_board_tier1`'s shared emit, now with `Stack::of(spec).unwrap()`.
- **No schema change, no stop condition hit.** `aethelgard-ir` moved only inside the carve-out (§2.5).

## 0. Prediction (§1.0), committed before the first run of the new emitter

The baseline is `main` `cd43b207`'s release binary, swept over all 36 JSON files under `examples/` and `crates/**/tests/fixtures/` (`emit`, `solve`, `generate`) into `~/.cache/aet-1315/base`.

| Figure | `main` | Predicted | Label |
|---|---|---|---|
| Loop rules on the power fixture (`board_tier2.rs:1186`) | 17 | **18**: one more, C1's, anchored to U10's VIN and PGND | inferred |
| Loop rules on Sentinel-03 | as measured on `main` | **+1**, the same rule | inferred |
| Sentinel-03 outline under its 100 × 100 mm bound | 85.25 × 80.95 mm | **grows in one axis, by under 10 mm**, still inside the bound: CP_Elec_8x10's courtyard (about 10 × 9 mm) is the largest passive on the board and joins U10's group | inferred |
| Sentinel-03 least `extract` slack | 9.2 pF | **9.2 ± 0.5 pF**: C1 is on `VBAT`, no judged I2C or SPI net; slack moves only if placement moves a judged line | inferred |
| Emitter-numbered capacitors on both power boards | from `C1` | **each moves up by one** below the first declared number (`C1` now claimed); none above it moves | read (`schematic.rs:643`, `Builder::reference`) |
| Every board without a power tree, Sentinel-01's package | — | **byte-identical**: every changed path reads `spec.power` or a `Stack::of` error that no committed specification raises | read |
| Controls drawn from the declaration (step 4 alone) | — | **no byte moves** on either power board: both declare EN `input`, PG `open`, OE `vcca`, which is what the emitter drew | read (WO §0, both fixtures) |
| Pinned figures that move | — | `board_tier2.rs` loop count; `outline_tier1.rs:330` outline and the name-free placement digest; the `ANSWER 1313c-fixture` line; any test naming an emitter-numbered capacitor of a power board | inferred |

**How the prediction did** [measured, §3]:
- **Loop count: right.** 17 became 18 on the power fixture; Sentinel-03 has 16, the full power fixture 20.
- **The renumbering: wrong.** Every emitter-numbered capacitor moves one slot along the free numbers, not only those below the first declared number. On Sentinel-03, C9 becomes C18 and C43 becomes C44 (§2.4).
- **The outline: wrong, in size and in shape.** It grew 11.25 mm in x and shrank 11.6 mm in y, against a predicted growth of under 10 mm in one axis.
- **Slack: just outside the band.** It went from 9.2 to 8.6 pF, against a predicted 9.2 ± 0.5.
- **Controls: right.** Both power boards are byte-identical after step 4 alone (§3.1).

## 1. Commits, in build order

| Commit | Step | Sweep against `main` (36 specifications; `emit`, `solve`, `generate`; unmasked) |
|---|---|---|
| `088011db` | §1.0 the prediction | — |
| `0f3fec05` | §1.2 the two library files | identical, 36/36 |
| `17e37a9f` | §1.6 `Stack::of` | identical, 36/36 |
| `610256df` | §1.4 controls from their declarations | identical, 36/36, both power boards included |
| `009c43e3` | §1.1, §1.5 `references`; the verdict strings | only the two power boards move (renumbering) |
| `cfabab05` and later | §1.3 the draw, the re-pins, then review-pass fixes | only the two power boards move |

The sweep script is `~/.cache/aet-1315/sweep.sh`; each binary writes into one fixed scratch path, so no embedded path differs. `main`'s binary was swept twice and came out byte-identical both times, so the comparison is deterministic.

## 2. What is built

### 2.1 The library files (§1.2)

- **`symbols/Device.kicad_symdir/C_Polarized.kicad_sym`:** sha256 `e0371fff…cc87a`, git blob `5e34d52e469f0a9e2fc017d87f285ccde14549f8`.
- **`footprints/Capacitor_SMD.pretty/CP_Elec_8x10.kicad_mod`:** sha256 `03187d2e…40cc3f`, git blob `48b4961d557fd1bdc14a7262a1b635e4c0084a2e`.
- **Provenance:** fetched raw from GitLab at tag `10.0.7`. GitLab's files API reports the same blob ids at `ref=10.0.7`, and `git hash-object` on the bytes agrees.
- **The lists:** `PINS`, `FILES` and the README table go from 50 to 52. `workspace_shape`'s list of files gains the two.
- **The `+` terminal is 1 in both files** [read, and tested]:
  - The symbol draws its `+` (two crossing strokes) at (−1.27, 2.286), beside pin 1 at (0, 3.81). Pin 2 is at (0, −3.81), with the filled plate.
  - The footprint's silkscreen `+` is at x = −5 mm, beside pad 1 at −3.25 mm; pad 2 is at +3.25 mm.
  - `the_bulk_capacitors_positive_terminal_is_where_the_vendored_files_mark_it` reads both from the vendored files. The row's `pins` is `{}` and carries no polarity role, so the emitter holds the constant `BULK_POSITIVE = "1"`, cited to these files. A role in the table would have been a schema change.

### 2.2 The hot-plug parts (§1.3, `schematic_power.rs`)

- **`bulk(table, hot_plug)`** returns the row when it is a `BulkCapacitor` on the board's side. Anything else is `PowerNotDrawable` by name:
  - a TVS (tested on a test-only table, the embedded one plus a `TVS_TEST` row);
  - a ceramic row;
  - `side: pack`, which validate refuses as `BatteryInputUndamped`, refused again here for an unvalidated caller;
  - a part the table lacks.
  Every declared part is judged before any is drawn.
- **Drawn:** `C1` with value `47u UCZ1H470MCL1GS`, using the row's symbol and footprint, pin 1 on `VBAT`, pin 2 on `GND`.
- **Placed:** as the last part of the first battery-fed regulator's sequence, anchored to its VIN and PGND pads (IN and GND for an LDO). With no battery-fed regulator it is drawn unanchored.
  - **The loop rule is 10.0 mm**, an engine choice set after measuring, which its text says (the `CIN_BULK_NM` precedent, `4300d2` D3).
  - **Measured** [`loops::judge` on the placed board]: 9.852 mm from + to VIN and 4.659 mm from − to PGND, the same on Sentinel-03 and on the power fixture. CP_Elec_8x10's pads are 6.5 mm apart and VIN and PGND are adjacent pins, so one pad is necessarily far.
  - **With the bound lifted to 30 mm, the placement is unchanged** [measured]: the bound does not drive the placer. A second declared bulk capacitor (a probe, not committed) lands at 9.143 / 7.975 mm and passes.
- **Draw order follows the declaration, never the parts list:** placement goes by `Anchor::seq`. `under_a_bound_the_board_does_not_depend_on_the_parts_order` is unchanged and passes.

### 2.3 Control pins (§1.4, `schematic_power.rs` `controls`)

- **Straps:** `input` goes on the regulator's input rail, `open` is a no-connect, and `vcca` and `vccb` go on the translator's A and B rails.
- **Wires:** drawn as a label on the declared net, only when a declared connector pin or test point ends that net. The set of ended nets is computed in `schematic.rs`, the one place pinned to read `declared_connectors` (`solved_devices.rs` `the_eda_reader_is_exactly_the_schematic`).
- **Refused by name** (`PowerNotDrawable`), none falling back to the old tie:
  - a row's control pin with no entry;
  - an entry for a pin the row does not list, or for one pin twice;
  - a strap the row does not allow, or one this part has no rail for;
  - a wire to a rail (`GND`, `VBAT` or a domain, any case);
  - a wire to a net nothing else ends;
  - both a strap and a wire, or neither.
- **One belt-and-suspenders branch:** a wire on a pin the row does not let be wired. `validate` refuses it (`ControlPinUnwired`), and every held row's control pins are wirable.

### 2.4 The reference claimed (§1.1, `aethelgard-ir` `power::references`)

- **The change:** each `input.hot_plug` reference is listed after `input.reference`. The emitter's claim list and validate's name checks read that one list.
- **Renumbering** [measured, the BOMs of the `009c43e3` sweep]. Every emitter-numbered capacitor takes the next free number, so all of them move one slot:
  - **Sentinel-03, before:** C1–C9, C18, C19, C28–C43. **After:** C2–C9, C18, C19, C28–C44.
  - **Power fixture, before:** C1–C9, C18, C19, C28, C29, C32–C39, C42–C45. **After:** C2–C9, C18, C19, C28, C29, C32–C39, C42–C46.
- **Name checks on a hot-plug reference** [`a_hot_plug_reference_the_emitter_refuses_is_refused_at_validate`]. In each case below, `validate` refuses and the emitter refuses too:
  - `#C1` gives `ReservedName`;
  - `c1` gives `DuplicateReference "C1 and c1"`;
  - `C10` (a declared CIN) gives `DuplicateReference`;
  - a test point's `TP9` gives `TestPointReferenceTaken`, while the emitter refuses it as `DuplicateReference`.
- **`C01` is refused by validate** [probe].
- **`unconnected-C1` is accepted by both** (F2, §5).

### 2.5 The verdict (§1.5, `power.rs`)

- **Removed:** the note that "the board emitter does not draw them … (FEEDBACK D5)", and the wire condition's clause "and the board does not draw it (FEEDBACK D5)".
- **Kept:** "neither the net's driver nor its level is judged".
- **Tested:** `ir/tests/power.rs` asserts that neither string appears, and that the wire condition ends with the kept clause. Nothing else in the repository (tests, goldens, docs outside `docs/work-orders`) held either string [grep].

### 2.6 `Stack::of` (§1.6)

- **Failing first:** `every_reader_of_a_board_refuses_a_partial_tolerance_block_by_name` failed on `main`. `check drc --spec` with `via_ring_um` removed answered `BoardUnreadable` ("a via Some(750000)/Some(300000)…"), because the board was read with the engine's via.
- **The shape:** `Stack::of` returns `Result<Stack, &'static str>`, the field path `RingRule::of` names. Every production caller refuses with `emit`'s kind, id and message (`FabToleranceUndeclared`, exit 2) through one CLI helper, `undeclared`: `check drc`, `check support`, `extract`, and `fab.rs` derive. `fab.rs` `notes` takes the stack derive already holds.
- **The test** covers `check drc`, `check support` and `extract`. Each refuses before any tool is looked for, so it stays tier 1.
- **`rules.rs` is outside the eda fence list but is a named caller** (WO grounds). `Rules::from_specification` returns the same `Result`, which is the only change there. Its production callers now build `Rules::from_stack` from the stack they hold.
- **Tests unwrap:** 42 call sites, mechanically.
- **Every production board reader takes its stack from `Stack::of`** [grep of `route::read`, `route::declared`, `extract::read*` and `bound::read` in `cli/src` and `eda/src`].

## 3. Acceptance (§2), measured

### 3.1 Boards without a power tree

- **The final sweep** (`hp-bin`, the full branch with the 10 mm rule) gives 34 of 36 specifications byte-identical to `main` `cd43b207`, solve, emit and generate outputs and exit codes alike, unmasked. Only `sentinel-03.json` and `sentinel-02-power.json` move, 9 files each.
- **Every other step was swept on its own** (§1). The vendoring, `Stack::of` and the controls each moved nothing, the power boards included.

### 3.2 Sentinel-01's package

**Identical: the branch moves no byte of Sentinel-01's package, `MANIFEST.json` included** [measured, in the pinned tools image].

- **`--against-fab`, the `bench-package` job's own command:** branch binary, `capped-run` 16G, swap 0.
  - It reports `differs` on `MANIFEST.json` only. All 16 other files equal the committed ones byte for byte.
  - The cause is this host: rootless podman gives `uid_boundary: false, unmet: user_namespace`, the verdict records the sandbox, and `MANIFEST.json` carries `verdict_sha256`. The job's comment says the same: the committed package is the one CI writes (F4).
- **So, an A/B in that same image:** `main` `cd43b207` built from `git archive` at the same `/repo` path, against the branch.
  - Both ran `verify --fab-out` with the job's five scenarios (1227 s and 1349 s).
  - `diff -r` of the two packages: **identical, all 17 files**, `MANIFEST.json` included (sha256 `c7516b34…` both).
  - Against the committed package, both differ only in `MANIFEST.json`'s `verdict_sha256` line (`a3e48664…` here, `3b585ef4…` committed): the sandbox, the same for `main`.
- **`fab/sentinel-01-bench/package/MANIFEST.json` does not move.** No file under `fab/` is touched.
- **The authoritative check is CI's.** Since `1910`, the `bench-package` job runs on every pull request; its run is cited at READY.

### 3.3 The two power boards [measured, tier 2 in the pinned tools image, `capped-run` 12G, swap 0]

**`board_tier2`, four tests, all pass (66 s):**
- `the_sentinel_03_fixture_passes_every_gate_inside_its_bound`: `validate`, `generate`, `emit`, ERC, DRC with and without parity, `check support` and `extract`, each with no finding. **ANSWER 96.50 × 69.35 mm, least `extract` slack 8.6 pF.**
- `the_power_fixture_passes_erc_drc_its_loop_rules_and_extract`: 18 loop rules, all passing.
- `the_full_power_fixture_routes_and_passes_every_gate_it_reaches`: still refused `TranslatorPinUnbounded`, by that name only.
- `each_size_sample_passes_every_gate_or_is_refused_by_name`: passes, but only on its refusal branch. See D1.

**Tier 1:** `hot_plug_tier1` (9 tests), `power_board_tier1`, `outline_tier1`, `supply_tier1`, `via_ring_tier1`, `solve_cli` and `campaign_tier1` all pass.

**C1 on both boards** (`the_declared_bulk_capacitor_is_c1_positive_on_vbat_on_both_power_boards`):
- **BOM:** `C1,47u UCZ1H470MCL1GS,Capacitor_SMD:CP_Elec_8x10,Device:C_Polarized,` is its only `C1` row.
- **Schematic:** one `C1` symbol, `Device:C_Polarized`, with that value.
- **Board:** pad 1 on `VBAT` and pad 2 on `GND`.
- **The schematic's pins** are proven equal to the board's by tier 2's DRC with schematic parity.

**The parity test** (`every_declared_power_reference_is_on_the_board_with_its_part`, tier 1, both boards):
- The set of every `reference`/`part` pair under `power` in the JSON equals the set `power::references` lists.
- Each reference is on the placed board exactly once, valued with its declared part number.
- The pairs are read from the specification, never from the emitter's part list.

### 3.4 Re-pinned figures, each from a measured run

| Pin | Before | After | Run |
|---|---|---|---|
| `power_board_tier1` `every_loop_rule_holds…` | 17 | 18 | tier 1 |
| `board_tier2` `the_power_fixture_passes…` loops | 17 | 18 | tier 2, in the image |
| `outline_tier1` outline | 85.25 × 80.95 mm | 96.50 × 69.35 mm | tier 1 |
| `outline_tier1` `FIXTURE_PLACEMENT_SHA256` | `5e30de78…` | `9b83d120…` | tier 1, from the failing assertion's `left` |
| `via_ring_tier1` ladder golden | rung 7, cost 320, IMU_SCK 24 945 fF | rung 5, cost 48, I2C_SCL 74 937 fF | `emit`'s `capacitance_ladder`, both binaries |
| `ANSWER 1313c-fixture` (stderr, not pinned) | 85.25 × 80.95, 9.2 pF | 96.50 × 69.35, 8.6 pF | tier 2 |
| `outline_tier1` `each_size_sample_…` | 2, 6, 8 each placed or refused | 1 places at 98.80 × 70.35 mm (pinned); 2, 6, 8 refused | tier 1 |
| `ANSWER 1313c-size` 2, 6, 8 (stderr) | `extract` refused (−5.1 pF), pass (5.4 pF), `OutlineUnreachable` | all `OutlineUnreachable` | tier 2 |

The ladder's smallest relative margin improves, from 16.9 % to 25.1 %. `extract`'s least absolute slack falls from 9.2 to 8.6 pF. These are two different measures: one relative, one absolute.

## 4. Decisions

### D1 · ruled (a), accept as built: the anchor costs Sentinel-03 its size headroom

**Measured** (`emit` under the fixture's own 100 × 100 mm bound, host release binaries; the in-image tier-2 tests agree):

| Sample (`common::rename::sized`) | `main` | Branch (C1 anchored, 10 mm rule) | C1 unanchored (scratch build) |
|---|---|---|---|
| fixture | 85.25 × 80.95 | 96.50 × 69.35 | 86.70 × 81.70 |
| 1 character | 87.75 × 82.70 | 98.80 × 70.35 | — |
| 2 | 89.20 × 84.95 | `OutlineUnreachable`, 100.50 wide | 89.20 × 82.70 |
| 4 | 91.75 × 83.20 | `OutlineUnreachable`, 107.25 | 91.95 × 85.20 |
| 6 | 96.90 × 87.95 | `OutlineUnreachable`, 112.15 | 96.90 × 88.45 |
| 8 | `OutlineUnreachable`, 101.90 | `OutlineUnreachable`, 119.75 | — |

**The cause is the anchor:**
- Lifting the loop bound to 30 mm gives identical outlines.
- With C1 in U10's group, the placer moves U10 and U11 from the top row (x ≈ 40 and 67 mm) into a right-hand column (x ≈ 78 mm). The board grows 11.25 mm wider and 11.6 mm shorter.
- The x headroom under the bound drops from 14.75 to 3.5 mm.

**Unanchored, C1 sits about 17 mm from U10,** and no loop rule judges it.

**Options:**
- (a) Accept as built: the fixture fits, and `1313c` promised no size.
- (b) Draw C1 unanchored on every board: the headroom stays as on `main`, and C1's distance is not judged.
- (c) Teach `pcb.rs` to place a large anchored passive. That is outside this fence and was not tried.

**Ruled (a)** (architect, 2026-11-19 16:30Z, the bus): honesty over headroom. The cost is F6; option (c) becomes a §7.1 row, the architect's.

### Forks taken (each for the architect to overturn)

- **D2 · `side: pack` at the emitter is refused by name, not drawn.** Drawing it on the board would contradict the declaration.
- **D3 · `BULK_POSITIVE` is a constant cited to the vendored files** (§2.1), not a table role: a role would be a schema change.
- **D4 · `Stack::of` returns `Result<Stack, &'static str>`.** `Rules::from_specification` was deleted in the reduction pass; each reader builds `Rules::from_stack` from the stack it holds.
- **D5 · `schematic::drawn`**, a second test-facing accessor beside `anchored`. It lets a test see the unanchored C1, which only a tree `validate` refuses (`PowerLoop`) can produce.
- **D6 · the unanchored C1 sits where its nets put it**, after the regulators, never placed by a rule. No committed tree reaches this case.

## 5. Findings

- **F1 · A control wire can reach a connector only on a net the specification already names.** `validate` lets a connector pin or test point name only a declared signal, a domain, `GND` or a debug net (`ir/src/connectors.rs` `net_kinds`, `ConnectorNetUnknown`, `TestPointNetUnknown`). So PG to a spare header pin as a new net (`PG_5V`) cannot be declared without a schema change. The tier-1 fixture wires PG to `NRST` beside J2 pin 3, which is a real pattern: hold the MCU in reset until the rail is good. **For `4500` or whoever wants a free control net.**
- **F2 · `unconnected-C1` as a hot-plug reference passes both `validate` and the emitter.** The `1206` F3 rule exists because a regulator's or crossing's reference names its own nets (`unconnected-x_FB`). A hot-plug reference names no net. Sentinel-03 with its C1 renamed `unconnected-C1` passes `validate`, `emit`, ERC, and DRC with `--spec` and schematic parity, each with no finding [measured in the image]. So this is harmless as far as KiCad reads it. `names.rs` is outside the carve-out, so nothing was changed there.
- **F3 · Test weights (for 1910/1911).** New tier-1 tests: `hot_plug_tier1` (9 tests, about 19 s locally in debug; two of them emit both power boards) and one `via_ring_tier1` test (about 39 s, one Sentinel-03 emit). Existing power-board tests now place and route a larger Sentinel-03. Posted on the bus; `ci/` is not mine.
- **F4 · Sentinel-01's committed MANIFEST cannot be matched off CI.** Rootless podman on Furnace gives `uid_boundary: false, unmet: user_namespace`. The verdict, and so `verdict_sha256` in `MANIFEST.json`, records it (§3.2). This is as the `bench-package` job's comment says, and is no change here.
- **F5 · Reduction findings outside scope:**
  - `solved()` is copied in `bench_tier1`, `outline_tier1` and `orientation_tier1`; `hot_plug_tier1` adds a fourth, and `power_board_tier1`'s `full()` a fifth body. One `tests/common` helper would remove about 22 lines.
  - The path of `sentinel-02-power.json` is re-derived in `power_board_tier1` (twice), `board_tier2`, `common::power` and `hot_plug_tier1`.

- **F6 · the cost of D1 (a), recorded as ruled:**
  - Sentinel-03's x headroom under its 100 × 100 mm bound goes from 14.75 to 3.5 mm.
  - Size samples of 2 to 8 characters now refuse `OutlineUnreachable`; on `main`, 2 to 6 placed.
  - The fixture and the 1-character sample still pass. `outline_tier1`'s `each_size_sample_…` pins the 1-character sample's outline (98.80 × 70.35 mm) so the pass branch stays tested.
  - `board_tier2`'s `each_size_sample_…` (2, 6, 8) now exercises only its refusal branch. Adding 1 there would add a full gate run, and with it a weight row, which is 1911's.
  - A placer that packs a large anchored passive better (option (c)) is the architect's §7.1 row.

## 6. Adversarial pass (§5) and mutants

| Attack | Result |
|---|---|
| **A reference collision** (§2.4) | `#C1`, `c1`, `C10`, `C01` and a test point's `TP9` are each refused by `validate` and by the emitter. The emitter never numbers `C1` (the BOM's one `C1` row is the declared part). `unconnected-C1`: F2. |
| **A reversed electrolytic** | Pad 1 on `VBAT` on board, BOM symbol and value pinned; schematic pins equal the board's by tier 2's parity DRC; the positive terminal is read from the vendored files by a test. Mutant M10 is killed. |
| **A declaration ignored** | OE `vccb`, PG wired to `NRST`, EN `open` on a buck, and a strap where a wire is declared (and the reverse) are each tested: M3, M4, M7. |
| **The old default coming back** | A missing EN, PG or OE given to the emitter without `validate` is refused by name (M1). The committed declarations draw as before (M8). |
| **A floating pin with a name** | Refused, naming the pin and the net (M6). |
| **A non-bulk row** | A TVS from a test-only table, a ceramic, a pack-side part and an unknown part are all refused (M9). A mutant admitting another kind does not compile: the let-else binds the bulk row's own type. |
| **`Stack::of` still quiet** | Every production reader takes its stack from `Stack::of` [grep]. `check drc`, `check support` and `extract` are tested (M15–M18). |
| **Sentinel-01 moving** | §3.1 and §3.2. |
| **Two bulk capacitors** (probe) | Both are anchored to U10 and pass: 9.852 / 4.659 mm and 9.143 / 7.975 mm. |
| **A battery-fed LDO first** (probe, unvalidated) | C1 is anchored to it (IN and GND pins). |

**Mutants:** 18, one per guard added or changed, each against the test named for it, in a detached worktree (`~/.cache/aet-1315/mut.py`, log `mut.log`). **18 killed.**
- M19's first run read as a survivor. Its filter (`control`) matched other tests but not the one asserting the wire condition; rerun on the right test, it was killed. A filter that matches some other test hides a survivor, and the harness's "no test ran" check cannot see that.
- One branch is labelled `belt-and-suspenders:` and has no mutant: a wire on a pin its row does not let be wired. No held row has such a pin.

## 7. Gates, reduction

- **`cargo fmt --all -- --check`:** clean.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean, and also with `--features aethelgard-cli/with-tools`.
- **`cargo test --workspace`:** 2270 passed, 0 failed, 0 ignored, exit 0, 1091 s (capped scope, 6 threads, at `ffe449cf`, the code as pushed).
- **Reduction pass (diff):** −5 net (`rules.rs` −7, its one test caller +2) · 1 duplicate removed (`Rules::from_specification`, a wrapper both production callers had replaced) · 1 mis-attached doc comment fixed · 0 dead paths left (the two strap refusals merged into one before the mutants) · 1 protected kept (the wirable check) · 2 findings outside scope (F5). The review was by a fresh subagent; each proposal was spot-checked against the code. Not taken: merging `extract.rs`'s refusal into `undeclared`, since its `Refusal` and the CLI's `Stop` are two types.
