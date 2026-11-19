# AET-WO-4600 — Spike: two boards that are not flight controllers, a sensor logger and a small motor driver, run through every gate with no engine change, to find the engine's Sentinel-shaped assumptions before P4's gate does

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`cd43b207` at writing; verify live HEAD).
**Branch:** `wo/aet-4600-second-board-spike`.
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4, the exit-gate clause "Aethelgard is general, not Sentinel's" (`docs/roadmap.md` L455). Written 2026-11-19. **Cadence: spike, ending in a stop.** Rulings on findings are the architect's. Anything that changes what the clause means is Scotty's: it is his clause, and a gate is "refined after P3, never weakened" (L448).

**Grounds:**
- **The clause** (L455): "A second board specification that is not a flight controller (a sensor logger or a small motor driver, written then) passes every gate with no engine change (Scotty, 2026-11-15)."
  - **"Written then"** means at the gate. **This spike does not satisfy the clause, and its FEEDBACK says so in its first line.** Its job is to find now, while there is time to fix them, every place the engine would refuse or mis-judge a board that is not Sentinel's.
- **§6's rule** (L479): "Capability enters through the specification, never as a board's branch … No engine code names a board; review checks for it. Measured 2026-11-15: outside tests and comments, no crate names Sentinel."
- **The northstar's failure list** (§6): "one microcontroller family is supported so deeply that the intermediate representation is secretly that vendor's register map", and "the specification becomes so hard to write that people hand-edit outputs". A second board is the cheapest test of both.
- **What the specification can say today** (`crates/aethelgard-ir/src/lib.rs`, `SPECIFICATION_SCHEMA_VERSION = 16` at L81; `Specification` at L102–152):
  - I2C and SPI buses and devices (L337–388). A device is a part with a bus. There is no device without a bus, no plain GPIO, and no ADC input.
  - `motor_outputs` (`motor.rs` L82–90): `pwm`, `dshot300`, `dshot600`. PWM is "the pulse range an ESC reads", and "the firmware starts the output at the range's minimum, its idle pulse" (`motor.rs` L66–70). `validate` refuses `pulse_min_us` = 0 (`PulseRange`, `lib.rs` L3543). The timer ticks in whole µs (`motor.rs` L253–259).
  - `serial_ports` with a role of `console`, `mavlink`, `gps`, `rc_crsf` or `rc_sbus` (`serial.rs` L31–37).
  - `power` or `null` (`power.rs` L109–127). A tree has one input and one `battery`: a series LiPo-style pack of `cells` (L200–213).
  - `connectors`: four JST parts (`connectors.rs` L67–110) and one power-in, `XT30PW-M` (L165).
- **The parts the engine knows** (compiled in by `include_str!`: `parts.rs` L45, `power_parts.rs` L31):
  - `parts.json`, schema 10, seven rows: BME280, BMP581, ICM20948 (eol, stand-in), ICM42688P, ICP10111 (eol, stand-in), TMP117 (no Renode model), W25Q64JVxIQ (nrnd).
  - `power-parts.json`, schema 4, fourteen rows. `STM32H753ZITx` is the only MCU row (`RUL-08`). One buck (LMR33630ADDA, VIN ≥ 3.8 V), one LDO (TLV75518PDBV, 1.8 V fixed), one translator, resistors and capacitors.
  - **The emitter also names parts in code.** `aethelgard-eda` `schematic.rs` matches part names from L1326 and refuses others as `UnknownPart` (L1388). `names.rs` L32 lists six vendored parts. ICP10111 has a table row but no emitter arm.
- **What the engine has run.** Every committed specification that reaches `generate` (`examples/`, `crates/*/tests/fixtures/`) is at schema 16 on `STM32H753ZITx`, and carries at least one I2C bus and one SPI bus with devices on them. Only the solver's fixtures (`fixtures/solve/`) go without one, and they stop at `solve`. No committed board has motor outputs without sensors. No committed board is anything but a Sentinel, a Sentinel-shaped fixture, or a probe shaped like Sentinel-01.
- **Sentinel-01 is already a sensor logger.** It is the northstar's "telemetry probe": BME280 and TMP117 on I2C, a W25Q64 on SPI, a console (`examples/sentinel-01-minimal.json`). A logger that copies its shape tests nothing. The logger here must differ from it on purpose.
- **`verify`'s gates** (`crates/aethelgard-cli/src/verify.rs` `run_gates`, L1532–1807), in order: `erc`, `drc`, `support`, `extract`, `prove` (one per generated harness; none is a `no_harness` error, L1600–1606), `build`, `simulate`, `campaign`.
  - Before any gate, with `--fab-out`: coverage (`coverage`, L882–946; refusal `CampaignCoverageShort`, L1221–1232), and every unwitnessed ledger row acknowledged (`UnwitnessedFieldNotAcknowledged`, L1233–1240).
  - Coverage marks every rail other than the MCU's supply unmet, and `battery` unmet whenever a tree is declared (L930–944).
  - `campaign` refuses any specification with a power tree, `PowerNotSimulated` (`cli/src/campaign.rs` L377–390). `2304` owns that row.
- **The CLI's other subcommands** (`cli/src/main.rs` L69–300): `validate`, `solve`, `emit`, `generate` (and `generate hwdef`), `build`, `extract`, `check erc|drc|support|pinout|speeds`, `prove`, `simulate`, `campaign`, `replay`, `attribute`, `propose`.
- **The roadmap row a logger hits:** no per-device I2C absence in the twin; "its only I2C absence is the whole bus" (§7.1 L836, `3304` F1). Owner: a twin or chaos work order after `4400g`.

> **What this is:**
> - two non-flight boards, each proposed from the parts tables as they stand, with the rows a real one would need named;
> - each board, and a few one-change variants, run through every subcommand in the pinned image;
> - every refusal, failure and suspicious pass recorded by name and `file:line`, and classified;
> - the §6 board-name check, run and reported hit by hit;
> - the follow-up work orders this implies, put to the architect as D-decisions.
>
> It is NOT:
> - the clause's board. That is written at the gate (D6);
> - a fix. No crate, model, parts table, golden, test, example, scenario, board, workflow or `ci/` file changes. A refusal is recorded, never patched around in the engine;
> - a parts-selection spike. A missing row is named with the datasheet it would cite, never added;
> - a second MCU. `RUL-08` anchors one; more families are "later, not yet a phase" (L457). Holding the MCU fixed is ruled scope, not a finding;
> - `2304`'s battery chain, `4240`'s crystal, or `1315`'s hot-plug parts. Where a candidate hits them, cite the owner and move on.

## 1. Spike

Every figure is labelled **measured**, **read at source** or **inferred**. **Commit `spike/4600/PREDICTIONS.md` before any measured run**, as `2303` and `2304` did. It holds, per specification and per subcommand: the expected exit code, the refusal kind if any, and the class. Copy, confirm or correct the architect's reading in §1.6. A run added later is reported as unpredicted.

1. **The two boards, proposed concretely, from existing rows only.** Write each under `spike/4600/` (never `examples/`), at schema 16, with a `project_name` that names no Sentinel.
   - **L, a field logger** (`spike/4600/logger.json`). It must differ from Sentinel-01 on purpose. The architect's proposal (change it if a better shape exists, and say why):
     - two I2C buses: `TEMP` with four TMP117s at 0x48–0x4B, and `BARO` with one BMP581 at 0x46. No BME280;
     - one SPI bus, `LOG`, with a W25Q64JVxIQ, its `nrnd` acknowledged in `accept_lifecycle`;
     - one serial port, role `console`, for the readout;
     - bench power at 3.3 V through an `SM04B-GHS-TB` (`mcu.supply_source`), SWD on an `SM06B-SRSS-TB`, two layers;
     - every `acknowledge_unwitnessed` row `verify --fab-out` demands, written by hand.
   - **M, a small motor driver** (`spike/4600/driver.json`). With today's rows it cannot carry a driver IC (§1.2), so it drives one off-board. The architect's proposal:
     - two outputs for an off-board H-bridge's inputs: `pwm`, 20 kHz, `pulse_min_us` 1, `pulse_max_us` 49;
     - two servo outputs: `pwm`, 50 Hz, 1000–2000 µs;
     - a command port, role `console` (no neutral role exists, §1.4);
     - **no I2C or SPI bus at all.** No board that reaches `generate` has had none;
     - bench power at 3.3 V as L; the outputs leave on an `SM06B-GHS-TB`.
   - **One-change variants**, each its own file, each named for the one thing it changes. The architect's list:
     - `logger-i2c-only.json`: L without its SPI bus;
     - `logger-spi-only.json`: a vibration logger, ICM42688P and W25Q64 on two SPI buses, no I2C bus;
     - `logger-1s.json`: L powered from a 1S cell through the power tree;
     - `logger-offboard.json`: one TMP117 on the far side of a connector, the way a logger's probe hangs on a cable;
     - `driver-tree.json`: M with a power tree from an XT30 (3S–6S, LMR33630 to `VDD_3V3` and `VDD_5V` for the servos), the shape of `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json`;
     - `driver-idle-zero.json`: M with the H-bridge outputs at `pulse_min_us` 0, an H-bridge's real idle (coast).

     Add a variant wherever one isolates a refusal. Never more than one change per variant.
   - **Authoring cost, measured.** For each of L and M, record the bytes written, the number of acknowledgement rows, and the wall time to a specification that `validate` accepts. Sentinel-01-minimal is 16,596 bytes, most of them acknowledgements. A specification so hard to write that people stop writing it is a northstar §6 failure.
2. **What a real one would need.** For each board, name the rows and constructs missing from the engine. The architect expects at least:
   - **driver:** a driver IC with no bus (a TI DRV8833 or DRV8871 class part): a device kind without a bus, its direction, enable, sleep and fault pins as GPIO, and a current-sense input (ADC). Also a motor-current connector, and a fault kind on the motor rail;
   - **logger:** a 3.3 V regulator from 1S (an LDO or a buck-boost), an RTC crystal (LSE; say whether `4240` covers it), and a removable medium if the logger needs one (an SD card on SPI).
   - For each, cite the datasheet you would use (document number and revision), and say what the engine would need beyond the row: a parts-table row only, a row plus an emitter arm (`schematic.rs`), a model under `models/renode/`, a specification field, or a fault kind. **Do not add any of them.**
3. **The runs.** On each specification, in this order, inside the pinned image (`container/IMAGE`, `aethelgard-tools@sha256:e5fc7ddb…` at writing; read it live):
   - `validate`; `solve`; `emit`; `check erc`; `check drc --spec --schematic`; `check support`; `check speeds`; `extract`;
   - `generate`; `build`; `prove`, once per generated harness; `simulate --firmware --spec --for` at the window `verify` uses;
   - `campaign` over scenarios you write under `spike/4600/scenarios/` (never `scenarios/`), then `replay` and `attribute` on one run each;
   - `verify` without `--fab-out`; then `verify --scenarios … --fab-out` into a scratch directory outside the repository;
   - `generate hwdef` and `check pinout`, recorded but not counted as gates for a board with no ArduPilot. D1 rules whether they count.
   - **Scenarios.** For each board, the set that would meet `verify`'s coverage: a baseline; a tolerated fault on each bus; a droop on the MCU's supply; `spi_device_absent` for each SPI device; `serial_rx_open` for each port. Say where no fault kind can cover a target (`chaos/src/record.rs` L173–191 lists the six overrides).
   - **When a step refuses,** later steps that need its output are `not_run`, by name. Steps that do not need it still run. A variant that routes around the refusal is a new file, labelled.
   - Build the CLI from the base SHA inside the image, as CI's `tools` job does, or use the engine image published for that SHA. Record which, with its digest.
4. **Every outcome, classified.** One row per (specification, subcommand): exit code, refusal kind or verdict, the `file:line` where it arises (read at source, not guessed from the message), and one class:
   - **(a)** a correct refusal of a bad specification. Say what was bad;
   - **(b)** a Sentinel-shaped engine assumption. **This is the finding.** It includes a **pass that should not pass**: a gate green on something the board would get wrong. The H-bridge outputs starting at their minimum pulse, so a motor turns at boot, is the architect's suspect;
   - **(c)** a missing capability the gate would need: a parts row, a peripheral, a specification construct, or a fault kind. Name the owner if a row in §7.1 already holds it;
   - **(d)** a twin limit under `INV-07`: what the twin cannot see on this board, so only the bench would judge it. TMP117 has no model, so a logger built on it has no witnessed I2C identity at all.

   Also report, for each candidate that passes `verify`: its gate list with verdicts, the coverage it met, its ledger's row count and how many rows had to be acknowledged, peak RSS and wall time per gate.
5. **The §6 board-name check.**
   - Run `git grep -niI sentinel` over `crates/*/src`, `bridge/src`, `models/`, `Cargo.toml` and `crates/*/Cargo.toml`. Classify each hit: inside a `#[cfg(test)]` module or test file; a comment; a string in non-test code; a manifest; or the word used in its plain sense.
   - Then grep for board-shaped literals that are not the name: the fixtures' bus ids (`SENSORS`, `EXT`, `IMU`, `FLASH` as a bus), references (`U2`, `U4`, `J3`), net names (`VDD_3V3`, `I2C_SDA`), and project names. Report each one in non-test code with what it decides.
   - **The architect's reading, unconfirmed:**
     - one string in non-test code: `cli/src/ledger.rs` L388, `GATE_SPI`, cites `boards/sentinel-01/sentinel-01.repl` as its evidence;
     - two manifest lines: `Cargo.toml` L56 (`sentinel-01-board`) and `crates/aethelgard-svd-hal/Cargo.toml` L25, a dev-dependency of the trybuild suite;
     - comments: `eda/src/pcb.rs` L672 and L897, `eda/src/route.rs` L137, `ir/src/connectors.rs` L66, `ir/src/ds009.rs` L12;
     - not a board: `cli/src/renode.rs`'s `sentinels` (L296–3522), marker lines in a log;
     - board-shaped literals in non-test code: `chaos/src/synthetic.rs` L26 (`VDD_3V3`, `VBAT`); `ir/src/ds009.rs` L70 (DS-009's role, a standard's pin, which looks legitimate). The `FLASH` hits are the MCU's flash peripheral.
   - **Part names in code are a separate question** from board names. Report them separately (`schematic.rs` L1326–1499, `names.rs` L32–45, `cli/src/ledger.rs` L1335–1352, `twin/src/model.rs` L144–149). Every new part costs code there today. D4 asks whether that is an "engine change".
6. **The architect's reading of the runs**, for `PREDICTIONS.md` to confirm or correct:

   | Spec | Step | Expected | Class |
   |---|---|---|---|
   | L | `validate` … `verify` (no fab) | pass. TMP117s unmodelled, listed (`twin/src/boot.rs` L505, L565) | (d) |
   | L | coverage | `TEMP` covered by a tolerated pull-up fault, judged by ngspice while no TMP117 answers in the twin. A bus "covered" with no device witnessed | (b) or (d): say which |
   | L | `--fab-out` | written, once every ledger row is acknowledged | — |
   | `logger-i2c-only` | `generate` onward | unknown: no generated board has had no SPI bus. A refusal or a panic here is (b) | ? |
   | `logger-spi-only` | `generate` onward | unknown: no generated board has had no I2C bus | ? |
   | `logger-1s` | `validate` | refused: the buck needs VIN ≥ 3.8 V and the LDO gives 1.8 V | (a), and (c): no 3.3 V regulator from 1S |
   | `logger-offboard` | authoring | not expressible: a device is placed on the board; an off-board one cannot be declared or read | (c) |
   | M | `validate` | pass | — |
   | M | `generate` onward | unknown: no generated board has had no bus. `no_harness` is possible if harnesses come only from buses (`verify.rs` L1600–1606) | ? |
   | M | `simulate` | pass, with H-bridge outputs at 1 µs of 50 µs (2 %) from boot | **(b)**: an H-bridge's idle is 0 %, the motor runs at boot |
   | M | duty resolution | 50 steps at 20 kHz (1 µs ticks) | (b) |
   | `driver-idle-zero` | `validate` | `PulseRange` (`lib.rs` L3543) | **(b)**: the model's idle is a servo's, not an H-bridge's |
   | `driver-tree` | `campaign` | `PowerNotSimulated` (`campaign.rs` L382) | (c), owner `2304` |
   | `driver-tree` | coverage | `VDD_5V` and `battery` unmet (`verify.rs` L930–944) | (c), owner `2304` |
   | M, L | `generate hwdef` | refused or meaningless: an ArduPilot emitter | not a gate (D1) |
   | all | power input | only an XT30 and a cell pack can be declared; a wall adapter or USB supply cannot | (b) |
   | all | serial roles | only flight-ecosystem roles and `console` | (b) or not: say |

7. **What `4400g` moves.** It is open as #238. It takes the specification from 16 to 17 (`mcu.foreign_clock`, refused on a generated-firmware run) and the trace from 13 to 14, and moves the twin's timers to the silicon's 64 MHz.
   - **Measure on `main`** at the SHA the kickoff names.
   - **If `4400g` merges mid-spike,** do not rebase measured runs. On the new `main`:
     - bump each spike specification's `schema_version` to 17. Confirm that is the only change `validate` needs;
     - re-run M's and `driver-idle-zero`'s `simulate` and `campaign`, because the PWM witness's clock changes;
     - re-run one `replay`, because the trace version changes;
     - give both figures for anything that moved.
   - **After it merges, whenever that is,** the follow-up work orders re-check: M's PWM timing verdicts, every archive hash quoted, and that a spike specification at 17 still validates.

## 2. Stop

**STOP** with FEEDBACK (`docs/work-orders/AET-WO-4600.FEEDBACK.md`) holding:
- **first line:** "This spike does not satisfy roadmap L455. The clause's board is written at the gate."
- the two boards as proposed, any changes to the architect's proposal and why, and §1.1's authoring cost;
- §1.2's missing rows and constructs, with datasheets;
- §1.6's predictions beside §1.3's results, one table per specification, with §1.4's class on every row;
- for each candidate that reached `verify`: the gate list, coverage, ledger counts, RSS and wall time;
- §1.5's hits, classified;
- §1.7's movers;
- findings in the house schema: `finding · where · type · recommendation · decision-needed`. Every (b) and (c) row is a finding. Type is `note`, `risk`, `scope-question` or `bug`;
- what did not work, and why;
- what was deliberately not built.

**Decisions to put, each with a recommendation:**
- **D1 · What "every gate" means for the clause.**
  - `verify` passing, or `verify --fab-out` writing a package (`INV-03` counts the campaign). Whether `check speeds` counts. Whether `generate hwdef` and `check pinout` count for a board with no ArduPilot.
  - The architect leans to: `verify --fab-out` writes a package, `check speeds` passes, and hwdef and pinout are excluded by name. Any wording change is Scotty's, as an amendment. It may refine the clause, never weaken it.
- **D2 · Which board the gate should be.** A logger, a driver, or either. Say which exercises more of the engine. Say what each would need first (§1.2).
- **D3 · The (b) findings.** One follow-up work order per finding, or grouped by crate. Give each an owner, and its order against `4500`. Write each finding in general terms ("a PWM output whose idle is 0 % duty"), never as "make `driver.json` pass".
- **D4 · Is a parts row an engine change?** The tables are compiled into the binary (`parts.rs` L45, `power_parts.rs` L31), and every part also needs an emitter arm (`schematic.rs`) and a vendored symbol.
  - Recommend whether the clause allows a cited data row added at the gate.
  - Recommend whether the emitter's per-part arms should become data, so that a part enters through a row alone. That is the parts-table version of §6's rule.
- **D5 · The board-name hits.** For each: remove, keep with a reason, or move into a test. Recommend whether §6's "review checks for it" becomes a tier-1 test (beside `crates/aethelgard-cli/tests/workspace_shape.rs`), so that review need not.
- **D6 · Keeping the clause honest.** The gate's board is "written then". Recommend how:
  - it is written at the gate by a fresh session from a short brief, not copied from `spike/4600/`;
  - `spike/4600/` is deleted when its follow-ups merge (§6: a spike merged as the record is deleted by its build);
  - the independent review at the gate (§6) is told to check that the board was not fitted to the engine.
- **D7 · Order against the schema epoch.** The P4 baseline is named once `4500` settles Sentinel-02's shape, and later additions are additive (L439). Say which (c) findings add specification fields, and whether they must land before the baseline.

Push an `AET-WO-4600 STOPPED:` commit and open one PR. It is merged as the record (FEEDBACK plus `spike/4600/`) after the ruling. Write `~/.cache/aet-4600/pr-title-body.md`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02`, `INV-03`, `INV-05`, `INV-06`, `INV-07`; `RUL-08`, `RUL-12`, `RUL-14`, `RUL-16`, `RUL-19`, `RUL-20`, `RUL-21`. Roadmap §6: capability enters through the specification (L479); a spike may probe in any language Furnace or the image runs; a spike merged as the record is deleted by its build.
- **Fence:**
  - `spike/4600/` and `docs/work-orders/AET-WO-4600.FEEDBACK.md` only.
  - The specifications, scenarios, `PREDICTIONS.md`, each run's stdout line, and one script that remakes every run (`spike/4600/run.sh`) live under `spike/4600/`.
  - Archives, generated trees, ELFs and packages stay outside the repository, in `~/.cache/aet-4600/`. Quote each one's sha256.
  - **No change** to any crate, `bridge/`, `models/`, parts table, golden, test, `examples/`, `scenarios/`, `boards/`, `fab/`, `.github/` or `ci/`. `git diff --stat <base> -- crates bridge models examples scenarios boards fab ci .github container Cargo.toml Cargo.lock` is empty at the stop.
  - The steering documents, `README.md`, `LICENSE-*` and `docs/inputs/**` are protected. A finding about them goes in FEEDBACK.
- **Running beside you:**
  - `AET-WO-4400g` (#238: twin, chaos, ir, the CLI's campaign; specification 16 → 17, trace 13 → 14). See §1.7;
  - `AET-WO-1910` (#233) and `1911`, which own CI;
  - `AET-WO-1315` (`aethelgard-eda`: the hot-plug bulk capacitor and control pins on boards with a power tree). If it merges, re-run `driver-tree`'s `emit`, `check erc`, `check drc` and `check support`. Boards without a tree stay byte-identical by its own rule;
  - `AET-WO-2304` and `AET-WO-4240` (spikes: the battery under the motors; the HSE clock tree). Cite them as owners, never re-measure their questions;
  - `AET-WO-4404` (the hwdef and pincheck build). Your `generate hwdef` and `check pinout` runs are on `main`. Say if `4404` would move them.

  You share no file with any of them.
- **Containers:**
  - One container of yours at a time, under `capped-run`, with `--network none`, at `nice -n 10`.
  - `--mem 8G` for every subcommand but `verify`; `--mem 16G` for `verify`. `verify` can need more than 8 GiB (`2304` §1.10).
  - Read the caps back inside the scope before each run, into `caps.log`.
  - For scale: the Sentinel-03 every-gate test weighs 210 s (`ci/test-weights.tsv` L90), and one `verify --fab-out` of Sentinel-01 takes 510–579 s on CI (`.github/workflows/ci.yml` L412).
- On Furnace: no `sudo`, no system packages, no credentials. Push with `gh-write`.
- One PR, left unmerged.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **Getting further would need an engine change.** Record the refusal, mark what depends on it `not_run`, and carry on with what does not. Route around it only with a labelled variant specification. Never patch a crate, a table or a model, even in scratch, to see what comes next: that measures an engine that does not exist.
- **A run is killed, or passes 900‰ of its cap.** Stop with the numbers. A cap is never raised here.
- **A gate passes something §1.4 classes (b): a pass that should not pass.** Not a stop. Record it as a `risk`, with the gate that passed it. If `verify --fab-out` wrote a package for that board, quote the manifest's sha256 only. Never commit, upload or order it.
- **Neither board, nor any variant, can be expressed with today's rows.** Stop with §1.2's list. D2 then becomes "what must exist first".
- **A finding would change an invariant, or what the clause means.** Do not prototype it. Put it as a D, marked Scotty's.
- **`4400g` merges and moves a result a recommendation rests on.** Re-measure it on the new `main` before the stop, or stop with both figures.
