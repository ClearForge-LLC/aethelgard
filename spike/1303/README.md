# AET-WO-1303 spike probes (deleted at the build)

Python 3 with numpy, scipy (any recent); no network. Run from each directory.

- `boards/`: `aethelgard emit` of `examples/sentinel-01-minimal.json` (`ref/`) and
  `crates/aethelgard-cli/tests/fixtures/four-layer.json` (`four/`) at `121344d`, with `extract` output.
- `s3/`: the certified odd-mode C bracket (`bracket.py`).
  - `run_exact.py` → `exact.out`: against Cohn's exact coupled stripline.
  - `run_usb.py` → `usb.out`: IPC-2141 against the bracket; Z intervals over the 4-layer stackup; 1305's shell trial.
  - `run_sens.py` → `sens.out`: what widens the interval.
- `s4/`: the Thomson resistance bound (`rbound.py`, `pcb.py`).
  - `run_paths.py conditions.json` → `paths.out`.
  - `check_exact.py` → `check_exact.out`: bar, annulus, barrel, 90° bend bracket, transfer resistance.
- `img.sh`: the capped container runner; `caps.log` is its read-back (the guard stopped the only run: FEEDBACK D9).
- `research-sources.md`: the research pass's record, labelled.
