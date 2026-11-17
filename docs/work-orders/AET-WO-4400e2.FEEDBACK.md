# AET-WO-4400e2 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-4400e2-hard-fault`, from `main` at `0829fe83`. The work order is read from `~/.cache/aet-4400e2-wo.md` (docs PR #213). The ruling that ordered it is #212, comment 6958822008, D1 (a). Prototypes and every diagnostic deviation live under `spike/4400e2/`; no crate, model, workflow or `container/` file changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

## Predictions — written before any run

### What they rest on (read at source, nothing run)

**ChibiOS's port** (the pinned ArduPilot image, ChibiOS `d103983f`, `os/common/ports/ARMv7-M/chcore.c`):
- `__port_irq_epilogue` (L312–) runs at the end of every kernel ISR.
- **When `SCB->ICSR & RETTOBASE` is set (L315)**, it pushes an artificial exception frame on the PSP. That frame returns to `__port_switch_from_isr`, which calls `chSchDoPreemption`. Afterwards, `__port_exit_from_isr` and `svc 0` discard the frame.
- **The decision rests entirely on `RETTOBASE`:** 1 must mean "this ISR returns to thread mode".
- `CORTEX_SIMPLIFIED_PRIORITY` is `FALSE` (`chcore.h` L221), so kernel ISRs at different priorities can nest.

**Renode 1.17.0's NVIC** (`renode-infrastructure` `cd4b002a`, `src/Emulator/Cores/Arm-M/NVIC.cs` L1079):
```
.WithFlag(11, FieldMode.Read, valueProviderCallback: _ => activeIRQs.Intersect(Enum.GetValues(typeof(SystemException)).Cast<int>()).Count() <= 1, name: "RETTOBASE")
```
`SystemException` (L3195–) holds exceptions 1–15 only. **So an external interrupt active beneath the current one is not counted:** with two external IRQs nested, or one over SVC, `RETTOBASE` reads 1.
- **Armv7-M (`ICSR`):** `RETTOBASE` is 1 only when no exception other than the current one is active.
- **Upstream:** `e7a8584c` (2026-10-07, "[GH#941] NVIC: Make ICSR.RETTOBASE take external interrupts into account") changes it to `activeIRQs.Count() <= 1`. That is after v1.17.0 (2026-09-07), and no release has followed (read through the GitHub API, 2026-11-17).

**Renode's SPI model** (`STM32H7_SPI.cs`, sha256 `bc3f1348…`, L187–194): `CFG2.MASTER` reads 1 always, and a write of 0 logs `[ERROR] Attempted to set peripheral into SPI slave mode`.
- **ChibiOS:** `spi_lld_stop` writes `CR1 &= ~SPE`, then `CFG2 = 0U` (`os/hal/ports/STM32/LLD/SPIv3/hal_spi_lld.c` L736–746).
- **ArduPilot:** it calls `spiStart` per transaction (`SPIDevice.cpp` L386) and `spiStop` on release (L372).
- **RM0433 Rev 9** §50.11.4 (p. 2221): `SPI_CFG2` reset value `0x0000 0000`; "write protected when SPI is enabled"; bit 22 `MASTER`: 0 slave, 1 master.

### Predicted

| id | What | Prediction |
|---|---|---|
| P1 | **The identity** (`4400c2`'s `crash-a`, with `4400e`'s PSP-frame hook) | the same switch: `R0` = `ch0`, `R1` = the idle thread, `INVSTATE`, at `4400c2`'s 1.601556130 s. **The same fault** |
| P2 | **`RETTOBASE` at each ISR's exit** (`__port_irq_epilogue`'s entry, with every active exception read from `IABR0`–`4` and `SHCSR`) | in the faulting cycle, an epilogue runs with **two or more exceptions active and reads `RETTOBASE` 1**: one kernel ISR nested in another. No earlier epilogue does both |
| P3 | Every interrupt in the last millisecond | DMA1 stream 2 (SPI2) and TIM5; the nested pair is one of them inside the other |
| P4 | `FPCCR`, `FPCAR` and the frame type | `FPCCR` `ASPEN` and `LSPEN` set; every `EXC_RETURN` `0xFFFFFFED`, frames extended as it says. Lazy stacking is **not** the cause |
| P5 | `SVCall` and `PendSV` priorities (`SHPR2`, `SHPR3`) against the ISRs' (`NVIC_IPR`) | as `chcore.h` sets them; tlib honours them. Not the cause |
| P6 | **Deviation A:** `RETTOBASE` given its architectural value at the epilogue's `ICSR` load (a hook that rewrites the loaded register; the firmware unchanged) | **no fault** at 3.5708 s: ArduPilot reaches `init_gyro` and polls `FIFO_COUNT` (`0x2E`), then waits (`4400e` B2). `4400c2`'s repro also does not fault |
| P7 | **Deviation B:** ArduPilot built with lazy stacking off (`FPCCR.LSPEN` 0) in the pinned ArduPilot image | **still faults**, at or near the same instant |
| P8 | **Deviation C:** a Renode with `e7a8584c` (no release has it; a scratch image of a development build) | no fault (if a build is obtainable inside a scratch image) |
| P9 | **The class** | **twin emulation:** Renode 1.17.0's `ICSR.RETTOBASE`. **Smallest fix:** pin a Renode carrying `e7a8584c`, or carry that one-line patch in the pinned image's build. Weakens no witness |
| P10 | **D4, the SPI `[ERROR]`** | written by `spi_lld_stop`'s `CFG2 = 0U` after `SPE` is cleared: one per `spiStop`. On silicon, the reset value written to a disabled SPI. **A false alarm** of a model that supports only master mode |
| P11 | Cost | each diagnostic run under 3.6 s virtual: 10–60 s of wall (hooks at high rate are slow, `4400e` `r7`) and 500–550 MiB |

## The answer, first

**Twin emulation, located by measurement.**
- **The bug:** Renode 1.17.0's NVIC computes `ICSR.RETTOBASE` from active **system** exceptions only (`NVIC.cs` L1079).
- **The trigger:** when TIM5's interrupt preempts a DMA stream's interrupt (a legal nesting: TIM5 at priority `0x70`, the DMA streams at `0xA0`), TIM5's ChibiOS epilogue reads `RETTOBASE` 1 with two exceptions active. It therefore pushes a thread-mode return frame onto the interrupted thread's stack, although it is returning to another ISR.
- **The fault:** when that stray frame targets `__port_switch_from_isr`, the thread's later `svc` exit lands in it. A second `chSchDoPreemption` then runs with the ready list empty, and `__port_switch` loads `ch0`, the list's header, as a thread: `INVSTATE`.
- **Fixing the one bit removes the fault in both repros.** That was measured twice: at the firmware's load of `ICSR`, and at the model's answer.
- **The fix exists upstream:** `e7a8584c` ("[GH#941] NVIC: Make ICSR.RETTOBASE take external interrupts into account", 2026-10-07). It is in no release.
- **`4400c2` D3 is the same fault** (measured). D4's `[ERROR]` is a false alarm (measured and read at source).

## Results — measured after the predictions (`4b1fcfa0`)

**How it ran (measured):**
- Every container went through `spike/4400e2/sh/img.sh` under `~/bin/capped-run --mem 8G`, one at a time. The caps were read back each time (`out/caps.log`: `memory.max=8589934592 memory.swap.max=0 pids.max=8192`).
- **The two repros:**
  - **`4400c2`'s `crash-a`.** `spike/4400e2/c2drive.py` is `4400c2`'s driver at `5a1bccaf` with only its ELF and argv paths moved. It boots `4400c2`'s ELF (sha256 `c8a2c9fa…`, `ardupilot-image` run 44051581563) on `4400c2`'s platform `p2-spi-usart.repl`. The 44-byte `COMMAND_LONG` goes in three times at 1.5 s.
  - **`4400e`'s boot.** `spike/4400e2/drive.py` (from `4400e`) boots CI's `sentinel-02-own` ELF from `main` `0829fe83` (sha256 `a799dfb6…`, run 44148995663) under the CLI's own monitor script. The script was captured again here (`out/renode-argv.txt`), and the booted platform is byte-identical to the emitted one.
- **Hooks:** every hook is observation only unless labelled a deviation. Each run's `setup.cmds` is its as-run record: the stub files were extended between runs (the fake-frame hook was added after `i1` and `l1`).
- **Run cost:** `out/costs.txt`.

### §1.2 The identity with `4400c2` D3 — measured (`i1`)

`4400c2`'s repro, with `4400e`'s PSP-frame hook:
- `HardFault` at **1.601556130 s**, the instant `4400c2` recorded. `CFSR` `0x00020000`, `HFSR` `0x40000000`.
- The frame: `R0` `0x30016510`, which is `ch0` in that ELF; `R1` `0x30016780`, inside `ch_c0_idle_thread_wa` (`0x30016590`, 560 B); `LR` `0x08000EDF`.

**The same switch to the ready list's header: the same fault.** It occurs here with USART1's receive line and in `4400e` without it. So the USART line does not manufacture it: it is one of the DMA streams that TIM5 nests over.

### §1.3 Located — measured

**`RETTOBASE` at each ISR's exit** (`i1`, `l1`):
- **Where:** at ChibiOS's `ands r3, r3, #0x800` (the 4400e ELF's `0x081034DC`, the 4400c2 ELF's `0x081000F8`), `r3` holds the `ICSR` the epilogue loaded.
- **What was logged:** every epilogue running with two or more exceptions active, from `IABR0`–`4` and `SHCSR`'s active bits.

| Run | Epilogues with two exceptions active | Each one | Then |
|---|---|---|---|
| `i1` (4400c2, 2 s) | **one**, at 1.601291870 s | TIM5 (IRQ 50) over DMA2 stream 1 (IRQ 57, SPI1 receive); `RETTOBASE` loaded **1** | `HardFault` 264 µs later |
| `l1` (4400e, 3.6 s) | three | at 0.018172840 s, TIM5 over DMA1 stream 4 (USART1 transmit); at 3.570530890 and 3.570578140 s, TIM5 over DMA1 stream 2 (SPI2 receive). Each loaded **1** | no fault after the first; `HardFault` 0.23 ms after the third |

**The chain, measured end to end** (`ch-c2`: from 1.6 s, every vector's entry with its `EXC_RETURN`; plus, at the epilogue's fake-frame store, the trampoline it stores and the frame's address):
1. **1.601289740 s:** DMA2 stream 1's interrupt is entered from thread mode (`EXC_RETURN` `0xFFFFFFED`). Its frame is at `0x30016718`, on the idle thread's stack.
2. **1.601290550 s:** TIM5's interrupt preempts it (`EXC_RETURN` `0xFFFFFFF1`: return to handler mode; basic frame on MSP; the stacked `xPSR`'s exception number is 73, DMA2 stream 1).
3. **1.601292110 s:** TIM5's epilogue, with two exceptions active, stores **target `0x08000EDB` (`__port_switch_from_isr`)** in a fake frame at `0x300166B0`, on the idle thread's stack (`FAKEFRAME n=2 … in_idle_wa=True`).
4. Another thread then runs its SPI cycles on its own stack (`0x300020E0`; which thread was not read).
5. **1.601555500 s:** the idle thread's `svc` (exception 11, its frame at `0x30016648` on the idle stack) unwinds into the stray frame. `chSchDoPreemption` runs with nothing ready, and the switch to `ch0` faults at **1.601556130 s**.

**Why 0.018 s did not fault** (`ch-e018`): the stray frame's target there was **`0x08000EDE`, `__port_exit_from_isr`** (no preemption required), on a non-idle stack. That frame only adds one more `svc`, which discards it. The fault needs a stray frame that requires preemption.

**Excluded, each by a run:**

| Candidate | Measured | |
|---|---|---|
| Lazy stacking (`FPCCR`, `FPCAR`) | at every exception entry in `ch-c2` (52), the stack `EXC_RETURN` names holds a valid stacked `xPSR` (Thumb bit set) where its frame type puts it: 51 extended frames from thread mode, 1 basic frame for the nested TIM5. `FPCCR` `0xC0000011`. **Deviation B** (`b-c2`, `FPCCR ← 0x80000000` at 1.0 s; `LSPEN` read 0 at the fault): **the same fault, at the same instant, through the same nested epilogue** | excluded |
| Priorities | `SHPR2` `0x20000000` (SVCall `0x20`), `SHPR3` `0x00300000` (PendSV `0x30`), as `chcore.h` sets them (SVCall 2, kernel ceiling 3, `BASEPRI` `0x30`). TIM5 `0x70`, DMA2 stream 1 `0xA0`, DMA1 stream 4 `0xC0`; `AIRCR` `0xFA052300`. tlib preempted exactly as these say | the nesting is the firmware's design and is honoured |
| Tail-chaining or exception return | every `EXC_RETURN` is consistent with the stack it named; the fault follows the stray frame, not a misread one | not implicated |
| ArduPilot or ChibiOS | the firmware's code is right under the architecture's `RETTOBASE`. **Deviation A** (only that bit corrected) and the monitor fix (below) remove the fault, the binary unchanged | excluded (silicon below) |

### The deviations (each labelled; none on any crate, model, workflow or image)

| Id | What | Run | Result |
|---|---|---|---|
| **A**, first attempt | a hook at the epilogue's `ands` rewriting `r3` bit 11 to the Armv7-M value | `a-c2`, `a-e` | **invalid:** every rewrite raised `name 'RegisterValue' is not defined` (my error), so nothing changed and both faulted as before. Kept as a record (`out/runs/a-*/marks.log`) |
| **A** | the same, with the import fixed | `a2-c2` | **no fault:** the one rewrite (`0x842 → 0x42` at 1.601291870 s) and nothing else differs. It answers the `COMMAND_LONG` (`AUTOPILOT_VERSION` with `fb773567`, `d103983f`) and ends idle (`ICSR` `0x800`) |
|  |  | `a2-e` (to 4.0 s) | **no fault:** `_init_gyro` at 3.570425710 s, `Init Gyro` printed, then **975 `FIFO_COUNT` reads, each `0x00 0x00`** (`4400e` B2, now measured), idle at 4.0 s. Not "the same boot minus the fault": the 0.018 s rewrite already moved the trajectory by about 0.9 ms |
| **Monitor-level fix** | one `sysbus SetHookAfterPeripheralRead sysbus.nvic` line giving `ICSR` reads (offset `0xD04`) `RETTOBASE` = (active exceptions ≤ 1), as `e7a8584c`; every other bit, `VECTACTIVE` included, the model's (`stubs-mon-fix.txt`) | `m-c2` | **no fault:** the firmware now loads `RETTOBASE` **0** at the nested epilogue (`loaded=0 n=2` at 1.601291870 s); it answers and ends idle |
|  |  | `m-e` (to 4.0 s; fault and INS hooks only) | **no fault:** ICM identity at 3.569253830 s, `_init_gyro` at 3.570425710 s, 975 `FIFO_COUNT` reads, idle. **9.4 s of wall, 500 MiB**, about the cost of an unhooked boot (`4400e` `j1b`: 8.7 s) |
| **B** | lazy stacking off (a substitute for a rebuild; a build would write `FPU->FPCCR = ASPEN` in ChibiOS's `port_init`) | `b-c2` | **the same fault** (P7) |
| **C** | a Renode development build with `e7a8584c`: `renode-1.17.0+20261117git8e500459a.linux-portable.tar.gz` (sha256 `b1b6a582…`; its infrastructure `f1b14dab` is 517 commits past the pinned `cd4b002a`, `e7a8584c` among them, by the GitHub compare API), run from a scratch mount in the tools image | `c-c2` | **no fault**, but **no epilogue ran with two exceptions active** (the hook was live), so the fix path was not exercised |
|  |  | `c-e`, `c2-e` (to 7 s) | **no fault to 7 s:** INS at 4.137774840 s, ICM identity at 4.138467820 s, `_init_gyro` at 4.139694620 s, 5,616 `FIFO_COUNT` reads. **Confounded:** every witness moves on this build. The barometers' identity is at 0.167 s (0.343 s pinned) and INS at 4.14 s (3.57 s). `TIM5` `PSC` is 199 in both, so the shift is not TIM5's clock |

**Silicon (inferred, from the architecture):** on an Armv7-M core `RETTOBASE` is 0 whenever another exception is active (`ICSR`, the Armv7-M ARM). TIM5 nesting over a DMA interrupt then takes ChibiOS's no-switch path, which `a2-*` and `m-*` exercise. **Silicon would not fault here.**

### The class and the smallest fix

**Class: twin emulation.**

| Candidate fix | Where it lives | Measured | Witness impact |
|---|---|---|---|
| **(1) the monitor-level read hook** (`spike/4400e2/stubs-mon-fix.txt`, one line) | the CLI's monitor script (`renode.rs`), as a named correction of a pinned Renode bug, until (2) or (3) | `m-c2`, `m-e`: no fault, about no cost | It changes only `ICSR` bit 11, in every read. `core_in_exception` reads `VECTACTIVE`, untouched. In thread mode with nothing active, both give 1, so the judge's end-of-run `ICSR` read is the same byte. **The generated firmware arms no interrupt** (`NVIC.ISER` 0 is judged), so nothing nests and its reads never differ: **no generated verdict should move** (inferred; a build measures it byte for byte). **Weakens no witness.** It adds a Python read hook to every boot |
| **(2) backport `e7a8584c`** (one line, `NVIC.cs` L1079) into the pinned Renode | `container/` (the tools image builds Renode from source with the patch) | not built here | the same semantics as (1), at the model. An image rebuild, and a pin the tools image owns |
| **(3) pin a later Renode** (a development build now; a release when one carries `e7a8584c`) | `container/IMAGE` | `c-*`: no fault, **but every virtual-time witness moves** (above), and 517 other commits come with it | not weakening, but every pinned time and verdict of the generated firmware has to be re-measured |
| **Upstream** | renode-infrastructure | already merged (`e7a8584c`) | — |

**Recommended: (1) now, then (3) at the next Renode release.** The architect's call.

### D4, the SPI `[ERROR]` — measured and read at source

- **The write** (`d4`, SPI1's access log to 0.2 s): each `[ERROR] spi1: Attempted to set peripheral into SPI slave mode` follows, at the same instant, `WriteUInt32 to 0xC (Configuration2), value 0x0` from `PC 0x08100F82`, which is **`spi_lld_stop`+0x1E**. It comes after `CR1 = 0` (`SPE` cleared), `CR2 = 0` and `CFG1 = 0` (ChibiOS `hal_spi_lld.c` L736–746).
- **Counts:** 9 errors and 9 zero writes to `CFG2`. There were 10 writes to `CFG2` with `MASTER` set (`spi_lld_start`, L722) and 10 chip-select windows. **"One fewer" is the last transaction's `spiStop`,** not yet reached when the window ended.
- **On silicon** (RM0433 Rev 9 §50.11.4, p. 2221): `SPI_CFG2`'s reset value is `0x0000 0000`, and the register is "write protected when SPI is enabled". The write returns a disabled SPI to its reset state, which reads as slave mode until the next `spi_lld_start` writes `MASTER`.
- **The model** (`STM32H7_SPI.cs` L187–194): `MASTER` reads 1 whatever is written, and a 0 logs at `Error`.
- **Answer: a false alarm** of a model that supports only master mode. The write is legal and harmless on silicon, and the model's behaviour (still master) matches what follows.
- **For `4400e`** (the architect's call): exempt exactly this message when it follows a `CFG2` write with `SPE` 0, which a judge can read from the same access log. Or ask upstream to log it below `Error` while `SPE` is 0. Exempting the text with no condition would also hide a firmware that really sets slave mode while enabled.

## Each prediction beside its result

| id | Predicted | Result | |
|---|---|---|---|
| P1 | the same switch to `ch0` in `4400c2`'s repro: the same fault | exactly, at 1.601556130 s | held |
| P2 | in the faulting cycle an epilogue runs with two or more active and reads `RETTOBASE` 1, and no earlier epilogue does | held for `4400c2`'s repro (one, 264 µs before). For `4400e`'s, an earlier one at 0.018 s did too, benign because its target was `__port_exit_from_isr` | half held |
| P3 | DMA1 stream 2 and TIM5, one inside the other | TIM5 inside DMA1 stream 2 (`4400e`), inside DMA2 stream 1 (`4400c2`) | held |
| P4 | `FPCCR` `ASPEN`/`LSPEN`; frames as `EXC_RETURN` says; not lazy stacking | `0xC0000011`; a valid stacked `xPSR` where `EXC_RETURN` puts the frame, 52 of 52 (frame size itself not measured); deviation B faults the same | held |
| P5 | priorities as `chcore.h`; honoured | as read; nesting as the priorities say | held |
| P6 | deviation A: no fault; `init_gyro` and `FIFO_COUNT` polls | first attempt **invalid** (my hook error); rerun: no fault in both, `_init_gyro`, 975 polls | held on the rerun |
| P7 | deviation B still faults | the same fault, the same instant | held |
| P8 | deviation C: no fault | no fault in both; the fix path not exercised in `c-c2`; witnesses shifted | held, confounded |
| P9 | twin emulation; fix: a Renode with `e7a8584c` or a backport; no witness weakened | twin emulation. The smallest fix is a third: a one-line monitor read hook | held, with a smaller fix found |
| P10 | `spi_lld_stop`'s `CFG2 = 0` after `SPE` cleared; a false alarm | exactly (`spi_lld_stop`+0x1E); the n−1 from the window's end | held |
| P11 | 10–60 s of wall, 500–550 MiB | 3.6–2,000 s: hooks at every epilogue cost 90–840 s, and the development build with them 1,956 s. Unhooked or lightly hooked: 4–17 s. Peak 500–637 MiB | missed on the wall time (hooks) |

## Cost (measured; `out/costs.txt`)

| Run | Virtual | Wall | Peak RSS |
|---|---|---|---|
| `m-e`, the monitor fix to 4.0 s | 4.0 s | 9.4 s | 500 MiB |
| `c2-e`, the development build to 7 s, light hooks | 7.0 s | 17.0 s | 539 MiB |
| `d4`, `ch-e018` | 0.2 s, 0.02 s | 3.9 s, 3.6 s | 500 MiB |
| per-epilogue hooks (`l1`, `a2-e`) | 3.6–4.0 s | 598–836 s | 500–535 MiB |
| `c-e`, the development build with per-epilogue hooks | 4.0 s | 1,956 s | 637 MiB |

The peak RSS of `i1`, `a-c2`, `a2-c2` and `c-c2` was not sampled: `c2drive.py` gained `drive.py`'s sampler afterwards.

## Evidence

- In git, under `spike/4400e2/`:
  - the drivers: `drive.py`, `c2drive.py` (with `RENODE`, `STUBS2` and `STUBS2_AT_S`);
  - `sh/`;
  - every stub file;
  - `reads-priorities.txt`;
  - `out/`:
    - per run, `setup.cmds` (as run), `steps.json`, the USART1 capture, the judge reads where taken, and `marks.log` (every `AET4400E2-` line);
    - `ch-c2`'s exception entries from 1.6012 s;
    - `d4`'s SPI1 access log;
    - `costs.txt`, `caps.log` and the CLI's argv.
- **Not in git:**
  - The ELFs, named by sha256 with their CI runs. `4400c2`'s platform and frames are in git history at `5a1bccaf`.
  - The development Renode tarball, named by URL and sha256.
  - Renode logs of up to 37 MB.
- **Sources read:** renode `f1dd1b4a`, renode-infrastructure `cd4b002a` and tlib `167decf9` (all fetched read-only from GitHub); ChibiOS `d103983f` and ArduPilot `fb773567` in the pinned image; RM0433 Rev 9 (sha256 `672a9433…`).
- **No crate, model, workflow or `container/` file changed:** `git diff --stat 0829fe83 -- crates models .github container` is empty.

**STOPPED at §2**, 2026-11-17: the cause is classed, the identity settled, D4 answered. For the ruling: which fix ((1), (2) or (3)), and D4's exemption. Title and body: `~/.cache/aet-4400e2/pr-title-body.md`.
