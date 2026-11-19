# AET-WO-3403 — FEEDBACK (build: Sentinel-01 as ordered, the bench plan made able to fail) — READY

**Branch and inputs.**
- Branch `wo/aet-3403-s01-bench-plan`, from `main` at `47ec18f6`.
- The work order is read from `~/.cache/aet-3403-wo.md`, the architect's copy; it is not on `main`.
- The review is `~/.cache/aet-rev-s01/REVIEW.md` (sha256 `cb488b65…a4cd`).

Each statement is labelled **measured** (run on Furnace in the pinned image), **read at source**, or **inferred**.

## 0. What changed, and the fence

| File | Change |
|---|---|
| `docs/reviews/2026-11-19-sentinel-01-as-ordered.md` | The review, verbatim, under one header line naming its source path, sha256 and session (**measured:** the file from line 3 on hashes to `cb488b65…a4cd`) |
| `fab/sentinel-01-bench/BENCH.md` | Every row rewritten to fail for its named reason, each pass line with its source class; "Before power"; "What the ordered ELF puts where"; "U2 empty"; new row B9 |
| `fab/sentinel-01-bench/ORDER.md` | U2 not fitted, and why (§2, §3) |
| `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` | New: the BMP581 on J4, its ELF, predictions and cable |
| `examples/sentinel-01-ext-bmp581.json` | New: the bench specification |
| this file | — |

**Nothing ordered moved** (**measured**): `git diff origin/main -- fab/sentinel-01-bench/package examples/sentinel-01.json boards/` is empty, against `origin/main` at `56639a89` (merged in). The local `main` ref on Furnace is stale, so the command was run against `origin/main`. No crate, model, golden, test, workflow, `ci/` or `container/` file changed.

## 1. The review's recommendations, one by one

Every recommendation was applied, two of them by a different method than the review named. Neither deviation disputes a finding: in both, the finding holds and the named method could fail for a reason the row does not name. So neither is a §7 stop.

| Finding | Applied as | Where |
|---|---|---|
| F1 | The console's 12 bytes at 115200 8N1 (B1, B8.1); the records read over SWD, word by word, with the `probe-rs` command; record 0 per U1 | BENCH "What the ordered ELF puts where", B1, B4, B8 |
| F2 | B7 rewritten: two edge rates solved jointly; the clock calibrated from the SCL low phase and cross-checked on the console; a band derived from DS12117, with its uncertainty formula | B7; §2 below |
| F3 | **Deviation:** a held reset read on the DMM, plus unpowered continuity from the probe's NRST wire to TP3, instead of a scope on a connect-under-reset | Before power step 4, B1 item 1; §2 below |
| F4 | Every pull-up read pad-to-TP4 before power, 2.2 k / 10 k; the implied C on every B6 line, pass or fail | Before power step 3, B6 |
| F5 | TP2 to TP1 = 10 kΩ unpowered, **and** the 100 kΩ divider as an optional powered check | Before power step 3, B8 |
| F6 | New row B9: the firmware's `0xEC` address phase decoded across reset, with an SCL period band; record 0 = TXIS; the TMP117 as the positive I2C1 witness. **Deviation:** an address probe at `0x48` (with a `0x76` control) instead of reading register `0x0F` | B9; §2 below |
| F9–F11 | "Before power": supply at most 3.6 V and current-limited; each cable labelled; J1 and J3 never swapped; J2's own pinout, not Dronecode's; SW1's pads checked before fitting | Before power steps 1, 2, 6 |
| F14 | `PWR_CR3`, `PWR_CSR1.ACTVOSRDY`, `PWR_D3CR`, labelled as `4240`'s bench row B-2, after a power cycle | B2 |
| U1 | The U2-empty path: the code trace by `file:line`, row by row, and what is lost | BENCH "U2 empty", ORDER §2 |
| U3 (a) | The bench specification, ELF, predictions and cable | `BENCH-EXT-BMP581.md`, §3 below |

F7, F8, F12 and F13 recommend nothing for this board's bench; F7's engine work is a §7.1 row, as the work order says.

## 2. Where the method differs from the review's, and why

### 2.1 F6/U1: the TMP117 is probed by address, not read

**The review's method:** "write `I2C1.CR2` for a 1-byte write then a 2-byte read, as 3302 drove I2C over SWD" and read register `0x0F` = `0x0117`.

**Why it can fail for the wrong reason** (**read at source**, TI SNOSD82D, the review's own copy, sha256 `78212e91…`):
- p. 7, the timing table: "Serial bus timeout (SDA bus released if there is no clock)", 20 to 40 ms.
- §7.5.3.1.7, p. 22: "The TMP117 resets the serial interface if the SCL line is held low by the master … for 35 ms (typical) between a START and STOP condition."
- A debugger-driven read waits on `TXIS`, `TC` and `RXNE` between bytes, and the H7 holds SCL low while it waits (RM0433 Rev 9 §47.4.9). One `probe-rs` invocation per register access is far slower than 35 ms [inferred: each attaches anew]. So the read would time out in the TMP117 and fail, with I2C1 and the copper correct.

**What B9 does instead.** `CR2` = address `0x48`, write, `NBYTES` 0, `AUTOEND` 1, `START`, in one write. RM0433 Rev 9 p. 1975 (**read at source**): with `AUTOEND` 1, "a STOP condition is automatically sent" once the `NBYTES` bytes are transferred, and "when a NACK is received, the TXIS flag is not set and a STOP condition is automatically sent. The NACKF flag … is set." So the controller sends the address and stops by itself, and nothing stretches SCL. `ISR` then reads `STOPF` with or without `NACKF`. That `NBYTES` = 0 ends the transfer right after the address is [inferred] from p. 1975's rule, which states no exception for zero; the `0x76` control in B9 is what makes the row judge it (if the probe did nothing, both reads would match and the row fails).

**What is weaker:** an ACK at `0x48` shows a device at the TMP117's address, on PB6/PB7, through the SENSORS copper. It does not show the device is a TMP117 (`0x0117`). The specification puts only U3 at `0x48`. The architect may want the identity read anyway, run fast enough: an OpenOCD script (OpenOCD is already B1's instrument) or a GDB script through `probe-rs gdb` could issue the whole sequence in milliseconds. I did not write one, because its timing could not be checked here.

### 2.2 F3: a held reset, not a scope on connect-under-reset

The review's first option was "TP3 at or below 0.3 V while the probe holds reset". A scope on a connect-under-reset can be fooled: the MCU itself drives NRST low for at least 20 µs on every internal reset, a software reset among them (RM0433 Rev 9 §8.4.2, p. 330, **read at source**), and a probe may fall back to one. So B1 holds reset explicitly and reads TP3 on the DMM, and Before power checks the cable-to-TP3 continuity unpowered. The OpenOCD `adapter assert srst` syntax is not run here (no OpenOCD on Furnace) and is labelled so.

### 2.3 F2: B7's band and its method

- **The threshold without the clock.** In each SCL period, `P = a(V_th) + t_SYNC2 + SCLH`, measured from the controller's own falling edge (RM0433 p. 1971's period formula, **read at source**). The delay and the count are the same at both edge rates, so `V_th` is the V where `a_B(V) − a_A(V) = P_B − P_A`. Neither the kernel clock nor any RC model enters. The review's "solve jointly" is this; the work order's "kernel clock calibrated from the SCL low phase" is kept for the delay figures, with its bias stated: the low phase includes `t_SYNC1` (RM0433 p. 1971). The console's `00` byte is a synchronisation-free cross-check on the same HSI (`BRR` 556, 9 bits low, 5004 cycles).
- **The band and its uncertainty** are derived in `BENCH.md` B7: DS12117 Rev 10's by-design thresholds (Table 146, p. 241, rev V; Table 49, p. 136, rev Y; read in the review's extraction `~/.cache/aet-rev-s01/ds/ds12117.txt`, sha256 `010f5f1b…`), 1.22–1.80 V at 3.30 V, against an expected ±17–35 mV. A controller counting from 0.7·VDD reads about 2.31 V and fails it.
- **The old band** (1.7 ± 0.2 V) is dropped. Its centre was `3302`'s sawtooth peak, which includes `t_SYNC2` (the review's F2).

## 3. The bench specification

**The file** is described in `BENCH-EXT-BMP581.md` §0 (sha256 `4060c9ef…b174`, specification 16), including why `acknowledge_unwitnessed` is emptied.
- **No purpose field.** The work order asks the specification's purpose to say it describes a board never made. The schema has none (`Specification` is `deny_unknown_fields`), and its free-text fields are cited sources of at most 200 bytes, which `validate` enforces (it refused my first attempt, `InvalidSource`). So the statement is in `BENCH-EXT-BMP581.md` §0 and in `BENCH.md`'s header.
- **The address `0x47`** rests on Adafruit's guide (read) and SparkFun's listings (search summaries; its own page returned 403). `BENCH-EXT-BMP581.md` §2.1 has Scotty confirm it on the board with an address probe before flashing.

**The runs** (**measured**, the pinned image `e5fc7ddb…`, `capped-run`, `nice -n 10`, `--network none` except the cargo build; the CLI and bridge built as CI's `bench-package` job builds them):

| Run | Result |
|---|---|
| `validate`, `solve` | ok. The solved topology equals the ordered one in `assignments`, `gpio`, `clock`, `i2c_timings`, `pin_speeds`, `spi_controllers`, `analog_lines`; only `i2c_devices` and the specification hash differ |
| `generate`, `build`, twice at two paths | ELF **`a60306c9ac32fbbf13bea1e610c546c94307bd04b080166651fef9953febb761`** both times. `diff -r` against `boards/sentinel-01-bench/`: names, hashes in comments, and `main.rs` only |
| `simulate --for 100ms` | **pass**: 25 register rows under their masks, U2 `0x60`, U5 identity `0x50`, configure, data, U4 `EF4017`, console pattern witnessed |
| `verify`, the package's five scenarios rebased | **refused**, exit 5, at the campaign: `droop-bor3` (alone too) stamps the console injection 1 ns early after the brown-out reboot. `4201`'s fault (#237); the twin is `4400g`'s area and was not touched |
| `verify`, the other four | **`fail`, one gate: `extract`**, `SPI_MOSI` 47.4 pF against the declared 30 pF. Every other gate passes; all four scenarios matched. 17 min 36 s wall |

**What the `extract` failure means** is in `BENCH-EXT-BMP581.md` §1, S2 [measured: `extract` on both emitted boards, SPI_MOSI 49.9 mm / 47.4 pF against the ordered 20.9 mm / 24.6 pF]: a board never made, not the ordered copper, and not the ELF; but this ELF has no passing `verify` verdict, and none of the four scenarios that ran touches EXT or U5.

**D1, ruled by the architect 2026-11-19 (bus, 18:06Z): accept.** This specification's `verify` verdict **fails**, and the sheet says so plainly:
- its `extract` judges copper that does not exist (SPI_MOSI 49.9 mm against the ordered 20.9 mm, U5 placed on a board never made);
- its campaign cannot run `droop-bor3`, the known BMP581 twin fault (`4201`).

**The ordered package's verdict is the one that judges the copper.** What the bench uses is this ELF, whose solved topology equals the ordered one in every assignment, timing, clock and pin speed. I did not widen SPI_MOSI's declared bound or move U5 to make the gate pass.

**D2, ruled by the architect 2026-11-19: keep `0x47`**, the common breakouts' default. `3304`'s Nucleo sheet assumed SDO to GND (`0x46`), so the sheet now predicts both straps:
- SDO high gives U5 identity `0x50`;
- SDO low gives a NACK at `0x47` and TXIS records (the generator's path, `BENCH-EXT-BMP581.md` §3).

Scotty reads the strap before power and records it.

**The sheet's B2 table** is generated by `~/.cache/aet-3403/sh/b2.py` from a `simulate` of the ordered ELF on Furnace (pass; `generate` reproduces `boards/sentinel-01-bench/` byte for byte and builds `008a2a59…`), classed by the ordered verdict's ledger (the review's Furnace run, `work/against.log`, 342 rows, 0 unacknowledged).

**The ELF and evidence for Scotty:** `~/.cache/aet-3403/bench/` with `SHA256SUMS` (ELF, specification, `.repl`, `simulate` verdict, both `verify` verdicts, the `droop-bor3` refusal).

## 4. Every row, the fault that fails it, and its source

| Row and line | A fault that fails it | Source |
|---|---|---|
| Before power 2: SW1 pairing | KiCad's PTS810 pairing wrong (NRST tied to GND) | not predicted (the maker's sheet was unreadable) |
| Before power 3: pull-ups, R8 | a 2.2 kΩ on EXT or 10 kΩ on SENSORS; R8 missing; a rail short | rule (`ORDER.md`'s parts) |
| Before power 4: NRST continuity | NRST open in the cable, at J2 pin 3 or in the copper | rule (the netlist) |
| B1.1 TP3 held low | NRST not reaching TP3 from the probe | rule (DS12117 Table 151's pull-up) |
| B1.2 flash and verify | SWD lines, the probe or the flash | not predicted |
| B1.3 the 12 console bytes, counted from the first `43` | PA9 not at AF7, a wrong `BRR`, J1 miswired | twin |
| B2 register rows | a register word differing from the twin's or the generator's | per row (twin, generator, rule, not predicted) |
| B2 PWR reads | none: recorded, not judged | not predicted (said in the row) |
| B3 `BOR_LEV` = 11 | the option-byte procedure not taking | not predicted (silicon-only); the `00` before is a repeat |
| B4 record 1 = `EF4017` | SPI1's pins, AF, the flash, PA0 | twin; Winbond |
| B4 `MODF` = 0 | the first scar back | not predicted (Renode has no `MODF`) |
| B4 `MASTER`, SCK 15.59–16.24 MHz | a wrong `CFG2` or `MBR`; PA5 not at AF5 (no clock) | rule |
| B5 BOR trips | `BOR_LEV` not set, or the part's VBOR3 outside Table 115 | rule |
| B6 rise ≤ bound | copper plus probe above `extract`'s bound (a wrong pull-up is Before power's) | rule (`extract`) |
| B7 `V_th` in 1.22–1.80 V ± u | the controller counting from 0.7·VDD (≈ 2.31 V), or a threshold outside DS12117's | rule (DS12117) |
| B8.1 BOOT0 ≤ 0.727 V | BOOT0 pulled up or shorted to VDD | rule (`1312`) |
| B8.2 0.25–0.35 V through 100 kΩ | R8 missing or wrong | rule (the divider) |
| B8.3 BOOT0 ≥ 1.161 V | BOOT0 shorted to GND (a check, not a judgement) | rule; the bootloader's answer is not predicted (AN2606 unread) |
| B9.1 decode `76`, NACK, Stop; record 0 TXIS | a wrong address, no STOP, a lockup | generator; RM0433; repeat |
| B9.1 SCL period ≥ 2.5247 µs | a wrong `TIMINGR` or clock; the 2.65–2.90 µs band beside it is recorded, not judged | rule (floor); repeat-derived (band) |
| B9.2 ACK at `0x48`, NACK at `0x76` | PB6/PB7 not at AF4, SENSORS copper open, U3 missing; the probe not reaching the controller (control) | RM0433; the specification's address |
| EXT 2.1 exactly one address ACKs | the breakout's strap, the cable, its supply | RM0433 |
| EXT 2.3 cable continuity | a reversed or crossed cable | not predicted (a wiring check with a pass line) |
| EXT 3 U5 records against the strap read before power: SDO high, OK `0x50`, OK, OK; SDO low, TXIS ×3; U4 at its new address | the cable, PB10/PB11, the breakout's supply; a misread strap | twin (SDO high); generator (SDO low) |
| EXT 3 U2 record TXIS | as B9.1 | generator; repeat |
| EXT 4 the decoded transactions | a missing repeated start, a wrong byte or count | twin; generator (Sr) |
| EXT 4 SCL period ≥ 10.037 µs | a wrong `TIMINGR` or clock | rule (can fail only so; labelled) |
| EXT data values | none: the room | not predicted |

**Predictions written from a run that already shows the answer** (§5's second lens): the twin's records and decoder lines are written from S1, which is the source they cite; none was adjusted after a run disagreed. The expectations for `verify`'s scenarios were written to `~/.cache/aet-3403/runs/EXPECTED-before-run.md` before the run.

## 5. Specification 17

`4400g` (#238, specification 17) was **open** when this was READY, so the new specification is written at **16**, as the work order says, and the bus says so. If #238 merges first, the new file needs its `schema_version` bumped, nothing else that I know of.

## 6. An observation outside the fence

The ordered verdict's ledger (the review's Furnace run, `work/against.log`) gives `I2C1.TIMINGR`'s field values as `PRESC` 5, `SCLH` 42, `SCLL` 64: the `Standard100k` word, `0x50D32A40`. The ordered firmware writes `Fast400k`, `0x10C7123C`, to I2C1 (`main.rs` L168; `board.rs` L175). The fields' class (independent-rule) is right; only the reported value is another mode's. B2 uses the firmware's word. It looks like the ledger takes the first speed mode's word for a register the board crate can write in several modes [inferred, not traced]. A §7.1 row if the architect agrees.

## 7. Adversarial pass (§5, targeted)

Two independent readers ran, each given the files and the work order, not my conclusions:
- an Opus reviewer, given all four files, the ground-truth sources and §5's four lenses;
- a reader given **only** the cable table, asked to wire it.

Their findings are in `~/.cache/aet-3403/adv/` (`FINDINGS.md`, `CABLE.md`). I checked each at source before adopting it, and fixed every one except where noted.

| # | Lens | Finding | Fix |
|---|---|---|---|
| C1 | pass line a fault slips through / fails a correct board | B6's `BSRR` does nothing on a pin in AF mode, and a `MODER` change persists. B2, B7 and B9 after it would then fail a correct board, and B9 would blame the AF | B6 gives the `MODER` read-modify-write and the `BSRR` words, then a reset; B2 runs before B6, B7 and B9 |
| C3 | fault slips through | The records are never cleared: `.aethelgard_boot` is `NOLOAD`, and a reset keeps RAM. A boot that dies early still shows the last good boot's words | Power cycle, or zero the words, read them back, then reset (both sheets) |
| C7 | fails a correct board | B1 left PA10 floating (no pull), so stray bytes would be echoed | The adapter's TX is wired to J1 pin 2 |
| C2 | could mislead | B8's "1 TX, 2 RX" reads as the adapter's TX on the board's TX | Wired as B1 |
| C4 | could mislead | B9's probe ran inside the 4 s capture that must show one transaction | Its own 60 s capture |
| C5 | could mislead | SDO and CS were "unconnected" on any breakout; Bosch says SDO must not float | Unconnected only where the breakout straps them; otherwise tied to its supply pad |
| C6 | WO | The never-made statement was missing from `BENCH.md` | Added to its header |
| W1–W3 | facts | `board.rs` lines off by two; RM0433's reset text is on p. 330; `proofs.rs` also differs | Corrected |
| N1–N12 | nits | B9's band is recorded, not judged; echo timing; header order J1 J2 J3 J4; PA9 hazard softened; Dronecode RX on BOOT0; B7's slope assumption labelled [inferred]; U2 DNP for fab assembly; flash commands; absolute scp path; source labels | Applied |
| cable | could wire backwards | Adafruit's 3Vo pad beside VIN; a wire on SDO passed "the pad I wired"; no SCL/SDA-to-GND check; the lead's pin 1 could not be identified; the Qwiic colour column contradicted "never by colour" | Each wire is identified by continuity to the test pads with the lead in J4, and probed by the pad's printed label; three rows added; the colour column removed |

**Left as is:**
- **B1.3, counting from the first `43`:** the reviewer could not confirm whether PA9 glitches before USART1 is enabled. A byte before the first `43` is recorded, not judged.
- **BOOT0's 15 nA leakage:** that is `1312`'s reading, unchanged.
- **`probe-rs` 0.32's syntax:** labelled unverified throughout.

**My own lens 2 check:** every twin prediction cites S1, its source, and none was edited after a run disagreed. `verify`'s expectations were written before the run (`runs/EXPECTED-before-run.md`). The droop refusal was expected there (`4201`), and the `extract` failure was not.

## 8. Reduction pass and gates

Reduction pass (diff): −3 lines net · 5 duplicated rationales collapsed to one home each (the TMP117 probe's reason, the held reset's reason, the bench specification's description, B7's band, the `extract` failure's meaning: each now lives in one file and the other cites it) · 0 dead paths · 0 protected kept · 0 findings outside the fence. The review file is verbatim and was not touched.

**Gates** (**measured**, in the pinned image, `capped-run --mem 16G`, `nice -n 10`):
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`, with `CARGO_TARGET_DIR` inside the repository as CI has it: **136 test binaries, 2,258 passed, 0 failed.**
  - A first run with the target directory outside the repository failed `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named` with `BaseOutsideRoot`. The test writes its scenarios under the target directory, and the CLI refuses a base outside the working directory. That is my harness, not the diff; in CI's layout the file passes 11/11.
- **The four tests that walk `examples/`, with the new file in place:** `silicon_rules` 1/1, `ledger_tier1` 12/12, `hwdef_tier1` 5/5 (cli), `silicon_words` 6/6 (svd-hal).

**What CI ran** (**measured**, PR #247 at `e9ce63c7`, with `origin/main` `56639a89` merged in):
- `ci` run 44579084240 and `ardupilot-image` run 44579084211: every job passed (27), and `publish` was skipped as designed.
- That covers `gates`, eight shards, eight tools shards and `bench-package`. Tier 2: **190 passed, floor 190**; the tools jobs passed 2,470.
- `1911` has not landed, so a docs-and-`examples/` PR still ran the full matrix.
- The READY commit after it changes only this file and `BENCH-EXT-BMP581.md`, the architect's D1 and D2 rulings. Its own CI run is the record.
