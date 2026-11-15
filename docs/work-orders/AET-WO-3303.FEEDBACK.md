# AET-WO-3303 FEEDBACK — package preconditions, the harness split, the pair judge's allowlist, a strict number reader, `main` kept green

**Status: in progress, stopping on D1** (§7, the second flag-and-stop condition) on branch `wo/aet-3303-preconditions`, from `main` at `4246200`. Campaign coverage is judged and stated in every package's verdict, and **not enforced** until D1 is ruled: enforcing it today refuses every board with an SPI bus, the reference among them. Everything else in §1 is built, and §1.2's measured stop did not fire: the split proves in seconds.

The predictions in §1 were committed before any container run, any measured run of the CLI and any Kani run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source, this repository at `4246200`.
- **[measured]**: run. On the host for tier-1 behaviour and refusals that come before any tool; in the pinned tools image (`container/IMAGE`) under rootless podman inside a capped user scope (`~/bin/capped-run`, caps read back) for Kani, KiCad and Renode.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the plan

- **A6 [read].** `cli/src/verify.rs:489-517`: `harness_plan` drops `proofs::every_valid_sequence` on a board with motor outputs and `skipped_finding` reports it at `info`; `report.ok` (`:726-728`) counts only `error` findings and gate verdicts, and `package` (`:784`) writes when `report.ok`.
- **The harness list [read].** `verify::harnesses` reads `src/proofs.rs` line by line: a `fn` after a `#[kani::proof]` line. A harness whose attribute is not on a line of its own, or whose `fn` line is not recognised, is not in the plan and not proved.
- **Coverage [read].** The campaign's override kinds (`chaos/src/record.rs:37-50`): `PullUpOhms` and `LineHeldLow` (an I2C bus), `MaxCapacitance` (a constrained net), `Droop` (a domain), `SpiDeviceAbsent` (an SPI device). `cli/src/campaign.rs:271` refuses a droop on any domain but the MCU's supply (`DroopNotMcuSupply`). No kind names an SPI bus, a USART or a rail other than the MCU's supply.
- **The no-fault case [read].** `scenarios/sentinel-01/pullup-2k2.json` sets `SENSORS` to 2200 Ω, the reference's declared value: every package test today (`fab_tier1`, `against_fab_tier2`, `verify_2k2_tier2`) packages a campaign with no fault in it. `verify_47k_tier2` packages one fault, on `SENSORS` only.
- **A9 [read].** `cli/src/main.rs:786-809` runs `route::read`, `connectivity`, `check` and `pairs` on the strict tree with no copper allowlist; `extract` has one.
- **A11 [read].** `eda/src/sexpr.rs:101`: `Sx::arg` returns an atom or a quoted string alike, so a quoted `"0.2"` reaches `nm`. `nm` (`:378-398`) strips one `-` and hands the rest to `i64::parse`, which takes a sign of its own: `--5` and `+0.2` read. `extract.rs:600` and `route.rs:3426,3672` recognise copper by `ends_with(".Cu")`.
- **A15.1 [read].** `pincheck/src/elf.rs:160`: `position(|&b| b == 0)` over the whole tail of the string table, once per symbol.

## 1. Predictions (committed before any run)

- **P1, the split (§1.2).** On `motors-pwm.json` and `motors-dshot600.json`, one harness per peripheral and per timer (I2C1, SPI1, TIM1 and TIM2 or as solved), each with every order of its own actions and a per-write frame assertion, finishes in under 5 minutes in the image on Furnace, and the sum of the parts is under 10 minutes. [inferred: the targeted timer harnesses verify in seconds (`4300a` §4); the whole reference schedule takes 121 s with I2C1 and SPI1 together]
- **P2, the split keeps the claim.** A mutant that makes one unit's start write a word or bit another unit owns (an I2C start writing a TIM1 register; a timer start writing a chip select's BSRR bit) fails its part on the frame assertion, though the whole-schedule oracle of the other unit alone would not see it. [inferred]
- **P3, the reference covers.** The reference with `pullup-47k`, `spi-absent-u4` and `droop-tolerated` has a fault on `SENSORS`, `FLASH` and `VDD_3V3`, and every precondition is met. [read]
- **P4, coverage cannot be met everywhere (§7).** `connectors.json` declares `VDD_5V` (not the MCU's supply) and the SPI bus `EXT` with no device: no override kind can fault either. A droop on `VDD_5V` is refused `DroopNotMcuSupply`, and `spi_device_absent` cannot name `EXT`. Sentinel-02's power fixture (`ir/tests/fixtures/sentinel-02-power.json`) declares `VDD_5V` and `VDD_1V8` beside the MCU's `VDD_3V3`. This meets the second flag-and-stop condition: D1. [read]
- **P5, campaign cost.** Each added scenario costs one Renode boot of the reference, about 15–40 s on a CI runner, so a packaged verify with three faults instead of one costs about one more minute. [inferred]
- **P6, numbers.** Every numeric atom KiCad 10.0.7 writes in the committed boards, the strict corpus and the vendored footprints matches `-?[0-9]+(\.[0-9]{1,6})?`; none uses an exponent, a leading `.` or `+`. [inferred]
- **P7, the version.** The `(version …)` KiCad 10.0.7 writes into a board is `20260206`, the emitter's `pcb::VERSION`. [inferred: the emitted boards are KiCad-loaded byte-stable today]
- **P8, the arc stub.** `check drc --spec` on `pair-standin.json`'s board with one `(arc …)` on `/SPI_SCK` is refused `UnmodelledCopper` once extraction's allowlist runs first, and the pair report of the unmodified board is unchanged. [inferred]
- **P9, the ELF bound.** With the compare bounded to `SYMBOL.len() + 1` bytes, a 16 MiB ELF of about 400,000 symbols pointing at one unterminated 8 MiB name is refused or read in under a second, against minutes before. [inferred]

## 2. Decisions needed

### D1 (§7): campaign coverage needs fault kinds the chaos crate does not have, so no board with an SPI bus can be packaged

**What the rule meets [read, measured].** `verify` passes only when every scenario's own verdict is `pass`: "A scenario that expects a fail can never let `verify` pass" (`cli/src/verify.rs`, the campaign gate, `AET-WO-3301` §3.2). So a package's campaign can hold only faults the design survives, and coverage means *a survived fault on every declared bus and rail*. What the override kinds give, on the reference:

| Declared | A fault the design survives | Source |
|---|---|---|
| I2C bus `SENSORS` | `pullup-3k3` (3.3 kΩ against the declared 2.2 kΩ, expects pass) | [read] `scenarios/sentinel-01/seeded/pullup-3k3.json` |
| rail `VDD_3V3` (the MCU's supply) | `droop-tolerated`, `droop-bor3` (expect pass) | [read] |
| SPI bus `FLASH` | **none.** `spi_device_absent` is the only SPI kind, and an absent flash fails the identity read (`spi-absent-u4` expects fail). A changed `max_capacitance_femtofarads` on `SPI_SCK`/`SPI_MOSI` either is refused by `solve` (`ir/src/solve.rs:630`) or moves only the solved `OSPEEDR`, which the firmware built from `--spec` then disagrees with [inferred] | [read] |

Counting any seeded fault, the reference covers (`pullup-47k`, `spi-absent-u4`, `droop-bor3`), but then its campaign gate fails and nothing is written. Counting only survived faults, `FLASH` is never covered. Either way **no board with an SPI bus gets a package** while the rule is enforced: the reference, the four-layer fixture, both motor fixtures, and Sentinel-02, for which `roadmap.md:312` asks both "a fault campaign that includes an SPI fault" and the fabrication package.

**Further rails and buses no kind reaches [measured on the host, `target/debug/aethelgard` at `0c23837`].**
- A rail other than the MCU's supply: a droop on `VDD_5V` of `connectors.json` is refused, `DroopNotMcuSupply` (`cli/src/campaign.rs:271`). Sentinel-02's power fixture (`ir/tests/fixtures/sentinel-02-power.json`) declares `VDD_5V` and `VDD_1V8` beside the MCU's `VDD_3V3`.
- The battery, when a power tree is declared: no kind names it [read].
- An SPI bus no device uses (`connectors.json`'s `EXT`): `spi_device_absent` names a device, and `EXT` is refused, `UnknownDevice`. A bus no device uses is named uncovered whatever is seeded on it: a fault there judges nothing (§5.2's first attack). This reading is mine; an I2C bus with no device is treated the same.
- Pending: `4300b` (PR #152) brings `serial_ports`; a USART becomes a declared bus with no override kind unless #152 adds one (asked in its inbox, 16:48 UTC).

**What is built meanwhile.** `verify::uncovered` judges coverage from the specification and the scenarios (a value equal to the specification's own, a droop floor at or above nominal and a bus no device uses cover nothing). Its list is the verdict's `fab.preconditions` row `campaign_faults_every_bus_and_rail`, `unmet` with each name and why. It does **not** refuse: wiring the refusal turns every tier-2 package test red and leaves the rest of §1.8 untestable end to end. Flipping it is one `if` before any gate (the refusal `CampaignCoverageShort` was built, measured on the host and then unwired; its tests are now unit tests on `uncovered`).

**Options.**
- **(a)** A fault scenario that fails exactly as it expects counts as covering *and* passes the campaign gate: "verified" becomes "survived every tolerable fault and detected every intolerable one". It reverses `3301` §3.2, which is the architect's.
- **(b)** Survivable kinds where none exist: an SPI fault the design can survive, and a droop on any rail (with its own judge). A chaos/twin work order; packages wait for it (`4300d2`'s D5 (c) is the precedent).
- **(c)** A ruled exemption where no survivable kind exists, named in the verdict's row instead of `unmet`.

Blocks: §2's "the reference package's `VERDICT` lists every precondition as met" (it lists coverage `unmet`: `SPI bus FLASH`), and every multi-rail or SPI package. Nothing else in §1.

## 3. What was built, and what was measured

### §1.1 Preconditions
- `--fab-out` and `--against-fab` refuse `HarnessSkipped` before any gate when the plan leaves a harness out: today the motor board's dropped `every_valid_sequence` **[measured, host: `motors-pwm.json` exit 3, `HarnessSkipped`]**, and any `kani::proof` attribute in the board crate that the plan's line reader does not name (`unplanned`, unit-tested with an attribute and `fn` on one line and a harness in another file).
- Every package verdict carries `fab.preconditions`: `every_harness_proved` (each planned harness has a `prove` gate that passed; a timed-out or failed harness is named), `campaign_faults_every_bus_and_rail` (D1: stated, not enforced), `every_gate_passed` (each gate not `pass`, and each error finding). It sits inside `fab`, so the manifest's `verdict_sha256` line is unchanged. `4230` appends its ledger row to the same list.
- **P3 was wrong [read].** It counted `pullup-47k` and `spi-absent-u4`, which fail, and a package needs every scenario to pass (D1).
- **P4 held [measured]**, and D1 grew beyond it.

### §1.2 The whole-schedule harness, split (an Opus subagent; its diff and the runner log checked by me)
- **The measured stop did not fire [measured, in the image].** On a board with a timer, `every_valid_sequence` becomes `every_valid_sequence_<unit>`: one part per I2C or SPI peripheral (its pins and chip selects), per timer (its channel pins and the reset-clock read), and `_gpio` for an output no unit owns. A board without a timer emits the whole harness byte for byte as before (every timerless fixture and `boards/sentinel-01` regenerate identical).

  | Part (wall clock through `aethelgard prove`, compile included) | `motors-pwm` | `motors-dshot600` |
  |---|---|---|
  | `every_valid_sequence_i2c1` | 12.4 s | 12.9 s |
  | `every_valid_sequence_spi1` | 36.0 s | 37.1 s |
  | `every_valid_sequence_tim1` | 43.6 s | 43.7 s |

  The reference's whole harness: 129 s, unchanged. A throwaway board with an extra output (no specification can declare one) proves `_gpio` in 2.6 s. **P1 held** (each part under 5 min, the sum under 10).
- **Why the parts are the same claim (rely–guarantee, mechanised).** Each action has a footprint, the bits the whole oracle lets it change, computed from the plan and the register facts. In a part, every write asserts it changes only the running action's footprint (a stored `BSRR` is judged on the value written), and an access outside the part's blocks fails; before each step every bit of another owner's footprint is havocked (the rely); every other bit is asserted unchanged directly. `generate` refuses footprints that overlap between owners (`FootprintOverlap`; each port enable is its own owner) and an action of the whole schedule in no part (`UnscheduledAction`). Sketch: a whole schedule is a permutation of take-once actions, and its projection on a part's actions is a schedule of that part; up to the first write outside a footprint, every bit another owner changed is one the rely may choose, so the part's step runs as in the whole, and its assertions (a superset of the whole's on its blocks) hold; at the end each owner's bits were last changed by its own actions. The parts are strictly stronger than the old claim: a set-then-restore of another unit's gate bit fails them, and would pass the old end-state oracle (not run on the whole harness, which does not finish).
- **P2 held [measured]:** each mutant fails its own part. (a) I2C start writes a TIM1 word: "a block outside the i2c1 part was accessed"; (b) TIM start sets the chip select by `BSRR`: "GpioA BSRR write touches a pin no solved output of the tim1 part owns"; (b3) the same through `ODR`: "a write changed a bit outside the running action's footprint"; (c) `enable_tim1` emptied: "Tim1 accessed before RCC_APB2ENR bit 0 is set"; (d) set then restore TIM1's gate in `enable_i2c1`: the footprint assertion; (e) `kani::assume(false)` in `_tim1`: `cover_unsatisfied`, 0 of 3.
- **Not measured:** a board with two timers (their shared reset-clock read is argued, not run).
- `verify` lost `harness_plan`'s drop and its `info` finding: nothing is skipped. `twin_tier2::kani_proves_every_part_of_both_motor_fixtures` proves all six parts in CI (weighed 240 s; TIER2_MIN 127, `main`'s 126 and this test).

### §1.3 The pair judge's allowlist
- `check drc --spec` with declared pairs now runs `extract::read_tree` (extraction's whole allowlist, the version pin, `declared`, `route::read`, `connectivity`, `check`) on its own strict reading, then `pairs`. `extract::read` is `read_tree` on a fresh reading, so the two cannot differ.
- **P8 held [measured, host].** `pair-standin.json`'s emitted board reaches the tool lookup (`ToolMissing`, exit 4, so every check before KiCad passed); with one `(arc …)` on `/SPI_SCK` it is refused `UnmodelledCopper` ("a top-level `arc`, which the bound does not read"), exit 3, before any tool.

### §1.4 Strict numbers (a Sonnet subagent, checked against its diff)
- `sexpr::nm` accepts exactly `-?[0-9]+(\.[0-9]{1,6})?`; `strict` refuses a quoted value or a value outside the grammar in every numeric child it reads (`at`, `start`, `end`, `mid`, `center`, `xy`, `size`, `width`, `thickness`, `roundrect_rratio`), naming the path (`Refusal::Number`). U+2212 never reaches it: the tokeniser refuses the byte.
- **A11's subagent items [read]: all real.** The version was never pinned; a copper row's user name passed `declared`; `ends_with(".Cu")` at `extract.rs:600`, `route.rs:3426` and `:3672`. One `strict::copper_layer` (`F.Cu`, `B.Cu`, `In1.Cu`…`In30.Cu`) now serves all three.
- **Deviation, stated:** KiCad 10.0.7 itself writes a fourth element on non-copper rows (`(9 "F.Adhes" user "F.Adhesive")` in the reference), so the fourth element is refused on a copper row, or on any row whose user name is a copper name, not on every row.
- **The version pin** (`20260206`, `pcb::VERSION`) is on `extract`'s path (`read_tree`), so it also holds on `check drc --spec`'s pair path. It is not in the shared strict reader: `check drc` without a declared pair, and its container fixtures (20240108, 20231120), let KiCad judge the version. **P7 held [measured, in the image]:** `kicad-cli pcb upgrade --force` (KiCad 10.0.7) re-saves `container/fixtures/drc-clean.kicad_pcb` from `(version 20240108)` as `(version 20260206)`.
- **P6 held [measured, host]:** 25,256 numeric atoms in the 32 committed KiCad files (boards, schematics, the 10 vendored footprints, symbols): none refused.
- **The strict corpus:** `board/control/user_layer_table_only` (1306 F10's control, which KiCad loads and passes) is now refused by `drc-parity` and `extract` (`LayersDisagree`), so it moved to a new class `layer-name`; `strict_tier1` pins it. `unmodelled-item/user_layer_name` and `gr_rect_top` now give `LayersDisagree` on `drc-parity` (before, KiCad's `ToolError`); `extract` still refuses them `UnmodelledCopper` first. The reference board remains the control (`the_reference_reaches_every_gate`).

### §1.5 The ELF compare
- The string table must end in NUL (so every name in it is terminated), then each symbol's compare reads at most `SYMBOL.len() + 1` bytes.
- **P9 [measured, host, debug build]:** 400,000 symbols naming one 8 MiB name: `check.rs`'s whole binary 0.10 s. The old scan was not timed; its cost (≈3 × 10^12 byte steps) is the reviewer's arithmetic.

### §1.6 Tests
- **A12 [measured]:** `extract::tests::a_lines_total_holds_every_term_and_the_offboard_budget` pins a connector line's total in `aethelgard-eda`; with `terms.offboard` dropped from the sum it fails. A Sonnet subagent's attempt in `eda/tests/` was denied by the permission classifier (it needed a `serde_json` dev-dependency and a `workspace_shape.rs` change); not worked around: the test is a unit test in `extract.rs`.
- **The missing-key walk** (`ir/tests/validate.rs`): every key of every object of one full document (Sentinel-02's power fixture with connectors, classes, a pair and motor outputs added) removed in turn, more than 250 keys, each refused. `/power/crossings` and `/accept_lifecycle` stay empty, each named with why. The per-section loops it subsumes were deleted; the closed-set, null and bound halves of those tests were kept.

### §1.7 The quickstart
- On a `main` run, a copy of the block has its `IMAGE`, `SPEC` (`file://` of `main`'s own `examples/sentinel-01-minimal.json`) and checksum lines replaced, each required exactly once, every other line required unchanged, and runs against the commit's image. The README block then runs as written against its own pin, on every run.
- **[measured, locally]:** the step's script, extracted from the YAML and run against this README with a stand-in digest, changes exactly `IMAGE` and `SPEC` (the example's hash is still the pinned `03a82a04…`), and with an edited example all three lines; the substituted `curl` and `sha256sum -c` pass. The `main` path itself runs only on `main`, after merge (the architect checks).

### §1.8 The backlog
- **`4300f` F2:** `FAB-NOTES.md` states the `extract` gate's own `conditions`, verbatim and in order, taken from that gate's verdict in the same run, so it can never state fewer (the reference's notes gain `SPI_CONDITION`). `Notes.offboard` is gone.
- **`4210` F-B3:** `FAB-NOTES.md` gains a `## Lifecycle` section, only when the verdict states a line: the banner, then each part whose lifecycle no held page states. The reference's W25Q64JV is pending, so the reference's notes gain the section.
- **`3300` F21:** `fab::LAYERS` carries each Gerber's description; `describe` reads it, and the inner layers' text is generated for each (the old match named `In3.Cu` and beyond "board outline").
- **`4300f` A3, A4:** `validate` refuses a connector named `U`, `R` or `C` and a number in any case (`ConnectorReferenceReserved`), and an off-board budget above 1 µF (`ConnectorBudgetTooLarge`, `connectors::MAX_OFFBOARD_FEMTOFARADS`). **Fence deviation:** both are in `aethelgard-ir` (`connectors.rs`, `lib.rs`), outside its "tests only" fence: the CLI may not read a specification's connectors (`clippy.toml`'s `RUL-16` rule on `declared_connectors`), and every other connector refusal is `validate`'s step 15. The ir README's step-15 row names both.

### §1.9 Crate READMEs (a Sonnet subagent, checked against its diff)
- `cli/README.md`: seventeen tools (`mcp_tier1::the_readme_names_the_real_tool_count` runs `tools/list`); the solve example's `schema_version` 6 and `03a82a04…` (`solve_cli::the_readme_example_line_names_the_real_version_and_hash`). `ir/README.md`'s version lines 8 (`ir/tests/readme_versions.rs`, std and ir only).

### §1.10 A16, verified at source (a read-only Sonnet subagent; claims 1, 3, 4 spot-checked by me at the lines)
| Claim | Verdict | Done |
|---|---|---|
| `route::part`'s circle `w.min(h)/2` | **real**: KiCad draws a disk of the x size, so a 1.0 × 0.6 circle was priced 0.4 mm narrow: a false pass on `check drc --spec`'s pair path (`extract` already refused it) | a circle that is not square is refused; unit test |
| No minimum coupled length | **real, a design gap**: `DiffPair` has no length field; a pair of conductors at most ≈8 mm can pass with a short coupled run (4 mm uncoupled each end); longer ones need at least L − 8 mm | none: backlog (§9) |
| `via1` / `track_x` references | **real, false refusal only**: pads of a part whose reference starts `via`, `track` or `thermal` were held to the 0.5 mm edge rule | items after the pads are chosen by position; unit test, the string mutant fails it |
| `connectivity` O(segments × pads²); no item cap | **partly**: the `nth(j)` walk made each pad lookup O(pads); there is no item cap in `extract` either (the claim's premise); the input is bounded by 16 MiB and 1 M tokens | pads indexed once; no cap added (backlog §9). The strict reader's path strings: real and bounded by the token cap (≈1 GB estimated, not measured): backlog |
