# AET-WO-3200 — FEEDBACK

**Branch:** `wo/aet-3200-propose` · **Base:** `bf869b4` (`origin/main`, as the kickoff set it). `AET-WO-1203` merged to `main` as `402fa6f`. `origin/main` is merged in with a merge commit (`e1317ab`), and every number below is from the merged tree (see "Merge of `main` after `AET-WO-1203`") · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, as project manager. A fresh Opus subagent ran the full adversarial pass, and a default-model subagent ran the diff-sized mutation run, each on a frozen copy in its own worktree. · **Cadence:** build-only. No §7 condition fired (see "§7, held").

**Where everything ran.**
- **Tier 1:** on Furnace, in a capped user scope.
- **Tier 2:** in the pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`), under rootless podman 5.7.0, offline, in the CI `tools` job's own layout: the repository at `/repo`, with `CARGO_HOME=/repo/target/cargo-home` and `CARGO_TARGET_DIR=/repo/target` (3100 F17). Each container ran with `--memory 16g --memory-swap 16g --pids-limit -1`, and the runner then wrote 0 into the container's `memory.swap.max` (3100 F11).
- **Caps, read back from the container's own scope on every run:** `memory.max` = 17179869184, `memory.swap.max` = 0, `pids.max` = `max`.

**§7, held:**
- **Every fact the proposer needs** is in the archive or in the solved topology that `propose` re-derives and checks against it (kinds 6 and 15).
- **The change goes through `campaign`'s override path** (`apply_overrides`, extracted with `campaign`'s behaviour unchanged: F7).
- **The 2.7 kΩ re-gate passes.** Its rise is 2 ps from prediction (0.001 %). The 0.8 was not touched.
- **No test was weakened.** Every test that changed is listed under "Tests changed".

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## §3, acceptance: P3 gate clause 2

### §3.1 and §3.2: the 47 kΩ run, proposed and re-gated

**`propose` on `pullup-47k`'s run** (tier 2, the real bridge and Renode): one `Proposal`, exactly one change, `{"bus": "SENSORS", "pull_up_ohms": 2700}`, exit 0.

**The worked numbers, measured.** Every rise is computed from declared values (`RUL-19`).

| | Work order | Measured (`proposal.json`) |
|---|---|---|
| `R_now` | 47,000 Ω | 47,000 Ω |
| `rise_worst` (SCL and SDA alike; SCL binds by order) | 3,982,301 ps | 3,982,301 ps |
| ceiling × 0.8 | 240,000 ps | 300,000 ps × 8/10 |
| `R_target` | ≈ 2,832 Ω | 2,832 Ω (2,832.5, rounded down) |
| E12 | 2.7 kΩ | 2,700 Ω |
| `Rp_min` | ≈ 967 Ω | 967 Ω, from `aethelgard_ir::electrical::i2c_pull_up_floor_ohms(400_000, 3_300)` |
| `predicted_rise_ps` | about 229 ns | 228,771 ps (rounded up) |

**The diff** from the run's effective specification (the reference with the scenario's 47 kΩ override) to `proposed-spec.json` is that one field, 47,000 → 2,700, checked field by field in tier 1 (`campaign_tier1.rs` §8) and in tier 2. Against the reference specification on `main` (2,200 Ω) it is also one field. The reference is not touched.

**The re-gate (§1.4)** of that `proposed-spec.json` (sha256 `54aaef1c…52bc`), in `attribute_seeded.rs`'s `every_gate`:

| Gate | Result |
|---|---|
| `validate` | exit 0 |
| `solve` | exit 0 |
| `emit` | exit 0 |
| `check erc` | pass |
| `check drc --spec --schematic` (schematic parity) | pass |
| board crate generated with `generate_with` from the proposed topology, then `prove --harness proofs::every_valid_sequence` | pass |
| firmware generated with it, built offline; a campaign of one `expect: pass` scenario on the proposed specification | `matched` |
| `attribute` on that run | `none`, `N1` |
| §5.5: kind 15 (bus, lines, ceilings, devices) equal to the failing run's; every device the failing run read is read and answers `ok` | holds |
| **The binding line's rise against the prediction** | **228,769 ps against 228,771 ps: 2 ps, 0.001 %** (limit 2 %) |

### §3.3: the proposal branch

PROPOSAL-BRANCH

### §3.4: the other outcomes, on 3100's seeded runs

Each case's expected answer is stated in the test (`want()` in `attribute_seeded.rs`), never read from the tool. For a generator report, the expected generator comes from the mutant's `file` in `cases.json`: `firmware/…` is the firmware's, and `src/board.rs` is the board crate's.

| Case | Attribution | `propose` | Exit | Detail |
|---|---|---|---|---|
| pullup-3k9 | E1 | proposal 2,700 Ω | 0 | `R_target` 2,832; re-gate rise 228,769 against 228,769 predicted |
| pullup-4k7 | E1 | proposal 2,700 Ω | 0 | the same proposed specification (sha `54aaef1c…`) |
| pullup-10k | E1 | proposal 2,700 Ω | 0 | ″ |
| pullup-47k | E1 | proposal 2,700 Ω | 0 | ″ |
| pullup-100k | E1 | proposal 2,700 Ω | 0 | ″ |
| cap-220p-2k2 | E1 | proposal 1,200 Ω | 0 | `R_target` 1,287; re-gated (sha `aa4d7c62…`): rise 223,682 against 223,685 predicted |
| **cap-390p-sda-2k2** (was `cap-470p-sda-2k2` before `AET-WO-1203`) | E1 | **no proposal: below the floor** | 1 | SDA 726,978 ps; `R_target` 726 Ω; E12 680 Ω < `Rp_min` 967 Ω. F1 |
| held-low-sda | E1 | no rule: model slot | 1 | `no rule; RUL-18 model proposer slot` |
| held-low-scl | E1 | no rule: model slot | 1 | ″ |
| trap-wrong-address-47k | E1 (witness masked) | proposal 2,700 Ω | 0 | F4 |
| wrong-address-2k2 | S1 | generator report: `firmware` | 6 | `I2C1.CR2` SADD solved `0xec`, observed `0xee` |
| trap-wrong-address-3k3 | S1 | generator report: `firmware` | 6 | ″ |
| wrong-register | S1 | generator report: `firmware` | 6 | `I2C1.TXDR` solved `0xd0`, observed `0xd1` |
| wrong-nbytes | S1 | generator report: `firmware` | 6 | `I2C1.CR2` NBYTES solved `0x010000`, observed `0x020000` |
| i2c-clock-skipped | S1 | generator report: `board_crate` | 6 | `RCC.APB1LENR` bit 21 solved set, observed clear |
| scl-af-wrong | S1 | generator report: `board_crate` | 6 | `GPIOB.AFRL` AFSEL6 solved 4, observed 5 |
| sda-af-wrong | S1 | generator report: `board_crate` | 6 | `GPIOB.AFRL` AFSEL7 solved 4, observed 5 |
| scl-mode-output | S1 | generator report: `board_crate` | 6 | `GPIOB.MODER` MODE6 solved AF, observed output |
| gpiob-clock-skipped | S1 | generator report: `board_crate` | 6 | `RCC.AHB4ENR` bit 1, and the four pin fields it left at reset |
| no-boot-word | S2 | generator report: `firmware` | 6 | `boot_word` solved `0x2100b007`, observed 0 |
| trap-unmutated-3k3 | N1 | nothing proposed (`INV-05`) | 1 | |
| unmutated-2k2 | N1 | nothing proposed (`INV-05`) | 1 | |

No generator report contains a specification change, and no `proposed-spec.json` was written for any of them. **The indeterminate run** (tier 1, through the chaos writer: a verdict whose only finding no event shows, `I1`) gives no proposal, exit 1, naming `I1` and `INV-05`.

### §3.5: twice, and refusals

- **`propose` twice on every seeded run:** identical files (tier 2). The same holds in tier 1 with another output directory and an empty `PATH`, where the stdout line differs only in `out`.
- **Refused, writing nothing** (tier 1):
  - a scenario that is not the run's (`ScenarioMismatch`);
  - the run's scenario whose base changed by one byte after the run (`BaseChanged`);
  - a base outside the current directory (`BaseOutsideRoot`, as `campaign` refuses it);
  - a kind 15 re-sealed to disagree with the topology (`TopologyChanged`);
  - a one-byte change to the run, with `replay`'s exact bytes (`HashMismatch`);
  - an `--out` that is not empty, is a file, or is a symlink (`UnwritableOutput`).
- The same scenario and base copied to another directory give the same run's proposal.

## What was built

1. **`aethelgard-evolve`.**
   - **`view(&Replayed) -> RunView`** (§8, review A8.12). For an event, it gives the event's bus and, for a register or witness event, each field's solved value against the observed one. `attribute` does not use it.
     - **`attribute`'s output is byte-identical:** on all 22 probe archives from 3100, `attribute` stdout on this branch equals `main`'s, byte for byte.
     - Internally, the run setup moved into `joined()`, and the witness check now returns its three fields, through `witness_fields()`. The logic is unchanged, and all 48 tests in `tests/attribute.rs` pass unmodified.
   - **`propose`:**
     - `Change { bus, pull_up_ohms }`: no other change can be expressed;
     - `BusFacts`, `Input`, `Sizing`, `Proposal`, `GeneratorReport` (with `Expectation` and `Generator`), and `Outcome`;
     - the `Proposer` trait;
     - the chain `propose()`;
     - the `Rules` table.
   - It gains no dependency and no copy of the floor.
2. **The CLI.**
   - **`aethelgard propose`** (`src/propose.rs`), with the checks of §1.3, the floor from `aethelgard_ir::electrical::i2c_pull_up_floor_ohms` (no copy anywhere), and output written only into an empty or absent `--out`.
   - **`campaign.rs`:** `read_scenario`, `effective` and `apply_overrides` were extracted from `prepare` and `apply` (F7).
   - **`mcp.rs`:** exit 6 is a successful call (F2).
   - **README:** a `## propose` section, the exit-code table, and the refusal kinds.
3. **Tests:**
   - `evolve/tests/propose.rs`: 26;
   - `campaign_tier1.rs` §8: 9;
   - `attribute_seeded.rs`: two tier-2 tests, with the `want()` table, `regate()` and `every_gate()`, and one tier-1 test of committed proposals.

## Findings and the forks taken

**Decision-needed:**

- **F1 · A capacitance case no pull-up can fix proposes nothing, against §3.4's wording · `Rules`, `want()` · high · work-order conflict · decision-needed.**
  - §3.4 says every pull-up and capacitance case gives a proposal whose re-gate passes. §1.2 says nothing is proposed when no E12 value lies in `[Rp_min, R_target]`. For the 390 pF case (and the 470 pF case it replaced) the two cannot both hold.
  - **Measured, on the merged tree:** `cap-390p-sda-2k2` has SDA at 726,978 ps with 2.2 kΩ, so `R_target` = 2,200 × 240,000 / 726,978 = 726 Ω. The E12 value is 680 Ω, under `Rp_min` 967 Ω.
  - **Before the merge,** `cap-470p-sda-2k2` gave the same refusal: SDA 876,105 ps, `R_target` 602 Ω, E12 560 Ω.
  - **Followed:** §1.2, the floor. The test asserts no proposal, with the numbers in `proposal.json`'s `sizing`. Neither the floor nor the 0.8 was touched.
  - Even with no headroom, `R_target` would be 2,200 × 300,000 / 726,978 = 907 Ω, still under the floor. The only electrical fix is less capacitance, which `INV-07` forbids as a lever.
  - **Recommendation:** amend §3.4 to "every pull-up case, and every capacitance case a pull-up at or above the floor can fix, gives a proposal whose re-gate passes; the others give a below-floor refusal with the numbers". Optionally, seed one capacitance case that is fixable. `cap-220p-2k2` already is: it proposes 1.2 kΩ and re-gates.
- **F2 · Exit codes: 0 proposal, 1 no proposal, 6 generator report · `main.rs`, `mcp.rs`, README · medium · contract · decision-needed.**
  - `RUL-10`'s table has two non-refusal codes, 0 and 1. Codes 2 to 5 mean a refusal or a tool error, so the third outcome needs a new code.
  - **1 for no proposal** matches `attribute`, where 1 means no determinate answer.
  - **6 is new, used by `propose` alone.** `mcp.rs` counts 0, 1 and 6 as successful calls; otherwise a generator report would reach an MCP client as an error.
  - The MCP mapping accepts 6 from any subcommand, but only `propose` exits 6 (adversarial note).
  - **Recommendation:** ratify, and add 6 to `RUL-10` by amendment.

**Forks (decision-needed: ratify):**

- **F3 · The binding line is the least `ceiling / rise`, not the slowest rise · `size()` · medium · design.** Kind 15 carries a ceiling per line (3100 F1). With equal ceilings, the least ratio is the slowest line, which is §1.2's `rise_worst`. With unequal ceilings, a slower line can bind less. Tested at 300 and 250 ns: SDA binds at 280 ns over SCL at 330 ns.
- **F4 · The 47 kΩ trap with a wrong address proposes 2.7 kΩ, and says what it masked · `proposal.json` · medium · design (adversarial B2).**
  - `E1` masks the software evidence, so the rules propose the electrical fix.
  - The proposed specification is byte-identical to the clean 47 kΩ case's. Its re-gate regenerates the firmware, so it passes and cannot speak to the mutant firmware that failed.
  - **Built:** `proposal.json`'s attribution carries `masked`, and the tier-2 test asserts it names the witness (kind 13).
  - The loop works one fault at a time: with 2.7 kΩ applied and the same firmware rerun, the run is `S1` and gets a generator report.
- **F5 · `propose` checks kind 15 against the topology too · `src/propose.rs` · low · beyond §1.3.** Besides §1.3's checks, the run's kind 15 records must equal what `campaign`'s own `solved_buses` writes from the re-derived topology. Kind 7's ceilings are tied to kind 15 by `check_run`, so the ceilings the rule sizes against are the topology's. A re-sealed run with a tighter ceiling is `TopologyChanged`.
- **F6 · The chain guards every proposer, not only the rules · `evolve::propose::propose` · medium · design.**
  - Whoever answers, a proposal stands only for an `electrical` run, with one change per bus the CLI gave facts for, each at or above that bus's floor. A generator report stands only for a `software` run.
  - So a later model proposer (`RUL-18`) cannot propose for an `indeterminate` run or below the floor.
  - It can still propose any value at or above the floor, `u32::MAX` included, which makes the rise worse (adversarial note). The re-gate is the check on that, as `RUL-07` intends.
- **F7 · `campaign`'s override path was extracted, not copied · `cli/src/campaign.rs` · low · fence.**
  - `prepare` now calls `read_scenario`, `read_base` and `effective`, and `apply` delegates to `apply_overrides(&[Override], …)`. `propose` applies its changes through `apply_overrides`, as `Override::PullUpOhms`.
  - Every pre-existing `campaign` and `replay` test passes unmodified, and the mutation run's mutants of the extraction (K01 to K05, K07, K08) are killed.
- **F8 · The rules' "no rule" rows are wider than §1.2's list · `past_ceiling()`, `size()` · low · design.** Besides held low, stuck high and an incomplete rise, these also have no rule:
  - a bus fault with any cause but a rise past the ceiling, or with a second cause;
  - a faulted bus with a line not measured exactly once and completely;
  - a binding line not past its ceiling.
- **F9 · `NoProposal` carries `sizing` when a rule matched but the floor refused every value · `Outcome::NoProposal { reason, sizing }` · low · interface.** §1.1 names `NoProposal { reason }`. The numbers are kept so a reader sees why: F1's case.
- **F10 · Which generator an expectation belongs to · `report()` · low · design.**
  - A controller witness (kind 13) and the boot (kind 10) are the firmware's. A register (kind 11) is the board crate's.
  - The tier-2 oracle is each mutant's `file` in `cases.json`, and all ten cases agree.
  - A board-crate defect that halts the CPU, or a fault in the board crate's `transfer`, would be named `firmware`. No seeded mutant has one (adversarial note).
  - A register bit no bus owns is reported with the judge's expectation as its solved value.

**Residuals and notes:**

- **F11 · The firmware ELF depends on its build path · tier 2 · high · harness.**
  - **Measured:** the board copy built at two paths gives two sha256s (`55574bc4…` and `ade7ffd0…`).
  - Only `.strtab` differs; the loaded code is identical. Cargo hashes the path dependency's absolute path into each symbol's crate disambiguator.
  - Run ids hash the firmware, so a committed `proposal.json` reproduces only from a firmware built at the same path. The unmutated reference firmware is therefore built at the literal `/tmp/aethelgard-reference-firmware`, which `TMPDIR` cannot move. Two tier-2 runs on one machine at once share it, and collide.
  - **Recommendation:** have the generator make firmware builds path-independent, with `strip = "symbols"` or `--remap-path-prefix` (backlog, `AET-WO-3301`).
- **F12 · The re-gate generates the board crate and firmware the way the repository's tests do · `every_gate()` · low · residual (§8).** It uses `aethelgard_svd_hal::board::generate_with` from the proposed topology, as `aethelgard-svd-hal/tests/board.rs` does, because `aethelgard generate` does not exist yet. `AET-WO-3301` removes this.
- **F13 · The committed-proposal tier-2 test is vacuous on this branch · `attribute_seeded.rs` · low · by design.** `main` holds no `proposals/`. The proposal branch's CI runs it. Its tier-1 half checks the files, the change form, the hash, validation, the scenario and the directory name (`<project>-<scenario name>`), and runs in `gates` as well. Its tier-2 half re-runs `propose` to the same bytes and re-gates.
- **F14 · `proposed-spec.json` is pretty-printed · low · format.** It is pretty-printed, with a final newline, for review, so its sha256 is not the compact serialization's that `campaign` would name for the same specification. `proposal.json` names the file's own hash.
- **F15 · `write_out` race · low · note.** If another process creates one of the output names between the emptiness check and the create, `propose` refuses with `proposal.json` already written. Nothing is overwritten or followed.
- **F17 · The `tools` job is near its 60-minute timeout · `.github/workflows/ci.yml` · medium · CI capacity, human-track.**
  - On `main`'s last run (43177112749), `tools` took 45 min, and its test step 43 min.
  - A passing board proof takes about 200 s there (`ANSWER 1400-C5 … board … wall_s=198.59`). The re-gate adds two proofs, one per distinct proposed specification, plus two firmware builds and two campaigns: about 8 minutes.
  - The proposal branch's committed proposal reuses the 2.7 kΩ re-gate, so it adds no proof.
  - `ci.yml` is outside this work order's fence. **Recommendation:** raise `timeout-minutes` for `tools`, or split tier 2 into two jobs.
  - **Measured on PR #86** (run 43202082866, head `0b9bd8c`): `tools` passed in 54 min, 9 more than `main`'s 45.
  - **Answered by `AET-WO-1900`,** which set the timeout to 90 minutes. That is kept through the merge.
- **F18 · The `gates` package step verified the CLI against crates.io's 0.1.0 · `.github/workflows/ci.yml` · high · CI, ruled.**
  - **Found** on the merged tree, by running the step verbatim from `ci.yml` on Furnace: exit 101, `E0432 unresolved import aethelgard_ir::electrical`, while verifying `aethelgard-cli`.
  - **Cause:** 0.1.0 of every crate is published on crates.io. `cargo package`'s verify build resolves a sibling to the published 0.1.0, which lacks `AET-WO-1203`'s module and evolve's `propose`. `main` passed only because its CLI did not yet call either.
  - **Ruled 2026-11-12 (architect):** `cargo package --locked --no-verify`, in this PR, with the reason beside the line. The step's second half is the check: it builds the CLI offline from the unpacked packages, with every sibling patched to them, and validates the example.
  - **Measured:** the step fails (exit 101) without the change and passes (exit 0) with it; every crate packages, and the CLI built from the packages validates the example.
  - **The architect owns the version bump to 0.2.0** at the next release, which retires the collision.
- **F16 · `gh-write pr` has no draft flag · Furnace · medium · human-track, ruled.**
  - **Ruled 2026-11-12 (architect, option 1):** the proposal PR (§3.3) is opened as a normal PR titled "PROPOSAL (do not merge): …", based on `wo/aet-3200-propose`, and the architect converts it to a draft on GitHub.
  - **Recommendation:** furnace777 adds a `--draft` flag to `gh-write pr`.


## Tests changed

- **`mcp.rs` unit tests and `mcp_tier1.rs`:**
  - the tool list gains `propose` in both pins;
  - the throwaway count goes from 11 to 12;
  - the exit-code test is renamed `exit_0_1_and_6_are_successful_calls_and_others_are_errors`, and gains 6 (success) and 7 (error). Codes 0 to 5, 64, 70 and a signal keep their old results.
- **`attribute_seeded.rs`, 3100's file:**
  - `firmware()` builds the unmutated firmware at the literal `/tmp/aethelgard-reference-firmware`, not a per-process directory (F11). Mutants still build in the per-process scratch directory.
  - `aethelgard()` calls the new `aethelgard_in(&root(), …)` (reduction R7).
  - Every 3100 test in it is unchanged.
- **`campaign_tier1.rs`:** a new §8 only. Every earlier test is unchanged.
- **`aethelgard-evolve` `tests/attribute.rs`:** unchanged.


## The fence

**Changed:**
- `aethelgard-evolve`: `src/propose.rs` (new), and in `src/lib.rs` the view, `joined()` and `witness_fields()`;
- the CLI:
  - `src/propose.rs` (new);
  - `src/campaign.rs`, the extraction only (F7);
  - `src/main.rs`, the subcommand and exit 6;
  - `src/mcp.rs`, exit 6;
- the CLI and evolve READMEs;
- tests;
- this FEEDBACK;
- one line of `.github/workflows/ci.yml`, by the architect's ruling (F18).

**Not touched:**
- `aethelgard-chaos`, `aethelgard-ir` and every generator;
- the judge (`renode.rs`, `aethelgard-twin`), `simulate --firmware` and `cli/src/analog.rs`;
- `replay`'s and `attribute`'s output: byte-identical on all 22 probe runs (measured twice, before and after the reduction pass);
- `boards/`, `scenarios/`, the steering documents and `AET-WO-3200.md`.


## Adversarial pass (full, §5)

**Who and how.**
- A fresh Opus subagent, chosen because this is judgement work. It worked on a frozen copy at `a1f5223`, in its own worktree and target directory.
- Its inputs were the 22 real archived runs from 3100's probe and runs it forged through the chaos writer.
- It ran a 400,000-input fuzz of the rules on hand-built inputs, `strace -f` on each outcome, and the CLI under altered cwd, `PATH`, `TZ`, `LC_ALL`, `HOME`, `SOURCE_DATE_EPOCH` and a non-UTF-8 `--out`.
- **Harnesses:** `~/.cache/aet-3200-adv/harness/`.

| id | attack (§5) | result | fix | test |
|---|---|---|---|---|
| B1 | 2: below `Rp_min` | **broke.** `facts()` took the speed from `i2c_ceilings`, which is the fastest speed the rise ceiling allows, not the bus's. A 400 kHz bus with 120 ns ceilings got the 1 MHz floor of 145 Ω, and the CLI proposed 390 Ω | the floor is judged at the bus's declared `speed_hz`, from the effective specification, as `AET-WO-1203`'s `solve` judges it | `the_floor_is_judged_at_the_buss_declared_speed_not_its_ceilings` |
| B2 | 5: a re-gate passed for the wrong reason | **broke** (silently). The trap's proposal re-gates on regenerated firmware, and nothing said the wrong address was masked | `proposal.json` carries `masked` (F4) | tier 2 asserts `masked` is `[13]` for the trap and empty otherwise |
| B3 | 4: a write that damages a run | **broke.** `--out` inside the run directory wrote there, and the run no longer replayed | refused: `--out` must resolve, symlinks followed, outside the run directory | `propose_never_writes_into_the_run_it_reads` |
| H1 | 1: move a gate | held. `Change` holds only a bus and a pull-up, and it is applied through `apply_overrides`. A malicious proposer is refused for a non-electrical run and below the floor (F6) | — | `no_proposer_may_propose_…` |
| H2 | 2: arithmetic edges | held. 400,000 inputs at u32/u64 bounds, with zero and huge rises and ceilings, unequal ceilings and incomplete rises. No panic. Every proposal is an E12 value, at or above the floor and at or below `R_target`, and every line predicts within 0.8 of its ceiling | — | — |
| H3 | 3: runs to refuse | held: every I-rule, N1, software runs, and a scenario, base, topology or kind 15 not the run's | — | existing tests |
| H4 | 4: two calls, writes, processes | held. strace shows one `execve` (its own), no fork, and writes only by `mkdir` of `--out` and `O_EXCL\|O_NOFOLLOW` creates inside it. The bytes are identical under every environment change; a FIFO or dangling-symlink `--out` is refused | — | — |
| H5 | `attribute` and `campaign` unchanged | held. `bf869b4` and `a1f5223` builds give identical stdout and exit codes for `attribute` and `replay` on all 22 runs (44 comparisons) | — | — |
| N1 | the view and `attribute` disagree on a register's bus | note. A register wrong both in a bus's field and in an unowned bit is S1 evidence on that bus, but the view says no bus. The report does not read it | the doc comment says so | — |
| N2 | the committed-proposal test | should-fix. It was vacuous without `proposals/`, did not bind the directory name, checked `sizing[0]` only, and ran only in tier 2 | a tier-1 half (F13); the name is bound; every `sizing` entry is checked | `every_committed_proposal_is_pull_up_changes_named_for_its_scenario` |
| N3 | the fixed firmware path | note. It moved with `TMPDIR`, and two runs share it | a literal path (F11) | — |


## Mutation testing (sized to the diff)

**Who and how.**
- A default-model subagent, on frozen copies at `a1f5223`.
- One mutant per guard this work order adds or changes, in evolve's `propose.rs` and view, the CLI's `propose.rs`, the extraction and the MCP mapping.
- Suites: evolve's tests, plus the CLI's `campaign_tier1`, `mcp_tier1` and unit tests. Tier 2 was not mutated.
- **Table:** `~/.cache/aet-3200-mut/MUTANTS.md`.

**Round 1:** 196 mutants. 117 killed, 64 survived, 14 equivalent, and 1 did not compile.
- **The 14 equivalent,** spot-checked against the source:
  - the scenario sha covers its overrides;
  - the specification and topology hashes move together;
  - the interim floor is continuous at 2 V;
  - the overlapping `write_out` checks;
  - the decade bound;
  - the order the files are written in.
- **The 64 survivors, and what killed them** (commit `4acbb38`):
  - hand-built `Input` rows for each `past_ceiling()` and `size()` edge (P17 to P43);
  - the whole E12 table, checked against a table the test writes out itself (P59, P60);
  - the boot report's instruction count, and the refusal for evidence with no unmet field (P68 to P80);
  - direct `RunView` tests on one and two buses, a partly covered reading, an unowned bit and a renamed address (L03 to L49);
  - the three artifact hashes, `schema_version` and the stdout `files` map;
  - a report naming both generators, once each, in order (C41 to C45, C52, C55).
- **Four view fields had no consumer** (L35, L36, L38, L39): `Rise.event`, `Rise.exceeds`, `SolvedBus.peripheral` and `SolvedBus.devices`. They were deleted, not tested.
- **Left as notes:**
  - C08 and C11, the kind-15 length and two-bus forgery: the reference specification has one I2C bus, and `replay` refuses a dropped kind 15;
  - C20, the interim floor's low-rail factor: it is replaced by `AET-WO-1203`'s function;
  - C28, the file mode under umask 0;
  - C34, a closed stdout: it is the shared `emit` path;
  - C40, an `apply_overrides` refusal of a floor-legal pull-up, which no specification rule makes;
  - K06, a message prefix: messages are not contract (README).


## Reduction pass

**Who and how.** A fresh subagent ran the four questions over the diff on a frozen copy at `242ed7c`. Each reduction was a separate patch, and each was proved with fmt, clippy (`with-tools`) and the touched suites. Report: `~/.cache/aet-3200-red/`.

**Applied** (6 of 8):
- **R2:** `GeneratorReport`'s `rule` and `evidence`, which were copies of the attribution's. The CLI writes them from the attribution.
- **R4:** four hand-written refusals now go through `mismatch`.
- **R5:** a vector collected only to take its first element.
- **R6:** a hand-rolled length-and-zip comparison of the kind 15 records, now `Iterator::eq`.
- **R7:** the seeded test's `aethelgard` now calls `aethelgard_in`.
- **R8:** inline JSON reads now go through `read_json`.

**Proof:** each was proved green, and after all six, a mutant of R6's guard still fails `propose_refuses_a_run_replay_refuses_or_a_topology_the_run_did_not_have`.

**Rejected by me:**
- **R1** moved the cause string into `aethelgard-chaos` beside `HELD_LOW` and pointed `cli/src/analog.rs` at it. Both are read-only under §2's fence. The string stays in evolve's `PAST_CEILING_CAUSE` and in `analog.rs`. A tier-1 test reads it back from a campaign run, so the two cannot drift unseen. **Finding:** one home for it, next to `HELD_LOW`, is a one-line change for the next work order that owns `chaos`.
- **R3,** a shared topology-hash helper, was net +3 and touched `campaign`'s `run_one`.

**Findings, not changed:**
- `View::Rise`'s `net`, `rise_ps` and `ceiling_ps`, `View::StuckHigh` and `View::BusFault.nets` have no production reader. They are the read-only view §8 asks for, so they stay.
- The "boot unmet" predicate is written in four places. A helper would be about line-neutral.
- `ChangeLine`, `SizingLine` and `EventLine` mirror evolve types, because evolve has no serde.
- The test helpers `names`, `hex_of` and `sha_hex` exist in two test crates.


## Merge of `main` after `AET-WO-1203`

The architect reported `AET-WO-1203` merged as `402fa6f`, and directed: merge `origin/main` with a merge commit, replace the interim floor, and note the seeded case's new name.
- **The history.** Nothing had been pushed, so the branch was rebuilt and no commit on it carries the interim floor:
  - `cefbdeb`, the evolve work, on `bf869b4`;
  - the merge commit `e1317ab`;
  - `d9e561f`, the CLI, tests and docs, with the real call.

  Until the merge, the CLI's floor was a labelled `AET-3200-INTERIM` stand-in. It was never pushed, and it lives only in the local ref `backup/aet-3200-pre-merge`. `git log wo/aet-3200-propose -S AET-3200-INTERIM` is empty.
- **The floor, one definition.** `propose` calls `aethelgard_ir::electrical::i2c_pull_up_floor_ohms(speed_hz, domain_millivolts)` with the bus's declared `speed_hz` and its rail. `solve`'s `i2c_electrical` passes the same two values (the bus's `speed_hz` and its domain's `nominal_millivolts`, which is `analog_lines`' `domain_millivolts`). Neither evolve nor the CLI keeps a copy.
  - The stand-in also differed from the merged function at or below 2 V, where the merged one uses a 2 mA sink. The swap mattered.
  - **The boundary is tested** on adversarial B1's specification (400 kHz, 120 ns ceilings): `solve` takes 967 Ω, `proposal.json`'s `rp_min_ohms`, and refuses 966 Ω with `PullUpBelowSinkFloor`.
- **The seeded case is now `cap-390p-sda-2k2`** (`AET-WO-1203` §1.3). `want()` names it, and it still gives a below-floor refusal (F1). Its SDA rise, measured here for the first time (1203 scaled it), is 726,978 ps, against 1203's scaled 726.97 ns.
- **A run whose specification `solve` now refuses is refused by `propose`,** which re-derives the topology before anything is written. The old 3100 probe run of `cap-470p-sda-2k2` is refused with `BusCapacitanceOverLimit`, exit 2.
- **Only conflict:** `campaign_tier1.rs`, where both sides appended tests at the end. Both were kept.

## Merge of `main` after `AET-WO-1900`

The architect reported `AET-WO-1900` merged as `89d4fb7`, and PR #86 conflicting with `main`. The directions (`~/.cache/aet-3200-merge.txt`): merge `origin/main` with a merge commit, keep both sides exactly, and raise the tier-2 canary by this work order's tests.
- **The merge commit** is `28711f7`.
- **The one conflict** was in `campaign_tier1.rs`, where both sides appended tests after the same test (1900's `FirmwareChanged` test; this work order's §8). Both blocks were kept as written.
  - 1900's change to `replaying_a_missing_directory_is_refused`, which now expects `Io`, merged without conflict.
  - Checked by diff: against `main` the file only gains this work order's lines, and against this branch's previous head it differs by 1900's lines alone.
- **`ci.yml`:**
  - 1900's 90-minute `tools` timeout is kept (it answers F17), and F18's `--no-verify` line merged cleanly.
  - **`TIER2_MIN` goes from 64 to 67,** with a comment naming this work order.
- **Why 67, not 66.** The canary sums the passing tests of seven test binaries. 1900 pinned it to exactly the 64 `#[test]` in those seven files, with zero slack (1900 FEEDBACK, adversarial 1).
  - This work order adds three `#[test]` to one of them, `attribute_seeded.rs`. Two are tier-2: `p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says` and `every_committed_proposal_reruns_to_its_own_bytes_and_regates`. The third is a tier-1 test the canary counts too, because it is in the same binary: `every_committed_proposal_is_pull_up_changes_named_for_its_scenario`.
  - Counting gives 67 on the merged tree: 23 + 14 + 5 + 5 + 12 + 6 + 2. A floor of 66 would leave one test of slack, so a dropped test would pass unseen.
  - **Decision-needed (ratify):** whether "the number of tier-2 tests your PR adds" meant the two tier-2 tests or the canary's three.
- **Measured on the merged tree:**
  - fmt and both clippies (with and without `with-tools`) are clean;
  - tier 1 on Furnace: 858 passed, 0 failed;
  - the CI `tools` job's steps in the image, in CI's layout: **933 passed, 0 failed,** in 58 suites;
  - the canary's own `awk`, run on that log, counts **67**, the new floor exactly;
  - the caps read back as `memory.max` 17179869184, `memory.swap.max` 0 and `pids.max` `max`.

## Gates

All on the merged tree.
- **On Furnace:**
  - `gates`' steps from `ci.yml`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (no `with-tools`), the bridge's fmt, and the two example validations. All clean.
  - The package step, verbatim from `ci.yml`: exit 0. Every crate packages, and the CLI built offline from the packages validates the example. F18 covers the step before the change.
  - Clippy with `with-tools` is clean too.
- **In the image, the CI `tools` job's steps** in its own layout, offline: the bridge's tests, `fmt`, `clippy` with `with-tools`, then `cargo test --workspace --features aethelgard-cli/with-tools`.
  - **920 passed, 0 failed,** in 57 suites, tier 2 included, on `d9e561f`.
  - The caps were read back: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` `max`.
  - Before the merge, at `021a1d2`: 909 passed, 0 failed.
- **Determinism of the committed run** (F11): the 11 unmutated seeded runs have identical run ids across two separate image runs.
- **CI on the PR is the witness for the head.**
- **Reduction pass (diff):** −31 net · 6 duplicates removed · 3 dead paths removed (the reduction pass), plus 1 in `outside_run` after the adversarial fix · 0 protected in scope (the test oracles kept on purpose) · 6 findings above (R1, R3, and the four not changed).
