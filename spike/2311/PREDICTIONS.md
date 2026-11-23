# AET-WO-2311 §1.1 — predictions, committed before the CR2 and PUPDR measurement

Written 2026-11-23 after one measurement only: the unmodified CLI's `simulate --foreign 5000ms` on
the four-port ELF (built here from the committed `four-port.hwdef.dat` in the pinned ArduPilot image
`e1809b90`, sha256 `5d931274…`), on `four-port.json` with sentinel-02-own's `mcu.foreign_clock` added
(`derive.py`; `four-port.json` declares none, so `--foreign` refuses it as `ForeignClockUndeclared`).
That baseline passes: booted by U4 at 0.166 s, U2 identity 0.390 s, 191,744 bytes kept, nothing dropped,
and every port's `CR1`/`CR2` equal to the solved framing under today's masks at 5 s.

Ports (`SERIAL_ORDER USART1 USART2 USART3 UART4`): SERIAL0 console USART1 (PA9/PA10), SERIAL1 GPS USART2
(PA2/PA3), SERIAL2 CRSF USART3 (PB10/PB11, RCIN, no `DEFAULT_SERIAL2_OPTIONS`), SERIAL3 GPS UART4
(PA0/PA1). None declares `inverted`.

Source read (pinned image, `/opt/ardupilot`): `set_options` (`UARTDriver.cpp:1562`) reads `CR2`, sets or
clears RXINV/TXINV/SWAP, and writes `CR2` (with `UE` cleared around it) only when the word changes; every
`begin` starts `sercfg.cr2 = _cr2_options` (`:436`), which ChibiOS's `usart_init` writes. AP_RCProtocol's
search applies the next of its configs once a second while nothing is detected (`AP_RCProtocol.cpp:388-395`):
115200 8N1, 115200 8N1 RX-inverted, 100000 8E2 RX-inverted (SBUS), 200000 8E2 inverted (FastSBUS, if
compiled), 416666 8N1 (CRSF), 2000000 8N1.

| # | Prediction | Why |
|---|---|---|
| P1 | Every declared port's USART takes at least one logged `CR2` write; SERIAL1–3 several (each `begin`). | `begin` writes `CR2` through `usart_init`; GPS autobaud calls `begin` at each baud. |
| P2 | Every port's first `begin` runs inside 5 s, the console's and the GPS ports' before 1 s. | The serial manager starts in `init_ardupilot`; U4's identity is at 0.166 s. |
| P3 | The RC port's `CR2` changes more than once: RXINV set and cleared, `STOP` 2 then 1, about a second apart, from the RC thread's first search. | The search above. The baseline's `USART3.CR1` reads PCE 0 at 5 s, so the cut is in an 8N1 config. |
| P4 | No write on any port sets SWAP or TXINV; RXINV appears only on USART3. | No option is declared and `DEFAULT_SERIALn_OPTIONS` is 0. |
| P5 | The hook adds under 100 lines and under 16 KiB, against the 8 MiB budget (0.2 %). | A few writes per port, about 80 bytes a line. |
| P6 | `PUPDR` at 5 s: SERIAL1–3 TX and RX pulled up (`01`); SERIAL2's RX pulled down (`10`) if the cut is in an inverted config, else up; SERIAL0's floating (`00`). | `set_options` pulls each non-inverted line up (`:1611`, `:1624`); SERIAL0 has no options row and is predicted not to call it. |
| P7 | No byte or half-word `CR2` write: every write is a word. | ChibiOS writes `u->CR2 = …` as a 32-bit store. |
| P8 | No trace field is needed: a foreign run is never archived (campaign runs are generated, `campaign.rs:1218`); the findings ride the verdict's `boot` and `findings`. | |
| P9 | No `fab/` byte moves: the bench board is generated, and nothing here touches eda, IR or generators. | |
