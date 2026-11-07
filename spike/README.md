# AET-WO-1200 spike probes

Measurements for `docs/work-orders/AET-WO-1200.md` §0; the answers are in `AET-WO-1200.FEEDBACK.md` under "Spike". The build half deletes this directory.

- `solver-probe/`: the search S1 describes, with optional pruning flags, a node budget, the S3 refusals and the S4 output shape (`SpikeTopology`). It is its own Cargo workspace, because `crates/aethelgard-cli/tests/workspace_shape.rs` pins the root to exactly seven members. It depends on `aethelgard-ir` and `aethelgard-svd-hal` by path, and on nothing the root does not already lock.
  - `src/bin/measure.rs`: timings, node counts and output digests per specification and pruning setting.
  - `src/bin/dump.rs`: prints every I2C and SPI signal in the table with its pin and AF, which is how the instance list was read.
  - `src/bin/fuzz.rs`: random specifications; checks that every setting agrees and verifies each solution independently of the search.
  - `tests/gates.rs`: the Furnace digests pinned, two solves per specification, and the clause-2 message.
- `specs/`: the stress specification, one specification per S3 refusal, and three adversarial infeasible ones (`pigeonhole-i2c`, `pigeonhole-spi`, `thrash-pins`).
- `results/`: the recorded output of each command below.
- `run.sh gates|measure|fuzz SEED COUNT`: every command FEEDBACK cites, run from the repo root.
