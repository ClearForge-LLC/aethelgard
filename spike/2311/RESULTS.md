# AET-WO-2311 §1.1 — results, against `PREDICTIONS.md` (committed `3ed3e7a2` before the CR2 run)

**The kit.** Every run is in the pinned tools image `1e0ab105` under `~/bin/capped-run` (8 GiB, swap 0,
read back), one container at a time.
- **ELFs** (`out/elves.sha256`): `four-port` was built here from the committed `four-port.hwdef.dat` in
  the pinned ArduPilot image `e1809b90` (29 s).
- **Rebuilt mutants:** `four-port-swap` and `four-port-inv` are that hwdef with one line changed:
  `DEFAULT_SERIAL1_OPTIONS` 8 (SWAP) and 3 (RXINV | TXINV).
- **Patched mutants:** `patch-swap`, `patch-inv` and `patch-rctx` are the unmodified ELF with one
  `var_info` default changed in place by `default.py`, with no rebuild:
  - `patch-swap`: `1_OPTIONS` 0 → 8;
  - `patch-inv`: `1_OPTIONS` 0 → 3;
  - `patch-rctx`: `2_OPTIONS` 0 → 2, TXINV on the RC port.
- **Specification:** `four-port.json` plus sentinel-02-own's `mcu.foreign_clock` (`derive.py`).
- **Hooked runs:** `sh/capture.sh` runs the unmodified CLI and copies its Renode argv. `measure.py` then
  replays that monitor script and adds three things:
  - a write hook on each declared USART logging every write to offsets 4–7, with CR1 as it stood;
  - optional probe commands;
  - after the run, each serial port's GPIO `PUPDR` and each `CR2`, read back.

  `sh/measure.sh` runs it under the CLI's own `renode.config`. Extracts are in `out/<run>.txt`.

## Per prediction

| # | Predicted | Measured | |
|---|---|---|---|
| P1 | Every declared USART takes at least one logged `CR2` write; SERIAL1–3 several | USART1 (SERIAL0) 4, USART2 (SERIAL1) **1**, USART3 (SERIAL2, RC) 27, UART4 (SERIAL3) **1** (`out/m1.txt`) | Half: the GPS ports take one write each in 5 s; no autobaud `begin` rewrites `CR2` |
| P2 | Every first `begin` inside 5 s; console and GPS before 1 s | First writes: USART1 0.0032 s, USART2 0.0953 s, UART4 0.0955 s, USART3 0.2017 s | Holds. WO §7's third stop does not fire |
| P3 | The RC port's `CR2` changes more than once, a second apart | Five groups on USART3, at 0.2017, 1.2039, 2.2060, 3.2072 and 4.2095 s: `0x40`; `0x10040` (RXINV); `0x12040` (RXINV, `STOP` 2 stop bits; `CR1` `0x50d`, `PCE`); `0x12040`; `0x40`. The next config (2 Mbaud) would be at about 5.21 s | Holds: AP_RCProtocol's six configs, one a second |
| P4 | No word sets SWAP or TXINV; RXINV only on USART3 | Holds on the unmodified ELF. Each mutant shows its bit: swap `0x8040`, then `0x8000` at 2.599 s; inv `0x30040`, then `0x30000`; rctx TXINV on each of USART3's 20 non-zero words (of 29; 9 are re-`begin`'s transient 0), last `0x20040` | Holds |
| P5 | Under 100 lines and 16 KiB | 33 lines, 2,061 bytes (base); 34–35 lines, at most 2,293 bytes (mutants). That is 0.025 % of the 8 MiB budget | Holds |
| P6 | `PUPDR` at 5 s: SERIAL1–3 pulled up, SERIAL0 floating | GPIOA `0x00140055`, GPIOB `0x00500000`: **all eight serial pins pulled up (`01`), SERIAL0's PA9/PA10 included**. inv: PA2/PA3 `10` (down). rctx: PB10 `10` (TX down), PB11 `01` | **Missed on SERIAL0.** The hwdef header's "SERIAL1 to SERIAL9" is narrower than what the twin shows (FEEDBACK §4, F1) |
| P7 | Every write is a word | Every firmware `CR2` write is at offset 4. Probes in `out/m2.txt`, `m3`: (a) byte and half-word writes at offsets 5–7 reach the hook at their own offset with the narrow value; (b) a half-word at offset 4 logs exactly as a word (`4 8000`), because the hook's scope has only `offset` and `value`, no width; (c) an unaligned word at offset 2 or 3 is dropped by Renode (`Unhandled write to offset 0x2`), and the hook's `offset >= 4` does not log it; (d) a write while the USART's clock is off is dropped as a locked access before any hook | Holds for ArduPilot. (b) is a hole in a hook-only judge (FEEDBACK D2) |
| P8 | No trace field | `simulate --foreign` is never archived (`campaign.rs:1218` `foreign: false`). The findings and a `boot` field ride the verdict JSON, which has no trace version | Holds. WO §7's second stop does not fire |
| P9 | No `fab/` byte moves | Nothing touched eda, IR, generators or the ledger. The `RULES` row's text appears in no ledger row and no `FAB-NOTES.md` line (grep) | Holds, by construction; the build re-checks it |

## Also measured

- **Today's judge on the mutants:** `simulate --foreign 5000ms` with the unmodified CLI **passes** `patch-swap`,
  `patch-inv` and `patch-rctx`, each with no failing finding (`out/today-*-verdict.json`). `CR2` reads 0 for
  every tag, so the register rule sees nothing.
- **The two mutation routes agree.**
  - `var_info` is identical between the rebuilt and the patched ELFs (`1_OPTIONS=8`, 4404's `varinfo.py`).
  - Their USART2 hook lines are identical.
- **UE at each write**, from the hook's `CR1` column:
  - USART1 writes `0x40` once with UE clear (0.0032 s), then three times with UE set (`CR1` `0x10d`).
  - USART3's last write in each group is with UE set (`0x40 10d` at 4.2095 s).
  - RM0433 Rev 9 §48.8.3, p. 2075, on SWAP, TXINV and RXINV: "This bitfield can only be written when the
    USART is disabled (UE = 0)". So silicon keeps the last word written with UE clear, not the last word written.
  - On USART3 that word at 4.2095 s is `0x10040`, with RXINV set (RC's, unjudged); the last word written is `0x40`.
- **The twin's read-back changes ArduPilot's path.**
  - `set_options` reads `CR2` and writes only when its word differs (`UARTDriver.cpp:1574`, `:1699-1709`).
  - The twin reads every tag as 0, so a cleared tag looks already clear and no write is made.
  - The 4.2095 s group has no `…10c` clearing write, unlike the 1.2 s and 2.2 s groups.
  - The sequence of words is the twin's, not silicon's.
- **Transient zeros.** Every RC re-`begin` writes `CR2` = 0 with UE clear first (`0 0`). A last-word judge
  whose window ends inside a group would read 0. The writes of one group share a quantum (same stamp)
  except the final UE-set write, one quantum later.
- **`LogPeripheralAccess` instead of a hook** (`m4`):
  - It gives the width, `WriteUInt16`.
  - On the four declared USARTs over 5 s it logs 807 lines and 80,630 bytes (about 1 % of the budget).
  - 568 of those lines are the console's `TDR` bytes.
- **The RC port's frame at the cut.**
  - Today's register rule reads USART3's `CR1` frame bits and `CR2.STOP` at the window's end.
  - At 5 s they are CRSF's 8N1, as declared.
  - A window ending between 2.206 and 4.209 s would read SBUS's 8E2 and fail `register_mismatch` on a firmware
    doing what ArduPilot always does. Today's 5 s passes by the search's phase.
- **The generated judge (WO §5, third item):** `judge_writes` refuses a generated firmware that sets SWAP,
  twice: `CR2 0x8000 set, a bit Renode does not store` and `CR2 0xA000 at UE, not 0x2000`. That is now
  verified by `serial::tests::a_generated_firmware_that_sets_swap_is_refused`.
