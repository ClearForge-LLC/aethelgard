# AET-WO-4241 — FEEDBACK (built to the architect's ruling of 10:05Z; STOPPED on D3's instants, and on droop flips that 2305 absorbs)

**Branch and inputs.**
- Branch `wo/aet-4241-read-every-identity`, from `main` at `d70a99e5`.
- The work order is read from `~/.cache/aet-4241-wo.md`, the kickoff from `~/.cache/aet-4241-from-architect.md`, and
  the ruling from `~/.cache/aet-bus/4241.md` (10:05Z).

Each statement is labelled:
- **measured:** run on Furnace, in the pinned tools image (`container/IMAGE`) under `capped-run`, nice 10, or by the
  host gates under a capped scope;
- **read at source:** file:line;
- **inferred.**

## 0. Status

- **History.**
  - **First stop:** at §1.1 (record `dc85747f`, §1 to §2 below).
  - **The 10:05Z ruling:** D1 accepted, D2 (a), and D3, under which unmodelled identity reads come after every
    modelled step.
  - **The build:** done to that ruling (§3), with every CLAUDE.md gate green on the host (§Gates).
- **Stopped again, for two reasons (§5):**
  1. **D3 says no modelled instant may move, but they all do, by a few instructions** (measured).
     - The shift is −2 to +25 instructions, before the first modelled step.
     - **The reset's `.bss` loop:** the record array grows by 2 words per unmodelled read, which costs 8
       instructions (read at source).
     - **`main`'s code generation:** the rest, inferred. `main`'s source up to the first unmodelled read is byte
       for byte the no-TMP117 board's, by a tier-1 test.
     - I cannot meet D3's letter without changing what every board's firmware compiles to, so I ask (D4).
  2. **Three droop verdicts flip on today's `main`.** `min/droop-bor3`, `s01/droop-bor3` and
     `dshot/motors-dshot600-droop` go from `pass` to `fail` with `brownout_unrecovered` (measured). The 3.6 ms wait
     puts the boot word past `main`'s fixed recovery bound.
     - **With 2305 trial-merged (clean), all three pass** (measured, `out/rejudge/trial-2305*`). That is the
       ruling's prediction.
     - Per D3 the flip is a stop until 2305 merges, and then I re-run it.
- **The second stop, ruled at 11:10Z:**
  - **D4, (a):** the bounded pre-step shift is accepted. The bound is stated here: each modelled status write moves
    at most 32 instructions (0.32 µs) for the unmodelled reads. It is held by a tier-2 test,
    `twin_tier2::the_unmodelled_read_moves_each_modelled_status_write_by_at_most_32_instructions`.
    - The test runs Renode directly, on the reference firmware with U3's read and against itself with that read and
      its record removed. It hooks U2's and U4's status words, and requires each written once and within 32
      instructions, and the boot word later by U3's wait.
    - It has a weight row in `ci/test-weights.tsv` (45 s, an estimate until the refit rule measures it), and
      `TIER2_MIN` rises 195 → 196.
  - **D5, confirmed:** after 2305 lands, merge `main`, re-run the re-judge, and confirm the three droops pass.
- **CI on `f48cadd7`, every failure root-caused (the architect's 11:12Z note):**
  - **Shards 3 and 4**, `against_fab_tier2`, and **shard 0**, `board_tier2::every_admitted_identifier_character…`:
    each `verify` runs a droop that is `brownout_unrecovered`. That is D5, fixed by 2305.
  - **Shard 10, `the_dshot600_fixture_s_campaign_passes_inside_its_window`:** the campaign runs `dshot600.json`,
    which passes with findings `[]`, and `dshot600-droop.json`, the third D5 flip. That flip is the exit 1. No
    window or instant is involved: in the re-judge every dshot line but the droop is unchanged. The test is
    unchanged, and passes once 2305 is in (the trial merge passed this droop).
  - **Shard 10, `a_locked_access_late_in_a_flood_reaches_the_judge_whole`:** D3's stated shift.
    - The mutant's locked read sits at a flood pair pinned 900 pairs before the 10 ms run's end. The flood starts
      after the boot word, which is 360,099 instructions later, measured on the reference (2662 → 362761). So pair
      183,081 fell past the run's end, leaving no locked access in the boot.
    - **Re-pinned from the stated shift, not widened:** 360,099 instructions at 5 instructions a pair (the test's
      own 50 ns a pair) is 72,020 pairs, so the read moves to 111,061, 900 pairs before the end as before.
    - The judged access is the same `GPIOC.MODER` read, so no verdict or judged access changes. §Gates gives the
      result in the image.
  - **Shard 7, `a2_3_a_9600_console_s_wire_time_is_judged_from_its_boot_word`:** D3's stated shift.
    - The test calibrates boot-word lateness by spins before the boot word, and the boot word moved 3,600,980 ns,
      measured on serial-dshot-9600 (31,660 → 3,632,640 ns, `main` against the branch).
    - At 30 ns a spin, 120,033 spins come off each: 2,650,000 → 2,529,967, 2,900,000 → 2,779,967 and
      3,150,000 → 3,029,967.
    - Verified in the image: 79 and 87 ms pass, and 94 ms fails with `serial_wire_past_run` and
      `serial_echo_missing`, as before.
  - **Negative tests that kept failing but lost their cause** (shard 5 `f2_a_late_dshot_anchor_fails_by_name`,
    shard 11 `m2_a_dma_start_of_cen_fails_closed_by_name`).
    - **Neither mutant passed.** Each still fails (exit 1), but by `boot_word_missing` and TIM1 `register_mismatch`,
      not by its named mechanism (CI run 44722692266).
    - **Mechanism:** each plants its event by spins from reset before the boot word, and the TMP117's 3,600,810 ns
      (measured on sentinel-02-shaped) comes before it. So the planted event lands past the run's fixed end
      (`MAX_VIRTUAL_NS`). The fault never happens in the boot; the judge missed nothing that occurred.
    - **Proposed fix, verified locally, pushed only on the architect's word (11:27Z):** re-pin by 120,027 spins
      (m2 3,230,000 → 3,109,973; f2 2,000,000 → 1,879,973). Both tests now assert their cause: the boot word written,
      so the planted event is inside the run, and for f2 the anchor below 100 ms.
    - **Verified result:** m2 fails by `motor_anchor_unobserved` ×4, and f2 by `motor_window_past_run`, its anchor
      between 98 and 100 ms.
  - **`bench-package`:** the committed package is `main`'s ELF. It moves by the architect's `write=true` dispatch
    only.
- **`main` merged:** `d0c4cfe0`, which is 2306, cleanly.
  - **The architect's 10:55Z ask:** an `identity_not_witnessable` read never counts toward a gate's `judged`. It is
    tested against 2306's fields:
    - `verify`'s `a_not_witnessable_read_never_counts_toward_judged`: the boot gate is `judged: 1` with zero, one or
      only unmodelled reads.
    - `boot_tier1`: U3 is in `boot.not_witnessable` and in none of the witness arrays the campaign archives as
      witnesses, which are what 2306's droop count reads.
    - `campaign_tier1`: the reference's droop is `observation_witness: {"judged": 1}`, U2's release, with U3 on the
      board.
  - **2306's `NO_WITNESSED_DEVICE`** now reads "the boot reads no modelled device's identity (an unmodelled one's
    is not witnessable)…". It stays true on the motor driver, which has no device, and is now true on an
    all-unmodelled board (F5). `board_tier2`'s expected string moved with it.
- **Nothing else moved** (measured):
  - **Campaign runs:** 47 committed and fixture runs. 44 are unchanged in verdict, `matched`, findings, replay and
    attribute, apart from the trace bytes.
  - **Boots:** in 11 committed boots every verdict and every finding kind but the new info row is unchanged. Each
    boot word is later by the measured total.

## 0a. Re-pins of planted instants (approved 12:06Z), each from a measured shift

Each test plants its event at a count of loop iterations from reset. The TMP117's TXIS wait, made before that
point, moves it by the measured shift. Each count is reduced by the shift divided by the loop's cost.

| test | old | new | measured shift (board) | loop cost |
|---|---|---|---|---|
| `a_locked_access_late_in_a_flood_reaches_the_judge_whole` | pair 183,081 | pair 111,061 | 360,099 instructions, boot word 2662 → 362761 (sentinel-01-minimal) | 5 instructions a pair |
| `a2_3_…` 79 ms | spin 2,650,000 | 2,529,967 | 3,600,980 ns, boot word 31,660 → 3,632,640 ns (serial-dshot-9600) | 30 ns a spin |
| `a2_3_…` 87 ms | spin 2,900,000 | 2,779,967 | as above | as above |
| `a2_3_…` 94 ms | spin 3,150,000 | 3,029,967 | as above | as above |
| `f2_a_late_dshot_anchor_fails_by_name` | settle 2,000,000 | 1,879,973 | 3,600,810 ns, boot-word shift (sentinel-02-shaped) | 30 ns a spin |
| `m2_a_dma_start_of_cen_fails_closed_by_name` | spin 3,230,000 | 3,109,973 | as above | as above |

**Each keeps its cause, asserted:**
- **flood, f2 and m2:** the boot word is written, so the planted event ran inside the run.
- **f2:** its anchor is between 98 and 100 ms.
- **a2_3:** each boot word lands at its intended millisecond.

**Verified in the image:** each fails, or passes, by its own named mechanism.

**Proposed §7.1 row (the architect's, 12:06Z; not built here):** an event planted at a fixed count of spins from
reset is brittle to any change in boot length. Planting it relative to the boot word would remove these re-pins.

## 1. The measurement table (§1.1), with the predictions beside it

**Static (`spike/4241/sh/survey.py`, `out/survey.md`; run before the predictions were written).**
- **The specifications:** 42 committed specifications hold 201 devices, and 192 of those devices have a part row
  with an identity.
- **The unmodelled ones:** 78 of the 192 are unmodelled, all **TMP117** (`renode: null`), in 28 specifications.
- **SPI:** TMP117 is the only part in the table with no model, so **no committed specification has an unmodelled
  SPI device**. The SPI rows below use a spike-only lever, `AET4241_UNMODEL` in `model::kind`, in
  `sh/instrument.diff` only.

**Dynamic (measured; `out/runs.md`, raw boot logs in `out/raw/`).**
- "Old" is `main`'s generator, and "new" is the generator with every identity read kept.
- Both were judged by `main`'s judge, instrumented to print the boot's log and the 48 RAM words from the boot word.

| case | prediction | measured |
|---|---|---|
| TMP117 read on I2C (C1–C5, C8) | status `timeout_txis`, value 0; one `Unknown slave at address 72.` | **as predicted**: `0x22020001`, `0x00000000`; one `[WARNING] i2c1: Unknown slave at address 72.` per read (C8: one per address, 72–75, on each of four controllers) |
| …against a modelled device that failed | byte for byte 3305's `wrong-address` U2 | **as predicted**: 3305's U2 sent to 0x75 left `timeout_txis`, 0 and `Unknown slave at address 117.` (`spike/3305/out/records.txt`) |
| SPI read to an unmodelled chip select (C6, C7) | status `ok`, value 0; multiplexer `Tried to transmit` lines | status **as predicted**, `0x22020000`; value **missed**: `0x00ffffff`, the multiplexer's dummy byte `0xFF` three times; lines **as predicted**: `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte`, one per byte |
| …against a modelled SPI device that failed | a mismatch | the same lines a modelled device absent behind its chip select leaves (`4200` D4's `unrouted` evidence); the value is the part's `0xFFFFFF`, a mismatch |
| one `TXIS` wait | 100k–300k instructions (1–3 ms) | **missed: 3.6 ms on the Sentinel-01 family** (C1's boot word 2,662 → 362,757 instructions; C2–C4 alike), **4.8 ms on Sentinel-03 and the `d3` fixture** (C5's TMP117 status written at 486,699; C10, about 480,100 a wait). The 20,000 spins cost what that board's code makes of them |
| 16 waits (C8, C10) | C8: the boot word at 16–48 ms, inside 100 ms; C10 (batch 3): about 58 ms | inside, **both times missed**: C8 at 57.6 ms (5,764,965 instructions), C10 at 76.9 ms (7,699,763). By extrapolation, 21 to 28 unmodelled I2C reads, by board, would not fit in `MAX_VIRTUAL_NS` (inferred) |
| old against new firmware, with a TMP117 | differ | **as predicted**: C1–C5 and C8, each `main.rs` and ELF |
| the `4600` pair and the control (C9, batch 2) | old: with U3 equals without U3; new: they differ; C9 old = new; C9 `pass` with no `device_unmodelled` | **as predicted, all four**: old `2bb1be79…` both (`4600` F19 reproduced); new with U3 `406af077…`, without U3 `2bb1be79…`; C9 `pass`, findings `[]` |
| today's judge, C1/C2 | U4 reads TMP117's slot: `spi_record_invalid`; U2 `absent` `[72]`; `fail` | **as predicted**: `spi_record_invalid` (status `0x22020001` on SPI); U2's `absent` `["0x48"]`; `fail` |
| today's judge, C3 | as C1; U2, U5 `absent` `[72]` | **as predicted** |
| today's judge, C4 | U5's records shift; U5 and U4 fail | **as predicted**: `i2c_timeout`, `i2c_configure_mismatch` and `i2c_data_mismatch` on U5, `spi_identity_mismatch` on U4 |
| today's judge, C5 | U7's records shift and fail; U7 identity `absent` `[72]` | **as predicted**, and every later record too: U4, U5 (`spi_identity_mismatch`, `spi_configure_mismatch`, `spi_record_invalid`) |
| C6 (SPI unmodelled), today's judge | `pass`, `device_unmodelled` ×2 | **as predicted** |
| C7 (every device unmodelled) | `pass` on the boot witnesses alone, `device_unmodelled` ×3 | **as predicted**: `pass`, no record judged |
| C8 (16 TMP117) | `fail`: shifted BME280 records | **as predicted**: `i2c_timeout` on S6, S7, S12, S13, `spi_record_invalid` on S1000; every BME280's `absent` holds 72–75 of its controller |
| C10 (12 unmodelled W25Q64, lever) | `ok`, no SPI wait; `fail` from shifted BME280 records | **as predicted**, but for the time above: 48 `Tried to transmit` lines (12 reads × 4 bytes), no SPI timeout; `fail` (`i2c_timeout` ×4) |


## 2. The first stop, and its ruling (10:05Z)

**The stop (§1.1).** An unmodelled read is, byte for byte, what a modelled device that failed leaves:
- **I2C:** `timeout_txis`, value 0, and `Unknown slave at address 72.`;
- **SPI:** `ok`, the multiplexer's dummy bytes `0xFFFFFF`, and `Tried to transmit … not connected`.

**The ruling:**
- **D1, accepted:** the device is unmodelled by the plan (`model::kind` None). The twin places its record. The step
  hook and the SPI window attribute its evidence. `identity_not_witnessable` (info) replaces `device_unmodelled`
  for generated firmware. `verify` lists it as a `twin_limits` row.
- **D2, (a):** a record that was never written is `record_invalid` (error).
- **D3:** the unmodelled reads come last. No modelled record's instant may move, and that is to be tested. The
  verdict states the cost as a twin limit. Any verdict that flips is still a stop. 2305 should absorb the
  recovery bound.

## 3. The build

- **The generator** (`svd-hal/src/board/firmware.rs` `reads`, the selection only):
  - every modelled device's steps, exactly as before;
  - then, before the boot word, every unmodelled device's identity step, in the specification's device order
    (I2C, then SPI);
  - `step_read` keeps configuration and data steps for modelled parts only.
  - `generate()` (with `answers` false) now reads every identity. `generate_with`'s doc says so, a one-paragraph
    edit in `board.rs` just outside the fence (flagged).
- **The twin** (`boot.rs`):
  - `Unmodelled.identity: Option<NotWitnessable>`, holding the table's identity, its record placed after every
    check's, and the path a model would take.
  - `Wire::status`, which reads a status word per bus. `StepCheck::judge` now uses it, with every mapping as before.
  - `checks` stays modelled-only, so nothing that reads `checks` sees an unmodelled read: the witness, stimulus,
    probe, kind-12/13 trace records and evolve's counts.
- **The judge** (`cli/src/renode.rs`):
  - **Re-arm and read:** the unmodelled records are re-armed and read.
  - **Step hooks:** each unmodelled read gets one, with index `checks.len() + j`, which never names a check.
    - `absent_by_step` takes the unmodelled I2C steps as steps on their controller, so their `Unknown slave` line is
      their own.
    - `attribute` registers an unmodelled SPI device's chip-select window.
  - **The cost:** one more hook, `cost_anchor`, on the last check's status word (on the brown-out word with no
    check). `unmodelled_costs` measures each read from the write before it, and a word not written exactly once
    gives no cost.
  - **The finding:** `unmodelled_finding` names each read `identity_not_witnessable` (info), giving the record,
    the absent addresses, the cost and the table value.
    - A never-written record after the fixed point is `<bus>_record_invalid` (D2).
    - Before the fixed point the finding is still info.
    - A part with no row keeps `device_unmodelled`.
  - **The foreign judge** is unchanged. It never calls `rearm` or `boot_hooks`.
- **The verdicts:**
  - **`simulate`:** `boot.not_witnessable` carries `count`, each read (on, register, width, table value, status,
    observed, `cost_ns`), `total_ns` and the twin-limit sentence. It is absent on a board with none, so such a
    board reports as before.
  - **`verify`:** `twin_limits` gains `IDENTITY.<ref>`, `silicon-only`, never acknowledged
    (`ledger.rs` `identity_rows`).
- **Goldens moved by tooling, each explained:**
  - **`boards/sentinel-01*`** (`AETHELGARD_BLESS=1`): `main.rs` gains the TMP117's read as record 2, before the boot
    word. Nothing else moves.
  - **`order_2203`:** `main.rs` and the boot image move; the platforms do not.
  - **`serial_ports`:** only `main.rs` moves.
  - **`spi_devices`:** U3 becomes record 13 after the data read, and the array grows 13 → 15 words.
  - **The tier-1 stub transcripts** (`boot_tier1`, `campaign_tier1`): two re-arm writes and U3's record (`TXIS`, 0).
  - **The pinned monitor scripts** (`renode.rs` tests): U3's hook and the cost anchor.
  - **`ledger_tier1`:** the `IDENTITY.U3` row.
  - **The tier-2 assertions** (`twin_tier2`, `campaign_tier2`): `identity_not_witnessable` and `not_witnessable`.

## 4. Re-judge (§1.4, measured; `out/rejudge/`)

- **What ran:** `main`'s CLI against `main`'s firmware, and the branch's CLI against the branch's firmware.
  - Every committed scenario of every group ran through `campaign`, `replay` and `attribute`.
  - Each group's specification was booted once by `simulate`, with an instrumented copy of each side (`RAWLOG`) for
    the hook instants.
- **Campaigns:** 47 runs.
  - 44 are `=`: the same verdict, `matched`, findings, replay and attribute, apart from the trace bytes.
  - 3 flip: `min/droop-bor3`, `s01/droop-bor3` and `dshot/motors-dshot600-droop`, each `pass`/matched →
    `fail`/unmatched, `brownout_unrecovered`, attributed I1.
  - With 2305 trial-merged, all 3 pass.
- **Boots:** 11 specifications booted.
  - **Verdicts:** all `pass` → `pass`, with other finding kinds equal.
  - **The new row:** one `identity_not_witnessable` per TMP117.
  - **The cost:** `total_ns` is 3,600,410–3,601,040 ns on the Sentinel-01 family and 4,801,180 on Sentinel-03. The
    boot word moves by the same, within 600 ns.
  - **Not runnable on either side, both identically:** the `own` and `imu` groups (generate or simulate exit 3).
- **Modelled instants (STEP counts, instructions):**

| group | moved |
|---|---|
| min | 2102→2127 |
| s01 | 2345→2370 |
| serial | 4240→4265 |
| dshot, pwm | 1555→1575 |
| pow | 1605→1627 |
| two | 1219→1229, 1382→1402 |
| ext | 2353→2371, 2538→2558, 2727→2747, 1952448→1952465 |
| s03 | 6845→6849, 7093→7101, 3910887→3910899 |
| shaped | 3905934→3905932 |
| spionly | (no I2C check hooked) |

## 5. The second stop: rulings asked

- **D4. D3's "no modelled record's instant may move. Test that."** It holds for every record's **place**, which is
  tested at tier 1 in the twin and the generator, and in every pinned layout. It does not hold for the **instants**,
  which move by −2 to +25 instructions (0.25 µs at most), measured.
  - **The `.bss` share (read at source):** 8 of those instructions. The record array grows by 2 words per
    unmodelled read, and the reset zeroes `.bss` at 4 instructions per word (`boards/sentinel-01/firmware/src/main.rs`
    L92–98).
  - **The rest is `main`'s code generation (inferred).** `main`'s source up to the first unmodelled read is byte for
    byte the board's without it (`the_unmodelled_read_moves_no_modelled_step`).
  - **No verdict moves by it.**
  - **Ruled (a) at 11:10Z** (§0). The options were:
    - **(a), recommended:** accept a bounded pre-step shift. I add a tier-2 test of the measured bound, ≤ 32
      instructions on the reference with and without U3, and keep the place test.
    - **(b):** put the unmodelled records in a separate no-load static, after the boot record. That removes the
      `.bss` share but not the code-generation share, and splits the judge's one record array in two.
    - **(c):** the source-identity test as the test of D3, as built.
- **D5. The droops.** They flip on today's `main` and pass with 2305 trial-merged. I merge `main` after 2305 lands
  and re-run all three before READY. Confirm that is the ruled path.
- **Package:** the bench package moves with the ELF, so it is third in the queue after 2306 and 2305. It is ready to
  dispatch once D4 and D5 are settled.
- **Mutants of the reduction:** re-run on `434c861e`, all killed.

## 6. Mutants (one per guard this build adds; each on a fresh tree at `3f886653`; `out/mutants.txt`)

All 12 are killed:

| mutant | what it does | killed by |
|---|---|---|
| M1 | unmodelled finding dropped (a silent pass) | `boot_tier1` |
| M1b | the kind renamed, so not reported as the named outcome | `boot_tier1` |
| M2 | the `answers` filter kept for identity steps (the WO's mutant) | `svd-hal board` (layout parity, golden) |
| M3 | unmodelled reads interleaved | `svd-hal board` |
| M4 | twin slots placed first | twin unit |
| M5 | D2 removed | cli unit |
| M6 | unmodelled steps left out of `absent_by_step` | cli unit |
| M7 | the SPI window ignored | cli unit |
| M8 | the unmodelled hook dropped | pinned scripts |
| M9 | the re-arm dropped | `boot_tier1` stub |
| M10 | the cost's once-check removed | cli unit |
| M11 | the twin-limit row dropped | `ledger_tier1` |
| M12 | a status counted off its bus | cli unit |

## 7. Adversarial pass (full; a fresh Opus subagent, given the code, the WO and the ruling, not my conclusions)

- **F1 (stop-class): the droops flip.** It found them in my own re-judge, which confirms them. Cause: `main`'s fixed
  `RECOVERY_BOUND_NS` against a boot word 3.6 ms later. §5 D5.
- **F2 (D3's letter): instants move.** Confirmed by the re-judge. §5 D4. The wording "no modelled record moves" in
  the twin-limit sentence and the ledger note now says "place".
- **F3 (backlog):** a failure caused by the unmodelled cost never names the twin limit.
  - With enough unmodelled I2C reads, `boot_word_missing` follows: 21 to 28 by board, inferred from 3.6–4.8 ms each
    against 100 ms. So does a motor or serial window past the run.
  - No committed board comes near: at most 16 TMP117, the `d1`/`d3` fixtures, which boot at 57.6 and 76.9 ms.
  - Proposed as a §7.1 row: refuse before the boot (count × measured cost against the window, as
    `SerialWireTimePastWindow` does), or name it in the failure.
- **F4 (fixed in wording):** with no modelled check, the cost anchor is the brown-out word, so the first read's cost
  includes the bring-up between. The twin-limit sentence now says so.
- **F5 (2306):** 2306's `NO_WITNESSED_DEVICE` text, "the boot reads no device's identity", becomes inaccurate on a
  board whose devices are all unmodelled. Its count is right: 4241 adds no trace witness. It is for whichever of us
  merges second.
- **F6 (nits, kept):**
  - The I2C loop checks the bus before the row, the SPI loop the row first. `validate` refuses a part with no row,
    so this cannot be reached.
  - The ledger's SPI note names the bus without the chip select.
- **The WO's four lenses:**
  - **Misclassification:** the generator's `answers`, the twin and the ledger all go through `model::kind`, and an
    error refuses. Found nothing.
  - **F-COLLIDE:** a same-bus address collision is refused at `validate` (`DuplicateI2cAddress`). Found nothing new.
  - **An all-unmodelled board (C7):**
    - `simulate` passes on the boot witnesses alone, with info rows, and 2306 counts it as `judged: 1`.
    - Its identity coverage of 0 shows as empty `i2c`/`spi` and `not_witnessable.count`. I offer it as a §7.1 row
      if the architect wants it named.
  - **SPI timing:** unmodelled reads come last, so nothing modelled follows one, and its window is its own (M7).

## 8. `BENCH.md`: the TMP117's expected record (for the architect to place)

- **The read:** Sentinel-01 as ordered (`examples/sentinel-01.json`, `boards/sentinel-01-bench`). U3 (TMP117) on
  I2C1 at `0x48`, ADD0 on GND (SNOSD82D Table 7-2). It is record 2, after U2's and U4's: status at boot word
  + `0x18`, value at + `0x1C`.
- **The register:** it writes `0x0F`, the `Device_ID` register, and reads 2 bytes, most significant first.
- **Expected status:** `0x22020000` (ok).
- **Expected value:** `0x00000117`, from SNOSD82D §7.6.11 (`Device_ID`, reset `0x0117`; `DID[11:0]` = `0x117`;
  `Rev[3:0]` reads 0 at reset). The table cites document sha256 `78212e91…b789`. The mask is the full 16 bits.
- **The twin's prediction:** none, since it has no model. The twin's record is `timeout_txis`, 0, which is the
  twin's limit, not a prediction of the board.
- **Supersedes:** it replaces B9's address probe as the TMP117's witness (`3403` F6, U1), if the architect agrees.

## 9. Owed and fence

- **Owed after 2305 lands:**
  - merge `main`;
  - re-run the three droops and the re-judge;
  - adopt 2305's hook additions: the STEP indices are checked against it, and its trial merge was clean;
  - the package dispatch, third in the queue.
- **Done after 2306 landed:** `main` merged, and F5's sentence settled (§0).
- **Fence:**
  - Everything is inside it but `generate_with`'s doc comment in `svd-hal/src/board.rs` and one unit test there
    (`spi_init_takes_every_select_and_drives_it_high`, whose firmware now reads both devices).
  - No IR, specification, `BENCH.md`, workflow or `ci/` file changed.
  - `TIER2_MIN` rises by this order's one tier-2 test, the D4 bound: 195 → 196 in `.github/workflows/ci.yml`, as
  the WO's acceptance asks. It also has one weight row in `ci/test-weights.tsv`, as P1/P2 require.

## Gates (host, capped scope, at the pushed head)

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: green (`t1-3`, rc 0).
- Tier 2 is CI's. The three droop tests in `campaign_tier2` are expected red until 2305 (§5).
- Reduction pass (diff): −8 net · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 6
  findings below.
  - **Removed (re-tested; M1, M5, M10 and M12 still die at `434c861e`):**
    - the status→name mapping, now one `status_name(wire, status)` for checks and unmodelled reads alike;
    - the ns conversion, now `Unwitnessed::cost_ns`.
  - **Findings:** each is a mechanism merge in the judge, left for review rather than churned at a stop.
    - The ledger's `identity_rows` could read `boot::identity(...).unmodelled`, a second walk of the device list.
    - `absent_by_step` could return one list indexed by hook index.
    - The unmodelled records could ride `Read::Record`.
    - `mux_of` and `controller` could take a path.
    - `NotWitnessable`'s construction chain could be shortened.
    - The twin-limit sentence is restated in the ledger note and the finding.
  - **Kept, deliberate:**
    - `steps_of` in the tests, an independent oracle of the hook order.
    - `count` in `not_witnessable`, which the ruling names.
