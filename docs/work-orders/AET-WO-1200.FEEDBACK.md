# AET-WO-1200 — FEEDBACK

**Branch:** `wo/aet-1200-solver` · **Base:** `2f13278` (PR #28, the base the kickoff named; `ReservedPin` present in the WO) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Result:** spike only. Every §0 question has a measured answer below, except S2's `gates` half, which has a recorded reason. The build half has **not** started; this branch is stopped for the architect's ruling.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Spike

Probes are under `spike/` (`spike/README.md`). Every command is `spike/run.sh` from the repo root, and the recorded output is in `spike/results/`:
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
- CI: pending; the first push's run URL is added below once it reports.
