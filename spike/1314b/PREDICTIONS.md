# AET-WO-1314b §1.1 — predictions, committed before any run

Written 2026-11-20 before either variant was emitted or extracted. Nothing below has been run on this branch.

## The inputs

- `logger-spi-only-cap.json`: `spike/4600/logger-spi-only-cap.json` with `schema_version` 16 → 17 and **nothing else changed** (re-serialized with `json.dumps(indent=2)`, which reproduces the 4600 file byte for byte at 16; `diff` shows the one line).
- `logger-spi-only-cap-tol.json`: the above with Sentinel-03's `board.stackup.fab_tolerances` block (`crates/aethelgard-cli/tests/fixtures/sentinel-03.json`, JLCPCB, read 18 Nov 2026) copied in, and **nothing else**: the logger's 2-layer stackup, its `board.rules` (0.6/0.3 mm via) and its bounds stay. Only the `fab_tolerances` block came from Sentinel-03; its 4-layer OSH Park stackup did not.
- The logger declares no `net_classes`, so no class declares a via, and `ViaRingUnheld` cannot fire (`RingRule::unheld` reads only classes with `via_declared`, `eda/src/stack.rs` L94–104).
- The ring rule's pad at the 0.3 mm drill: 300 + 130 + 2·⌈75·√2⌉ + 2·50 = 744 µm → **0.75 mm** (rounded up to 10 µm, `RingRule::pad_nm`).

## The binary

`target/debug/aethelgard` built on Furnace from this branch's base `03d89554` (no change of mine in it), debug profile as 4600 used (its `run.sh`: `target/debug`, built in the image). Run on the host, not in a container: `emit` and `extract` run no tool. Each run under `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0`, `nice -n 10`, `/usr/bin/time -v` for wall time and peak RSS. Furnace has 20 CPUs, shared with about eight builders; the ladder routes its eight rungs on eight threads, so wall time depends on load.

## Predictions

| # | run | predicted | confidence |
|---|---|---|---|
| P1 | `validate` both | exit 0: spec 17 adds nothing the logger lacks | 85 % |
| P2 | `emit` as is | exit 0, no `capacitance_ladder` in the report (no ladder runs) | 95 % |
| P3 | `extract` as is | exit 1, one finding `capacitance_over_declared_bound` on `LOG_SCK`: **37,508 fF** (trace 15,793, vias 12,152, pads 1,563, pins 8,000) against 30,000, as 4600 measured; VIB's SCK and MOSI and LOG_MOSI pass (VIB 23.9 / 24.3 pF) | 75 % (main has moved since 4600's base; any routing change since then moves the figure) |
| P4 | `emit` with tolerances | exit 0, the ladder **fits**: a rung with every bounded net within 30 pF. Rung **4–7** (bounded nets routed first), `LOG_SCK` the smallest margin, its bound 25–29.5 pF with ≤ 3 vias (each about 1.75 pF at 0.75 mm) and a trace near today's 48.9 mm | 55 % fits; 35 % `CapacitanceOverBound` naming `LOG_SCK` nearest at 30–33 pF (the trace alone is about half the bound, and SCK sits across the package from the flash on 2 layers); 10 % other (`Unroutable` on the bigger via) |
| P5 | `extract` with tolerances (on P4's board, when there is one) | agrees with the ladder's own figure: exit 0, every line passes, `LOG_SCK`'s `c_bound_femtofarads` equal to the ladder's smallest | 90 % given P4 fits |
| P6 | wall time / peak | as is: emit under 20 s, peak under 1 GB; with tolerances: emit 3–10× the as-is wall time (eight routings on threads), peak under 3 GB; each `extract` under 5 s and 500 MB | 70 % |

If P4 is `CapacitanceOverBound`, that is the §7 "not a stop" outcome: the rung and the margin are reported, and the finding goes to the placer (`4600` F18, the capacitance-aware placement row). In that case `emit` writes no board, so the per-net margins at every rung are not in its report; the nearest rung's nets over the bound are, in its message.
