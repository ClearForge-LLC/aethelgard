# AET-WO-4300i — the measurement's rules, committed with the predictions before any run

- **R0 · The universe.** A committed specification is a JSON file that meets all of these:
  - tracked by git;
  - outside `spike/` and `docs/`;
  - a JSON object;
  - `schema_version` equal to `SPECIFICATION_SCHEMA_VERSION` as `crates/aethelgard-ir/src/lib.rs` states it (27 on `f50c29f4`);
  - with an `mcu` object;
  - with no `base` key.

  Its name is the file's stem, prefixed `solve-` for files in a `solve/` directory (as `spike/1216/universe.sh`, re-keyed from 24 to the current constant). The list is `universe.sh`.
- **R1 · The baseline.** For each file, the CLI built from `f50c29f4` runs `validate`, `solve` and `emit`. Recorded per file:
  - each exit code;
  - the refusal kind;
  - the sha256 of `solve`'s stdout;
  - the sha256 of every file `emit` writes.

  Specifications meant to refuse are included. Each command runs twice, and the hashes must agree.
- **R2 · Connector-facing, and the preference.**
  - **Connector-facing:** an assignment whose net is the `net` of some `connectors[].pins[]` entry of the same specification. Test points (D2), `power.input` and the bench headers (`4300h` F2/F3) are outside it.
  - **The preference:** each such signal's candidate pins, in the solver's own order, partitioned stably, with FT first and non-FT after.
  - **Structure:** FT or not FT is `power-parts.json`'s MCU row (`io.pins`), the table `4300h` and `1211e` read.
  - **Moves:** a solver-chosen (non-hand) connector-facing assignment on a non-FT pin is listed as a move.
  - **Refusals:** a hand connector-facing assignment on a non-FT pin is listed as a refusal.
  - **Inference:** with the move list empty, the stable partition changes no first solution, because every chosen pin is already first among FT candidates it was ahead of. The claim "0 move" rests on that shape only.
- **R3 · Motor nets.** For every board with `motor_outputs`, one row per output:
  - its label (the net);
  - its pin, and whether it is hand or solved;
  - its protocol;
  - its idle;
  - `implied_pupd` as the solved topology implies it: `idle_pupd`'s rule, pull-up for `{"level": "high"}`, else pull-down;
  - its far ends: the connector pins whose net it is, the test points whose net it is, and the pin-device bridge inputs that name it;
  - whether `emit`'s `.kicad_pcb` carries a net of that name.
- **R4 · Reservations.** For Sentinel-03, every solved pin on PA11, PA12 or PC9 is listed, hand or solved, both in the baseline and after the PA8 copy.
- **R5 · The re-pin.** A scratch copy of Sentinel-03 is made with `motor_outputs[MOTOR1].pin` set to `"PA8"` and nothing else changed. `validate` and `solve` run on it, and its solved `assignments` are diffed against the baseline's.
