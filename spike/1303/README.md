# AET-WO-1303 spike probes (deleted at the build)

Python 3 with numpy, scipy (any recent); no network. Run from each directory.

- `boards/`: `aethelgard emit` of `examples/sentinel-01-minimal.json` (`ref/`) and
  `crates/aethelgard-cli/tests/fixtures/four-layer.json` (`four/`) at `121344d`, with `extract` output.
- `s3/`: the certified odd-mode C bracket (`bracket.py`).
  - `run_exact.py` → `exact.out`: against Cohn's exact coupled stripline.
  - `run_usb.py` → `usb.out`: IPC-2141 against the bracket; Z intervals over the 4-layer stackup; 1305's shell trial.
  - `run_sens.py` → `sens.out`: what widens the interval.
  - `run_nested.py` → `nested.out`: `microstrip_odd`'s bounds under nested refinement.
- `s4/`: the Thomson resistance bound (`rbound.py` pieces, `rbound2.py` graph and per-path recomputation, `pcb.py`).
  - `run_paths2.py conditions.json` → `paths.out`: the reported bounds, hole anywhere within ±0.075 mm.
  - `run_paths2.py conditions-concentric.json` → `paths-concentric.out`: the same, hole concentric (a condition).
  - `run_paths.py` → `paths-v1-inadmissible.out`: the first version, kept as the record of FEEDBACK F7; not a bound.
  - `check_exact.py` → `check_exact.out`: bar, annulus, barrel, 90° bend bracket, transfer resistance.
  - `check_barrel.py` → `check_barrel.out`: the spread barrel against a certified lower bound on the unrolled sheet.
  - `check_offset.py` → `check_offset.out`: hole-offset sensitivity of the via entry.
  - `check_fan_plane.py` → `check_fan_plane.out`: the bowtie against the exact two-hole sheet; the general fan and the pad pass against certified lower bounds; via-entry admissibility by brute force.
- `img.sh`: the capped container runner, by the architect's replaced rule 3 (a capped systemd user scope plus `podman --cgroups=disabled`). `caps.log` is its read-back; its first line is the earlier flag-based runner's stop (FEEDBACK D9).
- `p47/`: P4.7. `prep/` is a standalone probe that prepares a board as the DRC gate does. `prepared/` holds the 4-layer board so prepared, and `prepared/out/` its refilled `In1`/`In2` Gerbers. `out/` holds the unprepared board's (`boards/four/`, copied in for the run). `analyse.py` → `analyse.out`.
- `research-sources.md`: the research pass's record, labelled.
