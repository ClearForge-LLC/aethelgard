# AET-WO-3306 — FEEDBACK (F-COLLIDE: a step sent to another device's address that answers is the software's)

**Branch and inputs.**
- The branch is `wo/aet-3306-collide`, cut from `main` at `a75fffcc`.
- The work order was read from `~/.cache/aet-3306-wo.md`. The ruling on F1 is in `~/.cache/aet-bus/3306.md` (12:22Z).

Each statement is labelled:
- **measured:** run on Furnace, in the pinned tools image under `capped-run` at nice 10, or on the host under a capped user scope;
- **read at source:** given with file:line;
- **inferred.**

## 0. Status

- **Head:** `main` merged at `dc618bd8` (2305 and 4405 docs, 4405's pincheck), cleanly. `TIER2_MIN` on `main` is still 198, so this order makes it 199.

- **The collide case is the software's, by name** (measured).
  - two-bme with U2's identity read sent to 0x77, where U5 answers, was **I4** on `main` and is **S1** on the branch.
  - The evidence is U2's witness `[28,13]`, U5's witness `[29,13]` and the rises. Nothing is masked.
  - `propose`'s generator report names `i2c1.u5.transactions` (scripted 1, observed 2, at 0x77) and `i2c1.u2.identity.address` (solved 0x76, observed 0x77).
- **No other attribution moves.**
  - Of 67 runs (§2), 66 give byte-identical `attribute` output from `main`'s CLI and the branch's, on the same archives.
  - Every `propose` run gives the same files and exit code.
  - All 154 of evolve's committed tests pass unchanged.
- **F1 (ruled 12:22Z):** a collide onto a BMP581 stays I4, because no BMP581 witness is evidence. This is pre-existing. It is pinned by a named test. The architect has added it to roadmap §7.1, which owes it to "the next attribution work order" (§7).
- **The §7 stop did not fire.** Only the collide case changes attribution.

## 1. Read first (read at source, `a75fffcc`)

- **Why it was I4.**
  - U2's symptom (`i2c_transaction_unwitnessed`) already decided **S1**: its check claims `ok` and its witness logged nothing, the `silent` path in `witness_fields` (`evolve/src/lib.rs` ~L990).
  - U5's symptom decided **I4**: `witness_fields` read only U5's first `Write` and first `Read` (`logged(observed, …)`, ~L987–989). Those were its own, its `absent` was empty, and the bus was healthy.
  - In `attribute`, an indeterminate rule wins the run, so the run was I4.
  - The fix therefore needs U5's witness to become evidence. U2's already is.
- **What a device's witnesses hold.** `renode.rs` `attribute` (~L1025) gives a device's I2C lines to its steps in order: each step takes as many lines as it expects, and the last step takes the rest. A device's witnesses, in record order, therefore hold its whole log, and it is judged there.
- **4241's unmodelled reads** (read on `origin/wo/aet-4241-read-every-identity`): `checks` stays modelled-only, and an unmodelled read gets no kind 12 or 13 (its FEEDBACK, L185–186). So no unmodelled read reaches a modelled witness, unless two devices share an address.

## 2. Measurement (§1.1)

- **Predictions first:** `spike/3306/predictions.md`, committed at `32428d24` before any run.
- **Method:**
  - `spike/3306/sh/measure.sh` makes every run **once**, in the image, by `main`'s CLI (built from a `git archive` of `a75fffcc`).
  - `spike/3306/sh/rejudge.py` then judges each archive on the host with `main`'s CLI and with the branch's. For each run it records:
    - `attribute`'s output;
    - `propose`'s files and exit code, for a software rule;
    - each device's whole-sequence class from `spike/3306/dump` (`dump3306`).
  - This order changes no twin, trace or firmware, so one archive per run is the whole comparison.
  - It also shows the replay clause (§4): the attribution comes from the archive alone.
- **What ran (67 runs):**
  - every committed scenario group (sentinel-01-minimal with the seeded set's unmutated scenarios, sentinel-01-bench, motors, serial, Sentinel-03, sentinel-02-power);
  - the 13 seeded mutants over their own cases;
  - ext-bmp581 and two-bme (baseline and held-low SDA);
  - two-bme with U2's step sent to 0x75 and to 0x77;
  - `spike/3306/spec/bme-bmp.json` (two-bme with U5 a BMP581 at 0x47): baseline, held-low SDA, and U2's step sent to 0x47.
- **Records:**
  - `spike/3306/out/table.md`: every run, with main's and the branch's rule, evidence and masked;
  - `spike/3306/out/rejudge.jsonl`: the same, with the propose files;
  - `spike/3306/out/two-device-records.txt`: every witness's whole `observed` for 0x75, 0x77 and 0x47.

**Device classes:** 48 equal, 24 empty, 3 other, 1 surplus.

| Run | Prediction | `main` (measured) | Branch (measured) |
|---|---|---|---|
| **two-0x77 (F-COLLIDE)** | U2 empty, U5 surplus; I4 → S1 | U2 empty, **U5 surplus** (`Write D0`, `Read 60` twice); **I4** `[4:7,5:7]` | **S1** `[4:7,5:7,28:13,29:13]`, masked `[]` |
| bmp-0x47 (BMP581 answers) | U5 surplus; S1, unchanged | U2 empty, **U5 other**; **I4** `[4:7,5:7]` | I4, identical (F1, §7) |
| two-0x75 | U2 empty; S1 unchanged | S1 `[4:7,5:7,28:13]` | identical (propose files identical) |
| the seeded set (30 cases, 13 mutants) | as `cases.json` pins | every rule, evidence and masked as pinned; `trap-wrong-address-47k` masks `[34:13]` | identical |
| every committed scenario group | unchanged | as their expectations | identical |
| two, ext, bmp: baseline and held-low | N1 / E1, masked `[]` | as predicted | identical |

**Where the predictions missed (measured):**
- **bmp-0x47.** Predicted `surplus` and `main` S1; measured `other` and I4.
  - Our BMP581 model logged `[WARNING] i2c1.u5: access to unmodelled register 0x50` and `read 0x50 -> 0x00` for U2's transfer. That is a wrong transaction, not an extra one.
  - Its lines are not in the `Write`/`Read` format `witness_fields` reads, so no BMP581 witness is ever evidence (F1).
- **Pull-up and capacitance runs past the ceiling** (3k9 to 100k, 220p, 390p; the seeded leave-firmware ones; i2c-clock-skipped) were predicted `equal` and are `empty`.
  - The twin leaves every device out on a bus past its ceiling, so they log nothing.
  - The class cannot move under the rule, and the attributions are as pinned.

## 3. The rule (§1.2), `crates/aethelgard-evolve/src/lib.rs`

- **`transactions`:** a device's lines as transactions. A `Write` line opens one, and the `Read` lines after it are its. A `Read` with no `Write` before it opens one of its own. Any other line (a warning, an error, another device's) gives `None`.
- **`Run::surplus`:**
  - It concatenates the device's witnesses' `observed` and `expected`, in record order.
  - It matches each scripted transaction, whole, to the earliest observed one equal to it (greedy, which is exact for a subsequence test).
  - What is left is the extra transactions, each with the witness its first line went to.
  - It returns nothing unless every scripted transaction was found and every witness is readable. A missing or wrong transaction therefore keeps today's attribution (§1.2).
- **`Run::surplus_fields`, on the witness an extra transaction starts in:**
  - `{device}.transactions`, at the device's address (the address the firmware used): solved is the scripted count, observed is the observed count, and it is wrong.
  - For each extra, when **exactly one** step on the bus has its shape and went elsewhere, that step's `{owner}.{step}.address`: solved is the owner's address, observed is the address used, and it is wrong.
    - "Its shape": the same write, and a read of the same width (`shape` and `step_shape`).
    - "Went elsewhere" (`Run::sent_elsewhere`, after adversarial F1, §6):
      - its record claims `ok`, so something answered;
      - its step found no device at an address other than its own;
      - its device's whole log holds fewer transactions of that shape than its script.
    - The surplus device itself never qualifies: its log holds every scripted transaction and the extra.
  - With two such steps, none, or one that cannot be told (no witness, an unreadable one, or a line that is not a transaction), only the address is named (§7, not a stop).
- **The effect.** The surplus witness is software evidence (`witness_wrong`). So U5's symptom decides S1 and the run is S1. `propose` reports the fields unchanged in shape.
- **No CLI change.** `attribute`'s JSON prints no fields, and `propose` prints `Field`s as they are.

## 4. Replay (§1.3)

- No archive field and no trace change: the rule reads kinds 12, 13 and 15 as recorded.
- **Measured:** each re-judge used the archive alone, on the host, with no Renode.
- **Tier 2:** the tier-2 test attributes the collide archive twice and asserts the same bytes.

## 5. Tests and mutants (§1.4)

**Tier 1** (`evolve/tests/attribute.rs`, 9 new; the file holds 66):
- `a_step_sent_to_another_devices_address_that_answers_is_software`: the collide; S1, U5's and U2's witnesses in the evidence, and both fields exactly;
- `a_transaction_no_scripted_step_lacks_is_software_by_its_address_alone`: no other step lacks its transaction, so only the count is named;
- `a_shared_step_shape_names_only_the_step_that_lacks_its_transaction`: a third BME280 at 0x75 holding its read is not named; lacking it too, neither is named;
- `the_step_named_is_on_the_same_bus_with_the_extra_reads_width`: a device on another bus, or with a two-byte read of the same register, is not a second candidate;
- `the_surplus_is_shown_on_the_witness_its_first_line_went_to`: a two-step U5; the extra lands in its data witness, and its identity witness shows nothing;
- `a_repeat_the_script_holds_or_a_wrong_transaction_is_not_a_surplus`: a scripted repeat passes; two reads after one write, a wrong write, or a warning line keep today's result;
- `a_collide_onto_a_bmp581_stays_i4_because_no_bmp581_witness_is_evidence`: F1, pinned from the measured records (ruled 12:22Z).
- `a_step_that_reached_its_own_device_or_no_device_is_not_named_as_the_surplus` (adversarial F1). None of these steps is named, and each is told apart by one fact:
  - a logged own transaction with a wrong value;
  - a timeout with no absence;
  - an `ok` claim whose step found no device at 0x75.
- `a_candidate_that_cannot_be_told_names_no_step` (adversarial F1): a same-shape device with an `[ERROR]` line, so only the count is named.

**Tier 2** (`cli/tests/twin_tier2.rs`): `a_step_sent_to_another_devices_address_that_answers_is_the_softwares` on two-bme, with the new fixture `two-bme-collide.json`. It checks:
- campaign matched;
- S1 with both witnesses as evidence, nothing masked, subjects U2 and U5;
- `attribute` twice gives the same bytes;
- `propose` exits 6 and names both fields.

**Tier-1 mutants** (`~/.cache/aet-3306/mut/mut.py`, measured). Each is applied exactly once to `lib.rs`, the evolve tests run, and the file is restored. The final run is after the reduction pass: **all 14 killed**.

| Mutant | Killed by (first of) |
|---|---|
| M1 first write and read only | the collide test and 5 others |
| M2 the extra treated as the bus (count field not wrong) | the collide test and 4 others |
| M2b the named step not wrong | the collide test and 3 others |
| M3 a missing transaction beside an extra still a surplus | `a_repeat_…`, `a_read_counts_only_once_…` |
| M4 a warning line counted as a transaction | `a_repeat_…` |
| M5 every `Read` opens a transaction | 6 tests |
| M6a the owner need not claim `ok` | `a_step_that_reached_…` |
| M6b the owner may have found no device elsewhere | `a_step_that_reached_…` |
| M6c the owner need not lack its transaction | `a_shared_step_shape_…` and others |
| M6d a candidate that cannot be told is dropped | `a_candidate_that_cannot_be_told_…` |
| M8 a device on another bus can be the owner | `the_step_named_…` |
| M9 the read width not compared | `the_step_named_…` |
| M10 fields on every witness of the device | `the_surplus_is_shown_…` |
| M11 two candidates name the first | `a_shared_step_shape_…`, `a_candidate_…` |

- **Survivors along the way, each answered by a test:**
  - M8, M9 and M10 survived the first round, and `the_step_named_…` and `the_surplus_is_shown_…` were added.
  - After the F1 fix, M6a and M6b survived until each was given a case told apart by its one fact.
  - M8 and M9 survived again until the helper's candidates claimed `ok` with a script of their own width. They had been excluded for another reason.
- **M7 (the device itself can be the owner) is equivalent** under the F1 rule, so its guard was removed (reduction).

**Tier-2 mutants** (`~/.cache/aet-3306/sh/mut2.sh`, measured in the image on a source copy of the merged head `030cc7f6`'s code before the reduction pass; the reduction's own proof is §Reduction). Both are killed by the collide tier-2 test:
- **T1, first write and read only:** `attribute` gives `I4`, `ok:false`, evidence the rises only, which is `main`'s result.
- **T2, the extra treated as the bus:** the same I4.
- **T0, unmutated:** passes (with 3305's two-bme test, 9.3 s).

## 6. Adversarial pass (full)

A fresh Opus subagent did the pass at `63a8b495`, in its own worktree, given the code and the work order but not my conclusions. It had main's behaviour emulated: the lib.rs diff is additive, so it stubbed `surplus_fields`.

- **Adv-F1 (low to medium, new, run): the owner was named on weak grounds. Fixed.**
  - A candidate step only had to have a witness that was not exactly its script. A timeout to an absent address therefore qualified, and so did a logged transaction with a wrong value.
  - Both cases had already decided S1, and correctly. The fault was a false field in the generator report:
    - one case named U2's step at 0x77 while U2's own witness said 0x75 (ADV-1b);
    - one named U2 though U2 logged its own transaction (ADV-1a).
  - **The fix,** `Run::sent_elsewhere`. A step went elsewhere only when:
    - its record claims `ok`, so something answered;
    - its step found no device at an address other than its own;
    - its device's whole log holds fewer transactions of its shape than its script.
  - **A candidate that cannot be told** (no witness, an unreadable one, or a line that is not a transaction) names nothing.
  - **Tests:** `a_step_that_reached_its_own_device_or_no_device_is_not_named_as_the_surplus` covers ADV-1a, plus a timeout and an `ok` claim found absent at 0x75, each told apart by one fact. `a_candidate_that_cannot_be_told_names_no_step` covers the unreadable candidate.
  - **The collide still names U2.** Its check is `ok`, `absent` is `[]` and its log is empty. Measured again on every archive, `spike/3306/out/final/`: identical to the first branch measurement.
- **Self-exclusion became equivalent, so its guard is removed.** Under the fix, the surplus device can never be its own owner: its log holds every scripted transaction plus the extra one.
- **Adv-F2 (low, pre-existing, run): a read phase sent to another device on its own stays I4.**
  - `transactions` opens a transaction on a `Read` only first in the lines, so a stray read merges into the transaction before it.
  - A partial witness (`Write D0` alone) is no evidence today.
  - Reachability (read at source): the generated firmware makes each step one `transfer(addr, &write, &mut buf)` (write, repeated start, read) at one address. A wrong address therefore moves the whole transaction, which is the case built here.
  - Proposed as a §7.1 row only if the firmware ever splits a transfer. The `transactions` doc was corrected to say "first".
- **Adv-F3 (info, new, run): on a faulted bus, an extra transaction adds U5's witness to `masked`,** so `verify`'s detection refuses the run as "something else fails too".
  - Every faulted-bus run measured has its devices' logs empty (§2: the twin leaves them out). An extra on a faulted bus is therefore a second fault, the firmware's, and refusing it is the intended trap (`INV-05`).
  - In the full collide on a faulted bus, `masked` was already `[U2]` on `main`.
- **Latent, unreachable today (read at source).** Only the BME280 model logs `Write`/`Read` lines. Its only witnessed step is identity, at 0x76 or 0x77. So none of these occurs now:
  - a witness with a read wider than 4 bytes returns before `surplus_fields`;
  - extras starting in two witnesses each carry the count field;
  - 7 or more extras would meet the 16-line cap.
- **What held:**
  - §5.1: a scripted repeat, and identical scripted steps, give N1, and the firmware has no retry (read).
  - §5.2: a shared shape, the mirror collide (U5's step sent to 0x76) and a wrong own read plus a collide, each naming correctly.
  - §5.3: 4241 adds no record kind, and U3's (TMP117) read sent to 0x76 names the count only.
  - The `[ERROR]` line (I4, as on `main`).
  - Kind-13 order against step order: replay refuses a swap.
  - A transaction spanning two witnesses.
  - `std::ptr::eq`, both callers indexing `self.records`.
  - Greedy matching.
  - No electrical case moves to S1.
  - `INV-04`: no new archive field.
  - Every droop run measured (§2: Sentinel-01, the bench, Sentinel-03 and the motors' droop) classifies `equal`. A reset replays no transaction into a device's log, so no reset is read as software (`INV-05`).

## 7. Findings and rulings

- **F1 (ruled 12:22Z: a §7.1 row, not this order's). A collide onto a BMP581 stays I4.** Measured on bme-bmp, with U2's step sent to 0x47.
  - Our BMP581 model logs `read 0xRR -> 0xVV` and `write 0xRR <- …`. `witness_fields` reads only `Write XX` / `Read XX` (`witness_bytes`), so it returns `None` for every BMP581 witness. No BMP581 witness is evidence either way.
  - This is pre-existing (read at source): a wrong register on a BMP581 is I4 on `main` too.
  - The boards it reaches: Sentinel-03's U7, and Sentinel-02's barometers.
  - **The roadmap row** (§7.1, added by the architect at #284) owes it to "the next attribution work order".
  - The row that fixes it must flip `a_collide_onto_a_bmp581_stays_i4_because_no_bmp581_witness_is_evidence`.
- **What the rule can and cannot tell (inferred from the code, then tested):**
  - It names the extra address, and its device's count.
  - It names whose step it was only when exactly one same-bus device, with a step of that write and read width, lacks its transaction.
  - A collide between two parts whose steps differ in shape names the address only.

- **Adv-F2 (§6): a read phase sent alone stays I4.** Pre-existing. It is unreachable while the generated firmware makes each step one transfer at one address (read at source). Proposed as a §7.1 row only if a transfer is ever split.
- **Adv-F3 (§6): an extra transaction on a faulted bus is masked,** and `verify` refuses the detection. Intended: it is a second fault, the firmware's.
- **The count field's mask is `u64::MAX`,** so the report shows `0xffffffffffffffff`. It is a whole count with no width, and the report prints every value in hex when an address is set. This is cosmetic; I left it.
- **The predictions that missed (§2) are explained:** bmp-0x47 (F1), and the runs past the ceiling, whose devices log nothing.
- **Committed attributions not re-derivable:**
  - `spike/4600/out/*/attribute.json` are records of runs at older trace versions with no committed archive. They are not re-judged here.
  - The `spike/3305` archives are at trace 16, which `main`'s exact-version envelope refuses. The collide was re-made at trace 17 (§2).
- **Fence edges, said on the bus at push time:**
  - `ci/test-weights.tsv`: one row and its provenance line. twin_tier2 is lock-serialised, so every test needs a row.
  - `.github/workflows/ci.yml`: the `TIER2_MIN` line and its comment, as §2's acceptance asks.
  - `crates/aethelgard-cli/tests/fixtures/two-bme-collide.json`: a scenario fixture for the tier-2 test.
- **For the architect: no committed verdict moves, so the bench package does not move.** Measured locally in CI's layout (`verify --against-fab`, 16G, worktree at `/repo`, target inside):
  - `main` (`dc618bd8`) and the branch give byte-identical `verify` JSON;
  - both report `against_fab: differs` on `MANIFEST.json` alone, with the same derived hash `4ff34066…`, a difference of this machine's sandbox (the verdict names it), not of this order;
  - every Gerber, drill, BOM, CPL and FAB-NOTES file is identical.
  - CI's `bench-package` job decides it on the runner. No `write=true` dispatch is expected.

## Gates

- **`cargo fmt --all -- --check`:** clean.
- **`cargo clippy --workspace --all-targets -- -D warnings`:** clean, with and without `aethelgard-cli/with-tools` (host).
- **`ci/gates.sh run`** (the `gates` job's set, on the host, target inside the repository): **green**, 0 failed over 139 result lines each time. Run three times:
  - at `63a8b495` (2,362 passed);
  - at the merge of `main` `dc618bd8`, 2305 docs and 4405 (2,367 passed);
  - at the reduced head `faf36aeb` (2,367 passed).
- **Tier 2 in the pinned image** (measured):
  - the new collide test and 3305's two-bme test pass at every stage, the last at `faf36aeb` (8.65 s for both);
  - the seeded set (`attribute_seeded`) passes 11 of 11, every case's exact evidence and masked included: at `63a8b495` (293 s), and on the final head `faf36aeb`'s code (287 s). It also passed beside the T1 and T2 mutant builds, which change no seeded case.
- **Bench `--against-fab`, in CI's layout:** byte-identical to `main` (§7), at the merged head before the reduction pass. The reduction changes no result (§Reduction).
- **`ci/shard.sh plan`,** in the image in CI's layout, with the new weight row: **guard ok**, 2,642 test names over 12 shards (685 s planned each). The first run refused because it compiled, a known artefact of a fresh target; the second ran on built binaries.

## Reduction pass

Reduction pass (diff): −8 net · 2 duplicates removed · 1 dead path removed · 0 protected kept · 0 findings below.
- **The duplicates:**
  - `surplus` and `sent_elsewhere` each concatenated a device's witnesses; they now share `Run::whole_log`.
  - `witness_fields` re-derived a step's write value by the fold `step_shape` holds.
- **The dead path:** the self-exclusion guard, equivalent under `sent_elsewhere` (M7).
- **Proof:** the 163 evolve tests pass, all 14 tier-1 mutants are still killed, the 67-run re-judge is identical (`spike/3306/out/final`), and the gates above were run on the reduced head.

## What moved

- **Attribution:** two-0x77 alone, I4 → S1 (a fixture case; no committed scenario or seeded case moves).
- **Generator report:** a software run with a surplus gains the two fields. No committed run has one.
- **Bench package:** no committed verdict moves, and the package does not move. The branch's `verify` JSON is byte-identical to `main`'s (§7).
- **`TIER2_MIN`:** 198 → 199, this order's one tier-2 test.
