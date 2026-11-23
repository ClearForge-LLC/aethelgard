# AET-WO-2311 — FEEDBACK: serial options seen in the twin (a measurement, then a build with one stop)

**Branch:** `wo/aet-2311-serial-options-twin` from `main` `cc99de04`.
**Builder:** Claude (Opus 5.5) as project manager. No subagents: the hook, the judge and the rule table are
gate logic, kept single-threaded.
Written 2026-11-23.

**Status: STOPPED at §1.2**, as the WO plans. Predictions: `spike/2311/PREDICTIONS.md`, committed `3ed3e7a2`
before the CR2 run. Results: `spike/2311/RESULTS.md`.

**The answer:**
- **No other §7 stop fires.**
  - Every declared port's first `CR2` write is by 0.202 s, well inside 5 s.
  - No trace field is needed: a foreign run is never archived.
  - No `fab/` byte moves, and no ledger row is touched.
  - The four-port fixture has an RC port (CRSF on USART3), though no inverted port (F2).
- **Today's foreign judge passes all three ELF mutants.** These are SWAP on SERIAL1, RXINV|TXINV on SERIAL1, and
  TXINV on the RC port. In the twin `CR2` reads 0 for every tag.
- **The hook pattern works,** with one hole: a half-word write at offset 4 logs exactly as a word. The hook's
  scope has `offset` and `value`, no width (D2).
- **Six decisions are needed before the build** (§3). Two come from measured facts the WO did not foresee:
  - **SWAP, TXINV and RXINV take only while `UE` is clear.** RM0433 Rev 9 §48.8.3, p. 2075 says so, and ArduPilot
    writes `CR2` with `UE` set too (D1).
  - **The hook cannot see a write's width** (D2).

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

## 2. WO §5, the adversarial items, as far as measured

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

## 3. Decisions for the architect (§1.2)

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
  port declared `inverted` (e.g. a GPS port, `DEFAULT_SERIALn_OPTIONS 3`).
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

## 5. Gates

For this stop: one test added in `aethelgard-twin`, run alone and passing. The CLAUDE.md gates run at the build.

## 6. Signal

The last push is the commit "AET-WO-2311 STOPPED: §1.2 …". Nothing is pushed after it unless asked.
