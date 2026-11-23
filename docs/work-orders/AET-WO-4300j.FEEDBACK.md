# AET-WO-4300j — FEEDBACK: the motor pull-down weakened to a derived 442 kΩ, switchable by a drawn link (specification 29)

**Branch:** `wo/aet-4300j-motor-pull-fitment`, from `main` `cd79024c`. Main `3b59fa8d` (docs only) is merged in.

**Builder:** Claude (Opus 5.5) as project manager. The workers:
- Phase A, the IR and the specifications: one Opus worker, single-threaded;
- B1, the generators and judges: Opus;
- B2, eda: Opus, in parallel with B1 on disjoint crates;
- the adversarial pass: Opus, given the code and §5, not my conclusions;
- the reduction pass: Opus, with no stake in the code.

Every worker's summary was spot-checked against the artifact before it was adopted. Written 2026-11-23.

**Status: built; gates green except the bench package, which waits for the architect's dispatch** (§7).

**The answer, in six lines:**
- **The bound.** It follows AN4899 Rev 4 §5.1.5 (p. 18), mirrored for a pull-down. The mirroring is my derivation, labelled as such in `spike/4300j/DERIVATION.md`.
  - **Floor 422 561 Ω:** in BLHeli passthrough, RPU max against the pull-down, with the FC at VDD min and the ESC on its own rail at VDD max. The idle line must be ≥ ESC VIH + 10 % of VDD.
  - **Ceiling 895 714 Ω:** at reset, 700 nA of leakage must leave the line ≤ VIL − 10 % of VDD.
  - This is ruled D1 (a).
- **The value: 442 kΩ `CRCW0603_F_K`,** the smallest E96 whose ±1 % / ±2 % worst case lies inside the bound (D2). 4300i's 40.2 kΩ held the passthrough line at 1381.6 mV, which is neither level.
- **The rule moved from a band to the bound.** `io.motor_pull_bound` holds the inputs, including the ESC's own rail as a stand-in (F2). Validate computes the bound per board in i128: the pull-up on `mcu.supply`, the thresholds on the peer rail. It judges the resistor's tolerance + drift band against the bound (F3): `MotorPullOutsideBound`, `MotorPullFloorUnreachable`, `MotorPullBoundEmpty`.
- **`motor_outputs[].pull` is `{resistor, link, fitment, source}`.** `fitment` is `fitted` or `open`, required, with no default. The link is `CRCW0402_Z_0`, a new `jumper` kind (power-parts 10).
  - `fitted`: svd-hal writes `PUPDR 00` and hwdef `FLOATING`.
  - `open`: svd-hal writes `10` and hwdef `PULLDOWN`.
  - A comment line above each motor line in the hwdef names the fitment, the parts and the bound.
- **eda draws** `MOTORn` → link → `MOTORn_PD` → resistor → `GND` on every motor net. An `open` link is `DNP` in the schematic, on the footprint and in the BOM, and the JLCPCB files omit it. Support step 8 judges the chain.
- **No committed pin moved** (null sweep 34/34). Specification 29, solved 18, power-parts 10.

## 0. How it was made

| commit | what |
|---|---|
| `953e6413` | predictions and the derivation, before any run |
| `469b73ec`, `0dce579c` | the measurement and the STOP; the STOP revised for the ESC's own rail |
| `237f550f` | main `3b59fa8d` merged (docs) |
| `98eeebe0` | the ruling recorded |
| `9c35dc1f` | Phase A: the IR and the 34 specifications |
| `3f8195c0` | B1: the generators, judges and goldens |
| `600bce18` | B2: eda |
| `0fbf9816` | the null sweep |
| `bd0aa2f1` | the adversarial fixes F1, F7, F9, and the bump's literals |
| `c13a624f` | the reduction pass |
| `c60ab1e2` | F2 and F3, as ruled at 12:25Z |
| `955654fe` | main `75da8053` merged (4408, 1916, docs): `four-port` and its hwdef fixtures moved to spec 29 / solved 18 by their generator |
| `2fa78bba` | the null sweep re-run at the merged head |

**The kit** (`spike/4300j/`):
- `derive.py` / `derive.out`, the bound in exact fractions;
- `universe.sh` and `run.sh` (from 4300i), `motors.py`, `bump29.py`, `link_refs.py`, `nullsweep.py`;
- the mutant runners;
- the `open` goldens' sources in `open/`;
- the outputs in `out/`.

The baseline binary is in `~/.cache/aet-4300j/base`.

## 1. The measurement and the stop (§1.1–§1.2)

**What held:**
- All 34 committed specifications' baseline rows equal 4300i's merged-head record.
- There are 10 motor boards with 40 outputs, all at 40.2 kΩ, all on VDD_3V3 3300 mV ±50 ‰.
- P1–P7 held.

**What did not hold: P8–P10.** They assumed the ESC shares the FC's rail. DS70005664A p. 16 shows the ESC making its own 3.3 V (`VREG` → `R54` 0R → `VDD`). I found that while reading the page for S3, after the predictions were committed. The STOP led with the cross-rail reading, and the ruling took it.

**The ruling (10:11Z):** D1 (a), and D2–D9 as leaned (`spike/4300j/RULING.md`).

## 2. What was built

**The IR** (specification 29, solved 18, power-parts 10):
- **power-parts 10:**
  - `Kind::Jumper` (`max_milliohms`, `max_milliamps`), and the row `CRCW0402_Z_0` (VISHAY_20035 pp. 2–3);
  - the MCU row's `io.weak_pull_down_ohms` is replaced by `io.motor_pull_bound`, with these fields:

    | field | value | source |
    |---|---|---|
    | `pull_up_max_ohms` | 50 000 | DS12117 Table 49 p. 137 = Table 146 p. 241 |
    | `input_high_min_permille` | 700 | DS12117 Table 49 p. 136 = Table 146 p. 241 |
    | `input_low_max_permille` | 300 | the same tables |
    | `leakage_max_nanoamps` | 350 | DS12117 Table 49 p. 137 = Table 146 p. 241 |
    | `peer_input_current_nanoamps` | 350 | a stand-in, `{value, basis}` (D-c) |
    | `margin_permille` | 100 | the ruling, `{value, basis}` (D-c) |
    | `peer_rail_millivolts` | 3300 | a stand-in, DS70005664A p. 16 and S1, `{value, basis}` (F2) |
    | `peer_rail_tolerance_permille` | 50 | a stand-in, `{value, basis}` (F2) |

  - The reader refuses a missing or unknown field, a value ≤ 0, `low ≥ high`, `margin ≥ low`, `high + margin ≥ 1000`, and a peer tolerance ≥ 1000.
- **`offboard::motor_pull_window`** is the one computation, exact in i128:
  - `floor = ceil(B·RPU / (A − B − I·RPU))`;
  - `ceiling = floor((low − margin)·VDD_lo / I)`;
  - with A from the board's `mcu.supply` minimum (the pull-up's rail), and B = (high + margin) at the peer rail's maximum. The ceiling is taken at the peer rail's minimum (F2).
  - Validate judges the row's band against the window (F3): the low corner, ohms × (1 − tolerance − drift), must be ≥ `floor_ohms`, and the high corner ≤ `ceiling_ohms`.
  - Validate's step 26 and `solve` both call it. On 3300 ±50 it gives 17 325 000 / 41 → 422 561, and 6 270 000 / 7 → 895 714.
- **`MotorPull`** is `{resistor, link: {reference, part}, fitment, source}`. A link that is not a jumper row is `PowerPartUnknown`. Link references join `names::references`, and `<label>_PD` joins `names::own_nets`, so the existing `DuplicateReference` and `ReservedName` checks cover both.
- **`SolvedMotorOutput.pull`** carries `{fitment, resistor, ohms, link, floor_ohms, ceiling_ohms}`, and the reader checks each (F1). `silicon::implied_pupd` reads the fitment.
- **The 34 specifications** were edited as text (`bump29.py`): 442 000 Ω, a link (`out/link-refs.tsv`, the next free R numbers per board), `fitted`.

**The generators and judges:**
- **svd-hal:** a motor pin's `PUPDR` comes from the fitment (labelled `motor_pull`). The proof oracle still reads `implied_pupd` (INV-07).
- **hwdef:** `FLOATING` when fitted and `PULLDOWN` when open, with the comment line above each motor line. No `HAL_SERIAL_ESC_COMM_ENABLED` is written.
- **`check pinout`'s `WrongPull` and the ledger's `MOTOR_PUPD_RULE`** follow the fitment.
- **The `open` goldens:**
  - svd-hal `tests/ui/motors-pwm-open`: GPIOA PUPDR `0x64000000` → `0x64aa0000`;
  - hwdef `serial-dshot-9600-open`.

  Both come from sources in `spike/4300j/open/`, and both are checked pin by pin and byte for byte.

**eda:**
- **The schematic:** `MOTORn` → link (`Device:R`, value `0`, `R_0402_1005Metric`) → `MOTORn_PD` → resistor → `GND`, with the links drawn last, so no other reference moves. An `open` link is `(dnp yes)`, with the BOM note `DNP`.
- **The PCB:** a dnp footprint is `(attr smd dnp)`.
- **`fab::jlcpcb`** drops `DNP` rows from the BOM and the placement file.
- **`Part::beside`** places the resistor by the motor pad (D-a).
- **Support step 8** judges the chain:
  - `motor_link_missing`, `_part`, `_footprint`, `_off_net`, `_fitment`, `_count`;
  - the pull findings, now on `MOTORn_PD` / `to`;
  - `motor_net_extra_resistor`, on both nets.
- **The D4 stop did not fire:** ERC, DRC (`--spec --schematic`) and step 8 come back `[]` and route on the 7 drawn motor boards and an all-`open` motors-pwm. KiCad 10.0.7's own BOM and position export with `--exclude-dnp` drop the open links.

**Mutants** (all diff-sized, one per guard):

| set | killed |
|---|---|
| A (IR) | 29 / 29 |
| B1 | 15 / 15 |
| B2 | 27 / 27 |
| the adversarial fixes | F1: 4 / 5, where the fifth showed its clause was implied and the clause was deleted. F2 / F3: 6 / 6 |

The records are `spike/4300j/mutants-{a,b1,b2,adv}.md`.

## 3. The adversarial pass (§5, full)

The report and its probes are in `~/.cache/aet-4300j/adv/`.

| # | finding | severity | done |
|---|---|---|---|
| F1 | The topology reader did not check the new pull strings. A hand-made topology could inject live hwdef lines (`PULLUP`, `define HAL_SERIAL_ESC_COMM_ENABLED 1`) through `pull.link`, and could write a value outside its own window into the comment | medium | **Fixed** (`bd0aa2f1`): identifiers, distinct references, `MotorPullMismatch` |
| F2 | The ESC's rail is taken from `mcu.supply`, so a board declaring 3V3 ±1 ‰ admits 221 kΩ | medium (ruling) | **Ruled 12:25Z, built** (`c60ab1e2`): `peer_rail_millivolts` 3300 and `peer_rail_tolerance_permille` 50, held as stand-ins. A ±1 ‰ board now refuses 221 k. Its floor is 283 031, not the ruling's 422 561 (§5 D-d) |
| F3 | Validate judges the declared ohms, not the ±1 % / ±2 % band: 432 k passes with a low corner under the floor | low (ruling) | **Ruled 12:25Z, built** (`c60ab1e2`): the band is judged. On 3300 ±50 it admits 435 630..869 625; 432 k is refused and 442 k admitted |
| F4 | Step 8's extra-part check counts resistors only. A ferrite or a solder jumper from `MOTORn` or `MOTORn_PD` to a rail passes | low (inherited from 4300i) | Row R5 |
| F5 | Under the cross-rail model, an ESC reply at 3465 mV sits 330 mV above an FC at 3135 mV with its pull-up on. DS12117 p. 137 note 6 says pulls must be off above supply + 0.3 V | low | Row R1 |
| F6 | The bound treats every motor output as an ESC input, including on-board driver ICs (`DRV_EN`, `HB_IN1`) | low | Row R6 |
| F7 | Stale "no internal pull" text, wrong on an `open` board | cosmetic | **Fixed** in `MotorPinResetPull`. The ledger's table-wide string is row R7 |
| F8 | The link's BOM row reads `0`, with no part | cosmetic | Row R8 |
| F9 | The hwdef comment said passthrough is "safe" outright | cosmetic | **Fixed:** "checked for BLHeli passthrough idle …, the ESC threshold a stand-in (R1)" |

**What held:**
- edges and arithmetic on 8 rails, against exact rationals;
- the strict reading of spec 29 and solved 18;
- name and reference collisions;
- the firmware pull on every path, including mixed fitments on one timer, `css_hold`, the oracle, and no bootloader;
- 13 planted step-8 edits and 6 KiCad parity edits, in the tools image;
- every citation, rendered.

## 4. Versions and what moved

| what | before | after |
|---|---|---|
| specification | 28 | **29** |
| solved topology | 17 | **18** (ruled D9) |
| power-parts schema | 9 | **10** |

**The null sweep** (`out/null-sweep.tsv`) passes 34/34:
- exit codes and kinds are unchanged;
- no `/assignments/*/pin` moved;
- solves moved only in `schema_version`, `specification_sha256` and the `pull` objects;
- emitted files moved on the 7 drawn motor boards only;
- repeats are identical.

**Outlines** (B2, `out/b2-boards.tsv`):

| board | before → after (mm) |
|---|---|
| Sentinel-03 | 96.25 × 67.60 → 95.80 × 69.10, inside its 100 × 100 bound |
| motors-pwm, motors-dshot600, serial-dshot-9600 | 49.00 → 50.00 wide |
| others | moved by ≤ 0.5 mm, or not at all |

**Literals and goldens moved by their generators**, old → new where it is a number:

| crate | what moved |
|---|---|
| the IR | the `power.rs` table sha; the version ladder |
| svd-hal | `motors-pwm`, `hse-crystal` and `serial-four` by their sha line; `motors-pwm/topology.json` by version, sha and `pull`; the `order_2203` digests, with the old ones reproduced exactly with spec 28's sha put back |
| hwdef | `sentinel-01`, `sentinel-02` and `four-port` by version, sha and header line; `serial-dshot-9600` also by four comment lines |
| pincheck | the common topology, by its version only (R9) |
| the CLI | `solve_cli` digests `81246f99…` → `17515df2…` and `8eb15a7f…` → `2c4cb557…` (put back to spec 24 and solved 17, each still 1211c's); `stackups_tier1` ladder 21..28 → 21..29; `outline_tier1` (Sentinel-03 placement sha, outline and size samples); `via_ring_tier1`'s nearest net (IMU_MOSI → EXT_SDA, rung 3 unchanged); `pin_devices_drawn_tier1` R9 → R19; the README solve example |
| the twin | the `REFERENCE` literals 17 → 18 (granted), and its version-refusal test 18 → 19 |
| `boards/sentinel-01*` | sha lines only |

**The package:** `fab/sentinel-01-bench` moves by its specification and topology hash lines only (P17). That needs a dispatch, asked for on the bus.

## 5. Decisions for the architect

- **D-a · `Part::beside`** (B2). On `_PD` / GND the resistor shares no net with an IC, so the placer fell back to an IC's courtyard centre, 19–32 mm from its link. `beside` targets the motor pad instead. It is `None` on every other part, and nothing else moved.
- **D-b · Step 8's reading of D5.** It follows 4300i, not the STOP's wording:
  - any two-pin part from `MOTORn` or `MOTORn_PD` to `to` is refused;
  - a second resistor on either net is refused;
  - a non-resistor part from `MOTORn` to another rail passes, as 4300i's "a capacitor to the rail is no second pull" did. F4 / R5 is the gap that leaves.
- **D-d · The F2 test, as the ruling worded it, would assert a wrong number.** The ruling asked that "a board on VDD_3V3 ±1 ‰ still gets the 422 561 Ω floor".
  - The pull-up works from the board's own rail. At ±1 ‰ that rail's minimum is 3296.7 mV, not 3135 mV, so its pull-up is stronger and the floor is 1 386 000 000 / 4897 = 283 030.4 Ω.
  - The ceiling is the ESC's alone and stays 895 714 Ω.
  - The finding's concern holds: 221 k is still refused there, because its band starts at 291 785.
  - The test plants the derived numbers (`motor.rs` `the_bound_moves_with_the_rail`). **Accepted, 13:10Z:** the floor follows the board's own pull-up rail.
- **D-c · `{value, basis}`.** `peer_input_current_nanoamps` and `margin_permille` are held as `{value, basis}`, not `{value, at}`: an `at` must name a held document, and a ruling is not one. This follows the table's `bound_basis` precedent.

## 6. Rows (§7.1)

- **R1 · The ESC's input:** its threshold, input current, rail tolerance, and behaviour above supply (F5) wait for a held ESC MCU datasheet: dsPIC33CDVC256MP506 for the held reference design, or the BLHeli ESC's MCU. S1 and S2 stand in until then.
- **R2 · RPU near VDD is not held.** DS12117 states it at `VIN = VSS` only (note 7).
- **R3 · The reference ESC loads its input with 0.1 µF** (`C4`, DS70005664A p. 16). That is for (b), the off-board drive bound.
- **R4 · Whether an ESC's firmware enables its own input pull** is not held. A BLHeli bootloader that pulls up at power-on would contest a few-hundred-kΩ pull-down at reset.
- **R5 · Step 8 counts resistors only** for an extra part on the motor or pull net (F4, inherited).
- **R6 · On-board driver motor lines** (`DRV_EN`, `HB_IN1`, …) are judged by the ESC bound. That is harmless (it only tightens), but its cited basis does not describe them.
- **R7 · The ledger's PUPDR table string** (`ledger.rs` ~1089) still says "no internal pull" for every motor pin. Changing it moves every board's verify text, so it is the architect's call.
- **R8 · The link's BOM row reads `0`,** not `0 CRCW0402_Z_0`, unlike the resistor's `442k CRCW0603_F_K`.
- **R9 · The pincheck test topology was already stale before this WO** (no console, `bor_level` 0). Only its version moved; regenerating it turns 9 tests red (B1).
- **R10 · `source` is capped at 200 B** (`board::MAX_SOURCE`). The ruled text had to be shortened, and it keeps every page and the timestamp.
- **R11 · This worktree was made by moving 2310's.**
  - `.git` points at `worktrees/aethelgard-2310`, and its `target/` held 2310's test binaries, whose embedded paths name `/home/forge/aethelgard-2310`.
  - Cargo kept 171 of them fresh. My first gate run wrote 525 files of test scratch to `/home/forge/aethelgard-2310/target/tmp`, outside this worktree.
  - The classifier denied my removing it. It is safe to delete: every file in it is newer than 11:25Z today.
  - The gates were re-run from a clean target (§7).

## 7. Gates, CI, package

**Gates, on the merged head `2fa78bba`, from a clean target** (R11), plus the fix below:
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: 2,913 passed, 2 failed.
  - **`hwdef` `each_motor_line_s_pull_follows_its_fitment`:** my merge put main's `four-port` at index 3 of `BOARDS`, so the test's index for the `open` fixture went stale. It now finds both fixtures by name, and `cargo test -p aethelgard-hwdef` gives 20 passed.
  - **`bench_board_tier1` `the_committed_package_is_bound_to_its_specification_scenarios_and_emit`:** the committed package's specification sha is the old one. It waits for the dispatch.

**CI:** (after push)

**Reduction pass (diff):** −42 net · 2 duplicates removed · 1 dead path removed · 4 protected kept · 5 findings below.
- **The duplicates:** `PullFitment::name` (its single caller restated serde's names), and the jumper test's copy of a row lookup.
- **The dead path:** the reader's `high >= 1000`, implied by `high + margin >= 1000` with margin > 0.
- **Two test helpers** merged into the diff's own `motor_kinds`.
- **The protected four:** svd-hal `motor_pull` (the oracle's copy, INV-07), eda's footprint-without-`attr` refusal, the `dnp` node lookup, and step 8's row lookup. Two more unreachable checks in `motor_pulls()` are now labelled.
- **Findings kept:**
  - RD-a: svd-hal `Role::policy()`'s pull column is `0b00` in every arm, and the timer arm is now never read. It predates this WO.
  - RD-b: `offboard::validate` looks up the supply domain a second time, for the message alone.
  - RD-c: test assertions that restate others: `the_window_is_what_validate_admits`, the committed-rail edges in `motor.rs`, and `emit_tier1`'s link block.
  - RD-d: the fitment rule is restated in six svd-hal doc comments, four READMEs and three ledger texts. Prose cuts are the owner's.
  - RD-e: the window-only-when-a-motor Option pattern appears in both `offboard.rs` and `solve.rs`.
