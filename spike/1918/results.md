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
  - **Side effect:** CBMC's own work falls, because each reached reach check costs a solve and a trace. Sentinel-03's whole prove phase would drop from about 25 min toward 3–4 min on Furnace [inferred from the four measured; all 20 pending].
- **(S) splits the final check across K harnesses.** Each harness has the same schedule, unwind, covers and reach checks; the union of their statements is the original's, in order.
  - **Lost:** nothing.
  - **Peak:** about half per part.
  - **Wall:** both halves take 263 s against 336 whole. The driver parses half as much each time, and that outweighs CBMC running twice.
  - **Generator change:** `board.rs` splits `check_final_<part>` when its statement count passes a bound, measured to keep each part under the ceiling.
  - `spi1` and `spi2` (11.3–11.5) would split too [pending].

## 2. The 30 s wall bound

[the survey, `~/.cache/aet-1918/wall-survey.md`; Furnace's run below; CI's CPU models pending the diagnostic runs]

## 3. The refit

[pending: after the ruling on (1), which changes every Kani-heavy test's time]

## 4. Predictions against the measurements

[pending]
