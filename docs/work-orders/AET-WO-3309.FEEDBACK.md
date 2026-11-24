# AET-WO-3309 — FEEDBACK (a witness compares what was sent, byte for byte; the BMP581 log read whole; evidence placed by what it says)

**Branch and inputs.**
- The branch is `wo/aet-3309-witness-bytes`, cut from `main` at `9c3aeee3`.
- The work order was read from `~/.cache/aet-3309-wo.md`. It was not on `main` when the branch was cut; it landed in `1cb994c1`, byte-identical, which is merged in.
- The kickoff is `~/.cache/aet-3309-from-architect.md`, and the §1.1 ruling is `~/.cache/aet-bus/3309.md` (2026-11-24T12:45Z).

Each statement is labelled:
- **measured:** run on Furnace, in the pinned tools image under `capped-run` (8G, swap 0, nice 10), or on the host under a capped user scope;
- **read at source:** given with its file;
- **inferred.**

## 0. Status

- **ext-cfg37, the acceptance case, is S1 by its wrong write** (measured). It was I4 on `main`.
  - Its configure witness holds `write 0x37 <- 0x40 0x81`, and `i2c2.u5.Write` is `0x364081` against `0x374081`.
  - The read-only warning is placed in no step and is quoted in the configure step's finding.
  - Kind 14 is unchanged: two `i2c_transaction_unwitnessed`, one `i2c_data_mismatch`.
- **Every committed attribution is unchanged** (measured, the sweep in §3, re-run at the READY head). 84 runs were made by both CLIs: the 79 of §1.1 and five planted.
  - 74 are unchanged in attribution, kind 13, kind 14 and `propose`.
  - Of the 10 that move, 8 are spike or planted cases.
  - The other two are the seeded BME280 wrong-register and wrong-nbytes cases. Their kind 13 loses a model warning. Their rule, evidence and kind 14 are unchanged, as are `cases.json`'s pins.
- **The other acceptance cases hold** (measured, planted):
  - the prepended zero byte (`write 0x00 <- 0x36 0x40 0x81`) is wrong by count, `i2c2.u5.Write.bytes` 3 against 4;
  - a BMP581 configure write sent twice is a surplus, `i2c2.u5.transactions`.
- **Decided by the ruling and accepted:** bmp-0x47 (a 3306 spike case) is I1 until a record of its own carries unplaced lines (§6, F1).
- **The §7 stop did not fire after the ruling.** No committed verdict moved.

## 1. Measurement and stop (§1.1)

`spike/3309/out/results.md` holds the measurement (`a152af69` predictions, `4bcb0992` and `216ff067` results).
- **The stop:** placing by register reads no ICP10111 line. imu-baro's passing boot would have become I4.
- **The ruling:**
  - D1 is R3.
  - D4(a) comes first.
  - D2 is carrier B, with (c) as a row.
  - D3 is -min, and `witness_findings` is in the fence.
  - D5 is a `.bytes` field.
  - D6 and D7 are as leaned.

## 2. The change

**`crates/aethelgard-evolve/src/lines.rs` (new): the readers and the placement.**
- **`form`, `transactions` and `place`.** They cover the three forms an I2C model the twin uses writes:
  - `Write`/`Read` (BME280);
  - our BMP581's one line per transaction;
  - the ICP-101xx's `[DEBUG] Received command: [..]` and `Reading data with CRC: [..], current command: …, requested: …` (D4(a)).
- **The readers.** A `Write` or command stays open across a line that is none (D1). Each `Read` after it is a read transfer of its own (`reads`, one per transfer: Renode calls `Read(count)` once per transfer, `architecture.md` §2.7).
- **`place` (R3).**
  - Each transaction goes first to the step with its key. The key is the pointer, or the ICP's two-byte command, and the direction. A repeat joins its step, which then fails.
  - Each transaction left then goes to the first step of its direction that holds none.
  - Any other is placed nowhere.

**`crates/aethelgard-evolve/src/lib.rs`: attribution.**
- **(4)** `witness_fields` reads a step's `Write` and `Read` from the first transaction of its witness, never stitched. Its `Read` is that transaction's first read transfer.
  - A transaction that read nothing read 0 bytes when the record claims `ok` and every line of the witness is a transaction. A timeout cuts a transfer short, and an unread line may be the read.
- **(1), D5:** a count unequal to the step's is a field of its own, `{device}.Write.bytes` or `.Read.bytes` (solved count, observed count), emitted only when the counts differ.
- **(2):** `transactions`, `shape`, `surplus` and `sent_elsewhere` read the device's script form, the BMP581's included, through one reader.
- **`judged_form`:** attribution judges the `Write`/`Read` and BMP581 forms. The ICP's is placed, never judged (§6, F4).

**`crates/aethelgard-cli/src/renode.rs`: `attribute` and `witness_findings`.**
- **(3):** the I2C arm places each line with `lines::place`. A line no step is goes to `Observed::unplaced`, on the device's first step, and is never archived (carrier B). A form no reader knows places no line (`INV-06`).
- **D3, -min:** `witness_findings` quotes the unplaced lines in the device's first failing finding. With none, and a record of the device's claiming `ok`, it adds one more `i2c_transaction_unwitnessed` of its own. There is no new kind, so `cases.json`'s pins stay.
- **Added by the builder, inside D3's mechanism (§6, F2):** a device whose placed transactions were logged out of script order is named the same way. The count rule failed such a log, and placement alone would pass it: a fail → pass move. No run in the sweep has one.

## 3. The sweep (§2)

- **Method:**
  - every run of the §1.1 set, plus five planted firmwares (`sh/measure-plant.sh`), made twice in the image: by `main`'s CLI and by the branch's;
  - `sh/compare.py` compares, for each run:
    - `main`'s CLI on `main`'s archive;
    - the branch's CLI on `main`'s archive, which isolates evolve's (1), (2), (4);
    - the branch's CLI on its own archive.
  - Compared: rule, evidence, symptoms, masked and unexplained; kind 14; kind 13 line by line; and `propose` for an S rule.
- **Control:** ext-cfg37 and two Sentinel-03 runs were repeated, and are identical.

**The final sweep** (measured, the branch's CLI built in the image from `7f9d71f9`, the READY code; `spike/3309/out/final/`). It lists only the runs that move; every other run is `=`.

| group | scenario | main | branch's CLI, main's archive | branch | moved |
|---|---|---|---|---|---|
| bmp-0x47 | bmp-mutant | S1 [4:7,5:7,30:13,32:13] | S1 [4:7,5:7,30:13,32:13] | I1 [] unexplained ['event 30', 'i2c_transaction_unwitnessed'] | attribution, propose (evolve), propose, kind 13 (4) |
| bmp-cfg-u2 | bmp-mutant | I4 [4:7,5:7] | I4 [4:7,5:7] | I1 [] unexplained ['event 32', 'event 33', 'i2c_transaction_unwitnessed'] | attribution, kind 13 (3) |
| ctl-ext-cfg37 | ext-mutant | I4 [6:7,7:7] | I4 [6:7,7:7] | S1 [6:7,7:7,42:13] | attribution, propose, kind 13 (2) |
| ext-append | ext-mutant | I4 [6:7,7:7] | I4 [6:7,7:7] | S1 [6:7,7:7,42:13] | attribution, propose, kind 13 (3) |
| ext-cfg37 | ext-mutant | I4 [6:7,7:7] | I4 [6:7,7:7] | S1 [6:7,7:7,42:13] | attribution, propose, kind 13 (2) |
| ext-idwide | ext-mutant | S1 [6:7,7:7,41:13] | S1 [6:7,7:7,41:13] | S1 [6:7,7:7,41:13] | propose (evolve), propose |
| ext-surplus | ext-mutant | I4 [6:7,7:7] | I4 [6:7,7:7] | S1 [6:7,7:7,42:13] | attribution, propose, kind 13 (2) |
| ext-zero | ext-mutant | I4 [6:7,7:7] | I4 [6:7,7:7] | S1 [6:7,7:7,42:13] | attribution, propose, kind 13 (2) |
| seeded-wrong-nbytes | pullup-2k2 | S1 [5:7,6:7,33:13] | S1 [5:7,6:7,33:13] | S1 [5:7,6:7,33:13] | kind 13 (1) |
| seeded-wrong-register | pullup-2k2 | S1 [5:7,6:7,33:13] | S1 [5:7,6:7,33:13] | S1 [5:7,6:7,33:13] | kind 13 (1) |

- **Why each moves:**
  - ext-cfg37 and its control: (3), the acceptance.
  - ext-zero: (1), `Write.bytes`.
  - ext-append: `Write` and `Write.bytes`.
  - ext-surplus: (2), `transactions`.
  - ext-idwide: its report gains `Read.bytes`.
  - bmp-0x47 and bmp-cfg-u2: carrier B (F1).
  - The two seeded cases: the warning leaves kind 13.
- **The branch's CLI on `main`'s archives** moves only bmp-0x47's and ext-idwide's reports: evolve's (1) alone. Every other change is placement's.
- **The preview sweep** (`0fe5df14`, before the adversarial pass) gave the same table.

## 4. Tests and mutants

**Tests:**
- evolve `tests/attribute.rs`:
  - `a_witness_compares_counts_as_well_as_values`: (1) and the acceptance;
  - `a_bmp581_log_is_read_whole_for_a_surplus_and_a_step_sent_elsewhere`: (2);
  - `a_step_s_write_and_read_are_one_transaction_never_stitched`: (4), the planted stitch;
  - `an_icp_101xx_witness_is_placed_but_never_evidence`.
- evolve `tests/lines.rs`: the ICP reader's strictness, and the whole key.
- `renode.rs`:
  - `an_i2c_line_goes_to_the_step_whose_transaction_it_is`: (3), R3, D1;
  - `what_no_step_shows_fails_the_boot_once_and_is_quoted`: D3, order, `INV-06`;
  - `every_i2c_model_s_witness_is_read_and_placed_in_its_own_step`: D4, over the parts table, BME280, ICP-101xx and BMP581.
- **The D-A1 test** (`a_device_s_error_inside_a_step_fails_that_step_by_name`) is rewritten in the BMP581's real form. Its synthetic lines were no reader's, so they are now unplaced, as D4 rules.

**Pins moved, each by a rule:**
- 3307's BMP581 tests: a 9-byte read, a 2-byte write and a pointer alone in a read step now carry their `.bytes` field.
- bmp-0x47's configure witness carries `Write.bytes` 1 against 3.
- 3306's surplus case carries `Read.bytes`.
- A 9-byte BMP581 write was "no line" (3307). It is now a transaction wrong by count.

**Mutants:** `spike/3309/sh/mut.py`, one per guard this order adds, each against the suite it lives in. 24, all killed, each by the test meant for it.
- They were run first at `0c2c5d69`, then on the touched sites after the adversarial fix (M24 added: a read's count is its first transfer's), then all 24 again after the reduction pass (`~/.cache/aet-3309/mut/run2.log`).
- One mutant (M8, a write closing on a warning) moved suites when the two I4 pins were restored. It is killed by `renode`'s warning-between-write-and-read case.

## 5. Adversarial pass, targeted

A fresh Opus subagent ran the pass at `0c2c5d69`, in its own worktree. It had the code and §5, not my conclusions, and probed with throwaway tests (reverted).

**What held (run):**
- Old count rule against the new judge, exhaustively, for:
  - the BMP581: lines up to 5 long, 9 lines per position, every OK/TXIS record mix up to 4;
  - two steps on one register;
  - the BME280;
  - a `Write`/`Read` device with three witnessed steps;
  - the ICP-101xx.

  It found **no log the new judge passes and the old rule failed.**
- An appended byte, a widened pointer and a right value at another length are each wrong in both forms.
- No panic in 100k mutated multi-line logs.
- Every model line's bytes are formatted hex, so firmware cannot forge a prefix or an arrow (read at source; the device filters `own_lines` and `witness_shaped` are unchanged).

**Its findings:**
- **Adv-1 (medium): unplaced lines are never archived.** A write sent to a device where it keys to no step is lost to evolve: the planted bmp-cfg-u2 is I1. This is carrier B as ruled. The test of the sent-elsewhere reader now says it shows the reader alone. **§6 F1.**
- **Adv-2 (medium, fixed): `Read.bytes` named a count no transfer had.**
  - `Write D0, Read 60, Read 60` was read as one 2-byte read. Renode reads once per transfer, so it is two transfers.
  - Now a step's read is its transaction's first transfer. The two pins I had moved to S1 are back to I4.
  - Mutant M24 (count every transfer) is killed.
- **Adv-3 (low): a fallback can name a line another transfer's.** `write 0x01 <- 0x50` with configure's own write missing is placed in configure, as R3 rules. The boot still fails, and attribution reads configure's write as `0x0150`: the firmware's only write at that device. **Named, not changed.**
- **Adv-4 (low): a repeat in a step whose record is not `ok` is not named.** The boot fails by `i2c_timeout` anyway. **Named.**
- **Adv-5 (info): the readers' looseness.**
  - The ICP read line's tail is checked for shape only.
  - The `Write`/`Read` reader accepts lowercase and padded bytes, as before this order.
  - The judge's exact text comparison still decides pass or fail, and firmware cannot write `[DEBUG]` or model lines. **Named.**
- **Adv-6 (info): a read with no pointer write shows no `Write.bytes`.** This is the safe direction.

## 6. Findings

- **F1 (row, as ruled): unplaced lines in a record of their own (D2(c)), after `2314`.**
  - Measured: bmp-0x47 S1 → I1, and the planted bmp-cfg-u2 (a BMP581's configure write sent to a BME280) I4 → I1.
  - In both, the receiving device logged a transaction no step of its keys to. Carrier B removes it from the trace, and the -min finding is then one no record shows (unexplained).
  - A record per device would let (2)'s surplus read it, and name the step sent elsewhere (shown at tier 1 on evolve's reader).
- **F2 (builder's addition, inside D3): steps out of script order fail the boot.** Placement alone would pass a firmware that runs a device's steps out of order, which the count rule failed. Named by the same -min finding. Proposed for ruling at review.
- **F3 (builder's addition, inside (4)): a missing read counts as 0 bytes.** This applies only beside a record that claims `ok` and a witness of whole transactions, so the stitch `Write D0, Write D1, Read 60` is wrong. A timeout and an unread line keep today's judgement.
- **F4 (row): the ICP-101xx's witness is placed, not judged.**
  - attribution's evidence still reads two forms, so imu-baro's baro-pf stays I4 (measured).
  - Judging it is its own order.
- **F5 (info): `propose` exits 3 on the seeded wrong-register and wrong-nbytes cases**, with `main`'s CLI and the branch's alike (measured). Unchanged.

## Gates

- **`cargo fmt --all -- --check`:** clean, at `af53838e`.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean, at `af53838e`. At `7f9d71f9` it failed on two test helpers (`useless_format`, `ptr_arg`), fixed in `af53838e`.
- **`cargo test --workspace`:** 3,072 passed and 0 failed, over 174 result lines, at `7f9d71f9`, on the host under a capped scope (16G, swap 0). `af53838e` changes two evolve test files only; evolve's 174 tests pass on it.
- **The bench `verify --against-fab`, in CI's layout** (`ci-img.sh`, 16G, at `af53838e`, 228 s, peak 4.8 GB):
  - no finding;
  - each campaign verdict is its scenario's expectation;
  - `against_fab` differs only in `MANIFEST.json` (`uid_boundary: false`, `user_namespace` unmet: Furnace's sandbox, as 3307 measured).

  CI's `bench-package` job decides it on the runner.
- **CI:** the PR's checks on its head decide; the run id is posted with READY.

Reduction pass (diff): −12 net · 3 duplicates removed · 0 dead paths removed · 0 protected kept · 0 findings below.
- The device's line form is decided once, in `attribute` (`Unplaced::unread`). `witness_findings` had derived it again from the checks.
- The new `renode` tests stamp log lines through one `stamp`, with `placed` and `judged` built on it.
- The new evolve tests share one `unwitnessed` verdict.
- **Proof:** both suites are green, and all 24 mutants are killed after the reduction.

## What moved

- **Attribution:** ext-cfg37 I4 → S1 (acceptance). The spike and planted cases are in §3. No committed attribution moves.
- **The boot's finding messages:** an unplaced line is quoted in its device's first failing finding. Kinds and counts are unchanged in every committed run.
- **Kind 13:** the seeded wrong-register and wrong-nbytes runs lose the BME280's `Unhandled read` warning to the finding.
- **Generator reports:** a witness wrong by count names its `.bytes` field.
- **`TIER2_MIN`:** unchanged; no tier-2 test was added.
