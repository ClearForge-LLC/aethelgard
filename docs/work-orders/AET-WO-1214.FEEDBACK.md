# AET-WO-1214 FEEDBACK — `validate` refuses a serial baud ArduPilot would read as kbaud; the generator's console guard kept and tested

Builder: Claude (Opus 5.5), a fresh `1214` session, 2026-11-20 (predictions committed 20:02Z, `916046dc`), resumed 2026-11-22 01:30 UTC after the weekly limit. Worktree `~/aethelgard-1214`, branch `wo/aet-1214-serial-kbaud` from `origin/main` `781ea3b5`. `main` `b1056def` (#300, which carries this work order) and `ddfa64d6` (4240b, #293) are merged in. The work order was read from `~/.cache/aet-1214-wo.md`, and it is byte-identical to `docs/work-orders/AET-WO-1214.md` as #300 holds it [measured, `diff`].

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

**Two edges of the fence, stated first:**
- one test row in `crates/aethelgard-cli/tests/reserved_references_tier1.rs` (`serial-brr`) moves from `mavlink` to `rc_sbus`. Without it the new rule refuses that row's 300 baud first. Not posted before the edit; Q1.
- the hwdef test sits in `crates/aethelgard-hwdef/src/lib.rs`'s `#[cfg(test)]` module, since `generate` validates first and only the private `serial` reaches the guard. It changes no generator code; §2.

## 0. The source, read at the pinned tag

`AP_SerialManager.cpp` at `Copter-4.7.2`, from `4405`'s kit (`~/.cache/aet-4405/ap/`): sha256 `fe7cfce0…256d9b`, the hash `aethelgard_ir::serial::map_baudrate` cites, and byte-identical to `1213`'s copy [measured]. `AP_SerialManager.h`, `AP_GPS.cpp` and `AP_Param.h` come from the same kit [read]:

- `:199`, `:213`, `:227`, `:241`, `:255`, `:271`, `:352`, `:371`, `:390`: `1_BAUD` to `9_BAUD` hold `DEFAULT_SERIALn_BAUD` as it stands.
  - The `#ifndef` defaults (`:51-155`) are ArduPilot's own kbaud codes, such as `AP_SERIALMANAGER_MAVLINK_BAUD/1000`. They apply only when no define is emitted.
- `AP_SerialManager::init`, `:450-590`, for SERIAL1 and up:
  - `:469-475`: a MAVLink port (and `SerialProtocol_Console`) is begun at `state[i].baudrate()`.
  - `:488-493`: a GPS port is begun at `state[i].baudrate()`.
  - `:536-543`: an RCIN port is only `AP::RC().add_uart(uart)`. There is no `begin`, so AP_RCProtocol sets the rate (`AP_RCProtocol.cpp:326-352`, read by `4400b` F3).
- `AP_SerialManager.h:151-153`: `baudrate()` is `map_baudrate(baud)`.
- `:737-770`: `map_baudrate`. Above 2000 the value is the baud (`:762-766`). From 1 to 2000 it is a listed code or kbaud (`:742-769`).
- `AP_GPS.cpp:707-723`: the first probe is `port->get_baud_rate()`, which is `map_baudrate(SERIALn_BAUD)` (`AP_SerialManager.h:196`). After that, AP_GPS cycles through `_baudrates` (`:79`).
- `AP_Param.h:212`, `:225`: a parameter's default is stored as `const float def_value`.

## 1. The measurement (§1.1), predictions first

The predictions were committed before the run, in `spike/1214/PREDICTIONS.md` (`916046dc`). They are **not blind**: the builder had already read `spike/1213/measure-merged.out`, whose `n_BAUD` column is `map_baudrate(b) == b`. The harness is `1213`'s own `spike/1213/measure.py`, re-run here, so there is no second copy.

- **On `781ea3b5`** (`spike/1214/measure.out`, `e924e4fd`) [measured]: 36 console ports and **8 non-console ports**:
  - `mavlink` 57600 ×3, `gps` 115200 ×1, `rc_crsf` 420000 ×3, `rc_sbus` 100000 ×1;
  - every one has `map_baudrate(b) == b`.
- **On `ddfa64d6`** (`spike/1214/measure-merged.out`) [measured]: **12 non-console ports**.
  - 4240b's `hse-crystal-16.json` and `hse-external-8.json` add `mavlink` 57600 and `rc_crsf` 420000 each.
  - **None is refused.**
- The files the script could not read are malformed on purpose or are campaign logs. None of them holds `serial_ports` [measured, `grep -c`].
- **Prediction 1 (none refused): held.** Prediction 2 (roles): held at source (§0). Prediction 3 (coverage) is §3's evidence.

## 2. What is built

- **`IrValidationError::SerialBaudReadAsKbaud { id, role, baud, mapped }`**, step 13c, after `ConsoleBaudNotRoundTrip`. It fires on a `mavlink` or `gps` port whose declared baud `b` has `map_baudrate(b) != b`, and `mapped` is the rate ArduPilot would open.
  - It uses the IR's cited `serial::map_baudrate`; there is no second copy.
  - Within `int32_t` the condition is exactly `b <= 2000`, which is the generator's `SerialBaud` guard. A test sweeps 1 to 4000 and holds the two equal, with no shared code (`RUL-20`).
- **Roles, as an exhaustive `match` with no wildcard** (`serial::read_as_kbaud`), so a new role forces a decision:
  - covered: `mavlink` (`:473`) and `gps` (`:490`), both begun at `map_baudrate(SERIALn_BAUD)`;
  - not covered: `rc_crsf` and `rc_sbus`. ArduPilot never begins an RCIN port at its `SERIALn_BAUD` (`:536-543`), AP_RCProtocol sets the rate (`AP_RCProtocol.cpp:326-352`), and the generator emits no baud define for it (hwdef F3, PR #189);
  - not covered: `console`, which is SERIAL0 and keeps its own rule (`ConsoleBaudNotRoundTrip`, `1213`). At 2000 or less, that rule refuses every console baud except 1200, which SERIAL0 holds as `1` and reads back as 1200.
- **A baud above `i32::MAX`** is no `SERIALn_BAUD` at all (`AP_Int32`). The rule leaves it to step 21, where no BRR reaches it (`SerialBaudOutOfRange`). A test pins `i32::MAX` and `i32::MAX + 1`. This is a decision: see Q2.
- **`SolvedTopology::validate`** re-checks the rule in `serial::validate_solved`, as it does every 13c rule (`SerialPortMismatch`).
- **The IR README's 13c row** gains the clause and the kind.
- **F3, hwdef's guards kept and tested directly** — `the_serial_baud_guards_fire_when_called_directly`, a `#[cfg(test)]` test in `crates/aethelgard-hwdef/src/lib.rs`:
  - `generate` runs `t.validate()` on its first line (`:408`). Both guards therefore fire only when the private `serial(...)` is called directly, so the test has to sit in the crate's source file, not in `tests/`. **It changes no generator code.**
  - It shows `ConsoleBaud` firing at 76800 (mapped 76000), and `generate` on the same topology returning `InvalidTopology(SerialPortMismatch CONSOLE)`.
  - **Deviation, named:** the same test also covers `SerialBaud` (a `mavlink` SERIAL1 at 1200 is refused, and 2001 is not). This work order makes that guard exactly as unreachable as `ConsoleBaud`, and it had no test at all. The work order names only `ConsoleBaud`.
- **Two existing tests move off `mavlink`.** `1213` moved them onto it to reach `SerialBaudOutOfRange` at 300 and 900 baud, and this rule now refuses both first. They now use `rc_sbus`, whose baud step 13c leaves to AP_RCProtocol:
  - `ir/tests/serial.rs`, `a_baud_no_brr_reaches_is_refused_by_name`;
  - **outside the fence:** `cli/tests/reserved_references_tier1.rs`, the `serial-brr` row (Q1).
  - `1213`'s own test lost its `999 on mavlink passes` line, which is now refused, and asserts that only above 2000.

## 3. Acceptance (§2)

- **No committed specification is refused** [measured]:
  - §1, the measurement;
  - `cargo test --workspace` with the rule in, which covers every specification built inside a Rust test and every topology solved while a test runs: 2563 passed and 0 failed on `1b72cd97`, and **2593 passed and 0 failed** on `7e9c9971`, after the `ddfa64d6` merge;
  - **the real binary** (`spike/1214/accept.tsv`): `aethelgard validate --file`, built from `main` `ddfa64d6` (`git archive`, its own target) and from this branch, over all 49 tracked files whose top level holds a `serial_ports` list.
    - **All 49 printed the same bytes and exited the same way.** 31 pass.
    - The other 18 are refused alike by both binaries, for reasons unrelated to this work: `UnsupportedSchemaVersion` ×14, `SchemaViolation` ×2, `PartLimitUndeclared` ×1, `ZeroNominalVoltage` ×1.
    - None is `SerialBaudReadAsKbaud`.
    - (A first run passed each path as a positional argument and gave a usage error on all 49. It was discarded and re-run with `--file`.)
  - **the real binary refuses** [measured]. `serial-four.json` with its `MAVLINK` port at 1200, piped into this branch's `aethelgard validate`, exits 2 with `{"kind":"SerialBaudReadAsKbaud","id":"MAVLINK",…}` and `mapped` 1200000 in the message. At 2001 it exits 0. The JSON shape (no `claimants`) is the same as `ConsoleBaudNotRoundTrip`'s.
  - **Inline specifications in Rust tests** [measured, `grep`]: no `mavlink` or `gps` port at 2000 or less reaches `validate`. The only hits are this work order's own tests and the moved `serial-brr` row. One loop builds a `gps` `SolvedSerialPort` at 1200 (`campaign_tier1.rs:3191`) but never validates a topology, and it passed.
  - **Tier 2 compiles to 0 tests locally** (it is gated to CI's image). Its evidence is CI's run on this PR's head.
- **The package step** (CI's, run locally for `aethelgard-ir` and `aethelgard-hwdef`) [measured]: `cargo package --locked --no-verify` packs `tests/fixtures/serial-dshot-9600.topology.json` into the hwdef crate. CI's include-target check, run on the packaged `src/lib.rs`, finds the new `#[cfg(test)]` `include_str!` target in it.
- **`TIER2_MIN` is unchanged.** No `ci/` file is touched, and no weight row was needed.

## 4. The adversarial pass, targeted (§5)

- **A1. A baud source the rule misses: a role default** [measured]. No role takes a default baud in the IR: a port with no `framing.baud` is refused as `SchemaViolation` (`missing field \`baud\``, exit 2, on `serial-four.json`). ArduPilot's own defaults (`:51-155`) apply only to a port with no emitted define. The only such ports are RC ports, whose rate AP_RCProtocol sets. **Not a miss.**
- **A2. A protocol that sets its own rate: GPS** [read, a finding, not built]. ArduPilot begins a `gps` port at `map_baudrate(SERIALn_BAUD)` (`:490`), and AP_GPS's first probe is that rate (`AP_GPS.cpp:708`). After `GPS_BAUD_TIME_MS` it cycles through `_baudrates` (9600 … 460800, `:79`, `:710-714`), and the u-blox driver may move the module to another rate.
  - So the declared `gps` baud is where ArduPilot starts, not where it stays. Step 21's BRR and tolerance judge that first rate alone.
  - The rule still holds for `gps`: a baud of 2000 or less would make the first probe a different rate from the one declared. **F1 below.**
- **A3. A protocol that sets its own rate: RC** [read]. `rc_crsf` is held at 13c to at least `CRSF_MIN_BAUD`, so it cannot reach 2000 or less.
  - `rc_sbus` at 1200 passes step 13c (tested). The generator then refuses it as `FramingNotImplied` (SBUS is 100000 8E2 inverted), which is before its `SerialBaud` guard (`hwdef/src/lib.rs:630-677`).
  - So the IR and the generator still disagree on an SBUS port's framing. That gap was ruled earlier (`contradiction`: SBUS has no vendor-published source) and is unchanged here.
- **A4. A port the generator writes with a define the IR never sees** [read]. The generator writes exactly one baud define, `define DEFAULT_SERIAL{n}_BAUD {baud}` (`hwdef/src/lib.rs:726`), from the port's own `framing.baud`, and only for non-RC ports. `SERIAL_ORDER`, `_PROTOCOL` and `_OPTIONS` carry no baud. **No miss.**
- **A5. The float default** [read, inferred]. `AP_GROUPINFO` stores the default as a `float` (`AP_Param.h:212`). A define above 2^24 = 16,777,216 that is not float-exact would not round-trip.
  - No such baud passes step 21. The fastest kernel is `hclk_hz / D2PPRE`, at most 240 MHz on this MCU (the top HCLK row of RM0433 Table 17, `vos.rs:412`; inferred to be the ceiling that step 21 is reached under), and BRR ≥ 16 caps the baud near 15 Mbaud. **Not reachable**, so nothing is built.
- **A6. A console off SERIAL0** [read]. The generator refuses a second console (`ConsoleTwice`) and always puts the console first, so no console is emitted as SERIALn with n ≥ 1. The console stays outside the rule.

## 5. Mutants (§1.3), one per guard this work order adds

Harness: `~/.cache/aet-1214/mut/run.py`. Each mutant is applied to a backup copy and restored, and run against its own suite [measured, `out.txt`]:

| Mutant | Killed by |
|---|---|
| drop the rule in `validate` | `a_serial_baud_ardupilot_reads_as_kbaud_is_refused_by_name` |
| drop the rule in `validate_solved` | `the_topology_refuses_a_serial_baud_ardupilot_reads_as_kbaud` |
| flip `!=` to `==` | that test, `1213`'s console test, and two DS-009 GPS tests |
| cover `rc_sbus` | the kbaud test, and `a_baud_no_brr_reaches_is_refused_by_name` |
| cover `console` | the kbaud test (a console at 1200 passes) |
| drop `gps` | the kbaud test |
| hwdef: `ConsoleBaud` guard off | `tests::the_serial_baud_guards_fire_when_called_directly` |

7 of 7 are killed.

## 6. Gates and the reduction pass

- **Gates on `7e9c9971`** (`main` `ddfa64d6` merged) [measured, `~/.cache/aet-1214/clippy2.log`, `test-ws2.log`], each run in a capped user scope:
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0;
  - `cargo test --workspace`: 2593 passed, 0 failed, 0 ignored.
- **Merging `ddfa64d6`** (4240b) touched `ir/tests/serial.rs` in four lines, none of them this work order's. It added the two `hse-*` fixtures that §1 re-measured.
- **The reduction pass** (diff-sized): nothing is cut.
  - The rule is one predicate, `read_as_kbaud`, with one call site in each validator, and it reuses `map_baudrate`.
  - Its README clause is the only prose that restates the rule.
  - The hwdef test reuses a committed fixture, not a new one.
  - Both earlier tests were changed in place, not duplicated.
  - Freed: the 2.0 GB `main` baseline build, once the comparison was recorded.

## 7. Findings and questions for the architect

- **F1 (A2), backlog.** A `gps` port's declared baud is only AP_GPS's first probe (`AP_GPS.cpp:707-723`), so step 21 judges a rate ArduPilot may leave. A rule here would state which rates a `gps` port must also reach: every rate in `_baudrates`, or the one the driver configures.
  - This is a further round on a safeguard that works, so it is a backlog row under the scope test, not a build.
- **Q1, outside the fence.** `crates/aethelgard-cli/tests/reserved_references_tier1.rs` (`serial-brr`) changes one role, `mavlink` to `rc_sbus`, and its comment. Without that change the new rule refuses that row's 300 baud first, and the CLI test fails. `1213` made the same kind of change in the same row.
- **Q2, the `i32::MAX` boundary.** Above `i32::MAX` the rule does not fire, and step 21 refuses the baud. The alternative was to refuse it at 13c with an invented `mapped`. Is leaving it to step 21 the ruling you want?
- **Q3, the deviation in §2.** The hwdef test also covers `SerialBaud`. Keep it, or cut it to `ConsoleBaud` alone?

## 8. READY

**READY, 2026-11-22.** PR #304 at `086809f4` (`main` `7c1f369f` merged in): every CI check is green (run 44973990259; tier 2 201 passed at floor 201; `TIER2_MIN` unchanged). The token was revoked after the PR opened (HTTP 204). The architect merges, and this commit is the last push.
