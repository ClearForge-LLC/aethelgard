# AET-WO-4300j — FEEDBACK: the motor pull-down weakened to a derived 442 kΩ, switchable by a drawn link (specification 29)

**Branch:** `wo/aet-4300j-motor-pull-fitment`, from `main` `cd79024c`. Main `3b59fa8d` (docs only) is merged in.

**Builder:** Claude (Opus 5.5) as project manager. The workers:
- Phase A, the IR and the specifications: one Opus worker, single-threaded;
- B1, the generators and judges: Opus;
- B2, eda: Opus, in parallel with B1 on disjoint crates;
- the adversarial pass: Opus, given the code and §5, not my conclusions;
- the reduction pass: Opus, with no stake in the code.

Every worker's summary was spot-checked against the artifact before it was adopted. Written 2026-11-23.

**Status: (pending: gates, CI, package dispatch, the F2/F3 ruling).**

**The answer, in six lines:**
- **The bound.** It follows AN4899 Rev 4 §5.1.5 (p. 18), mirrored for a pull-down. The mirroring is my derivation, labelled as such in `spike/4300j/DERIVATION.md`.
  - **Floor 422 561 Ω:** in BLHeli passthrough, RPU max against the pull-down, with the FC at VDD min and the ESC on its own rail at VDD max. The idle line must be ≥ ESC VIH + 10 % of VDD.
  - **Ceiling 895 714 Ω:** at reset, 700 nA of leakage must leave the line ≤ VIL − 10 % of VDD.
  - This is ruled D1 (a).
- **The value: 442 kΩ `CRCW0603_F_K`,** the smallest E96 whose ±1 % / ±2 % worst case lies inside the bound (D2). 4300i's 40.2 kΩ held the passthrough line at 1381.6 mV, which is neither level.
- **The rule moved from a band to the bound.** `io.motor_pull_bound` holds the inputs, and validate computes the bound per board from `mcu.supply` in i128 (`MotorPullOutsideBound`, `MotorPullFloorUnreachable`, `MotorPullBoundEmpty`).
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
| `bd0aa2f1` | the adversarial fixes and the bump's literals |

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

  - The reader refuses a missing or unknown field, a value ≤ 0, `low ≥ high`, `margin ≥ low`, and `high + margin ≥ 1000`.
- **`offboard::motor_pull_window`** is the one computation, exact in i128:
  - `floor = ceil(B·RPU / (A − B − I·RPU))`;
  - `ceiling = floor((low − margin)·VDD_lo / I)`;
  - with A from the board's `mcu.supply` minimum and B = (high + margin) at its maximum.
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
| the adversarial fixes | 4 / 5: the fifth showed its clause was implied, and the clause was deleted |

The records are `spike/4300j/mutants-{a,b1,b2,adv}.md`.

## 3. The adversarial pass (§5, full)

The report and its probes are in `~/.cache/aet-4300j/adv/`.

| # | finding | severity | done |
|---|---|---|---|
| F1 | The topology reader did not check the new pull strings. A hand-made topology could inject live hwdef lines (`PULLUP`, `define HAL_SERIAL_ESC_COMM_ENABLED 1`) through `pull.link`, and could write a value outside its own window into the comment | medium | **Fixed** (`bd0aa2f1`): identifiers, distinct references, `MotorPullMismatch` |
| F2 | The ESC's rail is taken from `mcu.supply`, so a board declaring 3V3 ±1 ‰ admits 221 kΩ | medium (ruling) | (pending: architect) |
| F3 | Validate judges the declared ohms, not the ±1 % / ±2 % band: 432 k passes with a low corner under the floor | low (ruling) | (pending: architect) |
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

(pending: the literals table, gates, the package)

## 5. Decisions for the architect

(pending: F2, F3, unless ruled first)

- **D-a · `Part::beside`** (B2). On `_PD` / GND the resistor shares no net with an IC, so the placer fell back to an IC's courtyard centre, 19–32 mm from its link. `beside` targets the motor pad instead. It is `None` on every other part, and nothing else moved.
- **D-b · Step 8's reading of D5.** It follows 4300i, not the STOP's wording:
  - any two-pin part from `MOTORn` or `MOTORn_PD` to `to` is refused;
  - a second resistor on either net is refused;
  - a non-resistor part from `MOTORn` to another rail passes, as 4300i's "a capacitor to the rail is no second pull" did. F4 / R5 is the gap that leaves.
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

(pending)
