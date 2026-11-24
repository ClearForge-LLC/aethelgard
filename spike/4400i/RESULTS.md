# AET-WO-4400i (1): results, and the stop

**STOP: §1.1, the measurement** (2026-11-24). Every answer below is from a run whose command and record are in
this directory, except where a line says it was read.

**The short answers:**
- **The arm does not fit the 5 s window.**
  - Sentinel-03's unmodified ArduPilot arms at 8.602 s virtual and witnesses it in its own heartbeat at 8.754 s.
  - The binding step is EKF3's tilt alignment (8.564 s). Nothing these plans sent moved it: in the proposed delivery
    mode, alignment comes at 8.555 s, ARM is accepted at 8.608 s and the armed heartbeat at 8.753 s (`early-fill`).
  - There is no force, no skipped check and no defaults file. The rest stimuli are derived from U5's declared
    orientation and the parts table, each cited.
- **The receive request lines.**
  - RM0433 Rev 9 Table 124 gives one request per USART. `4400h`'s "Table 121" was a mis-cite.
  - Emitted on every board, they move one committed verdict, the bench package's: 16 hashes, and no outcome (§2.3).
  - Emitted on foreign boards only, they move none.
- **IDLE.** Padding at one instant is not a sound stand-in: a padded frame over 64 B delivered at one instant kills
  the receive path for the rest of the run.
  - **Hooks that raise IDLE are not enough.** They meet two more model gaps:
    - Renode's DMA does not end a stream with TCIF when software clears `EN`, which RM0433 §15.3.16 says it must;
    - it keeps its write offset across the re-arm, so the next frame lands mid-buffer (measured).
  - **Proposed: bytes spaced a character time apart, fill declared in the input file, and the gap named in the
    verdict.**
    - Measured on the request plan (`idle-fill`): every delivered frame was answered at the padded latencies, the
      98 B frame included.
    - Measured on the window plan (`early-fill`): it arms.
  - The models become a row.
- **The input and the trace:** proposed in §6.

## Inputs

| | |
|---|---|
| Board | Sentinel-03, as `4400h` ruled: main's fixture plus `mcu.foreign_clock`, U5's orientation (x right, z up) and `board.forward` (`spike/4400h/py/derive.py`). Spec sha256 `135c96fa…` |
| ELF | Copter-4.7.2 built unmodified for it (`spike/4400h/sh/s03.sh`), sha256 `dbf598d8…`. Same bytes as `4400h`'s |
| CLI | main `fa12abf4`, built in the tools image `1e0ab105…` |
| Capture | `sh/capture.sh s03 5000ms`: the CLI's own Renode invocation (`out/capture-s03-argv.bin`); its verdict passes (`out/capture-s03-verdict.json`) |
| Replays | `sh/inject.sh NAME PLAN` replays the capture with a plan's frames (`py/plans.py`, `py/frames.py`) |
| Containers | Every container under `~/bin/capped-run` (4 GiB, 16 GiB for `verify`; swap 0, read back), one at a time, `nice -n 10` (`sh/img.sh`) |
| Witness | ArduPilot's own transmitted bytes. A write hook on USART1's `TDR` stamps each byte with its virtual time (`sh/replay.sh`), checked byte for byte against the port's file backend in every run (`same_bytes`). `py/timeline.py` decodes them as MAVLink 2 into `out/inj/<run>/mav.tsv` |

Two kit facts the build inherits:
- **A write hook on a peripheral displaces `LogPeripheralAccess`'s write lines for it.** With the `TDR` hook, USART1's
  access log kept 16 lines (reads only), against `4400h`'s 2,886. The foreign serial judge reads those lines, so any
  hook on a declared port must log what it displaces.
- **The capture's first process is the CLI's reaper, not Renode** (`cap/s03/1`): replays use `cap/s03/2`.

## 1. Serial injection today

- **The generated path** (read, `crates/aethelgard-twin/src/serial.rs`:88–154, `crates/aethelgard-cli/src/anchor.rs`:384–537):
  - It sends one 8-byte frame per port: the first 8 bytes of `sha256(domain ‖ firmware ‖ label)`.
  - Each byte is a `WriteChar` from a `ScheduleAction`, one character time rounded up to a 100 µs quantum apart.
  - It starts a quantum after the boot word, and each action is guarded by an instruction count.
  - **Its record:** `Record::SerialWitness`, trace kind 29 (`aethelgard-chaos` `record.rs`:794–803, :1336), with
    `frame` and, since trace 13, `injected_ns`. The trace is at version 19 (`event.rs`).
  - **Replay:** `replay` re-reads the archive and re-judges the records. It never re-runs Renode. It recomputes the
    frame from the firmware hash and the label (`campaign.rs`:3695–3703).
- **The foreign path injects nothing and writes no trace** (`foreign.rs`:6–9; `main.rs`:1834–1837).
- **What a foreign firmware lacks, measured on Sentinel-03:**
  - **The receive request line.** `noreq` sent the window plan's first 4 s with no line: 0 replies, against 6
    heartbeats out. With the line, every short frame is answered (§3).
  - **IDLE** (§3).
  - **Rest stimuli** (§4).

## 2. The receive request lines

### 2.1 The table

RM0433 Rev 9 (sha256 `672a9433…`, the copy `dmamux1-spi-rx.tsv` cites) Table 124, p. 697, DMAMUX1 inputs:

| USART | request |
|---|---|
| USART1 | 41 |
| USART2 | 43 |
| USART3 | 45 |
| UART4 | 63 |
| UART5 | 65 |
| USART6 | 71 |
| UART7 | 79 |
| UART8 | 81 |

LPUART1 has no DMAMUX1 input. `4400h`'s patch said "Table 121"; in Rev 9 it is Table 124. Renode raises
`ReceiveDmaRequest` only on an empty-to-ready transition of its receive queue, and only while `CR3.DMAR` is set
(`STM32F7_USART.cs`:100–111).

### 2.2 Where the line would go

- The emitter writes every USART only as a clock entry (`platform.rs` `emit_clock`, all eight, whatever the
  specification declares).
- The measurement patch (`patches/rx-request-all-declared.diff`) adds `ReceiveDmaRequest -> dmamux1@N` under each
  declared port's entry, on every board.

### 2.3 Whether it moves a committed verdict

`sh/req.sh` ran on main's build and on the patch's, in the tools image (`out/req/`; every differing field in
`out/req/verify-diff.tsv`).

- **Platforms** (`out/req/*/repl.tsv`): the six examples and the five `sentinel-0*` fixtures (5 of the 32 fixtures;
  the others were not emitted).
  - **Seven move:**
    - `sentinel-01-ext-bmp581`, `sentinel-01-minimal`, `sentinel-01` (the two `boards/` goldens' specs);
    - fixtures `sentinel-01-bench`, `sentinel-02-own`, `sentinel-02-shaped` and `sentinel-03`;
    - also the derived Sentinel-03.
  - **Two keep their hash:** `sentinel-02-bench`, `sentinel-02-bench-hse8`.
  - **Two emit nothing on either build:** `invalid-zero-domain`, and `sentinel-02-own-base`, a schema-1 base that
    `generate` refuses.
- **Foreign verdicts** (`simulate --foreign 5000ms`, `sentinel-02-own` and Sentinel-03): both pass on both builds.
  - They differ only in `/input/path`, `/input/sha256` (the platform) and the run's own memory reading.
  - Every judge, device step, register and finding is identical.
- **The bench package** (`ci.yml`'s own `verify … --against-fab fab/sentinel-01-bench/package`):
  - On Furnace, main's own run already differs from the committed package, in `MANIFEST.json` alone, so in its
    verdict line. Which field differs from CI's was not read; the ELF and every other file are identical. So the
    comparison is main against the patch, both on Furnace.
  - **The patch moves 16 fields, every one a hash:**
    - the `board/sentinel-01-bench.repl` file entry;
    - the `input_sha256` of the 12 `prove` gates and the `build` gate, which hash the board crate's tree, `.repl`
      included;
    - the `simulate` gate's `input_sha256`;
    - the manifest's own hash.
  - **No gate verdict, campaign result or finding moves.**
  - But the committed `fab/sentinel-01-bench/package/MANIFEST.json` names that verdict's hash. CI's `bench-package`
    job would fail until the package is derived again, and `fab/` is outside this fence.
- **So:** on foreign boards only, no committed verdict moves. On every board, the bench package's verdict moves
  (hashes only), and so do the two `boards/` goldens.
- **Not run against the patch:** the workspace's tests that pin emitted platform text (the regeneration test, the
  twin's platform tests). They are the build's, in its fence.

## 3. IDLE

### 3.1 The runs

Eleven requests from 3.0 s, 200 ms apart, each answered by name: ten `PARAM_REQUEST_READ`s and one
`FILE_TRANSFER_PROTOCOL` frame of 98 B. The last is followed by silence. Latencies are in `out/idle-latency.tsv`
(`py/latency.py`).

| run | frames | result |
|---|---|---|
| `idle-pad` | padded with zeros to a multiple of 64 B, every byte at one instant (`4400h`'s stand-in) | the first five answered in 55–66 ms. **At the 98 B frame (128 B padded) the receive path dies:** `dma1: Attempting to create DMA request with no data to send` at 4.000 s, and nothing after it is answered |
| `idle-raw` | unpadded, one character time apart (115200 Bd 8N1: 86.8 µs, rounded up to the 100 µs quantum), no IDLE | answered in bursts, as each 64 B fills: 214–459 ms late. The 98 B frame is answered. **The last three are never answered:** of their 87 B, 26 went out at the 320 B boundary, and 61 sit in a buffer 3 bytes short of 64 (the stuck tail) |
| `idle-hook` | as `idle-raw`, plus hooks that raise IDLE a character time after a frame's last byte (`sh/replay.sh` `IDLE=1`) | IDLE fires 11 times. The first frame is answered in 56.5 ms, as padded; the second 864 ms late; **none after** |
| **`idle-fill`** | **padded with fill to a multiple of 64 B and spaced a character time apart: the mode proposed** | **every delivered frame answered at the padded latencies (55–69 ms), the 98 B frame (128 B filled) in 25 ms; the receive path stays alive.** Two requests got no reply: `ARMING_CHECK`, an unknown name, which drew `PARAM_ERROR` in `idle-raw` but nothing here; and the one at 5.0 s, during `Initialising ArduPilot`, as in `idle-pad`. No DMA error after boot |

### 3.2 Why

- **`idle-pad`'s death is the request line's edge.**
  - 128 B queued at once: the stream takes 64, ends at `NDTR` 0, and ArduPilot re-arms it.
  - The other 64 stay queued, so the queue never passes through empty, and the line never rises again
    (`STM32F7_USART.cs`:100–111).
  - Any frame over 64 B, or two frames sent at one instant, does the same. MAVLink 2 frames reach 280 B.
  - Spaced bytes avoid it: each byte is its own empty-to-ready transition (`idle-raw` delivered the 98 B frame).
- **`idle-raw`'s stuck tail is ArduPilot's buffer.**
  - With RX DMA, ArduPilot hands bytes to its parser on the stream's transfer-complete (64 B, `UARTDriver.cpp`:606–642)
    or on IDLE (`rx_irq_cb`, :576–600).
  - Its 1 kHz fallback drains only a stream that is already disabled (:1138–1168).
  - SERIAL0 takes no `RX_NoDMA` option (`AP_SerialManager.cpp`:164–180, :451–457).
- **`idle-hook`: IDLE alone is not enough.** Two model gaps, both measured in `idle-hook-dma`
  (`out/inj/idle-hook-dma/dma1-frames-1-3.txt`):
  1. **`IDLEIE` is a tag** (`STM32F7_USART.cs`:154). ArduPilot's read-modify-write of `CR1` (to clear `RXNEIE`) reads
     it back as 0 and clears it. The hooks had to store it, as silicon does; until they did, IDLE never fired.
  2. **Renode's DMA ends no stream when software clears `EN`.**
     - `STM32DMA.HandleEnable` acts only on a set `EN`, and TCIF is set only at `NDTR` 0 (`STM32DMA.cs`:237–262).
     - RM0433 Rev 9 §15.3.16, p. 654: "The transfer complete interrupt flag (TCIF …) is set in order to indicate the
       end of transfer … The DMA_SxNDTR register contains the number of remaining data items".
     - So after `rx_irq_cb` clears `EN`, ArduPilot's transfer-complete handler never runs. Its 1 kHz fallback drains
       instead, 0.15–0.77 ms later. The counts are right: `NDTR` 0x25, 0x23, 0x25, …, and 64 + 34 for the 98 B frame.
  3. **Renode's DMA keeps its write offset across a software stop.**
     - `dataOffset` is zeroed only by `Reset()` and at `NDTR` 0 (`STM32DMA.cs`:158–161, :256–262). Writing `M0AR`,
        `NDTR` or `EN` leaves it.
     - So after frame 1 (27 B) and the re-arm, frame 2 was written at the new buffer's byte 27 (0x3800195B), not 0.
       ArduPilot read that buffer's first 29 bytes, which are zeros (`idle-hook-mem`, the bounce buffers dumped at
       3.1 s and 3.3 s: `out/inj/idle-hook-mem/bounce-buffers.txt`).
     - The offset keeps growing. Frames after it would be written past the 64 B buffer, into memory ArduPilot holds
       other data in (0x38001980 onward).
     - That is the stall: it is not ArduPilot's. On silicon, a stopped stream restarts at `M0AR`.
- **Is the padding sound?** Spaced (`idle-fill`), delivery is right and prompt for any length. At one instant, it is
  right only for a frame of at most 64 B sent alone. In both cases:
  - the zero bytes are input silicon would not see (ArduPilot's parser skips them; `BAD_DATA` is unchanged at 348,
    all of it ArduPilot's own console text);
  - ArduPilot's IDLE path never runs.

  **The 64 is ArduPilot's buffer size, a property of the firmware under test. If the judge chose it, the judge would
  be reading the artifact (`INV-07`).**

### 3.3 What an overlay costs

| overlay | measured | cost |
|---|---|---|
| hooks (USART only) | wall 13.80 s, peak 1,077,472 KiB, against `idle-pad`'s 14.20 s and 1,184,956 KiB | no cost in time. Not sound alone (§3.2). Displaces USART1's access-log writes |
| hooks (USART and DMA) | not possible | a hook cannot reset the DMA's private write offset (gap 3). TCIF on an `EN` clear alone would hand ArduPilot the wrong bytes |
| models of our own | not built | a USART model with IDLE and a stored `IDLEIE`, plus a DMA stream that ends with TCIF on an `EN` clear and restarts at `M0AR` on re-arm. Both written fresh (the models' README: no Renode source copied), pinned, and loaded on foreign boards only. Otherwise `usart_stored` and every generated serial judge's stored-bit list move (`serial.rs`:571–600). The DMA model is shared by every stream: SPI's receive DMA too |

## 4. The rest stimuli

- **The 1 g on y** is the parts table's ICM-42688-P witness stimulus (`parts.json`:528–530). The table has no
  `acceleration_z` channel, so z stays at the model's default, 0. For ArduPilot that is a board on its edge
  (`4400h`: "Leaning").
- **At rest,** DS-000347 rev 1.9 §4.7 (sha256 `9c6b57b0…`, the parts table's cited copy): "When the device is placed
  on a flat surface, it will measure 0g on the X- and Y-axes and +1g on the Z-axis".
  - A chip axis declared `up` reads +1 g, `down` −1 g, and any other 0. Y is Z × X, and the board frame is x
    forward, y right, z down (`orientation.rs`).
  - U5 (x right, z up) gives **X 0, Y 0, Z +1 g** (`py/plans.py` `rest()`, computed from the orientation alone).
  - On a board with no declared orientation there is no rest. That is a refusal, `INV-06`; Sentinel-03's fixture on
    main has none (§7).
- **The barometer at rest:**
  - 101325 Pa, the parts table's stimulus (`parts.json`:184), inside the 30–125 kPa operating range
    (BST-BMP581-DS004-13 rev 1.13 §2 Table 1, p. 8, sha256 `ddcf4202…`, the parts table's cited copy).
  - A constant reading is unhealthy to ArduPilot after 2 s (`AP_Baro_Backend.cpp`:36–41, 63–73).
  - **Proposed:** the intrinsic noise as ±0.08 Pa, Table 9's ×128 row (p. 19, "Typical pressure RMS noise at
    100kPa"), alternating at that row's typical 12 Hz (every 83.333 ms).
    - It is 5 counts of the 1/64 Pa resolution.
    - ArduPilot itself sets ×64 (`OSR_CONFIG` 0b01110010, `AP_Baro_BMP581.cpp`:130), whose row reads 0.11 Pa. It is
      noted, not used, so the stimulus does not depend on the firmware.
- **Measured (`early`):** with both rest stimuli from t = 0, neither `Baro: not healthy` nor `Leaning` appeared, and
  the board armed.

## 5. The window

`early`, padded frames each at its own instant. The four parameters and the accelerometer accept go every 250 ms
from 0.25 s to 6 s; prearm checks every 500 ms; ARM with no force every 250 ms from 3.1 s. The whole sequence is in
`out/inj/early/mav.tsv`.

| virtual s | ArduPilot, in its own words |
|---|---|
| 0.754 | first reply to a frame (`PARAM_VALUE COMPASS_USE` 0) |
| 1.003 | first heartbeat |
| 1.066 | `PreArm: System not initialised` |
| 5.214 | `ArduPilot Ready` |
| 5.352 | `Arm: EKF attitude is bad`, `AHRS: EKF3 not started` |
| 7.215 | `EKF3 IMU0 initialised` |
| 7.351 | `Arm: AHRS: EKF3 still initialising` |
| 8.564 | `EKF3 IMU0 tilt alignment complete` |
| **8.602** | **`COMMAND_ACK` ARM, result 0**: the first ARM after alignment |
| **8.754** | **`HEARTBEAT` `base_mode` 209 (`SAFETY_ARMED`)**, the next 1 Hz heartbeat |

- **The arm needs at least 8.76 s of virtual time;** a run that witnesses it needs a little more.
  - The binding step is EKF3: it starts 2.0 s after `ArduPilot Ready`, and its tilt alignment takes 1.35 s more.
  - Nothing these plans sent moved it. `early-fill`, the same plan spaced with fill, aligned at 8.555 s, armed at
    8.608 s and witnessed at 8.753 s.
  - The heartbeat comes up to 1 s after the ACK; here 0.152 s.
- **Against `4400h`'s bounds:**
  - 8.8 s lies between its 8 s run (97 % of the 8 MiB log budget, off-peak) and 9 s (over the budget).
  - Its 10 s runs passed 900‰ of the memory cap.
  - This replay's tree peaked at 3,141,860 KiB over 12 s at load 15. That is the replay's own measure, not the CLI's,
    and is compared only replay to replay.
- **So the acceptance, as the work order provides, is the replay outside the bounds with the gap named,** until
  `AET-WO-4400j` sets the window.

**The rejected parameter:**
- `reject` set `AET_NOT_A_PARAM`; ArduPilot answered `PARAM_ERROR` error 1 (`MAV_PARAM_ERROR_DOES_NOT_EXIST`) to
  255/190.
- It took `FRAME_CLASS` 99 and the read-only-looking `STAT_RUNTIME` 5, and echoed both. So a value out of a
  parameter's documented range is not refused by ArduPilot, and the only refusal ArduPilot names in its own reply is
  an unknown name.

## 6. Proposed: the input, the trace record, and the armed witness

### 6.1 The input file

`simulate --foreign TIME --input FILE`. The file belongs to the run, not the board, so no specification key is added.

```json
{
  "input_version": 1,
  "port": "CONSOLE",
  "frames": [
    {"at_ns": 750000000, "mavlink": "fd1c0000…", "fill": 0},
    {"at_ns": 800000000, "mavlink": "fd2100000…", "fill": 0}
  ]
}
```

- **`port`:** a declared serial port's label. Its USART carries the request line (§2), and it must receive.
- **`at_ns`:** the virtual instant of the frame's first byte.
  - A whole number of 100 µs quanta, so the trace never rounds one.
  - Strictly increasing, and inside the window.
- **Spacing:** each byte goes one character time after the last, by the port's solved framing (`serial::spacing_ns`,
  the generated path's rule).
  - No two bytes share an instant, so §3.2's dead receive path cannot arise.
  - A frame may not start before the previous one's last byte plus one idle character.
- **`mavlink`:** the hex of exactly one MAVLink 2 frame.
  - Checked: `0xFD`, the length, `incompat_flags` 0 (no signing), a message id in a closed table, and the X.25 CRC
    with that message's `CRC_EXTRA`.
  - The table is cited to `common.xml` sha256 `43b194d1…`, ArduPilot's own submodule copy, the same bytes in the
    pinned ArduPilot image. It starts with the messages measured here:

    | message | id | `CRC_EXTRA` |
    |---|---|---|
    | `HEARTBEAT` | 0 | 50 |
    | `PARAM_REQUEST_READ` | 20 | 214 |
    | `PARAM_REQUEST_LIST` | 21 | 159 |
    | `PARAM_SET` | 23 | 168 |
    | `COMMAND_INT` | 75 | 158 |
    | `COMMAND_LONG` | 76 | 152 |
    | `SET_ATTITUDE_TARGET` | 82 | 49 |
- **`fill`:** the number of `0x00` bytes sent after the frame. It is the declared stand-in for IDLE, if the input
  wants ArduPilot's 64 B buffer filled. The judge never adds fill; the verdict names every filled frame and the gap.

**Refused, each by name, with a test and a mutant** (every refusal is `INV-06`'s: no default is substituted):

| kind | refused input |
|---|---|
| `InputNotMavlink` | a frame that does not parse, or whose CRC fails; names the frame's index |
| `InputMessageUnlisted` | a message id outside the table, whose CRC cannot be checked |
| `InputSigned` | `incompat_flags` other than 0 |
| `InputForcesArm` | a `COMMAND_LONG` or `COMMAND_INT` with command 400 and param2 21196, MAVLink's force code (`common.xml`:1920) |
| `InputSkipsChecks` | a `PARAM_SET` of `ARMING_SKIPCHK` to anything but 0 (`AP_Arming.cpp`:195–201), or of `ARMING_REQUIRE` to 0 |
| `InputTimeOffQuantum`, `InputFramesOverlap`, `InputPastWindow` | timing the trace could not hold exactly |
| `InputPortUndeclared` | a port the specification does not declare |

### 6.2 The verdict

- **`input`:** `{path, sha256, port, frames, fill_bytes}`. The input's hash is part of the verdict.
- **`armed`:** `{witnessed, heartbeat: {offset, sysid, compid, base_mode}}`.
  - Read from ArduPilot's own transmitted bytes on that port (the file backend, as the generated path's capture),
    never from the input.
  - **A heartbeat counts only if** it decodes with its CRC, its source differs from every input frame's source,
    its bytes equal no input frame's bytes, and `base_mode` has `SAFETY_ARMED` (128).
- **`replies`:** for each command, ArduPilot's `COMMAND_ACK` result. For each `PARAM_SET`, its `PARAM_VALUE` echo,
  or its `PARAM_ERROR`, which is a finding naming the parameter (measured: error 1 for an unknown name).
- **`stimuli`:** each stimulus set, its value, and its citation. A witness stimulus that is not at rest is named
  so.

### 6.3 The trace (version 20)

A foreign run with `--input` writes an archive (`--archive DIR`), with these records:

| record | kind | content |
|---|---|---|
| `SerialInput` | 30 | the port, the peripheral, `spacing_ns`, and the input file's bytes verbatim, so replay recomputes its hash and every byte's instant (`at_ns + k·spacing`: integer nanoseconds, every one on a quantum) |
| `Stimulus` | 31 | device, property, `at_ns`, value: the rest stimuli and every later one |
| `SerialCaptured` | 32 | ArduPilot's transmitted bytes on the port, in order |

Replay has two possible meanings, for the architect to rule (§7 D6):
- **R1, re-judge.** `replay --archive` re-judges from the records alone, as it does for a campaign today: the input's
  hash, the armed witness and the replies from `SerialCaptured`. No Renode.
- **R2, re-drive.** A foreign replay re-runs Renode from the records, with no input file, and compares the new
  capture with the archived one byte for byte.

### 6.4 The rest stimuli

- **On a foreign run with `--input`, and only there, the board starts at rest:**
  - each IMU by its declared orientation (§4);
  - each barometer at its parts-table pressure, moved ±0.08 Pa at 12 Hz (§4).
- **Without `--input`,** the witness stimuli stay, so no committed foreign verdict moves.
- An IMU with no declared orientation is refused on an input run (`RestOrientationUndeclared`, `INV-06`).
- The two datasheet figures live in a cited data file in `aethelgard-twin`, beside `dmamux1-spi-rx.tsv`.

## 7. The stop: decisions for the architect

- **D1. The window.**
  - The arm needs at least 8.76 s of virtual time (§5); 5 s holds none of it.
  - **Lean:** build the input, the trace and the witness inside the CLI's 5 s bound. Under that bound, ArduPilot
    answers the `PARAM_SET`s, and refuses every ARM with its own `Arm: System not initialised` (ACK 4, measured to
    4.86 s; `ArduPilot Ready` comes at 5.21 s). That refusal is the CLI's acceptance for now.
  - Show the arm in a spike replay of the CLI's own invocation with the window lifted, the gap named, as `4400h` did,
    until `AET-WO-4400j` sets the window.
- **D2. The request lines: where.**
  - **(a) Foreign boards only (lean).** No committed verdict moves. The foreign verdicts differ only in the platform's
    hash (§2.3).
  - **(b) Every declared port on every board.**
    - The generated firmware never sets `DMAR`, so no behaviour moves.
    - But seven of the eleven committed platforms emitted move, and with them the bench package's verdict (§2.3).
    - Its `MANIFEST.json` is in `fab/`, outside this fence, so it would need a ruled re-derive.
- **D3. IDLE.**
  - **Lean:** declared fill plus spaced bytes, with the gap named in every input verdict (`usart_idle_unmodelled`).
  - Measured: `idle-fill`, and `early-fill` arms.
  - **A row:** the two models of §3.3:
    - a USART with IDLE and a stored `IDLEIE`;
    - a DMA stream that ends with TCIF on an `EN` clear and restarts at `M0AR`.
  - Foreign boards only. Three named model gaps, each cited and measured.
- **D4. The rest stimuli** as §6.4, cited, and only on input runs.
- **D5. The acceptance board and firmware.**
  - Main's Sentinel-03 fixture has no U5 orientation and no `board.forward` (`4400h` D5), and its ELF exists only on
    Furnace.
  - **Lean:**
    - CI's added `foreign-judge.sh` cases run on `sentinel-02-own`, which has the orientation and a CI-built ELF
      (`4400h` armed it, `arm4`).
    - Sentinel-03's arm is shown in the spike, on `4400h`'s derived spec, until `4300k`'s `examples/sentinel-03.json`
      lands.
- **D6. Replay: R1 or R2** (§6.3).
  - **Lean:** R1 for `replay`, the semantics every archive has.
  - Plus one R2 re-drive, in the spike, as the acceptance's "replays from the trace alone".
- **D7. The fence.**
  - The input and archive flags live in `main.rs`, and the records and trace 20 in `aethelgard-chaos` (`record.rs`,
    `event.rs`). Neither is named in the fence.
  - The rest data file is in `aethelgard-twin` (fenced).
  - **Asked:** may the build touch `main.rs`'s `Simulate` and `aethelgard-chaos`'s records? No sibling has claimed
    trace 20: `4300k` and `4411` add no `TRACE_CHANGES` row.
- **D8. The refusal list** (§6.1). In particular `InputSkipsChecks`'s two parameters, and the closed message table.
- **D9. "A parameter ArduPilot rejects."** Measured: ArduPilot refuses only an unknown name, in its own
  `PARAM_ERROR`. It took `FRAME_CLASS` 99 and `STAT_RUNTIME` 5. **Lean:** the planted input sets an unknown name.

**Rows (not this work order's):**
- the IDLE and DMA models (D3);
- the bench package's re-derive if D2 (b);
- Renode's DMA write offset kept across a stopped stream (§3.2 gap 3). It is the same model every SPI receive DMA
  uses, so an SPI firmware that stops a stream early would see it too (not measured).

## 8. Predictions, graded

| | prediction | measured |
|---|---|---|
| P1.1 | generated injection by `ScheduleAction`; kind 29; trace 19; replay re-judges | **right, by reading** (not re-run) |
| P1.2 | a foreign run injects nothing; no reply without the line | **right:** `noreq`, 0 replies, 6 heartbeats |
| P2.1 | Table 124's eight requests; 121 was a mis-cite | **right** |
| P2.2 | no generated behaviour moves, only the platform's hash; the `.repl` goldens and the fab doc move | **right, and more:** the board crate's tree hash moves too, so 13 gates' input hashes and the bench package's `MANIFEST.json` (§2.3) |
| P2.3 | foreign verdicts unchanged but for the platform's hash | **right:** both differ only at `/input/*` and the run's memory reading |
| P3.1 | unpadded: 64 B hand-offs, delayed, the tail stuck, the arm still reached | **right for the first three:** 214–459 ms late, the last three never answered. The arm under unpadded frames was not run |
| P3.2 | padding sound for delivery; a frame over 64 B is handed over when its multiple fills | **wrong at one instant:** a padded frame over 64 B kills the receive path (§3.2). **Right spaced** (`idle-fill`). The rest right |
| P3.3 | hooks give every frame in order, about as soon as padded, under 5 % cost | **wrong:** first frame only (56.5 ms); three model gaps (`IDLEIE` a tag, no TCIF on an `EN` clear, the DMA's offset kept). No time cost. The subclass question was not checked |
| P4.1 | the 1 g on y is the parts table's; no z channel | **right** |
| P4.2 | rest from the orientation is X 0, Y 0, Z +1 g; no Leaning | **right** (`early`) |
| P4.3 | 101325 Pa ±0.08 Pa at 12 Hz passes the barometer and the arm | **right** (`early`) |
| P5.1 | ArduPilot reads from about 2.5 s; heartbeat first | **wrong:** first reply at 0.754 s, before the first heartbeat (1.003 s) |
| P5.2 | earliest arm between 8.6 and 9.0 s, set by EKF3's alignment | **right:** 8.602 s, alignment at 8.564 s (`early-fill`: 8.608 s, 8.555 s) |
| P5.3 | witness within 1 s of the ACK | **right:** 0.152 s |
| P5.4 | does not fit 5 s; needs at least 10 s, past the 9 s log budget | **right that it does not fit; wrong on how much:** about 8.8 s, between `4400h`'s 8 s (97 % of the budget) and 9 s |
