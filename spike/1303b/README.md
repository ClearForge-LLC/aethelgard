# AET-WO-1303b §1.1 evidence (deleted by the build once cited)

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
