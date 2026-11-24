# AET-WO-4400i: FEEDBACK

**The work order:** a foreign firmware hears MAVLink. The §1.1 stop is ruled; this is the build.
**Branch:** `wo/aet-4400i-foreign-hears-mavlink`, PR #396.
**Record:** `spike/4400i/RESULTS.md` holds the measurement, D1–D9 as asked, and the predictions graded. The rulings are on the bus: 18:55Z (D1–D9) and 19:13Z (kinds 38–40, the `Replay` dispatch).

## What was built, against the acceptance

| acceptance | where it holds |
|---|---|
| Sentinel-03's unmodified ArduPilot arms under `simulate --foreign` with a declared input, witnessed from its own heartbeat | D1: inside the CLI's 5 s window, ArduPilot refuses every ARM with its own `Arm: System not initialised`. That is the CLI's judgement, witnessed from its reply (`ci/foreign-judge.sh` `input-arm`, `sentinel-02-own`'s CI ELF). The arm itself is shown in the spike (`spike/4400i/out/build/arm12-s03`): Sentinel-03 through the CLI's own path, with the window lifted by the named `patches/window.patch`. It took 13 ARMs with ACK 0, and its heartbeat shows `base_mode` 209 at byte 7,383 of its transmission |
| …and replays from the trace alone | **R1:** `replay --archive` re-judges the archived run from its directory alone (`boot_tier1::an_archived_input_run_replays_from_its_trace_alone`; CI `input-archive`/`input-replay`: the same witness). **R2:** the input, read back from the trace with no Aethelgard code (`py/archive.py`), re-drives Renode to a byte-identical `trace.cbor` and manifest (`out/build/archive`, `archive-r2`) |
| No forced arm, no skipped check, no defaults file; a planted force code and a non-MAVLink input refused by name | `InputForcesArm` and `InputNotMavlink`, exit 2, before any tool (tier 1 and CI). `InputSkipsChecks` refuses `ARMING_SKIPCHK` ≠ 0 and `ARMING_REQUIRE` = 0 (D8) |
| A rejected `PARAM_SET` reported from ArduPilot's own reply | `foreign_param_rejected`, read from its `PARAM_ERROR` (D9; CI `input-reject`, tier 1) |
| Every committed verdict unchanged, or named and ruled | No committed verdict moves. The receive request lines are on foreign boards only (D2). A run without `--input` reports as before, except for `boot.not_at_rest` (F12). The golden trace moved in its version byte alone (19 → 20; version 19's hash is kept and checked) |
| The CLAUDE.md gates and every CI guard run locally | §Gates below |

## Findings

**F1. My §1.1 proposal named kinds 30–32. Kinds 30–37 already exist** (judge stops, battery sag, recovery bound, pin devices). The three records are kinds 38–40, as content-ruled, with the correction accepted at 19:13Z.

**F2. `AET-WO-4400h` cited RM0433 "Table 121" for `usart1_rx_dma`; Rev 9 numbers it Table 124** (p. 697, the copy `dmamux1-spi-rx.tsv` cites). The mis-cite is in `spike/4400h/patches/usart1-rx-request.diff` and `sh/replay.sh`, outside this fence and left as recorded. The new table, `aethelgard-twin/data/dmamux1-usart-rx.tsv`, cites 124.

**F3. ArduPilot arms past its checks with two codes, not one.**
- MAVLink's 21196 (`common.xml`:1920, `GCS.h`:779) and ArduPilot's own 2989 (`GCS.h`:778): both set `do_arming_checks` false (`GCS_Common.cpp`:5247).
- The work order named "the force code". `InputForcesArm` refuses both, in `COMMAND_LONG` and in `COMMAND_INT`, within 0.5 of either code. ArduPilot compares within `FLT_EPSILON`, so the wider band refuses more, never less.

**F4. The trace's 4,096-byte event bound** (`archive::MAX_EVENT_BYTES`, a reader bound) refused the first archive: the input file and a capture are each about 12 KiB. Kinds 38–40 are written in ordered parts instead:
- 3 KiB of a file or capture, or 48 stimuli, per part;
- each part numbered, with the kind's count;
- replay joins them under one order rule and refuses anything else.

The reader's bound is unchanged.

**F5. What the grant did not name literally, each forced by what it did:**
- **`aethelgard-chaos` `campaign.rs`:** its record ranking is an exhaustive match, so kinds 38–40 need an arm. They are refused in a campaign run (`check_run`), with a test.
- **The chaos README:** records 38–40 and trace 20.
- **`main.rs`:**
  - `simulate`'s body carries the two flags' plumbing: `simulate_with`, the input read before any tool, the timeline file, the witness and the archive after the judge. `simulate`'s and `campaign`'s calls are unchanged.
  - The `Replay` arm is the granted branch alone.
- **The MCP surface:** derived from the CLI's flags, it gains `input` and `archive` on `simulate`. Two tests pin that list: `mcp.rs`'s unit test and `tests/mcp-tools.json`, re-blessed with exactly those two entries. `campaign --archive` was already served.
- **The CLI's and the twin's READMEs:** the input, its refusals, the rest, the witness and the replay.
- **The unit tests in `renode.rs`:** five calls take `commands`' new `None` argument.

**F6. The window's cost of an input** (for `AET-WO-4400j`):
- **At 5 s on `sentinel-02-own`,** an input run kept 5.0 MB of the 8 MiB budget against 1.5 MB without. That is mostly the firmware's own response, not the injection: about 27,000 lines of the SPI flash model, as ArduPilot stores the parameters it was sent.
- **The timeline's `Machine paused/resumed` lines** were two per byte. The foreign filter now drops them (`the_timelines_pause_lines_are_dropped_and_nothing_else`).
- **The armed 12 s run** kept 56 MB of log and peaked at 2.13–2.45 GB, against the CLI's 1.5 GiB cap (1,352–1,558‰). It is only possible with the spike's lifted bounds.

**F7. A parameter sent before `ArduPilot Ready` (5.2 s) is not in force at the arm.**
- `arm12-s03-early-params` sent the four parameters to 2.75 s, and ArduPilot refused with `Motors: Check frame class and type` and `3D Accel calibration needed`.
- Sent again after Ready (`arm12`), the board arms.
- The input author's concern, not the judge's. Named because an agent's first input will meet it.

**F8. The IDLE gaps (D3 rows here), each measured in `spike/4400i` §3:**
- Renode's USART stores no `IDLEIE` and raises no IDLE.
- Its DMA ends no stream with TCIF on a software `EN` clear (RM0433 §15.3.16).
- It keeps `dataOffset` across a re-arm, so frame 2 lands at byte 27 of the next buffer, and later frames past it.

  Every input verdict names the gap (`usart_idle_unmodelled`). `TVI-WO-1305` models these for the guarded twin.

**F9. Adversarial pass (§5), what holds and what is residual:**
- **An arm believed because it was asked for: holds.**
  - The witness reads only the bytes the firmware wrote to the port's `TDR`, through the file backend, CRC checked.
  - A heartbeat counts only if its system and component ids are no input frame's. An armed heartbeat the input carried, echoed back, never witnesses (tier 1, `an_armed_heartbeat_the_input_carried_never_witnesses_the_arm`).
  - A mutant found the "same bytes" guard redundant with the "same source" guard: same bytes means same ids. It was removed (F11).
- **Residual (a):** an input frame sourced as the vehicle (1/1) makes every real heartbeat uncountable. The witness then says not armed, which is the safe direction.
- **Residual (b):** a heartbeat embedded in the payload of a message the table does not list could be scanned as a frame, since the scanner steps over listed frames only. It needs the firmware to echo 21 contiguous attacker bytes in an unlisted message. No input message admitted reaches such a path (`PARAM_SET` ids are 16 bytes, and a rejected id comes back in `PARAM_ERROR`, which is listed and stepped over). Not fixed; named.
- **Residual (c):** `MAV_CMD_DO_MOTOR_TEST` (209) spins motors disarmed, with no arm. It is not refused (D8's list is the arm and the checks). Proposed as a row if the agent-flown case should refuse motor commands.
- **An input replay cannot reproduce: holds.**
  - Every frame starts on a quantum, and every byte is a character time after it.
  - The trace holds the file verbatim and every stimulus's exact text.
  - R2 is byte-identical.
- **A rest that is not rest: holds as far as measured.**
  - The rest is computed from the declared orientation alone (`rest::accel_at_rest`; four orientations unit-tested; the sign mutant killed).
  - Only Sentinel-03's and `sentinel-02-own`'s x-right/z-up was flown. A board declared z-down, with its hwdef's rotation, was not built.
- **The archive:** a write that fails leaves an uncommitted run directory, which `replay` refuses (`Uncommitted`), as a campaign's does.

**F10. ArduPilot 4.7.2 refuses a parameter only by an unknown name** (`PARAM_ERROR` error 1). `FRAME_CLASS` 99 and `STAT_RUNTIME` 5 are accepted and echoed (D9, recorded).

**F11. Reduction pass** (diff-sized, after the adversarial pass). Removed, each found by a mutant that survived or a key that decides nothing:
- the heartbeat's "echoed" guard (F9);
- the timeline's tie-break key (the sort is stable and the stimuli are pushed first);
- three overlapping part-order guards in replay, made one rule (kinds 38, 39, 40 in order, none skipped).

Nothing labelled `belt-and-suspenders:` was touched.

**F12. "A witness stimulus that is not at rest is named."**
- A foreign run without an input still applies the parts table's witness stimuli (D4). It now names each IMU left off rest, in `boot.not_at_rest`, which is absent when empty.
- On `sentinel-02-own`: `U5: the parts table's witness stimulus (X 0, Y 1, Z 0 g) is not at rest by its declared orientation (X 0, Y 0, Z 1 g)`.
- This adds a field to every such foreign verdict. No committed verdict holds one, and CI's positive case gains a check reading it.

**F13. Kit facts for the next builder:**
- A Renode write hook on a peripheral displaces `LogPeripheralAccess`'s write lines for it (measured, `spike/4400i` §Inputs).
- The CLI capture's first process is the reaper, not Renode.
- On Furnace, main's own `verify --against-fab` differs from the committed package in `MANIFEST.json` alone. That is its verdict line; which field differs from CI's was not read.

## Gates

PENDING: the full suite's result, the CI guards and the shard plan.

## Mutants

`spike/4400i/mutants/`: 33 mutants, one per guard this work order adds, each run against the suite it lives in. All are killed:
- **m14 survived on the first pass.** It found the redundant "echoed" guard; m13 was re-pointed at the guard that remains.
- **m34, run by hand against Renode:** the rest's data witnesses dropped from the judge. `input-arm` fails `i2c_transaction_unwitnessed`.
