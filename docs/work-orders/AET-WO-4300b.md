# AET-WO-4300b — Serial links: UARTs solved, generated, witnessed by their bytes

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300b-serial`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, the third piece of `AET-WO-4300`'s split (PR #123). **Cadence: build, with one measured stop (§1.6).**
- **It starts after `4300a` merges,** because both change the specification, the solver and the board crate's pin configuration, and it reads `4300a`'s solved clock fact.
- **`4400b` waits for it** (`4400` D2: ArduPilot cannot build without a USART).

**Grounds, all from the `4300` spike (`AET-WO-4300.FEEDBACK.md` S2, measured), on `main`:**
- **USARTs are `UART.STM32F7_USART`.** A byte enters the attached backend only through `TransmitCharacter`, from a `TDR` write with `UE` and `TE` set, so captured bytes are a witness that a store cannot make (P2.5). The model logs nothing per byte, and no baud time passes.
- **Framing does not reach the bytes.**
  - `BRR`, `PCE`, `PS` and `STOP` are stored and can be judged as registers.
  - `M0` (the 9-bit word), `TXINV` and `RXINV` are tags: visible only as the unhandled-write warning.
  - SBUS (100 kbaud, 8E2, inverted) can be judged only through those warnings; CRSF (420 kbaud, 8N1) through registers.
- **Injected bytes** (`sysbus.usartN WriteChar` between two `RunFor`s, after `RE`) come back through a firmware echo (P2.2).
- **An ungated USART is dropped** with a `locked peripheral` warning per access (P2.4).
- **Enable bits:** USART1/6 are in `APB2ENR`; USART2/3 and UART4/5/7/8 are in `APB1LENR`. Both registers are already cold-written. LPUART1 is in `APB4ENR`, which is not.
- **The clock:** `4300a` declares the bring-up firmware's clock as a solved fact (reset HSI, 64 MHz, prescalers 1). A USART's kernel clock at reset follows from it (RM0433, `D2CCIP2R` reset value; cite it).

**Ruled (`4300`, PR #123):** bytes are witnessed, framing is static, and there is no framing fault.

> **What this is:** a specification declares serial ports; the solver assigns USARTs and pins; the board crate and generated firmware configure them; the twin witnesses a pattern out and an injected frame echoed on each port; framing is judged statically; and one fault (`serial_rx_dead`) renders and attributes. The `SolvedTopology` then carries everything `4400b` needs for `SERIAL_ORDER` and the UART pins.
>
> It is NOT:
> - LPUART1 (refused by name; its enable bit is outside the cold-written registers);
> - DMA;
> - half-duplex or single-wire modes;
> - flow control;
> - USB (`4300e1`/`e2`);
> - ArduPilot (`4400b`).

## 1. Build

1. **The specification gains `serial_ports`.** Every key is required (`INV-06`), and the specification takes the next free `schema_version`, once. Each port has:
   - **a label;**
   - **a peripheral** (`usart1`…`usart3`, `uart4`, `uart5`, `usart6`, `uart7`, `uart8`), or `null` for the solver to choose;
   - **its TX and RX pins,** each, or `null` for the solver to choose;
   - **a role:**
     - `console`;
     - `mavlink`;
     - `gps`;
     - `rc_crsf`;
     - `rc_sbus`;
   - **its framing:**
     - baud;
     - data bits;
     - parity;
     - stop bits;
     - `inverted` (true or false).

   The framing is declared, never inferred from the role. Where a role has a published framing, `validate` refuses a declaration that contradicts it, by name, with the citation:
   - **CRSF:** TBS's published CRSF specification. Fetch it, hold it by sha256, and cite the baud and frame format.
   - **SBUS:** if no vendor-published source exists, say so. The declaration then stands as the specification's own statement, and nothing is refused on SBUS's behalf.
2. **The solver assigns peripherals and pins** from the pin and AF data it already reads:
   - pins the specification fixes are honoured or refused by name;
   - two ports on one peripheral are refused;
   - a port on a pin another function holds is refused.

   The `SolvedTopology` carries, for each port:
   - the peripheral, both pins and both AFs;
   - the role and the framing;
   - the solved `BRR` from `4300a`'s clock fact, with the actual baud and its error.
3. **The static check:**
   - **The baud error** against the receiver's tolerance in RM0433 (the USART receiver-tolerance table for the configuration used: oversampling and the fractional `BRR`). Cite the table and the row, and refuse by name above it. The table is data, as `4300c`'s Fmax table is.
   - **The registers:** `PCE`, `PS`, `STOP` and `M` as the framing needs them.
   - **`TXINV`/`RXINV`** set exactly when `inverted` is true.
4. **The board crate and generated firmware:**
   - a USART typestate, written in full;
   - the enable bit set (a Kani harness proves the writes match the solved values);
   - at start, each port sends a fixed pattern that holds its label and the bytes `00 FE 7F 80 FF`, then echoes what it receives, polled.
   - The generator emits each USART pin at `00` (slowest), unless `4300c`'s check says otherwise. A UART line's rate is its declared baud; it belongs to `4300c`'s scope as a peripheral-driven line (`4300c` D4).
5. **The twin and the judge:**
   - **The capture:** one byte capture per port, taking the cheapest witness the spike's P2 runs proved: a file backend or an analyzer. Measure which one fits the CLI's composition, and say why.
   - **The pattern out:** exact, in order.
   - **The echo:** inject a frame after the pattern; the frame echoed back exactly.
   - **The framing, statically:** register read-back, plus the tag warnings for `M0`/`TXINV`/`RXINV`. Each warning is matched to its port and its bit.
   - **The records:** kinds for the serial witness and the static framing judgement, taking the next trace version once. Replay joins each record to its solved port.
6. **`serial_rx_dead`:** one port's RX pin on the wrong AF. The pattern is still out, but the echo is missing.
   - Seed the case, and attribute it electrical or software as the evidence allows (state which, and why).
   - Write its trace records, replay and rule.
7. **Fixtures:**
   - **Sentinel-01 gains one port:** `console`, 115200 8N1, not inverted, with the peripheral and pins left to the solver. This is the USART `4400b` needs. Its committed generated files change by this port and its named consequences only.
   - **A four-port fixture:** `mavlink` 57600, `gps` 115200, `rc_crsf` 420000, `rc_sbus` 100000 8E2 inverted. It passes every gate in the image.

**§1.6 Measure first, and STOP if it fails: the board.** A serial port's lines leave the board, and no off-board connector part exists (`aethelgard-kicad-lib` vendors none). Before building the board side, measure what `emit` produces for a net with one MCU pin and no other part, and what ERC and DRC say about it.
- **If the gates pass it honestly** (no waived check, no dangling label), proceed. Name the gap in the verdict: "off-board, no connector".
- **Otherwise, STOP with the messages.** Connectors are `AET-WO-4300f`'s job, and `4300a` meets the same question.

## 2. Acceptance

- **Witnessed in the twin:** every port of both fixtures, with the pattern out and the injected frame echoed, plus static framing.
- **Forgeries fail:**
  - a store to `TDR` with `UE` clear;
  - a capture filled from the host;
  - a pattern sent on the wrong port.
- **`serial_rx_dead`** attributes as its manifest says.
- **The reference's verdicts are unchanged**, apart from the new port and its named consequences. Measure this against `main`'s binaries.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, refitting by `ci/shard.sh`'s rule if needed, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01` to `INV-07`; `RUL-12`, `RUL-14`, `RUL-16`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir` (the specification, the solver, the framing and tolerance data);
  - `aethelgard-svd-hal` (USART);
  - `aethelgard-twin` (the capture);
  - the CLI (judge, campaign);
  - `aethelgard-chaos` (records, rules);
  - `aethelgard-evolve` (the rule);
  - the seeded set, fixtures and tests;
  - `examples/`, `boards/sentinel-01/**` (regenerated);
  - `ci/`;
  - FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-4300b/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 5. Adversarial pass, full

1. **Forge a port:**
   - bytes from a store or from the host;
   - an echo the firmware never received;
   - a pattern on a port the topology does not name.
2. **Make a correct port fail:**
   - a baud at the tolerance's edge;
   - a fractional `BRR` rounding the wrong way;
   - SBUS's tags read as faults.
3. **Break the solver:**
   - two ports on one USART;
   - TX and RX on different peripherals;
   - a port on an I2C or SPI pin;
   - LPUART1.
4. **Make `serial_rx_dead` attribute wrongly.**

## 7. Flag-and-stop conditions

- §1.6: the board gates cannot pass an off-board net honestly.
- `4300a`'s clock fact is absent or differs from what §1.3 assumes.
