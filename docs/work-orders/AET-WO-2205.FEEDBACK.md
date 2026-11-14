# AET-WO-2205 — FEEDBACK

**Branch:** `wo/aet-2205-archive-hygiene` · **Base:** `a5be795` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build, with one stop (§1.6) before the seeded set's exact-evidence code.

**Where everything runs.** Every Renode and tier-2 step runs in the pinned tools image (`container/IMAGE`, `…@sha256:c69ff9bc…`) under rootless podman with `--memory X --memory-swap X --pids-limit -1`. After start, `0` is written into the container's `memory.swap.max`, and `memory.max`, `memory.swap.max` and `pids.max` are read back before the command inside starts (`~/.cache/aet-2205/caps.log`, the runner `~/.cache/aet-2205/img.sh`, 2204's). `TMPDIR` is unset inside the container, as in CI. One exception, named: the first `renode --version` probe (below) ran in the image with `--memory 4g --memory-swap 4g` and no swap write or read-back. It was a 1-second version print.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## Status: STOPPED at §1.6

Items 1, 2, 3, 4, 5, 7, 8 and 9 are built and measured. Item 6 is not: §1.6 stops before its code. The evidence table and the indeterminate proposal are below, with decisions D1 to D4. Nothing after them is built until the ruling.

## Item 4 first: no status-to-phase gating survives `2204` (`3100` F2 closed)

- **What was searched.** Every non-test source in `crates/` for the transfer statuses (`timeout_tc`, `timeout_rxne`, `timeout_stopf`, `timeout_txis`) and for any phase table.
  - Two places name them. `cli/src/renode.rs` `status_name` maps the firmware's status word to its name for the verdict. `cli/src/verdict.rs` documents those names. Neither gates evidence.
  - The chaos run rules (`chaos/src/campaign.rs`) name no status.
- **What `2204` removed.** `79bdfe1` (PR #111) deleted `READ_PHASE`, `WROTE_TXDR`, `NBYTES_BITS` and the `CR2`/`TXDR` fields they gated from `aethelgard-evolve` (`git show 79bdfe1 -- crates/aethelgard-evolve/src/lib.rs`, the removed lines 108, 164–168, 200–201, 280–282, 357). The `CR2`/`TXDR` evidence went with them (2204 D7).
- **What remains is a claim check, not phase gating.** evolve reads `status == "ok"` in two places:
  - `witness_fields`: a record that claims a completed read while the model logged nothing is evidence;
  - the `i2c_transaction_unwitnessed` selector: the firmware's claim, not a phase.
- **The test that shows a TXIS timeout alone is still never software:** `crates/aethelgard-evolve/tests/attribute.rs` `a_txis_timeout_with_nothing_logged_is_indeterminate_not_software` (passing at this branch).
- **Recommendation:** close `3100` F2 with `79bdfe1` and that test. No code.

## What was built

| Item | What changed | Where |
|---|---|---|
| 1 | The header carries `tools`: `[name, version]` pairs, sorted, each 1 to 256 printable ASCII bytes. Replay reports them as `tools`, never compared with the replaying host's. | `chaos/src/event.rs` (`Header::tools`), `cli/src/campaign.rs` (header, `replay`'s `tools`) |
| 1 | `aethelgard`: `env!("CARGO_PKG_VERSION")`. `renode`: the `build:` line of `renode --version`, which a boot now requires (`1.17.0+20260907gitf1dd1b4af` in the pinned image, measured). `aethelgard-bridge`: the identity line's `source_sha256` as the bridge printed it. `ngspice`: the `ngspice` field of the bridge's answers. | `cli/src/main.rs` (`simulate`), `cli/src/renode.rs` (`build`), `cli/src/analog.rs` (`identified`, `answers`, `rail`) |
| 1 | The run rules require exactly the tools that ran: `aethelgard` and `renode` always, `aethelgard-bridge` when kinds 7, 8 or 16 exist, and `ngspice` when kinds 7, 8 or 17 exist. | `chaos/src/campaign.rs` (`tools_of`, `check_archived`) |
| 2 | Kind 7 is `analog.rise_time_computed` (Rust `Record::RiseComputed`). A bus fault's cause is `analog.rise_time_computed > ceiling_ns`, in the CLI and in `propose`'s `PAST_CEILING_CAUSE`. The doc comments that called a rise "measured" now say "computed". | `chaos`, `cli/src/analog.rs`, `evolve/src/propose.rs`, READMEs |
| 3 | Kinds 7 and 8 carry `window: [deck_hz, declared_hz]`. The verdict's analog records carry `deck_hz` and `declared_hz`. The deck's window is `aethelgard_twin::analog::DECK_HZ` (400 kHz, from `BIT_PS`). The declared speed is the solved topology's `i2c_speeds[bus]`. A line whose bus declares no speed is refused before the bridge runs (`INV-06`). The window itself is unchanged. | `twin/src/analog.rs` (the constant only), `chaos/src/record.rs`, `cli/src/analog.rs` |
| 5 | Kind 10 carries `executable`: the `[start, end)` ranges the judge placed `pc` against (`elf::Image`, physical, which equals virtual), sorted, at most 16 (`elf::check` refuses more). Replay's rules require `pc_outside_firmware` in the verdict exactly when kind 10's `pc` is outside them. In evolve, `pc_outside_firmware` selects kind 10 and is board software evidence (`S2`). `propose` reports it as `pc_inside_firmware`, solved 1 and observed 0 (F3). | `chaos/src/record.rs`, `chaos/src/campaign.rs`, `cli/src/elf.rs`, `evolve` |
| 5 | Seeded case `leave-firmware`: after the boot word, the core jumps to `0x70000001`, twin_tier2's measured A7 jump, on the 2.2 kΩ reference. Measured: `pc_outside_firmware` alone, `software S2`, evidence `3:7 4:7 5:10`, and a generator report for `firmware` with `pc_inside_firmware`. | `scenarios/sentinel-01/seeded/cases.json` |
| 7 | `power::SOURCE`: DS12117 Rev 10, January 2026, Table 16 (section 6.3.4, p. 107), read from st.com (fetched by Scotty, 2026-11-14), sha256 `607b9ead…c032`. I hashed the PDF myself (`~/.cache/aet-2301/ds/ds12117-rev10.pdf`) and read Table 16 in its text (`ds10.txt` L7238–7253). Every `VBOR0` to `VBOR3` value and `tRSTTEMPO` is equal to Rev 7's. A test holds Rev 7's values as independent literals. The verdict's `thresholds` text names Rev 10. | `chaos/src/power.rs`, `cli/src/analog.rs` |
| 8 | `ObservedReport.witness` is `Option<Vec<String>>`: the lines kind 19 archives, and nothing else. The droop judge still reads the whole witness (`expected` included) before slimming. | `cli/src/verdict.rs`, `cli/src/analog.rs`, `cli/src/campaign.rs` |
| 9 | `TRACE_VERSION` 5 → 6, once, for all of the above. Versions 1 to 5 and 7 are refused by name (`UnknownVersion`). The golden trace is re-pinned. The test derives version 5's and version 4's hashes back from the new bytes. | `chaos/src/event.rs`, `chaos/tests/archive.rs`, `chaos/tests/golden.rs` |

**Tests added (tier 1):**
- chaos:
  - `the_header_names_the_engine_and_renode_and_the_bridge_only_when_it_ran`;
  - `a_header_whose_tools_are_unsorted_or_not_printable_is_not_written`;
  - `pc_outside_firmware_is_exactly_a_pc_outside_the_archived_segments`;
  - `kind_10s_segments_and_kind_7s_window_are_checked_on_write_and_read`;
  - `every_rev_10_value_equals_rev_7s`;
- evolve: `a_core_outside_the_archived_segments_is_software_on_healthy_buses`, `a_core_outside_the_firmware_is_a_firmware_report_of_where_it_ended`;
- twin: `the_deck_window_is_400_khz`;
- CLI: `the_header_names_the_renode_build_the_stub_printed_and_replay_reports_it` (§4's stub test: a stub prints `build: 9.9.9+20991231gitdeadbeef0`, and the header and `replay` name it; a stub with an empty build line is refused, exit 4, `named no build`).

No tier-2 test function was added, so `TIER2_MIN` is unchanged. The seeded block builds one more firmware (`leave-firmware`) inside its existing tests.

## Baseline: `main` against the build

`main`'s binaries (`a5be795`, built in the image) and this branch's (`de96ed9`) ran the same scenarios and the same firmwares (`~/.cache/aet-2205/baseline/run.py`, `compare.py`, output `compare-1.txt`). The firmwares were built once with `main`'s `aethelgard build`, since the generated firmware is outside the fence.
- **Coverage:** all 15 scenarios against the unmutated firmware, and each seeded mutant's cases against its firmware. That is 24 runs, plus `leave-firmware`. Every run went through `campaign`, `replay`, `attribute` and `propose`, plus one static `simulate` of the reference.
- **Unchanged.** Every campaign line except `state_sequence_hash` and `trace_bytes`, which move with the trace. `event_count` is unchanged. Every verdict, finding, match and exit code. Every `attribute` answer: domain, rule, evidence, symptoms, masked, unexplained, index for index. Every `propose` outcome and file, except its `state_sequence_hash` and the files' hashes that follow it.
- **Changed, each named by this work order:**
  - `replay` gains `tools` on every run (§1.1): `aethelgard 0.1.0`, `aethelgard-bridge 91e72fcc…cebb`, `ngspice 44.2`, `renode 1.17.0+20260907gitf1dd1b4af`.
  - The static `simulate` verdict's analog records: `kind` `analog.rise_time_measured` → `analog.rise_time_computed` (§1.2), and `deck_hz` 400000 and `declared_hz` 400000 added (§1.3).
  - A droop's verdict: `observed[].witness` changes from `{peripheral, reference, device, expected, observed, absent}` to the observed lines (§1.8), and `rail.thresholds` names Rev 10 (§1.7). No command prints a droop's verdict: `campaign` archives it as kinds 16 to 19 and prints its line. Kind 19's bytes do not change, since it already held only the observed lines.
  - The new seeded case, present only in the build.

## §1.6 — the seeded set's exact evidence (STOP: ruling needed)

**The table.** It is measured, not predicted: each row is `attribute`'s answer for the case on this branch, read from `compare-1.txt`'s build side. `index:kind`, with indices into the run's trace. The evidence, symptoms and masked columns are identical, index for index, to `main`'s for every case `main` has (the baseline above). Kind 7 is a computed rise, 9 a bus fault, 10 the boot, 11 a register, 12 an identity read's record, and 13 its witness.

| Case | Scenario | Mutant | Rule | Evidence (exact) | Masked (exact) | Symptoms (for reference) |
|---|---|---|---|---|---|---|
| `pullup-3k9` | `seeded/pullup-3k9.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `pullup-4k7` | `seeded/pullup-4k7.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `pullup-10k` | `seeded/pullup-10k.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `pullup-47k` | `pullup-47k.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `pullup-100k` | `seeded/pullup-100k.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `cap-220p-2k2` | `seeded/cap-220p-2k2.json` | — | E1 | 3:7 4:7 5:9 | — | 3:7 4:7 14:12 |
| `cap-390p-sda-2k2` | `seeded/cap-390p-sda-2k2.json` | — | E1 | 4:7 5:9 | — | 4:7 14:12 |
| `held-low-sda` | `seeded/held-low-sda.json` | — | E1 | 4:9 | — | 4:9 13:12 |
| `held-low-scl` | `seeded/held-low-scl.json` | — | E1 | 4:9 | — | 4:9 13:12 |
| `trap-wrong-address-47k` | `pullup-47k.json` | `wrong-address` | E1 | 3:7 4:7 5:9 | 15:13 | 3:7 4:7 14:12 |
| `wrong-address-2k2` | `pullup-2k2.json` | `wrong-address` | S1 | 3:7 4:7 14:13 | — | 13:12 |
| `trap-wrong-address-3k3` | `seeded/pullup-3k3.json` | `wrong-address` | S1 | 3:7 4:7 14:13 | — | 13:12 |
| `wrong-register` | `pullup-2k2.json` | `wrong-register` | S1 | 3:7 4:7 14:13 | — | 13:12 14:13 |
| `wrong-nbytes` | `pullup-2k2.json` | `wrong-nbytes` | S1 | 3:7 4:7 14:13 | — | 13:12 14:13 |
| `i2c-clock-skipped` | `pullup-2k2.json` | `i2c-clock-skipped` | S1 | 3:7 4:7 11:11 | — | 11:11 13:12 |
| `scl-af-wrong` | `pullup-2k2.json` | `scl-af-wrong` | S1 | 3:7 4:7 9:11 | — | 9:11 |
| `sda-af-wrong` | `pullup-2k2.json` | `sda-af-wrong` | S1 | 3:7 4:7 9:11 | — | 9:11 |
| `scl-mode-output` | `pullup-2k2.json` | `scl-mode-output` | S1 | 3:7 4:7 8:11 | — | 8:11 |
| `gpiob-clock-skipped` | `pullup-2k2.json` | `gpiob-clock-skipped` | S1 | 3:7 4:7 8:11 9:11 10:11 | — | 8:11 9:11 10:11 |
| `no-boot-word` | `pullup-2k2.json` | `no-boot-word` | S2 | 3:7 4:7 5:10 | — | 5:10 |
| `trap-unmutated-3k3` | `seeded/pullup-3k3.json` | — | N1 | — | — | — |
| `unmutated-2k2` | `pullup-2k2.json` | — | N1 | — | — | — |
| `leave-firmware` | `pullup-2k2.json` | `leave-firmware` | S2 | 3:7 4:7 5:10 | — | 5:10 |

What the table shows that the `allowed` test did not:
- **Kind 7 in `S1` and `S2` is not "any kind 7".** It is exactly the symptom bus's rises, which make the bus measured healthy, and software needs that (`3100` R2). Every S1 and S2 row has both lines' rises, `3:7 4:7`. A run whose evidence named a third kind 7, or dropped one, passed `allowed`. It fails an exact assertion.
- **Masked is a column of its own.** The 47 kΩ wrong-address trap's software witness, `15:13`, is masked on the faulted bus, never evidence. Today only the trap test asserts it. The table pins `masked` for every case, empty for 22 of 23.

**The indeterminate case, proposed and measured end to end.** It needs no new mechanism: `leave-firmware`'s firmware on the 47 kΩ scenario. I measured candidates first (`~/.cache/aet-2205/baseline/cand.py`, runs under `~/.cache/aet-2205/baseline/cand/`):

| Firmware | Scenario | Findings | `attribute` | Evidence | Symptoms | `propose` |
|---|---|---|---|---|---|---|
| `leave-firmware` | `pullup-47k.json` | rise, timeout, `pc_outside_firmware` | **indeterminate I3** | 3:7 4:7 5:9 | 3:7 4:7 6:10 14:12 | `no_proposal`, exit 1: "attribution rule I3 (`indeterminate`): attribution precedes mutation, so nothing is proposed (INV-05)" |
| `leave-firmware` | `seeded/held-low-sda.json` | held low, timeout, `pc_outside_firmware` | indeterminate I3 | 4:9 | 4:9 5:10 13:12 | `no_proposal` |
| `leave-firmware` | `seeded/cap-390p-sda-2k2.json` | rise, timeout, `pc_outside_firmware` | indeterminate I3 | 4:7 5:9 | 4:7 6:10 14:12 | `no_proposal` |
| `no-boot-word` | `pullup-47k.json` | rise, `boot_word_missing` | indeterminate I1 | — | 3:7 4:7 6:10 | (not run) |
| `gpiob-clock-skipped`, `i2c-clock-skipped`, `scl-mode-output` | each of the three | … | electrical E1, the software masked | … | … | — |

- **Proposed: `trap-indeterminate-leave-47k`.**
  - Scenario: `scenarios/sentinel-01/pullup-47k.json`. Mutant: `leave-firmware`. Domain: `indeterminate`. Rule: `I3`.
  - Findings: `analog_rise_exceeds_ceiling`, `i2c_timeout`, `pc_outside_firmware`.
  - Evidence: exactly `3:7 4:7 5:9`. Masked: none.
  - Both causes are present: a 47 kΩ pull-up past its ceiling, and a firmware that leaves its code. The bus's symptoms are `E1`. The core outside the firmware is a symptom on no bus, beside a faulted bus, so it is `I3`, "the fault is elsewhere". The run as a whole is `I3`: attribution declines to name one half, and names the bus fault as its evidence.
  - It is a trap in the manifest's sense: the 47 kΩ run alone is `E1`, and the 2.2 kΩ run with this mutant alone is `S2`.
  - It adds no firmware and no campaign: `leave-firmware`'s campaign gains one scenario.
- **The alternative: `I1`,** `no-boot-word` on 47 kΩ. Its symptom set leaves event 14 unexplained: the identity read's record shows a timeout, which no finding names, because the boot word's absence suppresses the I2C findings (`3100` F15). That is a gap in what the verdict names, more than a case of two causes, so I do not recommend it as the indeterminate case.

**What the build would do after the ruling (proposed):**
1. `cases.json` goes to `schema_version` 2. Each case gains `evidence` and `masked`, each a list of `[index, kind]`, from the table above.
2. `p3_gate_clause_1_…` asserts `evidence` and `masked` equal to the case's, exactly. It checks each evidence kind against `aethelgard_evolve::Rule::evidence_kinds`, read from evolve by the rule's id, so the `allowed` table goes. It asserts domain and rule as today.
3. The tier-1 manifest test admits `("indeterminate", "I3")` and requires at least one indeterminate case.
4. `p3_gate_clause_2_…` expects `no_proposal` for an `I*` case, with the reason naming the rule.
5. The new case `trap-indeterminate-leave-47k`, as above.
- No tier-2 test function is added. `TIER2_MIN` stays exact, and the block's cost grows by one campaign scenario.

### Decisions needed

- **D1 · The table above is the exact evidence the seeded test asserts per case.** Recommendation: yes, `evidence` and `masked` exact. `symptoms` are listed for the reader. Should they be asserted too? Recommendation: no. They are what the verdict's findings select, already pinned by each case's `findings`.
- **D2 · The indeterminate case is `trap-indeterminate-leave-47k`, `I3`, evidence `3:7 4:7 5:9`.** Recommendation: yes, rather than the `I1` alternative.
- **D3 · F1: ngspice's version is the bridge's checked pin, read from its answers.** Recommendation: accept now, and move the banner into the bridge's identity line when a work order next touches the bridge.
- **D4 · F3: `propose` reports a core outside the firmware as `pc_inside_firmware` (1 solved, 0 observed).** Recommendation: accept.


## Findings and forks taken

- **F1 · ngspice's version is the bridge's pin, as its answers name it · `cli/src/analog.rs` · low · provenance · decision-needed (D3).**
  - The header's `ngspice` is read from the bridge's answers, not written by the CLI. The bridge writes its own constant `NGSPICE` (`bridge/src/main.rs:234,292`) after it refuses any library whose banner is not `** ngspice-44.2 shared library` (`bridge/src/solver.rs:57`). So the value is the library's, through the bridge's check, but the text archived is the bridge's.
  - Reading the banner itself needs a bridge change, which is outside §2's fence and moves `BRIDGE_SOURCE_SHA256`.
  - Recommendation: accept for this work order, and have the bridge's identity line carry the banner it read in the next work order that touches the bridge.
- **F2 · Fence extension, named: `cli/src/main.rs` · low · scope.**
  - `simulate` reads `renode --version`, and its old helper kept only the first line. `main.rs` now keeps the whole output, reads the `build:` line through `renode::build`, and refuses a boot whose Renode names none (exit 4).
  - It also passes the tools and the executable segments to the campaign, in two verdict fields marked `#[serde(skip)]`. The verdict's JSON is therefore unchanged beyond the fields named above.
- **F3 · `propose`'s expectation for a core outside the firmware is a 1/0 · `evolve/src/propose.rs` · low · fork.**
  - A generator report's `Expectation` holds one solved value. For a `pc` outside every segment, there is a range, not a value. It is reported as `pc_inside_firmware`, solved 1, observed 0, with `address` and `mask` null, and documented on the field.
  - The alternative was no report. `report()` then answers `no_proposal` for a software attribution with no unmet expectation, so the new S2 case would have been the only software case with no generator report.
- **F4 · The engine's version is the package version (`0.1.0`), not a commit · low · residual.**
  - It is what the binary knows of itself. Two builds of different commits both say `0.1.0`.
  - The header's `firmware` artifact and the bridge's digest are content hashes. The engine's is not.
  - Recommendation: backlog, if replay should tell engine builds apart.
- **F5 · The rule docs in evolve still say "measured healthy" (I2, I4, S1, S2) · low · wording.** They describe rules over computed rises. They are not kind or field names, so §1.2 does not reach them, and I left them.
- **F6 · Replay cannot bind the segments to the ELF · residual (`RUL-19` (3)).** The firmware is not archived, so replay checks that kind 10's segments agree with the verdict's `pc_outside_firmware` (the new rule). It cannot check that they are the ELF's. A forged archive with both changed consistently replays, as any forged archive with a rewritten manifest does. §5.1 probes this.

## Gates (at `3d12877`)

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (with and without `with-tools`), `cargo test --workspace`: green on Furnace. Tier 2 in the image, the suites this work order touches: `attribute_seeded` 12 passed (327 s, the new case included), `campaign_tier2` 8 passed, `twin_tier2` 22 passed (`~/.cache/aet-2205/t2-1.log`). CI: on the PR (every suite, eight shards).

