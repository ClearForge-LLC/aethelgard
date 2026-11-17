# AET-WO-2302b FEEDBACK — attribution for the twin's new findings, the dead window code removed

Branch `wo/aet-2302b-attribution`, cut from `main` at `ecc6be9` (#175, `AET-WO-2302`, merged). The work order is on the docs branch (`claude/argon-repo-docs-synthesis-rjrb0d`, docs PR #198) until it merges; it is the source of truth. Every claim is labelled **measured**, **read at source** or **inferred**.

## §1 The classification, before any rule (WO §1.1)

Each of the ten kinds `AET-WO-2302` added is classed by what produces it in the twin, with the record that decides it.
- **Electrical** is the board, a part or the specification's declaration.
- **Software** is the firmware's own behaviour: its register writes and its timing.
- **Harness** is the judge's or the run's own bound, never a fault of the device under test.

Read at source for every row: `cli/src/renode.rs` (where the judge emits each kind), `cli/src/anchor.rs` (the hooks and the judge's actions), `chaos/src/record.rs` (kinds 10, 26, 29, 30) and `chaos/src/campaign.rs` (what replay binds).

| Kind | Class | What produces it in the twin | Evidence record that decides it | Where the records cannot decide |
|---|---|---|---|---|
| `motor_anchor_unobserved` | **software** | Edges on a motor pin, with no `CEN` rise by the CPU in the boot: the firmware toggled the pin itself (GPIO, `BSRR`), or started the timer in a way the CPU's stores do not show (DMA). Every CPU store width is hooked (word, half-word, byte, and `strd`/`stm`, measured in `2302`'s pass 2). No board fault drives a motor pin in the twin. | Kind 26: `cen_rises == 0`, `anchor_ns` absent, `rises + falls > 0`; and the verdict (kind 14). | None. |
| `motor_anchor_repeated` | **software** | A second `CEN` rise in one boot: the firmware wrote `CEN` again, by restart or clear-and-set. | Kind 26: `cen_rises >= 2`, which replay binds. | None. |
| `motor_anchor_out_of_order` | **software** when booted | The firmware's own code order: `CEN` before its last device step's witness, or after its boot word. | Kind 26: `anchor_ns`; kind 10: the boot word set; the verdict. | The device steps' instants are not in the trace, only the verdict says the anchor preceded one. When **no boot word** was set, the order "after the boot word" cannot be judged, and a board fault (a droop's reset) can also cost the boot word. Then it is **unexplained by name**, missing the boot word (kind 10), never guessed. |
| `motor_window_past_run` | **harness** | The anchor plus `DSHOT_WINDOW_NS` passes the run's end, `MAX_VIRTUAL_NS`. A firmware that starts its outputs at 99 ms is not wrong on silicon; the twin's run is too short to witness it. | Kind 26: `anchor_ns + DSHOT_WINDOW_NS > ` kind 10's run, the verdict. | The bound to name: `MAX_VIRTUAL_NS`, `DSHOT_WINDOW_NS`. |
| `motor_edge_before_anchor` | **software** | Edges before the timer's anchor in its boot: the firmware drove the pin before `start_outputs`. | Kind 26: `edges_before > 0`, which replay binds. | None. |
| `motor_edge_after_window` | **software** when the stop ran | Edges after the judge's stop cleared `CEN`: the firmware restarted the timer or drove the pin after the window. | Kind 26: `edges_after > 0`; kind 30: the stop present, with `cr1_after` showing `CEN` clear. | If the stop did not run or left `CEN` set (`judge_stop_missing`), the edges may be the timer still running. Then the edges are the harness's (below), not the firmware's. |
| `judge_stop_missing` | **harness** | The judge's own act did not run in the boot it reads, or left `CEN` set. Its cause is the judge's action (stale after a core lag, or not scheduled), never the device. | Kind 30 absent for an anchored DShot timer, or `cr1_after` with `CEN` set; the verdict. | The bound to name: the judge's stop at anchor + `DSHOT_WINDOW_NS`. |
| `judge_stop_mismatch` | **software** (a call against the work order's example: **for the architect**) | The judge's read-back of the timer's solved registers at the window's end differs from the solved values. The judge only reads them. The firmware wrote the timer inside its window: a `CCR`, `ARR` or `PSC` changed. It is `register_mismatch`'s evidence at another instant, which the software rules already own. | Kind 30: `read_back` entries off kind 11's expected values (bound by replay since `2302` A2-1). | None. The work order lists `judge_stop_*` among the harness examples. By its evidence, only `judge_stop_missing` is harness: a mismatch is the device's register state. I class it software and flag it rather than follow the example against the record. |
| `serial_wire_past_run` | **harness** | The summed wire time of the frames and echoes, from the boot word, passes the run's end. That is the twin's run bound, not a fault: the same firmware with an earlier boot word passes (measured in `2302` A2-3). | Kind 29: `injected_ns`; kind 27: the framing; the verdict (bound by replay since A2-5). | The bound to name: `MAX_VIRTUAL_NS` against the wire time. |
| `serial_injection_out_of_order` | **harness** | The judge's own injection chain went in out of order. The injections are the judge's act, and `2302`'s pass 2 read it as effectively unreachable (defence in depth). | Kind 29: `injected_ns` across the ports; the verdict. | The bound to name: the judge's sequential injection (`AET-WO-2302` D4). |

**Consequences for the rules (WO §1.2):**
- One software rule for each software kind: six. `motor_anchor_out_of_order` is attributed only when booted, and `motor_edge_after_window` only when the stop ran and cleared `CEN`.
- An explicit refusal for the four harness kinds, naming each bound. It never attributes them to either half.
- Where the records cannot decide, the kind is reported unexplained by name, with its missing evidence (`INV-06`), never plain `I1`.
- No kind is electrical. In the twin, nothing a board declares produces any of the ten. A board fault reaches them only through a lost boot word, which the rules above refuse to read as either half.
- **§7's stop does not fire.** Each kind's evidence is in the trace or the verdict.

## §2 The build (`a589cd0d`; one worker, merged after a spot-check at source)

**The software rules** (`aethelgard-evolve`; each has a firing and a non-firing test, measured passing):

| Rule | Kind | Evidence kinds | Subjects | Fires | Must not fire |
|---|---|---|---|---|---|
| `S6` | `motor_anchor_unobserved` | 26 | the output | `s6_edges_with_no_anchor_are_software_by_the_witness` | `s6_never_fires_on_an_output_with_no_edges` (that run is `I4`) |
| `S7` | `motor_anchor_repeated` (per timer, `cen_rises` > 1) | 26 | the output | `s7_a_timer_started_twice_is_software_by_its_own_witness_alone` | the same test: the other timer is never cited |
| `S8` | `motor_anchor_out_of_order` | 10, 26, 29 | the output, the boot, the port | `s8_an_anchor_after_the_boot_word_s_instant_cites_that_instant` | `s8_never_fires_on_a_rewritten_verdict_alone` |
| `S9` | `motor_edge_before_anchor` | 26 | the output | `s9_edges_before_the_anchor_are_software_by_the_witness` | inside the S6/S10 non-firing runs (no edge before) |
| `S10` | `motor_edge_after_window` (kind 30 at the anchor has cleared `CEN`) | 26, 30 | the output, the timer | `s10_edges_after_the_window_are_software_only_once_the_stop_cleared_cen` | that test's stop-left-set half, and `s10_never_fires_with_no_stop_to_cite` |
| `S11` | `judge_stop_mismatch` (a read-back off its solved value) | 30 | the timer | `s11_a_register_read_back_off_its_value_is_software_by_the_stop` | its unread-register half (unexplained by name) |

**The harness refusal:** four rules, one per harness kind, each in domain `indeterminate`. Each names its bound through `Rule::bound()`. A harness rule names the run only when no other rule decided a symptom. Its events never join another rule's evidence, and `I5` never counts them.

| Rule | Kind | The bound it names | Evidence kinds |
|---|---|---|---|
| `H1` | `motor_window_past_run` | `DSHOT_WINDOW_NS` past `MAX_VIRTUAL_NS` | 24, 26 |
| `H2` | `judge_stop_missing` | the judge's stop at anchor + `DSHOT_WINDOW_NS` | 24, 26, and 30 when present |
| `H3` | `serial_wire_past_run` | `MAX_VIRTUAL_NS` against the wire time | 27, 29 |
| `H4` | `serial_injection_out_of_order` | the judge's sequential injection | 29 |

- `propose` answers an `H` rule with nothing proposed and its bound named. The tests: `h1_*`, `h2_*`, `h1_h2_select_each_dshot_timer_by_its_own_window_and_stop`, `h3_*` (including `h3_never_masks_a_serial_fault`, beside `S5` and `E3`), `h4_*`, `a_harness_finding_never_masks_a_fault_nor_joins_its_evidence` (beside `S4`, `E1`, `I4`), and `propose_answers_a_harness_rule_with_its_bound_and_nothing_else`.
- **Precedence is unchanged** (`a_window_software_rule_keeps_the_existing_precedence`): `S4` beats the new `S` rules, a new `S` beside `E1` is `I5`, and any indeterminate rule wins.

**Where the build departs from §1's table (each accepted by me; §1 is left as written):**
1. **`S8` is narrower than "software when booted."** Built as first written, it fired on the verdict alone: the records were those of a passing run, so rewriting the verdict moved a run to software. That is the README's rule ("rewriting the verdict never moves a domain") and WO §5's target.
   - It now decides only when kind 29 places the anchor after the boot word's instant (the first port's injection, less a quantum), for exactly the timers the verdict names.
   - An anchor before the last device step's witness is the boot's log-line order, which no record carries. It is **unexplained by name**, with that missing evidence.
   - The unbooted branch is unreachable: replay refuses every window kind before the boot word. It carries a `belt-and-suspenders:` comment rather than a branch.
2. **Two existing kinds now go to a harness rule when, and only when, the bound cut them.**
   - `motor_timing_mismatch` (`I4` before) goes to `H1` when the DShot witness is `stopped`, its window passes the run, its last rise reaches the run's end, and no register is wrong.
   - `serial_echo_missing` (`I6` before) goes to `H3` for a port in the past-run wire sum. The echo must be `missing` or `partial`, never `not_counted`, which the worker's own mutation run caught as a firmware fault refused as harness. Framing and tags must hold, with no `S5` or `E3` evidence.
   - Without this, `H1` and `H3` could never name a real run: their findings always arrive with these companion symptoms.
3. **`motor_edge_after_window` with no stop that cleared `CEN` is unexplained by name, not harness.** The edges may be the timer still running or the firmware, and no record tells which.
   - So `judge_stop_missing` with edges after the window is `I1` by name, never `H2`.
   - §1's table said "the harness's". The narrower reading is the honest one (`INV-06`).
- **The "unexplained by name" shape is rule `I1`** with an `unexplained` entry written `"{kind}: {what the records lack}"`. Like any `I1`, it outranks a real fault beside it. It is never a plain `I1`.
- **`H4` caveat:** replay binds no count for `serial_injection_out_of_order`, so a verdict naming it in a run nothing else decides makes the run `H4` (indeterminate), never a domain. The README says so.
- **`judge_stop_mismatch` on a register the stop did not read back** is unexplained by name.

**The removals:**
- **evolve:** `Rule::I7`, `Scope::Window`, `Run.late`, `outside_window`, `unset_record`, and the `I7` entries in the selection and `PER_EVENT`. Rules now run `I1`–`I6`, 25 in all, and the count test asserts that "I7" names no rule.
- **evolve tests removed:** `late_dshot`, `a_read_still_unset_when_the_short_window_ends_is_i7_by_its_record` and `a_late_read_is_only_the_unset_record_of_an_unbooted_short_window`.
  - `an_unset_record_on_the_long_window_is_never_i7` tests another replay refusal, so it is kept, renamed `an_unset_record_claimed_invalid_before_the_boot_word_is_refused`.
- **chaos:** `short_window`, `OUTSIDE_WINDOW_FINDING`, `check_run`'s bound on the kind, and the test `identity_read_outside_window_is_bound_to_an_unbooted_short_window`. A verdict naming `identity_read_outside_window` is now unbound in replay and gives a plain `I1`: no boot produces it.
- **Still naming it, outside my allowed edits:** `crates/aethelgard-cli/README.md` L346, a historical "no longer given" note, and the protected `docs/roadmap.md` L626, which is the architect's.

**The `.is_err()` tightening:** 87 occurrences now assert the refusal's message or variant (motor 34, serial 35, droop 11, campaign 4, archive 1, `power.rs` 2; two were helper or macro bodies covering 22 call sites). **Four passed on another guard's refusal**, and are fixed:
- `a_static_mismatch_is_named_once_per_failing_check_booted_or_not` was refused by the boot word;
- `a_run_with_its_rises_dropped_is_refused` was refused by the bus fault;
- `a_boot_that_read_nothing_names_its_bound`: the pass forgery is now caught first by B1;
- archive.rs's "u64::MAX timestamp" case was a payload-length refusal, not a codec one.

**Outside the strict fence:**
- the CLI README's attribute section and `propose` table only (1604 and 4400b2 were told by inbox);
- the chaos README's replay list;
- chaos's `open_ports`, `boot_word_instant`, `wire_past_run_ports`, `TWIN_MAX_VIRTUAL_NS` and `TWIN_DSHOT_WINDOW_NS` made public for evolve.

No CLI source file changed.

**Setup note:** the old `~/aethelgard-2302` worktree (clean, level with `origin/wo/aet-2302-windows` at `4b9cfe31`) was to be torn down at the start. The auto-mode classifier denied the delete, so it is left for the operator: `podman unshare rm -rf ~/aethelgard-2302 && git -C ~/aethelgard-2302b worktree prune`.

## §3 The adversarial pass (targeted, WO §5; a fresh Opus on `a589cd0d`)

Every attack was built from records that replay (`check_run`) accepts, written and read back through the archive. The reproductions are copies of evolve's `motor.rs` and `serial.rs` with the attacks appended, kept in `~/.cache/aet-2302b/adv_{motor,serial}.rs`.

**WO §5's targets:**
1. **A harness kind attributed to the board or the firmware.**
   - **Direct: none.** Keeping the `H` rules apart puts no `H` rule in the `E` or `S` domain (measured).
   - **By masking:** two forged-record cases, ADV-S1 and ADV-M1, where `H3` or `H1` absorbed a symptom the cut did not cause. That let a software rule elsewhere name the run instead of `I6` or `I4`. Fixed (§4).
   - **One real-run candidate:** ADV-M3, ruled below.
2. **A rule firing on absent evidence.**
   - Every `S6`–`S11` cites its own kind 26 or 30 record.
   - `S8` on a rewritten verdict alone is `I1` by name.
   - `H1`/`H3` on symptoms the cut did not cause: ADV-M1 and ADV-S1, fixed.
   - `I5`'s evidence-kind contract was broken by kind 30: ADV-M2, fixed.
3. **A removed constant still named.** Clean in code, the CLI source, verdicts, the `propose` table and the rule list: `Rule::from_id("I7")` is `None`, with 25 rules.
   - Text mentions remain only in the CLI README L346 (historical) and `docs/roadmap.md` L626 (protected, for the architect).
   - A stale CI weight row for a test `2302` removed is deleted here.

| id | sev | finding | disposition |
|---|---|---|---|
| ADV-M3 | med-high, **a ruling I took, for the architect** | A firmware that sets `CEN` again after the judge's stop is harmless on silicon, where `CEN` was still set. In the twin the judge's clear makes it a second rise, edges after the window, and an end read of `CR1` with `CEN` set against kind 11's expectation of the judge's own cleared value. Attribute said `S4` (on `CR1`) or `S7`: software, for an artifact the judge's stop created. | **The records cannot tell a harmless re-enable from a harmful restart once the judge has cleared `CEN` and `CR1` reads `CEN` set at the end.** `S7` and `S10` then report the kind unexplained by name, the missing evidence being when each rise fell. `S4` never cites the `CEN` bit of a timer the judge stopped. With `CEN` clear at the end and edges after the window, the firmware drove the pin itself, and `S10` stands. This follows the D2 residual (`AET-WO-2302`) and `INV-06`. |
| ADV-S1 | med (forged records) | `H3` took a port whose RX pin was unrouted at the injection: its echo was missing because of the pin, not the bound. | **Fixed:** `H3`'s cut excludes a port whose `rx_at_injection` is off its alternate function. |
| ADV-M1 | med-low (replay accepts; the judge cannot emit it) | Replay accepted a `stopped` witness that was not evenly spaced (10 rises where about 2 187 were due). `H1`'s cut read only the last rise. | **Fixed:** replay requires a `stopped` witness to be spaced and alternating as the judge's `witness_motor` does, and `H1` requires a continuous witness. |
| ADV-M2 | low-med | `I5` cited kind 30, which `Rule::I5.evidence_kinds()` did not allow. | **Fixed:** kind 30 added. |
| ADV-M4 | low (README) | "A harness rule's events never join another rule's evidence" was literally false: `S9` cites the same kind 26 that `H1` does. | **Fixed:** they join only when the record is that rule's own evidence, never because of the harness. |
| ADV-S4 | low (forged) | A first injection less than a quantum after 0 made the boot word's instant unknown, so the run's `I1` text ("the first port took no frame") was false. | **Fixed:** replay refuses it. |
| INFO-1 | info | `ci/test-weights.tsv` still weighed `renode::tests::a_read_left_unset_by_a_short_window_is_named`, removed by `2302`. | Row deleted (the only `ci/` change). |

**Closed, measured:**
- `serial_injection_out_of_order` with count 0 is refused by the archive.
- A rewritten `H4` only moves `N1` to indeterminate, never to a domain.
- `propose` on `H1`/`H3`/`H4` gives nothing, naming the bound.
- A mixed PWM-and-DShot timer is refused upstream by the IR (`TimerShared`).
- Every windowed rule's count equals replay's counter for every run replay accepts, so no plain `I1` exists for the ten kinds.
- Precedence for existing runs is unchanged, with one intended exception: a DShot `stopped` witness cut at the run's end was `I4` and is now `H1`.
