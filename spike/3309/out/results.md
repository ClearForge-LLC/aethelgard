# AET-WO-3309 §1.1: the measurement, the decisions it raises, and the sweep's plan

Measured 2026-11-24 on Furnace from `main` at `9c3aeee3`. Predictions are in `spike/3309/predictions.md`,
committed at `a152af69` before any run.

Each statement is labelled:
- **measured:** run on Furnace, in the pinned tools image under `capped-run` (8G, swap 0, nice 10), or
  on the host under a capped user scope;
- **read at source:** given with its file;
- **inferred.**

## Method

- **The runs (measured):** 79 campaign runs, each made once by `main`'s CLI in the image. The CLI was
  built from a `git archive` of `9c3aeee3`.
  - `sh/measure.sh` runs every committed scenario, grouped by the spec its `base` names (c0 to c6), and
    the seeded set's 13 mutants over their own cases.
  - It also runs 3306/3307's spike cases: ext, two and bme-bmp (baseline and held-low SDA), two-0x75,
    two-0x77, bmp-0x47 and ext-cfg37.
  - `sh/measure-t2.sh` runs the committed tier-2 campaigns outside `scenarios/` that carry a multi-step
    I2C device or a pinned attribution (`twin_tier2.rs`): the BOR recovery fixtures (logger-4600 and
    sentinel-02-shaped, each with a BMP581, and ext-bmp581), imu-baro-1v8 (an ICP10111 on I2C) with its
    baro-pressure-first mutant, and `two-bme-collide.json`.
- **The control (measured):** ext-cfg37 and two Sentinel-03 scenarios were run a second time. Their
  attribution, kind 13 and kind 14 are identical to the first runs, so a difference later is
  placement, not noise.
- **What would move (measured, in memory, no Renode):** `tool/` (`place3309`) replays each archive with
  `main`'s chaos.
  - It re-derives today's count rule from the concatenated kind 13 lines, which matched every device in
    every run.
  - It re-places each I2C device's whole log by each candidate rule, re-derives kind 14's
    `i2c_transaction_unwitnessed` count from the new witnesses, and runs `main`'s
    `aethelgard_evolve::attribute` and `view` on the result.
  - It also reads (1), (2) and (4) from the archive as it is.
- **Records:** `out/place-main.md` (the tables) and `out/place-main.jsonl` (every run, every variant,
  with the wrong fields).
- **bme-bmp is rebuilt** (`spec/bme-bmp.json`): today's `two-bme.json` with 3307's two edits. 3306's
  spec (schema 19) and 3307's (schema 27) are refused at schema 29 (measured).

## Results

**P1, the baseline (measured): holds.** All 68 runs 3307 measured give the same rule and evidence
kinds today, bmp-0x47 S1 and ext-cfg37 I4 included. The 11 new runs: the tier-2 groups are N1, but
t2-baro-pf is I4 and t2-collide S1 `[4:7,5:7,28:13,29:13]`, as `twin_tier2.rs` pins.

**Where the count rule decides a witness's lines (measured).** In every other run, every device's
placement is the same under every rule.

| Run | Device | Today (count) | R2 or R3, carrier B |
|---|---|---|---|
| ext-cfg37 (spike) | U5, BMP581 | configure holds the warning; data holds the wrong write and its read | configure holds `write 0x37 <- 0x40 0x81`; the warning is unplaced |
| bmp-0x47 (spike) | U5, BMP581 | identity holds the warning, configure U2's stray `read 0x50 -> 0x00`, data the other three | each step holds its own line; the warning and the stray read are unplaced |
| seeded wrong-register (committed) | U2, BME280 | identity holds `Write D1`, `[WARNING] … Unhandled read from offset 0xD1.`, `Read 0` | the warning is unplaced |
| seeded wrong-nbytes (committed) | U2, BME280 | identity holds `Write D0`, the same warning, `Read 60 0` | the warning is unplaced |
| t2-baro, t2-baro-pf (committed tier 2) | U6, ICP10111 | by count, as `twin_tier2.rs` pins | **every line unplaced**: the ICP form is neither reader's (with an ICP reader and R3, as today) |

**P2: missed in two places.**
- The two committed seeded BME280 cases carry a model warning, which the count rule puts in identity
  and a match rule cannot place.
- The ICP10111 (`[DEBUG] … Received command: [0x78, 0x66]`) is a multi-step I2C device in a committed
  tier-2 fixture. I had not inventoried it.

**What each rule and carrier would do (measured, in memory).** The variants:
- **Rules:**
  - **R1:** by key alone.
  - **R2:** by key, then by direction when the device has exactly one step of it.
  - **R3:** by key for every transaction first, then each remaining one, in log order, to the first step
    of its direction (in script order) that holds no transaction yet.
- **Carriers:** **A**, the step under way; **B**, out of kind 13.
- **D3 counts**, for an unplaced line: **-line**, one more `i2c_transaction_unwitnessed` per line;
  **-dev**, one per device; **-min**, one per device none of whose witnesses already fails.
- **The ICP10111 is read in its own form (D4(a)):** the command is the key, and a `Reading data with
  CRC` line makes it a read.

| Run | `main` | R1B | R2A | R2B | R2B-line / -dev | R2B-min | R3B | **R3B-min** |
|---|---|---|---|---|---|---|---|---|
| ext-cfg37 | I4 `[6:7,7:7]` | S1 `+42:13`, by silence | S1 `+42:13`, 3 unwitnessed | S1, by the wrong write | **I1** | S1 | S1 | **S1 `[6:7,7:7,42:13]`, `i2c2.u5.Write` `0x364081` against `0x374081`; kind 14 unchanged** |
| bmp-0x47 | S1 `[4:7,5:7,30:13,32:13]` | S1 `[4:7,5:7,30:13]` | S1, `31:13` for `32:13` | S1 `[4:7,5:7,30:13]` | **I1** | **I1** | S1 `[4:7,5:7,30:13]` | **I1** (see D2) |
| seeded wrong-register (committed) | S1 `[5:7,6:7,33:13]`, `Write` `0xd1` | S1, `Write` and `Read` **silent** | = | S1, fields unchanged | **I1** | S1 | S1 | **S1, fields and kind 14 unchanged; kind 13 loses the warning** |
| seeded wrong-nbytes (committed) | S1 `[5:7,6:7,33:13]` | S1 | = | S1 | **I1** | S1 | S1 | **the same** |
| t2-baro (committed) | N1 | = | = | = | = | = | = | **=** |
| t2-baro-pf (committed) | I4 | I4, data silent | I4, data silent | I4, data silent | **I1** | I4, data silent | = | **=** |

Every other run is `=` under every variant. Without the ICP reader, t2-baro is I4 under every variant:
the passing boot fails (the first table in `out/place-main.md`'s history; this stop).

- **R1 regresses a committed case:** seeded wrong-register's generator report would lose the `Write D1`
  it names today. **R1 also fails ext-cfg37's acceptance:** S1 comes by configure's silence.
- **R2 silences a wrong command on a device with two read steps:** baro-pf's data step. Its wrong
  command is a read, and the ICP has two read steps. **R3 keeps it** in the data step, as today.
- **Carrier A misattributes:** ext-cfg37's configure warning fails U5's *identity* witness at the boot.
- **Counting an unplaced line in kind 14 that no archived event shows is I1** (evolve names it
  unexplained). Counted per line or per device, that turns the committed seeded S1s into I1. Counted
  only when nothing else fails (-min), it leaves every committed run as it is, but bmp-0x47's U5 still
  becomes I1, since its stray read is in no record. That is the measured case for D2(c).
- **R3B-min is the lean:** every committed attribution, field and kind 14 unchanged, and ext-cfg37's
  acceptance met. bmp-0x47 needs D2(c).
- **P3:** R2 and R3 predicted right, R1 right. **P4, bmp-0x47 under carrier B: wrong.** It stays S1
  with no finding for the unplaced lines. U2's silent witness alone is S1 today. The I4 before 3307
  came from the BMP581 witnesses being unreadable (inferred).

**(1), counts (measured): P5 holds.** No witness in the sweep shows a right value at a wrong count.

**(2), the BMP581 log whole (measured): P6 holds.**
- No committed BMP581 log holds a transaction beyond its script.
- In both spike cases the log holds a warning, so today's `transactions` would refuse it whole:
  - bmp-0x47: the extra `read 0x50 -> 0x00`, its 3 scripted transactions held;
  - ext-cfg37: the extra `write 0x37 …` and a data read whose values differ, 1 of 3 held.

**(4), stitches (measured): P7 missed.** One exists today: ext-cfg37's data witness takes its `Write`
from the wrong write's line and its `Read` from the data read. It is harmless today, because that
6-byte read is refused, and it disappears under any match rule.

**P8, the boot verdict (measured, in memory):** under R3B-min no committed run's kind 14 moves. Under
carrier A, ext-cfg37 gains one finding.

**Not run: sentinel-02-own** (a multi-step BMP581, U7). It declares `mcu.foreign_clock`, so `campaign`
refuses generated firmware on it, as 3307 measured. Its `twin_tier2.rs` own-board boots run in the
build's tier-2 suite.

**Read at source: the boot's pinned tier-1 tests.**
- `kinds()` (`boot_tier1.rs` L456, `twin_tier2.rs` L243) lists every finding, not every kind, so one
  more `i2c_transaction_unwitnessed` breaks an exact pin.
- Under R3 with -min, `boot_tier1.rs`' `any_witness_but_the_model_s_exact_lines_fails` keeps its single
  finding:
  - "no read" (`Write D0` twice): both are unplaced, identity is silent and fails, and -min adds none;
  - "out of order": the `Read 60` with no pointer goes to identity by direction, `Write D0` is unplaced,
    identity fails, and -min adds none;
  - "a second read": both go to identity, which fails.
- `renode.rs`' `a_device_s_error_inside_a_step_fails_that_step_by_name` uses lines in no model's form
  (`[NOISY] i2c1.u7: id`), so it must be rewritten in a real form (D4).

## The stop (§7)

**A committed verdict would move.** (3) as written moves `twin_tier2.rs`'s imu-baro fixture from a
passing boot (N1) to I4. The ICP10111's `[DEBUG] Received command` / `Reading data with CRC` lines are
neither `Write`/`Read` nor the BMP581 form, so placement by register reads none of them, and every line
is unplaced (measured). With a reader for that form (D4(a)) nothing committed moves (measured).

## Decisions for the architect (leans first)

- **D1, the match rule: R3 (lean).**
  - A device's log is read as transactions in its script's form. A `Write` (or an ICP command) stays
    open across a line that is no transaction, so the `Read` after a model warning is still its
    transaction. Seeded wrong-register keeps its `0xd1` only because of this (measured).
  - Each transaction first goes to the device's step with the same key: the pointer (the ICP's
    two-byte command) and the direction. A repeat goes to the same step, so it fails there.
  - Then each remaining transaction, in log order, goes to the first step of its direction, in script
    order, that holds none. Otherwise it is unplaced.
  - R2 ("the one step of its direction") silences baro-pf's wrong command. R1 fails the acceptance and
    silences seeded wrong-register's `Write`.
  - The key is the twin's `StepCheck` (`write`, `width`), from the topology, never the firmware's
    record (`INV-07`).
- **D2, where an unplaced line lives.** Kind 13 has no field for it, and its record (chaos),
  `campaign.rs` and `verdict.rs` are outside the fence.
  - **(a) Carrier A,** the step under way, as `[ERROR]`: rejected. It fails a right step (measured).
  - **(b) Carrier B,** the line leaves kind 13: in fence. But the trace no longer holds the device's
    whole log (`INV-04`). A finding for it that no record shows is I1 (measured: bmp-0x47 under -min),
    and (2) never sees bmp-0x47's stray read.
  - **(c, lean) A record of its own:** a new kind per device holding its unplaced lines, archived after
    its kind 13s, with a trace bump.
    - It needs chaos (`record.rs`, and `campaign.rs`'s `shown`/`witness_steps` so that it shows the
      count), the CLI's `campaign.rs` (archive) and `verdict.rs` (report): all outside the fence.
    - It is the evidence event for (2): an unplaced *transaction* is the device's surplus, as
      `{device}.transactions`. An unplaced *non-transaction* (a warning) is named as a field of its own,
      quoted in the report.
    - `2314` also edits `renode.rs` and the CLI's `campaign.rs`: the order of merges is the architect's.
- **D3, the boot still fails on an unplaced line.**
  - Under the count rule, any extra line fails some witness. Under a match rule, a stray warning beside
    right transactions would pass, so a boot verdict would move fail → pass.
  - **(iii, lean) -min:** one more `i2c_transaction_unwitnessed` for a device with unplaced lines and no
    failing witness, quoting the lines in its message.
    - No committed kind 14 or exact pin moves (measured and read at source above).
    - Chaos bounds that kind with `Reach` ("at least"), so replay accepts it (read at source,
      `chaos/src/campaign.rs` L165-185, L242). With D2(c) the record shows it, so it is not I1.
    - The emitting function is `witness_findings`, beside `attribute`: in fence only if it counts as a
      helper.
  - **(i)** A new kind `i2c_line_unplaced`: it moves seeded wrong-register's and wrong-nbytes'
    `cases.json` pins, and chaos must register it.
  - **(ii)** Counted per line or per device: it turns the committed seeded S1s into I1 (measured).
- **D4, the forms.**
  - The I2C models are the BME280, the ICP-101xx and the BMP581 (read at source, `twin/src/model.rs`
    `bus()`).
  - **(a, lean)** A reader in evolve for each, the ICP's added. A test pins that every I2C model's twin
    witness lines parse in its reader.
  - A device whose script is in no reader's form is a named refusal of placement, by `INV-06`: its
    lines are all unplaced and its steps fail by name. There is no count rule fallback, which would
    keep F2 open silently.
  - The D-A1 unit test's synthetic lines are that case and are rewritten in a real form.
- **D5, how (1) names a count.** `Field` carries no count, so `0x00364081` against `0x364081` would
  print equal values marked wrong.
  - **Lean:** a field of its own, `{device}.Write.bytes` and `.Read.bytes` (solved count, observed
    count), emitted only when the counts differ. No committed generator report moves (measured: no
    count differs).
- **D6, (2) over what is placed.** `transactions` reads a device's placed lines, plus its unplaced
  transactions under D2(c), so a warning never reaches it. 3306's "every line a transaction" holds
  over what it reads.
- **D7, bmp-0x47's evidence (a spike case).**
  - Under D2(c) with R3-min it stays S1 (inferred). U5's configure witness (`32:13`) is no longer
    evidence, since it holds its own right line. The stray read becomes U5's `transactions` surplus on
    the new record.
  - The masked pointer (`0xD0` logged as `0x50`) keeps it from matching U2's step shape (3307 F1).
  - Named, not a committed verdict.

## The sweep's plan (for the build)

1. Build the branch CLI in the image from a `git archive` of its head.
2. Run `sh/measure.sh` and `sh/measure-t2.sh` with it into `runs-branch`. Placement runs at judge
   time, so the archives are the branch's own.
3. Add planted cases:
   - acceptance:
     - ext-zero, configure `&[0x00, 0x36, 0x40, 0x81]`;
     - ext-surplus, configure sent twice;
     - ext-cfg37;
   - §5's:
     - a byte appended, `&[0x36, 0x40, 0x81, 0x00]`;
     - the identity read at width 2;
     - a pointer widened;
   - two steps on one register, a warning between a write and its read, and a firmware-forged line, as
     tier-1 tests, since no fixture has them.
4. First, the branch's `attribute` on `main`'s archives. That separates evolve's (1), (2) and (4) from
   (3)'s placement, so each change has one named cause.
5. Then, for each run, compare `main`'s CLI on `main`'s archive with the branch's CLI on the branch's:
   - the rule, evidence, symptoms, masked and unexplained;
   - kind 14's kinds and counts;
   - kind 13 line by line;
   - for an S rule, `propose`'s files and exit code.

   The control's floor is zero (measured above).
6. Expected under the leans (inferred from the in-memory measurement):
   - ext-cfg37 I4 → S1;
   - bmp-0x47 S1, with U5's configure evidence replaced by its surplus field;
   - seeded wrong-register and wrong-nbytes: kind 13 loses the warning to the new record; kind 14 is
     unchanged (-min, since identity already fails), and so are the rule, evidence and fields;
   - imu-baro and baro-pf unchanged (with D4(a));
   - nothing else.
7. Then the CLAUDE.md gates, the full tier-2 suite (`twin_tier2.rs`'s imu-baro, own-board and recovery
   tests are the committed ones this touches), and the bench `--against-fab` in CI's layout.
