# AET-WO-2311 — FEEDBACK: serial options seen in the twin (a measurement, then a build with one stop)

**Branch:** `wo/aet-2311-serial-options-twin` from `main` `cc99de04`; `main` `4b466f03` merged.
**Builder:** Claude (Opus 5.5) as project manager. No subagents: the judge, its expectation and the rule table are
gate logic, kept single-threaded.
Written 2026-11-23.

**Status: READY** (§8). The §1.2 stop was ruled at 13:45Z on the bus: D1 (b), D2 (b), D3, D4, the leans of D5 and D6,
and F2 conditional. Predictions: `spike/2311/PREDICTIONS.md`, committed `3ed3e7a2` before the measurement.
Results: `spike/2311/RESULTS.md`. Mutants: `spike/2311/mutants.py`.

**The answer:**
- **`simulate --foreign` judges each declared port's `USART_CR2` options against the specification.**
  - It reads them from the USART's access log (`LogPeripheralAccess`, D2 (b)).
  - It judges the last word written while `CR1.UE` was clear, the word silicon keeps (D1 (b), RM0433 Rev 9 §48.8.3,
    p. 2075).
  - **The expectation comes from the topology alone** (`aethelgard_twin::serial::options`, `INV-07`): SWAP clear;
    on RC input TXINV clear and RXINV unjudged; on every other port TXINV and RXINV each as `framing.inverted`.
- **What fails, each by name:**
  - a wrong bit: `serial_swap_set`, `serial_txinv_wrong`, `serial_rxinv_wrong`;
  - a `UE`-set write that changes a judged bit: `serial_options_ignored`, as ruled;
  - a non-word or misplaced write into `CR1`/`CR2`, or a log not read whole: `serial_options_unjudgeable` (D3);
  - no `UE`-clear word at all: `serial_options_unwritten`.
- **Each serial line's `PUPDR`** is read back at the run's end and reported as `serial_pull` (info, D5), and named in
  `boot.serial_pulls`. Per-port judgements are in `boot.serial_options`.
- **`ci/foreign-judge.sh` runs the four-port ELF** from the generated matrix job (D6). Run locally on the merged head,
  every run holds (§7):
  - unmodified ArduPilot passes, judged on all four ports;
  - SWAP on GPS and TXINV on the RC port are refused by name, as `var_info` default patches of that ELF;
  - GPS declared inverted refuses the unmodified ELF (TXINV and RXINV) and passes the ELF with both inverted.
- **Unchanged:** no trace field, no `fab/` byte, no ledger row, no IR or generator change.
- **Checks:** 16 of 16 mutants killed. The CLAUDE.md gates pass (§6).

## 1. The measurement (§1.1)

On the four-port ELF (built here from the committed hwdef in the pinned ArduPilot image), under `--foreign 5000ms`:

| Port | USART | Role | CR2 writes | First | Last written | Last written with UE clear |
|---|---|---|---|---|---|---|
| MAVLINK (SERIAL0) | USART1 | console | 4 | 0.0032 s | `0x40` at 0.1007 s, UE set | `0x40` at 0.0032 s |
| GPS (SERIAL1) | USART2 | gps | 1 | 0.0953 s | `0x40`, UE clear | the same |
| CRSF (SERIAL2) | USART3 | rc_crsf | 27 in 5 groups | 0.2017 s | `0x40` at 4.2095 s, UE set | `0x10040` (RXINV) at 4.2095 s |
| GPS2 (SERIAL3) | UART4 | gps | 1 | 0.0955 s | `0x40`, UE clear | the same |

`0x40` is LBDIE, set by ChibiOS and not stored by Renode.

- **The RC port's `CR2` changes four times.** AP_RCProtocol tries a config a second (`AP_RCProtocol.cpp:388-395`):
  - 8N1;
  - 8N1 with RXINV;
  - SBUS 8E2 with RXINV;
  - FastSBUS 8E2 with RXINV;
  - CRSF 8N1, at 4.2 s.
  - The next, 2 Mbaud, would be at about 5.21 s.
- **The hook adds 33 lines and 2,061 bytes** (mutants: at most 35 lines and 2,293 bytes). That is 0.025 % of the
  8 MiB budget.
- **`PUPDR` at the run's end:** all eight serial pins are pulled up (`01`), SERIAL0's PA9/PA10 included. The hwdef
  emits `FLOATING` for all eight.
  - The inversion mutant pulls PA2/PA3 down (`10`).
  - The RC TXINV mutant pulls PB10 (TX) down.
- **Mutations without a rebuild.** Patching one `AP_SerialManager::var_info` default in the ELF (`spike/2311/default.py`,
  pure Python) gives the same `var_info` and the same hook lines as rebuilding the hwdef with that
  `DEFAULT_SERIALn_OPTIONS`, measured for SWAP and for inversion.
- **Which accesses the hook sees:**
  - Every firmware `CR2` write is a word at offset 4.
  - Byte and half-word writes at offsets 5–7 reach the hook at their own offset, with the narrow value.
  - A half-word at offset 4 is indistinguishable from a word.
  - An unaligned word at offset 2 or 3 is dropped by Renode, and changes nothing in the twin.
  - A write while the USART's clock is off is dropped as locked, before the hook (silicon ignores it too).
  - The hook is armed before `RunFor`; the CLI's earlier writes are RCC only.

## 2. WO §5, the adversarial items, as measured at the stop

- **A CR2 write the hook misses.**
  - None is missed for ArduPilot.
  - A half-word at offset 4 is seen but misread as a word (D2).
  - A write through an alias: the USART has one registration, and a hook is per peripheral, so an alias would hit it.
  - A write before the hook is armed: none reaches a USART.
- **An expectation read from the artifact.** None yet: the build takes SWAP, TXINV and RXINV from
  `SolvedTopology::serial_ports` (role, `framing.inverted`) in `aethelgard-twin`'s serial module, never from
  `var_info` or `hwdef.h`. A test will name that.
- **A generated firmware that sets SWAP.** `judge_writes` refuses it today, by two rules:
  - `CR2 0x8000 set, a bit Renode does not store`;
  - `CR2 0xA000 at UE, not 0x2000`.

  This is now verified by a test (`serial::tests::a_generated_firmware_that_sets_swap_is_refused`, committed with this stop).

## 3. Decisions for the architect (§1.2), as put at the stop (ruled: see §5)

**D1. Which write is judged.** The WO says the last word before the window's end. RM0433 Rev 9 §48.8.3 (p. 2075) says
SWAP, TXINV and RXINV "can only be written when the USART is disabled (UE = 0)", and the hook's `CR1` column shows
ArduPilot (ChibiOS) writing `CR2` with `UE` set: USART1 three times, and every RC group's last write.
- **(a)** The WO's last word written. It can judge a word silicon ignored.
- **(b) Lean.** The last word written while `CR1.UE` read clear, which is the word silicon keeps. A write with `UE` set
  is not judged for these three bits.
- **(c)** Must-be-clear bits on every `UE`-clear write, must-be-set bits on the last.

All three pass the unmodified ELF and refuse every mutant. The WO's first-versus-last mutant applies to (a) and (b).

**D2. Width.**
- **(a)** The hook, as the WO says (the `CFG2` pattern), with a named residual. A half-word write at offset 4 is read as
  a word, so a firmware that set TXINV with a word and later wrote a half-word there would be judged TXINV-clear while
  silicon keeps it.
- **(b) Lean.** The access log (`LogPeripheralAccess`) on each declared USART, parsed by `serial::access`, the generated
  judge's parse. It carries `WriteUInt16`, so a non-word `CR2` write is refused by name, and `CR1`'s writes give `UE`.
  It costs 807 lines and 80,630 bytes per 5 s on four-port (1 % of the budget), 568 of them the console's `TDR` bytes,
  which the foreign log filter could drop.
- **(c)** Both: redundant.

**D3. A non-word or misaligned `CR2` write, and a port never written.**
- **Lean:** each fails closed by name: `serial_options_unjudgeable` and `serial_options_unwritten`.
- ArduPilot makes neither.

**D4. The finding kinds and their text (proposed).** Each failing kind names the port, its USART, the word, the time
and the specification's reason:

```
serial_swap_set:    GPS (USART2): CR2 0x00008000, the last word written with UE clear before the window's end (2.599100000 s), sets SWAP; serial_ports declares no swap
serial_txinv_wrong: CRSF (USART3, rc_crsf): CR2 0x00020000 … sets TXINV; on RC input TXINV is clear (RXINV is AP_RCProtocol's, unjudged; AET-WO-4404 FEEDBACK §2.5)
serial_txinv_wrong: GPS (USART2): CR2 0x00030000 … sets TXINV; framing.inverted is false
serial_rxinv_wrong: <label> (<USART>): CR2 … clears RXINV; framing.inverted is true
serial_pull (info): GPS TX (PA2, USART2): PUPDR pull-up (01) at the run's end, read back; not judged (roadmap §7.1 L1054, triage class B)
```

The verdict's `boot` gains `serial_options`. Per port it gives the label, the peripheral, the role, `inverted`, the
number of writes, the judged word and its time (or `null`), and each bit's expectation: `clear`, `set` or `unjudged`.
It also gains `serial_pulls`, per line: the label, TX or RX, the pin and the pull.

**D5. WO (2), the pulls: recorded or judged.**
- **Lean: recorded** (info), as the WO defaults, and L1054 stays with its ruling.
- Facts for that ruling: ArduPilot pulls every non-inverted line up and every inverted line down, on every port,
  SERIAL0 included. The hwdef emits `FLOATING` for all of them.

**D6. CI's shape, both inside the fence by the leans:**
- **(i)** `four-port.json` declares no `mcu.foreign_clock`, so `--foreign` refuses it.
  - **Lean:** `foreign-judge.sh` derives the declaration from sentinel-02-own's, as it already derives its other
    variants.
  - Adding it to the fixture would move the specification's hash, which is in the hwdef golden's header, and the
    topology golden: both outside the fence.
- **(ii)** The mutants.
  - **Lean:** `foreign-judge.sh` patches the `var_info` default of the four-port ELF (`default.py`'s logic, inline).
  - Rebuilt hwdef mutants would need the `generated` matrix job, outside "the foreign job only".
- **(iii)** Which mutants.
  - **Lean:** `SERIAL1_OPTIONS` 8 (SWAP) and `SERIAL2_OPTIONS` 2 (TXINV on the RC port), which runs the RC rule on
    real firmware.
  - `SERIAL1_OPTIONS` 3 (inversion on a non-inverted port) is a third, about 8 s more, if wanted.
- **(iv)** The foreign job downloads `generated-four-port` beside sentinel-02's.

## 4. Findings beside the fence

- **F1 (hwdef header, `aethelgard-hwdef`).** The four-port hwdef says "ArduPilot's set_options then pulls up TX and RX
  on SERIAL1 to SERIAL9". The twin shows SERIAL0's PA9/PA10 pulled up too. The sentence is narrower than the run; the
  owner of the generator should check the source path for SERIAL0. This is not changed here (no generator change).
- **F2 (fixture).** The four-port fixture has no inverted port. A required RXINV/TXINV, and RXINV missing on an
  inverted port, can only be tier-1 tests on planted logs. A fixture that judges it on real firmware needs a non-RC
  port declared `inverted` (e.g. a GPS port, `DEFAULT_SERIALn_OPTIONS 3`). (Since ruled and built in CI without a fixture change: §5.)
- **F3 (a twin residual for the rule row).** `set_options` writes `CR2` only when its word differs from what it reads
  (`UARTDriver.cpp:1574`, `:1699-1709`). The twin reads every tag as 0, so ArduPilot's write sequence is the twin's,
  not silicon's: the 4.2095 s group has no clearing write. The judged word is the twin's. The rewritten `RULES` row
  will say so.
- **F4 (today's register rule on the RC port, a backlog row).**
  - Today's judge reads USART3's `CR1` frame bits and `CR2.STOP` at the window's end, and at 5 s they are CRSF's
    8N1, as declared.
  - A window ending between 2.206 and 4.209 s would read SBUS's 8E2 and fail `register_mismatch` on unmodified
    ArduPilot.
  - The pass depends on the search's phase. This is outside this WO; it is proposed as a §7.1 row.
- **F5 (fence).** `crates/aethelgard-cli/README.md`'s "Foreign firmware" section lists the verdict's `boot` fields and
  kinds. The new kinds belong there, but the file is not in the fence. Ask: may the build add one paragraph there?
- **F6 (ledger).** The ledger's `CR2` rows (class `Ign`, `ledger.rs:1553-1566`) stay true and unchanged: "STOP is the
  only CR2 field the model stores". The foreign judge reads written words, not the model's state. No `RULES` text is
  quoted in the ledger or in `FAB-NOTES.md` (grep).

## 5. The rulings (13:45Z, `~/.cache/aet-bus/2311.md`) and what was built

- **D1 (b):** the judged word is the last `CR2` word written with `UE` clear. A `UE`-set write that changes a judged bit
  is `serial_options_ignored`.
  - It counts judged bits only: on an RC port RXINV is unjudged, so ArduPilot's own search does not trip it. The
    4.2095 s group's `UE`-set `0x40` clears only RXINV, the twin path of F3.
- **D2 (b):** `sysbus LogPeripheralAccess sysbus.<usart> true` for each declared USART, before the run.
  - The access log is parsed by `serial::access`, the generated judge's parse.
  - The foreign log filter already keeps every USART write.
  - It costs about 82 KiB on four-port (log kept 274,070 bytes, against 191,744 before).
- **D3:** fail closed by name (above).
- **D4:** the kinds as proposed, plus `serial_options_ignored` for the D1 rule. The text is in
  `renode::foreign::judge_serial`.
- **D5 (14:35Z):** pulls recorded, info.
- **D6 (14:35Z):** in `ci/foreign-judge.sh`:
  - sentinel-02-own's `mcu.foreign_clock` is derived onto four-port;
  - the mutations are `var_info` default patches, with no rebuild, measured identical to rebuilds;
  - the workflow's foreign job downloads `generated-four-port`.
- **F2 (conditional):** an inverted port in the fixture would move the four-port hwdef and topology goldens, which are
  outside the fence, so the fixture is unchanged. The inverted case still runs on real firmware: the script derives
  the specification with GPS declared inverted and judges the unmodified ELF (refused) and the ELF with
  `SERIAL1_OPTIONS` 3 (passes, PA2/PA3 pulled down). `SERIAL1_OPTIONS` 3 is what an inverted GPS port compiles
  (rebuild measured identical). A real inverted fixture remains a row (§9).
- **The `RULES` table:**
  - the row "USART CR1, CR2" says what is now judged and what is still weakened (RC RXINV, the twin's read-back path
    F3, CR1.RE);
  - a row "serial pins' PUPDR" says the pull is reported, not judged.

### Files

- **`crates/aethelgard-twin/src/serial.rs`:**
  - `Expect`, `Options`, `options(topology)`, `expected(role, inverted)`;
  - `OptionsJudgement`, `judge_options(options, accesses)`;
  - `pull_name`, `PinMux::pupdr`;
  - five tests.
- **`crates/aethelgard-cli/src/renode/foreign.rs`:**
  - the USART logs and `PUPDR` reads in `commands`/`reads`;
  - `judge_serial`, its report types and kinds, the `RULES` rows;
  - five tests on recorded access lines in Renode's measured shape.
- **`crates/aethelgard-cli/src/renode.rs`:** the two `commands` call sites, and one test holding the commands to D2.
- **`crates/aethelgard-cli/tests/boot_tier1.rs`:**
  - every foreign transcript now starts with the CONSOLE's measured bring-up and reads GPIOA's `PUPDR`;
  - one end-to-end test (pulls info; a planted SWAP fails by name).
- **`ci/foreign-judge.sh`, `.github/workflows/ardupilot-image.yml`** (the foreign job's download only).
- **`crates/aethelgard-cli/README.md`:** one paragraph (F5, granted).
- **Beside the named fence, flagged:** `crates/aethelgard-cli/src/main.rs`. It is plumbing only: it computes
  `serial::options(&topology)` once for a foreign run and passes it to `commands` and `judge` (+12 lines).
  - No other route reaches the topology: `BootPlan` carries none, and is built in `main.rs` too.

## 6. Adversarial pass, targeted (§5), on the build

- **A1. A `CR2` write the judge misses.**
  - With the access log every width is seen. A byte or half-word into `CR1` or `CR2`, or a word at offsets 1–3 or 5–7,
    is `serial_options_unjudgeable`, tested.
  - A write while the USART's clock is off is dropped before any log (measured), as silicon ignores it. The port then
    has no `UE`-clear word and fails `serial_options_unwritten`.
  - Logging is armed before `RunFor`.
  - The USART has one bus registration (`<0x40004400, 0x400047FF>`, measured).
- **A2. An expectation read from the artifact.** None.
  - `options` reads `SolvedTopology::serial_ports` (role, `framing.inverted`) and ST's header only.
  - A test flips `inverted` in the specification and the expectation follows.
  - CI judges the same ELF against two specifications, with opposite verdicts.
  - Nothing in the path reads `var_info` or `hwdef.h`.
- **A3. A generated firmware that sets SWAP.** `judge_writes` refuses it, by two rules, now under test.
- **A4. Renode's `collapse-repeated-log-entries`.** Measured: identical consecutive access lines are printed in full
  (seven identical `TDR` writes in a row, m4). The judgement is also idempotent under any fold of identical writes;
  only `writes` could undercount.
- **A5. A flood (finding, not built).**
  - The access log keeps each declared USART's `TDR` writes: ArduPilot's console wrote 568 in 5 s.
  - A foreign firmware that transmits faster than the 8 MiB budget allows fails closed: every port is unjudgeable and
    every witness is unread. That is never a false pass.
  - The foreign judge reads no `TDR` line, so `FOREIGN_LOG_FILTER` could discard them. This is proposed as a row
    under the scope test, not built.
- **A6. UE's source.** `UE` is rebuilt from `CR1` word writes from reset (clear). A narrow `CR1` write is
  unjudgeable, so no write can flip `UE` unseen.

## 7. The four-port runs, on the merged head (local, tools image `1e0ab105`, ELFs built here in `e1809b90`)

FINAL_RUNS

The existing sentinel-02 runs keep their kinds: `window-before-u5-data` reads `core_in_exception` as `main`'s CI run
45179832134 does.

## 8. Gates and mutants

GATES_LINE
- **Mutants** (`spike/2311/mutants.py`; one per guard this WO adds, each against the suites it lives in): **16 of 16
  killed**, each by a named test. The SWAP check removed; TXINV and RXINV exchanged (masks, and kinds); the first
  `UE`-clear write judged in place of the last; `UE` ignored; the ignored-write rule dropped or widened to unjudged
  bits; a narrow or misplaced write accepted; RC RXINV judged; RC TXINV following `inverted`; unwritten demoted to
  info; a log not read whole judged; the pull made to fail; the USARTs not logged; a pull not read.
  - One mutant survived the first run: unwritten demoted to info. The tests' `failing()` filtered by kind, not
    severity. Fixed, and re-killed.
- **Reduction pass (diff):** −16 net · 1 duplicate removed (the expected-options rule, once in `options` and once in
  the twin tests' helper, now `serial::expected`) · 2 dead paths removed (the pull loop's unreachable `else continue`
  branches) · 0 protected kept · 2 findings below.
  - Findings: `spike/2311/default.py` and `derive.py` are the measurement's record, with their live copies in
    `ci/foreign-judge.sh`; the spike's are kept as the record of what was measured. The RC rule also lives in
    pincheck's `serial_settings`, a separate judge of the ELF, out of fence and deliberate (`RUL-20`).

## 9. Questions and rows for the architect

- **F5, granted 14:35Z:** one paragraph, "The serial options", in `crates/aethelgard-cli/README.md`'s "Foreign
  firmware" section, and nothing else in that file.
- **Proposed §7.1 rows:**
  - F4, the RC port's frame bits read at the cut;
  - A5, drop `TDR` writes from the foreign log filter;
  - F2, a real inverted port in a fixture;
  - F1, the hwdef header's "SERIAL1 to SERIAL9" pull sentence (SERIAL0 is pulled up too).

## 10. Signal

The last push is the commit "AET-WO-2311 READY: …". Nothing is pushed after it unless asked.
