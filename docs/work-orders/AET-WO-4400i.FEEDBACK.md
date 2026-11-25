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
| Every committed verdict unchanged, or named and ruled | **One moves, in hashes only: §7's stop, F16.** The bench package's `verify` verdict moves, so its committed `MANIFEST.json` no longer matches. The receive request lines are on foreign boards only (D2). A run without `--input` reports as before, except for `boot.not_at_rest` (F12, ruled). The golden trace moved in its version byte alone (19 → 20; version 19's hash is kept and checked) |
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
- **`workspace_shape.rs`:** it fixes the twin's file set. It now lists the three cited data tables (`TWIN_DATA`) and the two ArduPilot capture fixtures (`TWIN_FIXTURES`), each by name.

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

**F14. Found in the review before the push, each fixed with a test and a mutant:**
- **A root write into the tool's scratch after it was handed over.**
  - The timeline file and the capture directory were made after Renode's first invocation (`--version`). By then the CLI, as root under CI's uid boundary, has handed the scratch to the tool uid, and "root acts there by no path" (`run.rs` `hand_over`).
  - Under the boundary that was a root path write where the tool could have planted a link.
  - Both are now made as soon as the input is declared, before any tool, and sealed with the rest. The capture is read with `firmware::read_beneath` (never through a link, bounded) as the generated path reads its own.
  - Furnace cannot run the boundary: rootless podman gives `uid_boundary_unmet: user_namespace`. The CI cases were run as container root (`img.sh` `ROOT=1`), and they pass. The boundary itself is CI's first test.
- **`ARMING_REQUIRE` 0.5 removed the checks past the guard.**
  - ArduPilot stores an integer parameter as the value plus 0.01 away from zero, truncated (`AP_Param::set_float`, `AP_Param.cpp`:2234–2263). So ±0.5 is stored as 0.
  - `InputSkipsChecks` now judges the integer ArduPilot would store (`mavlink::stored`): `ARMING_REQUIRE` refused when it would be 0, `ARMING_SKIPCHK` when it would not be.
  - The pymavlink vectors at ±0.5 and the 0.995 boundary are tested; mutants m36 and m37.
- **`replay`'s `verdict` was the input's witness, not the run's.** The trace holds the input, the stimuli and the transmission, not the run's other findings, so a run that failed on a fault replayed as `pass`. The field is now `input_verdict` (ruled 2026-11-24, item 5).
  - The output says what it is (`input_verdict_is`), and so does `replay --help`, which the MCP tool list's description carries.
  - The 0.5 → 0 case is its own named test, `arming_require_one_half_is_stored_as_zero_and_refused` (item 4).
- **The archive header named only `renode` and its version.** It now names the engine and the Renode build, as a campaign's header does (`AET-WO-2205` §1.1). Replay prints the artifacts and the tools.
- **A capture past `MAX_FOREIGN_BYTES` would have been archived, and then refused by replay.** It is now `foreign_capture_unreadable` and is not archived, as a missing capture is.
- **Refusals that had no test:**
  - `MalformedMessageTable` and `MalformedRestTable` (the parsers now take their text, as `dma_rows` does);
  - `RestNoiseUndeclared` (`rest::stimuli_with`, with the noise table emptied);
  - `foreign_capture_unreadable`;
  - replay's part order, early count, other-port and port-versus-record reasons;
  - the chaos records' label and spacing checks.

  Still untested, named:
  - `RestUnmodelled`: unreachable with today's models; each IMU model has all three axes, and the BMP581 has pressure;
  - replay's over-bounds reason: needs an archive over 1 MiB, which the writer no longer makes.

**F15. CI.**
- `ardupilot-image.yml` runs `ci/foreign-judge.sh` on this PR (paths `crates/**` and the script).
- The ELF its added cases were checked against locally (`5eace4ca`) is main `29b3a97d`'s, from before `AET-WO-4410` changed sentinel-02's hwdef. CI builds its own, so CI's run is their first on that ELF.
- The mid-script `run_spec=` lines are per-call prefixes, so the added block sees the script's own `spec`, `platform`, `elf` and `window`.

**F16. STOP (§7, "a committed verdict moves"): the trace version moves the bench package's verdict.**
- CI's `bench-package` on 902d4917 fails `against_differs`, with `MANIFEST.json` alone differing.
- Diffed against main's last green run (`45456542716` on 352a6813), `verify`'s verdict differs in exactly the five campaign runs' `state_sequence_hash`:
  - pullup-3k3 `7ec61f16…` → `a4c18827…`
  - droop-bor3 `4fcf5f63…` → `2d2c6caf…`
  - spi-absent-u4 `fa5380e1…` → `904be070…`
  - pullup-2k2 `d437e01d…` → `dd7667ef…`
  - serial-rx-open-console `9059924e…` → `0f7429b7…`
- Nothing else differs: every gate, finding, file and outcome is the same.
- The cause is trace 20: the state hash chain starts from the header, whose version byte moved, as the golden state hash did (`tests/golden.rs`).
- My §2.3 comparison measured the request line, not the version bump, so the claim that no committed verdict moves was wrong.
- **Asked:** the bench package re-derived on this branch (`bench-package`'s write dispatch), and its `MANIFEST.json` committed. `fab/` is outside this fence.

**F16, ruled 2026-11-24T21:38Z: the move is accepted.**
- A trace version bump moves every committed campaign state hash, by design.
- The order is serialized: #390 (4300k) merged first (1b110f35), and main was merged here (12ec62f0).
- The architect dispatched `write` on 12ec62f0 (run `45514834808`, every job green, shard 11 included).
- Both packages are committed byte for byte from that run's artifacts: `bench-package` as `fab/sentinel-01-bench/package`, `sentinel-03-package` as `fab/sentinel-03/package`.
- **What moved,** checked by `spike/4400i/py/package_diff.py` against main's own CI at 1b110f35 (run `45509862564`), package and verify verdict both:
  - **Files:** in each package, every file but `MANIFEST.json` is byte-identical, and in `MANIFEST.json` only `verdict_sha256` moves:
    - Sentinel-01: `33bd1714…` → `395a35c6…`;
    - Sentinel-03: `429c62c9…` → `ed3ce866…`.
  - **The verdict:** the only fields that differ are the campaign runs' `state_sequence_hash` (Sentinel-01's 5, Sentinel-03's 11), the fab block's hash of the `MANIFEST.json` they move, and three mode fields.
  - **The mode fields** say which mode ran (main's CI runs `--against-fab`, the dispatch `--fab-out`), not what it found: `against_fab`, `fab.status`, `fab.fabrication_ready`.
  - Main's state hashes at 1b110f35 are not §F16's above (352a6813's): #390 moved them first. The comparison is against the main this branch merged.
- Sentinel-01's `fab/sentinel-01-bench/ORDERED.sha256` checks before and after the dispatch: 10 of 10 ordered files OK, so its copper and Gerbers are byte-identical.
- **`bench-package-s03`'s peak RSS:** 14.72 GiB (`kani-driver`) on a runner with 15 GiB. 4300k's was 14.73 GiB. It did not die, but the margin is under 0.3 GiB.

**F17. CI's shard 11 on 902d4917.**
- `twin_tier2::bor_recovery::the_bmp581_extension_recovers_after_its_settle` failed with `wall_bound_exceeded`, then `PlantedEventUnplaced`: its undisturbed boot ran past the 30 s wall-clock bound on a loaded runner.
- That is 2314's cause naming, working as built. The test is untouched by this branch, and it passed in the local full run.
- Ruled not this work order's: the architect files the 30 s wall bound as a row (it also hit 4300k's `d1`). It gets no re-run; the dispatch commit re-runs CI.

**F13. Kit facts for the next builder:**
- A Renode write hook on a peripheral displaces `LogPeripheralAccess`'s write lines for it (measured, `spike/4400i` §Inputs).
- The CLI capture's first process is the reaper, not Renode.
- On Furnace, main's own `verify --against-fab` differs from the committed package in `MANIFEST.json` alone. That is its verdict line; which field differs from CI's was not read.

## Gates

Run locally in the pinned tools image, on the branch with main `2cbbb354` merged:
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- **`cargo test --workspace --no-fail-fast`:** 3,090 passed. One failed, `workspace_shape::twin_has_only_its_known_files_keys_and_dependencies`, on the twin's new files. It is fixed (F5).
- **After the review's fixes (F14),** the three changed crates (`aethelgard-twin`, `-chaos`, `-cli`) were run again in full: 1,614 passed, 0 failed.
- **CI guards:** `ci/route.sh check ci/docs-tests.tsv`, `ci/test-route.sh` and `ci/test-shard.sh` pass. `ci/shard.sh build` and `ci/shard.sh plan` in the image: 12 shards, guard ok, 3,349 test names.
- **`ci.yml`'s package step,** run in the image with the network, `jq` replaced by an equivalent Python line: every crate packages with every include target inside, under 10 MiB, and the CLI builds from the packages alone, offline. The examples validate.
- **`ci/foreign-judge.sh`:** its added cases, run on `sentinel-02-own`'s CI ELF (`5eace4ca`, F15) with a trimmed copy that keeps the script's setup and positive case. All pass, also as container root (`uid_boundary` unmet on Furnace: `user_namespace`). The other existing cases need CI's `noimu`, `four-port` and `udf` ELFs; they run in CI.

## Mutants

`spike/4400i/mutants/`: one mutant per guard this work order adds, each run against the suite it lives in.
- **`run.sh` over `mutants.tsv`:** 41 mutants, all killed (`result-final.txt`). Two needed attention:
  - **m14** survived the first pass. It found the redundant "echoed" guard, which was removed with its mutant; m13 was re-pointed at the guard that remains.
  - **m37** survived until the 0.995 boundary was tested.
- **m34**, run by hand against Renode: the rest's data witnesses dropped from the judge. `input-arm` fails `i2c_transaction_unwitnessed`.
