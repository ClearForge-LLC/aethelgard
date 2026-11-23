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
  - `spike/3307/sh/rejudge.py` then judges each archive on the host with `main`'s CLI and with the branch's. The branch's CLI was built in the image from a `git archive` of `936b9fa3`, whose `lib.rs` behaves as the head's (§Reduction).
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
  - A line is read whole or not at all. A line whose bytes do not fit in 8 is no line, including an empty read.
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
  - the model's line with a wrong value, register, pointer-only write or written byte, each named exactly;
  - 24 lines that are none it reads, each carrying a wrong value, all showing nothing wrong: another model's or form's line, a warning, another device's, and malformed ones;
  - a malformed script, which is no evidence;
  - a BME280 witness, which does not read the BMP581 form.
- **The helper `bme_bmp`** builds two-bme with U5 a BMP581 at 0x47. It is the old pinning test's records, factored.

**Mutants** (`~/.cache/aet-3307/mut/mut.py`, measured). Each is applied exactly once to `lib.rs`, the evolve tests run, and the file is restored. The final run is after the reduction pass: **all 8 killed.** Each was killed by the line meant for it, read from its panic message.

| Mutant | Killed by |
|---|---|
| M1 no BMP581 form (the change undone) | the collide test (I4), and `read 0x01 -> 0x51` |
| M2 either form on every line | `Write 02` in a BMP581 witness |
| M3 lower-case digits | `0x5a` |
| M4 any digit count | `0x5` |
| M5 pointer not 7-bit | `read 0x81 -> 0x51` |
| M6 half a line read | `read 0x01 -> (none)` |
| M7 arrow not paired with the verb | `read 0x02 <- 0x50` |
| M8 any log level | `[WARNING] … read 0x01 -> 0x51` (and 3306's `a_repeat_…`) |

- **One guard was removed as equivalent:** the empty read (`read 0xRR -> (none)`). The whole-line pack already refuses it, since `packed` gives `None` for no bytes. M6 shows the pack is load-bearing.
- **No tier-2 test was added.** The re-judged archives are the end-to-end proof (§2), and `TIER2_MIN` does not move.

## 5. Adversarial pass, targeted

ADVERSARIAL-SECTION

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
- **F3 (info): the twin's script does not mask a BMP581 pointer, and the model does** (read at source). A step whose first byte is `0x80` or more would be scripted `read 0x81 …`, which the parser refuses. So that step is no evidence, the safe side. No committed step has such a write.
- **Fence:** nothing outside `crates/aethelgard-evolve`, `spike/3307/` and this file. No `ci/` row, since no tier-2 test was added.

## Gates

- **`cargo fmt --all -- --check`:** clean.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean (host).
- **`ci/gates.sh run`** (the `gates` job's set; host, capped user scope, 16G, swap 0): **green**, 2,733 passed and 0 failed over 155 result lines, at `3540dfcf`.
- **Bench `--against-fab`, in CI's layout** (`ci-img.sh`, 16G):
  - `main`'s source and the branch give byte-identical `verify` JSON;
  - each derives 17 files, and only `MANIFEST.json` differs from the committed package (`uid_boundary: false`, `user_namespace` unmet: Furnace's sandbox, as 3306 found). The rest are identical.
  - CI's `bench-package` job decides it on the runner.
- **CI:** CI-LINE

## Reduction pass

Reduction pass (diff): −2 net · 1 duplicate removed · 1 dead path removed · 0 protected kept · 0 findings below.
- **The duplicate:** the `[NOISY] {device}: ` prefix, stripped in `witness_bytes` and `bmp581_bytes`, is now one `noisy`.
- **The dead path:** the empty-read guard, equivalent under the whole-line pack (§4).
- **Proof:** the 164 evolve tests pass, all 8 mutants are still killed after it (M8 now mutates `noisy`), and the gates above ran on the reduced head.

## What moved

- **Attribution:** bmp-0x47 alone, I4 → S1. It is a spike case; no committed scenario or seeded case moves.
- **Generator report:** a software run with a sound BMP581 witness that disagrees now names its `Write`/`Read`. No committed run has one.
- **Bench package:** no committed verdict moves, and the package does not move.
- **`TIER2_MIN`:** unchanged.
