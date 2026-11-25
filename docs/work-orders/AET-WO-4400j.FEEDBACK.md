# AET-WO-4400j: FEEDBACK

**The work order:** the foreign window's cost. The §1 stop was ruled 2026-11-25T18:49Z (D1–D5), and the acceptance question at 19:33Z ((a)). This is the build.
**Branch:** `wo/aet-4400j-foreign-window-cost`, PR #403.
**Record:**
- `spike/4400j/RESULTS.md`: the measurement, the decisions and the predictions graded (§1–§10), and the build's evidence (§11).
- `spike/4400j/PREDICTIONS.md`: committed first, in `e6a87129`.

## What was built, against the acceptance

| acceptance | where it holds |
|---|---|
| An input run on Sentinel-03's ELF arms inside the CLI's own bounds at the ruled window, witnessed from its heartbeat, replayed from its archive | **CLI, unlifted, 12 s:** ELF `97afa171…` (Copter-4.7.2, CI's `container/ardupilot-build.sh` in the pinned image `e1809b90…`) with `arm12.json`. Verdict pass, armed (`base_mode` 209). 352‰ of the cap; log 895,449 B (11 % of 8 MiB); 0 dropped. Archived; `replay --archive` re-judges it (RESULTS §11) |
| The log of a 12 s run keeps the first and last line of each repeating source with its count; the judge's findings unchanged against the unsummarised log, a planted firmware per source | **Real log:** against the unsummarised run of the same cell, every finding (but D4's info), every step's time, the exempt count (30,650), the serial options, the witness, the warnings and the locked accesses are identical. **CI:** the oracle runs the judge's own reading on the summarised and the whole log, for a Sentinel-03-shaped log and sixteen planted differences, one per source or rule, five of them the adversarial pass's counterexamples. Fifty cycles also run through the CLI (`boot_tier1`) |
| The retained growth fixed, or named with its dump | **Fixed (D1):** Renode's stdin is a held pipe. The dump that named it is in `spike/4400j/out/dump/` |
| The window's end gives the same verdict at the ruled length and one quantum either side | **CLI:** 11.9999 s and 12 s. **Spike:** 12.0001 s, the branch with `patches/plus-one-quantum.patch`, since the CLI refuses past its ruled ceiling (ruled (a), 19:33Z). RESULTS §11 says which run is which |
| The 14 generated boards' committed verdicts unchanged, or each change named | §Moves below |
| CI green; every new test in a shard at its measured weight | §Gates below |

## Findings

| # | finding | proposed owner |
|---|---|---|
| F1 | **`log_kept_bytes` is host-dependent:** the scratch directory's name is in Renode's echo of its own command, and Renode logs `Thread has been woken up N times` on host timing. Both are kept lines (0–303 B between runs of one cell). No rule reads either | the architect: a row |
| F2 | **Renode 1.17.0's USART drops `PCE` and `M0` on read-back** (`0x2000153D` written, `0x10D` read) | ruled: a row for Tviburi's USART guard |
| F3 | **Renode 1.17.0's `ConsoleIOSource` queues EOF without bound** when stdin is redirected and at its end | an upstream row, with the dump (`spike/4400j/out/dump/`) |
| F4 | **Likely explained, to re-measure** (ruled wording): 4400f F1 (RSS follows wall time); Tviburi `TVI-WO-1301`'s 768,856–1,049,556 KiB CI spread; and so `RUL-13`'s 1 → 1.5 GiB raise. Also likely: 4400h F1 (5 s killed at load 22–28), its core-count effect, and `architecture.md:300`'s 55 MB/s hang. Possibly 4400h F10 and F15 (the reboot's and DShot's out-of-memory) | the architect: re-measure from CI's figures after this lands |
| F5 | **Edits the grant did not name literally, each forced by a ruling:** `run.rs` (D1, granted); `main.rs` (D4's timeline tail, D3's refusal and window, D2's wiring); the CLI README's `--foreign` line (D3); `ci/foreign-judge.sh` (D1's band); `renode.rs` unit tests (D4's four reads, D3's longest `RunFor`) | none |
| F6 | **CI builds no Sentinel-03 ELF** (D5 (b), ruled a row): an `ardupilot-image` job for it needs a committed spec with `mcu.foreign_clock` derived for its HSE (`spike/4400j/py/derive.py`: USARTs at 100 MHz, not the HSI tree's 72) | a row |
| F7 | **D1's band test may not discriminate on CI.** On Furnace, held stdin grew 17–113 MB from 1 s to 5 s, and `/dev/null` 238–578. CI's unfixed growth is slower (one runner core spinning), so its `/dev/null` arm was not measured. The stdin stub tests are D1's guard | the architect: read the band's ANSWER line on CI |
| F8 | **D3, a reason to move both (not built):** with D1 and D2, a no-input 12 s run on Sentinel-03 sits at 355–362‰ and would keep about 0.2 MB of log. The no-input bound of 5 s now protects only CI's wall time and today's verdicts | the architect |
| F9 | **The residual kept log is mostly lines no rule lists:** USART1's `ReadByte` access lines (213 KB over 6 s of the armed run) and U7's reads. Not summarised here (outside D2's ruled list) | a row if the budget tightens |
| F10 | **D4 adds about 12 KB to every foreign verdict's `log_kept_bytes`** (the longer command echo and four read blocks), and `earlier`, `icsr_earlier` and `rc_search` to its report | none (named) |
| F11 | **An input with `"frames": []` is accepted** (adversarial pass): a 12 s run that sends nothing, the rest stimuli still set | the architect: a ruling, if an empty input should be refused |
| F12 | **Process:** two measurement runs were voided by my own edits to a running script. Both were rerun and are disclosed in RESULTS §10. The first dump failed because `createdump` needs the DAC beside it. `dotnet-dump`'s `gcroot` exited 139 after printing its first, whole chain | none |
| F13 | **Reduction proposals not taken:** three merges of `run.rs`'s capture plumbing (one tally helper for the two summaries, one call path into a summary, one key-cap helper), a few lines each in the capture path; and reading PC, the clock and the pulls only at the last end read, which would stop validating the earlier blocks' values. The filter's own `Unhandled write` summary and this one could become one mechanism, but that changes the verdict's shape (`warnings` and `summarised`) | the architect, at the phase gate |

## Adversarial pass (§5), full

A separate reviewer read the code with no conclusions given; I traced every finding by hand before acting on it.

- **A summary that hides a finding: four found, all fixed.**
  - (F1/F2) A held pair let other sources' lines pass ahead of it, which reordered the kept log. An error, or an idle warning, could then sit beside a different line than in the whole log.
  - (F3) A dropped line before a read block was forgotten, although the judge joins the run's log around the blocks.
  - (F4) The filter's own `Unhandled write` lines bypassed the summary.
  - **The fix:** the summary never reorders. A held pair keeps its place in an ordered queue, so the kept log is always the whole log less the lines dropped.
    - a device warning after a held error releases it;
    - markers release all;
    - block lines pass untouched;
    - decided lines pass through in their place;
    - a line kept by force is never held.
  - Each counterexample is now a planted case in the oracle.
- **A growth fix that changes behaviour: none.** Across all eight cells, `/dev/null` against held, the verdicts differ only in memory and in host-dependent log bytes (F1). The kept logs are byte-identical with those two masked. An input run archived by main and by this build has a byte-identical trace and manifest. Instructions executed are identical.
- **A window that passes by where it ends:** D4 is ruled to pass a state that holds for one of four reads.
  - Measured, the transients last at most two quanta.
  - A state held for all four fails (tested).
  - The RC exemption covers exactly four fields on exactly a declared RC port (tested on the four-port board).
  - A window under three quanta reads at 0 (adversarial pass): cosmetic, since such a run fails `foreign_not_booted`.
- **D1:** none found. The pipe's ends are close-on-exec, and the write end outlives the reaper's wait. Only `renode` and the bridge get it.
- **D3:** no path past the guard.
- **Mutants:** one per guard, each killed: D1 3/3, D4 6/6, D3 4/4. D2: 15 of 16, re-run on the final code (`spike/4400j/out/build/mutants-d2.txt`). The one survivor is equivalent: the first read's instant as `<` or `<=` in the timeline's split, where an event at that instant runs before the same block either way.

## Moves

- **Committed packages and goldens: none.** The fab packages record no memory. The 14 generated boards' committed verdicts never take the foreign filter or `--foreign`. The tier-2 tests assert only `peak > 0` and the cap's absence.
- **Every Renode verdict's `memory` drops** (D1).
- **Every foreign verdict:**
  - D4's fields and rule text;
  - `log_kept_bytes`;
  - the new `summarised` and `summary_capped`;
  - `spi_errors_exempted` unchanged in value;
  - on a declared RC port, the `rc_protocol_search` info.
- `ci/foreign-judge.sh` gains the band's two runs.

## Gates

On the final head, in the pinned tools image `1e0ab105…` under `capped-run --mem 32G`:
- `cargo fmt --all -- --check`: 0;
- `cargo clippy --workspace --all-targets -- -D warnings`: 0;
- `cargo test --workspace --no-fail-fast`: **3,206 passed, 0 failed**.

`ci/foreign-judge.sh` on the branch, with main's CI-built ELFs (`ardupilot-image` run 45663731874), exit 0. Its new band case: the peak at 1 s is 553,708 KiB, at 5 s 549,528 KiB.

- **Weights:** eight new rows in `ci/test-weights.tsv`, each test alone on Furnace times 2.8 (the AET-WO-1310 rule; `spike/4400j/out/build/weights.txt`).
- **Reduction pass (diff):** −61 net · 6 duplicates removed · 1 dead path removed · 0 protected kept · 1 finding below (F13).
