# AET-WO-4400e3 — FEEDBACK (build)

Branch `wo/aet-4400e3-rettobase`, from `main` at `7d52d29e`. The work order is read from `~/.cache/aet-4400e3-wo.md` (docs PR #216). It was ordered by the ruling on #215 (comment 6961943794, fix (1)).

Every answer is labelled **measured**, **read at source** or **inferred**.

## Sources (read at source)

- **Armv7-M Architecture Reference Manual,** ARM DDI 0403E.b (ID120114), §B3.2.4 "Interrupt Control and State Register, ICSR", p. B3-657. The PDF is from `documentation-service.arm.com/static/5f8fef3af86e16515cdbf816`, sha256 `2d4af213859ed4650066b28937e3fc7a62974e521d0ff6df965bb3766d171f2c`, recorded in `~/.cache/aet-ds/SHA256SUMS`. The text:
  > "RETTOBASE, bit[11]. In Handler mode, indicates whether there is an active exception other than the exception indicated by the current value of the IPSR: 0 There is an active exception other than the exception shown by IPSR. 1 There is no active exception other than any exception shown by IPSR. In Thread mode the value of this bit is UNKNOWN."
- **Renode 1.17.0** (the pinned image's build `1.17.0+20260907gitf1dd1b4af`; `renode-infrastructure` `cd4b002a`), `src/Emulator/Cores/Arm-M/NVIC.cs` L1079: `RETTOBASE` = `activeIRQs.Intersect(SystemException).Count() <= 1`, which counts system exceptions only.
- **Upstream `e7a8584c`** (2026-10-07, "[GH#941] NVIC: Make ICSR.RETTOBASE take external interrupts into account"): `activeIRQs.Count() <= 1`, which counts every active exception. It is in no release (`4400e2` FEEDBACK).
- **The pin:**
  - The CLI already reads `renode --version`: it refuses any version but v1.17.0 (`main.rs` `renode_pinned`) and parses the `build:` line (`renode::build`).
  - The image installs Renode from `container/Dockerfile` L42–50: `renode-1.17.0.linux-portable.tar.gz`, sha256 `4ba7c68b…`.

## The design (read at source; nothing run yet)

- **The hook:** one monitor command, `sysbus SetHookAfterPeripheralRead sysbus.nvic "…"`.
  - On a read at offset `0xD04` (`ICSR`), it sets bit 11 to `activeIRQs.Count <= 1`. It reads the NVIC model's own active-exception stack, which is upstream's expression exactly. It passes every other bit through.
  - **Why the model's stack, not IABR plus SHCSR (the `4400e2` spike's choice):** the architecture defines `RETTOBASE` from every active exception. HardFault and NMI have no active bit in `SHCSR` or `IABR`, so the spike's count missed them.
- **Where:** it goes in every boot's monitor commands, right after the platform loads, while `renode --version` names the pinned buggy build.
  - A build measured to carry the fix gets no hook. That list is empty: no Renode carrying the fix has been measured here.
  - Any other build is refused by name before the boot (`INV-06`): whether its `RETTOBASE` is architectural is a fact the twin does not have.
- **The pin tests:**
  - **tier 1:** the Dockerfile's Renode tarball and version equal the constants the hook is tied to;
  - **tier 2, in the image:** the image's `renode --version` build is the buggy one; the bug is present without the hook (a test named for it); and the hook corrects it.
- **The regression firmware:** a dependency-free fixture, like `forge-log`.
  - It gives TIM5 (IRQ 50) priority `0x70` and DMA2 stream 1 (IRQ 57) `0xA0`, the priorities `4400e2` measured. It enables both and pends IRQ 57 from thread mode.
  - IRQ 57's handler pends IRQ 50, which preempts it.
  - It records `ICSR` in thread mode, in the outer handler and in the inner one, then disables both and parks.

## Predictions — written before any run

| id | What | Prediction |
|---|---|---|
| P1 | The regression firmware **without** the hook | the inner handler reads `RETTOBASE` **1** (the bug); the outer reads 1; thread mode reads 1. `VECTACTIVE` is 66 inner and 73 outer |
| P2 | **With** the CLI's hook | inner **0**; outer 1; thread 1. Every other bit of each word equals the run without the hook |
| P3 | `4400e`'s ELF under the CLI's newly emitted script (only the ELF swapped) | no fault; reaches `Init Gyro`; polls `FIFO_COUNT` |
| P4 | `4400c2`'s repro with the hook from the CLI's emitted script | no fault past 1.6016 s; answers the `COMMAND_LONG` |
| P5 | Every generated-firmware verdict (the reference and the bench board at 100 ms; every scenario's campaign line) | **byte-identical** to `main`'s, outside the `memory` sample. The generated firmware enables no interrupt, so no `ICSR` read ever differs, and the judge's own `ICSR` read happens in thread mode |
| P6 | The hook's cost on a generated boot | below the noise of the 100 ms boot's wall time |
| P7 | The reflection on a private field | works in Renode 1.17.0's IronPython (`GetField(..., NonPublic \| Instance)`) |
