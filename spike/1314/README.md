# AET-WO-1314 measured stop: evidence

- `measure.py <exe> <spec> <out> <label>`: emit, extract, per-net bound and via counts, the supply gate's verdict when gated.
- `sweep.py <exe> <repo> <out> [gated]`: the ring rule (`AET_RING_UM`) by bounded-net via cost (`AET_BOUNDED_VIA_COST`) on Sentinel-03 and the power fixture (as the CLI tests take it). `RINGS`, `COSTS`, `BOARDS` narrow it.
- `sweep1.jsonl`: the sweep; `sweep2.jsonl`: the same with `AET_BOUNDED_FIRST=1`; `gated.jsonl`: six gated runs.
- `goldens.out`: the fixture-emitting tier-1 tests under `AET_RING_UM=50 AET_BOUNDED_VIA_COST=48`.

The binary is this branch's head (`cargo build --release -p aethelgard-cli`); the knobs are measurement-only and are replaced by the ruled rule and costing.
