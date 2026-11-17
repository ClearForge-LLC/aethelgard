# spike/1313b — AET-WO-1313b: an order-independent compaction, and test pads beside their nets

The evidence for `docs/work-orders/AET-WO-1313b.FEEDBACK.md`. No crate changes on the branch: the prototypes are patches, applied in a scratch tree at `f845b6f` (`main`).

| File | What it is |
|---|---|
| `placer.patch` | the methods, by the letters in `AET_1313B`: `k` canonical order and a name-free target; `m` the centroid target; `s` sidestep compaction; `p` skyline repacking (`w` searches its width); `t` the test-pad rule; `q` `main`'s anchored parts by draw sequence (F1). `AET_1313B_DUMP` writes the placement, pads and counters. `AET_1313B_SHUFFLE` and `AET_1313B_REVTIE` are the independence checks. `AET_1313B_BLOCKS` adds the headroom blocks. Also adds `Anchor::seq` (`schematic*.rs`). With nothing set, every byte is `main`'s |
| `gnd-exemption-3402.patch` | 3402 §1.1's `GND` test-point exemption, applied in scratch for the two-`GND` probe only |
| `run.sh` | every gate for one specification (from `1313`); KiCad in the pinned image under `capped-run` |
| `renamings.sh`, `rename_fixture.py`, `rename_perm.py`, `rename_spec.py` | 20 renamings, every gate, fit, signature and least `extract` slack; the renamers are `1313`'s |
| `independence.sh`, `sig.py` | shuffle, reversed tie and order-only renaming against the name-free placement signature |
| `corpus.sh` | no-bound bytes: `main`'s binary against the prototype over the corpus |
| `tp_dist.py`, `probe-s01-two-gnd.json` | test-pad distances; Sentinel-01 with a second `GND` pad |
| `area.py`, `geom_check.py` | `1313`'s, kept for reuse |
| `results-*.txt` | every figure in FEEDBACK §2 |
