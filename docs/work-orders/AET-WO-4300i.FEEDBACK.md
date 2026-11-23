# AET-WO-4300i — FEEDBACK: the motor pull as a declared part, FT pins only at connectors, Sentinel-03's MOTOR1 on PA8, and declared pin reservations (specification 28)

**Branch:** `wo/aet-4300i-offboard-build`, from `main` `f50c29f4`. Merged in: `28fba951` and `fa9d7edd` (docs only), then `29a34afa` (1914's P3, 4407, 2309).

**Builder:** Claude (Opus 5.5) as project manager. The workers:
- Phase A, the IR and solver: one Opus worker, single-threaded;
- B1, the generators and judges: Opus;
- B2, eda: Opus;
- the adversarial pass: Opus. It was given the code and §5, not my conclusions;
- the reduction pass: Opus, with no stake in the code;
- the fixes: Opus for the IR, Sonnet for eda.

Every worker's summary was spot-checked against the artifact before it was used. Written 2026-11-23.

**Status: READY.** The bench package is the one from dispatch run 45117832704, committed byte for byte (§6). Main `aaad1555` (1607) is merged in. That merge changed no file under `fab/`. The bench-package job's `against_fab` on the pushed head is the witness that the package still matches.

**The answer, in six lines:**
- **(1) Every motor output declares its pull-down,** `motor_outputs[].pull`, a required key:
  - shape: `{resistor: {reference, part, ohms, to}, source}`;
  - value: 40.2 kΩ, the CRCW0603 E96 value nearest DS12117's RPD of 40 kΩ;
  - rules: `to` must be GND, the value must lie inside RPD's 30–50 kΩ (held as data), and the part must be a resistor row.
  - Neither generator writes an internal motor pull. svd-hal writes `PUPDR` `00`, and hwdef writes `FLOATING`.
  - `implied_pupd`, `check pinout` and the witness ledger judge `00` on a motor pin.
  - eda draws the resistor on all 28 drawn motor nets, and support step 8 judges it from the netlist.
- **(2) At a connector the solver offers FT pins only.** A non-FT candidate is withheld, and named when that leaves a claimant short. A hand pin that is not FT is refused (`ConnectorPinNotFt`). There is no escape field (D5).
- **(3) Sentinel-03's MOTOR1 is on PA8.** PA11, PA12 and PC9 are reserved there, and the PC9 reason carries `3400` E5's clock-out, now on MCO2 (F3 of the ruling).
- **(4) `mcu.reserved_pins` is required.** Each entry gives a pin and its reason. The solver never offers a reserved pin in any branch, and a hand pin on one is refused with the declared reason.
- **No committed board's pin moved** other than Sentinel-03's MOTOR1 (null sweep, 34/34). Emitted bytes moved on the 7 motor boards only. `boards/` and `fab/` moved by their specification-hash lines alone.

## 0. How it was made

**Commits:**

| commit | what |
|---|---|
| `45412fe0` | predictions and rules, before any run |
| `0de4f99f` | the measurement |
| `8424fe5e` | the STOP |
| `50302b80`, `9a23a2e6` | Phase A: the IR and solver; the 34 specifications by text edits |
| `86ce74c1` | B1: generators, ledger, pinout |
| `448774bb` | B2: eda draws and judges the pull |
| `9118a05e` | the null sweep |
| `54e091df` … `05f1b28e` | the adversarial fixes |
| `521d85be` | the support-gate fixes |
| `2e8a1de6` … `dfc53649` | the reduction pass |
| `172e0657` | the labels; this is the head CI and the dispatch ran on |
| `5f10de68` | the bench package (dispatch 45117832704) and the P3 weight row |
| `591bdd30` and after | main `aaad1555` merged; FEEDBACK; the re-checks on the merged head |

**Kit** (`spike/4300i/`):
- `universe.sh`, `run.sh` (with `OUT=`), `motors.py`, `facing.py`, `repin.*`;
- `pinmoves.py`, `nullsweep.py`;
- the three mutant runners;
- outputs in `out/`.

The baseline binary and its outputs are kept in `~/.cache/aet-4300i/base`.

## 1. The measurement and the stop (§1.1–§1.2)

The measurement is in `spike/4300i/RESULTS.md`, and the stop with its decisions is in `STOP.md`. 16 of 17 predictions held; P16 (versions) is settled in §4.

The ruling (2026-11-23T01:15Z):
- **V (iii):** DS12117 RPD is the source. The value is the row's nearest to 40 kΩ inside [30, 50] kΩ.
- **B:** the pull is required on all 40 outputs.
- **I:** `to` is GND.
- **F1, F2:** both in fence.
- **F3:** my lean.

## 2. What was built

**The IR, specification 28:**
- `MotorPull` (in `motor.rs`) and `ReservedPin` (on `Mcu`). Both keys are required, and neither has a default (`INV-06`).
- `power-parts.json` schema 9 holds `io.weak_pull_down_ohms`, cited to DS12117 Rev 10 Table 49 p. 137 and Table 146 p. 241.
- Step 26 (`offboard.rs`) refuses these new kinds:

  | kind | refuses |
  |---|---|
  | `MotorPullNotDown` | a pull whose `to` is not GND |
  | `MotorPullOutsideRpd` | a value outside 30–50 kΩ |
  | `MotorPinResetPull` | a motor on PA13, PA14, PA15 or PB4, whose reset `PUPDR` holds a pull |
  | `ReservedPinTwice` | a reserved pin listed twice |
  | `ReservedPinEngineReserved` | a pin the engine already reserves |
  | `ConnectorPinNotFt` | a hand pin at a connector that is not FT |

  It also reuses `PowerPartUnknown` (not a resistor row; a rowless MCU part once a net faces a connector), `McuPinStructureUnheld`, `InvalidSource` and `UnknownPin`.
- The solver takes its reservations from `all_reserved` (the engine's and the declared) in every branch, and `offboard::AtConnector` withholds non-FT pins at a connector in every branch that can place a connector-facing net.
- `silicon::implied_pupd` is 0 on a motor net. `motor_output_pupd` and `idle_pupd` are removed.

**The generators and judges:**
- svd-hal: `Role::policy` gives a timer channel no pull. `css_hold` drives low through BSRR and MODER, as before.
- hwdef: motor lines write `FLOATING` (ArduPilot's own default on a PWM line, `chibios_hwdef.py:387-427` at `fb773567`). `IdleHighNotEmitted` is removed.
- The ledger has one motor rule, `MOTOR_PUPD_RULE`, citing D0. `MOTOR_PUPD_HIGH_RULE` is removed.
- `check pinout` refuses a pulled motor pin as `WrongPull`.

**eda:**
- `schematic.rs` `motor_pulls()` draws each pull after every other part, so no other reference moves.
- The resistor is placed 4.96–11.07 mm from the MCU pad.
- eda now claims `names::references` directly; the reduction removed its hand copy (1218 F2).
- Support step 8 has six findings, each judged from the netlist:

  | finding | the netlist shows |
  |---|---|
  | `motor_pull_missing` | no declared resistor |
  | `motor_pull_off_net` | the resistor not between the motor net and `to` |
  | `motor_pull_value` | a value other than the declared one |
  | `motor_pull_footprint` | a footprint other than the row's |
  | `motor_pull_count` | another two-pin part between the motor net and `to` |
  | `motor_net_extra_resistor` | another resistor on the motor net |

**The specifications** were edited as text:
- `schema_version` set to 28;
- `reserved_pins: []` added;
- a pull on each of the 40 motor outputs, with references in `spike/4300i/out/pull-refs.tsv`;
- Sentinel-03's MOTOR1 moved to PA8, with its three reservations.

## 3. The adversarial pass (§5, full)

The report and probes are in `~/.cache/aet-4300i/adv/`, with before and after in `spike/4300i/out/adv-fixes.tsv`.

| # | finding | severity | done |
|---|---|---|---|
| F1 | A connector-facing net reached a solver-chosen TT pin through another peripheral instance (EXT_SCK on PA5 via SPI1). The first build's FT-first order worked within one instance only | high | **Fixed.** FT pins only at a connector, non-FT withheld and named. The probe now solves on SPI2, all FT. The partition was removed (reduction) |
| F2 | A motor could be placed on PA15 or PB4, whose reset pull-up fights the pull-down from power-up | medium | **Fixed.** Never offered, and a hand pin is refused (`MotorPinResetPull`) |
| F3 | An idle-high output is pulled down, so it sits low until `start_outputs` | medium (ruling) | **Not refused, by my reading** (bus, 04:2xZ): ruling I fixed only `to`, and D0 holds every motor line low from power-up. Refusing would retire 1211's ruled IN/IN brake idle. Q9's pull-up went with Q-G |
| F4 | Support step 8 counted a parallel part only when it had the same symbol | medium | **Fixed** (`motor_pull_count`, any two-pin part) |
| F5 | Step 8 judged by net name only | low | **Fixed** for a second resistor on the motor net (`motor_net_extra_resistor`). The line net holding the MCU's solved pad and the connector pin is not judged (row, §7) |
| F6 | 30 000 and 50 000 Ω pass, though a 1 % row reaches 29.7 k and 50.5 k | low (ruling) | **Kept, by my reading** (bus): the range is judged on the declared ohms, because 9 of the 10 motor boards declare no temperature. No free-text escape was built (D5) |
| F7 | ArduPilot's BLHeli passthrough sets PULLUP on a motor line while in passthrough (`RCOutput.cpp:2300-2313`; `HAL_SERIAL_ESC_COMM_ENABLED` defaults to 1, `AP_HAL_Boards.h:322`) | low | **Decision, D2 below** |
| F8 | `ft_pins` read a rowless part as "nothing is FT" | cosmetic | **Fixed** (`PowerPartUnknown` once any net faces a connector) |

**Clean:**
- a 338-case reservation sweep (every solved pin of every committed specification reserved in turn): no violation;
- a hand TT pin refused through every declaration kind;
- the pull's edge values, refused or accepted as specified: 0, 29 999, 50 001, `u32::MAX`, non-GND, `mcu` arms, a null pull, a missing pull;
- the generated `PUPDR` on every motor board;
- a 12-motor emit;
- the BOM against the schematic and the board;
- both §6 claims, re-derived.

**From my own reading,** the pinned ArduPilot's bidirectional DShot pulls each of its channels up at runtime (`RCOutput_bdshot.cpp:117-128`). hwdef emits no `BIDIR` (D7), so that code is not reached. A runtime parameter could still reach it; that is row R3.

## 4. Versions and what moved

| what | before | after |
|---|---|---|
| specification | 27 | **28** |
| power-parts schema | 8 | **9** |
| solved topology | 17 | 17, unchanged, so no twin source moved |

`STOP.md` predicted the power-parts table would not move. It did, because the RPD range is held as data (bus note, 01:2xZ).

**Moved by their generators:**
- svd-hal goldens: motors-pwm and hse-crystal by a `PUPDR` word, the rest by the hash line;
- hwdef fixtures: `serial-dshot-9600`'s four motor lines;
- `boards/`: 16 files, one hash line each;
- `fab/sentinel-01-bench`: `FAB-NOTES.md` and `MANIFEST.json` moved, by their hash lines only. Those are the specification, topology, verdict and `FAB-NOTES` digests. No Gerber, BOM, placement file or ELF byte moved.

**Literals moved, each recorded old → new in its commit:**
- the CLI tests: digests re-taken in `order_2203` and `solve_cli`, the ledger counts, `stackups` 21..28;
- Sentinel-03's placement and outline: 95.80 × 67.60 → 96.25 × 67.60 mm, inside its 100 × 100 bound;
- `solve_cli`'s `an_unknown_part_is_exit_2`: the kind moved to `PowerPartUnknown`, still exit 2 (F8).

## 5. Decisions for the architect

- **D1 · F1's reading of D1.** At a connector the solver offers FT pins only, the same bar a hand pin meets.
  - **Alternative:** a two-pass preference that falls back to TT. This moved no committed pin.
- **D2 · BLHeli passthrough (F7).** Write `define HAL_SERIAL_ESC_COMM_ENABLED 0` in the generated hwdef, or accept the pull-up during passthrough as an operator mode.
  - **Lean:** accept and name it. Passthrough is a configuration session with the ESC, not flight.
- **D3 · The RPD band (F6).** Judge the declared value (built), or a tolerance band, which would need a declared temperature on every motor board.
  - **Lean:** as built.

## 6. Gates, CI, package

- **Gates on the merged tree** (main `aaad1555`):
  - `cargo fmt --all -- --check` is clean.
  - `cargo clippy --workspace --all-targets -- -D warnings` is clean.
  - `cargo test --workspace --no-fail-fast` passes 2,850 with 0 failed, run in a capped scope with the target inside the repository, as CI runs it.
- **CI on `172e0657`** (ci 45117199226, and dispatch 45117832704 at the same head):
  - Every verify gate passed for the bench board.
  - It failed in two places, both expected: the committed package was stale (`against_fab: differs`, and `bench_board_tier1`'s hash binding), and 1914's P3 refused one unweighted test.
  - The package is now committed. P3's row is `tests/emit_tier1.rs` `every_motor_net_carries_its_declared_pull_down_in_the_emitted_files`, at its worst in-shard time of 29.939 s over the two runs. No other test this work order adds took 4 s or more; the next slowest took 1.854 s.
- **Package:** dispatch 45117832704's `bench-package` artifact, copied byte for byte into `fab/sentinel-01-bench/package` (`5f10de68`).

**Mutants:**

| set | killed |
|---|---|
| IR (`mutants.md`) | 42 / 42, re-run after the labels |
| B1 | 6 / 6 |
| B2 | 22 / 22 |

**Null sweep:** 34/34 (`out/null-sweep.tsv`), run on the build before main's `c7a62975`. It was re-run after the adversarial fixes and after the reduction pass, and its output did not change.
- Exit codes and refusal kinds are unchanged.
- The solves move by their specification hash alone, except Sentinel-03's `/assignments/MOTOR1/pin`.
- Emitted files move on the 7 motor boards only.
- Repeats are identical.

**Reduction pass (diff):** −61 net · 2 duplicates removed · 2 dead paths removed · 1 protected kept · 8 findings below.
- **The duplicates:**
  - eda's hand copy of `names::references`;
  - the connector-facing filter, which had two homes in `offboard.rs`.
- **The dead paths:**
  - `arms::references`;
  - `faces()` and `ft()` (single-caller wrappers).
- **Kept as protected:** support's part lookup, labelled.
- **The reducer's F4:** four unreachable defensive re-checks, now labelled `belt-and-suspenders:` (`172e0657`).
- **Findings (kept):**
  - R-a: the claimants are enumerated in four places that cannot derive one another;
  - R-b: "a structure's family is the text before the first `_`" has four copies, three of them older than this WO, in `power_parts.rs`;
  - R-c: the reset-pull test has three copies;
  - R-d: `WeakPull.typical` has no reader in `src`;
  - R-e: the three mutant runners are one mechanism;
  - R-f: the motor-pull rule is restated in four READMEs;
  - R-g: RPU is a constant while RPD is data (RPU is older);
  - R-h: the hwdef README's history note.

## 7. Rows (§7.1)

- **R1 · The fetch list.** Held:
  - AN4899 (ST, "STM32 GPIO configuration for hardware settings and low-power consumption");
  - the input-stage datasheet of the ESC J5 will drive.

  Either is a held source that could replace the RPD-derived value (ruling V). Scotty fetches them. This is proposed here because the roadmap is protected.
- **R2 · The generator does not see declared reservations.** svd-hal (`board.rs` ~1823) refuses only the engine's reserved pins. A solved topology carries no reservation, so a hand-edited topology could put a pin on PC9.
  - **Proposed:** carry `reserved_pins` at the next solved-topology bump, or name it.
- **R3 · Runtime pulls the engine cannot see:**
  - ArduPilot bidirectional DShot sets PULLUP on its channels (not emitted, D7);
  - BLHeli passthrough does the same (D2).

  Both are set by parameters at runtime, which `check pinout` (init words) cannot see.
- **R4 · Support step 8 does not check the motor net's other ends.** It does not judge that the net holds the MCU's solved pad and the connector pin (adversarial F5, P3/P4). Step 5's connector judgement predates this WO and covers supply, GND and debug nets only.
- **R5 · Stand-in motor boards draw nothing:** 7 nets on `hse-external-8` and `d1-drv8876`. The 5 on `d2-drv8833` are refused at validate by design. "Drawn and judged" holds for the 28 nets on the 7 drawn boards. The other 12 are declared and validated, not drawn.
- **R6 · The FT and reservation tests use synthetic pin tables.** The IR holds no real one. The real cross-instance probe is recorded in `out/adv-fixes.tsv` only.
- **R7 · A mistyped pin on a connector-facing net** is now refused at validate as `McuPinStructureUnheld`, where before it was `UnknownPin` at solve. It is still refused by name, and earlier.
- **R8 · New CLI tests with no weight row** get the default weight unless CI measures them at 10 s or more (1914 P3, §6).

## 8. Fence

**Inside the fence:**
- `aethelgard-ir`, `-svd-hal`, `-hwdef`, `-pincheck` and `-eda`;
- `examples/`, plus `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` (ruled F1);
- the version literals and goldens a bump moves, by their generators: the CLI test fixtures, `boards/` and `fab/`;
- `spike/4300i/` and this file.

**Outside it, each with its reason:**
- **`crates/aethelgard-cli/src/ledger.rs` and `tests/ledger_tier1.rs`:** ruled F2.
- **`ci/test-weights.tsv`:** one row, by the architect's note of 04:30Z (1914 P3).
- **Literals moved in CLI tests:**
  - `README.md`: the example hash prefix, and the ledger row's rule;
  - `battery_sag_tier1.rs` and `ledger_tier1.rs`: counts;
  - `solve_cli.rs`: digests, and the F8 kind;
  - `stackups_tier1.rs`: 21..28;
  - `outline_tier1.rs` and `via_ring_tier1.rs`: Sentinel-03's measured outline, digest and ladder.
- **Shared test helpers:**
  - `tests/common/netlist.rs` now writes each footprint, so step 8 can judge it;
  - `bench_tier1.rs`: one replace string lost its closing `)` and still matches.
- **New CLI tests:** in `emit_tier1.rs` and `support_tier1.rs`.
- **Refusal kinds that moved on existing tests:**
  - a hand motor on PA13 or PA14 is now `MotorPinResetPull` where it was `ReservedPin` (`motor.rs` now checks `ReservedPin` on PC14);
  - serial PA5 is now `ConnectorPinNotFt`;
  - serial PZ1 is now `McuPinStructureUnheld` (R7);
  - a rowless part in `solve_cli` is now `PowerPartUnknown`.
- **Public error fields added:** `SolveError::ReservedPin.reason`, and `withheld` on `NoFreePin`, `NoAdcPin` and `AdcUnmodelled`. When nothing is withheld the message is byte-identical.

**Not touched:** the twin, chaos, `verify`, and the workflows.

## 9. Re-checks on the merged head

- **Null sweep:** 34/34 with the release binary of the merged tree. `out/after` came out byte-identical to the committed copy.
- **The adversary's reservation sweep:** 338 cases, 299 moved off the reserved pin and 39 refused by name (31 `ReservedPin`, every one a hand pin). No violation, and identical to its first run (`out/adv-final-reserve-sweep.txt`).
- **The probes** (`out/adv-final-repro.txt`):
  - F1's p2 solves on SPI2, all FT;
  - PB4 and PA15 are refused `MotorPinResetPull`;
  - the solver avoids PA15 unaided;
  - a hand TT pin at a connector is refused for each of the eight kinds.
- **`ardupilot-image` on `172e0657`:** all green, including the generated `serial-dshot-9600` motor board and `pinout`.

