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

| Run | Device | Today (count) | R2, carrier B |
|---|---|---|---|
| ext-cfg37 (spike) | U5, BMP581 | configure holds the warning; data holds the wrong write and its read | configure holds `write 0x37 <- 0x40 0x81`; the warning is unplaced |
| bmp-0x47 (spike) | U5, BMP581 | identity holds the warning, configure U2's stray `read 0x50 -> 0x00`, data the other three | each step holds its own line; the warning and the stray read are unplaced |
| seeded wrong-register (committed) | U2, BME280 | identity holds `Write D1`, `[WARNING] … Unhandled read from offset 0xD1.`, `Read 0` | the warning is unplaced |
| seeded wrong-nbytes (committed) | U2, BME280 | identity holds `Write D0`, the same warning, `Read 60 0` | the warning is unplaced |
| t2-baro, t2-baro-pf (committed tier 2) | U6, ICP10111 | by count, as `twin_tier2.rs` pins | **every line unplaced**: the ICP form is neither reader's |

**P2: missed in two places.**
- The two committed seeded BME280 cases carry a model warning, which the count rule puts in identity
  and a match rule cannot place.
- The ICP10111 (`[DEBUG] … Received command: [0x78, 0x66]`) is a multi-step I2C device in a committed
  tier-2 fixture. I had not inventoried it.

**What each rule and carrier would do (measured, in memory):**

| Run | `main` | R1A | R1B | R2A | **R2B** |
|---|---|---|---|---|---|
| ext-cfg37 | I4 `[6:7,7:7]` | S1 `+42:13` | S1 `+42:13`, by silence | S1 `+42:13` | **S1 `[6:7,7:7,42:13]`, `i2c2.u5.Write` `0x364081` against `0x374081`** |
| bmp-0x47 | S1 `[4:7,5:7,30:13,32:13]` | S1, `31:13` in place of `32:13` | S1 `[4:7,5:7,30:13]` | as R1A | S1 `[4:7,5:7,30:13]` |
| seeded wrong-register | S1 `[5:7,6:7,33:13]`, `Write` `0xd1` | = | S1, but `Write` and `Read` **silent** (the wrong `Write D1` is unplaced) | = | S1, fields unchanged; kind 13 loses the warning |
| seeded wrong-nbytes | S1 `[5:7,6:7,33:13]` | = | S1; kind 13 loses the warning | = | the same |
| t2-baro | **N1** (boot passes) | **I4**, boot fails | **I4** | **I4** | **I4** |
| t2-baro-pf | I4 | I4 | I4 | I4 | I4 |

- **R1 regresses a committed case:** seeded wrong-register's generator report would lose the wrong
  `Write D1` it names today. **R1 also fails ext-cfg37's acceptance:** S1 comes by configure's silence,
  not by its wrong write.
- **Carrier A misattributes:** ext-cfg37's configure warning fails U5's *identity* witness at the boot
  (three `i2c_transaction_unwitnessed` where there are two today), and bmp-0x47's stray read becomes
  U5's identity evidence.
- **R2B meets ext-cfg37's acceptance.** The evidence is the wrong write, and kind 14's count is
  unchanged (2).
- **P3:** R2 predicted right, R1 right. **P4, bmp-0x47 under carrier B: wrong.** It stays S1: U2's
  silent witness alone is S1 today. The I4 before 3307 came from the BMP581 witnesses being unreadable,
  not from silence being weak (inferred from `place-main.jsonl`).

**(1), counts (measured): P5 holds.** No witness in the sweep shows a right value at a wrong count.

**(2), the BMP581 log whole (measured): P6 holds.**
- No committed BMP581 log holds a transaction beyond its script.
- In both spike cases the log holds a warning, so today's `transactions` would refuse it whole:
  - bmp-0x47: the extra `read 0x50 -> 0x00`, its 3 scripted transactions held;
  - ext-cfg37: the extra `write 0x37 …` and a data read whose values differ, 1 of 3 held.

**(4), stitches (measured): P7 missed.** One exists today: ext-cfg37's data witness takes its `Write`
from the wrong write's line and its `Read` from the data read. It is harmless today, because that
6-byte read is refused, and it disappears under any match rule.

**P8, the boot verdict (measured, in memory):** under R2B no committed run's kind 14 moves, but for
t2-baro. Under R1 or R2 with carrier A, ext-cfg37 gains one finding.

## The stop (§7)

**A committed verdict would move.** (3) as written moves `twin_tier2.rs`'s imu-baro fixture from a
passing boot (N1) to I4. The ICP10111's `[DEBUG] Received command` / `Reading data with CRC` lines are
neither `Write`/`Read` nor the BMP581 form, so placement by register reads none of them, and every
line is unplaced. `a_seeded_barometer_fault_is_named_by_its_step` names the data step by the count
rule today. Fixing this needs D4.

## Decisions for the architect (leans first)

- **D1, the match rule: R2 (lean).** A transaction goes to its device's step with the same pointer and
  direction. Failing that, it goes to the device's one step of its direction, if there is exactly one.
  Otherwise it is unplaced. A repeat of a step's transaction goes to that step, so it fails there.
  - R1 (literal) fails the acceptance and silences seeded wrong-register's named `Write`.
  - The key is the twin's `StepCheck` (`write`, `width`), from the topology, never the firmware's
    record (`INV-07`).
- **D2, where an unplaced line lives. Kind 13 has no field for it, and its record (chaos), `campaign.rs`
  and `verdict.rs` are outside the fence.**
  - **(a) Carrier A,** the step under way, as `[ERROR]`: rejected. It fails a right step (measured).
  - **(b) Carrier B,** the line leaves kind 13: in fence. But the trace no longer holds the device's
    whole log (`INV-04`), and (2) can never see bmp-0x47's stray read, since the line is gone from
    every witness.
  - **(c, lean) A field of its own:** kind 13 `unplaced` (or a new kind), with a trace bump. It needs
    chaos `record.rs`, `campaign.rs` (archive) and `verdict.rs` (report), all outside the fence.
    - The line stays in the trace, quoted, and no step holds it.
    - (2) then reads an unplaced *transaction* as a surplus (bmp-0x47's `read 0x50 -> 0x00`), and an
      unplaced *non-transaction* (a warning) is named as its own field.
    - `2314` also edits `renode.rs` and `campaign.rs`: the order of merges is the architect's.
- **D3, the boot must still fail on an unplaced line.**
  - Under the count rule, any extra line fails some witness. Under a match rule with (b) or (c), a
    stray warning beside right transactions would pass, so a boot verdict would move fail → pass.
  - **(i)** A new kind, `i2c_line_unplaced`, quoting the line. It moves the pinned findings of seeded
    wrong-register and wrong-nbytes (`cases.json` pins kinds), and chaos must register it (outside the
    fence).
  - **(ii, lean)** Count it as the device's `i2c_transaction_unwitnessed`, quoting the line in that
    finding's message. Kinds stay as pinned, and only the count grows. Chaos bounds that kind with
    `Reach` ("at least"), so replay accepts it (read at source, `chaos/src/campaign.rs` L165-185,
    L242). The emitting function is `witness_findings`, beside `attribute`: in fence only if it counts
    as a helper.
- **D4, a model whose form neither reader knows (the ICP10111; read at source, `twin/src/model.rs`
  `Icp101xx`).**
  - **(a, lean)** A third reader in evolve for the ICP form: the command is the key, and it is a read
    if a `Reading data with CRC` line follows.
  - **(b)** Keep the count rule for a device whose script is in neither form, named as such in code. F2
    then stays open for the ICP.
- **D5, how (1) names a count.** `Field` carries no count, so `0x00364081` against `0x364081` would
  print equal values marked wrong.
  - **Lean:** a field of its own, `{device}.Write.bytes` and `.Read.bytes` (solved count, observed
    count), emitted only when the counts differ. No committed generator report moves (measured: no
    count differs).
- **D6, (2) with a warning in the log.** Under D2(c) the warnings are no longer in `observed`, so
  `transactions` reads the placed lines plus the unplaced transactions. Under D2(b) a warning never
  reaches it. Either way 3306's "every line a transaction" holds over what it reads.
- **D7, bmp-0x47's evidence (a spike case).** It stays S1, but U5's configure witness (`32:13`) is no
  longer evidence: it holds its own right line. Under D2(c) and D6 the stray read is named as U5's
  `transactions` surplus. The masked pointer (`0xD0` logged as `0x50`) keeps it from matching U2's
  step shape (3307 F1). Named, not a committed verdict.

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
4. For each run, compare `main`'s CLI on `main`'s archive with the branch's CLI on the branch's:
   - the rule, evidence, symptoms, masked and unexplained;
   - kind 14's kinds and counts;
   - kind 13 line by line;
   - for an S rule, `propose`'s files and exit code.

   The control's floor is zero (measured above).
5. Expected under the leans:
   - ext-cfg37 I4 → S1;
   - bmp-0x47 S1, with U5's configure evidence replaced by its surplus field;
   - seeded wrong-register and wrong-nbytes: kind 13 loses the warning to `unplaced`, kind 14's count
     grows by one, and the rule, evidence, fields and pinned kinds are unchanged;
   - nothing else.
6. Then the CLAUDE.md gates, the full tier-2 suite (`twin_tier2.rs`'s imu-baro, own-board and recovery
   tests are the committed ones this touches), and the bench `--against-fab` in CI's layout.
