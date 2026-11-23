# AET-WO-3307 — FEEDBACK (a BMP581's witness counts as evidence in attribution)

**Branch and inputs.**
- The branch is `wo/aet-3307-bmp581-witness`, cut from `main` at `28fba951`.
- The work order was read from `~/.cache/aet-3307-wo.md`, and the kickoff from `~/.cache/aet-3307-from-architect.md`.

Each statement is labelled:
- **measured:** run on Furnace, in the pinned tools image under `capped-run` at nice 10, or on the host under a capped user scope;
- **read at source:** given with its file;
- **inferred.**

## 0. Status

- **The BMP581 collide case is the software's, by its witness** (measured).
  - This is bme-bmp with U2's identity read sent to 0x47, where U5, a BMP581, answers.
  - It was **I4** on `main` and is **S1** on the branch. Its evidence is the two rises, U2's silent identity witness and U5's configure witness: `[4:7,5:7,30:13,32:13]`. Nothing is masked.
  - `propose` exits 6 with a generator report. On `main` it exited 1 with `no_proposal`.
- **No other attribution moves** (measured).
  - Of 68 runs, 67 give byte-identical `attribute` output from `main`'s CLI and the branch's, on the same archives. Every software run gives the same `propose` files and exit code.
  - The bench's `verify --against-fab` JSON is byte-identical from `main` and the branch. No `fab/` byte moves.
- **The §7 stop did not fire.**
- **No C# model change, and no specification or trace change.** The diff is `crates/aethelgard-evolve` (`lib.rs`, `tests/attribute.rs`), `spike/3307/` and this file.

## 1. Read first (read at source, `28fba951`)

- **Why it was I4.**
  - `witness_fields` read a witness's register written and value read with `witness_bytes`. That reads only the BME280's lines, `[NOISY] {device}: Write XX …` and `Read XX …`.
  - Our BMP581 model logs one line per transaction (`models/renode/BMP581.cs` `FinishTransmission`): `write 0xRR <- 0xBB …`, `write 0xRR <- (none)` for a pointer alone, and `read 0xRR -> 0xVV …`. That last form is the pointer write and the read together.
  - So every BMP581 witness returned `None`, which is no evidence either way.
- **The twin writes the same form for the script** (`crates/aethelgard-twin/src/model.rs`, `Model::Bmp581`), with `listed` giving `(none)` for no bytes.
- **The ICM42688P model logs the same words behind a `bank N ` prefix** (`ICM42688P.cs` L137, L148). That is the §5 case of "a line another part's model also emits".
- **`witness_fields` refuses a read wider than 4 bytes.** So the BMP581's 6-byte data step is no evidence, before this order and after it.

## 2. Measurement (§1)

- **Predictions first:** `spike/3307/predictions.md`, committed at `0057d2ce` before any run.
- **Method** (3306's, reused):
  - `spike/3307/sh/measure.sh` makes every run **once**, in the image, by `main`'s CLI. The CLI was built from a `git archive` of `28fba951`.
  - `spike/3307/sh/rejudge.py` then judges each archive on the host with `main`'s CLI and with the branch's. The branch's CLI was built in the image from a `git archive` of `936b9fa3`, and again from the final `7d198c17` after the adversarial fix (§5). The two judgements are byte-identical.
  - For each run it records `attribute`'s bytes; for a software rule, `propose`'s files and exit code; and each device's whole-sequence class (`spike/3306/dump`).
  - This order changes no twin, trace or firmware, so one archive per run is the whole comparison.
- **What ran (68 runs):**
  - every committed scenario group: sentinel-01-minimal with the seeded set's unmutated scenarios, sentinel-01-bench, motors, serial, Sentinel-03 (U7 a BMP581, including held-low SDA on its bus, droop and BOR), and sentinel-02-power;
  - the 13 seeded mutants over their own cases (`scenarios/sentinel-01/seeded/cases.json`);
  - ext-bmp581 and two-bme, each baseline and held-low SDA;
  - two-bme with U2's step sent to 0x75 and to 0x77;
  - bme-bmp: baseline, held-low SDA, and U2's step sent to 0x47;
  - ext-cfg37: ext-bmp581 with U5's configure write sent to register 0x37 (not committed).
- **Records:**
  - `spike/3307/out/table.md`: every run, with `main`'s and the branch's rule, evidence and masked;
  - `spike/3307/out/rejudge.jsonl`: the same, with the `propose` files;
  - `spike/3307/out/bmp581-records.txt`: the kind 12 and 13 records of bmp-0x47 and ext-cfg37.

| Run | Prediction | `main` (measured) | Branch (measured) |
|---|---|---|---|
| **bmp-0x47** | I4 → S1; U2's and U5's configure witness | **I4** `[4:7,5:7]` | **S1** `[4:7,5:7,30:13,32:13]`, masked `[]` |
| ext-cfg37 | I4 → S1 | I4 `[6:7,7:7]` | **I4, identical** (a miss, below) |
| the seeded set (unmutated cases and 13 mutants) | as `cases.json` pins | as pinned | identical |
| every committed scenario group | unchanged | as their expectations | identical |
| ext, two, bmp: baseline and held-low; two-0x75, two-0x77 | unchanged | N1, E1, S1, S1 | identical |

**bmp-0x47's generator report** (measured; this is the prediction):
- `i2c1.u2.Write` (solved `0xd0`) and `i2c1.u2.Read` (solved `0x60`), each observed `null`: U2's witness is silent, as before.
- `i2c1.u5.Write`, mask `0xffffff`, solved `0x364081`, observed `0x50`: U5's configure witness holds `read 0x50 -> 0x00`. The model masks U2's pointer `0xD0` to 7 bits.
- The records are the ones 3306 measured on its bme-bmp, at the same indices (`spike/3306/out/two-device-records.txt`). The tier-1 test is built from them.

**Where the predictions missed (measured):**
- **ext-cfg37: predicted S1, measured I4 on both.**
  - Writing `0x40 0x81` from 0x37 reaches 0x38, which is read-only. The model logs `[WARNING] … write to read-only register 0x38 ignored` before the transaction's line.
  - The line distribution (`renode.rs` `attribute`: each step takes as many lines as it expects, the last takes the rest) gives that warning to the configure witness. That witness holds no line in its form, so it is sound and nothing in it is wrong.
  - The wrong write, `write 0x37 <- 0x40 0x81`, lands in the data witness. Its 6-byte read is refused, so it is no evidence.
  - This is F2 (§7). It is not a stop, because the run is not committed and does not move.
- **sentinel-02-own was predicted but not run.** It declares `mcu.foreign_clock`, so `campaign` refuses generated firmware on it (`ForeignClockOnGeneratedRun`, measured). Sentinel-03's U7 is the committed BMP581 under a fault, and it ran.
- **The bench sheets pin no attribution** (read at source): `fab/sentinel-01-bench/BENCH-EXT-BMP581.md`'s `S1`/`S2` are source-run names, and `BENCH.md` names none. The bench board's own verdicts are judged by the bench `--against-fab` run (Gates).
- **No tier-2 test asserts an attribution on a BMP581 fixture** (read at source). The CLI's `attribute` assertions are on two-bme, the dshot and serial fixtures, and the seeded set, all of which are in the sweep. No `ci/` file names the renamed test.
- **bme-bmp is rebuilt, not reused.** `spike/3306/spec/bme-bmp.json` is at schema 19, which `main` refuses (measured). `spike/3307/spec/bme-bmp.json` makes the same two edits to today's `two-bme.json`: U5 becomes a BMP581 at 71.

## 3. The change (§1.3), `crates/aethelgard-evolve/src/lib.rs`

- **`bmp581_bytes`** reads one BMP581 line in the same shape as `witness_bytes` (`what` is `Write` or `Read`):
  - `write 0xRR <- 0xB1 …` is a write of `RR B1 …`;
  - `write 0xRR <- (none)` is a write of `RR`;
  - `read 0xRR -> 0xV1 …` is a write of `RR` and a read of `V1 …`.
- **What it accepts:**
  - Each byte is `0x` and two upper-case digits, single-spaced, as `Join` writes it.
  - The pointer is at most `0x7F`, as the model masks it.
  - The verb and the arrow must pair.
  - A malformed line is no line. A read is at least one byte. A read too wide to pack keeps its pointer as the `Write` and has no value, as the other form's `Write XX` line does (Adv-2). A write too wide to pack is no line.
- **The witness's form is its script's.** If any expected line is a BMP581 line, the witness reads only BMP581 lines; otherwise it reads only `Write`/`Read` lines.
- **`noisy`** is the `[NOISY] {device}: ` prefix, shared by both readers (§Reduction).
- **Unchanged:**
  - `transactions`, `shape`, `surplus` and `sent_elsewhere` (3306) still read only `Write`/`Read`, so a BMP581 log is never a surplus (F1);
  - the 4-byte read limit;
  - the CLI, the IR, the twin and the models.

## 4. Tests and mutants (§1.3)

**Tier 1** (`evolve/tests/attribute.rs`; the file holds 67, and evolve 164):
- **`a_collide_onto_a_bmp581_is_software_by_its_witness`** replaces `a_collide_onto_a_bmp581_stays_i4_because_no_bmp581_witness_is_evidence`, from the same measured records. It asserts:
  - S1, with evidence kinds `[7,7,13,13]`: U2's witness and U5's configure witness, and nothing masked;
  - U5's configure fields exactly (`i2c1.u5.Write` `0x364081` against `0x50`, wrong);
  - U5's identity fields all sound (a warning only);
  - U5's data witness has no fields.
  - It failed on unchanged `lib.rs` (measured: I4, evidence `[3:7,4:7]`). That is the WO's "a test that fails without the change".
- **`a_bmp581_witness_reads_only_its_models_lines_whole`** (§5) covers:
  - the model's line with a wrong value, register, pointer-only write, written byte, or a too-wide read from the wrong pointer, each named exactly;
  - a too-wide read from the right pointer, which shows its pointer and no value;
  - 23 lines that are none it reads, each carrying a wrong value, all showing nothing wrong: another model's or form's line, a warning, another device's, and malformed ones;
  - a malformed script, which is no evidence;
  - a BME280 witness, which does not read the BMP581 form;
  - a silent BMP581 witness under an `ok` claim, which is the software's.
- **The helper `bme_bmp`** builds two-bme with U5 a BMP581 at 0x47. It is the old pinning test's records, factored.

**Mutants** (`~/.cache/aet-3307/mut/mut.py`, measured). Each is applied exactly once to `lib.rs`, the evolve tests run, and the file is restored. The final run is after the reduction pass and the Adv-2 fix: **all 9 killed.** Each was killed by the line meant for it, read from its panic message.

| Mutant | Killed by |
|---|---|
| M1 no BMP581 form (the change undone) | the collide test (I4), and `read 0x01 -> 0x51` |
| M2 either form on every line | `Write 02` in a BMP581 witness |
| M3 lower-case digits | `0x5a` |
| M4 any digit count | `0x5` |
| M5 pointer not 7-bit | `read 0x81 -> 0x51` |
| M6 an empty read accepted | `read 0x01 -> (none)` |
| M7 arrow not paired with the verb | `read 0x02 <- 0x50` |
| M8 any log level | `[WARNING] … read 0x01 -> 0x51` (and 3306's `a_repeat_…`) |
| M9 a read too wide drops its pointer | `read 0x01 -> ` and nine bytes |

- **No tier-2 test was added.** The re-judged archives are the end-to-end proof (§2), and `TIER2_MIN` does not move.

## 5. Adversarial pass, targeted

A fresh Opus subagent ran the pass at `3540dfcf`, in its own worktree. It was given the code, the model sources and the two §5 questions, but not my conclusions, and it probed with throwaway tests (reverted).

- **§5.1, a line another part's model also emits, read as a BMP581's: none found.** These are all refused:
  - our model's other lines (`INT set`/`INT cleared`, its three `[WARNING]`s);
  - the ICM42688P's `bank N …`, the ICM-20948's, the GenericSpiFlash's, the ICP-101xx's `[DEBUG]`, and the BME280's `Write`/`Read`;
  - other device paths (`i2c1.u55`) and other levels.

  No other model's script line parses in the BMP581 form, so a BME280 witness is never read as one. Renode's framework files were not all read.
- **§5.2, a malformed BMP581 line taken as evidence: none.** It found three gaps that hide a fault, the safe side (I4, never a false S):
  - **Adv-1 (medium, pre-existing, not fixed): the `Write` and `Read` fields compare values, not byte counts.**
    - `write 0x00 <- 0x36 0x40 0x81` in configure's slot packs to `0x00364081`, which equals the solved `0x364081`, so it shows right.
    - The `Write`/`Read` form has the same flaw (`Write 0 D0`), but BME280 scripts are identity reads only. The BMP581's 3-byte configure is where it now matters.
    - Fixing it changes how every witness is compared, the BME280's included, which this order's "not any other part's attribution" fences out. **A §7.1 row is proposed:** compare `(value, count)`, with a sweep.
  - **Adv-2 (low, new, fixed): a read of 9 or more bytes lost its pointer.** My first rule read a line whole or not at all. The `Write`/`Read` form keeps the `Write XX` line when its read is too wide.
    - Now a well-formed read too wide to judge keeps its pointer as the `Write`, and has no value. A malformed line is still refused whole.
    - **Tests:** `read 0x02 -> ` and nine bytes is named wrong (`Write` `0x02`), and `read 0x01 -> ` and nine bytes shows its pointer and no value.
    - **Mutant M9** (drop the pointer again) is killed. The empty-read guard is back (M6), since the pack no longer refuses the whole line.
  - **Adv-3 (latent, pre-existing): the `Write` and `Read` can come from different lines,** since `find_map` takes the first line that yields each. It is unreachable today: each step takes one line, and only the last (the 6-byte data read, never judged) takes the rest. The other form behaves the same.
- **Model-log ambiguities it noted (read at source, not parser faults):**
  - a read with no pointer write logs the last pointer;
  - `0x81` and `0x01` log alike (7-bit mask);
  - a write, read, write in one transaction logs as one line;
  - the twin's script pointer is not masked (F3).
- **What held** (run): a FIFO read (`read 0x29 -> 0x7F 0x7F`) named wrong; a pointer-only write in configure named wrong; a write then a read in one transaction read right; a correct line followed by `INT cleared` read right.
- **Re-judged after the fix** (measured): every archive, with a CLI built in the image from `7d198c17`. The result is byte-identical to the first branch judgement (`spike/3307/out/final/`, 68 rows equal).

## 6. Findings

- **F1 (backlog row proposed; outside the fence): a BMP581 log is never read whole.**
  - 3306's `surplus` reads a device's whole log as `Write`-then-`Read` transactions, and it still reads no BMP581 line.
  - So a collide onto a BMP581 is software only by the line distribution. The evidence lands on whichever witness took the misdirected line (here configure, by one line's shift), and the field names U5's `Write` where the cause is U2's address.
  - Neither `{device}.transactions` nor the step's `.address` is named.
  - **Inferred:** reading a BMP581 line as one transaction in `transactions` and `shape` would give the 3306 fields. But U2's pointer is logged masked (`0xD0` as `0x50`), so its step's shape would not match, and only the count would be named.
- **F2 (backlog row proposed; pre-existing): a BMP581's own wrong transaction can stay I4** (ext-cfg37, measured). Two limits of per-witness reading meet:
  - a warning the wrong transaction makes takes its step's line slot;
  - the 6-byte data step is never judged (`read_width > 4`).
  - Whole-sequence reading (F1) would see the wrong `write 0x37` wherever it lands.
- **Adv-1 (§5; §7.1 row proposed): `Write`/`Read` compare values, not byte counts.** A zero byte prepended to a BMP581's configure write passes.
- **F3 (info): the twin's script does not mask a BMP581 pointer, and the model does** (read at source). A step whose first byte is `0x80` or more would be scripted `read 0x81 …`, which the parser refuses. So that step is no evidence, the safe side. No committed step has such a write.
- **Fence:** nothing outside `crates/aethelgard-evolve`, `spike/3307/` and this file. No `ci/` row, since no tier-2 test was added.

## Gates

- **`cargo fmt --all -- --check`:** clean.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean (host).
- **`ci/gates.sh run`** (the `gates` job's set; host, capped user scope, 16G, swap 0): **green**, 2,733 passed and 0 failed over 155 result lines, at `3540dfcf`, and again at `7d198c17`, the final `lib.rs` (2,733 passed, 0 failed; fmt and clippy clean). The head adds one tier-1 assertion after that (the silent witness): evolve's tests, fmt and clippy were re-run on it.
- **Bench `--against-fab`, in CI's layout** (`ci-img.sh`, 16G, on the branch at `06576f66`, whose `lib.rs` is `936b9fa3`'s; the bench board has no BMP581):
  - `main`'s source and the branch give byte-identical `verify` JSON;
  - each derives 17 files, and only `MANIFEST.json` differs from the committed package (`uid_boundary: false`, `user_namespace` unmet: Furnace's sandbox, as 3306 found). The rest are identical.
  - CI's `bench-package` job decides it on the runner.
- **CI:** the PR's checks on its head decide; the result and its run id are posted on the bus with READY.

## Reduction pass

Reduction pass (diff): −2 net · 1 duplicate removed · 0 dead paths removed · 0 protected kept · 0 findings below.
- **The duplicate:** the `[NOISY] {device}: ` prefix, stripped in `witness_bytes` and `bmp581_bytes`, is now one `noisy`.
- **One guard was removed and then restored.** The empty-read guard was removed as equivalent under the whole-line pack. The Adv-2 fix made it load-bearing again, and M6 now kills its removal.
- **Proof:** the 164 evolve tests pass, all 9 mutants are killed on the final head (M8 now mutates `noisy`), and the gates below ran on it.

## What moved

- **Attribution:** bmp-0x47 alone, I4 → S1. It is a spike case; no committed scenario or seeded case moves.
- **Generator report:** a software run with a sound BMP581 witness that disagrees now names its `Write`/`Read`. No committed run has one.
- **Silence:** a BMP581 witness that logged nothing while its check claims `ok` is now software evidence by silence, as a `Write`/`Read` witness already was (asserted in `a_bmp581_witness_reads_only_its_models_lines_whole`). No committed run has one: every BMP581 witness in the sweep logs its script, or nothing under a timeout.
- **Bench package:** no committed verdict moves, and the package does not move.
- **`TIER2_MIN`:** unchanged.
