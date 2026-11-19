# AET-WO-3305 — FEEDBACK (STOPPED at §1.3, under §7's first condition)

**Branch and inputs.**
- Branch `wo/aet-3305-witness-elsewhere`, from `main` at `35d23eae`.
- The work order is read from `~/.cache/aet-3305-wo.md`, the architect's copy; the grounds are `2304b`'s F-MASK (#254).
- **No crate, test, golden, scenario, specification, workflow, `ci/` or `container/` file changed.** The diff is this file and `spike/3305/`.

Each statement is labelled **measured** (run on Furnace in the pinned tools image, `capped-run`, nice 10), **read at source** (file:line at `35d23eae`), or **inferred**.

## 0. Status: STOPPED at §1.3

The archived record cannot tell this witness's own transfers from another step's.
- Every I2C witness on a bus carries the same list: every address the controller found no device at during the whole last boot.
- Nothing in the trace places an I2C transfer in a step: no per-step time, position or address.
- "Elsewhere" as the work order defines it ("an address this witness's own step addressed") cannot be computed in `evolve` from what is recorded.
- The fix was not built. §1 and §2 below are the record the stop asks for: the record's fields, the measurement, and the change I would need.

## 1. Read first (§1.1)

### 1.1 Where `absent` is filled, and over what window (read at source)

1. **Renode's line.** The controller logs `[WARNING] i2c1: Unknown slave at address N.` (N in decimal) for each transfer it starts to an address with no device behind it. `absent_address` reads it whole (`crates/aethelgard-cli/src/renode.rs:1827-1840`).
2. **The window is the whole last boot, per bus, not per step.** `Observed.absent` (`renode.rs:2119-2129`) gives every I2C check the same list: the `Unknown slave` lines of its controller over the last boot. The field's own doc says "during the run" (`renode.rs:857-862`).
3. **The archive** writes that list into each kind 13 (`campaign.rs:1243-1257`), at most `MAX_WITNESS_LINES` (16) entries (`chaos/src/record.rs:404-407`, `:769`).
4. **The CLI's I2C witness attribution has no step window either.** A device's own lines go to its steps by count: each step takes as many lines as it expects, and the last step takes the rest (`renode.rs:960-967`, `:1043-1078`). On SPI there is a chip-select window (`:1003-1041`); on I2C there is none.

### 1.2 Whether the record can tell this witness's transfers from others' (read at source, then measured)

**The record's fields:**
- kind 13 `I2cWitness`: `peripheral`, `reference`, `device`, `step`, `expected`, `observed`, `absent: Vec<u8>` (`record.rs:396-407`);
- kind 12 `I2cCheck`: `reference`, solved `address`, `step`, `write`, `width`, `expected`, `mask`, `observed`, `status` (`record.rs:380-390`);
- kind 15 `I2cBusSolved`: the declared devices.

**No per-step timing exists.** Both kinds are archived at the boot's end time, the same stamp for every record read at the end (`Record::timestamp`, `record.rs:1042-1049`).

**No I2C register access reaches the trace.** The logged peripherals are only the serial ports' USARTs, their GPIO ports and the RCC (`renode.rs:519-531`). `CR2` holds the address each transfer starts with, and no write to it is archived.

**Measured** (`spike/3305/out/records.txt`, two-bme, §2): both kind 13s carry the same `absent` in both runs. Faulted: `[118, 119]` for U2 and for U5. Wrong address: `[117]` for U2 and for U5. Nothing in the kind 12, 13 or 15 records differs per witness.

**Order is not enough.** `absent` is in log order, but mapping its entries to steps trusts the firmware under test (§4, R4): a guess, which §1.3 forbids.

## 2. Measured on `main` before any change (§1.2)

- **Board.** Sentinel-03 at `main` cannot run a campaign. **Measured:** `PowerNotSimulated`: "the specification declares a power tree, which the twin does not model" (`2304b` adds that). So the fixture is the committed test specification `crates/aethelgard-cli/tests/fixtures/two-bme.json`: two modelled BME280s on SENSORS, U2 at 0x76 and U5 at 0x77, and an unmodelled U3 at 0x48. Its firmware reads each BME280 once, in order (identity steps; `spike/3305/out/records.txt`).
- **Commands.** `spike/3305/sh/gen.sh` and `run.sh`: `main`'s CLI and bridge built in the image, `generate`, `build`, `campaign`, then `attribute`. `run.sh` and `collide.sh` write each scenario, as run, beside a copy of the fixture. The archives are committed (`spike/3305/out/archives/`, replayable). `spike/3305/dump` prints their I2C records.

| Run | Firmware | Verdict (campaign) | `attribute` | Kind 13 `absent` (U2, U5) |
|---|---|---|---|---|
| baseline | reference | pass, matched | none N1 | `[]`, `[]` |
| held-low-sda | reference | fail `i2c_line_held_low`, `i2c_timeout` ×2, matched | electrical E1, evidence `[5,9]`, **masked `[28,13]`, `[29,13]`** | `[118,119]`, `[118,119]` |
| wrong-address (U2 → 0x75) | `2204`'s patch shape, U2's address replaced, but 0x75, not 0x77: 0x77 is U5 on this board (`out/wrong-address-mutant.diff`) | fail `i2c_timeout`, matched | software S1, evidence `[4,7]`, `[5,7]`, **`[28,13]`** | `[117]`, `[117]` |

- **The bus fault would be refused by `detection`.** E1 with both witnesses masked is measured; `verify` was not run on this fixture. The refusal is read at source: E1 with a non-empty `masked` is refused at `verify.rs:990-1002`, and `2304b` measured that refusal on Sentinel-03 at `ba8fb43b` (#254 F-MASK): "attribute masks … on the faulted bus: something else fails too". This is F-MASK on a second board: each witness's `elsewhere` is the other device's address (`evolve/src/lib.rs:1006`).
- **The wrong-address mutant is caught (measured on `main`).** S1 cites U2's witness.
- **New finding (read at source and confirmed by the measured records).** The shared list also marks **U5's** address field wrong in the wrong-address run (`absent` `[117]`, U5 at 119). U5 read correctly. It is not cited only because S1 filters software evidence to the symptom's device (`lib.rs:1876-1877`). Under E1 (`software(b, None)`, `lib.rs:1873`), every witness on the bus is read this way. That is the same defect seen from the other side: any absent address on the bus becomes every witness's wrong address.
- **The raw Renode log** (`spike/3305/out/raw/`) comes from a scratch build that printed the last boot's log (`sh/rawlog-instrument.diff`, never committed to a crate). What the log holds between transfers on a faulted bus:
  - `Unknown slave at address 118.`, then the firmware's flag clear after the timeout (`i2c1: Unhandled write to offset 0x1C … 0x30 … NACKCF`), then `Unknown slave at address 119.`, then the flag clear again;
  - no line names a step;
  - the stamps are quantum-coarse (both `Unknown slave` lines sit at 0 and 3.6 ms), so time alone is too coarse to be a step boundary at this quantum: **inferred** from the stamps.

- **F-COLLIDE (measured, §5's first case on a healthy bus; outside this fence).** U2's read sent to 0x77, where U5 answers (`sh/collide.sh`, archive `out/archives/collide-0197d90b`, `out/attribute-collide.json`).
  - **The twin's judge fails the boot:** `i2c_transaction_unwitnessed` ×2.
  - **The records.** Both kind 13 `absent` lists are `[]`. U2's witness is empty while its record claims `ok` (silent, so wrong). U5's witness carries both transactions, four lines.
  - **`attribute` names indeterminate I4, not software.** U5's symptom decides I4, because its fields compare only the first logged write and read (`lib.rs:987-989`), and any indeterminate rule wins the run (`lib.rs:2488-2489`).
  - **No address rule touches this case,** old or new: no address went unanswered. This is an attribution gap of its own, for the architect (a witness carrying another step's lines is not judged wrong).

## 3. The stop (§7, first condition): the change I would need

**What is missing:** a step boundary in the log, attributed by the twin before archiving, so that each kind 13 carries only its own step's absent addresses. Each candidate is listed with its fence, its effect on the trace, and what it hides.

- **(A) Recommended: hook each I2C step's record and attribute by position.**
  - **Mechanism (proposed; not built or run here).** The twin hooks each I2C step check's status word in RAM. The boot word is already hooked this way (`anchor.rs:307-316`, `sysbus AddWatchpointHook 0x20000000`). Each check already knows its status word: `StepCheck.status_at`, the first record at boot word + `RECORD_OFFSET` (8, after the brown-out word), each record after the last (`twin/src/boot.rs:138-141`, `:638-649`). Each `Unknown slave` line goes to the step whose status write first follows it in the log. A line after the last write goes to the bus's last step, as the witness attribution's last step takes the rest (`renode.rs:1053-1057`).
  - **The rule then.** `evolve`'s rule becomes the work order's exactly: `elsewhere` is any address in this witness's own list other than its device's.
  - **Fence.** `cli` `renode.rs`/`anchor.rs` (the twin's judge), the record doc in `chaos`, `evolve`. Outside `3305`'s fence.
  - **Trace.** kind 13's field shape is unchanged (`Vec<u8>`), but its meaning narrows from "during the run" to "during this step". **Recommend a trace version bump with it,** so an old archive is never read under the new meaning. This is the architect's call (`2304b` is moving the trace to 15).
  - **What moves (inferred).** Every committed board on `main` has one I2C step: `boards/sentinel-01` and `-bench`, one `i2c1.transfer`, 0x76. A one-step bus's per-step list equals its whole-run list, so no committed board's kind 13 content moves; only the version byte, if bumped. The lists that would shrink are those of multi-step or multi-device faulted buses: Sentinel-03 (U7's three steps), `sentinel-01-ext-bmp581`, and this fixture. To be measured at build.
  - **What it still cannot tell.** A firmware whose step writes its status before its transfer (out of order) puts that step's absent line on the next step. A wrong address that collides with another real device's leaves no `Unknown slave` line at all, so no address rule sees it, old or new (measured, F-COLLIDE below).
- **(B) Not recommended: the flag clear as a delimiter.** `Unhandled write to offset 0x1C … NACKCF` is Renode warning about a bit it does not model. It would vanish when Renode models NACKCF. It delimits transfers, not steps. And the firmware under test writes it, so a firmware that skips it merges two steps.
- **(C) Forbidden by §1.3, listed for the architect: `evolve`-only, "an absent address no declared device owns".** It separates the measured runs. It hides any wrong address that equals another declared device's address while that device is also absent, for example a device-absent or bus-wide fault plus a firmware fault.
- **(D) `evolve`-only, "when the witness's own address is also absent, no other absent address counts".** It also separates the measured runs, but it is not "this step's transfers". It hides a firmware that addresses wrongly in any step of a device that also fails to answer somewhere on the run, such as a configure step to a wrong address after an identity step that met a faulted bus.

## 4. Adversarial pass (§5)

**The setup.**
- A fresh subagent on Opus got the three measured runs (`out/records.txt`) and the code, not this conclusion.
- Its brief: find an `evolve`-only rule from the archived records that does not count run 2 (held-low-sda), still catches run 3 (wrong-address), reads no declared list, and is truly "this step's own transfers".
- I spot-checked its claims at source; the one that did not hold is marked.

**Verdict: no rule meets the last condition.** What it confirmed:
- A Renode campaign's trace holds only `Event::Record` (`campaign.rs:674-679`).
- Kinds 12 and 13 carry the boot's end time.
- Kind 12's `address` is the plan's, not what the firmware sent, so other steps' addresses are the declared list under another name.
- `absent` is cut silently at 16 entries (`campaign.rs:1252-1253`).

Its candidates, each tested against the brief, each with what it hides:

- **R4: align the `timeout_txis` steps, in kind-12 order, with `absent`'s entries.** It separates runs 2 and 3, and it does not misfire on U5. But it infers which step made which transfer from the behaviour of the firmware under test: table order, one START per failed step, and the status it reports. It hides:
  - two steps' addresses swapped together with their order;
  - a wrong address followed by a successful retry;
  - two faults that cancel in the count.

  With 18 failed steps and 16 entries kept, it fails the faulted bus too.
- **R5: the device's own address absent fewer times than it has failed steps.** It cannot say which address the step sent, or which step it was. It fails once the 16-entry cut drops a late device's own address.
- **R6: ignore `absent` on any bus with a `BusFault` (kind 9).** It hides every wrong address on a faulted bus (measured shape: `[117, 119]`, nothing flagged).
- **R0 (today) and R2 (the declared list)** behave as §2 and §3 (C) say.

**Its claim not adopted.** It said R0 misfires "with U5 left absent by override" on a healthy bus, citing `record.rs:509`. That line is SPI's kind 23. The only device-absent override is `spi_device_absent` (`chaos/src/campaign.rs:169`), so on I2C that case is not reachable from a scenario today (read at source).

**Its last point bears on (A).** Filling `absent` per step by the position of a line among the device's own lines would still be count-based on I2C (`renode.rs:1053-1068`), and on a faulted bus no device logs anything. That is why (A) uses the firmware's step-record writes as the boundary and not the device's lines. The boundary still trusts the firmware to write each step's status after its transfer (§3 (A), "What it still cannot tell").

## 5. Decisions for the architect

- **D1 · the record change for F-MASK.** Recommend (A) with a trace version bump: a twin work order (the cli judge plus `evolve`). After it, this work order's fix is one line and the tests are as §1.4 writes them, with `spike/3305/out/archives/` as the fixture's before.
- **D2 · until then.** After `4202`, Sentinel-03's SENSORS has one modelled device, so no committed board meets F-MASK. A future board with two modelled devices on one bus will have every I2C bus fault refused by `detection`. Recommend a §7.1 row naming that, as `2304b` D2 asks.
- **D3 · F-COLLIDE.** A witness that carries another step's transaction (extra lines) is not judged wrong, so a wrong address that collides with a real device is attributed I4 and not software. Recommend an `evolve` work order: judge a witness's whole `observed` against `expected`, not its first write and read. Its effect on every committed attribution is to be measured there.

## Gates

- No workspace source changed. `spike/3305/dump` has its own `[workspace]` table and is not a member.
- `cargo fmt --all -- --check`: clean (host, at this head).
- `cargo clippy` and `cargo test`: not re-run on the host. The workspace is byte-identical to `main` `35d23eae`; CI runs both on the PR, and its result is reported on the PR and the bus.
- Reduction pass (diff): −3 files (the scenario copies) · 1 duplicate removed (the scenario copies; the run scripts are the one source) · 0 dead paths · 0 protected kept · 0 findings below (no crate in the diff; the prose duplicate of §4's R4 cut to one line).

## What moved

Nothing committed. No trace or verdict was regenerated, because nothing that produces one changed.
