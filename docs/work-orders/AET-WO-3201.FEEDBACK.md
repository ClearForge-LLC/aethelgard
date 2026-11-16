# AET-WO-3201 FEEDBACK — evolve, replay and the judge

**Branch:** `wo/aet-3201-evolve` · **Base:** `8030fa5` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build, with one measured stop (§1.1).

Labels: **measured** (run on this branch), **read** (read at source, file:line at `8030fa5`), **inferred**.

## §1.1 — where each item lives and what it costs (committed before any measured run)

**No item needs a trace-version bump** (read). So §7's first stop does not fire, and the build goes on.

- **Severity (B3).** It is archived in the CBOR, not just the verdict JSON: kind 14 carries `[kind, severity, count]` per finding, and `severity` is a text field (`crates/aethelgard-chaos/src/record.rs:368`, encode at `:1121`). Today's read checks it only as 1 to 64 printable bytes (`record.rs:1942`). The judge writes two values in a campaign verdict, `error` and `info` (`crates/aethelgard-cli/src/verdict.rs:699`, `:710`). KiCad's `warning` (`kicad.rs:182`) never reaches a campaign. A closed set `{error, info}`, checked in `Record::check` (which runs on write and on read), changes no byte and no encoding. **No bump.**
- **B1 and B2: committed replay hashes.** The state hash is the fold's (`replay.rs`). The joins are judged afterwards, in `campaign::check_run` → `check_joins` (`campaign.rs:735`, `:1911`), which reads the records and adds no byte. A new refusal there cannot move any hash (read). The golden stream (`chaos/tests/golden.rs`) holds no campaign record, so `check_run` never sees it (read).
- **B1 and B2: seeded answers.** The literal rules would refuse honest runs, so **two of the work order's wordings need an exception, which I build and name here**:
  - **B1, held low.** A held line has no kind 7 or 8 *by design*. The analog pass leaves it out of the decks (`cli/src/analog.rs:160-164`), and its fault is a kind 9 `from: [held_low]` naming that net (`:184-200`). "Each kind-15 line requires its kind 7 or 8" would refuse `held-low-sda` and `held-low-scl`, both E1 seeded cases: a §7 stop. **Built instead:** each kind-15 line has exactly one of a kind 7, a kind 8, or a held-low kind 9 naming that net. This is what the campaign writes (read), and no seeded case moves (predicted).
  - **B2, an unread boot.** A boot killed at a bound is honestly unread, and its verdict holds exactly one of `cpu_aborted`, `wall_bound_exceeded` or `memory_cap_exceeded`, **never** `boot_word_missing` (`cli/src/main.rs:1563-1567`, `renode.rs:263-290`). "No read and no `boot_word_missing` is refused" would refuse every honest killed boot. **Built instead:**
    - a boot that read nothing must name the bound it was killed at;
    - a boot that was read names `boot_word_missing` exactly once when its word is not the expected one, and never otherwise (the judge, `renode.rs:2142`).

    No seeded case is killed at a bound, and every seeded case's verdict already obeys this (predicted).
- **F-B10.** A finding's kind is text in kind 14, so a new name needs **no new trace kind**: only the judge, evolve's table and replay (read).
- **F15** changes only the verdict of an unbooted run with a timed-out record. No committed archive or hash holds one, and no seeded case is one: `no-boot-word` reads its device before skipping the word (`3100` F15). So no hash moves (predicted).

### Predictions (before measurement)

| # | Prediction | Basis |
|---|---|---|
| P1 | Every seeded case keeps its manifest's domain, rule, evidence and masked, index for index. | No new record in any run; F15 touches unbooted runs with timeouts only |
| P2 | `held-low-sda` and `held-low-scl` replay under the new B1 rule (their held line is bound by its kind 9) | read, as above |
| P3 | The golden trace and state hashes are unchanged | read |
| P4 | Some chaos and evolve test fixtures hold a kind 15 with no rise for some line, or a boot with no matching findings. They need the missing record added, and no assertion weakened | inferred |
| P5 | The 47 kΩ proposal is byte-identical: the E1 path never builds a `Field`, and `facts` reads the same 400 kHz from `i2c_speeds` | read |
| P6 | **Revised before any measurement** (the advisor's review): a held-low DShot run (`4300a` P12) would name **I3**, never the new rule. The held line faults its bus, the missing boot word then decides `I3` at board scope, and `I3` sorts below any appended rule. Only a read that is late on a *healthy* bus lets the new rule name the run: a firmware-side delay on the DShot board, as a mutant. Predicted for that case: findings `boot_word_missing`, `identity_read_outside_window`, and possibly `register_mismatch` (outputs not yet started); rule = the new one; evidence = the unset kind 12 | inferred |
| P9 | Renaming F-B10's data mismatch moves no seeded case. The reference board's parts (BME280, W25Q64JV, TMP117) have no script, so every mismatch is an identity step's (`ir/data/parts.json`). One tier-2 assertion names a *data* step's mismatch as `{bus}_identity_mismatch`: `twin_tier2.rs:3092-3093`, `4220`'s no-wait test. It moves to the new name | read |
| P10 | An unset record is archived as status `invalid`, with the judge's fill as its value. `status_name` has no arm for the fill (`renode.rs:2734`), and `rearm` fills both words (`:609-622`). So the archive cannot tell a record never written from one the firmware filled with garbage, and no new status word is needed: the rule selects `invalid` records in an unbooted run on a short (DShot) window, with an exact count, which fails closed | read |
| P7 | `cap-390p-sda-2k2` × `leave-firmware` is `I3` with evidence `[6,7] [7,9]`: the healthy SCL line's rise is not evidence. The one-bus M1 mutant (board-scope `I3` plus every rise) names `[5,7]` too, so only this case kills it | inferred from `2205`'s candidate table (`4:7 5:9`, shifted by two since `4300b`) |
| P8 | `TIER2_MIN` is unchanged: the new seeded cases ride the existing seeded tier-2 tests, with no new tier-2 test function | read |

## Fence: where the items live (read; flagged, not a scope change)

- **B1–B3 are not in `replay.rs`.**
  - B1 and B2 live in `chaos/src/campaign.rs`: `check_run` and `check_joins`, the rules `replay` and `attribute` apply after the fold.
  - B3 lives in `chaos/src/record.rs`, in `Record::check` for kind 14.
  - `replay.rs` is the fold and the hash, which this work order must not move. I edit `campaign.rs` and `record.rs` only where these items need it.
- **F-B10 changes `identity_finding` in `renode.rs`.** That is the booted path, outside "only the no-boot-word path", and §1.5 requires it. The change is one expression, the finding's name.
- **Overlaps with `4230` (PR #161), told by inbox:**
  - `check_run` after the self-loop check;
  - `PER_EVENT`'s length in evolve;
  - `propose::facts`: 4230 adds the domain maximum on the same lines where I take the speed from `i2c_speeds`;
  - `judge_boot`: 4230 inserts directly above "The identity reads". My F15 change touches only the `else` arm, through a helper.

# The build

## What changed, item by item

- **B1, replay binds every solved line** (`chaos/src/campaign.rs`, `check_run`).
  - Each line of each kind 15 is bound by exactly one record: its kind 7, its kind 8, or, for a held line, its held-low kind 9.
  - The held-low fault now binds the override's own line: SCL is `lines[0]` and SDA `lines[1]`, both enforced by kind 15's check. Before, any net of the override's bus was accepted.
  - The forgery `2205` found (every rise and the fault dropped, the verdict rewritten to pass) is refused. So is one line's rise dropped, a line bound only by another bus's rises, and a held line measured as well as faulted.
- **B2, replay binds the boot word** (`check_run`).
  - A boot that was read names `boot_word_missing` exactly once, as an error, when its word is not the firmware's, and never otherwise.
  - A boot that read nothing names the bound it was killed at, once, as an error. A read boot names none.
  - The constants `BOOT_WORD_FINDING` and `BOUND_FINDINGS` are in `campaign.rs`. The judge's no-boot-word path uses the first.
- **B3, closed severity** (`chaos/src/record.rs`). `SEVERITIES = ["error", "info"]` is checked exactly in kind 14's `Record::check`, which runs on write and on read. Another case, whitespace, `warning` or an empty string is refused, with no byte of the format changed.
- **The outside-window finding in replay.** `identity_read_outside_window` is refused unless the boot was read without its boot word on a short (DShot) window, and at most once per read archived `invalid`. `campaign::short_window` is the one statement of "a DShot output gives the short window"; `check_motor_joins` uses it too.
- **F15, the judge** (`cli/src/renode.rs`, the `else` arm only). With no boot word, each record whose status says its wait expired is named, as `{bus}_timeout`, by `identity_finding` itself (`timeouts_without_the_boot_word`). A mismatch or the fill is not judged before the fixed point. `boot_word_missing` stays.
- **F-B10, the mismatch kinds.**
  - The judge names a mismatch `{bus}_{step}_mismatch`, where the step is `identity`, `configure` or `data`.
  - Evolve selects each kind by the record's step, through `record::step_index`, so a data read's mismatch named as an identity mismatch selects nothing and is `I1`.
  - Their scope and rules are the identity mismatch's: the device's bus for I2C, the device for SPI.
- **Q7, rule `I7`** (`evolve/src/lib.rs`).
  - **Selection.** `identity_read_outside_window` selects each kind 12 or 21 still at the judge's fill: status `invalid`, and every value word `0xFFFFFFFF`. It does so only in a run whose boot was read without its boot word on a short window. The count must be exact.
  - **Decision.** Such a record has a scope of its own, `Window`, which decides `I7`, indeterminate, with the record as its evidence (kinds 12, 21). Nothing is proposed (`INV-05`).
- **Motor witnesses are judged once the boot word is set** (evolve).
  - Found while building `I7`: evolve selected an unbooted run's `unwitnessed` kind 26 as a failure. But the judge names motor witness findings only after the boot word, and `check_run` binds them that way ("before it, none"). So every unbooted motor run was `I1` by an unexplained event.
  - Fix: the two motor witness kinds and the motor evidence select a witness only when booted, as the serial ones already did.
- **F-B5.** `Field`'s `mask`, `solved` and `observed` are `u64`, and so is `Expectation::mask`. An SPI read of 5 to 8 bytes keeps its whole value, with a 64-bit mask; 32-bit fields are unchanged in value and in text.
- **R2.** `propose` writes through `tree::write`, and `write_out` is gone. `tree::write`'s emptiness message no longer says "generate", since it now serves `propose`, `generate` and `fab`.
- **`1205` review.** `propose::facts` reads the declared speed from the topology's `i2c_speeds`, so the specification is no longer passed to it.

## Measured (tier 2 in the pinned image, `~/bin/capped-run`, 8 GiB, swap 0, read back on every container)

- **Baseline at `8030fa5`** (`~/.cache/aet-3201/t2/base`): `attribute_seeded` 11 passed, 496 s. `attribute` was saved for all 30 archived runs, and every proposal directory kept.
- **This branch at `9e133ba`, before the new cases** (`~/.cache/aet-3201/t2/head1`):
  - `attribute_seeded` 11 passed, 482 s;
  - `campaign_tier2` 9 passed;
  - `twin_tier2` 65 of 66 (the one failure is below).
- **P1 held (measured).** All 30 `attribute` outputs are **byte-identical** to the baseline, with the same `run_id`s and `state_hash`es: domain, rule, evidence, symptoms and masked, for every seeded case.
- **P5 held (measured).** Every `propose` output directory is byte-identical (`diff -r`), the 47 kΩ proposal and both re-gate specifications included.
- **P3 held.** The golden test passes unchanged, and no committed hash moved, because no archive changed.
- **P2 held.** `held-low-sda` and `held-low-scl` replay and attribute as before.
- **P9 held (measured).** `twin_tier2`'s `without_the_cited_wait_each_data_read_answers_its_reset_value` failed on the rename alone: its kinds are now `i2c_data_mismatch` and `spi_data_mismatch`. The test now asserts those, and that neither identity mismatch appears, since both identities read right.
- **P4 held (measured, tier 1).** Fixtures that are not honest runs now break: a solved bus with no rises (chaos `campaign.rs`, `droop.rs` `run_with_check`, evolve's `the_lowest_indeterminate_rule…`), a wrong boot word with no `boot_word_missing` (evolve, CLI `campaign_tier1`'s serial sweep), a killed droop boot without its bound (chaos `droop.rs`), and `warning` (evolve A10). Each now carries what the CLI writes. Tests whose run is now refused assert the refusal. No guard was weakened.
  - **Consequence: `I2`'s first clause is now unreachable.** "A line with no computed rise" is refused by `replay`, so `I2` remains for a bus the header cannot join, or a board with no bus. Evolve's `healthy` check of a rise per line stays, as a second statement, and the README says so.

## Each B item, before and after (measured)

The six new chaos tests, run against `8030fa5`'s code (`~/.cache/aet-3201/before-after-chaos.txt`), all **fail there**: each forged run is accepted. On this branch they are refused.
- `a_run_with_its_rises_dropped_is_refused` (B1);
- `a_line_bound_only_by_another_bus_or_another_line_is_refused` (B1);
- `a_boot_that_read_nothing_names_its_bound` (B2);
- `boot_word_missing_is_named_exactly_when_the_word_is_wrong` (B2);
- `a_severity_outside_error_and_info_is_refused_on_write_and_on_read` (B3: `zzz`, `warning`, `Error`, `INFO`, leading or trailing space, tab, empty; on write and through the reader);
- `identity_read_outside_window_is_bound_to_an_unbooted_short_window`.

In evolve, B2's unnamed missing boot word was `I1` and is now refused (`a_boot_word_missing_unnamed_is_refused`). Q7's run was `I1` (`4300a` P12) and is now `I7`.

## Mutation testing (diff-sized: one mutant per guard this work order adds)

`~/.cache/aet-3201/mut/mut.py` (each mutant an exact-once edit of a scratch copy, its suite run, the edit reverted). **18 of 18 killed** (`result.txt`, `result2.txt`). Four survived the first run, and each now has a test:
- `boot_word_missing` at `info` beside a boot that has its word;
- the replay bound on `identity_read_outside_window` for a *booted* DShot run, a boot word written after the window;
- the I2C mismatch selected by step: an identity mismatch dressed as a data or configure step's;
- `I7`'s conditions, unbooted and on a short window. The first survived because the booted variant's `I4` sorted below the mutant's `I7`, so the test now asserts the exact rule and its rises.

No mutant was run on `propose`'s speed source: the topology's speed and the specification's are one value on every honest input, so a mutant there would be equivalent.

## Adversarial pass, full (a fresh read-only Opus subagent, on a frozen copy of `b7b6a24`)

It forged runs as Rust tests: `~/.cache/aet-3201/adv-src/crates/{aethelgard-evolve,aethelgard-chaos}/tests/adv3201.rs`, 14 + 1 tests, kept for rerun. I spot-checked F1 at source before fixing it.

**The four required attacks HELD (measured).**
- **A kind-15 line bound by another bus's or line's record** is refused: a relabelled bus, another bus's net, a dropped rise, a second rise on the other line, an unmeasured second bus, a held-low fault on the wrong line, a held-low fault naming both nets or another bus's net, and a held line also measured.
- **A severity in another case or with whitespace** is refused on write, and on read through a re-sealed archive: `Error`, ` error`, `error `, `info\n`, `warning`, `ERROR`, `Info`, `error\0`, a fullwidth `ｅ`, `err`, `errors`, `fatal`.
- **A boot word written after the window** is refused with the outside-window finding beside it. So is the finding on a PWM board, on an unread boot, on a timeout status, and at `info`. The honest late DShot run is `I7`. A record not at the fill, a 6-byte record with a 32-bit fill, or the finding omitted, is each `I1`.
- **A data mismatch dressed as an identity mismatch**, and the reverse, configure included, for I2C and for SPI: each is `I1` with the kind and the event unexplained. Named honestly, each is decided.

**Findings, all fixed:**
- **F1, should-fix: a regression of mine, and a wrong §1.1 answer.**
  - §1.1 said a boot that read nothing is only a boot killed at a bound. That is wrong. `judge_boot`'s load-error branch (`renode.rs:1844-1860`) returns an unread boot naming `platform_load_error` or `firmware_load_error`, and `boot_tier1.rs:688-692` measures exactly that shape. So B2 as built refused an honest verdict, and `campaign` would have exited internal on it. No seeded case and no tier-2 run reaches it, which is why my runs did not show it.
  - **Fixed:** `UNREAD_FINDINGS` holds the three bounds and the two load errors. An unread boot names exactly one of them, as an error. The test iterates all five.
- **F2, should-fix: older than this work order, inside the fence.**
  - Rewriting the verdict alone moved a run from `I1` to a confident `S1`. The run is unbooted, its record `ok` with a wrong value, its witness wrong. The judge names nothing of the record, so it is honestly `I1`. Adding `i2c_identity_mismatch` and `i2c_transaction_unwitnessed` made it `S1`, which breaks `3100` A1.
  - **Fixed in replay, not evolve:** `check_run` refuses any of `BOOTED_READ_FINDINGS` (a mismatch of any step, `*_record_invalid`, `*_transaction_unwitnessed`) in a run without its boot word. That is the judge's rule, as the motor and serial gates already state it. A timeout and the outside-window finding stay allowed unbooted.
  - **Why not evolve:** gating evolve's selections instead would turn the honest `I1` into `S2`, a more confident answer.
  - The same fix closes the adversary's 3.6 (an unset record named `i2c_record_invalid` unbooted gave `I7`). Evolve's short-window condition on `I7` is now a second statement of replay's rule, labelled `belt-and-suspenders:`.
- **F3, low:** evolve's `boot_word_missing` selected an unread boot, which replay forbids naming, so every honest killed boot listed its boot as an unexplained event. **Fixed:** it selects a read boot whose word is wrong.

**Re-measured after the fixes** (the adversary's tests against the fixed code, `~/.cache/aet-3201/adv/rerun-after-fix.txt`):
- 3.6 and 7.1 (F2) are refused;
- the unread boots 5.1 (pass, no finding) and 5.10 (a bound twice) are still refused, with the new message;
- the load-error runs (F1) are accepted.

The five assertions that fail there are the adversary's pins of the old behaviour.

**Residual, not fixed (backlog): the adversary's 5.8.** A booted, passing run whose registers and identity records are set to unread (status and value `None`) replays as `N1`. The judge reads all of them or none, as kind 10's "all read or all unread" rule says for the boot. A like rule across kinds 10 to 13, 21 and 22 would close it. That is a new cross-record rule outside the B items, so it is not built here.

## Reduction pass (diff), by a fresh subagent; each proposal checked, applied and re-proved

**Applied.** Each change keeps the suite green, and its guards' mutants are re-run and all killed (`~/.cache/aet-3201/mut/result3.txt`, `result4.txt`).
- **One selection arm for the six mismatch kinds.** The I2C and SPI arms were one condition stated twice. The bus now comes from the record's kind, 12 or 21.
- **One `record_words(width)`** for the value words' bits, used by `unset_record` and by the SPI `read` field's mask (F-B5). It was the same expression twice.
- **`SEVERITIES` moved above a protected comment it had split** (`record.rs`, the `belt-and-suspenders:` note on `SERIAL_FRAME_LEN`).
- **The judge's `identity_read_outside_window` literal** is now `campaign::OUTSIDE_WINDOW_FINDING`.
- **A mutant survived** on the reshaped `mismatch_kind`: no test held a configure step's mismatch. `a_configure_steps_mismatch_is_named_by_its_step` now kills it.

**Protected kept, labelled:**
- evolve's `unset_record`, a second statement of the twin's fill;
- evolve's `late`, a second statement of `check_run`'s window rule;
- the serial frame constants.

**Skipped:** a shared closure for "the solved line's net" in `check_run`, which is marginal and would add a mechanism.

**Findings outside this fence (not changed):**
- The five unread findings are literals in `renode.rs` (`bounded`, `judge_boot`'s load-error branch) beside chaos's `UNREAD_FINDINGS`. These are `1603`'s and `4230`'s hunks. A follow-up can use the constants.
- `"info"` and `"error"` are bare literals across chaos, evolve and the CLI. `SEVERITIES` now closes the set, and a `pub const` per severity would end the scatter.
- `verify.rs` holds `"spi_identity_mismatch"` literals.

Reduction pass (diff): +1 net in source (measured, `git diff --numstat`; rustfmt spreads the merged arm over more lines, so the cut is copies, not lines), plus the 42-line test · 3 duplicates removed · 0 dead paths removed · 3 protected kept · 3 findings above

## The new seeded cases, measured (tier 2 at `1d9c93d`, `~/.cache/aet-3201/t2/head2`)

- **`trap-indeterminate-leave-390p`** (`cap-390p-sda-2k2` × `leave-firmware`):
  - Findings: `analog_rise_exceeds_ceiling`, `i2c_timeout`, `pc_outside_firmware`.
  - Answer: **`I3`**, evidence exactly `[6,7] [7,9]`: the faulted SDA rise and the fault. The healthy SCL rise `[5,7]` is a symptom, never `I3`'s evidence. **P7 held**, index for index.
  - Masked: none. `propose` gives `no_proposal`.
- **`identity-late-dshot`** (`scenarios/motors/dshot600.json` × `identity-late`, on the DShot fixture's board):
  - Findings: `boot_word_missing`, `identity_read_outside_window`, `register_mismatch` (TIM1, never started).
  - Answer: **`I7`**, evidence exactly the two unset records `[49,12] [51,21]`, nothing unexplained.
  - **The revised P6 held:** S2 for the boot word and S4 for the timer's registers decide too, and `I7`, the only indeterminate rule, names the run. Before this work order the run was `I1`, as `4300a` P12 measured.
- **The evidence mutant only the new trap kills (measured).** M1, "board-scope `I3` evidence plus every rise", was built into a scratch copy of evolve and run with `attribute` over the archived `leave-firmware` runs (`~/.cache/aet-3201/m1-mutant.txt`):
  - the 47 kΩ trap gives `[5,7] [6,7] [7,9]`, exactly its manifest, so it cannot see M1;
  - the 390 pF trap gives `[5,7] [6,7] [7,9]` against its manifest's `[6,7] [7,9]`, so clause 1 fails;
  - the 2.2 kΩ `leave-firmware` run stays `S2`.
- Every other case's answer, evidence and masked list are unchanged (clause 1 failed only on the placeholder indices of the new I7 case, before they were filled in from this run).
- `TIER2_MIN` is unchanged: no tier-2 test function was added; both cases ride the seeded block.

## Merges of `main`

- **`a45eb02`** (docs #162), with a merge commit. It is docs only, and the work order's text there is identical to the one I built from.
- **`4419c59`** (4300g #159, 4401 #163), with a merge commit. Neither touches a file of this fence, and there were no conflicts.
  - 4300g's specification 11 changes every specification hash, so every seeded `run_id` and every proposal's hash fields move. The only non-hash differences in the proposals are specification 11's own fields (`schema_version`, `supply_source`). Every proposed value is unchanged (measured, `diff -r` against the baseline).
  - At this head, clause 1 again asserts every case's evidence and masked lists exactly, and passes.
- **`TIER2_MIN` is 157**, `main`'s, unchanged: this work order adds no tier-2 test function.

## Gates (at the merged head `f52effd`)

- **Host:**
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`: 113 suites, 0 failed.
- **Pinned image**, through `~/bin/capped-run` (8 GiB, swap 0 and pids 8192, read back on each of the four containers):
  - `clippy --features aethelgard-cli/with-tools -D warnings`: exit 0;
  - `attribute_seeded`: 11 passed, 488 s;
  - `twin_tier2`: 66 passed;
  - `campaign_tier2`: 9 passed.
- **Earlier in the build** (`1d9c93d`): `board_tier2` 18 passed (1,603 s), `verify_47k_tier2` 2 passed.
- **CI's plan.** On Furnace the seeded block ran 482 s before the two cases and 471 s and 488 s after, so its colocated weight in `ci/colocate.tsv` (726 s) stands. `ci/` is not changed.
- **CI:** recorded below, from the PR's run.

## Questions and findings for the architect

1. **B1 and B2 as built differ from the work order's wording, to keep honest runs** (§1.1). A held line is bound by its held-low fault. A boot that read nothing names why: one of three bounds or two load errors. Ruling wanted only if either should be otherwise.
2. **Fence.**
   - B1, B2 and B3 live in chaos's `campaign.rs` and `record.rs`, not `replay.rs`.
   - F-B10 is one expression in `identity_finding`, the booted path.
   - F2's fix is a rule in `check_run`.
   - All of these are inside what the work order asks for, and named here.
3. **Backlog (the adversary's 5.8, not built).** A booted run whose registers and records are set to unread replays as a pass. A rule of all read or all unread across kinds 10 to 13, 21 and 22, beside kind 10's own, would close it.
4. **Backlog (reduction).** These are bare literals outside this fence:
   - the five unread findings in `renode.rs`;
   - `"info"` and `"error"` across crates;
   - `"spi_identity_mismatch"` in `verify.rs`.
5. **`I2`'s "a line with no computed rise" is now refused by replay** before attribution. The clause stays in evolve as a second statement, so the CLI README's rule table now says so.

## CI (PR #167, run 43850749076, head `d6654e8`)

**Green:** `gates`, `shards`, all eight tools shards, the aggregate `tools` (its `TIER2_MIN` 157 check included), and the `cli`, `image`, `pinned`, `pinout` and four `ardupilot` jobs.

Tools shards, start to end:

| Shard | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| Time | 19:37 | **24:20** | 21:31 | 18:25 | 16:25 | 16:22 | 20:44 | 18:45 |

- Shards 1, 2 and 6 passed 20 minutes. By the eight-shard rule (2026-11-16 04:35) that is a measurement, not a failure.
- **No refit is triggered.** The plan is unchanged, and shard 1 leads shard 2 by 2:49, under the rule's 3 minutes, on one run. Shard 1 was the slowest here; the next run decides whether it is again.
