# AET-WO-1200 — FEEDBACK

**Branch:** `wo/aet-1200-solver` · **Base:** `2f13278` (PR #28, the base the kickoff named; `ReservedPin` present in the WO) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Result:** spike answered (below), ruled at review of PR #32 (2026-11-07), and §1 built under that ruling on the same branch after merging `origin/main` (`c923cd3`). Every §3 item passes on Furnace; CI is cited in the build's Gates section.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Spike

The probes were under `spike/` (`spike/README.md`), which the build deleted by ruling 8. They remain at commit `03490cb`, read with `git show 03490cb:spike/<path>`. Every command was `spike/run.sh` from the repo root, and the recorded output is in `spike/results/`:
- `furnace-release.txt`: `spike/run.sh measure` (its header names what it ran on);
- `furnace-debug.txt`: the debug profile, as `gates` builds it;
- `unbounded.txt`: two runs with no node budget;
- `fuzz-furnace.txt`: `spike/run.sh fuzz`.

All measurements were taken on Furnace (`furnace777`) with rustc 1.99.1.

**How the probe is built.** `spike/solver-probe` is its own Cargo workspace, not a root member. A member broke `crates/aethelgard-cli/tests/workspace_shape.rs::members_are_exactly_the_seven_parts` (measured: 1 failed). That test pins the root to seven members, and changing it is outside this WO and would weaken it. `AET-WO-0300a`'s spike took the same route. Root `Cargo.toml` and `Cargo.lock` diff to empty. The probe adds no dependency the root does not already lock: `serde`, `serde_json`, `sha2`, plus the two crates by path. No `unsafe`. Its gates are `cargo fmt --check`, `clippy --all-targets -D warnings` and `test`, all run through `spike/run.sh gates`, and they pass.

**Pruning settings measured.** The WO's search is `fc` (forward checking only). Three optional additions were measured, each only cutting subtrees that hold no solution:
- `im`, **instance matching**: the buses still to place can take distinct instances. This is Hall's condition, checked by Kuhn's augmenting paths.
- `pm`, **pin matching**: the signals still to place can take distinct free pins, each from the union of its candidates over the bus's still-placeable instances.
- `memo`, **failed-state memo**: a failed suffix is remembered by (position, the used pins it could touch, the used instances).

### S1 The search

**Answer: the WO's search meets the bound on both named specifications, by four orders of magnitude. Z3 not measured: the search met the bound** (`RUL-02`'s inverted prior, applied). It is exponential on adversarial infeasible input, though. Two pure-Rust propagation checks remove every such case found; the architect's decision is below.

The model is built as §0 S1 states, with these interpretations (all decision-needed: confirm or correct):
- An instance is "offered" when the table carries each of its kind's signals for it on some pin. The `SMBA` and `NSS` signals are ignored. Offered: `I2C1` to `I2C4`, `SPI1` to `SPI6`.
- **Proposed order for `cs`, which the WO leaves open.** Every `cs` comes after all buses, in `spi_devices` document order, each taking the first free, unreserved pin in key order. Greedy is complete here, because buses always use the same number of pins (2 or 3). Placing a `cs` earlier could steal a later bus's only pin.
- Signals within a bus go `sda, scl` and `sck, mosi, miso`. The forward check runs after each bus is fully placed, not after each pin.
- **"The table's key order" is byte order**: `PA0, PA1, PA10, …, PA15, PA2, …`. It is deterministic, but it is not the order a human expects, and it decides which pin wins. For example, `SPI2_SCK` takes `PA12` before `PA9`. In `stress.json` the chip selects take `PA0, PA1, PA10, PA11, PA15, PA2, PA3, PA4`. Instance names sort correctly only while they stay single-digit (`I2C10` would sort before `I2C2`).

Solve time is the median of 21 runs, release from `furnace-release.txt` and debug from `furnace-debug.txt`. It includes parsing and validating the specification, and excludes loading the pin table (measured separately, last row). Medians move by up to about 30 % between runs; the files are the record.

| Specification | Search | Release | Debug | Nodes |
|---|---|---|---|---|
| `examples/sentinel-01-minimal.json` | `fc` | 106 µs | 625 µs | 7 |
| `spike/specs/stress.json` (4 I2C, 4 SPI × 2 devices) | `fc` | 137 µs | 986 µs | 42 |
| `stress.json` | `fc+im+pm` | 208 µs | 1452 µs | 42 |
| `pigeonhole-i2c.json` (5 I2C buses, 4 instances) | `fc` | 1.6 ms | 11.0 ms | 17 310 |
| **`pigeonhole-spi.json`** (7 SPI buses, 6 instances) | **`fc`, unbounded** | **329 s** | — | **3 105 219 359** |
| `pigeonhole-spi.json` | `fc+im` | 0.28 ms | — | 308 |
| `pigeonhole-spi.json` | `fc+memo` | budget spent, 3.4 s | — | 10 000 010 |
| **`thrash-pins.json`** (see note) | **`fc+im`, unbounded** | **> 600 s, killed** | — | — |
| `thrash-pins.json` | `fc+memo` | 0.88 s | — | 4 176 810 |
| `thrash-pins.json` | `fc+pm` | 0.26 ms | — | 41 |
| `pigeonhole-spi.json` / `thrash-pins.json` | `fc+im+pm` | 0.47 / 0.27 ms | 3.5 / 1.9 ms | 308 / 41 |
| `pin_table("STM32H753ZITx")`: load, hash check, parse | — | 1.36 ms | 18.0 ms | — |

Note on `thrash-pins.json`: four free I2C buses, three free SPI buses, then three SPI buses hand-set to `SPI1`, `SPI3` and `SPI6`, whose spare MISO pins (`PG9`, `PG12`, `PC11`) are held by `cs` hand pins. That leaves three MISO signals for two pins, `PA6` and `PB4`. Instance matching cannot see a pin conflict, so the search re-fails it under every combination of the earlier free buses.

Fuzz (`fuzz-furnace.txt`): 8 000 random specifications (seeds 777, 2026 and 31337), with hand values at rates of 0, 15, 30 and 50 %. Of these, 3 128 solved and 1 138 were infeasible after the search; the rest were refused before it. Results:
- 0 disagreements between settings and 0 failures of the independent verifier.
- The worst `fc+im+pm` case took 1 729 nodes, against about 3.4 M for `fc`.
- The random distribution never produced the adversarial shapes; those were built by hand.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S1.1 | The WO's search (`fc`) meets the 1 s bound on both named specifications in either profile: 137 µs release, 0.99 ms debug, on stress | `furnace-release.txt`, `furnace-debug.txt` | measured | Z3 not measured; `RUL-02`'s prior stands | no |
| S1.2 | `fc` is exponential on infeasible input. Seven SPI buses over six instances took 329 s (3.1 × 10⁹ nodes). This is §5.3 answered early | `unbounded.txt` | measured | Build `im` (instance matching) into the forward check | **yes** |
| S1.3 | `fc+im` is still exponential when the conflict is over pins rather than instances (`thrash-pins.json`, killed at 600 s). `pm` (pin matching) removes it: 41 nodes | `unbounded.txt`, `furnace-release.txt` | measured | Build `im` and `pm` together. Neither alone covers both shapes: `pm` alone spends the 10 M-node budget (2.3 s) on the SPI pigeonhole | **yes** |
| S1.4 | `memo` adds nothing once `pm` is present (the same nodes on every specification). Alone it is 0.88 s on `thrash-pins` and spends the budget on the SPI pigeonhole | `furnace-release.txt` | measured | Leave it out | no |
| S1.5 | No polynomial check is complete for this problem, so a shape that defeats `im+pm` may exist. None was found by hand or by 8 000 fuzz cases | — | gap | Add a **node budget** as a backstop: a count, never wall time. At 10 M nodes it costs about 1.4 s release here, and refuses `BudgetExceeded` naming the bus being placed | **yes**: whether to have one, and its value |
| S1.6 | The order of `cs` is unspecified in the WO | §0 S1 | missing rule | `cs` last, in document order, greedy | **yes** |
| S1.7 | Key order is byte order, not natural order (`PA10` before `PA2`) | §0 S1 | note | Keep it, since it is a function of content; document it in `aethelgard-ir/README.md` | yes, if natural order is wanted |
| S1.8 | Every stress or reference solve is under 1.5 ms in debug, but loading the pin table is 18 ms in debug and 1.36 ms in release | `furnace-debug.txt` | measured | `solve`'s latency is dominated by the table, not the search | no |

### S2 Determinism

**Answer: byte-identical on Furnace. The `gates` half cannot be measured in the spike. The reason is below, and the build's §1.4 test is that measurement.**

Measured on Furnace:
- **Two solves in one process are identical**, for all 15 specifications under all 6 settings, 21 runs each (`identical=true` throughout `furnace-release.txt`).
- **Separate processes and profiles give the same bytes.** The release binary and the debug test produce the same output sha256 for all 15 specifications. The test pins those digests and passes (`spike/run.sh gates`).
- **The answer does not depend on the pruning setting.** Every setting that finishes gives the same bytes, across 15 specifications × 6 settings and 8 000 fuzz cases × 4 settings. The only differing outputs are `BudgetExceeded` refusals.
- The reference digest is `c7f59a497e8ed989e83491a3a55af2a86a076f06470c8ac13c64e987e561f0cd`, and stress is `759f41463aaf205dec2eed472c0929e38a300f749cfe51ff49f3937c23e4b578`. Both are sha256 of the one-line JSON.

The **`gates` half cannot be measured here.** `gates` runs only the root workspace, and every route into it is closed to the spike:
- a root member fails `workspace_shape`;
- a probe test in `aethelgard-ir` would need `aethelgard-svd-hal` as a dev-dependency, which the same test forbids;
- a test in `aethelgard-cli` breaks the kickoff's rule that the spike touches nothing there;
- a workflow is a protected surface.

The PR's CI run below is the root's `gates`, green; it does not include the probe. The build's §1.4 "two solves byte-identical" test is the `gates` measurement. The digests above are ready to become its constants.

What in the search could make output depend on anything but its inputs:
1. **A `HashMap` or `HashSet`.** None is used: the probe's library has no `HashMap`, threads, randomness or clocks (grep). Every collection is a `BTreeMap`, `BTreeSet` or `Vec` (`RUL-16`).
2. **A budget measured in wall time.** That would make the answer depend on the host. If S1.5 rules a budget in, it must count nodes, as the probe's does.
3. **A pruning step that changes the exploration order** rather than only cutting failed subtrees. The probe's checks answer yes or no and never reorder, so the first solution found is unchanged (measured above). Kuhn's matching only answers whether a matching exists, so its internal order cannot leak into the output.
4. **The hash.** `specification_sha256` is of the input bytes as read (`INV-03`), so reformatting the file changes the hash but not the assignments. This is by design.
5. **The table** is an input (`INV-01`): embedded, hash-checked, and pinned to one commit.
6. **`serde_json` output** contains only ASCII strings and integers (`RUL-14`), with no floats and no escaping choices.

### S3 Refusals that name causes

The exact messages, produced by the probe (`furnace-release.txt`, any setting).

The probe checks in this order, each step in document order (buses, then `cs`) — decision-needed: confirm:
- **(A)** every hand pin resolves to a table key, which is `A5` by construction, and is not reserved. A repeat of an earlier hand pin is caught in the same walk, so the first repeat in document order is the one named;
- **(B)** every hand peripheral is an instance of the bus's kind, and no two buses hand-set one instance;
- **(C)** each bus alone, honouring reserved pins and every hand pin;
- **(D)** the search;
- **(E)** on failure, the minimal set.

| Case | Specification | Kind | Message |
|---|---|---|---|
| Hand pin claimed twice (**gate clause 2**) | `ref-pin-twice.json` | `PinClaimedTwice` | ``pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs` `` |
| Pin the table lacks | `unknown-pin.json` | `UnknownPin` | ``` `i2c_buses[0].scl.pin` names pin `pb8`, which the pin table does not have ``` |
| Reserved pin | `reserved-pin.json` | `ReservedPin` | ``` `spi_devices[0].cs.pin` names pin `PA13`, which is reserved for the SWD debug port ``` |
| Peripheral the table does not offer | `unknown-peripheral.json` | `UnknownPeripheral` | ``` `i2c_buses[0].peripheral` names peripheral `I2C5`, which the pin table does not offer for this bus kind ``` |
| … or offers for the other kind | `wrong-kind-peripheral.json` | `UnknownPeripheral` | the same, naming `SPI1` |
| Bus no instance can serve: hand pin | `no-instance-pin.json` | `NoInstance` | ``no instance can serve bus `SENSORS`: I2C1: `I2C1_SDA` is not on pin `PB6`; I2C2: …; I2C3: …; I2C4: `I2C4_SDA` is not on pin `PB6` `` |
| … hand peripheral and pin | `no-instance-peripheral.json` | `NoInstance` | ``no instance can serve bus `SENSORS`: I2C3: `I2C3_SDA` is not on pin `PB7` `` |
| … every pin held by another hand pin | `no-instance-held.json` | `NoInstance` | ``no instance can serve bus `SENSORS`: I2C3: every pin for `I2C3_SDA` is held by hand (`PC9` by `spi_devices[0].cs`) `` |
| Two buses hand-set one instance (**not in the WO's list**) | `peripheral-twice.json` | `PeripheralClaimedTwice` | ``peripheral `I2C2` is claimed by both `i2c_buses[0]` and `i2c_buses[1]` `` |
| Infeasible after the search | `infeasible-shared.json` | `Infeasible` | ``buses `I2C_BUS_0`, `I2C_BUS_2`, `I2C_BUS_3` cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)``. The free `I2C_BUS_1` is correctly left out |
| … pigeonhole / pin conflict | `pigeonhole-spi.json`, `thrash-pins.json` | `Infeasible` | all seven SPI buses; `SPI_BUS_3`, `SPI_BUS_4`, `SPI_BUS_5` |
| Budget spent (if S1.5 rules it in) | `pigeonhole-spi.json` under `fc` | `BudgetExceeded` | ``the search tried 10000000 values without an answer; it was placing bus `SPI_BUS_3` `` |

**The minimal set is affordable.** It is computed by deletion: drop each bus in document order if the rest stay infeasible. That costs n + 1 searches (measured: 6 for 5 buses, 8 for 7, 11 for 10). With `im+pm`, a whole refusal including the set takes at most 0.47 ms release and 3.5 ms debug. The result is *minimal* (irreducible), not necessarily *minimum*.

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S3.1 | Two buses hand-setting one instance is a refusal the WO does not list; without it, the search would silently move one of them | §0 S3 | missing rule | Add `PeripheralClaimedTwice` with both claimants | **yes** |
| S3.2 | `BudgetExceeded` names only the bus being placed, never a set of claimants. That is §7's "a refusal that cannot name its claimants", reached only if S1.5 rules a budget in. No such stop occurred in the spike | §7 | design tension | If a budget is wanted, accept the one-bus naming for that refusal alone | **yes** |
| S3.3 | `Infeasible` names bus **ids**, while the other refusals name JSON **paths**. Both are stable | §1.3 | design choice | `claimants` holds paths for pin and peripheral refusals, and ids for `Infeasible` | yes |
| S3.4 | A `cs` with no free pin left is unreachable on this part (112 usable pins), and the probe folds it into `Infeasible` | probe | note | The build gives it its own variant, or proves it unreachable in a test | no |

### S4 The `SolvedTopology` change

**Answer: the default holds, with two additions to `SolvedTopology::validate`.** The prototype is `SpikeTopology` in `spike/solver-probe/src/lib.rs`: today's fields, plus `gpio: BTreeMap<String, String>` mapping net to pin. The reference solves to:

```json
{"schema_version":1,"specification_sha256":"9a42e84f…","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},…,"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"}}
```

What `validate` must gain for the default to hold:
1. **`DuplicatePin` across both maps.** A `cs` and a bus signal must never share a pin.
2. **A net in both maps refuses**, as a new variant or as `DuplicateNetName`.
3. **Parsing:** `gpio` needs the same repeated-key refusal as `assignments` (`unique_keys` made generic over the value type), and identifier checks on its keys and values.

The alternative rejected is making `PinAssignment.alternate_function` an `Option<u8>`. A `cs` has no meaningful `peripheral` or `signal`, and a `null` that means "GPIO" is a guess dressed as data (`INV-06`).

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| S4.1 | The architect's default `gpio: BTreeMap<String, String>` is the smallest change that represents `cs`; prototyped and serialized | `SpikeTopology` | measured | Build the default with validate items 1 to 3 | **yes** (ratify) |
| S4.2 | `gpio` records no direction. Every entry is an output today; an input (an interrupt line) would need a value type, such as `{ pin, direction }`, which is a schema change | §0 S4 | note | Accept; not needed until a later work order | no |

## Notes for the ruling (outside §0)

- **Main moved during the spike:** PR #29 (`AET-WO-1500`, MCP) and PR #30 (`AET-WO-0300c` authored) merged after this branch was cut. The spike's diff (`spike/`, this file) does not conflict. The build half should rebase, and §1.4's MCP-tool-list test now applies: `AET-WO-1500` has merged.
- **The JTAG pins and others in the stress solve.** `PA15` (a JTAG-only pin, assignable per the ruling) is taken by a `cs` in `stress.json`. `PA11`/`PA12` (USB) and `PC14`/`PC15`/`PH0`/`PH1` (the crystal pins) are I/O in the table, and the search may give them to a `cs` or a bus. Clocks and USB are fenced, so this is a note, not a finding.
- **§7 conditions:** none hit. The search is byte-deterministic, there is no Z3 or C++, no `unsafe`, no `sudo`, and no secret beyond the push route. S3.2 is the one that would arise if a budget is ruled in.

## Gates (spike)

- Root, on Furnace: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass (257 passed, 0 failed). The root diff outside `spike/` and this file is empty.
- Probe: `spike/run.sh gates` passes.
- CI, first push (`03490cb`): `gates` and `tools` both green, https://github.com/ClearForge-LLC/aethelgard/actions/runs/42419462859 (PR #32). As S2 records, the probe was not in it.

## Build (after the ruling on PR #32)

### What was built

- **`aethelgard-ir`:**
  - `src/solve.rs`: `solve`, `solve_with_limit`, `SolveError` (with `kind()`, `id()` and `claimants()`), `RESERVED_PINS` and `NODE_LIMIT`, re-exported from `lib.rs`.
  - `SolvedTopology.gpio`, with `unique_keys` made generic over the value type, and `validate` extended (S4).
  - README: the model, the search order, the reserved pins and the SWD-only assumption, and the refusals in check order.
- **`aethelgard-cli`:**
  - `Command::Solve { file }`, placed after `Validate`, so the generated tool list is `validate, solve, check_drc, check_erc, prove, simulate`;
  - `fn solve`, which carries its own refusal envelope (`claimants`) so the shared `RefusalBody` stays untouched;
  - the README's `solve` section, its exit-code rows 0, 2 and 70, and the MCP tool list.
- **Tests:**
  - `aethelgard-ir/tests/solve.rs`: 21 tests on a synthetic `PinTable`. The real table cannot be used there, because `ir_depends_on_no_other_member` counts dev-dependencies.
  - `aethelgard-ir/tests/validate.rs`: 4 new `gpio` tests, and the existing fixtures updated for the new key.
  - `aethelgard-cli/tests/solve_cli.rs`: 11 end-to-end tests on the real table, over four fixtures copied byte for byte from the spike (`tests/fixtures/solve/`).
  - The two tool-list expectations, in `mcp.rs` and `mcp_tier1.rs`.
- **`spike/` deleted** (ruling 8).

### The ruling, as built

| Ruling | As built | Where |
|---|---|---|
| 1. `im` and `pm` before the search; no memo | Both run in `Search::consistent` at the root and after each bus is placed, together with the forward check | `solve.rs` `consistent` |
| 2. Node budget 10,000,000, `BudgetExceeded`, exit 2 | `NODE_LIMIT`, counted in nodes **across the whole solve, minimal-set searches included** (my reading: the ruling did not say per search). Per search, the worst case would be (buses + 1) × 10 M. Pinned by an exact-sum test | `the_budget_covers_the_whole_solve_minimal_set_included` |
| 3. `BudgetExceeded` names the bus being placed; `PeripheralClaimedTwice` accepted | As ruled; `claimants()` is that one bus | ir tests |
| 4. `cs` last, greedy; check order confirmed | As ruled; the order is in `aethelgard-ir/README.md` | `refusals_come_in_the_readme_order` |
| 5. S4: `gpio`; `DuplicatePin` across both maps; a net in both refused | The new variant is `NetAssignedTwice`, checked before `DuplicatePin` | `validate.rs` gpio tests |
| 6. `PC14`, `PC15`, `PH0`, `PH1` reserved | `RESERVED_PINS` has six pins. They are dropped from every candidate list and skipped by the `cs` pass, and a hand pin on one refuses `ReservedPin` | `a_hand_pin_on_each_reserved_pin_refuses`, `no_reserved_pin_is_ever_assigned` |
| 7. Probe out of CI; §1.4 is the `gates` measurement | `solve_cli.rs` pins the spike's digests. The build reproduces both (`c7f59a49…`, `759f4146…`), and its node counts match the spike's `fc+im+pm` exactly (7, 42, 44, 308, 41) | `gate_clause_4_…` |
| 8. Delete `spike/`, merge `origin/main`, keep `main.rs` to the command tree and `solve` | All `main.rs` changes are the `Solve` variant, its match arm and `fn solve`, using fully qualified paths so the import block is untouched. **`mcp.rs` is touched in its test module only**: the tool-list expectation, and the throwaway test's count from 6 to 7 | see below |

### Choices the WO and ruling left open (each flagged)

| # | Finding | Where | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| B1 | **The signature takes the hash:** `solve(&Specification, &PinTable, specification_sha256: &str)`. The WO's two-argument form cannot see the bytes that `INV-03` says to hash. Taking bytes instead would add `sha2` to `aethelgard-ir` and put parsing inside `solve`. The CLI hashes the bytes it read | `solve.rs` | deviation from the WO's signature | Accept | **yes** |
| B2 | `solve` re-runs `Specification::validate`, refusing with `InvalidSpecification` (which reports the inner kind). Without it, a duplicate net would collapse silently in the `BTreeMap`. It also checks the hash format first, and checks `mcu.part` against the table (`PartMismatch`) | `solve.rs` | addition (fail-closed) | Accept | no |
| B3 | A `cs` with no free pin gets its own variant, `NoFreePin` (spike S3.4). It is unreachable on the STM32H753ZI and tested on a small synthetic table | `solve.rs` | addition | Accept | no |
| B4 | **Claimants are signal paths, not field paths:** `i2c_buses[0].sda`, as the WO's own example has it, where the spike printed `….pin` for some refusals. Peripheral refusals name the bus path (`i2c_buses[0]`). `NoInstance`, `Infeasible` and `BudgetExceeded` name bus ids (spike S3.3, which was not ruled explicitly) | `solve.rs`, CLI README | contract choice | Ratify | **yes** |
| B5 | A vendored pin table that fails its own check (`SourceHashMismatch`, `MalformedSource`) is **exit 70**, with nothing on stdout and the reason on stderr, the existing meaning of 70 extended in the README's row. Only a corrupted build can cause it | `main.rs` `fn solve`, CLI README | contract | Accept, or name another code | yes |
| B6 | Root `Cargo.toml` gains `aethelgard-svd-hal` in `[workspace.dependencies]` (one line), and `Cargo.lock` the CLI→svd-hal edge. Neither file is on §2's protected list | root | scope touch | Accept | no |
| B7 | The test `mcp::tests::the_five_tools_come_from_the_tree` now checks six names. It was **not renamed**, to keep the overlap with `AET-WO-0300c` in `mcp.rs` to the expectation lines; `mcp.rs` then merged without conflict | `mcp.rs` | stale name | A one-line rename in any later CLI work order | no |
| B8 | The final `out.validate()` in `solve` is labelled `belt-and-suspenders:`. Its mutant survives by construction: the search never produces an output it would refuse | `solve.rs` | protected redundancy | Keep | no |

### §3 acceptance

1. **Every §0 answer is in FEEDBACK with its measurement:** the Spike section above.
2. **§1.4's tests pass on Furnace** (Gates below):
   - clause 2: `gate_clause_2_a_pin_set_twice_names_both_claimants_and_emits_nothing_else`. Exit 2, `PinClaimedTwice`, claimants `i2c_buses[0].sda` and `spi_devices[0].cs`, and no file written in the working directory or beside the input;
   - two solves byte-identical: `gate_clause_4_two_solves_are_byte_identical_and_match_the_spike`, also pinning the spike's digests;
   - each S3 refusal, at both the library and the CLI;
   - the stress specification within the 1 s bound. That is a bound, not a benchmark: debug takes about 1.5 ms;
   - the MCP tool list includes `solve`, through `AET-WO-1500`'s test (`mcp_tier1.rs`, merged) and the unit test in `mcp.rs`.
3. **CI green on the first push after the ruling:** cited in Gates.

### Measured on the build (Furnace, release)

- **Nodes**, from `solve_with_limit`'s smallest finishing budget (a scratch harness, not committed): reference 7, stress 42, `infeasible-shared` 44, `pigeonhole-spi` 308, `thrash-pins` 41. These are identical to the spike's `fc+im+pm`.
- **`aethelgard solve` end to end** (process start, file read, table load and solve; median of 21): reference 2.47 ms, stress 2.57 ms, `infeasible-shared` 2.44 ms, `pigeonhole-spi` 2.99 ms, `thrash-pins` 2.53 ms.

### For `architecture.md` §2, by amendment (the architect's to write)

- **`RUL-02`, as measured:** the pure-Rust search meets the bound by four orders of magnitude, so Z3 was not measured. Plain forward checking is exponential on infeasible input (329 s). Instance plus pin matching, before every node, makes every case found sub-millisecond, and a 10 M-node budget backstops the rest.
- **§2.8, the table's use:** reserved pins `PA13`, `PA14`, `PC14`, `PC15`, `PH0`, `PH1`; debugging assumed SWD only; key order is byte order.

### What was not built

Clocks and timing beyond bus speed bounds, voltage-domain routing, any emitter, a second part, and any change to `PinTable` or its source (§4). The pin table's own README was not touched, since that crate is fenced.

### Promote-or-needs-none pass over `AET-WO-1101` FEEDBACK

- **Finding 1, the datasheet cross-check of five AF entries:** still open. This WO adds no data and cannot close it. **Promote:** it should stay tracked until a session with datasheet access records the revision.
- **Finding 2, hash-check bypass:** verified there; needs none.
- **Finding 3, `_C` pads and ports above `K`:** needs none. The solver compares pins as table keys only, which is `A5` by construction.
- **Finding 4, debug pins:** ruled at review of PR #27 and at the spike ruling, and built here (`RESERVED_PINS`, `ReservedPin`).
- **Additions 1 to 6** (refusals on malformed source): needs none.

## Adversarial pass (§5)

This pass was run by a separate reviewer (Claude Opus 5.5, its own worktree, at `369b8c5`, whose search code matches the head's apart from the reduction pass's refactors). A1 was reproduced on `9c5fec6`.

Its harness:
- an independent verifier, checking every `Ok` against the table directly;
- an independent model of the checks before the search;
- an unpruned backtracking oracle in the README order, against which every `Ok` must be byte-equal;
- for every `Infeasible`, a check that the set is infeasible and that dropping any one bus makes it placeable;
- random generation, plus two hill-climbers seeking high node counts.

| # | Finding | Severity | Type | Recommendation | Decision-needed |
|---|---|---|---|---|---|
| **A1** | **An infeasible specification can still spend the whole budget.** When two instances' only remaining pin for one signal is the same pin, both matching checks pass: pin matching unions over instances, and instance matching ignores pins. An example is `SPI1_MISO` and `SPI6_MISO` both reduced to `PB4` by `cs` hand pins, while all six SPI instances are needed. The search then re-fails under every earlier choice. **Reproduced** with `aethelgard solve`, release, on `9c5fec6`: `fuzz-natural-6spi.json` (found by random fuzzing, with no hand peripherals) took 5.6 s, and `core-infeasible.json` took 6.6 s. Both refuse `BudgetExceeded`, naming `SPIB3`, the first bus of the colliding set to be placed, not the collision. Unbounded (400 M nodes), `fuzz-natural-6spi` is `Infeasible` across all six SPI buses after 18.7 s (measured). The reviewer measured about 43 s in debug, and 0.55 to 0.9 µs per node in release. **This refutes the spike's S1.3 claim that `im+pm` "removes every such case found":** the spike's fuzz never produced the shape. The bound holds and no answer is wrong. `BudgetExceeded` naming one bus is the ruled exception to §7, so this is not a §7 stop | medium | missing propagation | A further check, proposed by the reviewer and **unmeasured**: when the buses left of a kind equal that kind's placeable instances, every instance is forced, so run pin matching over (instance, signal) rows. It answers yes or no only, so it cannot change an answer. Not built: it changes the ruled design, and per the scope test it is the architect's call, as an amendment here or a follow-up WO. Reproducers: `docs/work-orders/AET-WO-1200.repro/` | **yes** |
| A2 | `BudgetExceeded` is exit 2, "the specification cannot be realized", which the solver has not proven. A1's specifications are in fact infeasible, but a budget refusal only means "not decided" | low | contract tension | Keep exit 2 as ruled, or give an undecided search its own code | yes |
| A3 | The budget covers the minimal-set searches, so the main search can prove infeasibility and the solve still return `BudgetExceeded`. The refusal kind then depends on the budget, not only on feasibility. This is documented (B-table, ruling 2) and deterministic | low | design consequence | Accept, or exempt the minimal set from the budget, which bounds the time less tightly | yes |
| A4 | `instances()` decides whether an instance is offered from every pin, reserved ones included, while `candidates` drops reserved pins. On this table no reserved pin carries an I2C or SPI signal, so it cannot trigger. If it could, an instance living only on reserved pins would be "offered" and then refused `NoInstance`, which is a correct refusal | none on this part | note | None now | no |
| A5 | **§5.1 determinism: nothing found.** Every fuzz case was solved twice in-process, and the CLI was run twice from different working directories and with `LC_ALL=C TZ=UTC` (`cmp` identical). There is no `HashMap`, `HashSet`, clock, environment read, thread or randomness in `solve.rs`, and `matching` returns only a bool | none | verified | none | no |
| A6 | **§5.2 false unsat, or an invalid result accepted: nothing found** in 508 000 random cases. About 188 000 solved. All were verified except 46 in the first run, where the oracle hit its 2 M cap; the 8 000-case run then verified unconditionally. Every solve the oracle finished was byte-equal to the unpruned first solution, so pruning never changed an answer. About 14 300 were infeasible, each with a correct minimal set. Every check before the search was exercised and matched the independent model. No feasible specification over the budget was found; the worst built by hand took 192 204 nodes and 149 ms | none | verified | none | no |

**§5.3 answer:** a bound is needed, and 10 M holds. A1 is a realistic shape that reaches it: about 5.5 to 7 s in release, and about 45 s in debug.

## Gates (build)

- **Furnace:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass at `069863c`, after merging `origin/main` `1b4fe5f`, which includes `AET-WO-0300c` (#34): 340 tests passed, 0 failed. Before that merge, at `203d76f`, it was 329. The merge's one conflict was the CLI README's exit-70 row; both meanings are kept. `spike/` is gone. The protected surfaces in §2 diff to empty against `origin/main`: the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, `container/`, `.github/`, and the five other crates. **§3.2's "on Furnace":** every §1.4 test is tier 1, and all are among the 329. They are `solve_cli.rs` (11), `aethelgard-ir/tests/solve.rs` (21), the four `gpio` tests in `validate.rs`, `mcp_tier1.rs`'s tool list, and `mcp.rs`'s unit tests.

**Tier 2 (`--features with-tools`) was also run on Furnace**, at `069863c`, after `AET-WO-0300c`'s reaper changes merged, with the local Kani 0.68.0 (`~/.cache/aet-0300-kani`) and Renode 1.17.0 (`~/.cache/aet-0300a-renode`) kits: **10 of 15 pass**. That covers all six `prove` cases, all three `simulate` cases, `mcp_prove…`, `mcp_simulate…` and `each_tool_removed_from_path_is_named`.

The five KiCad cases (`drc_…`, `erc_…`, `mcp_check_drc…`) fail `ToolMissing`, because Furnace has no `kicad-cli`. Installing it is a system package, which Furnace forbids (§7), so those five are proven only by CI's `tools` job inside the pinned image (green below). None of the 15 is a §1.4 test, and this diff touches none of their code. The command was `target/debug/deps/with_tools-<hash> --test-threads=2`, built with `cargo test --workspace --locked --features aethelgard-cli/with-tools --no-run`, run under `env -i` with the kits' `PATH`, `KANI_HOME`, `RUSTUP_HOME` and `CARGO_HOME`.
- **Mutation**, sized to the diff: one mutant per new guard, against `aethelgard-ir`'s suites and `solve_cli.rs`. **21 of 23 killed at first.** The two survivors were the hash check before the search and the final output check, each backing up the other:
  - A test was added (`a_malformed_hash_is_refused_before_the_search_finds_infeasibility`), and the first is now killed.
  - The second is labelled `belt-and-suspenders:` (B8).

  The mutants:
  - `im` off and `pm` off (killed by the adversarial fixtures' bound);
  - the budget off, and the budget per search (killed by the exact-sum test, which was added after the first test passed both ways);
  - reserved pins out of the candidates, the hand refusal and the `cs` pass, and one oscillator pin unreserved;
  - each of the five checks before the search, case-folded pin lookup (`A5`), the part check, and the re-validation;
  - `cs` from the end, pins in reverse, and the minimal set keeping every bus;
  - the net in both maps, `DuplicatePin` ignoring `gpio`, and the `gpio` identifiers.

  After the reduction pass, the four mutants on the guards it touched (budget off, budget per search, `PeripheralClaimedTwice`, `UnknownPeripheral`) are still killed.
- **Reduction pass (diff):** −7 lines net · 8 duplicates removed · 0 dead paths removed · 1 protected kept · 3 findings below.
  - The duplicates: the limit threaded beside `Budget`; the bus path built twice; two doc comments restating the README; four CLI README sentences restating `aethelgard-ir/README.md`.
  - The protected one: the final `out.validate()`.
  - The run was a fresh read-only subagent. Each removal was checked by the suites and the touched guards' mutants.

  Findings, not applied because each needs an edit outside `fn solve`, which ruling 8 fences:
  - R1: `fn solve` repeats the sha256-to-hex fold that `input::hex` already holds (3 lines; `input.rs` would need `pub(crate)`);
  - R2: the solve-local `SolveRefusal` mirrors `Refusal` (about 11 lines; `RefusalBody` would gain an optional `claimants`);
  - R3: `fn solve` and `fn validate` share their read-parse-validate block (about 12 lines, as a shared helper).

  They are worth doing after `AET-WO-0300c` merges.
- **CI**, first push after the ruling (`824acad`), before `AET-WO-0300c` merged: `gates` and `tools` both green, https://github.com/ClearForge-LLC/aethelgard/actions/runs/42423285608 (PR #32). `gates` ran the new tests in its debug profile, so clause 2, clause 4's solver half with the spike's digests, the stress bound and the six-tool MCP list all passed there. Live acceptance is not done; it is the operator's.
- **CI after merging `AET-WO-0300c` (`ae04803`): `gates` failed once, on a test this diff does not touch.** The test is `mcp_tier1::servers_sigkilled_while_calls_start_leave_nothing`, from `AET-WO-0300c` (#34). The log reads `survivors [], leftover directories ["aethelgard-7773-1794093670722372015"]`: one `check_drc` call's scratch directory outlived its SIGKILLed server. https://github.com/ClearForge-LLC/aethelgard/actions/runs/42424760372

  On Furnace the same test passed **30 of 30** at `ae04803` (its binary run alone, 30 times), and it passed in `main`'s own CI at `1b4fe5f`. `solve` creates no scratch directory and never reaches the guardian. This diff's only change to `mcp_tier1.rs` is the tool-list line.

  | # | Finding | Severity | Type | Recommendation | Decision-needed |
  |---|---|---|---|---|---|
  | C1 | `AET-WO-0300c`'s parent-kill test is timing-sensitive on a loaded runner. Rarely, a guardian does not remove its directory, which is an escape in 0300c's own guarantee, not a test artifact. This was not retried away silently. The next push (`cbae67d`) re-ran CI with no other change, and `gates` and `tools` were both green: https://github.com/ClearForge-LLC/aethelgard/actions/runs/42425262854. One failure in three runs including `main`'s, and none in 30 on Furnace | medium | finding for `AET-WO-0300c`'s owner | **Ruled by the architect:** this is `AET-WO-0300c`'s guardian residual (`run.rs` `guard_main`), not `AET-WO-1200`'s, and it is handed to `AET-WO-0300d` | no |
- **Rootless podman (one line, per the architect):** podman 5.7.0 is now present for `forge` on Furnace, where `AET-WO-0200` had measured no container runtime. The pinned tools image was pulled (24 s, about 4.8 GB, kept in `forge`'s podman storage), but nothing ran in it: by ruling, tier 2 is proven in CI (roadmap §6).
