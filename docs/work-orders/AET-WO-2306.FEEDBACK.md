# AET-WO-2306 FEEDBACK — a gate that judged nothing says so

**Branch** `wo/aet-2306-nothing-judged`, from `main` 2fda6e13. Builder: Claude (Opus 5.5), standard tier, 2026-11-20.

## 0. What is built

- **Each gate's verdict line carries `judged`.** Exactly when it is 0, the line also carries `nothing_in_scope: true` and a one-line `nothing_in_scope_reason`.
  - The JSON change is additive. No existing key or value moves.
  - A pass with nothing in scope stays a pass (§1, "It is NOT").
  - One type, `verdict::Scope`, builds all three keys. It is flattened into each line that carries them.
- **Where the keys appear:**
  - `extract`'s own line. `judged` is its lines plus its supply domains. With neither, the reason reads "no bounded line: the specification declares no I2C bus, no SPI bus and no supply domain with a drop limit".
  - `check speeds`' own line. `judged` was already there and is unchanged. With 0 judged, the reason reads "no declared speed: no SPI bus's SCK or MOSI, the only lines driven at a peripheral's rate; every listed line is out of scope and held only to the slowest speed".
  - `verify`'s entry for every gate that ran, with the counts in §1. A `not_run` gate carries none.
  - `campaign`'s scenario line, as `observation_witness`, on a run whose droop released at least once. `judged` counts the releases judged against a device's witness. With no identity read on the boot: `judged: 0`, `nothing_in_scope: true`, and the reason "no witnessed device: the boot reads no device's identity, so each recovery is judged by the boot word alone". `verify`'s copy of each run forwards it.
- **`verify`'s summary gains `judged_nothing`.** It lists every gate with nothing in scope, by name, in the verdict's order, then each campaign run whose droop witnessed no device, as `campaign:<scenario>`.
  - It is always present, `[]` when nothing judged nothing.
  - It sits after `gates`, before `against_fab` and `fab`, so the line `MANIFEST.json` hashes is still the verdict without the fab keys.
- **No archived record moves.** `observation_witness` is derived in the CLI from the records the run archives, by the rule replay's judge uses (`first_witness`: any I2C or SPI witness record). So no CBOR, trace version or `state_sequence_hash` changes. `aethelgard-chaos` is untouched.

## 1. The inventory (§1.1)

### 1.1 Every gate `verify` runs

| Gate | What it judges | Where its count lived before | `judged` now | Can it be 0? |
|---|---|---|---|---|
| `erc` | the schematic, whole, by KiCad's ERC | nowhere: the line names the schematic by sha256 and lists findings; KiCad's report lists violations per sheet, not what it checked | 1 (the schematic) | no, by construction: every emitted schematic holds the MCU. See A4 for a report with no sheet |
| `drc` | the board, whole, by KiCad's DRC with schematic parity | nowhere: the board's sha256 and the findings | 1 (the board) | no, by construction |
| `support` | each support role (bypass, bulk, VCAP, reset, …) and each strap | `support.roles`, `support.straps` (each with its verdict) | roles + straps | no on any committed board: 19 + 4 on every one of the 22 that emit. 0 only for a line with no `support` record, which `verify` labels with the generic reason |
| `extract` | each I2C line, each SPI SCK and MOSI, and each supply domain with a drop limit; a declared pair's conductors are lines | `lines`, `supplies` | lines + supplies | **yes**: a board with no I2C bus, no SPI bus and no drop limit (`4600`'s motor driver) |
| `prove` (one entry per harness) | one generated Kani harness | the harness's name; `cover_count` (its covers reached) | 1 (the harness), `cover_count` unchanged | no: a harness with no reachable path already fails as `cover_unsatisfied`, and no harness at all is the existing `no_harness` error |
| `build` | the board crate, into the ELF | the crate's sha256, the ELF's | 1 (the crate) | no, by construction |
| `simulate` | the boot: boot word, register image, identity reads and their witnesses, serial ports, motor outputs, brown-out | `boot.registers`, `boot.i2c`, `boot.spi`, `boot.serial`, … | 1 (the boot) | no, by construction: the boot word is always judged |
| `campaign` | each scenario's run | `campaign` (the runs) | runs | no: `verify` runs it only with `--scenarios` |
| `campaign`, a droop's observations (§1.4) | each release's recovery, against the first identity read's witness | nowhere: kind 19's `witness` is `null` both with no identity read and with a log not read whole | `observation_witness.judged`: releases, when a device is in scope | **yes**: a boot with no identity read (the motor driver) |
| `attribute` (per detected run) | one archived run | its `run_id` | 1 (the run) | no |
| `check speeds` (not a `verify` gate) | each SPI SCK and MOSI against DS12117 Table 149 | `judged` | unchanged, plus the reason when 0 | **yes**: no SPI bus |

**The rule.** `judged` counts the items that each carry a verdict on the gate's own line. A gate whose line lists none judges its one input whole, and counts 1. `verify` reads the count from the gate's line (`scope_of`): the line's own `judged` and reason where it has one (`extract`), `support`'s roles and straps, and 1 otherwise. The campaign's count is its runs.

**No gate needs a change to what it checks to be counted** (§7: no stop). ERC's 1 is by construction, not measured from the tool's report. That residual is A4.

### 1.2 Every committed specification, run

Run with this branch's CLI on Furnace:
- `validate`, `emit`, `extract` and `check speeds` on the host, every file;
- `check support`, and the campaign of every committed droop scenario, in the pinned tools image.

Kit: `~/.cache/aet-2306/kit/inventory.sh`, `sh/inv2.sh`; tables in `~/.cache/aet-2306/inv/`.

There are 39 committed specification files:
- 27 at specification 18 (`examples/`, `crates/*/tests/fixtures/`);
- 12 `spike/` records at 16 or 17 (`4600`'s ten, `1314b`'s two).

Results:
- **Specification 18: none judges nothing.** 22 emit. `extract` judges 4 to 16 lines on each and supplies 0 (no committed specification declares a drop limit). `check speeds` judges 2 to 8. `support` judges 19 roles and 4 straps on each.
- The rest stop before any gate, as on `main`:
  - `imu-baro-1v8` emits nothing: `UnknownPart`, by design (`speeds` still judges 2, labelled unverified).
  - The three `solve/` infeasibility fixtures are refused `Infeasible`.
  - `invalid-zero-domain` is refused `ZeroNominalVoltage`.
- **The `spike/` records are refused `UnsupportedSchemaVersion` by `main`.** They are records of runs at their own specification; this PR leaves them alone. `4600`'s motor driver is the one board that judges nothing. It is migrated to 18 as the test fixture (§2).

| Board · scenario (droop) | Releases | Device witnessed | `observation_witness` (measured) |
|---|---|---|---|
| `sentinel-01-minimal` · `sentinel-01/droop-bor3` | 1 | U4 (W25Q64, SPI) | `{"judged": 1}` |
| `sentinel-01` (bench) · `sentinel-01-bench/droop-bor3` | 1 | U4 | `{"judged": 1}` |
| `motors-dshot600` · `motors/dshot600-droop` | 1 | U4 | `{"judged": 1}` |
| `sentinel-01-minimal` · `sentinel-01/droop-tolerated` | 0 | — | absent (no release, no observation) |
| `sentinel-03` · `sentinel-03/droop-tolerated` | 0 | — | absent |
| `driver-judges-nothing` (4600's motor driver) · its `droop-bor3` | 1 | none | `{"judged": 0, "nothing_in_scope": true, …}` |

**So on every committed board, `judged_nothing` is `[]`.** On Sentinel-01 that is asserted on a real seeded `verify` (§2). The motor driver's is `["extract", "campaign:droop-bor3"]`. Its `speeds` judges nothing too, but `check speeds` is not a gate `verify` runs (D2).

## 2. Acceptance (§2)

- **The inventory:** §1.
- **The fields appear exactly when a gate judged nothing.** Tests:
  - **The motor driver** (`tests/fixtures/driver-judges-nothing.json`: `4600`'s `spike/4600/driver.json`, its `schema_version` 18 and each output's `esc: null`; after `4240v` merged, spec 19 as 4240v migrated Sentinel-03 (same MCU): `mcu.core_supply` and the three silicon-only acks `PWR.CSR1`, `PWR.D3CR` and `FLASH.ACR`, copied word for word; nothing else changed):
    - `extract_tier1::a_board_with_no_bounded_line_judges_nothing_and_says_so`: `judged: 0`, `nothing_in_scope: true`, the reason, verdict `pass`, findings `[]`.
    - `speeds_tier1::the_motor_driver_judges_no_line_and_says_so`: the same, and its six lines are out of scope at `00`.
    - `board_tier2::verify_names_what_judged_nothing_on_the_motor_driver` runs `verify` with its droop past BOR 3, in the pinned image: 73.1 s, scope peak 6.1 GB. It checks:
      - the verdict is `pass`;
      - `extract`'s entry has `judged: 0`, `nothing_in_scope` and the reason;
      - every other gate has `judged ≥ 1` and no `nothing_in_scope`;
      - the campaign's `judged` is 1;
      - the droop's run has `observation_witness` with no witnessed device;
      - `judged_nothing` is `["extract", "campaign:droop-bor3"]`.
  - **Sentinel-01:** `verify_2k2_tier2::verify_fab_out_writes_the_package_its_verdict_names_at_any_path`, the seeded set and `--fab-out`, now also asserts:
    - `judged_nothing == []`;
    - every gate has `judged ≥ 1` and no `nothing_in_scope`;
    - `droop-bor3`'s `observation_witness` has `judged ≥ 1` with no `nothing_in_scope`;
    - every other run has no `observation_witness`.

    In the pinned image on Furnace it passes in 235.9 s, with a scope peak of 3.8 GB.
  - The reference's `extract` (`judged: 4`) and `speeds` (`judged: 2`) carry no `nothing_in_scope` key (tier 1).
  - Unit tests:
    - `verdict::tests::nothing_in_scope_appears_exactly_when_nothing_was_judged`;
    - `campaign::tests::a_droop_observation_with_no_device_witness_says_so`: I2C, SPI, no device, no release;
    - `verify::tests::judged_nothing_names_exactly_the_gates_and_runs_that_judged_nothing`: `scope_of` per gate kind, a `not_run` gate carries no scope, the order kept.
- **The mutant.** `Scope::of` always says `nothing_in_scope: false`, with no reason. It runs on a copy of the tree (`~/.cache/aet-2306/mut/`) with its own target dir. 8 tests fail; the other 184 pass. The first test fails, as §1.5 asks:
  - the motor driver's `extract` and `speeds` tests (`extract_tier1.rs:1051`, `speeds_tier1.rs:238`);
  - the no-SPI `speeds` test (`:284`);
  - the three unit tests;
  - the reference's `extract` and `speeds` tests: the key they assert absent now appears as `false`.

  Log: `~/.cache/aet-2306/mut/mutant.log`. Re-run on the tree with `main` 943f7112 merged (the unit suite and tier 1 only): the same 8 fail (`mutant-48.log`, `mutant-48-bin.log`).
- **The bench package:** `MANIFEST.json`'s `verdict_sha256` moves, because the verdict gains keys. It is not hand-edited: it is committed byte for byte from the architect's `write=true` dispatch (§6).
  - **Reproduced locally**, on the merged tree, in the pinned image: CI's `bench-package` `verify --against-fab` exits 1 (186 s, 6.4 GB scope peak), and its one difference is "MANIFEST.json differs from the derived file".
  - On that verdict `judged_nothing` is `[]`, and `droop-bor3`'s `observation_witness` is `{"judged": 1}`.
  - On the merged tree the driver's tier-2 test passes again, in 94.6 s. With `main` 943f7112 merged (4240v, spec 19) it passes in 106.2 s, and Sentinel-01's `verify --fab-out` (`judged_nothing == []`) in 322.4 s, each under a 16G cap read back (peak 6.6 GB).
- **The gates,** on the tree with `main` 943f7112 merged (1103, 1209b, 1316, 1317, 4240v), on Furnace in a capped scope, each exit read directly: `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0, and again with `--features aethelgard-cli/with-tools` 0; `cargo test --workspace --no-fail-fast` 0, 2,398 passed, 0 failed, 0 ignored. CI is reported on the bus and in the PR.

## 3. How verdict consumers treat the new keys (§1.2)

- **`--against` and `--against-fab`** compare files, not verdict keys.
  - `--against-fab` compares the package byte for byte, and `MANIFEST.json` names the verdict line's sha256. So the committed bench package differs in `MANIFEST.json` alone until it is re-committed from the dispatch. That is the bench-package job's expected red before the dispatch.
  - `against_fab_tier2` writes its own package and compares that, so it does not move.
- **Byte identity:** two runs still print the same bytes. The keys are functions of the gates' own lines, and the campaign's of the archived records, never of a path or a time. Asserted by the existing two-run tests.
- **Goldens:** no committed golden holds a whole `extract`, `speeds`, `campaign` or `verify` line. The full workspace suite passes, 2,398 tests, with only the tests above changed.
- **No archived record changes:** replay, `attribute`, every trace and every `state_sequence_hash` are unchanged.
- **MCP:** `verify`, `extract` and `check_speeds` pass the line through. The tool snapshot (`tests/mcp-tools.json`) holds input schemas only, and is unchanged.
- **FAB-NOTES** reads no gate key and does not move: `extract`'s conditions, `support`'s record and the campaign's conditions are unchanged.
- **A consumer that reads `gates[].findings`, `verdict` or any existing key** sees them unchanged, in the same order. `judged` and its two keys sit between `verdict` and `exit_code` in each entry.

## 4. The adversarial pass (§5, targeted)

- **A1. A gate that judged something but reports 0** (counting the wrong collection).
  - `extract` counts the lines it judged (`out`, after each line's declared bound is found) and its supply domains. A pair's conductors are lines already (a conductor that is not one is refused, `PairConductorUnbounded`), so they are not counted twice.
  - `verify` reads a top-level `judged` from a gate's line. No other gate's line has a top-level `judged`: `speeds`' per-line `judged` objects are nested, and `as_u64` ignores them. Checked by grep across every crate's `src`.
  - The droop's device scope keys on the archived I2C or SPI witness record, the record replay's judge (`first_witness`) compares each observation's lines with. It does not key on the observation's own `witness: null`, which is also what a log not read whole archives. That case is a device in scope whose witness failed, and it is judged (and fails), not nothing.
  - **Not reached by this WO:** a gate whose line counts nothing gets 1 by construction (A4).
- **A2. Items in scope that are all skipped: "nothing judged"?** Decided case by case:
  - **`check speeds` with every line out of scope:** nothing judged. No line is judged against the table. Each out-of-scope line is held only to the slowest speed, which the reason says. This is `4600` F5's reading.
  - **A droop whose boot was killed before its probe:** judged, not nothing. The device is in scope and the release is judged as `brownout_unrecovered`, which fails the run. So `judged` counts it, and nothing-in-scope is not claimed for a failure already named.
  - **A board whose only I2C devices have no Renode model** (TMP117s, `device_unmodelled`): `simulate` reads no identity for them. A droop on such a board with no SPI device has no device in scope, so it says `nothing_in_scope`. `simulate` itself still judged the boot.
  - **`simulate` with every device unmodelled:** not nothing. The boot word and the register image are judged; the skipped devices are already named `device_unmodelled` (info).
  - **A droop that never crosses its BOR level** (`droop-tolerated`): no release, so no observation and no field. The run judges the uninterrupted boot.
- **A3. A verdict consumer that breaks on the new keys:** §3. `serde(flatten)` on `Option<Scope>` writes no key for `None`; checked on a `not_run` gate in the unit test. The keys deny nothing on input: no subcommand reads a verdict with `deny_unknown_fields`. The only reader of `verify`'s gate lines is `verify`, and it reads by key.
- **A4. Residual: ERC's 1 is not measured.** `kicad::judge` accepts an ERC report whose `sheets` is `[]` with no findings, and `verify` would then say `judged: 1`. Counting sheets on `check erc`'s own line is outside this fence (§3: only `extract`, `speeds`, `verify` and campaign lines), and refusing a report with no sheet changes what ERC checks. Backlog: `check erc` reports `judged` as the report's sheets. A sheet is what KiCad says it judged; an empty list is then `nothing_in_scope`.

## 5. The reduction pass

- Diff-sized, over this WO's change only.
- **One type, `Scope`,** carries the three keys everywhere. The field name, the reason key and the "0 means nothing" rule each have one home (`Scope::of`).
- **Removed:**
  - a committed scenario under `tests/fixtures/` (a scenario's `base` must sit inside the run's directory: `BaseOutsideRoot`), replaced by the scenario the test writes, as `board_tier2`'s seeded test does;
  - `judged_nothing`'s de-duplication of gate names and its test case. Only `prove` and `attribute` repeat in `gates`, and each counts 1, so it guarded nothing reachable. The loops became one filtered chain.
  - `speeds` named in `scope_of`'s doc as a gate whose line it reads: `verify` runs no `speeds` gate.
- **Nothing in the diff is `belt-and-suspenders`.** `verify` re-deriving `nothing_in_scope` from `judged == 0` is the rule itself, not a second guard. A gate's own reason is kept when it gives one.
- **Kept, with the reason:**
  - the three reason texts as constants, because each is asserted by a test;
  - `NOTHING_LISTED`, `verify`'s reason for a gate whose line counts 0 and states none (only `support` with no record reaches it).

## 6. Decisions and open items

- **Ruled 05:10Z (architect, on the 04:41Z bus note):** every default is accepted:
  - the units per gate (§1.1), with the rule stated in `scope_of`'s doc comment;
  - `speeds` left out of `verify` (D2);
  - the droop's scope keyed on the boot having no identity check;
  - `judged_nothing`'s form and place.

  With 2305, which changes the droop recovery rule on the same observation (trace 17), whichever lands second merges. These fields stay CLI-derived and move no archived record.
- **D1 · How `judged_nothing` names an observation (ruled: accepted).** It reads `campaign:<scenario>` beside the gate names, so one list shows everything that judged nothing.
- **D2 · `check speeds` is not a gate `verify` runs.** The P4 clause (roadmap §5) lists "`check speeds` passes" separately. Its own line now says `nothing_in_scope`, but `verify`'s `judged_nothing` cannot list it. Adding `speeds` as a `verify` gate changes `verify`, which is outside this WO. Recommend it to a later WO if the P4 board's verdict should carry it.
- **D3 · `TIER2_MIN` (ruled 06:05Z: raise it yourself; the fence is widened to that line).** It is raised 194 → 195 in `.github/workflows/ci.yml`: this PR adds one tier-2 test, `board_tier2`'s driver test, and CI counted `tier2_tests_passed=195` on bc96bc3f. It is recomputed at each merge of `main`: with 943f7112 merged (1103, 1209b, 1316, 1317, 4240v; main still 194, so none of them added a tier-2 test) it stays 195. The test's weight row is in `ci/test-weights.tsv`: a Furnace estimate, 73.14 s × 2.8 = 204.8 s, since every test of the lock-serialised `board_tier2` needs one.
- **D4 · The CLI README** (`crates/aethelgard-cli/README.md`, outside the fence) does not yet name `judged`, `nothing_in_scope` or `judged_nothing`. Suggested sentence for `verify`'s verdict: "Each gate's entry carries `judged`, the items it judged, and when that is 0, `nothing_in_scope: true` and its reason; `judged_nothing` lists every gate, and each droop run as `campaign:<scenario>`, that judged nothing."
- **D5 · Backlog A4:** `check erc` counts sheets.

## 7. What did not work, and why

- **The first inventory read** printed nothing for `judged`: an f-string quoting error in an inline reader. Re-read from the saved outputs with a script file. The run itself was fine.
- **A committed scenario beside the fixture,** with `base: "../driver-judges-nothing.json"`, would be refused `BaseOutsideRoot`. It was removed before any run; the test writes its scenario beside a copy of the specification.
