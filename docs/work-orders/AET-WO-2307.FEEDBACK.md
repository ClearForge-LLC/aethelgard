# AET-WO-2307 FEEDBACK — the acknowledgement refusal hands a person the entries to fill

Builder: Claude (Opus 5.5), a fresh `2307` session, 2026-11-20. Worktree `~/aethelgard-2307`, branch `wo/aet-2307-ack-entries` from `origin/main` `adb8f61a`. The work order is read from `~/.cache/aet-2307-wo.md`, the architect's copy (it is not on `main` yet).

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 1. The measurement (§1.1), predictions first

Predictions committed before any run: `spike/2307/PREDICTIONS.md`, `939593d3`. Harness: `spike/2307/measure.sh` (the CLI's refusal, `--fab-out` into scratch) and `spike/2307/probe/` (a scratch crate that derives the entries from `ledger::build` and `ledger::acknowledge` alone, by its own grouping, not the refusal's code). Outputs in `spike/2307/out/before/` (the base CLI, `adb8f61a`) and `spike/2307/out/after/` (this branch). No container: the refusal precedes every gate.

**Sentinel-03** (`crates/aethelgard-cli/tests/fixtures/sentinel-03.json`, its 11 scenarios) [measured]:

| | predicted | measured |
|---|---|---|
| today's refusal | exit 2, `UnwitnessedFieldNotAcknowledged`, `RCC.AHB4ENR.GPIOCEN`, 280 rows, 35 registers | **exit 2, same kind and id, 280 rows, 35 registers**: `2304b`'s figures hold, so no §7 stop |
| (register, class) pairs | 40 (35–45) | **43**: 29 `same-source`, 14 `ignored-by-model`. Eight registers carry two classes (`TIM1.CCER`, `CCMR1`, `CCMR2`, `CR1`, `USART2.CR1`, `CR2`, `USART3.CR1`, `CR2`) |
| completing a held entry | about 20 | **6** (`GPIOA.AFRL`, `GPIOA.MODER`, `GPIOA.OSPEEDR`, `GPIOA.PUPDR`, `GPIOB.MODER`, `GPIOB.OSPEEDR`, all `same-source`): **missed**; 37 are new |
| fields | 280 missing, plus the held fields | 280 missing (no row repeats a register, field and class); **313** in the 43 entries |
| wall | under 5 s | 4.6–5.8 s |

**Board M** (`4600`'s motor driver) [measured]:
- `spike/4600/driver.json` is schema 16 and is refused today as `UnsupportedSchemaVersion` [measured with `validate`]. M is held at schema 22 as `crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json` (project `bench-motor-driver`). Later work orders kept it current. It holds **44 entries / 254 fields**, against `4600`'s 41 / 250 at schema 16. That file is what was measured.
- With its 44 entries removed (scratch copy, beside `4600`'s three driver scenarios re-based on it): exit 2, `UnwitnessedFieldNotAcknowledged`, `SCB.CPACR.CP10`, **254 rows, 34 registers, 44 pairs, all new**.
- **The committed 44 entries, each `reason` emptied, equal the probe's entries field for field**, and after the build the refusal's `entries` equal both. The committed entries' `fields` were sorted before the comparison (the committed order is the author's).
- First run of M was refused `CampaignCoverageShort` (my harness gave only the droop scenario); `4600`'s three scenarios cover it.

**After the build** [measured, `spike/2307/out/after/compare.txt`]: on both boards, `error.entries` equals the probe's entries exactly.

## 2. What is built

- **`ledger::missing_entries(ledger, spec) -> Vec<UnwitnessedAcknowledgement>`** (`crates/aethelgard-cli/src/ledger.rs`). It groups the rows `acknowledge` left unacknowledged by register and class, sorted by register, then class. Each entry's fields are sorted and appear once. With no held entry, `reason` is `""`. With a held entry for that register and class, the result is that entry completed: the missing fields are added and its `reason` is kept. It returns the IR's own type, so the shape is `acknowledge_unwitnessed`'s by construction.
- **The refusal** (`crates/aethelgard-cli/src/verify.rs`):
  - `RefusalBody` gains `entries`, skipped when absent. Only this kind sets it.
  - The message is the old message with the rule appended: the count of entries, that each replaces any held entry for its register and class, that a held entry keeps its reason (which then covers the added fields), and the reason rule. The rule reads "1 to 200 bytes of printable ASCII with no space at either end", with 200 taken from `aethelgard_ir::board::MAX_SOURCE`, and says that `validate` refuses an empty reason (`InvalidSource`).
  - The row ids stay in the message: the README says it "names every one", and two tests read them.
- **How `validate` and the stale check treat the result** (§0's ask) [measured, §4]:
  - A new entry pasted with `""` is refused (`InvalidSource`).
  - With its reason written, the stale check passes it: every field is a ledger row of that register and class. That holds because `stale` runs before `acknowledge` in `verify` (`verify.rs`, the `stale` call precedes `acknowledge`), so a held entry's fields are already ledger rows.
  - A completed entry pasted in place of the held one passes. Pasted beside it, it is refused as `UnwitnessedAcknowledgedTwice`.
- **`crates/aethelgard-cli/README.md`**, the fab-flag row: `error.entries` described, with these rules.
  - The row's stale "164 of its rows in 27 entries" is corrected to **168 in 30**, which `ledger_tier1::the_reference_acknowledges_every_unwitnessed_row_by_register_and_class` pins. It sits in the sentence this work order edits.
- **Tests:**
  - `ledger_tier1.rs` (4 new):
    - `with_no_acknowledgement_each_register_and_class_gets_one_unwritten_entry`: the reference with no entry equals an independent grouping of the ledger, and equals its own 30 hand-written entries with each reason emptied. It is stable and sorted, every reason is `""`, and 168 fields.
    - `pasted_entries_pass_once_written_and_are_refused_unwritten`: pasted with `""`, `validate` gives `InvalidSource` at `acknowledge_unwitnessed[0].reason`. With each reason a test string: `validate` ok, `stale` none, `acknowledge` empty.
    - `a_partial_entry_is_reported_completed_with_its_reason_kept`: `GPIOA.MODER` comes back completed with its reason; `GPIOB.OTYPER` comes back new. Pasted beside the held entry: `UnwitnessedAcknowledgedTwice`. In place of it: passes.
    - `sentinel_03_s_entries_never_carry_the_ledger_s_judge_text`: 280 / 35 / 43 / 6 and 313 fields. No reason equals any row's `judge`. Every field is a ledger row of its register and class, and either missing or the held entry's own.
  - `fab_tier1.rs::an_unacknowledged_unwitnessed_row_is_refused_by_each_fab_flag_naming_every_one`: under both flags, `error.entries` is exactly the two entries, and the message ends with the rule. Its id check now accepts the last id followed by `. ` (the message no longer ends with it).
  - `battery_sag_tier1.rs::sentinel_03_…`: 43 entries, 37 with `""`.

## 3. Acceptance (§2)

- **The measurement:** §1. Sentinel-03 needs **43** (register, class) entries.
- **Every other refusal's bytes are unchanged** [measured, `spike/2307/differential.sh` → `spike/2307/out/differential.txt`]:
  - The base CLI (`adb8f61a`) and this branch's CLI ran on 16 refusing inputs: `verify --fab-out` on `UnsupportedSchemaVersion`, `InvalidJson`, `SchemaViolation`, `UnwitnessedAcknowledgementStale`, `InvalidSource`, `UnwitnessedAcknowledgedTwice`, `ScenarioBaseMismatch`, `BaseOutsideRoot` and `CampaignCoverageShort`; `validate` on five of those; two usage errors.
  - **All 16 are byte-identical, stdout and exit.**
  - The control, `UnwitnessedFieldNotAcknowledged`, differs, as it must.
  - Every refusal test in the workspace ran in the gate run (§6).
- **Gates and CI:** §6. **`TIER2_MIN` does not move:** every new or changed test is tier 1, and no tier-2 test was added or removed. Sentinel-03's refusal test is tier 1 (`battery_sag_tier1`).
  - No weight row was added. The new tests take seconds, and an unweighted test gets `DEFAULT_WEIGHT` (`ci/shard.sh`); CI refuses only an unweighted test past 60 s.

## 4. The adversarial pass, targeted (§5)

`spike/2307/adversarial.sh` → `spike/2307/out/adversarial.txt` [measured]. Sentinel-03's own `error.entries`, pasted into its specification in each way below, then run through `validate` (the IR's shape and the CLI's stale check):

| # | Probe | Result |
|---|---|---|
| A1 | As printed, every new reason `""` | exit 2 `InvalidSource` at `acknowledge_unwitnessed[28].reason`, the first pasted new entry |
| A2 | Reason one space, a tab, `" written "` (padded), 201 bytes, `"§ reason"` | each exit 2 `InvalidSource`, same path |
| A3 | `reason` key removed (a "default") | exit 2 `SchemaViolation`, `missing field reason`: no default exists |
| A4 | A completed entry pasted beside the held one | exit 2 `UnwitnessedAcknowledgedTwice` (`GPIOA.AFRL`) |
| A5 | Every reason written, each its own | exit 0 |
| A6 | **Every reason the same string `"x"`** | **exit 0**: `validate` accepts one reason copied over all 37 entries. F2 |
| A7 | A copy of a ledger row's `judge` | The refusal prints no `judge`, and none of the 43 entries' reasons equals any row's `judge` (test). A person who copies `judge` from `verify`'s JSON ledger would pass `validate`, which cannot tell where a string came from. That is the IR's rule, outside this fence, and the same residual as A6 |
| A8 | **A completed entry, its kept reason covering fields nobody wrote for** | **passes by construction, as ruled.** On Sentinel-03 the six kept reasons cover **20 added fields**. The kept reason can be wrong for them: `GPIOA.AFRL`'s reason names "PA5-PA7 at AF5 for SPI1", and the entry now also covers `AFSEL2`, `AFSEL3` (PA2/PA3). `GPIOB.MODER` and `GPIOB.OSPEEDR` each keep a reason written for 2 fields that now covers 8. F1 |
| A9 | An entry covering more fields than the rows that need it | None: every field of all 43 Sentinel-03 entries is a ledger row of that register and class, and is either a missing row or the held entry's own (test, and `covers_only_ledger_rows` in three tests). A completed entry's held fields are ledger rows because `stale` refuses first (`verify.rs`, the `stale` call precedes `acknowledge`) |
| A10 | The probe's own binary | The first run of this script used `target/debug/aethelgard` as `mutants.sh` had left it, built with the copy-judge mutant. The probe's last line caught it ("non-empty reasons 43, each the held entry's: False"). `mutants.sh` now rebuilds after restoring the source, and the runs above are on the restored build |

The refusal on Sentinel-03 is 12,443 bytes.

## 5. Mutants (§1.3)

`spike/2307/mutants.sh` → `spike/2307/out/mutants.txt` [measured]. Each mutant is one edit to `missing_entries`, run against `ledger_tier1`, `fab_tier1` and `battery_sag_tier1`. The source is restored from a copy and checked with `cmp`.

| Mutant | Result | Killed by |
|---|---|---|
| drop a field from an entry (`fields…skip(1)`) | **killed** | 5 tests, among them `with_no_acknowledgement_…`, `a_partial_entry_…`, `sentinel_03_s_entries_…`, `fab_tier1::an_unacknowledged_…` |
| merge two classes into one entry (`silicon-only` keyed as `same-source`) | **killed** | `with_no_acknowledgement_…`, `pasted_entries_pass_…` |
| copy `judge` into a new entry's `reason` | **killed** | 6 tests, among them `sentinel_03_s_entries_never_carry_the_ledger_s_judge_text` and `battery_sag_tier1::sentinel_03_…` |

Run twice: before the reduction pass, and after it changed `ledger_tier1`.

## 6. Gates

GATES-FINAL

- **Reduction pass (diff):** −12 lines net in tests · 1 duplicate removed (my `every_pair_unwritten` grouping re-derived what `every_row_acknowledged` in the same file already does; the test now sorts and empties that helper's output) · 0 dead paths · 0 protected kept · 1 finding (F4). Done inline: the production diff is about 50 lines. Mutants re-run after it, all killed.

## 7. Findings and decisions

- **F1 · decision (the architect's) · a completed entry passes with no reason written for its added fields.**
  - What: the work order rules that an entry already held for a register and class is reported completed, its `reason` kept. Pasted in place, it passes `validate`, the stale check and the acknowledgement step with no person writing anything for the added fields. That is §5's first path, built as ruled.
  - Measured on Sentinel-03 (A8): 6 entries, 20 added fields. At least one kept reason is wrong for its new fields (`GPIOA.AFRL`: "PA5-PA7" now also covers PA2/PA3).
  - The message and README say the kept reason "then covers the added fields too", so a reader is told. Nothing forces a re-read.
  - Options, none built:
    - (a) keep as ruled;
    - (b) report a held entry's missing fields as the held entry with its `reason` emptied, so the person re-writes it for the whole set. This is the same shape, and one line in `missing_entries`;
    - (c) (b) plus naming the added fields in the message.
  - Lean **(b)**: it closes the only path in this refusal by which a field is acknowledged unread, at the cost of re-typing 6 reasons on Sentinel-03.
- **F2 · finding, outside the fence (IR `validate`) · one reason copied over many entries passes.** `validate` accepts the same string as every entry's reason (A6), and a copy of `judge` (A7). The refusal never supplies either. Closing it is an IR rule, for example distinct reasons per entry, as the reference already has (its 30 are distinct, pinned by `the_reference_acknowledges_every_unwitnessed_row_by_register_and_class`). Decision for the architect, or a backlog row.
- **F3 · disclosure · M is measured on its held schema-22 fixture.** `spike/4600/driver.json` (schema 16) no longer loads. The 4600 M as kept current is `driver-judges-nothing.json` (44 / 254, not 41 / 250). The committed entries' field order was sorted before the comparison: the refusal sorts, a hand-written entry need not.
- **F4 · reduction finding, outside the fence · the reason rule's wording has a second home.** The message spells the rule ("1 to 200 bytes of printable ASCII with no space at either end"), with the 200 from `MAX_SOURCE`. `board::source`'s `InvalidSource` message spells it in the IR. Two texts for one rule. A shared `const` description in `aethelgard-ir` would make it one, but that crate is outside this fence.
- **F5 · prediction missed · completed entries.** Predicted about 20, measured 6. Pairs were predicted 40 (35–45) and measured 43, inside the range.
- **F6 · the README's reference count was stale** (164 rows / 27 entries; the pinned figures are 168 / 30). Corrected in the row this work order edits.
