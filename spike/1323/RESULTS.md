# AET-WO-1323 — results of §1.1, beside `PREDICTIONS.md`

Measured 2026-11-23 on Furnace. The base is a `git archive` of `main` `cd79024c`; each prototype is that archive with one patch, built in its own target dir, release unless stated. Evidence is in `~/.cache/aet-1323/` (`logs/`, `out-*`, the trees). Every script named here is in `spike/1323/`. `inst.py` is the shim's first form. The timers added later (the solve's phases, the energy by element kind) and the bound trip are in `patches/inst.patch`, the shim as last run. `patches/all.patch` is the combined tree.

## 1. The baseline

- **B1, hit.** The sweep selects **34** specifications: **25** emit (exit 0), **9** exit 2.
- **B2, hit.** `spike/1319/harness/run.sh` ran unchanged on the base: the sweep, `d4diff` and `d1319` all completed (exit 0).
- **B3, hit.** `sha256-base.txt`: a sha256 per emitted file, 75 files (25 boards × `.kicad_pcb`, `.kicad_sch`, `.bom.csv`). The fab package's specification (`examples/sentinel-01.json`, R1) emits `sentinel-01-bench.kicad_pcb` `401b9e1e…` and `.kicad_sch` `aa9d78af…`.
- **The shim moves nothing.** The measuring shim (`inst.py`: counters and timers only) emitted all 75 files byte-identical to base.

## 2. (1) The work bound

`inst.py` counts, per `route_with` call: `search` expansions (states popped and not stale), `pair_run` expansions, and the nodes `zone_blockers` and `components` visit (`flood`), with wall time and the time inside `search()`. `work-sweep.sh` logs every committed specification; the workspace suite logs every routing any test makes.

**Every committed specification** (release; one routing per spec except Sentinel-03's ladder, 8). The ten heaviest, by search expansions:

| specification | routings | search | flood | wall | in search | rip-ups |
|---|---|---|---|---|---|---|
| `ir/…/sentinel-02-power.json` | 1 | **86 374 584** | 32 603 017 | 11.6 s | 7.4 s | 147 |
| `cli/…/d3-24-i2c-12-spi.json` | 1 | 48 402 186 | 15 832 743 | 6.1 s | 4.2 s | 135 |
| `cli/…/sentinel-03.json` (heaviest rung, 96/first) | 8 | 43 322 777 | 8 589 957 | 5.2 s | 3.7 s | 77 |
| `cli/…/solve/stress.json` | 1 | 35 119 802 | 3 812 165 | 4.1 s | 3.0 s | 112 |
| `cli/…/hse-crystal-16.json` | 1 | 23 064 834 | 4 801 961 | 2.6 s | 1.9 s | 117 |
| `cli/…/sentinel-02-shaped.json` | 1 | 18 062 455 | 3 892 239 | 1.9 s | 1.5 s | 51 |
| `examples/sentinel-01-ext-bmp581.json` | 1 | 14 065 638 | 3 826 311 | 1.5 s | 1.2 s | 46 |
| `cli/…/serial-four.json` | 1 | 12 979 354 | 3 108 873 | 1.3 s | 1.1 s | 31 |
| `cli/…/connectors.json` | 1 | 11 080 371 | 1 729 895 | 1.0 s | 0.9 s | 25 |
| `examples/sentinel-01.json` | 1 | 9 587 815 | 2 887 519 | 1.1 s | 0.8 s | 40 |

`pair_run` expanded 24 176 states on the only board with a pair (`pair-standin.json`), 0 elsewhere.

**The adversary** (`adv1307.rs`, a **reconstruction**: R5), seed 1307, release, under a capped scope:

| | `1307` A4 (its own board and machine) | reconstruction |
|---|---|---|
| outcome | refused by `RIPUPS` | refused by `RIPUPS`: `/N076: still blocked by [/N084, /N093, /N207, /N279, /N073] after 500 rip-ups` |
| wall | 616 s | **203.1 s** |
| peak RSS | 568 MB | 353 MB |
| search expansions | — | **1 370 785 826** (15.9 × the heaviest committed routing) |
| time in `search()` | — | 182.3 s (**90 %**) |
| flood | — | 0 (no stitched net) |

- **W1, as built in the shim.** The count is per routing (`route_with`), and the capacitance ladder routes Sentinel-03 eight times.
- **W2, hit.** `search` holds 90 % of the adversary's time. On the committed boards it holds 64–92 %; the rest is chiefly `flood` (`zone_blockers`' fence flood after every pass, 32.6 M nodes on the heaviest).
- **W3, hit.** The count is the same in release and in the dev profile (`eda` at `opt-level = 2`, as CI builds it), and on every repeat. `sentinel-02-power` counts 86 374 584 in the release sweep and in each of its three dev-profile routings in the suite. Each of Sentinel-03's eight rungs gives in nine dev-profile routings exactly its release count.
- **W4, wrong in order.** I predicted Sentinel-03 or d3. The heaviest committed routing is `sentinel-02-power` (an IR fixture that emits), at 8.6 × 10⁷, the top of my range.
- **W5, hit.** The suite's heaviest is 1.37 × the committed heaviest (§2.1).
- **W6, hit.** The adversary's count is 15.9 × the heaviest committed board's.
- **W7:** the name is for the ruling (§7).

### 2.1 Every routing in `cargo test --workspace`

The shimmed tree, dev profile, `RUST_TEST_THREADS=6`, in a capped scope: 18 min 2 s, peak 2.1 GB. Every test passed, with two exceptions, neither of them the shim's counting:
- 17 tests failed by `ScratchUnsafe` because my own kit directory was mode 775. The CLI refused it, rightly. Re-run with the directory at 700, all 17 pass, and their routings are in the log.
- `strict_fence` refused the spike's extra module file, by design.

**701 routings** were logged: 675 completed and 26 refused, the refusals as tests assert.

| | search | flood | search + flood |
|---|---|---|---|
| heaviest routing in the suite: `power_board_tier1`'s `power_tree()` fixture through the ladder, a 112 × 89 mm variant, 64 parts, 139 nets, rung 320/false | **118 746 381** | 24 120 869 | **142 867 250** |
| heaviest committed specification (`sentinel-02-power`) | 86 374 584 | 32 603 017 | 118 977 601 |
| heaviest flood in the suite (a 204 × 129 mm board, 91 parts) | 54 790 592 | 66 339 967 | 121 130 559 |
| the adversary (reconstruction), to `RIPUPS` | 1 370 785 826 | 0 | 1 370 785 826 |

**The adversary to a candidate bound** (`AET1323_BOUND`, a spike trip at search + flood > B, read once per routing), on Furnace under a capped scope:

| bound B | release | dev profile (as CI) | peak RSS |
|---|---|---|---|
| 300 000 000 | 37.5 s | 41.6 s | 352 MB |

| 350 000 000 | 50.8 s | 57.2 s | 353 MB |

The 350 M run had a busier box than the 300 M run, so the times do not scale exactly. That is a test cost of 40–57 s here, and 1.7–2.8 × that on CI by earlier work orders' ratios.

- **Refused routings are far from any bound.** The largest count among the suite's 26 refused routings is 145 877 (`route::tests::a_net_blocked_only_by_a_fixed_pair_names_the_pair`), under base and under the combined prototypes. No bound near 10⁸ can change a refusal a test asserts.
- **`pair_run` is left out of the unit.** Its state space is capped per call (`states ≤ 4 × MAX_NODES`, or the window is skipped). It runs at most twice per declared pair, before the rip-up loop, never inside it. It expanded 24 176 states on the only committed board with a pair.
- **Tier 2 is not in this maximum.** The `with-tools` tests were not compiled here. They emit committed fixtures (Sentinel-03, the reference, Sentinel-01, `sentinel-02-own`, d1, `four-layer`, …) and variants of them, so their routings are unmeasured.

## 3. (2) U1

The prototype (`~/.cache/aet-1323/u1.patch`): `unhop` takes `soft`, and a flattened hop's node passes when it is `open`, or, for a soft search's path, `hard_open` (open by pads, fields and the edge alone), which is how the soft search itself judged it. `BANS` stays 16. This is `1310`'s U1 (`~/.cache/aet-1310/spike-1310.patch`) without its environment switch.

`run.sh` on the U1 tree, `compare.sh` against base:

| harness | verdict |
|---|---|
| sweep (34 specifications) | **4 boards move, each `.kicad_pcb` only**: `cli/…/d1-24-i2c.json`, `cli/…/d3-24-i2c-12-spi.json`, `cli/…/sentinel-03.json`, `cli/…/solve/stress.json` (and each one's `emit` JSON, which carries the board's hash). Every exit code identical. |
| `d4diff` | identical: 277 cases, 926 files |
| `d1319` | 3 cases move, each `out.kicad_pcb` only: `d1-own-M1_OUT1-U2_VCP`, `d1-own-M1_OUT2-U2_CPL`, `d2-two` |

- **`examples/sentinel-01.json` is byte-identical. No `fab/` byte moves.**
- **U1-a, half wrong.** I predicted d3 alone among the committed specifications; d1, Sentinel-03 and `solve/stress` move too.
- **U1-b, hit.** No board changes between routed and refused.
- **U1-c, a killer exists, and U1 is a trade, not a strict gain.** The killer's own cost under U1 (the combined tree, shimmed): 137 281 875 search + 32 777 568 flood = **170 059 443**, 307 rip-ups, 16.5 s release. As a test, it would be the suite's heaviest routing (§7). `u1hunt.rs` reads each board back as `route::Board`, shifts every part of at most 8 pads by a seeded whole-pitch offset (as `escape_tier1`'s `shifted_placements…` does), and routes it with U1 and with bans alone. `route::BANS_ONLY` is a spike-only switch.

  | board | seeds | same outcome | differ |
  |---|---|---|---|
  | reference (`sentinel-01-minimal`) | 40 | 40 | 0 |
  | d1 | 40 | 40 | 0 |
  | `four-layer` | 60 | 60 | 0 |
  | Sentinel-01 | 40 | 40 | 0 |
  | d3 | 20 | 14 | 6 |

  Of d3's six:
  - **Seed 13: U1 routes, bans alone is refused** (`/SCK2: still blocked … after 500 rip-ups`). This is the mutant's killer.
  - **Seed 16: the reverse.** Bans alone routes; U1 is refused (`/SDA1 … after 500 rip-ups`).
  - Seeds 3, 10, 11 and 14: both refused, by different names. In seeds 3 and 10 the shift itself overlapped two parts' pads, which `check` refuses.

### 3.1 Tests whose literals move

`cargo test --workspace` on the `all` tree (the shim plus U1 plus §5's prune plus the early pair judge), dev profile, in a capped scope: 18 min 22 s, peak 1.97 GB.
- **2 881 passed, 1 failed: `strict_fence`,** which refuses the spike's extra module files by design.
- **No tier-1 test's literal moves** under the three router changes together.
- **The tier-2 tests are not run here** (`with-tools`, the pinned image). They judge some of U1's moved boards: `with_tools`' d3 and d1 DRC, and `board_tier2`'s Sentinel-03. A build of (2) re-runs them in the image.
- **The work maximum moves under U1.** The suite's heaviest routing becomes a 204 × 129 mm board (91 parts) that a CLI test emits: 74 082 965 search + 74 594 159 flood = **148 677 124** (it was 121.1 M on base). The `power_tree()` ladder board keeps its 142.9 M.

## 4. (3) The keepout held to its footprint

- **R3 confirmed through the real reader.** A prototype (`k3`: each keepout's rectangle inside its courtyard's box or its pad field's box, else refused) was run three ways.
  - **`emit` of all 34 specifications.** `emit` builds every part through `route::part_of`, so the check is on emit's own path: all 75 files byte-identical to base, every exit code the same.
  - **`extract` on the boards that place a keepout footprint.** Seven were read: `sentinel-02-own`, `sentinel-02-bench`, `sentinel-02-shaped`, `logger-4600`, `sentinel-03`, `hse-crystal-16` and `sentinel-01-ext-bmp581`. An eighth, `sentinel-02-power`, `extract` refuses before reading the board, in both trees.
  - **`emit` of `sentinel-02-own` with U5 turned** to each heading, Z up: `right` 0°, `forward` 90°, `left` 180°, `aft` 270°. Each is byte-identical to base, with the B.Cu keepout turned to 2.6 × 3.1 mm at 90° and 270°.

  On every board, every keepout lies inside its courtyard. Every B.Cu keepout (TDK LGA-14, Bosch LGA-10) lies **outside** the pad field, so the rule is "courtyard or field". `extract`'s stdout and exit are identical to base on all 8.
- **No §7 stop:** no vendored keepout lies outside its own courtyard and field.
- The QFN-24 (ICM-20948) is placed by no board that emits (`imu-baro-1v8` is refused before `emit`); its keepout was read from the footprint only.
- The build's test is exact against the courtyard polygon, not its box; every vendored courtyard is a rectangle.

## 5. (4) The pair's stub

- **S1, hit.** `1311` F2 reproduces on base (`stub1323.rs`): `S` at x = 7 mm, y = 3 and 8 mm gives `connectivity`'s "/N: track F (2125000, 3625000)-(2500000, 3625000) ends at (2125000, 3625000), joined to nothing".
- **S2, hit: `route_pair` lays the stub itself,** on the bare board, before any other net routes (dumped after the pair loop; nothing rips a fixed net). The coupled run's `/N` starts at (2.125, 3.625) mm. The breakout from A pad 2 cannot rise in column x = 2.125 (`/P`'s breakout passes at (1.875, 4.875), too close), so it rises at x = 2.5. That meets its own run three pitches in, and the search, whose goal is the run's first node alone, walks back along the run's own copper to reach it. The run's head between the join and its start is then a leaf. B's end has the mirror stub (11.5 → 11.875 mm).
- **S3, hit (low end).** Sweep of `S` (x 0.5–19.5 mm by 0.25, y1 ∈ {1, 2, 3, 4} mm, y2 ∈ {6.5, 7, 8, 9} mm; 1 232 placements) on base: **788 route, 152 stub, 292 other refusals.** Of the 292, 78 are the pair judge (`pairs()`, after `connectivity`) refusing the router's own pair copper ("a conductor runs … uncoupled at an end"): the same failure class as F2, pre-existing, not a stub.

Two prototypes (`s4` tree, switched by environment):

| sweep of 1 232 | route | stub | refused |
|---|---|---|---|
| base | 788 | 152 | 292 |
| **prune**: after a pair's breakouts, each conductor's copper loses every leaf that lies in none of its pads, until none is left; the maps rebuilt as `rip` does | **907** | **0** | 325 |
| prune, and the pair judge run on the pairs' own copper right after the pair loop | 907 | 0 | 325 |
| the early pair judge alone (no prune) | 788 | 0 | 444 |

- Of the 152 stubs, prune routes 119 cleanly; 33 are refused by name by the pair judge (15 "the coupled run breaks at …", 18 "… uncoupled at an end"). With prune alone those 33 are refused by `pairs()` **after** `connectivity`; with the early judge, **before** it, as §2 (4) asks ("refused by name before `connectivity`"). The early judge also moves 12 refusals that today name `/S` ("only the fixed copper of the pair") to the pair's own refusal, on boards whose pair copper the judge already refuses.
- The early judge alone already names F2 ("`/N`: its copper branches at (2500000, 3625000)"): the stub is a branch the pair judge sees. Today `connectivity` runs first and speaks first.
- **S4, hit.** The whole sweep, run with the `s4` binary under prune and the early judge, emits all 75 files byte-identical to base, with every exit code the same. That includes `pair-standin.json`, the only committed specification with a pair. **No `fab/` byte moves.**

## 6. (5) The supply gate

`supply_tier1`'s two tests run `extract` on `s024l_board()` with `drop_limits` 165 mV (pass) and 60 mV (fail), each in its own process. The inputs were kept from the suite's scratch (`~/.cache/aet-1323/t5/`: the board `c784a546…` and both specifications). The phases were timed by the shim, release, on Furnace. The shim's stdout equals base's byte for byte at 60 mV, and is identical across every shim version at 165 mV.

| phase | 165 mV | 60 mV |
|---|---|---|
| specification, solve, lines | 0.02 s | 0.02 s |
| board read and the capacitance bound | 0.02 s | 0.02 s |
| `supplies`: | 302.9 s | 277.8 s |
| · `copper()`, once per net (VDD_3V3, GND) | 31.6 s | 30.7 s |
| · network build, once per pin (27) | 10.8 s | 10.4 s |
| · solve, once per pin (27) | 260.1 s | 236.5 s |
| `extract`, whole | 302.9 s (568 MB) | 277.9 s |
| base binary, unshimmed | — | 285.2 s (568 MB) |

The solve, split on a third run at 165 mV (292 s whole):

| inside the 27 solves | s | share of `extract` |
|---|---|---|
| CG iterations (124–125 per pin, 819 603 free nodes) | 138.1 | 47 % |
| `energy_by_kind`: the interval energy of the repaired flux | 100.9 | 35 % |
| AMG setup (7 levels) | 4.2 | 1 % |
| adjacency, component, CSR | 1.7 | 1 % |
| `repair` | 1.9 | 1 % |
| `conserves` | 0.15 | 0 % |

- **P1, hit.** The solve dominates (86 %). The limit enters only the verdict: the two runs split alike. Their 25 s difference is inferred to be the shared box's load, not the work: the same 27 solves ran, and the 165 mV run's own repeats took 287 and 292 s.
- **P2, hit.** The test-side `emit` of `s024l` takes 0.98 s (release); `extract` is the whole cost. In the dev suite, `supply_tier1`'s ten tests took 377.5 s together.
- **The energy, split once more** (a fourth run, stdout identical):
  - the `Group` elements, one per via (35, each of 1 024 hole-position boxes × 336 cells × 4 corners), take **98 %** of it: 113.6 s of 114.9 s over the 27 pins;
  - the sheet cells take 1.3 s;
  - the joints and the lone `Tall` cells, under 1 ms.
- **P3, wrong in its hope.**
  - Nothing is shared between pins today. Each pin's network differs in its terminal cells, so its free set, its matrix and its AMG hierarchy differ too. No solved state can be reused across sinks without moving a float.
  - Inside a `Group`, no operand repeats. I hoped the corner products `ca·a + cb/a` recur across boxes, but every (column, width) pair is distinct: 49 152 of 49 152 in the first via. Each box's widths move with its hole position. There is no recomputation to remove.
  - **What keeps every bit is parallelism only,** each unit computing exactly what it computes today:
    - (a) the 27 per-pin solves (`copper()` once per net as now, then `bound()` per pin) on scoped threads, with the results collected in pin order; or
    - (a′) inside `energy_by_kind`, each `Group`'s 1 024 boxes evaluated on threads, and the worst box then chosen in box order, as today.

    Either lowers the wall time toward 1/k of its part on k free cores; neither lowers the CPU time.
  - Neither changes what the gate proves. **The §7 stop for (5) does not fire.** Whether a thread inside `extract` is the cut the WO means, and how many threads a CI shard should spend, is for the ruling (§7, D5).

## 7. At the stop

- **The planned §1.2 stop.** No other §7 condition fires:
  - no `fab/` byte moves under (2), (3) or (4), and (1) and (5) do not touch drawing;
  - no vendored keepout lies outside its own courtyard and field;
  - (5) can be cut without changing what the gate proves (by threads only).
- **The decisions** (the bound and its name, U1, the keepout rule, the stub fix, the supply gate's threads) are D1–D5 in `docs/work-orders/AET-WO-1323.FEEDBACK.md` §3.
- **The bound and U1's killer depend on each other.** If d3's seed 13 becomes a test (170.1 M), a 300 M bound stands only 1.76× above it, and 350 M restores a 2× margin (2.06×). Without that test, 300 M is 2.02× the heaviest routing under U1.
- **The prototypes**, as diffs against `cd79024c`, are in `patches/`:
  - `u1.patch`;
  - `k3-keepout.patch`: box containment, the build's is the exact polygon;
  - `s4-stub.patch`: prune, remap and the early judge behind environment switches, with the bisect's dumps.

  The measuring shim is `inst.py`.
