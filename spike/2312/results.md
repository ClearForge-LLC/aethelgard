# AET-WO-2312 §1.1 — results, measured after the predictions (`79edf4a3`)

**Before side.** Main is `7dec1d7b`.
- Its CLI was built in the pinned tools image (`e5fc7ddb…`) from a `git archive` of `7dec1d7b`
  (`~/.cache/aet-2312/main`).
- It ran under `capped-run`: memory.max 8 GiB, swap 0, pids 8192, read back (`~/.cache/aet-2312/caps.log`); nice 10;
  one container.
- The same binary replays and attributes on the host.

**Other tools.**
- The CLI-made archives are 2310's sweep, unchanged (`sh/measure.sh`).
- The probe (`probe/`, its own workspace) holds a prototype of (1) and `select`'s predicates, copied.
- **Branch side:** the build on this branch, the same archives, on the host.

Labels: **measured**, **read at source** (file:line at `7dec1d7b`), **inferred**.

## Against each prediction

| # | Held? | Measured |
|---|---|---|
| P1 | **yes** | All 5 committed archives are refused `UnknownVersion` (trace 18 and 14 against 19), exit 3, by `replay` and `attribute`, before and after (`out/committed-and-shapes-main.txt`) |
| P2 | **yes** | 64 CLI-made runs. `replay` exits 0 on all 64. `attribute` gives an `Attribution` for all 64: 62 exit 0 and 2 `I3` exit 1 (both `seeded-leave-firmware`). Rules: N1 20, E1 13, S1 11, E3 8, E2 5, I3 2, S2 2, S3 1, S4 1, S5 1 (`out/sweep-main.txt`) |
| P3 | **yes** | On all 64, every kind's verdict count is inside the prototype's bound (`outside_interval` empty on every line of `out/census-main.jsonl`). The verdicts name no failing kind outside the derivation |
| P4 | **half** | Exercised (archives / records = verdict): analog past ceiling 10 / 18; held low 5 / 5; `i2c_timeout` 19 / 24; `register_mismatch` 7 / 9; `i2c_identity_mismatch` 2 / 2; `spi_identity_mismatch` 6 / 6; `spi_data_mismatch` 2 / 2; `i2c_transaction_unwitnessed` 3 / 4; `spi_transaction_unwitnessed` 6 / 12; `pc_outside_firmware` 3 / 3; `boot_word_missing` 1 / 1; `motor_output_unwitnessed` 1 / 1; `serial_echo_missing` 9 / 10. Statuses: `ok` and `timeout_txis` only, as predicted. **Wrong:** `locked_peripheral_access` **is** named, on 2 archives (`seeded-gpiob-clock-skipped`, `seeded-i2c-clock-skipped`), each once, beside one RCC register read with an enable bit off. And **no** archive is halted: `cpu_halted` is not exercised |
| P5 | **yes** | One run is read and not booted (`seeded-no-boot-word`). Its records show `boot_word_missing` only, so gating moves nothing measured |
| P6 | **yes** | 0 of 1,890 kind 11 records differ between the name prefix `RCC.` and `svd-hal`'s RCC range (`0x58024400..0x58024530`). **No `svd-hal` in chaos** |
| P7 | **yes** | The prototype's index set equals `select`'s on all 64, for every kind (`proto_vs_select` empty on every line). After the build: **all 128 outputs (64 `replay`, 64 `attribute`) are identical**, main against branch (`out/sweep-branch.txt`) |
| P8 | **wrong on three counts** | Below. (a) `replay` is not under 0.5 s on `big_pass`: `check_run`'s unread-witness join is quadratic in `ok` reads. (b) `attribute` is slower than predicted in the debug CLI. (c) The lean changed from an index to a bound |
| P9 | **yes** | At trace 19: `reg_unnamed`, `timeout_unnamed` and `idmismatch_unnamed` replay `pass`, `matched: true`, exit 0 at main; `attribute` says `I1`. `halted_unnamed` is refused at main. Sizes 1,276–1,320 B (trace and manifest). Committed under `crates/aethelgard-chaos/tests/fixtures/round3/` with their healthy original |
| P10 | **yes, read at source** | The fold's `ONCE_KINDS` refusal (`DuplicateRecord { index, kind }`) fires before `check_run` and names the index; removing it moves every duplicate refusal's kind. **Kept.** Only its misplaced doc comment is fixed |
| P11 | **yes** | No §7 stop fires: no committed or CLI-made verdict, `matched` or `Attribution` moves (P1, P7); no format bump; no `svd-hal` (P6); records and judge agree on every committed scenario (P3) |

## (4) The cost

The debug CLI, wall time on the host at nice 10 (`out/cost-*.txt`). `big_fail` and `big_pass` are the reviewer's
shapes, K buses × D devices.

| Archive | Bytes | Records | main `replay` | main `attribute` | branch `replay` | branch `attribute` |
|---|---|---|---|---|---|---|
| K=20 D=128 `big_fail` | 499,945 | 5,244 | 0.058 s | 2.13 s | 0.056 s | refused `TooManyRecords`, 0.056 s |
| K=20 D=128 `big_pass` | 663,763 | 5,244 | 0.15 s | 0.32 s | 0.062 s | refused, 0.060 s |
| K=100 D=128 `big_fail` | 2,496,485 | 25,904 | 0.25 s | **251 s** | 0.24 s | refused, 0.24 s |
| K=100 D=128 `big_pass` | 3,315,663 | 25,904 | **2.7 s** | 6.1 s | 0.32 s | refused, 0.29 s |
| K=200 D=128 `big_fail` | 4,992,163 | 51,804 | 0.51 s | (see below) | 0.54 s | refused, 0.55 s |
| K=200 D=128 `big_pass` | 6,630,541 | 51,804 | **10.6 s** | 23.4 s | 0.62 s | refused, 0.59 s |
| K=200 D=1 `big_fail` | 73,162 | 1,004 | 0.020 s | 0.63 s | | |
| K=200 D=5 `big_fail` | 226,763 | 2,604 | 0.038 s | 4.3 s | | |
| K=200 D=10 `big_fail` | 418,763 | 4,604 | 0.061 s | 14.3 s | | |

**Why the lean changed from indexing to a bound:**
- **Under a bound on records alone, attribute stays cubic.** The worst shape is many buses with few devices. K=200 D=10 holds 4,604 records and takes 14 s against replay's 0.06 s. The per-symptom work is O(records) in many places: `decide`'s `faulted` over every bus, `software`, `healthy`, `bus_of_device`, `check_of`, and `witness_fields`'s `surplus` and `whole_log`.
- **Near-linear means indexing all of them.** That is a refactor of attribution's internals, and the 128-output identity would have to be re-proved through it.
- **The largest CLI-made run holds 113 records.** At about 1,000 records the slowest shape measured takes 0.63 s.

**So, the choice:**
- `MAX_ATTRIBUTE_RECORDS` = 1,024, refused by name: `AttributeError::TooManyRecords { records, limit }`. That is 9× the largest run.
- The 5 MB archive is refused in 0.55 s, against replay's 0.54 s.
- **Fence:** the variant needs one match arm in each of two CLI files, `cli/src/campaign.rs` and `cli/src/propose.rs`. They are outside the WO's fence, and FEEDBACK flags them.

Replay itself got faster: `shown` replaces the quadratic `unread_ok_witnesses` with one index. K=200 `big_pass`
went from 10.6 s to 0.62 s.

## What the measurement adds to the WO

- **`*_record_invalid` is reached, not equalled** (read at source):
  - an unmodelled device's read is not archived as kind 12 or 21 (`main.rs:2441`, `verdict.rs:309-318`);
  - the judge still names its invalid status `{bus}_record_invalid` (`renode.rs` `unmodelled_finding`).
- **`*_transaction_unwitnessed` stays bound from below,** by the unread `ok` witnesses, as in 2310. Two refinements, read at source:
  - a step with no witness format has an empty `expected`, and the judge names it `{bus}_witness_unavailable`, not unwitnessed. 2310's bound counted it, so a flooded log on such a step was refused falsely. The bound now excludes it;
  - archiving is lossy (`archived_lines`, `fit`), so a differing witness may hide an `[ERROR]` line past the cut, which the judge names `{bus}_step_error`. The differing set is attribute's evidence, never a count.
- **`locked_peripheral_access` is named at most once, from Renode's log** (`renode.rs` `LOCKED`). Its evidence is an `RCC.` register read with an enable bit off. That is neither necessary nor sufficient for the access, so it is bound `<= 1` only.
- **`cpu_halted` is named at any boot the judge read** (`renode.rs` `let halted = …` and its unconditional push). 2310 bound it only once booted. The derivation binds it at any boot, which is also what `select` used for attribute.
- **Mismatches and invalid records are named only once booted. Timeouts and registers at any boot** (`renode.rs:3019-3027`, `timeouts_without_the_boot_word`). `select` gated none of them. No archive tells the two apart (P5).
