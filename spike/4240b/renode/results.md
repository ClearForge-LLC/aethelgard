# AET-WO-4240b: Renode 1.17.0 STM32H7_RCC hseFrequency and FLASH_ACR

Renode v1.17.0 (build 1.17.0+20260907gitf1dd1b4af.aet1605), tools image, via kit/img.sh. Raw output: `out_all.txt` (3 RCC runs), `out_acr.txt`. Files in this directory: `rcc_16.repl`, `rcc_8.repl`, `rcc_ctl.repl`, `run16.resc`, `run8.resc`, `runctl.resc`, `acr.resc`.

## Verdict
Renode takes hseFrequency: yes. sys_ck after SW=PLL1: 64,000,000 Hz for the 16 MHz fixture; 64,000,000 Hz for the 8 MHz fixture; 32,000,000 Hz for the control (no hseFrequency, default 8 MHz).

## .repl lines (override form)
A second full definition `rcc: Miscellaneous.STM32H7_RCC @ sysbus 0x58024400` after `using` FAILS: `Error E02: Variable 'rcc' was already declared, previous declaration was at :513:1.`
The update form works (it parses and the parameter takes effect):
```
using "platforms/cpus/stm32h753.repl"

rcc:
    hseFrequency: 16000000
```
(`rcc_8.repl` is the same with 8000000; `rcc_ctl.repl` is only the `using` line.) The stock `systick: nvic` stays from the stock file. The board repl `using`s `stm32h753.repl`, which includes `stm32h7.repl` (rcc at line 513), so a generated board would need this update form after its `using`.

## How sys_ck was read
Monitor `nvic Frequency` (SysTick frequency). RCC.cs UpdateSystick (L1388-1398) sets `systick.Frequency = SysCkBaseFrequency / d1CpreDivider`, D1CPRE = 1 after reset, and the log line `rcc: systick clock frequency changed to N` is printed by RCC itself. It is pushed only on writes to CR/CFGR/D1CFGR/D2CFGR. `timer2 Frequency` / `timer5 Frequency` agree (D1CPRE=HPRE=1, APB divisor 1 at reset). Renode's own number, not mine.

Confound: HSI at reset is 64 MHz, the same as the target, so a final read of 64 MHz alone proves nothing. The sequence therefore inserts CFGR = 1 (SW = CSI) before CFGR = 3, to move sys_ck off 64 MHz. Observed on every run: `systick clock frequency changed to 4000000` (CSI), then on SW=PLL1 the value below. The control run moves to 32 MHz, so the update path is live.

## Reads (all three runs)
Reset CR = 0x00000025 and nvic.Frequency = 0x3D09000 (64,000,000, HSI) at reset.

| step | 16 MHz | 8 MHz | control (16 MHz seq, default HSE) |
|---|---|---|---|
| CR after HSEBYP (8 MHz only), value written 0x40001 | n/a | 0x25 (bit 18 not stored) | n/a |
| CR after HSEON (HSERDY bit 17) | 0x00030025 | 0x00030025 | 0x00030025 |
| CR after HSECSSON | 0x000B0025 | 0x000B0025 | 0x000B0025 |
| PLLCKSELR written 0x52, readback | 0x00000052 | 0x00000052 | 0x00000052 |
| PLLCFGR written, readback | 0x01FF0000 (wrote 0x30004) | 0x01FF0000 (wrote 0x30002) | 0x01FF0000 |
| PLL1DIVR readback | 0x0103064F | 0x0103069F | 0x0103064F |
| D2CCIP1R (0x050) after writing 0x12 | 0x0 | 0x0 | 0x0 |
| D3CCIPR (0x058) after writing 0x3 | 0x0 | 0x0 | 0x0 |
| CR after PLL1ON (PLL1RDY bit 25) | 0x030B0025 | 0x030B0025 | 0x030B0025 |
| nvic.Frequency after SW=CSI | 0x3D0900 (4,000,000) | same | same |
| CFGR after SW=PLL1 | 0x0000001B | 0x0000001B | 0x0000001B |
| **nvic.Frequency after SW=PLL1** | **0x3D09000 = 64,000,000** | **0x3D09000 = 64,000,000** | **0x1E84800 = 32,000,000** |
| timer2 / timer5 Frequency | 64,000,000 | 64,000,000 | 32,000,000 |
| nvic.Divider | 1 | 1 | 1 |

Notes:
- D2CCIP1R/D3CCIPR offsets are 0x050 / 0x058 (RM0433). Both read 0 (tags) with warnings `Unhandled write to offset 0x50 ... Tags: SAI1SEL, RESERVED` and `offset 0x58 ... Tags: LPUART1SEL`.
- PLLCFGR: the written bits are mostly tags; readback stays at the reset 0x01FF0000. Warning: `Unhandled write to offset 0x2C. Unhandled bits: [2, 18-24] when writing value 0x30004. Tags: PLL1GRE (0x1), DIVR1EN ...` (8 MHz: `[1, 18-24] ... 0x30002. Tags: PLL1VCOSEL`). PLL1RGE and PLL1VCOSEL are not modelled; they do not affect sys_ck.
- HSEBYP (CR bit 18) is a tag: `Unhandled write to offset 0x0. Unhandled bits: [18] ... Tags: HSEBYP (0x1)`; it never reads back. The 8 MHz external case is therefore just `hseFrequency: 8000000`.
- Both 64 MHz results are not a no-op: the CSI step shows the update fires and PLL1 recomputes to 64 MHz (16/5*80/4 and 8/5*160/4). The control gives 8/5*80/4 = 32 MHz.

## FLASH_ACR (flashController, 0x52002000 offset 0)
Run `acr.resc` (platform rcc_16.repl, stock flash controller):
```
ACR at reset          0x00000037
ACR after write 0x11  0x00000011
```
No "unhandled" or tag log line was printed. Source (STM32H7_FlashController.cs, hint copy from infra 066a7f13, not verified identical to the pinned build, but the behaviour matches): lines 76-80, `Registers.AccessControl.Define(this, 0x37)` with `WithValueField(0, 4, "LATENCY")`, `WithValueField(4, 2, "WRHIGHFREQ")`, `WithReservedBits(6, 26)`, with the comment "Software writes to this register and expects written value back during reads". It is a plain read-back register: no wait-state effect.
