# AET-WO-2309 §1.1 — predictions, written before any run

Committed before any sweep runs. Main is `28fba951`. Kinds are wire kinds (`record.rs` `FIRST_KIND` = 5):
- 10 `BootObserved`, 11 `RegisterObserved`;
- 12 `I2cCheck`, 13 `I2cWitness`, 21 `SpiCheck`, 22 `SpiWitness`;
- 24 `MotorOutputSolved`, 26 `MotorWitness`.

"Read" is kind 10's own rule: its boot word, instruction count, `pc` and brown-out word are all read or all unread. "Booted" is `check_run`'s: the boot word read as `BOOT_MAGIC`.

## The three sweeps

| Sweep | What is replayed | By what | Before | After |
|---|---|---|---|---|
| A | The committed archives: `git ls-files '*/trace.cbor'`. There are four, all under `spike/3305/out/archives` | `aethelgard replay`, main's binary, then the branch's | main's binary | the branch's |
| B | Current-version archives made once by main's CLI in the tools image, over every committed scenario group and the seeded set's mutants (`sh/measure.sh`, 3306's groups) | `aethelgard replay`, main's binary, then the branch's | main's binary | the branch's |
| C | Every trace fixture the tier-1 suite builds: each `replay_dir` and each `check_run` call `cargo test --workspace` makes | an instrumentation patch (`sh/sweep-instrument.diff`, never committed into `crates/`) that logs each call's input hash and outcome | main's source plus the patch | the branch's source plus the patch |

## Predictions

| # | Prediction |
|---|---|
| P1 | **Sweep A is vacuous.** All four archives are trace version 14. Today's reader refuses them with `UnknownVersion` before any record rule runs. Before and after: byte-identical output, `UnknownVersion` for each. Not a §7 stop, because nothing moves |
| P2 | Sweep B: 60–80 archives (3306 made 71). Every one replays with byte-identical `aethelgard replay` output under main's binary and the branch's: the same verdict, `matched`, event count, trace bytes and state-sequence hash |
| P3 | Sweep B census: every read boot holds every kind 11, 12 and 21 read (value, and for 12 and 21 the status too); every unread boot holds them all unread |
| P4 | Sweep B census: an unread boot's kinds 13 and 22 have `observed` absent, and `absent` and `unrouted` empty. A read boot's are present unless Renode's log passed the filter's budget; no group in sweep B passes it |
| P5 | Sweep B census: every read run (booted or not) holds exactly one kind 26 per kind 24; an unread run holds none. That is `main.rs` `boot_report` plus `campaign.rs` `records`: a witness is written whenever Renode ran |
| P6 | Sweep B census: the largest trace is under 64 KiB, about 4,000 times under `MAX_TRACE_BYTES` |
| P7 | Sweep C: every (input hash, outcome) pair main's suite logs is logged by the branch's suite with the same outcome. The pairs only the branch logs are the new tests' fixtures |
| P8 | The one read run that keeps unread witnesses, by design: a read boot past the filter's budget (`renode.rs` L2612, `campaign_tier2` `flood`). It replays after the change, so rule (3) can bind kinds 13 and 22 one way only: unread boot ⇒ unread witness |
| P9 | The writer's test at the real bound writes about 256 MiB under `CARGO_TARGET_TMPDIR` (on disk) in under 10 s in a debug build |
| P10 | Each new guard has one mutant, and the new tests kill each one by the guard's own named refusal |

## The readings this work order needs, stated before building

- **R1 (fence):** rules (2) and (3) are archive rules, so they go in `campaign.rs`'s `check_run` and `check_motor_joins`, beside the "at most one" they tighten. The scenario format is not touched. `Replayer` is also driven by the writer and by non-campaign streams (golden's every-kind trace), so a rule there would refuse those.
- **R2 (rule 3, kinds 13 and 22):** an unread boot implies an unread witness. A read boot may hold an unread witness, because the judge writes one when the log passed the filter's budget (P8). Kinds 10, 11, 12 and 21 are bound both ways.
- **R3 (rule 2):** "booted" as the work order says. P5 predicts that "read" would also hold, which would be stricter than asked; that is recorded, not built.
- **R4 (rule 1):** a new `ArchiveError` variant. `TraceTooLarge`'s text says "the manifest declares", which is the reader's case. The writer refuses an event that would take the trace past the bound; a trace of exactly `MAX_TRACE_BYTES` is accepted, as the reader accepts it.
