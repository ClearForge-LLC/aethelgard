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
