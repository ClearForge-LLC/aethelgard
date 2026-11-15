# AET-WO-4300b FEEDBACK — serial links

**Builder:** Claude (strong tier), worktree `~/aethelgard-4300b`, branch `wo/aet-4300b-serial` from `main` `7ca65e6`.
**Work order:** `docs/work-orders/AET-WO-4300b.md` on `main`, as amended by the architect's kickoff amendments (`~/.cache/aet-4300b-ruling.md`, 2026-11-15 12:00 UTC).
**Cadence:** build, with one measured stop (§1.6, now answered by connectors unless ERC or DRC fails).

The predictions in §1 are committed before any container run, any firmware build for a probe, and any baseline run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source. Renode 1.17.0 at renode `ab721d88` / Infrastructure `066a7f13` (`~/.cache/aet-2301/src`); RM0433 Rev 9 (`672a9433…b6a0`, text in `~/.cache/aet-2301/ds/txt/rm9.txt`); TBS's CRSF specification (below); this repository at `7ca65e6`.
- **[measured]**: run in the pinned image (`container/IMAGE`) under rootless podman through `~/bin/capped-run` (RULE3), caps read back.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the plan

Each item is **[read]**.

- **TBS's CRSF specification, held.** `github.com/tbs-fpv/tbs-crsf-spec`, `crsf.md` at commit `a73f7dc5386e5b8744aa4a23b3af5354ee0d4c92` (2026-10-21), fetched from `raw.githubusercontent.com` on 2026-11-15 with no refusal, held as `~/.cache/aet-4300b/ds/crsf-a73f7dc.md`, sha256 `58293cefb558425729b749f5d9700ba51a83cb578e9867dd1f94224303b6f918` (`PROVENANCE.md` beside it).
  - "Hardware", *Dual-Wire Full-Duplex UART* (L137–139), the configuration on a flight controller: "Only non-inverted (regular) UART is supported in this configuration. The UART runs by default at **416666 baud 8N1** … but baudrate can be negotiated to be higher".
  - *Single-Wire Half-Duplex UART* (L131–135): 400 kbaud 8N1, inverted or not, also 115.2 kbaud. A port here has a TX and an RX pin, so it is the dual-wire configuration.
  - Frame `0x32.0x0A` sub-command `0x70`, "CRSF Protocol Speed Proposal" (`uint32 proposed_baudrate`): the negotiation the dual-wire text refers to.
  - **The work order's fixture declares `rc_crsf` at 420000.** The text makes 8N1 and non-inverted unambiguous; it makes 416666 the default and admits higher rates by negotiation. So `validate` refuses a CRSF port that is not 8 data bits, no parity, 1 stop bit, or that is inverted, and one whose baud is below 416666; 420000 is a higher rate and validates. Whether 420000 should be read as "negotiated higher" or as a contradiction of the default is a question for the architect (§6 Q1), not a silent choice.
- **SBUS.** No vendor-published SBUS specification exists that I found: Futaba publishes none, and the commonly cited 100 kbaud, 8E2, inverted framing comes from reverse-engineering write-ups. So the declaration stands as the specification's own statement, and `validate` refuses nothing on SBUS's behalf (as the work order says).
- **RM0433 Rev 9, the USART kernel clock at reset.** `RCC_D2CCIP2R` (§8.7.21, offset `0x054`, reset `0x0000 0000`): `USART16SEL[2:0]` (bits 5:3) `000` selects `rcc_pclk2` for USART1/6; `USART234578SEL[2:0]` (bits 2:0) `000` selects `rcc_pclk1` for USART2/3, UART4/5/7/8. With `4300a`'s solved clock (HSI 64 MHz, `D1CPRE`, `HPRE`, `D2PPRE1`, `D2PPRE2` all "not divided"), `rcc_pclk1` = `rcc_pclk2` = 64 MHz. `USART_PRESC` resets to 0 (÷1).
- **RM0433 Rev 9, the receiver's tolerance** (§48.5.8, p. 2035–2037): Table 406 (`BRR[3:0]` = `0000`) and Table 407 (`BRR[3:0]` ≠ `0000`), each by `M` bits (`00` 8-bit word, `01` 9-bit, `10` 7-bit, where the word includes parity), `OVER8` and `ONEBIT`. The generated firmware leaves `OVER8` = 0 and `ONEBIT` = 0, so the columns are "OVER8 = 0, ONEBIT = 0": Table 406 `00` 3.75 %, `01` 3.41 %, `10` 4.16 %; Table 407 `00` 3.33 %, `01` 3.03 %, `10` 3.7 %. The RM's condition is "DTRA + DQUANT + DREC + DTCL + DWU < USART receiver tolerance"; the work order judges the solved baud's error alone against it. The check refuses an error equal to or above the row (the RM's strict "less than"), which is the work order's "above it" read conservatively.
- **The solved `BRR`s at 64 MHz** (OVER8 = 0, `BRR = round(64 000 000 / baud)`, half up): 115200 → 556 (`0x22C`, 115 107.9 baud, −0.080 %, Table 407); 57600 → 1111 (`0x457`, +0.010 %, Table 407); 420000 → 152 (`0x98`, +0.251 %, Table 407); 100000 → 640 (`0x280`, exact, Table 406, the only row on Table 406). Each is far inside its row.
- **Renode's USART** (`UART/STM32F7_USART.cs`):
  - stored: `CR1.UE`, `RE`, `TE`, `RXNEIE`, `TCIE`, `TXEIE`, `PEIE`, `PS`, `PCE`, `OVER8`, `RTOIE`; `CR2.STOP`; `CR3.DMAR`, `DMAT`; `BRR` (16 bits);
  - tags (stored nowhere, read 0): `CR1.M0` (named `MO`), `CR1.M1`, `CR2.RXINV`, `TXINV`, `SWAP`, `DATAINV`, `MSBFIRST`, `CR3.ONEBIT`, `HDSEL`, `RTSE`, `CTSE`, among others;
  - `USART_PRESC` (offset `0x2C`) is not defined;
  - a `TDR` write reaches the attached backend only through `TransmitCharacter`, when `TE` and `UE` are set; otherwise it logs "Char was to be sent, but the transmitter (or the whole USART) is not enabled. Ignoring.";
  - `TXE` always reads 1, so a polled transmit never waits.
- **Tag warnings** (`PeripheralRegister.cs` L593, L682–760): a write logs `Unhandled write to offset 0x…. Unhandled bits: […] when writing value 0x…. Tags: NAME (0xV).` for each tag whose written bits differ from the stored value. Tags are never stored, so **every write that sets a tag bit logs it, with the whole value written**, and a write that leaves it 0 logs nothing.
- **Injection** (`UARTBase.cs` L80–92): `WriteChar` enqueues into an unbounded queue when `RE` and `UE` are set, and otherwise drops the byte with a `Debug` line. Silicon holds one byte in `RDR` and sets `ORE`; the twin cannot render an overrun.
- **No pin on Renode's USART path.** `stm32h7.repl` declares each USART at its address with `rcc` registration and an `IRQ`, and no GPIO route. `WriteChar` and the file backend hang off the USART object. So a USART's RX pin on the wrong AF still receives an injected byte in Renode (P5 below predicts it).
- **The file backend** (`UartFileBackend.cs`): `CreateFileBackend path true` opens the file `FileMode.CreateNew` through `SequencedFilePath`, which first moves an existing file at that path to `path.N` and logs "Old file … moved to …" at `Info`. Bytes go in as written, flushed per byte with `true`. No stamps.
- **Renode's RCC defines `RCC_D2CCIP2R`** (`STM32H7_RCC.cs` L419–431, `0x54`) with every field a tag, so it reads 0.
- **`aethelgard-twin/src/tests.rs:119` asserts USART1 unsupported**; it is replaced by another unsupported peripheral, not deleted.
- **DS-009** (`4300f` §3.1, its `ds009.rs`): the Telemetry port is "(any serial port)": VCC +5V, TX (out), RX (in), CTS (in), RTS (out), GND; Basic GPS: VCC +5V, TX, RX, I2C1 SCL, I2C1 SDA, GND. **DS-009 rev 0.1.0 names no RC port.** The debug ports carry a console TX/RX but also SWDIO/SWCLK, which no net of this schema can carry.

## 1. Predictions (committed before any run)

Probes run through the CLI's composition with the work order's generated firmware, or a copy of the generated board patched by a script (`4300a`'s `mutate.py` way), built offline in the pinned image. The harness lives in `~/.cache/aet-4300b/` and is not committed.

### P1. The reset clock and the kernel clock (§1.3, a §7 stop)

| id | probe | prediction |
|---|---|---|
| P1.1 | The generated firmware's clock check, extended with `RCC_D2CCIP2R.USART16SEL` and `USART234578SEL`, on Sentinel-01 + console | Passes: the four `4300a` registers as `4300a` measured, `D2CCIP2R` reads 0 (its fields are Renode tags). **The §7 stop does not fire.** |
| P1.2 | The read of `D2CCIP2R` logs anything | Nothing: a read of a tag is not logged as an unhandled write. |

### P2. Bytes out (§1.5)

| id | probe | prediction |
|---|---|---|
| P2.1 | Sentinel-01 + console, file backend per USART, 100 ms | The capture holds the console's pattern exactly (its label then `00 FE 7F 80 FF`), then the injected frame. Nothing per byte is logged. |
| P2.2 | The four-port fixture, 100 ms | Each of the four captures holds its own port's pattern, then its own frame, and nothing else. |
| P2.3 | A `TDR` store with `UE` clear (a patched board) | Nothing captured on that port; one "Char was to be sent … Ignoring." warning per byte; the judge fails the pattern. |

### P3. Injection timing

| id | probe | prediction |
|---|---|---|
| P3.1 | Inject one quantum (100 µs) after the last release | Every port's `RE` and `UE` are set by then: the generated start reaches the USART configuration within a few thousand instructions. The frame is echoed. |
| P3.2 | Inject at 0 (before the firmware runs) | The frame is dropped (`Debug` line only); the echo is missing; the judge fails closed. |

### P4. Static framing in the twin

| id | probe | prediction |
|---|---|---|
| P4.1 | SBUS port (100 kbaud 8E2 inverted) | `CR1` reads back with `PCE`, `TE`, `RE`, `UE` and without `M0`; `CR2` reads `STOP` = `10` and no `TXINV`/`RXINV`; `BRR` 640. The log holds one `Tags: MO (0x1)` warning on `CR1` and one `RXINV (0x1), TXINV (0x1)` warning on `CR2`, each from that USART, one per write that carries them. |
| P4.2 | CRSF, MAVLink, GPS, console (8N1, not inverted) | No tag warning from those USARTs. |

### P5. `serial_rx_dead`

| id | probe | prediction |
|---|---|---|
| P5.1 | The console's RX pin left on its reset mode (analog) or on another AF | **Renode still echoes the frame**: no pin is on its USART path (§0). So the twin cannot render the fault by itself; the judge must model the pin mux: the RX pin's `MODER` and `AFR` read at the injection instant decide whether the frame reached the receiver. With that model, the pattern is witnessed and the echo is not; the register image names the pin's `AFR` (or `MODER`). Attribution: **software** (the AF is the firmware's own write, and the twin models nothing electrical on the line), as `motor_output_dead` was. |

### P6. Forgeries (§2)

| id | probe | prediction |
|---|---|---|
| P6.1 | A file at the capture path before the run | Renode moves it to `….1` and starts a fresh file; the judge refuses a capture directory holding anything but its own files. |
| P6.2 | A firmware that sends the pattern and a constant frame without reading `RDR` | Fails: the frame is derived from the firmware's own sha256 and the port's label, which the firmware cannot know when it is built. |
| P6.3 | A pattern on the wrong port | Fails: each capture must hold its own port's label. |

### P7. The board side (§1.6, amended)

| id | probe | prediction |
|---|---|---|
| P7.1 | `emit`, `check erc`, `check drc --spec --schematic` on Sentinel-01 + console (an explicit 4-pin JST-GH map) and on the four-port fixture (DS-009 TELEM for MAVLink and CRSF, DS-009 GPS for the GPS, an explicit map for SBUS) | Exit 0, pass, **no findings** on each; no `no_connect` on a used MCU pin. **The §1.6 STOP does not fire.** |

### P8. Kani

| id | probe | prediction |
|---|---|---|
| P8.1 | A targeted USART harness (`usartN_start_writes_the_solved_values`) | Verified in seconds. |
| P8.2 | `every_valid_sequence` on the regenerated Sentinel-01 | Finishes, slower than the reference's 121 s by the USART's writes, under 4 min. **If it does not finish, that is a decision for the architect** (as `4300a` Q5), not a skip. |

### P9. The reference's verdicts against `main`'s binaries

- Every committed scenario keeps its verdict and finding kinds. Each differs by: the specification's and topology's hashes and versions (7 and 6), the trace version (9), the console's registers in the image (its USART's `CR1`, `CR2`, `CR3`, `BRR`; its pins' port registers already judged), the console's records (solved port, static framing, witness), and the board's layout around the console's connector. The seeded cases' evidence indices move by the records added before them; each case keeps its domain and rule.
- A droop scenario on Sentinel-01 re-sends the pattern after the reset, and the frame is injected after the last release, so the capture is the pattern twice then the frame.

## 2. Plan decisions (before the build)

- **Versions:** specification 7, topology 6, trace 9, each once, as single constants; whoever of `4210` and this merges second renumbers.
- **The echo is bounded and the core still parks.** The firmware sends each pattern at start, runs its identity reads, writes the boot word, then polls each port for its frame's length a bounded number of times, echoing each byte, and parks on the `yield; b.n` loop. A port that receives nothing ends its wait and the core still parks, so a dead port is named by its echo alone.
- **The frame** is eight bytes per port, the first eight of `sha256("aethelgard-serial-frame" ‖ firmware sha256 ‖ label)`: replay re-derives it from the header's `firmware` artifact; a firmware cannot hold it at build time.
- **The capture** is one Renode file backend per port in a directory the CLI creates for the boot; the judge reads every file whole and refuses an unexpected file.
