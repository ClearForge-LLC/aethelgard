# AET-WO-2300 — FEEDBACK

**Branch:** `wo/aet-2300-campaign` · **Base:** `ad3321c` (`origin/main`, as the kickoff set it). `AET-WO-2203` merged first, at `fc8f9b5`, and `origin/main` is merged in with a merge commit (see "Merge of `main` after `AET-WO-2203`", at the end). Every number below is measured on the merged tree · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace. Two Sonnet subagents wrote tests from fixed commits in their own worktrees: `aethelgard-chaos/tests/campaign.rs` (69 tests) and `aethelgard-cli/tests/campaign_tier1.rs` (27 tests). The lead re-ran both and read the held-low, size-bound and float tests. A fresh Opus subagent ran the full adversarial pass on `1e26985`, in its own worktree, image container and caches. · **Cadence:** build-only. No §7 condition fired.

**Where everything ran.** The pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:2df68259…` (`container/IMAGE`) under rootless podman, started with `podman run --memory 12g --memory-swap 12g`. The cap was read back from the container's own scope on every run: `memory.max` = 12884901888 (12 GiB); `memory.swap.max` = 12884901888, as 2200 F13 found. Renode 1.17.0, libngspice 44.2, Rust 1.99.1. Tier 1 also ran on the Furnace host. Nothing was installed; no `sudo`, system package or secret.

**§7, held:**
- The archive's profile carries every record with integers, byte strings, text strings and arrays. No float and no map was needed, and the profile is unchanged.
- `simulate --firmware`'s behaviour is unchanged. Its pre-pass gained one parameter, the lines a campaign scenario holds low, and `simulate` passes none (F6). Every `boot_tier1`, `twin_tier2` and bridge test passes unmodified.
- No test was weakened. Every test that changed is listed under "Tests changed", with the reason.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## §3, acceptance: tier 2

`crates/aethelgard-cli/tests/campaign_tier2.rs`, in the image, with the real bridge beside the CLI, the real Renode, and the reference firmware built offline. Five tests, all passing:

| §3 | Test | Measured |
|---|---|---|
| 1 | `p2_gate_clause_2_the_campaign_passes_2k2_and_fails_47k_as_each_expects` | exit 0. `pullup-2k2`: `pass`, expected `pass`, `matched`. `pullup-47k`: `fail` for exactly `analog_rise_exceeds_ceiling` and `i2c_timeout`, expected so, `matched`. Summary `matched: 2` |
| 1 | `p2_gate_clause_2_the_47k_trace_shows_the_rise_the_fault_and_the_timeout_as_three_events` | read back through `replay_dir`: two kind-7 records, `rise_30_70_ps` **3,982,301** on both lines (the bridge's 3,982.300059 ns, rounded up), `ceiling_ps` 300,000, `complete` and `exceeds`; then one kind-9 `bridge.fault_state` on `SENSORS`, nets `I2C_SCL` and `I2C_SDA`; then U2's kind-12 identity read with status `timeout_txis`. Separate events, in that order. The verdict counts `analog_rise_exceeds_ceiling` ×2 and `i2c_timeout` ×1 |
| 2 | `p2_gate_clause_3_replay_twice_is_identical_and_equals_the_committed_hash` | two `replay` calls print identical bytes, exit 0. Their hash equals the manifest's and the campaign line's: `0483991c…076828fb8` |
| 2 | `p2_gate_clause_3_a_one_byte_change_is_refused_even_with_the_manifest_resealed` | one flipped byte: `HashMismatch`, exit 3. The same kind of change with the manifest re-sealed around the new bytes (the rise's last byte, `trace_bytes` and `trace_sha256` recomputed, the committed hash kept): `ReplayDiverged`, exit 3, from the recomputed state-sequence hash |
| 3 | `p2_gate_clause_3_the_same_campaign_twice_gives_identical_hashes_per_scenario` | a second campaign into a second archive: the same run ids, the same hashes (`fcc9aa20…` and `0483991c…`), and byte-identical traces |

Wall time per campaign of two scenarios: about 6.4 s (`time`, debug build).

## `OQ-03`: trace bytes, measured on the two reference scenarios

Each scenario boots for 100 ms of virtual time (F4).

| Run | Events | `trace.cbor` | `manifest.cbor` | Trace bytes per simulated second |
|---|---|---|---|---|
| `pullup-2k2` | 15 | 907 B | 94 B | 9,070 B/s |
| `pullup-47k` | 16 | 1,335 B | 94 B | 13,350 B/s |
| **The campaign** (2 runs, 0.2 s simulated) | 31 | **2,242 B** | 188 B | **11,210 B/s** |

**Where the bytes go,** in the 47 kΩ trace:
- the header: 261 B, of which the five artifact hashes are most;
- the scenario: 105 B;
- the topology: 71 B;
- the two rises: 62 B;
- the bus fault: 322 B, of which about 260 are the `firmware_sees` text;
- the boot: 80 B;
- the seven registers: 262 B;
- the I2C read: 32 B;
- the witness: 49 B;
- the verdict: 91 B.

**What these numbers are, and what they are not.**
- They are the bytes of what a static-fault campaign records: one record per measurement, decision and witness, and full fidelity for that. Nothing is sampled or dropped.
- They do not grow with virtual time. A boot's records are read once, at its end. So "per simulated second" is a ratio at the fixed 100 ms, not a rate: at 10 ms the same run is the same bytes.
- They say nothing yet about a dynamic campaign (`AET-WO-2301`). That one records per lock-step quantum. 2101's spike log (`firmware.observed` every 10 µs quantum) would be about 100,000 events per simulated second, about 30 to 60 B each in this encoding: 3 to 6 MB/s. That is an estimate from the event shape, not a measurement. The reader's 256 MiB `MAX_TRACE_BYTES` would then hold roughly 45 to 90 s of simulated time per run.
- **Recommendation:** amend `architecture.md` §2 with the measured static figures. Keep `OQ-03` open for the dynamic case, to be measured by `AET-WO-2301`'s first quantum log. Full-fidelity recording survives the static campaign by four orders of magnitude.

## What was built

1. **`aethelgard-chaos`**
   - **Ten event kinds, 5 to 14** (`src/record.rs`), one per campaign record: the scenario, the solved topology, a rise, a stuck-high line, a bus fault, the boot, a register, an I2C read, an I2C witness, and the verdict.
     - Every field is an integer, a byte string, a text string, or an array of them. An optional value is an array of zero or one element.
     - `Record::check` holds the field rules, which the writer and the reader both apply. Among them: `exceeds` must equal `!complete || rise > ceiling`; nets and findings are sorted; `pass` holds exactly when every finding is `info`.
     - The fold keeps the records as `State::records`. A record's delta in the chain is its whole payload.
   - **The writer refuses an event over 4,096 encoded bytes.** The reader always did. No kind before this one could reach the bound, so the writer never checked.
   - **The reader opens the run directory once and reads every file through it** (`rustix`'s `openat`, `O_NOFOLLOW | O_NONBLOCK`), after the adversary raced a symlink and a FIFO past its by-name checks (A1).
   - **Trace header version 2** (F1).
   - **`src/campaign.rs`:**
     - `Scenario::parse`: strict, object-only, duplicate keys refused, closed-world overrides, bounded at 32 overrides and 16 expected kinds;
     - `Scenario::apply`: sets the field, refuses an unknown id, then runs the specification's own `validate`;
     - `run_id`;
     - `matches`;
     - `check_run`, the run rules, and `check_archived`, which `replay` adds for the header and for any event that is not a campaign record.
   - **`ArchiveError::kind`**, so the CLI can report a refusal by its variant name.
   - **README:** "Campaign records", "Scenarios", "The run rules", and the version and state-hash notes.
2. **The CLI**
   - **`campaign`** (`src/campaign.rs`). It prepares every scenario before anything runs: read, parse, resolve `base` inside the current directory, read the specification as `validate` does, apply, serialize, hash, solve. Then each scenario runs through `crate::simulate` unchanged, with the lines it holds low, at 100 ms. The verdict's records are mapped to integers, judged by `check_run`, and written to `<archive>/<run id>` and committed. Only then is the line printed. The summary line follows, exit 0 or 1.
   - **`replay`**: `replay_dir`; the directory's name must be its header's run id (`RunIdMismatch`); then `check_archived` and `check_run`; then one line. It creates no scratch directory and starts no process.
   - **`analog::pass`** takes `Held`. A held line is left out of the bridge's request, and the bridge is not run when no line is left. The held line gets an `i2c_line_held_low` finding, and a `bridge.fault_state` whose `source` is `scenario <name>` and whose cause is `scenario.line_held_low`. `FaultRecord.source` became a `String`; `simulate`'s value is still `"aethelgard"`.
   - **`solve_spec`**, split out of `solved()`, so a specification that is not a file can be solved with the same refusals.
   - **MCP** (F5). The README has a new "`campaign` and `replay`" section, with the exit-code rows and the MCP notes.
3. **`scenarios/sentinel-01/pullup-2k2.json` and `pullup-47k.json`.**
4. **Root `Cargo.toml`:** `aethelgard-chaos` in `[workspace.dependencies]`, which the CLI's new dependency needs. `docs/releasing.md`'s order already publishes `chaos` (6) after `ir` (1) and before the CLI (7), so it needs no change.

## Findings and the forks taken

- **F1 · The trace header's version is now 2, and both golden constants moved · `aethelgard-chaos` · medium · format · decision-needed (ratify).**
  - The chaos README's rule says a change to the event table bumps the version, and ten kinds were added.
  - The golden `trace.cbor` changed only in the header's version byte. Setting that byte back to 1 gives the old golden `51c6193f…a896d` exactly (checked).
  - The trace's sha256 moved from `51c6193f…a896d` to `e1d39213…3c86e8`.
  - `state_hash` moved from `af2d46bb…b4939` to `eb6408ee…693edc`, because `h0` hashes the header.
  - The mirror record keeps version 1. It has its own constant now, since it shared one with the header.
  - Manifest version 2 is unchanged: what `state_hash` means did not change.
- **F2 · "The state-sequence hash" is 0400a's chain, extended, not a new hash · `aethelgard-chaos` · medium · interpretation · decision-needed (ratify).**
  - §1.3 defines the hash as "sha256 over the canonical CBOR of the ordered event list, excluding wall-clock fields". It also says replay refuses a run whose recomputed hash differs from the one in its manifest. The manifest's `state_hash` is 0400a's chain, and a one-shot sha256 over an array could never equal it. Replacing it would re-rule 0400a.
  - So each campaign record's delta is its whole payload, and the chain is sha256 over the ordered events' canonical CBOR, linked. It holds no timestamp: those are virtual nanoseconds, and outside the chain anyway. It holds no wall-clock field, because `run_id` is a function of the inputs (F3).
  - It has every property §1.3 names. If the architect wants the literal one-shot array hash, it is a third manifest field and a manifest version 3.
- **F3 · `run_id` is a function of what the run judged · `aethelgard-chaos` `run_id` · medium · design · decision-needed (ratify).**
  - The chain starts at `h0 = sha256(header)`, and the header holds `run_id`. A random or time-based id would make §3.3, two campaigns with identical hashes, fail before the first event.
  - So `run_id` is the first 16 bytes of `sha256("aethelgard-run\0" ‖ scenario sha256 ‖ specification sha256 ‖ firmware sha256)`.
  - The cost: the same campaign twice into one archive is `RunExists`, which is the archive being append-only. `DuplicateScenario` refuses two scenarios in one campaign that would name one run.
- **F4 · Every scenario boots for a fixed 100 ms · the CLI · low · scope · decision-needed (ratify).**
  - The format has no `--for`, and the work order's command line has none.
  - The 47 kΩ case needs 100 ms: at 10 ms, U2's timed-out read does not reach the boot word (2202's finding; 2200's merged test runs at 100 ms).
  - It is `aethelgard_twin::boot::MAX_VIRTUAL_NS`, named once, printed in the summary as `virtual_time_ns`.
- **F5 · MCP needed two generator changes, both driven from the tree · `src/mcp.rs` · medium · scope · decision-needed (ratify).**
  - `leaf()` asserted that every argument is `ArgAction::Set`, so `--scenarios <file>...` would have panicked the tool list.
    - A repeatable argument (`Append`) is now an array of strings with `minItems: 1`. It becomes one `--scenarios=<v>` token per item.
    - A string, an empty array, a non-string item and a NUL are refused as invalid parameters.
  - `result()` took exactly one JSON line, and §1.2 asks for one line per scenario and then a summary.
    - A subcommand named in `main.rs`'s `LINE_PER_RECORD` (only `campaign`) may print several lines. Every one must be a JSON object; the last is `verdict`, and the others are `structuredContent.records`.
    - Every other tool keeps the one-line rule, and the existing test that refuses `{}\n{}\n` is unchanged and passing.
  - The CLI README's opening contract names `campaign` as the one exception to one line.
- **F6 · `line_held_low` reaches the pre-pass through one new parameter · `src/analog.rs`, `main.rs` `Boot` · medium · fence · decision-needed (ratify).**
  - §2 says `simulate --firmware`'s behaviour does not change, and that the campaign calls it or the same internals, unchanged. A held line has to reach the fault decision somehow.
  - `Boot` and `Ready` carry `scenario` and `held`. `simulate` sets them empty, so its request, records and platform are byte-for-byte as before.
  - "The bridge decides on it without measuring" is built as: the held line is left out of the request, and its bus is faulted by its own `bridge.fault_state`. That record is separate from any measured fault on the same bus, so `check_run` can tie each fault to its cause.
- **F7 · The scenario format adds `expect` and `expect_findings` to §1.1's four keys · `aethelgard-chaos` · low · format · decision-needed (ratify).**
  - §1.1 lists `{schema_version, name, base, overrides}`. §1.2 then needs `expect`, and §5.4 needs the finding kinds a scenario expects.
  - A failing scenario matches only when its finding kinds above `info` equal `expect_findings` exactly, so a fail for another reason, or for more reasons, is a mismatch. A passing scenario names none (`ExpectContradiction`).
  - `info` findings, such as U3's `device_unmodelled`, never count.
- **F8 · `base` must resolve inside the current directory · the CLI `read_base` · low · design · decision-needed (ratify).**
  - The reference scenarios must reach `examples/sentinel-01-minimal.json` without a copy (`INV-01`), so `..` cannot be refused outright.
  - The rule:
    - `base` is relative to the scenario file;
    - it must not be absolute;
    - once every symlink is resolved, it must be a regular file under the canonical current directory;
    - it is opened once (`O_NOFOLLOW | O_NONBLOCK`), and the open descriptor's real path (`/proc/self/fd`), its type and its bytes are what is checked and read (adversarial A2);
    - its bytes are parsed by `validate`'s parser, and their sha256 is the header's `base` artifact (A11).
  - So a campaign runs from the repository root, as CI's `-w /repo` does. Run from `/`, "inside" admits anything the user can read, which is the user's own choice (A10).
  - A generated file is not a specification: a `.repl` is `InvalidJson` and a solved topology is `SchemaViolation` (tier 1).
- **F9 · Picoseconds from the bridge's nanoseconds round up for a rise, down for a lower bound · the CLI `picoseconds` · low · design.**
  - `Record::check` requires `exceeds` to equal the integer comparison. Rounding up keeps a rise past a whole-nanosecond ceiling past it. Rounding down keeps a lower bound a lower bound.
  - That is why the 47 kΩ rise is archived as 3,982,301 ps and not 3,982,300, well inside §3.1's ±500.
  - Volts round to the nearest microvolt.
- **F10 · `aethelgard-chaos` now depends on `aethelgard-ir`, `serde` and `serde_json` · `workspace_shape.rs` · low · fence.**
  - The scenario model reads and changes a `Specification`, and the fence puts it in `chaos`. `chaos_has_only_its_known_files_keys_and_dependencies` pinned exactly `sha2` and `thiserror`. It now pins the five, with the reason in its doc comment.

## Tests changed

- **`aethelgard-chaos/tests/golden.rs`:** both constants (F1), with their history in the doc comments.
- **`aethelgard-chaos/tests/archive.rs`:**
  - the "unknown kind" fixture is 15, not 5;
  - the version test checks that 1 and 3 are refused, now that 2 is current;
  - a hand-built header uses `TRACE_VERSION`;
  - the plumbing report's exhaustive `match` gains the record arm (unreachable for the synthetic stream).
- **`aethelgard-cli/src/mcp.rs` tests and `tests/mcp_tier1.rs`:** the tool list gains `campaign` and `replay`, and the throwaway count is 10. New: the array argument and the line-per-record result.
- **`aethelgard-cli/tests/workspace_shape.rs`:** chaos's dependencies (F10).

## The fence

Inside it as written:
- `aethelgard-chaos` (`record.rs`, `campaign.rs`, `event.rs`, `replay.rs`, `archive.rs`, `lib.rs`, README, tests);
- the CLI (`campaign.rs`, `mcp.rs`, `main.rs`'s commands and `solve_spec`, README);
- `scenarios/`;
- tests;
- this file.

Edges, each named:
- **`analog.rs` and `verdict.rs`'s `FaultRecord.source`:** F6;
- **root `Cargo.toml`:** the workspace dependency;
- **`Cargo.lock`:** regenerated by cargo, never by hand.

Nothing of `AET-WO-2203`'s fence was touched. Its call sites of `parts::i2c_targets` in `main.rs` are unchanged, and the campaign calls `i2c_targets` the same way, in `campaign.rs::devices`. If 2203 merges first, that call moves with it.

## Adversarial pass (full, §5)

A fresh Opus subagent attacked `1e26985` in its own worktree. It used its own podman containers (`memory.max` read back 12884901888 on each) and its own caches under `~/.cache/aet-2300-adv/`, where its scripts, probes and archives remain. It ran the real bridge and Renode for 15 scenarios beyond the two references, and one mutant per guard this work order adds. The lead reproduced nothing by hand; each fix below has its own test, and the dispositions are the lead's.

| | Finding (measured) | Severity | Disposition |
|---|---|---|---|
| A1 | **Replay race.** Swapping `trace.cbor` for a symlink to an identical copy outside the run directory, between the entry check and the read: 32 of 200 replays read the outside file and said `ok`. Swapping `manifest.cbor` for a FIFO: 48 of 300 replays hung. §5.3, reached. | should-fix | **Fixed.** The directory is opened once with `O_DIRECTORY \| O_NOFOLLOW`. It is listed through that descriptor, and each file is opened with `openat(…, O_NOFOLLOW \| O_NONBLOCK)` and checked and read on that descriptor. Tier 1: a symlinked run directory, a symlinked trace and a FIFO manifest are refused promptly. **Re-measured by the lead with the adversary's racers, on the fixed build:** 0 of 300 replays opened the outside file (each was `UnexpectedEntry`, or read the real file), and 0 of 300 hung on the FIFO. |
| A2 | **Campaign base race.** Flipping `spec.json` between a file and a symlink outside the current directory, between the check and the read: 89 of 1,000 campaigns solved the outside specification. A FIFO hung 20 of 300. | should-fix | **Fixed** (F8): one open, `O_NOFOLLOW \| O_NONBLOCK`; the descriptor's own path, type and bytes are what is checked and read. **Re-measured with the same racer on the fixed build:** 0 of 1,000 campaigns solved the outside specification. 824 were `BaseOutsideRoot`; 176 read the inside file and stopped at its own `UnknownBus`. |
| A3 | **A run copied under another run's name** replayed `ok`, as the other run, with no forgery. | should-fix | **Fixed:** `RunIdMismatch`. |
| A4 | **Header disagreeing with records** (firmware hash zeroed, `run_id` zeroed, a 2,200 Ω override under the 47 kΩ hash), re-sealed: accepted. | should-fix | **Fixed:** `check_archived` ties the header's hashes to the records and `run_id` to the hashes. |
| A5 | **Raw kind-0 and kind-4 events** inside a campaign run, re-sealed: accepted, and not shown by `replay`. | note | **Fixed:** `check_archived` refuses any event that is not a campaign record. |
| A6 | Two traces differing only in timestamps share one state hash. | note | **By design** (0400a: timestamps are outside the chain). For the architect, with F2: §1.3's "excluding wall-clock fields" does not say virtual time is excluded. |
| A7 | Reorder, drop, float, flip, one byte, unsealed: each refused (`HashMismatch`, `Truncated`, `NotDeterministicCbor`). Re-sealed: a flipped pass bit, every finding at `info`, `exceeds` 0, a dropped rise, a fault before its rise, two verdicts, and a held-low fault without its override are each refused. Accepted when re-sealed: swapping the two rise records, and a fully consistent rewrite to pass, which then reads `matched: false`. | held | The manifest proves integrity, not authenticity (0400 residual). After A3 to A5, a re-sealed forgery must also be internally consistent. |
| A8 | Can the campaign itself misstate a result? 15 real scenarios: 1 Ω (stuck high, 3,000,000 µV), `u32::MAX` Ω and `u64::MAX` fF (lower bound 19,999,999 ps), 1 fF, SDA held, both held, SDA held at 47 kΩ, and expectation variants. Plus a brute force of the rounding for ceilings 1 to 100,000 ns. | held | Each run matched what `simulate` decided, each replayed, and no rounding counterexample was found. |
| A9 | Scenario schema: unknown or duplicate keys at both levels, `null` in an unused key, positional arrays, floats, strings, `4.7e4`, negatives, 0, overflow, version `1.0`, a BOM, trailing bytes, names, ids, 33 overrides, contradictions. | held | Each refused with its kind. A 400-digit number is `InvalidJson` (exit 3). |
| A10 | Base escapes, static: absolute paths, `..` to `/etc`, `/dev/zero`, `/proc/self/environ`, an outward symlink, a FIFO, a directory, NUL, empty, not a specification, the scenario itself. | held | Each refused. F8's note on running from `/`. A refusal from the base's parser now names the scenario. |
| A11 | The archive recorded the overridden specification's hash (`fd9146dd…`), a re-serialization, and nowhere the base file's own hash (`acd60081…`, the one `solve` prints). | should-fix | **Fixed:** the header's `base` artifact. |
| A12 | Expectations are kind sets: a fault on one line or on two, caused by 47 kΩ or by `u64::MAX` fF, all match `[analog_rise_exceeds_ceiling, i2c_timeout]`. An `info` kind can never be expected. | note | **Decision-needed** (F11 below). |
| A13 | SIGKILL during the second scenario: the first run is committed and replays; no second directory is left; no `/tmp` entry is left. A re-run is `RunExists`, with no resume. | held | — |
| A14 | MCP: wrong types are `-32602`. A flag-looking item stays one token (`--scenarios=--archive=/etc`). A real call returns `records` and the verdict. `campaign` over MCP can create an archive anywhere the server user can write, as `emit --out` already can. | note | — |
| A15 | In a container whose PID 1 never reaps, each campaign or `simulate` leaves about two defunct helpers. They fill podman's default `pids.max` of 2048. | note | F12 below. The lead met the same thing: `mcp_tier1` failed with `EAGAIN` until the runner lifted the pids limit. |
| A16 | `simulate --firmware` on the 2.2 kΩ and 47 kΩ specifications at 100 ms, this branch against `main` `ad3321c`: byte-identical JSON. | held | §2's fence holds. |
| — | Reasoned, not measured: a held-low line's platform comment, written by `twin::platform::emit_faulted`, still says "measured past its rise-time ceiling". | note | Outside the fence: F13. |

**Mutation, one mutant per guard this build adds** (the adversary's run): 64 mutants, 52 killed. Of the 12 survivors:
- **Nine were test gaps, each now closed by a test:**
  - P6: an expected kind that is not a name;
  - B1: an absolute base inside the current directory;
  - B3: a directory or FIFO base;
  - B6: one matched and one unmatched scenario;
  - B7: the rounding direction, now a unit test in `campaign.rs`;
  - R19: a trailing field;
  - R20: a flag of 2;
  - the reader-side name and field rules.
- **Three were equivalent or unreachable, and the reduction pass removed them:**
  - K8, "a passing verdict beside a bus fault": the verdict counts already force it;
  - K9 and K10, the `pass == expect_pass` and non-empty conjuncts. With `Record::check`'s rules, matching is exactly `failing_kinds == expect_findings`.

  The lead's own mutant of that one comparison (`matched = true`) is killed by four tests.

## More findings, from the adversarial pass

- **F11 · Expectations name finding kinds, not counts or nets · `aethelgard-chaos` · low · design · decision-needed.** A regression that moves a fault from one line to two, or changes its cause, still matches (A12). Options: (a) accept, since kinds are what §5.4 asks; (b) add optional `expect_counts`. Recommendation: (a) for P2. Counts belong with `AET-WO-2301`'s dynamic faults, where one scenario can fault many times.
- **F12 · Runs leave defunct helpers when PID 1 does not reap, and podman's default `pids.max` of 2048 is then reached · Furnace practice · low · measured.**
  - The lead's runner hit it as `EAGAIN` across `mcp_tier1` until it ran with `--pids-limit -1` (read back as `pids.max` = `max`). It then ran 50 suites, 740 passed.
  - GitHub's Docker sets no pids limit, which is why CI never met it.
  - The helpers are the reaper's and guardian's own children, orphaned to PID 1 by design.
  - Recommendation: Furnace's podman recipe passes `--pids-limit -1` (or an init), and the contract reads `pids.max` back beside `memory.max`.
- **F13 · The faulted platform's header comment calls a held-low bus "measured past its rise-time ceiling" · `aethelgard-twin` `emit_faulted` · low · outside the fence.** The archive and the verdict say `scenario.line_held_low` correctly; only the `.repl` comment is wrong. Recommendation: `emit_faulted` takes the cause, in a later twin work order.
- **F14 · `FirmwareChanged` has no test · the CLI · low · mutation gap.** The firmware is hashed once for the run id and the header, and the boot hashes its own copy. A file changed between the two is refused. No test can change the file mid-campaign from inside the sandboxed stub, so the check stands on review alone, as 2200's F16 did.

## Gates

- **Reduction pass (diff):** −24 net · 2 duplicates removed · 3 dead paths removed · 0 protected kept · 0 findings below.
  - **Duplicates:**
    - the analog finding kinds are now `aethelgard-chaos` constants, used by the pre-pass and the run rules alike;
    - which records are read at the boot's end is now one constant, `record::FIRST_BOOT_KIND`.
  - **Dead paths:**
    - `check_run`'s "passing verdict beside a bus fault", which the verdict counts already force (adversary's K8);
    - the `matches` function and its two conjuncts, which `Record::check`'s rules make equivalent to `failing_kinds == expect_findings` (K9, K10);
    - `Line::name` and two unused `Ord` derives.
  - **Proof:** the suites stayed green, and the one comparison left was mutated (`matched = true`): 4 tests kill it.
  - **Considered, not cut:**
    - `ScenarioLine` and `Replayed` share fields, but they are two output contracts;
    - `FirmwareChanged` is untested (F14), and kept as a guard.
- **Local, in the tools image** (`--network none`; `memory.max` 12884901888, `pids.max` `max`), at `00db795`:
  - the bridge's tests pass;
  - `cargo fmt --all -- --check` is clean;
  - `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -D warnings` is clean;
  - `cargo test --workspace --features aethelgard-cli/with-tools`: **52 suites, 775 passed, 0 failed**, tier 1 and tier 2.
- **Local, on the Furnace host** (no Renode, no libngspice, as in `gates`): `cargo test --workspace`, fmt and clippy are green.
- **CI:** the PR's checks. This branch's last push carries this file, so CI's result is on the PR, not here.

## Merge of `main` after `AET-WO-2203`

The architect approved PR #74 and ratified F1 to F8 and F11 (a). `AET-WO-2203` merged first, at `fc8f9b5`: `SolvedTopology` carries `i2c_devices`, `parts::i2c_targets` is gone, and `solve` refuses `DeviceNotPlaced` itself. `origin/main` is merged into this branch with a merge commit (`77f664f`).

- **The conflict** was in `main.rs`, at `Boot` and `Ready`. The `devices` fields are gone, as on `main`, and the campaign's `scenario` and `held` stay.
- **The `i2c_targets` uses:** the campaign's own call (`campaign.rs::devices`) and its `DeviceNotPlaced` refusal are deleted. `platform::emit(&topology)` reads the devices from the topology, and `simulate` gets them there too, so nothing names `i2c_targets` or `I2cTarget` (2203's `the_interim_derivation_is_gone` passes).
- **A rule of 2203's met one of this work order's.**
  - 2203's guard `nothing_outside_solve_reads_devices_from_a_specification` forbids `aethelgard-chaos` naming `Specification` (`RUL-16`), and `Scenario::apply` did.
  - The guard was not narrowed. `apply` moved to the runner, in the CLI's `campaign.rs`, with its six tests as that module's unit tests, unchanged but for the call form.
  - `aethelgard-chaos` keeps the scenario format, the records and the run rules, and no longer depends on `aethelgard-ir`. `workspace_shape.rs` pins `rustix`, `serde`, `serde_json`, `sha2` and `thiserror`.
  - F10's dependency list is superseded accordingly.
- **Goldens, regenerated by the tools and compared:** none in this branch moved through the merge.
  - The chaos golden trace and state hash (F1) pass unchanged.
  - The solve digests and the board regeneration are `main`'s own, regenerated by 2203, and pass.
  - `Cargo.lock` was regenerated by cargo (the `aethelgard-ir` edge from chaos removed), never by hand.
- **The campaign's measured values moved, and the tier-2 tests read them rather than pin them:**
  - the state-sequence hashes are now `fcc9aa20…` (2.2 kΩ) and `0483991c…` (47 kΩ), because the solved topology's bytes now include `i2c_devices`, and because the header gained A11's `base` artifact;
  - each trace grew by 40 B, exactly that artifact;
  - run ids are unchanged (`ebc69f79…`, `011cc38d…`), since the specification and the firmware are;
  - the rise is still 3,982,301 ps.
- **Gates on the merged tree:**
  - the Furnace host: 52 suites, 720 passed; fmt and clippy are clean;
  - the tools image (`memory.max` 12884901888, `pids.max` `max`, `--network none`) on `77f664f`: the bridge's tests, fmt and clippy with tier 2 are clean, and **54 suites, 787 passed, 0 failed**.
