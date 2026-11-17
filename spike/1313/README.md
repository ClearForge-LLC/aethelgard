# spike/1313 — AET-WO-1313 §1, a declared board outline (measured)

Evidence for `docs/work-orders/AET-WO-1313.FEEDBACK.md` §2. Nothing here is built into the engine. `placer.patch` is the spike's placer: a diff against `pcb.rs` at `main` 9a37ae6 (1312 merged; `pcb.rs` unchanged on `main` since), applied only in a scratch worktree. It is driven by environment variables and never pushed as code.

| File | What it is |
|---|---|
| `make_fixture.py` | writes `crates/aethelgard-cli/tests/fixtures/sentinel-03.json`: V5 from `2b43672`'s `make_variants.py` (read from git), plus the flight sensors, the bench access and four layers |
| `placer.patch` | the options, by letter in `AET_1313_OPT`: `a` compaction, `b` bounded placer (box area), `B` bounded first-fit, `c` connectors on the left and right edges, `C` bottom and top, `d` the row's gap at 0.5 mm, `o` ICs in geometric order (`r` its first half only, reserve owners first; `s` its second, larger first). `AET_1313_BOUND=W,H` (mm) is the declared bound, `AET_1313_DUMP` writes the placed keep-out boxes and `AET_1313_ROUTE` the router's figures. With none set, every emitted byte is today's (checked on V0, V5-4L and the fixture) |
| `run.sh` | every gate for one specification: validate, generate, emit (timed), check erc, check drc --spec, drc with parity, check support, extract. KiCad runs in the pinned image through `~/bin/capped-run`, caps read back, swap 0 |
| `options.sh` | `run.sh` per option, one TSV row each → `results/options.tsv` |
| `renamings.sh`, `rename_fixture.py`, `rename_perm.py` | every gate on 20 renamings of the fixture: 1307's prefix renaming carried to connectors and test points (`rename_fixture.py`) → `results/renamings-fixture.txt`; the length-keeping permutation (`RENAMER=rename_perm.py`) → `results/renamings-perm.txt` |
| `k3.sh`, `rename_spec.py` | 1307's spec-level renamings (seeds 0–19) of the reference, D1 and D3, counted routed of 20 → `results/k3-1307-boards.tsv`. `rename_spec.py` is 1307's from `0a3397e4`, with one comment line added |
| `area.py` | §1.2: outline, keep-out sum, packing efficiency, connector band, largest empty rectangles → `results/area-v0-v5.txt`, `results/area-fixture.txt` |
| `geom_check.py` | every keep-out box inside the outline, none overlapping, every connector's cable side on its edge → `results/geom.txt` |
| `probes/` | `v5-4l.json` (V5 at four layers, for §1.2); `sentinel-03-nospitp.json` (the fixture without test pads on the six SPI lines); `sentinel-03-cs-free.json` (the fixture with IMU_CS left to the solver) |

Rerun, from a scratch worktree at `main` with `placer.patch` applied and `cargo build --release -p aethelgard-cli` built: `python3 -I <this dir>/make_fixture.py`, then the runners as their headers say.
