# AET-WO-1303b evidence (deleted by the build once cited)

## The measured stop after D5 (b) (FEEDBACK §2)

- `b/run.sh`: every board, net and pitch, through `crates/aethelgard-cli/examples/supply_stop.rs`
  (`cargo build --release --locked -p aethelgard-cli --example supply_stop`; run from the repository
  root; `BOARDS` defaults to `~/.cache/aet-1303b/boards`, the boards below). `b/*.out`: its output,
  one line per pin (bound, iterations, residual, time, the vias carrying 1 % or more).
- `b/ref.sh`, `b/ref-*.out`: each 4-layer board's worst VDD and GND pin alone at 31.25 and 15.625 um,
  for the uncertified reference.
- `b/shares-31250.out`: the worst and best pins' bounds split into sheets, junctions and barrels.
- `b/summarize.py` -> `b/summary.out`: the tables, the reference (labelled UNCERTIFIED) and the drop.
- `fill/`: the copper checks.
  - `prep/` (a crate outside the workspace, `1303` P4.7's) prepares a board as the DRC gate does;
    `gerbers/` are KiCad 10.0.7's In1/In2 after `--check-zones`, from the pinned tools image under
    `~/bin/capped-run`; `check_cells.py` tests each plane cell against them (`out-*.txt`,
    `result-*.json`); `islands.py` finds each violator's component in the model (`islands.out`).
  - `check_copper.py` tests every cell and junction square on every layer against an independent
    reading (`spike/1303/s4`'s `pcb.py`) and the refill, with an exact integer pass for cells the
    polygons flag; `copper-run4.sh`, `copper-*.out`. Cells come from the driver with
    `AET_DUMP_CELLS=<file>`.

## The §1.1 stop (FEEDBACK Part A)


- `oracle.diff`: the changes to `spike/1303/s4` (`pcb.py`, `rbound2.py`) that these boards need. Each is commented at its site:
  - circle and oval pads are read as plane obstacles;
  - each plane obstacle clears by its own size;
  - a run uses the vias of one geometry, and the other own-net vias stay holes;
  - tracks into an excluded via are dropped;
  - a pass through a pad keeps off every hole.
- `out/regress-spike-paths.out`: the modified oracle, run with the spike's own `run_paths2.py` on the spike's boards. It is byte-identical to `spike/1303/s4/paths.out`.
- `run13b.py`: the driver. The source is the regulator's current-entry pad (L2 on VDD_3V3); the return is the regulator's ground pads (U11).
- `build-fixtures.py`: the constructed 4-layer power fixture and the `-gnd` variants.
- `out/cov{,-gnd}-{0,0.075}.out`: every pin's bound or refusal, coefficients, pieces and plane hops (bound against the exact two-hole sheet), with the hole concentric and at ±0.075 mm.

Run: a venv with numpy and scipy; `python run13b.py <spike/1303/s4/conditions.json> <offset> <boards dir> [-gnd]`. The boards are `aethelgard emit` of each specification, as built from `main` `002dae07`.
