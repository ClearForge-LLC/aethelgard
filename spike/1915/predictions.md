# AET-WO-1915 predictions

Written 2026-11-23 04:32 UTC, before any measured run (WO §1.1). Committed on their own so the commit shows they came first. Nothing below was measured. Each line gives the guess, its grounds, and what would prove it wrong.

## The ground

- **The committed boards with Kani harnesses:**
  - `boards/sentinel-01`, from `examples/sentinel-01-minimal.json`, with 9 harnesses and the `SEEDED` set (`crates/aethelgard-cli/tests/common/verify.rs`);
  - `boards/sentinel-01-bench`, from `examples/sentinel-01.json`, with 11 harnesses and the five bench scenarios (`bench_board_tier1.rs`).
- **The two grounds:**
  - `3304`'s 9.1 GiB came from `sentinel-02-bench.json`, which has no `boards/` directory. It is not a committed board.
  - `4201`'s OOM at 8 GiB came from `sentinel-01.json`, the bench board.
- **Read from the code:**
  - `verify` runs its gates one after another, each as a child `aethelgard` (`verify.rs` `run_gates`, `gate`).
  - Each `prove` runs `cargo kani --harness <one>`, on its own copy of the crate with its own target directory (`main.rs` `prove`).
  - The board crate has no dependencies.
  - `campaign` starts no threads.

## Predictions

| # | Prediction | Grounds | Wrong if |
|---|---|---|---|
| P1 | The peak comes from one CBMC process, in anonymous memory, while it proves one of the `every_valid_sequence_*` harnesses (`#[kani::unwind(77)]`) | the largest unwind bound; the gates run serially, so nothing overlaps | another process or phase holds the highest sample, or the peak is page cache |
| P2 | The peak is **serial**: at the peak, one CBMC process holds ≥ 90 % of the scope's anonymous memory, and no `cargo` or `rustc` job is alive | one harness per `prove`; gates in sequence | two heavy processes are alive at the peak |
| P3 | Peak per harness: `every_valid_sequence_*` 6–10 GiB each. `*_transfer_is_bounded_and_ordered` and `spi1_transfer_refuses_too_many_frames` ≤ 2 GiB. The other four ≤ 1 GiB | the unwind bound sets the size of the unrolled formula | any short-bound harness above 2 GiB, or no harness above 6 GiB |
| P4 | The `cargo` build (kani-compiler and rustc for one harness, and `build`'s thumbv7em rustc) peaks at ≤ 1.5 GiB | a crate with no dependencies | the build phase above 1.5 GiB |
| P5 | Renode, in `simulate` and in each campaign run, peaks at 0.5–1.5 GiB per instance, with one instance at a time | Mono and .NET runtime with one H7 model | above 2 GiB, or two instances alive at once |
| P6 | ngspice peaks at ≤ 200 MiB | small transient decks | above 500 MiB |
| P7 | The bench board's peak is within 15 % of the reference's. Each harness is proved alone, so the peak is the largest harness, not a sum; the bench adds `every_valid_sequence_i2c2` | serial harnesses | the bench is more than 15 % higher |
| P8 | `prove` alone on the worst harness reproduces `verify`'s peak within 5 % | P1 and P2 | `verify`'s peak is more than 5 % above the worst `prove` |
| P9 | Two runs of each measurement agree within 5 % on the peak | CBMC and Kani are deterministic for a fixed input | a spread above 5 % |
| P10 | `memory.peak` is above the largest 1 s sample of anonymous memory by 0–1.5 GiB: page cache from target directories and scratch, and growth between samples | `memory.peak` counts file pages | `memory.peak` below the sampled anonymous peak (impossible), or more than 1.5 GiB above it |
| P11 | **The lean (a guess).** With the bound unchanged, no flag-only change brings a 6–10 GiB CBMC peak under 8 GiB with certainty. The candidates, most likely first: (a) a different SAT back end through Kani's `--solver`, which changes the solver's memory but not the formula; (b) CBMC's formula options. Kani `--jobs` cannot help: each `prove` runs one harness | the formula is set by the unwind bound, and harness and bound changes are fenced | a flag measured from a scratch copy gets the worst harness under 8 GiB with a cost below 2× its wall time |
