# AET-WO-1918 results: the measurement, before the stop

Measured 2026-11-25 on Furnace (Intel Core Ultra 7 265K, 20 cores, one thread each), base `main` `fd12b2f8`.
- **The setup:**
  - the pinned image (`container/IMAGE`) and a debug CLI built inside it;
  - a scratch tree exported from this branch (`~/.cache/aet-1918/tree`);
  - one container at a time, under `capped-run --mem 16G` with swap 0 read back.
- **Every peak is the scope's `memory.peak`, read inside the container as its last step** (`tools/ctl.sh`), as in `AET-WO-1915`.
  - The 1 s host samples (`tools/sample.py`) only attribute that peak to a process.
- GiB = 2^30 bytes. The raw runs stay in `~/.cache/aet-1918/runs/`.
- Predictions: `predictions.md` (committed first, `0ebdc43f`), checked in §4.

## 1. The prove peak

### 1.1 Every harness, as `prove` runs it today (one container per harness)

**Sentinel-03** (`examples/sentinel-03.json`, generated; 20 harnesses; schedule parts unwind 90):

| Harness | `memory.peak` GiB | Wall s |
|---|---|---|
| `every_valid_sequence_tim1` | **15.094** | 336 |
| `every_valid_sequence_spi2` | 11.519 | 180 |
| `every_valid_sequence_spi1` | 11.295 | 202 |
| `every_valid_sequence_i2c1` | 8.376 | 146 |
| `every_valid_sequence_i2c2` | 8.359 | 134 |
| `every_valid_sequence_gpio` | 2.632 | 44 |
| the 14 others | 0.63–1.87 | 16–53 |

- **One harness, `every_valid_sequence_tim1`, is over 12 GiB.** It is 0.9 GiB under the 16 GiB cap, and its `oom_kill` count is 0. So this is a measurement, not a cap hit.
- `spi1` and `spi2` are under 12 GiB by 0.5–0.7 GiB.
- The prove phase takes 1,475 s on Furnace in all, which is 25 min of `bench-package-s03`'s 54.
- **The shape** (1 s samples):
  - CBMC grows to 2.5 GiB over 280 s.
  - Then kani-driver reaches 14.95 GiB alone, in the last 50 s, after CBMC has exited.
  - This is `1915`'s `hse8` shape (D4).
- **CI agrees:** run 45501452199's 2 s sampler read 14.73 GiB (kani-driver), a lower bound on the same harness.

**Sentinel-01**, both committed boards, generated from `examples/sentinel-01.json` (bench) and `sentinel-01-minimal.json`. Each is byte-identical to `boards/`.

| Board | Heaviest | `memory.peak` GiB | Wall s |
|---|---|---|---|
| `sentinel-01-bench` (12 harnesses) | `every_valid_sequence_spi1` | 3.531 | 63 |
| `sentinel-01` (10 harnesses) | `every_valid_sequence_spi1` | 3.523 (and 3.523 again) | 63 |

Both are unchanged since `1915` (3.48–3.49). Nothing on Sentinel-01 is near 12 GiB.

**`4411`'s module:** not measured. Its pushed branch (`de132429`) is the spike record: it proposes a `sentinel-03-wifi` specification but holds none.

### 1.2 What CBMC hands kani-driver (`every_valid_sequence_tim1`; `tools/cbmc-out.sh`, `tools/cbmc_json.py`)

**The setup:** Kani's own CBMC command, taken from `--keep-temps --verbose`, was run again with its output to a file. It took 220 s, and CBMC's own scope peaked at 6.75 GiB, page cache included.

**What it wrote:**
- 4.117 GiB of JSON;
- 3,850 properties;
- 527 of them FAILURE, which for a reach check or a cover means "reached", each with a full trace;
- 5.82 million trace steps.

**Who owns the stream:**

| Properties with a trace | Count | Bytes |
|---|---|---|
| `check_final_tim1`'s reach checks | 449 | 3,993 MiB (97 %) |
| `write`'s, `rely`'s, `clocked`'s, `read`'s and `slot`'s reach checks | 70 | 164 MiB |
| the harness's own reach checks and 3 covers | 8 | 55 MiB |

- **So the peak is Kani's assertion reach checks on the final check.**
  - Kani turns them on by default: `--assertion-reach-checks` is passed to the compiler.
  - Every reachable assertion yields one trace of the whole 90-step schedule.
  - kani-driver holds about 3.6× the stream when it parses it.
- **The 14 reach checks that came back SUCCESS (unreachable)** are all `kani::assert(false, …)` guards, such as "a block outside the tim1 part was accessed" and "GpioA BSRR write touches a pin no solved output of the tim1 part owns". Their unreachability is the property.
- **The judge reads no reach check** (`aethelgard_cli::kani::judge`). It reads:
  - the exit code;
  - `VERIFICATION:-`;
  - `Complete - `;
  - the harness checked;
  - the one cover summary.
- An UNREACHABLE assertion passes today, unjudged. The covers are the non-vacuity witnesses (`AET-WO-1400` E1).

### 1.3 Two shrinks, measured on scratch copies (`tools/cand.sh`; no `--spec`)

| Candidate | Harness | `memory.peak` GiB | Wall s | Kani's result |
|---|---|---|---|---|
| today | `every_valid_sequence_tim1` | 15.094 | 336 | SUCCESSFUL, 3 of 3 covers |
| **(R)** `--no-assertion-reach-checks` | `tim1` | **1.813** | **21** | SUCCESSFUL, 0 of 3,284 failed, 3 of 3 covers |
| (R) | `spi2` | 1.030 | 15 | SUCCESSFUL |
| (R) | `spi1` | 1.067 | 15 | SUCCESSFUL |
| (R) | `i2c1` | 0.787 | 8 | SUCCESSFUL |
| **(S)** the final check split in two (`tools/split_final.py`) | `tim1_0` (80 of 160 statements) | **7.156** | 128 | SUCCESSFUL, 14 UNREACHABLE (the guards) |
| (S) | `tim1_1` (the other 80) | **7.676** | 135 | SUCCESSFUL, 14 UNREACHABLE |

- **(R) removes the reach checks.** Every assertion is still proved on every path, and every cover is still judged. The checked properties fall from 3,850 to 3,287: the 563 reach checks go.
  - **Lost:** Kani's informational UNREACHABLE status, which no gate reads.
  - **Proposed beside it, so the final check's reachability is judged rather than only reported:** one `kani::cover!(true, "the <part> part's final check is reached")` at the top of each `check_final_*`. The final check is straight-line, so it is reached exactly when that cover is satisfied. A cover is judged (`cover_unsatisfied`).
  - **Side effect:** CBMC's own work falls, because each reached reach check costs a solve and a trace.
  - **Measured on all 20 Sentinel-03 harnesses:**
    - every one is SUCCESSFUL, with every cover satisfied;
    - the largest peak is 1.814 GiB (`tim1`), then 1.07 (`spi1`) and 1.03 (`spi2`);
    - the 20 sum to about 130 s on Furnace, against 1,475 s today.
- **(S) splits the final check across K harnesses.** Each harness has the same schedule, unwind, covers and reach checks; the union of their statements is the original's, in order.
  - **Lost:** nothing.
  - **Peak:** about half per part.
  - **Wall, back to back in one window:**
    - whole `tim1` took 395 s (15.02 GiB);
    - its halves took 229 + 196 = 425 s, that is **+8 %**.
    - The 128 and 135 s in the table above ran in a quieter window than the 336 s whole run, so they are not comparable with it.
    - (S)'s effect on `bench-package-s03`'s minutes is therefore near zero to slightly worse [inferred from one harness]. (R) removes about 22 of its 25 prove minutes on Furnace.
  - **Generator change:** `board.rs` splits `check_final_<part>` when its statement count passes a bound, measured to keep each part under the ceiling.
  - **The bound is a proxy.** The peak per final-check statement runs from about 0.02 GiB (`gpio`, 2.56 GiB / 122) to 0.094 (`tim1`, 15.09 / 160), because a trace's size depends on the part's image and schedule. So a statement bound does not guarantee a peak. **The CI ceiling read from the probe (§1.6) is the control;** the bound only makes it pass.
  - **`spi1` and `spi2`, measured split in two** (71 + 71 of 142 statements):
    - `spi1`: 5.401 and 5.898 GiB, in 86 and 87 s (whole: 11.295 GiB, 202 s);
    - `spi2`: 5.575 and 6.016 GiB, in 88 and 89 s (whole: 11.519 GiB, 180 s).
    - Each half is SUCCESSFUL with 5 of 5 covers and 5 UNREACHABLE (the guards).
  - **With all three split,** Sentinel-03's largest part is 7.676 GiB. Its prove phase is about 1,370 s against 1,475 [the sum of the measured parts and the unchanged rest].

### 1.4 (S) over Sentinel-03's whole crate: every final check over 80 statements split (`s03-s80`)

**The split:**
- `i2c1` and `i2c2`: 130 statements each, into 65 + 65;
- `spi1` and `spi2`: 142 each, into 71 + 71;
- `tim1`: 160, into 80 + 80;
- `gpio` (121) is not split: its final check ends in a `for` block, which `split_final.py` refuses to cut, and it peaks at 2.56 GiB anyway.

The 25 harnesses were proved one container each, on Kani's defaults with reach checks on:

| Harness | `memory.peak` GiB | Wall s |
|---|---|---|
| `every_valid_sequence_tim1_1` / `_0` | **7.984** / 7.504 | 229 / 236 |
| `every_valid_sequence_spi2_1` / `_0` | 6.016 / 5.576 | 166 / 129 |
| `every_valid_sequence_spi1_1` / `_0` | 5.899 / 5.391 | 125 / 122 |
| `every_valid_sequence_i2c1_1` / `_0`, `i2c2_1` / `_0` | 4.31 / 4.07, 4.32 / 3.99 | 69–80 |
| `every_valid_sequence_gpio` | 2.562 | 32 |
| the 14 others | 0.09–1.86 | 1–43 |

- **All 25 are SUCCESSFUL, with every cover satisfied.** The largest part is **7.98 GiB**, which leaves 4 GiB under the 12 GiB target.
- **Wall time:** the 25 sum to 1,598 s against today's 1,475 s.
- **But this pass ran while Furnace was busier.** The same `tim1` halves took 128 and 135 s in §1.3 and 229 and 236 s here. So compare wall times only within one pass. CI's own runs decide (3).
- **Sentinel-01 would not move at 80.** Its final checks hold 41–52 statements, so `boards/sentinel-01*` stay byte-identical.
- **Sentinel-03's package does move, in one binding line.** `verdict_sha256` hashes the verify verdict line, and each prove gate in it names its harness and the crate's `input_sha256`.
  - The ELF does not move: `mod proofs` is `#[cfg(kani)]`.
  - **This is a §7 stop condition: "a harness's peak moves a committed verdict or package".** (S) moves `fab/sentinel-03/package/MANIFEST.json`'s `verdict_sha256`.
  - **The path back is a `write` dispatch, not `--rebind`.** Sentinel-03's package is written by `bench-package-s03` with plain `--fab-out`, readiness judged. `--rebind` is pinned by a tier-1 test to Sentinel-01's path only (`fab_tier1::ci_rebinds_only_sentinel_01_s_committed_package`). So it would be:
    - the architect's `write` dispatch on the built head;
    - the package committed byte for byte, as at `4300k`;
    - expected to differ only in `verdict_sha256` [inferred: the ELF and the board's files do not move].
  - (R) passes a flag and leaves the crate alone, so its prove gates' `input_sha256` would not move. Whether the verdict line moves under (R) is not measured.
- **What else (S) renames, at a bound of 80** (`tools/count-finals.sh`: every JSON under `examples/`, `templates/` and the test fixtures, generated; 31 generate a board crate):
  - Only four crates have a final check over 80:
    - `examples/sentinel-03.json` (122–161);
    - `examples/sentinel-02-bench-hse8.json` (`spi1` 166, `i2c1` 131), `1915`'s board that is OOM-killed at 16 GiB, which (S) may also fix [not measured];
    - the fixtures `hse-crystal-16.json` and `hse-external-8.json` (132–155).
  - The next largest is `sentinel-02-bench`'s `spi1`, at 77.
  - **No test names a harness on those four boards:**
    - every test that names a schedule harness, or counts the parts, builds a board under 80: Sentinel-01, the motor fixtures, `d1-drv8876`, or a synthetic topology in `board.rs`'s own tests;
    - the only committed `proofs.rs` files are `boards/sentinel-01*`.
  - So no test's assertion changes at 80 [grep over `crates/*/tests` and `board.rs`'s tests].

### 1.5 Should the judge read the reach checks? (asked at 02:15Z)

**The data:** every UNREACHABLE status in the `s03-s80` pass, our own assertions only. Kani's automatic checks (bounds, shift, dereference) are set aside.
- **Per harness, UNREACHABLE is normal and by design.**
  - The transfer and start harnesses carry 176–205 unreachable checks each; the schedule parts carry 2–14.
  - `Image::write` holds every block's checks, and a harness that drives one peripheral never reaches another's.
  - **A judge that refused UNREACHABLE per harness would fail 21 of the 25.** That counts our own non-guard assertions only: 1–2 per schedule part, 64–84 per transfer or start harness.
- **Across the board's harness set, 1,492 of our own assertion sites:**
  - 1,439 are reached by at least one harness;
  - 37 are `kani::assert(false, …)` guards, whose unreachability is the property;
  - **16 are reached by no harness. All 16 were traced, and each property is proven at another site, or the site is dead by design:**
    - **8 SPE ordering checks** (`… SPE set before CFG2 holds MASTER and SSM`, `… SPE set without SSI`):
      - the 4 in `Image::write` are dead because every transfer writes SPE through its own spy;
      - `Spi1Spy`'s and `Spi2Spy`'s copies of both checks are reached and SUCCESS in the two transfer harnesses;
      - the 4 in the schedule-part images (`proofs.rs:2036–2040`, `2756–2760`) are dead because `init` leaves SPE clear.
    - **5 ODR checks** sit after a BSRR guard in the same branch, so they are dead by that guard.
    - **`Tim1 accessed before RCC_APB2ENR bit 0 is set`** in `Image::clocked` (`:109`) is proven in `ImageTim1::clocked` (`:3351`, SUCCESS in both `tim1` parts).
    - **`a PWR register other than CR3 and D3CR was written`** in `Image::write` (`:190`) is proven in every part image's `write` (SUCCESS in `gpio`, `i2c1`, `i2c2`, `spi1`).
    - **`a GpioC MODER write moved a pin the topology does not use`** in `ImageGpio::write` (`:4208`) is proven at `Image::write:516` (SUCCESS in `usart3_start_writes_the_solved_values`).
  - **No assertion of ours was found vacuous on Sentinel-03.**
- **Lean: a row, not this work order.**
  - The judgement that means something is board-wide: every generated assertion is reached by some harness, guards excepted.
  - That needs the generator to name its guards, and `verify` to read every prove gate's statuses together. It is a new judge, not CI headroom.
  - Under (S) the reach checks stay on, so the row can be built on them later. Under (R) there would be nothing left to read.

### 1.6 CI's own peaks (diagnostic run 45528571905, head `9c06e112`; `ci/probe.py` (measured as `tools/ci_probe.py`, moved at the build), `tools/ci_diag.py`)

The host probe reads each kani-driver's VmHWM every 0.2 s. VmHWM is a high-water mark, so a missed poll loses only the growth after the last read.

| Job (CPU) | Harness | VmHWM GiB | s |
|---|---|---|---|
| `bench-package-s03` (EPYC 7763) | `every_valid_sequence_tim1` | **14.780** | 399 |
| | `spi2` / `spi1` | 11.407 / 11.186 | 279 / 278 |
| | `i2c2` / `i2c1` | 8.254 / 8.076 | 161 / 159 |
| `bench-package` (EPYC 7763) | Sentinel-01's `every_valid_sequence_spi1` | 3.479 | 106 |
| every `tools-shard` | the largest any test proves | 3.93 (`tim1` of a test board, Xeon 8573C) | 96 |

- **CI's peaks equal Furnace's.** Memory is deterministic here: `tim1` is 14.78 GiB of kani-driver VmHWM in CI, against Furnace's `memory.peak` of 15.09, which includes everything else in the scope.
- So a ceiling read from CI's probe judges the same quantity Furnace measured, minus the small processes beside kani-driver (0.1–0.3 GiB, §1.1).
- **Time:** CI's EPYC 7763 proves `tim1` in 399 s, against 336 on Furnace.

## 2. The 30 s wall bound

### 2.1 The code (from the survey, `~/.cache/aet-1918/wall-survey.md`, spot-checked)
- **Where it is set and judged:**
  - `renode.rs:226` `BOOT_BOUNDS` (30 s, 1.5 GiB) bounds every `simulate`, every campaign boot and a droop's boot.
  - `analog.rs:250` `BRIDGE_BOUNDS` (30 s) bounds the ngspice bridge alone.
  - The reaper's clock runs from the spawn (`run.rs:1092`), so it includes Renode's start-up.
- **What a kill becomes:**
  - A killed boot becomes the error finding `wall_bound_exceeded` and a `fail` verdict.
  - A droop scenario whose undisturbed boot is killed is refused `PlantedEventUnplaced`.
  - A killed bridge run is a `ToolError`.
- **The determinism hole (`INV-04`):** `WALL_BOUND_FINDING` is one of `UNREAD_FINDINGS`, which replay accepts (`chaos/src/campaign.rs:114-125`). So a run killed for host load can be archived, and the same scenario can get a different verdict on a slower host.
  - The wall time itself is in no verdict or hash.
  - No committed scenario or package names the finding.
- **A virtual-time bound already exists, and it is already the judged one:**
  - `aethelgard_twin::boot::MAX_VIRTUAL_NS` = 100 ms is every boot's `RunFor`.
  - The firmware's own timing is judged in virtual time (`boot_word_ns`, `boot_word_missing`).
  - The wall bound only answers whether Renode returned at all. It must stay as a hang guard: after "This machine has aborted", virtual time stops, and `RunFor` never returns (`renode.rs:233`, A6).

### 2.2 Each boot's wall time, measured

Renode's process lifetime, spawn to exit, which is a little less than the reaper's clock:

| Where | CPU | Boots | Median s | p95 s | Max s (A6 aside) |
|---|---|---|---|---|---|
| CI run 45528571905, 12 shards plus 2 bench jobs | EPYC 7763 (7 jobs) | 268 | 4.5–11.0 | 4.9–15.3 | **20.8** (shard 10, median 10.4) |
| | EPYC 9V45 (4) | 149 | 2.6–2.8 | 3.6–5.9 | 5.9 |
| | EPYC 9V74 (1) | 86 | 3.6 | 9.4 | 16.2 |
| | Xeon 8370C (1) | 79 | 4.5 | 5.1 | 6.3 |
| | Xeon 8573C (1) | 16 | 0.2 | 9.7 | 9.7 |
| Furnace, `twin_tier2` + `campaign_tier2` pinned to 4 cores, 4 test threads (1 s samples) | Core Ultra 7 265K | 195 | 3 | 9 | 12 (one at 24) |
| Furnace, unpinned, 4 test threads | Core Ultra 7 265K | 195 | 3 | 7 | 7 (one at 25) |

- **The boots at the bound, by their argv:**
  - **CI shard 9, 30.0 s:** a tier-1 test's stand-in, `/tmp/aet-2100-t1-…/renode`, killed at the bound on purpose (`boot_tier1`'s `a_boot_past_the_wall_bound_fails_and_names_it`).
  - **CI shard 0, 30.1 s:** a real Renode boot run by the CLI (`--config /var/lib/aethelgard/…`, seed 2100). It fits the A6 test, which runs an aborted machine to the bound and is the only CLI boot that does. The log does not name the test, so this is not confirmed.
  - **Furnace, 24–25 s, once in each run:** also a CLI boot. It is likewise unidentified, and A6 is the likely one.
  - These are set aside in the maxima below. `twin_tier2`'s direct `renode` helpers (`record_on`, `writes_at`) run with no `--config` and under no bound, so none of these boots is theirs.
- **By board, on the same CPU and with no shard load (EPYC 7763, the bench jobs):**
  - Sentinel-03's boots: median **11.0 s**, max 11.8, over 20 boots;
  - Sentinel-01's: median 4.5 s, max 4.9, over 10.
  - **Sentinel-03's boot is the one that needs the margin.** It is 2.4× Sentinel-01's, and under a shard's load it stacks on that runner's own factor. Shard 10's median of 10.4 s and max of 20.8 s are consistent with Sentinel-03 tests [inferred: the probe names no test].
- **The bridge's 30 s (`BRIDGE_BOUNDS`):** the CI probe did not watch it, so CI is unmeasured. On Furnace (1 s samples), 32 and 30 bridge runs took median 3–4 s and max 10 s.
- **Earlier CI runs, from the survey:** across 7 runs, each whole `simulate` call's wall time (`ANSWER 2100`) had median 5.5 s, p95 12.0 s and max 23.6 s.
- **The one kill in those runs** was `bor_recovery::the_bmp581_extension_recovers_after_its_settle` (run 45494968504, shard 11). Its boot ran at least 30 s; its CPU is not logged.
- **The slowest runner measured is the EPYC 7763.** Its worst shard (shard 10) runs boots at a median of 10.4 s and a maximum of 20.8.
- **So 30 s holds a 1.4× margin over the slowest boot seen** (20.8 s on this run; 23.6 s in the survey), and the margin is gone at least once in eight runs.
- **What a boot would need to pass with a 2× margin on that runner:** about 50 s on the evidence so far, more if the bmp581 boot's true time is above 30.
  - The four further diagnostic runs extend this table (§5).

### 2.3 The options (stop: a product bound, ruled first)
- **(a) Raise the judged wall bound**, for example to 60–120 s.
  - Only a run between 30 s and the new bound changes, from `fail` to `pass`.
  - `boot_tier1`'s `a_boot_past_the_wall_bound_fails_and_names_it` and `twin_tier2`'s A6 test wait out the bound, so their timing assertions need rescaling.
  - The `INV-04` hole stays, at a higher threshold.
- **(b) Virtual time is the only judge, and the wall bound becomes a hang guard.**
  - Set it well above any boot seen, say 120 s.
  - A guard kill is a `ToolError` refusal, never a finding, so host load can never reach an archived verdict. That closes the `INV-04` hole.
  - The two tests above keep a short bound through the existing internal `--wall-ms`.
- **(c) Keep 30 s and cut the load:** serialise `campaign_tier2`, put cargo builds under `RENODE_TURN`, or add shards. This treats the symptom, not the bound.
- **Lean: (b).** The bound that judges the firmware is already virtual. A verdict that a host's load can change is the defect.

## 3. The refit

**Pending the ruling on (1).** The shrink changes every Kani-heavy test's time, so the refit is measured from CI runs of the built head.

**What the first diagnostic run gives now:**
- Each shard's test step ran 14–35 min. The worst were shard 3 (34.5 min, Xeon 8370C), shard 4 (25.1) and shard 9 (22.8).
- `bench-package-s03` took 58 min of its 90, with the probe running.
- `verify_2k2_tier2`'s block took 1,303 and 1,546 s on the two runs posted at 23:26Z, against its row of 624.

## 4. Predictions against the measurements

| # | Verdict | Measured |
|---|---|---|
| P1 | **Wrong on the part, right on the shape.** The heaviest is `every_valid_sequence_tim1`, not an SPI part | 15.09 GiB, in the range. kani-driver holds 14.95 of 15.08 GiB in the cgroup, after CBMC exits. Both earlier samples were under the true peak |
| P2 | Holds | non-schedule harnesses ≤ 1.87 GiB; one harness over 12 |
| P3 | Holds | 3.52–3.53 GiB, `every_valid_sequence_spi1` |
| P4 | Holds | 4.12 GiB of CBMC JSON; 97 % of it the final check's 449 reach-check traces |
| P5 | Holds | split at 80 statements, the largest part is 7.98 GiB, every property and cover kept; wall +8 % back to back on `tim1` |
| P6 | **Not measured as stated** | the killed boot (bmp581) was not isolated. Boot medians run 1.5–3.5× Furnace's, by runner |
| P7 | Holds on the bound, **wrong on "only a guard"** | `MAX_VIRTUAL_NS` is the judged window, but a wall kill does become a finding (§2.1) |
| P8 | Pending (3) | the block is 1,303–1,546 s on the slowest runners, before (1) |
| P9 | Pending (3) | |

## 5. Still running at the stop
- **The architect's four re-runs of `9c06e112`:** the further rows of §1.6 and §2.2.
- **The refit:** from the built head's runs.

## 6. The built head (after the rulings: (1) (S), (2) (b))

### 6.1 (S) as built: `board.rs` `split_final`, 80 statements
- **Sentinel-03:** 26 harnesses.
  - Every schedule part is split in two, `gpio` included.
  - The built split counts a statement as its check plus the bindings before it, so `gpio`'s 122 lines are 110 statements, and its `for` block is one of them; `split_final.py` had refused to cut it.
- **Each split is exact** (`spike/1918/out/`, checked on the generated crate):
  - the parts' bodies, concatenated, are the original check;
  - each harness is the original, but for its one call;
  - everything else in `proofs.rs` is byte for byte the old.
- **Sentinel-01's two crates** regenerate byte-identical to `boards/`.
- **Sentinel-03's whole `verify --fab-out` in the image** (`tools/built.sh s03`, one container, 16G):
  - all 26 prove gates pass, and every gate passes;
  - the ELF is `6e4a81f2…`, unchanged;
  - every package file is byte-identical but `MANIFEST.json`, whose only difference is `verdict_sha256`;
  - the scope's `memory.peak` is 9.24 GiB for the whole verify, every process included;
  - 3,090 s, on a Furnace loaded by the gates beside it.
- **`sentinel-02-bench-hse8`, recorded as ruled:**
  - its `spi1` parts prove at 7.53 and 7.26 GiB (284 and 254 s), and its `i2c1` parts at 3.86 and 4.42 GiB, each at 16G;
  - unsplit, 1915 measured the `spi1` part OOM-killed at 16G.

### 6.2 (b) as built
- **`BOOT_BOUNDS.wall` is 120 s.**
- **A plain wall kill is `renode::hung`, a `ToolError`.** That holds for the boot, the droop's undisturbed boot (`undisturbed_boot_word_ns`) and a foreign run whose hook caught a fault.
- **`cpu_aborted` and the memory cap stay findings.**
- **Replay refuses `wall_bound_exceeded`** on a read or an unread boot (`HANG_GUARD_FINDING`).
- **`simulate --wall-bound-ms`** is hidden, 1 to 120,000 ms, and absent from the MCP tools. It is recorded as `wall_bound_ms`.
- **`twin_tier2`'s A6 and `m2` pass in the image.** A6 returned in 25.0 s under the test's 30 s guard.
- **The planted hang:** `campaign_tier1::a_renode_that_never_returns_is_refused_and_archives_nothing` (122 s on Furnace).
