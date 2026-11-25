# AET-WO-4400i (1): predictions, committed before the runs

Written 2026-11-24, before any run of this work order, on main `fa12abf4`. Each is graded in `RESULTS.md`.

**Board and firmware for every run:**
- Sentinel-03 as `AET-WO-4400h` ruled it: main's fixture plus three declarations (`spike/4400h/py/derive.py`).
  Spec sha256 `135c96fa…`.
- Copter-4.7.2, built unmodified from it (`spike/4400h/sh/s03.sh`), ELF sha256 `dbf598d8…`.
- The CLI's own Renode invocation is captured at 5 s on main, then replayed with the run's frames
  (`sh/replay.sh`), each container under `~/bin/capped-run` with an explicit cap, one at a time.

## P1. Serial injection today

- **P1.1** Reading, to be confirmed by a run.
  - The generated path injects one 8-byte frame per port. Each byte is a `WriteChar` from a `ScheduleAction`, one
    character time rounded up to a quantum apart, starting a quantum after the boot word.
  - It is recorded as trace kind 29, `SerialWitness`, with `frame` and `injected_ns`, in trace 19.
  - `replay` checks the frame against the firmware hash and the label, without re-running Renode.
- **P1.2** A foreign run injects nothing and writes no trace. A frame written into Sentinel-03's USART1 with no
  request line gets no reply (as `4400h` `arm-noreq` on `sentinel-02-own`).

## P2. The receive request lines

- **P2.1** RM0433 Rev 9 Table 124 (p. 697) gives the receive requests: USART1 41, USART2 43, USART3 45, UART4 63,
  UART5 65, USART6 71, UART7 79, UART8 81. LPUART1 is on DMAMUX2 and gets no line. `4400h`'s patch cited
  "Table 121"; Rev 9's number is 124.
- **P2.2** Adding the line moves no behaviour on any generated board.
  - The generated bring-up leaves `CR3` 0 (`boot.rs`:938), so `DMAR` is clear and Renode never raises the line
    (`STM32F7_USART.cs`:109).
  - What moves is the platform text, and so the platform hash a verdict names as its `input`. Every committed
    `.repl` golden moves with it: `boards/sentinel-01*/…repl`.
  - `fab/sentinel-01-bench/BENCH-EXT-BMP581.md`:33 quotes a `.repl` hash. `fab/` is outside this fence.
- **P2.3** On a foreign run with no frame, the line moves nothing either. ArduPilot sets `DMAR`, but the receive
  queue stays empty, so the line never rises. The foreign verdicts are unchanged except for the `input` hash.

## P3. IDLE

- **P3.1** Unpadded frames, no IDLE:
  - A frame is handed to ArduPilot only when the bytes queued since the last hand-off reach 64. It is delayed,
    not lost.
  - A last frame followed by silence is never read (the tail is stuck).
  - Replies come late and in bursts: each hand-off answers every whole frame in it.
  - The arm is still reached if enough frames follow it.
- **P3.2** Padding is a sound stand-in for delivery, but not for the path:
  - ArduPilot's IDLE path (`rx_irq_cb` clears the stream's `EN`, then the transfer-complete handler or the 1 kHz
    tick drains `64 − NDTR`) never runs.
  - The zero bytes are input bytes silicon would not see. ArduPilot's parser skips them, so no reply changes.
  - A frame over 64 B is handed over only when its padded multiple of 64 fills.
- **P3.3** An IDLE overlay made of Renode hooks:
  - **The hooks:** a read hook on `ISR` that ORs in `IDLE` while one is pending, a write hook on `ICR` that clears
    it on `IDLECF`, and the USART's own `IRQ` raised a character time after a frame's last byte, when `IDLEIE`
    is set.
  - **Result:** every unpadded frame is answered in the same order and about as soon as padded, with no zero byte
    sent.
  - **Cost:** under 5 % wall and peak memory against the padded run.
  - **A C# overlay model:** cannot subclass `STM32F7_USART`'s registers, which are private. It would need a model
    of our own, with every serial judge's stored-bit list re-proved for it.
  - **Scope:** keeping the overlay to foreign runs keeps every generated verdict.

## P4. Rest stimuli

- **P4.1** The 1 g on y is the parts table's ICM-42688-P witness stimulus (`parts.json`:528–530). The table has no
  `acceleration_z` channel, so z is the model's default, 0.
- **P4.2** At rest, by DS-000347 rev 1.9 §4.7 ("placed on a flat surface, it will measure 0g on the X- and
  Y-axes and +1g on the Z-axis"):
  - each chip axis reads +1 g if declared `up`, −1 g if declared `down`, and 0 otherwise;
  - Y is Z × X (`orientation.rs`);
  - Sentinel-03's U5 (x right, z up) gives X 0, Y 0, Z +1 g: what `4400h` used, and it passes `Leaning`.
- **P4.3** The barometer at rest:
  - The pressure is the parts table's 101325 Pa (`parts.json`:184), inside the 30–125 kPa operating range
    (BST-BMP581-DS004-13 §1).
  - A constant reading fails `Baro: not healthy` after 2 s (`AP_Baro_Backend.cpp`:36–41).
  - The proposed stand-in for its intrinsic noise is Table 9's smallest row (×128): ±0.08 Pa RMS (a square wave of
    that amplitude) at that row's typical 12 Hz, so it moves every 83.3 ms.
  - It is above the 1/64 Pa resolution (5 counts), and below ArduPilot's own ×64 setting's 0.11 Pa.
  - It passes the barometer check, and the arm.

## P5. The window

- **P5.1** ArduPilot reads its first frame after about 2.5 s of virtual time. Its first heartbeat goes out before
  that.
- **P5.2** The earliest arm with no force, no skip and no defaults file is between 8.6 and 9.0 s, set by EKF3's
  tilt alignment (`4400h`: EKF attitude bad at 8.5 s, gone at 9.0 s). The accelerometer accept and the parameters
  are taken by 4 s.
- **P5.3** The armed witness, a heartbeat with `MAV_MODE_FLAG_SAFETY_ARMED`, comes within 1 s of the ACK (1 Hz).
  So the run needs about 10 s.
- **P5.4** **The arm does not fit the 5 s window.** It needs at least 10 s, past `4400h`'s 9 s log budget and its
  900‰ line at 10 s. So the acceptance is the replay outside the bounds, with the gap named.
