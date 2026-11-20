# AET-WO-1213 FEEDBACK — `validate` refuses a console baud that ArduPilot cannot open at boot as declared

Builder: Claude (Opus 5.5), a fresh `1213` session, 2026-11-20, from ~17:40 UTC. Worktree `~/aethelgard-1213`, branch `wo/aet-1213-console-baud` from `origin/main` `c1012a2d`. The work order is read from `~/.cache/aet-1213-wo.md`, the architect's copy; it is byte-identical to `docs/work-orders/AET-WO-1213.md` as `main` `69c77c96` (#296) holds it, merged in here.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. The source, read at the pinned tag

`libraries/AP_SerialManager/AP_SerialManager.cpp` at `Copter-4.7.2`, fetched from `raw.githubusercontent.com` into `~/.cache/aet-1213/ap/`: sha256 `fe7cfce006fcef8e47388e965f17c4758da54613b67002877122dfec27256d9b`, byte-identical to `4405`'s copy [measured]. These are the lines the rule cites [read]:

- `:40-41`: `DEFAULT_SERIAL0_BAUD` defaults to `AP_SERIALMANAGER_CONSOLE_BAUD`.
- `:172`: `AP_GROUPINFO("0_BAUD", …, DEFAULT_SERIAL0_BAUD/1000)`. Only SERIAL0 stores its define divided.
- `:199`, `:213`, `:227`, `:241`, `:255`, `:271`, `:352`, `:371`, `:390`: `1_BAUD` to `9_BAUD` store `DEFAULT_SERIALn_BAUD` as it stands. Only when no define is emitted does the `#ifndef` default (`…_BAUD/1000`, `:47-162`) apply, and the generator emits one for every non-RC port.
- `:413-424`: `init_console`, whose `begin` at `:419` opens `hal.serial(0)` at `DEFAULT_SERIAL0_BAUD`. It opens no other port.
- `:737-770`: `map_baudrate`.

## 1. The measurement (§1.1), predictions first

The predictions are committed before any run: `spike/1213/PREDICTIONS.md`, `0e02f43d`. The harness is `spike/1213/measure.py`, an independent transcription of `:737-770` in Python, run over every tracked `.json`. Its output is `spike/1213/measure.out`, `68a31ff2` [measured]:

- **587 tracked JSON files.** 13 are not JSON (fixtures that are malformed on purpose, and campaign logs). None of those 13 holds `serial_ports`.
- **Every serial port found, 42 in all:** each specification's list and each solved topology's map.
  - `console`: **34 ports** (24 in specifications, 10 in solved topologies), at **115200** (32) and **9600** (2, `serial-dshot-9600` and its topology). **Every one round-trips: 0 refused.**
  - `mavlink` 57600, `gps` 115200, `rc_crsf` 420000, `rc_sbus` 100000.
- **Prediction 1 (all pass): held.**
- **Prediction 2 (console only):** held at source (§0). `rc_crsf` and `rc_sbus` emit no baud define (hwdef F3, PR #189). `mavlink` and `gps` store their define unchanged, so any baud above 2000 maps to itself.
- **Prediction 3, a finding against the work order's text (F1 below):** the WO's examples, 100000 and 250000, both round-trip. `100` is a listed code (`:750`), and 250 is kbaud (`:769`). The refused examples are bauds that really fail: 76800 → 76000, 111000 → 111100, 115000 → 115200, 3000000 → 3000, 2001000 → 2001, 999 → 57600.

**The real binary agrees** [measured, `~/.cache/aet-1213/accept.tsv`]. `aethelgard validate` from `main` (`c1012a2d`, built from `git archive`) and from this branch ran over all 45 tracked files whose top level holds a `serial_ports` list. Each pair printed the same bytes and exited the same way:
- 28 pass;
- `examples/invalid-zero-domain.json` is `ZeroNominalVoltage` on both;
- 16 stale spike specifications are refused on both by schema: 14 by `UnsupportedSchemaVersion` and 2 by `SchemaViolation`. Fifteen hold a console, each at 115200 by the measurement; `spike/3306/spec/bme-bmp.json` declares no serial port.

§6a repeats this against `main` `25203c58` after the merge: 47 files, every pair identical.

**Specifications built in Rust tests:** two pre-existing tests built a console at a baud no `BRR` reaches, to test `SerialBaudOutOfRange`. Both flipped to `ConsoleBaudNotRoundTrip` [measured]:
- `ir/tests/serial.rs::a_baud_no_brr_reaches_is_refused_by_name` (900 and 4,200,000);
- `cli/tests/reserved_references_tier1.rs::aet_1208_…` pin `serial-brr` (300).

Each now builds that port as `mavlink`, so it still tests what it tested, and its expected bytes are unchanged. The rule was not narrowed. No other test changed.

## 2. What is built

- **`aethelgard_ir::serial::map_baudrate(i32) -> u32`:** ArduPilot's types, transcribed line by line with each arm cited (`:739-741`, `:742-760`, `:762-766`, `:768-769`). It is labelled `belt-and-suspenders:` as the third copy, beside `hwdef`'s and `pincheck`'s, all `RUL-20`'s pair by design.
- **`serial::console_mapped(baud) -> Option<u32>`:** what `SERIAL0_BAUD` maps back to when that is not the declared baud.
- **`IrValidationError::ConsoleBaudNotRoundTrip { id, baud, mapped }`:** the last check of step 13c, so every earlier refusal keeps its precedence (a zero baud is still `SerialFramingShape`). Its message names the port, the declared baud, the mapped one and the cited lines.
- **Roles covered: `console` only** (§0). It is the one role the generator places at SERIAL0, and the one port `init_console` opens from its define.
- **`SolvedTopology::validate` re-checks it** as it does every sibling 13c rule: `SerialPortMismatch`, whose message now lists it. That goes past the WO's word `validate`, and is F2 below.
- **IR README:** the 13c row lists the rule and its kind.
- **Tests:**
  - `ir/tests/serial.rs::a_console_baud_serial0_cannot_hold_is_refused_by_name`:
    - 115200, 9600, 57600, 100000, 250000 and 2000000 pass;
    - six failing bauds are each refused with the mapped value;
    - each passes on `mavlink` and `gps`;
    - 0 is still a shape error.
  - `ir/tests/serial.rs::the_topology_refuses_a_console_baud_serial0_cannot_hold`: a solved console moved to 76800, with `BRR`, actual baud and error recomputed, is a mismatch; 250000 is not.
  - `cli/tests/silicon_rules.rs::the_ir_s_map_baudrate_is_pincheck_s`:
    - the cross-check against `pincheck`'s copy, over every listed code, the `int32_t` extremes, and -1000 to 4,300,000 (every `b / 1000` of a `u32`);
    - `console_mapped` against `pincheck`'s map over every baud from 0 to 5,000,000, plus the `u32` extremes.

    It lives in `aethelgard-cli`, which depends on both crates; `aethelgard-ir` may depend on no member.

## 3. Acceptance (§2)

- **The measurement is §1. No committed specification is refused** [measured, by the Python transcription and by the binary].
- **Gates:** §6. **`TIER2_MIN` is unchanged:** no tier-2 test was added or removed.

## 4. The adversarial pass, targeted (§5)

| # | Probe | Result |
|---|---|---|
| A1 | **The two copies disagreeing on any input.** A scratch release crate (`~/.cache/aet-1213/exh`, path dependencies on this branch's `ir` and `pincheck`) ran both over **every `int32_t`**, and `console_mapped` against `pincheck`'s map of `b / 1000` over **every `u32`** | **Agree on all 4,294,967,296 inputs, both sweeps** [measured]. Exactly **2000** `u32` bauds are holdable on the console, one for each `k` in 1..=2000. |
| A2 | **A declared baud the generator rewrites.** `hwdef` emits `define DEFAULT_SERIAL{n}_BAUD {framing.baud}` verbatim (`hwdef/src/lib.rs:694`) [read]. `solve` copies the declared framing (`solve.rs:613`). No scenario override touches a baud: `campaign.rs:150-176` lists every override key [read] | No rewrite: the baud the rule judges is the define ArduPilot reads. |
| A3 | **A baud source that bypasses `validate`: a hand-edited solved topology.** The console of `sentinel-01`'s topology was moved to 76800, with `BRR`, actual baud and error recomputed so that no other check trips, then run through `generate hwdef --topology` | `main`: exit 2, `ConsoleBaud`, the generator's own refusal. Branch: exit 2, `InvalidTopology` (`SerialPortMismatch`) before the generator runs [measured]. 250000 is emitted on both. |
| A4 | **The late refusal on `main`.** `sentinel-01` with its console at 76800 | `main`: `validate` exit 0 and **`generate` exit 0**. `generate` writes the board and the firmware, and only the separate `generate hwdef` refuses. Branch: `validate`, `solve` and `generate` all exit 2 with `ConsoleBaudNotRoundTrip` [measured]. |
| A5 | **Edges of the type.** A console at 4294967295 (`b / 1000` = 4,294,967, inside `int32_t`); at 4294967296; with role `Console` | `ConsoleBaudNotRoundTrip`, with no panic; `SchemaViolation` (not a `u32`); `SchemaViolation` (unknown variant) [measured]. |
| A6 | **Two consoles.** The rule is per port, so each console is judged. `hwdef` refuses two (`ConsoleTwice`) | Held by construction [read]. |

## 5. Mutants (§1.3), one per guard this work order adds

The harness is `~/.cache/aet-1213/mut/mut.py`, the results `results.txt`; it ran on `c830681e`, and again on the merged head before the last push (`results-final.txt`), with the same result. Source is restored by rewriting the original bytes [measured]. Suites: `ir --test serial`, and `cli --test silicon_rules --test reserved_references_tier1`. **5 of 5 killed.**

| Mutant | Killed by |
|---|---|
| M1 `validate` does not call the rule (the WO's "drop the rule") | `a_console_baud_serial0_cannot_hold_is_refused_by_name` |
| M2 `validate_solved` does not re-check it | `the_topology_refuses_a_console_baud_serial0_cannot_hold` |
| M3 the rule holds every role | `a_console_baud_…`, `a_baud_no_brr_reaches_…`, `a_crsf_port_s_baud_…`, `aet_1208_…` |
| M4 `console_mapped` maps `b`, not `b / 1000` | `a_console_baud_…`, `the_topology_refuses_…`, `the_ir_s_map_baudrate_is_pincheck_s` |
| M5 `map_baudrate`'s code 111 falls through to kbaud | `a_console_baud_…`, `the_ir_s_map_baudrate_is_pincheck_s` |

## 6. Gates and the reduction pass

- **Gates on the merged tree `a6c8a0d4`** (`main` `25203c58` merged in), run under `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, log `~/.cache/aet-1213/gates-1.log` [measured]:
  - `cargo fmt --all -- --check`: exit 0;
  - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0;
  - `cargo test --workspace`: exit 0, 150 suites, 2,548 passed, 0 failed.

  Every tier-2 test self-skips here, and CI is the run of record. `bddc2e2f` after it adds only `spike/1213/measure-merged.out`.
- **Reduction pass (diff): −2 net · 1 duplicate removed · 0 dead paths removed · 1 protected kept · 2 findings (F3 and R2 below).** It ran through a fresh read-only subagent on `git diff c1012a2d..48cd755d^ -- crates`.
  - **Removed** (`48cd755d`): `serial::SERIAL_MANAGER_SOURCE`, a public const read only by one doc link. Its file, tag and sha are now in that doc comment.
  - **Protected and kept:** the IR's `map_baudrate`, labelled `belt-and-suspenders:` (`RUL-20`).
  - **Declined (R2):** dropping the test's `(kind(), id())` assertion beside its full-variant `assert_eq!`. The kind string is what the CLI prints, and no test enumerates every variant's kind, so it is the only check that the `kind()` arm is right.
  - **Finding (F3):** the subagent confirmed `hwdef`'s `ConsoleBaud` is unreachable. `hwdef::generate`'s first line is `t.validate().map_err(Refusal::InvalidTopology)?` (`hwdef/src/lib.rs:391`), and the CLI's only caller goes through it (`cli/src/hwdef.rs:41`).

## 6a. The merge of `main` `25203c58` (`1211a`, the architect's 18:09Z note)

- **Merged** as `a6c8a0d4`. One conflict, in the IR README's step table: `main` changed row 13b, and this branch changed 13c. Each side's row was kept.
- **Nothing to renumber.** This branch commits no specification or fixture, and its tests build specifications at `SPECIFICATION_SCHEMA_VERSION`.
- **The measurement again** (`spike/1213/measure-merged.out`): 36 console ports, the two new ones being `1211a`'s IR fixtures `d1-drv8876` and `d2-drv8833`, both at 115200. **None is refused** [measured].
- **The binary against the new `main`** [measured, `~/.cache/aet-1213/accept-25203c58.tsv`]: `validate` built from `25203c58` (by `git archive`) and from the merged branch ran over all 47 tracked files whose top level holds a `serial_ports` list. **Every pair printed the same bytes and exited the same way.**
  - 29 pass;
  - 1 `ZeroNominalVoltage`;
  - 1 `PartLimitUndeclared` (`d2`, below);
  - 16 spike files are refused by schema, as in §1.
- **The pin:** the repository's ArduPilot image is `Copter-4.7.2`, commit `fb773567` (`container/ARDUPILOT-SOURCES.md:13`), the tag of the file the rule cites (§0) [read].
- **CI-only jobs:** the `ardupilot` seeds (`container/ardupilot-pinout.sh:57-72`) edit hand-written ArduPilot boards, never a topology's baud. Every topology they judge is committed, and every one passes [read].
- **The binary on those two:** `d1` passes. `d2` is refused, but by `PartLimitUndeclared`, `1211a`'s own designed refusal: `pin_devices.rs::d2_is_refused_part_limit_undeclared_and_only_that` asserts it is the fixture's only refusal, and that test passes here.

## 7. Findings and questions for the architect

- **F1, the WO's examples (§1, prediction 3):** 100000 and 250000 both round-trip, so a test built on them would pass with no rule at all. The test asserts that both pass, and refuses six bauds that do fail. Nothing to rule unless the WO is to be corrected.
- **F2, the rule also in `SolvedTopology::validate` (§2), a scope edge.** The WO says `validate`. Every sibling 13c rule (shape, `negotiated`, the CRSF contradiction, the peer floor) is re-checked there, and §5's "a baud source the rule misses" names exactly a topology handed to `generate hwdef` (A3). All 10 committed topologies pass. It is one line and mutant M2. If the ruling is `validate` alone, it is reverted in one commit.
- **F3, `hwdef`'s `ConsoleBaud` refusal is now unreachable from the CLI** (A3, A4): a specification is refused at step 13c, and a topology at its validation, before the generator runs. It is the generator's own check, consistency under `RUL-20`, and outside this fence. A backlog row: keep it as the generator's guard, or remove it and its copy of `map_baudrate`.
- **F4, an observation, not this WO:** a non-console, non-RC port at 2000 baud or less is refused only by the generator (`SerialBaud`, `hwdef/src/lib.rs:640`), because ArduPilot reads such a `SERIALn_BAUD` as kbaud. `validate` passes it. It is the same kind of residual for SERIAL1 to 9: the define is stored as it stands, and `map_baudrate(b)` differs from `b` only when `b ≤ 2000`. A one-line sibling rule if wanted; not built (scope).
