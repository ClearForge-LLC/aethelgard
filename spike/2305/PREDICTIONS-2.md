# AET-WO-2305 §1.5 predictions: the re-judge on the branch, written before its runs

These are for the branch's CLI, with main 416a6dd3 merged and the ruling of 2026-11-20 05:10Z built. They were committed before `sh/run-all.sh` ran on it.

**How it works now.** A droop that releases a reset first runs its own prepared boot with no reset, and that boot's boot word fixes the bound:
- **bound** = that boot word's instant + 100 µs;
- **probe** = release + 10 ns + bound;
- **kind 32** holds the bound.

| Run | Bound (ns) | Probe (ns) | Witness lines at the probe | Verdict | Bytes against main |
|---|---|---|---|---|---|
| S-01 minimal `droop-bor3` | 125,700 | 1,357,400 | 2 | pass, matched | move: version, kind 32, kind 19's at_ns |
| S-01 minimal `droop-tolerated` | none (no release, so no kind 32) | — | — | pass | the version byte alone |
| S-01 bench `droop-bor3` | 128,130 | 1,359,830 | 2 | pass, matched | move as above, so the bench MANIFEST moves |
| `dshot600-droop` | 121,750 | 453,450 | 2 | pass, matched | move |
| `four-layer-droop` (BOR 0, no reset) | none | — | — | pass | version byte alone |
| `spi-only-droop-bor3` | 115,240 | 1,346,900 | 8 (U4's lines, as at +100 µs: the boot word is at 15 µs) | pass, matched | move |
| S-03 `droop-tolerated` | none | — | — | pass | version byte alone |
| **S-03 `droop-bor3` (new)** | 39,227,180 | ~40.9 ms (release ~1.6 ms) | 3 (U7: identity, configure, data) | **pass, matched: recovers** | new |
| **S-02-shaped `s02-droop`** | 39,172,410 | 39,505,090 | 2 (U2: identity) | **pass, matched: recovers** | — |
| **logger `droop-bor3`** | 19,622,270 | 20,854,130 | 3 (U20: identity, configure, data) | **pass, matched: recovers** | — |
| **ext-bmp581 `droop-bor3`** | 19,623,760 | 20,855,540 | 2 (U2: identity) | **pass, matched: recovers** | — |

**Undisturbed boots on the branch:** the same instants as on main. The branch changes no firmware and no boot.

**Sentinel-01's verdicts and counts:** none moves.
