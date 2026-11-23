# AET-WO-2312 — FEEDBACK: replay and attribute bound to the records

**Status:** built to the leans below, unmerged. **Branch:** `wo/aet-2312-replay-records`.
- The WO names `-replay-bound-to-records`; the kickoff and the operator's goal named this one.
- Main `7dec1d7b` is merged (it has not moved).

**The stop.**
- The WO's §1.2 stops unconditionally after the measurement.
- The operator's `/goal` for this session stops only on three conditions: a committed verdict moves, a format bump looks needed, or `svd-hal` is needed in chaos. None fired.
- So the build went ahead on the leans in §4. The table and the leans were posted on the bus first (`architect.md`, "from 2312, …, §1.2 measured").
- Each lean is reversible on a ruling.

## 0. How it was made

- **Predictions** were committed alone, before any output (`79edf4a3`, `spike/2312/predictions.md`).
- **The "before" side** is main `7dec1d7b`'s CLI:
  - built in the pinned tools image under `capped-run` (8 GiB, swap 0, read back);
  - it ran 2310's sweep unchanged (`spike/2312/sh/measure.sh`): 64 CLI-made runs;
  - it replayed and attributed every archive on the host (`sh/sweep.sh`).
- **The probe** (`spike/2312/probe`, its own workspace) holds two things:
  - a prototype of (1), with `select`'s predicates copied, to compare the two before any crate moved;
  - the reviewer's shapes and the cost archives (`make`), on the tests' `archived` header.
- **Results:** `spike/2312/results.md` (P1–P11 against what was measured).

## 1. What was built

1. **(1) `campaign::shown(records) -> Vec<Shown>`**, in chaos, `pub`.
   - Each of 29 kinds (`SHOWN`, each with its refusal reason), then `locked_peripheral_access`. For each kind: the records that show it, and a `Bound`:
     - `Equal`: the verdict's count is exactly their number;
     - `Reach(n)`: at least `n`, because the judge also names the kind for facts no record holds;
     - `AtMostOnce`.
   - It follows the judge (`cli/src/renode.rs`) rule by rule, as its doc lists.
   - It is linear: each read finds its witness through one index. That also replaces 2310's quadratic `unread_ok_witnesses` (K=200 `big_pass` replay went from 10.6 s to 0.62 s).
2. **(2) `check_shown`**, called by `check_run` after its older checks, so their reasons still fire first. Every kind's count must lie in its bound and be an error, never `info`. The refusal names the kind.
3. **(3) `attribute` takes its symptoms from `shown`.**
   - A failing kind is explained when its records are non-empty and, unless `AtMostOnce`, their number is the count.
   - The converse rule, an event no verdict names, walks `shown`'s records.
   - `select`, `PER_EVENT`, evolve's `read_matches` and `mismatch_kind`, and `Run`'s RCC range are removed. B6's "one shared function" is met, and `evolve` holds no judging of its own for these kinds.
4. **(4) `MAX_ATTRIBUTE_RECORDS` = 1,024**, refused as `AttributeError::TooManyRecords { records, limit }`.
5. **(5) The two bare `is_err()` tests** assert their reason: `evolve/tests/serial.rs` (`a_rewritten_verdict_cannot_move_the_domain`) and `chaos/tests/battery.rs` (two rails out of band).
6. **(6) The reviewer's four shapes,** built in code at trace 19 (see the deviation below). Replay refuses each by the kind its records show, and attribute refuses with the same reason.
   - CLI on the probe's archives, measured: exit 3, `NotACampaignRun` naming the kind.
7. **B6 doc:** `replay.rs`'s `ONCE_KINDS` doc was attached to `MAX_RECORDS`. Moved, and it says why the check stays.

**Every test whose premise changed** (architect, 11:00Z: each asserts the refusal kind by name).
- A changed **fixture** is one the judge could not have written. Each such test keeps its corrected fixture and also asserts that the old form is refused, naming the kind.
- A changed **premise** was "a verdict the records contradict is `I1`". Each such test now asserts the refusal by kind, through `refusal(kind)` or `why(kind)`, read from `campaign::SHOWN`.
- No assertion was weakened to pass.

| Test (file) | Was | Now |
|---|---|---|
| `a_held_low_run_is_ok_and_matches` (chaos `campaign.rs`) | `held_low_run`'s verdict omitted `good()`'s timed-out read | names `i2c_timeout` too, as every committed held-low scenario expects; the old verdict is refused: `i2c_timeout` |
| `a_boot_that_read_nothing_names_its_bound` (chaos) | an unread boot's verdict named `i2c_timeout` | none named; the old verdict is refused: `i2c_timeout` |
| `two_devices_on_one_controller_have_two_witnesses` (chaos) | two timed-out reads, `i2c_timeout` 1 | 2; the old count is refused: `i2c_timeout` |
| `a_record_judged_without_the_boot_word_is_refused` (chaos) | booted, any booted-read kind accepted ("replay binds no count") | a mismatch kind with no record is refused by that kind; `record_invalid` and `unwitnessed` are reached (D1, D2) |
| `replay_binds_the_anchor_findings_kinds_26_and_30_carry` (chaos `motor.rs`) | `left` moved kind 11's expected CR1, not its observed | observed as the stop left it; the old form is refused: `register_mismatch` |
| `the_view_reads_every_field_of_a_rise_a_stuck_line_a_boot_and_a_witness` (evolve `propose.rs`) | unbooted, 7 instructions, no `cpu_halted` | `cpu_halted` named (D4); the old verdict is refused: `cpu_halted` |
| `a_ports_clock_gate_left_off_is_software_and_explains_the_locked_access` (evolve `attribute.rs`) | `locked_peripheral_access` 3 | 1; 3 is refused: locked more than once (D3) |
| `a1_…_is_refused` (was `…_leaves_it_unexplained`; `attribute.rs`) | `I1` with `event N` | refused: `i2c_timeout` (three cases) |
| `a_finding_count_the_events_do_not_reach_is_refused` (was `…_unexplained`) | `I1` | refused: `i2c_timeout` |
| `a_timeout_claimed_as_an_invalid_record_is_refused` (was `…_unexplained`) | `I1` | refused: `i2c_timeout` |
| `an_identity_mismatch_dressed_as_another_steps_is_refused` (was `…_unexplained`) | `I1`, the dressed kind unexplained | refused: `i2c_identity_mismatch` |
| `every_kind_of_failing_event_the_verdict_omits_is_refused_or_unexplained` (was `…_is_unexplained`) | `I1` for each | refused by `i2c_record_invalid`, `i2c_identity_mismatch`, `register_mismatch` (register and locked); a differing witness stays `I1` (D2) |
| `a_masked_identity_check_counts_as_passing` | `I1` | the claimed mismatch is refused: `i2c_identity_mismatch` |
| `changing_only_the_verdicts_finding_kinds_does_not_move_the_domain` | the forged timeout was `I1` | refused: `i2c_timeout` |
| `each_finding_selects_only_the_events_that_show_it` | `I1` four times | refused: `i2c_timeout` (three) and `register_mismatch` |
| `adversarial_4_a_rewritten_verdict_cannot_move_the_domain` (evolve `motor.rs`) | `I1` | refused: `register_mismatch`, by replay and attribute |
| `a_rewritten_verdict_cannot_move_the_domain` (evolve `serial.rs`) | `I1`; then a bare `is_err` (review A15) | refused: `register_mismatch`; the bare `is_err` now asserts "the verdict's serial findings do not match the serial records" |
| `a_configure_steps_mismatch_is_named_by_its_step` (evolve `spi.rs`) | the dressed identity was `I1` | refused: `spi_identity_mismatch` |
| `a_data_step_found_absent_is_electrical_e2_by_that_steps_witness` (evolve `spi.rs`) | the dressed identity was `I1` | refused: `spi_identity_mismatch` |
| `a_rail_out_of_band_without_its_rendering_is_refused_in_replay` (chaos `battery.rs`) | a bare `is_err` (review A15) | asserts "more than one rail out of band in one scenario: their interaction is not modelled" |

**New tests.**
- Chaos `campaign.rs`:
  - `each_kind_the_records_show_binds_the_verdicts_count`: every status spelling, a zero mask, a configuration's bits, the boot gate, `Reach`, the no-witness-format step, `cpu_halted` unbooted, and the locked access;
  - `shown_names_every_kind_once_and_finds_the_locked_access_by_name`.
- Evolve `attribute.rs`:
  - `the_round_3_archives_are_refused_as_replay_refuses_them`: the four shapes, built in code, through replay's `check_archived` and attribute;
  - `a_records_value_before_the_boot_word_shows_nothing_to_attribute`: kills "(3) back on `select`";
  - `a_run_past_the_attribution_bound_is_refused_by_name`;
  - `a_reached_count_past_its_records_is_unexplained`.

**(6), and a deviation from the WO.** The shapes are not committed as archives. `cli/tests/workspace_shape.rs` holds chaos to `.rs` files under `tests/`, and that test is outside the fence. So they are built by hand through the tests' `archived` helper, which is the WO's own fallback. The reviewer's mutations are applied to evolve's `healthy()`, the run the reviewer's probe copied. The trace-19 archives made by the probe are in `~/.cache/aet-2312/made` (1.2–1.3 KB each), and `results.md` P9 measures them.

## 2. Measurement and acceptance

| Acceptance | Measured |
|---|---|
| The four shapes are refused by `replay` (nonzero, JSON naming the kind) and `attribute` (`NotACampaignRun`) | Yes, by the CLI (exit 3) and by both crates' tests. `halted_unnamed` was already refused at main (2310) |
| No committed or CLI-made verdict, `matched` or `Attribution` moves | 5 committed archives: refused by version, before and after. 64 CLI-made: 128 of 128 outputs identical, main against branch (`out/sweep-*.txt`) |
| `attribute` on 5 MB within 10× of `replay`, or refused by name | Refused `TooManyRecords` in 0.55 s, against replay's 0.54 s. Main: 251 s at 2.5 MB, and over 900 s at 5 MB (killed at the cap) |
| Mutants: a kind dropped from (1); `==` to `>=`; (3) back on `select`; (4)'s bound removed | All killed by named tests, with 14 more (§3) |
| CLAUDE.md gates, CI green | §7 |

## 3. Mutants (sized to the diff)

**24 of 24 killed** on `5ba62a28`, each by a named test against a green baseline (`spike/2312/sh/mutants.py`; `out/mutants-final.txt`):
- **kinds dropped:** `register_mismatch`, timeouts, read mismatches, `record_invalid`, `cpu_halted`, the locked evidence;
- **bounds:** `Equal` to `>=`; `AtMostOnce` unbounded; `Reach` made `Equal`; `info` counted; the boot gate dropped; the unread bound dropped; a no-format unread step counted as unwitnessed; locked evidence by any name; `check_shown` not called;
- **after the adversarial pass:** the status set dropped; the joint bound dropped; `equal_fits` always true; an `[ERROR]` line ignored; witness steps judged before the boot word; the no-format joint dropped;
- **attribute:** (4)'s bound removed; exactness dropped; the converse loop dropped.

"(3) back on `select`" is killed by `a_records_value_before_the_boot_word_shows_nothing_to_attribute`. There `select` showed an unbooted read's value, and the judge, and so `shown`, show nothing.

**Two lessons:**
- **The first run's last three mutants were contaminated** (`out/mutants-run1.txt`). Each restore put back `campaign.rs` with an mtime older than the mutant's build, so the evolve mutants ran on M9's stale chaos. Restores now touch the file. That is 2310's rsync-mtime lesson again.
- **M11 first survived:** no test pinned a reached count past its records. The new test's first form then failed on the unmutated branch. Its assertion was corrected before the kill was counted (`out/mutants-run3.txt`).

## 4. Decisions for the architect

- **D1, `{bus}_record_invalid`: `Reach`, lean.**
  - An unmodelled device's read is not archived (`main.rs:2441`, `verdict.rs:309-318`).
  - The judge names its invalid status by the same kind (`unmodelled_finding`).
  - So the records bound the count from below only. A verdict may add invalid reads no record shows. Replay cannot refuse that, and attribute leaves the excess unexplained.
- **D2, the three witness findings: accepted as `Reach(unread)` at 11:00Z, tightened after the adversarial pass (§5).**
  - Per kind: `step_error` reaches the archived `[ERROR]` lines; `witness_unavailable` and `unwitnessed` reach the unread `ok` reads.
  - Jointly with `step_error`, they reach the steps whose `[ERROR]` line may have been cut.
  - 2310's bound counted an unread step with no witness format as unwitnessed. The judge names it `witness_unavailable`, so it is now that kind's.
  - Attribute's evidence for unwitnessed is every `ok` read whose witness differs, as before. The other two kinds give attribute no records. Posted on the bus.
- **D3, `locked_peripheral_access`: `AtMostOnce`, lean.**
  - The judge names it once, from Renode's log, which is not archived.
  - Its evidence is an `RCC.` register read with an enable bit off, by name. On all 1,890 kind 11 records the name equals `svd-hal`'s address range, so chaos needs no `svd-hal` (B6).
  - That evidence is neither necessary nor sufficient for the access. So nothing binds it from below, and a firmware that touches a gated peripheral and then enables its clock replays whatever the verdict says up to 1.
- **D4, `cpu_halted` at any boot read, a tightening of 2310.**
  - The judge pushes it whenever the core executed fewer instructions than the run holds, the boot word set or not (`renode.rs`, `let halted`).
  - 2310 bound it only once booted. `shown` binds it at any boot, which is also what `select` gave attribute.
  - No archive exercises it unbooted.
- **D5, (4) by a bound, not an index, and its fence.**
  - Indexing every per-symptom scan (`decide`'s `faulted`, `software`, `healthy`, `surplus`, `whole_log`, `check_of`, `bus_of_device`) is a refactor of attribution. Under a records bound alone attribute stays cubic: 14 s at 4,604 records. At about 1,000 records it is 0.63 s.
  - The new variant needed one match arm each in `cli/src/campaign.rs` and `cli/src/propose.rs`, which are outside the fence. They are mechanical: refuse `TooManyRecords`, exit 3. Ruling wanted: keep them, or have the bound use an existing variant.
  - Largest real run: 113 records.
- **D6, the branch name** (above).

## 5. Adversarial pass (full, a fresh Opus subagent, frozen at `1d0c89b3`)

The reviewer was given the code and the judge, not my conclusions. It wrote 22 probes in its own copy, now kept as `~/.cache/aet-2312/adv-probes/{chaos,evolve}-adv2312.rs`. I re-ran F1–F4 there; all four reproduced.

**Fixed, in `5ba62a28`:**
- **F1, false pass.** An `ok` read with no witness format (empty `expected`) under a passing verdict. The judge names `{bus}_witness_unavailable`, an error (`renode.rs:4029-4036`).
- **F2, false pass and disagreement.** An `ok` read whose short, whole witness differs, verdict silent. Replay passed it while attribute said `I1`. The judge names `{bus}_transaction_unwitnessed` (`renode.rs:4037-4043`).
- **F3, false pass.** An `[ERROR]` line in a witness, at any status. The judge names `{bus}_step_error` (`renode.rs:4003-4013`).
- **F4, false pass on forged input only.** A status the judge never writes (`"bogus"`, I2C `timeout_rxp`). `status_name` writes a closed set per bus (`renode.rs:4368-4379`).

**The fix** (`witness_steps`, `check_shown`), once booted. Each rule is an implication from archived bytes to the judge's output:
- **an archived `[ERROR]` line** reaches `{bus}_step_error` (a per-kind `Reach`);
- **an unread `ok` read** (`observed` absent, so no line) reaches `witness_unavailable` with no format, or `unwitnessed` otherwise. This is exact: no line can hide an `[ERROR]`;
- **where an `[ERROR]` line may have been cut away**, the step counts jointly with `step_error`: `n(kind) + n(step_error) ≥ certain + joint`. This covers an `ok` read with no format whose log was read, and a differing witness that would have archived whole had it equalled the model's lines. The latter is `equal_fits`: the reader holds `expected` to `text_ok` and 16 lines, and `fit` never cuts `expected`;
- **a closed per-bus status set.**
- `step_error` and `witness_unavailable` join `BOOTED_READ_FINDINGS`, so neither is named before the boot word.
- Attribute takes no records for those two kinds. A run naming them is `I1`, as on main.

**Must-accept tests** (genuine judge outputs) are pinned in `the_witness_findings_and_the_statuses_are_bound_as_the_judge_writes_them`:
- an `[ERROR]` past line 16 with only `step_error` named, with and without a witness format;
- a matching witness `fit` popped, in a passing run;
- a timed-out read with a differing witness;
- an unbooted differing witness.

**Kept as nits** (rows in §6):
- **N1a.** A genuine pass whose matching witness `fit` cut is `I1` in attribute: its evidence is every differing witness, as `select`'s was, and the converse rule flags it. Replay accepts it.
- **N1b.** A genuine `locked_peripheral_access` with every RCC bit right is `I1`. There is no evidence for it (D3).
- **N1c.** A reached count past its records is `I1` by design (`a_reached_count_past_its_records_is_unexplained`).
- **N2.** Shared refusal texts:
  - unwitnessed is now per bus;
  - the motor, serial and brown-out kinds keep the older checks' shared texts, which fire first.

**Not broken:**
- every `SHOWN` kind, omitted or at `info`; every status spelling; a zero mask; bits above the width;
- an inflated `Equal`, a deflated `Reach`; a booted-read finding before the boot word; an unbooted `cpu_halted`;
- the Ok-against-`NotACampaignRun` sense of disagreement (attribute calls `check_archived` first; `TooManyRecords` is attribute's own, by design);
- cost: 1,024 records with 1,010 symptoms took 257 ms;
- **`ONCE_KINDS`:** kept. Without it a duplicate is refused only by `check_run`, without its index.

**Unverified** (the reviewer read these, did not probe): DShot CEN's adjusted `expected` in kind 11 (`main.rs:2420-2427`); `windowed()`'s panic path.

## 6. Findings (rows for §7.1)

- **R1 (D1):** `{bus}_record_invalid`'s upper side is open. Closing it needs the unmodelled reads archived, which is a trace bump.
- **R2 (D3):** `locked_peripheral_access` has no lower bound. Renode's `LOCKED` lines are not archived.
- **R3:** `serial_capture_forged` with a port is `Reach`, as `check_run` had it. Attribute now needs the witness count to equal the verdict's, where `select` allowed any count with one witness. No archive names it.
- **R4 (§5 N1a):** a genuine pass whose matching witness `fit` cut is `I1` in attribute. Telling it apart needs the cut recorded, which is a trace bump.
- **R5:** what a cut log hides stays joint. Which of `step_error` or the named kind a cut step is cannot be told from the archive. A cut flag on kinds 13 and 22 would make each exact (a trace bump).
- **R6 (reduction, for the phase gate):** `check_shown` re-binds kinds `check_run`'s older checks already bind (analog, held low, boot word, pc, brown-out, flash, motor ×3, serial ×5, `cpu_halted` once booted). They run first, so their reasons are unchanged. Folding them into `shown` would move which reason fires first on a run with several faults, so it is not done here.

## 7. Gates, fence, reduction pass

- **Gates** on the final head, capped (32 GiB, swap 0, 6 jobs):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace`: 163 of 163 test binaries ok, on `0f592518` (11:17–11:37Z).
  - CI: on the PR.
- **Fence:** chaos (`campaign.rs`, `replay.rs`, tests), evolve (`lib.rs`, tests), `spike/2312/`, this file. Plus one match arm each in `cli/src/campaign.rs` and `cli/src/propose.rs` (fence widened at 11:00Z).
  - No `fab/`, `scenarios/`, IR, eda, generator, judge, `verify.rs`, `mcp.rs` or workflow byte.
  - No `ci/test-weights.tsv` row: every new test runs well under 0.5 s.
- **Reduction pass (diff): −3 copies of one fact · 2 duplicates removed · 1 dead path removed · 1 protected kept · 1 finding (R6).**
  - **Duplicates removed:**
    - the boot-word predicate, three copies (`check_run`, `shown`, evolve's `joined`), is now `campaign::booted`;
    - `read_matches` and `mismatch_kind` lived in evolve as copies of the judge's rules. They are now in chaos only, beside `shown`.
  - **Dead path removed:** `unread_ok_witnesses` and its quadratic join, folded into `witness_steps`.
  - **Protected and kept:** evolve's `belt-and-suspenders` boot filter (`lib.rs`, the DShot window path).
  - **Finding:** R6, the older per-kind checks `check_shown` re-binds. That is gate-sized.
