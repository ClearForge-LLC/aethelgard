# AET-WO-4240 read: Renode STM32H7_RCC (pinned) and ArduPilot Copter-4.7.2 H7 clock config

Read-only. Nothing in any repository was changed. Fetched sources are in `/home/forge/.cache/aet-4240/reads/src/`.
Tags: `[inferred]` = my reasoning, not a line quoted. `NOT FOUND` = looked, absent.

Short names used below:
- `RCC.cs` = `/home/forge/.cache/aet-4240/reads/src/RCC.cs` (fetched file, line numbers are its own).
- `repl` = `/home/forge/aethelgard-4240/crates/aethelgard-twin/tests/renode-1.17.0/platforms/cpus/stm32h7.repl` (stock Renode platform, vendored in the repo).
- `mcuconf` = `libraries/AP_HAL_ChibiOS/hwdef/common/stm32h7_mcuconf.h`.
- `AP` = ArduPilot tree. The sparse checkout at `/home/forge/.cache/aet-4210/ardupilot` has no Nucleo hwdef and no `modules/ChibiOS`. I did NOT run git commands that could lazy-fetch objects. I read the full tree of the same commit from a podman image layer:
  `/home/forge/.local/share/containers/storage/overlay/68211d1c69bdb0a4cd12bb088bbd787f2ad2f193c71bdfee93b3f92961f6e780/diff/opt/ardupilot` (abbreviated `$IMG` below). Its `stm32h7_mcuconf.h` has the identical sha256, and its HEAD is the same commit.

---------------------------------------------------------------------------------------------------
## PART A. Renode STM32H7_RCC

### A0. Provenance checks

- Full sha: `cd4b002aac2398994c4a61a34a8b31b5700facf2` (GitHub API, `commits/cd4b002a`).
- Local `/home/forge/.cache/aet-4400g/src/STM32H7_RCC.cs` sha256 = `7f10d491afd279d1a2009022f308a913a3b7b6412b112d214ee697d35ee4cca1`.
- Fetched `raw.githubusercontent.com/renode/renode-infrastructure/cd4b002a.../src/Emulator/Peripherals/Peripherals/Miscellaneous/STM32H7_RCC.cs` sha256 = `7f10d491afd279d1a2009022f308a913a3b7b6412b112d214ee697d35ee4cca1`. **IDENTICAL.** 1860 lines. The local copy IS the cd4b002a file.
- Also fetched at cd4b002a (paths in the repo tree):
  - `src/Emulator/Main/Peripherals/Timers/LimitTimer.cs` (362 lines)
  - `src/Emulator/Peripherals/Peripherals/Timers/STM32_Timer.cs` (1018)
  - `src/Emulator/Cores/Arm-M/NVIC.cs` (SysTick owner)
  - `src/Emulator/Peripherals/Peripherals/UART/STM32F7_USART.cs` (456)
  - `src/Emulator/Peripherals/Peripherals/Miscellaneous/STM32_PWR.cs` (125). NOT used by the H7 platform; see A1.
- NOT checked: `STM32H7_FlashController.cs`. The local copy `/home/forge/.cache/aet-4230/renode-src/STM32H7_FlashController.cs` comes from a different infra commit (`SOURCE.txt`: `infra 066a7f13c05...`), so it is not the pinned file. I quote it only as a hint.
- The platform file is the repo's vendored `renode-1.17.0` copy. I did not verify it is byte-identical to the platform scripts of the pinned Renode build.

### A1. RDY flags, SWS, PWR. Does a firmware poll hang or pass?

Every RDY is a pure read-back of the matching ON flag, so each reads 1 as soon as the ON bit is written 1. There is no delay and no timer. A `while(!RDY)` loop passes on its first read.

| flag | RCC.cs | quote |
|---|---|---|
| HSIRDY | L147 | `.WithFlag(2, FieldMode.Read, valueProviderCallback: _ => hsion.Value, name: "HSIRDY")` |
| CSIRDY | L169 | `.WithFlag(8, FieldMode.Read, valueProviderCallback: _ => csion.Value, name: "CSIRDY")` |
| HSI48RDY | L173 | `.WithFlag(13, FieldMode.Read, valueProviderCallback: _ => hsi48on.Value, name: "HSI48RDY")` |
| HSERDY | L185 | `.WithFlag(17, FieldMode.Read, valueProviderCallback: _ => hseon.Value, name: "HSERDY")` |
| PLL1RDY | L197 | `.WithFlag(25, FieldMode.Read, valueProviderCallback: _ => pll1On.Value, name: "PLL1RDY")` |
| PLL2RDY | L200 | `.WithFlag(27, FieldMode.Read, valueProviderCallback: _ => pll2On.Value, name: "PLL2RDY")` |
| PLL3RDY | L202 | `.WithFlag(29, FieldMode.Read, valueProviderCallback: _ => pll3On.Value, name: "PLL3RDY")` |

- Reset value of CR is `0x01` (L136 `Registers.ClockControl.Define(this, 0x01)`), so HSIRDY reads 1 at reset (`hsion` is set).
- A RDY for a source that is OFF reads 0. The firmware must set ON first.
- RDY never depends on a valid configuration. PLL1ON with garbage dividers still reads PLL1RDY=1.
- The PLL ON/RDY flags are state only. Comment at L198: `// We don't use PLL2/3 for any frequency configuration but we track their state for invalid configurations`.

SWS follows SW at once:
- L219: `.WithEnumField<DoubleWordRegister, SystemClockSourceSelection>(0, 3, out systemClockSwitch, name: "SW", writeCallback: (previous, value) =>`
- L226: `.WithValueField(3, 3, FieldMode.Read, valueProviderCallback: _ => (ulong)systemClockSwitch.Value, name: "SWS")`
- The SW write callback only rejects values above 3 (L221-224 `if((int)value > 3) { systemClockSwitch.Value = previous; }`). It does NOT check that the chosen source is ON or ready.
- Selecting HSE while HSEON=0, or PLL while PLL1ON=0, is accepted and SWS follows. `sys_ck` is then computed as if the source were there (L1518-1540). The only case that gives null is PLL1 with a bad DIVN/DIVP/PLLSRC=3 (L99-105, L116-118).
- Guards that DO exist, all "ignore the write and log Warning":
  - L178-183: HSEON=0 refused while HSE is sys_ck, or drives PLL1 which is sys_ck. Message: `Cannot disable HSE while it is used to drive the sys_ck. Ignoring write.`
  - L193: PLL1ON=0 refused while SW=PLL. The code is `if(!value && systemClockSwitch.Value == SystemClockSourceSelection.Pll)`.
  - L267-274: PLLSRC changes refused while any PLL is on.
  - L276-283: DIVM1 refused while PLL1 is on.
  - L316-326 / L329-347: DIVN1 / DIVP1 refused while PLL1 is on. Illegal values (DIVN1 outside 3..511, an odd DIVP1 other than 1) log Warning. An odd DIVP1 is dropped.
- Consequence [inferred]: firmware that writes PLL1DIVR AFTER setting PLL1ON would have the divider write dropped. ChibiOS writes the dividers first (hal_lld.c, A5), so this is not hit.

PWR. The H7 platform does NOT use `STM32_PWR.cs` or any C# PWR model. It uses a Python stub.
- `repl` L728-745 (`pwr: Python.PythonPeripheral @ sysbus 0x58024800`). Quotes:
  - `if request.Offset == 0x00: # CR1` -> `request.Value = 0xF000C100`
  - `elif request.Offset == 0x04: # CSR1` -> `# Reset value + ACTVOSRDY (b13) indicates the voltage level is valid` -> `request.Value = 0x6000`
  - `elif request.Offset == 0x0C: # CR3` -> `request.Value = 0x4000000`
  - `elif request.Offset == 0x18: # D3CR` -> `# VOSRDY flag (b13) indicates the voltage was properly set.` -> `request.Value = 0x2000`
  - `else:` -> `self.WarningLog("Unhandled access to offset 0x{0:X}", request.Offset)`
- So `PWR->CSR1 & ACTVOSRDY` (bit 13 of 0x6000) is permanently 1. `PWR->D3CR & VOSRDY` (bit 13 of 0x2000) is permanently 1. ChibiOS's two VOS waits (hal_lld.c L107, L127) pass instantly. Whatever the firmware writes to D3CR (VOS scale) is not stored.
- PWR CR2 (0x08), CPUCR (0x10) and other offsets log `Unhandled access to offset 0x..` at Warning. ChibiOS writes `PWR->CR2` and `PWR->CPUCR` (hal_lld.c L111, L113). I expect 2 such warnings per boot [inferred; the else branch is the only one that logs, I did not run it].
- Whether the Python stub treats writes like reads (so a write to 0x00 also "sets" request.Value) is Renode core behaviour. NOT checked.
- 4400f does not cover PWR. The facts above are new.

FLASH_ACR [no clock effect]: `stm32h7.repl` L66 `flashController: MTD.STM32H7_FlashController @ sysbus 0x52002000`. The hint copy (different infra commit) defines ACR as a plain read-back: `// Software writes to this register and expects written value back during reads to work correctly` (line 75-79 of `/home/forge/.cache/aet-4230/renode-src/STM32H7_FlashController.cs`, `LATENCY` 4 bits, `WRHIGHFREQ` 2 bits). So ChibiOS's LATENCY poll passes [inferred for the pinned build].

### A2. HSE frequency: constructor parameter

- RCC.cs L26-27: `public STM32H7_RCC(IMachine machine, IHasDivisibleFrequency systick = null, ulong hseFrequency = DefaultHseFrequency, ulong csiFrequency = FallbackCsiFrequency) : base(machine)`
- RCC.cs L1633: `private const ulong DefaultHseFrequency = 8000000;`. L1630: `FallbackCsiFrequency = 4000000`.
- Stored once, readonly: L36 `this.hseFrequency = hseFrequency;`. L1628 `private readonly ulong hseFrequency;`.
- A `.repl` sets it as a constructor parameter by name, in the same style as `systick: nvic`. The stock platform (`repl` L513-514) is:
  ```
  rcc: Miscellaneous.STM32H7_RCC @ sysbus 0x58024400
      systick: nvic
  ```
  So the entry `hseFrequency: 25000000` on the rcc line would set it [inferred from Renode's constructor-parameter syntax; the stock repl shows only `systick:`]. Default if omitted = 8 MHz.
- HSE frequency is NOT readable or writable from firmware. It does not depend on HSEBYP.
- HSEBYP is a tag, with no effect: L186 `.WithTaggedFlag("HSEBYP", 18)`. It is accepted silently or with a tag warning [inferred; tag behaviour is Renode core], and does not change the frequency, RDY or anything else.
- The HSEBYP bit does not read back what was written [inferred: a tagged flag is not backed by storage].
- If the stock repl is used unchanged, HSE = 8 MHz. That equals the ArduPilot NucleoH743 / NucleoH753ZI `OSCILLATOR_HZ 8000000` (Part B4), so no repl entry is needed for those boards.

### A3. How sys_ck is computed and what is pushed

`SysCkBaseFrequency` (RCC.cs L1518-1540), switch on SW:
- Hsi -> `HsiFrequency` (L1542-1560): 64/32/16/8 MHz from `HSIDIV`.
- Csi -> `csiFrequency`. Hse -> `hseFrequency`. L1528-1529 `case SystemClockSourceSelection.Hse: return hseFrequency;`
- Pll -> `Pll1Frequecy` (L92-135), else null.

PLL1 (L96-135):
- L99-102: DIVN1 < 4 or > 512 returns null.
- L103-106: odd DIVP1 (other than 1) returns null.
- ref1_ck = source / `pll1Prescaler` (DIVM1), source chosen by `pll1Source` (PLLSRC): L108-114. Case `NoSelection` returns null.
- L122: `if(ref1_ck < Ref1ClockMinimumFrequency && ref1_ck > Ref1ClockMaximumFrequency)` logs a Warning. The `&&` can never be true [inferred], so the range check is dead. The constants are 1 MHz and 16 MHz (L1635-1636).
- Result: `vco = ref1_ck * divn1Multiplier; return vco / divp1Divider;` (L127-128).
- `divn1Multiplier = value + 1` (L326). `divp1Divider = division` = value + 1 (L347). The default `divn1Multiplier = 0x081` is set in `Reset()` (L76) and is not the register's reset value 0x01010280 [inferred: reset field 0x80 -> N=129; the code's 0x081 = 129 agrees].
- FRACN1 is a tag: L370 `.WithTag("FRACN1", 3, 13)`. PLL1FRACEN is a tag: L291 `.WithTaggedFlag("PLL1FRACEN", 0)`. Fractional settings are ignored.
- DIVQ1, DIVR1 are stored (L350-366) and never used in any frequency. DIVP1EN/DIVQ1EN/DIVR1EN are tags (L301-303). PLL1VCOSEL / PLL1RGE are tags.
- Division by zero [inferred]: DIVM1 = 0 sets `pll1Prescaler = 0` (L276-283) and `HSE / pll1Prescaler` in `Pll1Frequecy` would divide by zero. ChibiOS never writes DIVM = 0.
- HSI `Reset()` quirk: `Reset()` sets `pll1Prescaler = 1` (L78) while the PLLCKSELR reset value 0x02020200 has DIVM1 = 0x20 [inferred]. Harmless until DIVM1 is written.

Dividers applied (L1450-1475 `UpdatePeripheralsFrequency`): `peripheral.Frequency = frequency / (d1CpreDivider * hpreDivider * divider);`
- D1CPRE (L245), HPRE (L242): decode `IsBitSet(value,3) ? 2^((value&7)+1) : 1`.
- D2PPRE1 (L254), D2PPRE2 (L256): `IsBitSet(value,2) ? 2^((value&3)+1) : 1`.
- D1PPRE (APB3) and D3PPRE (APB4) likewise (L243, L262). D3CFGR has NO PropagateClock callback (L261-264): a D3CFGR write alone pushes nothing.

What is pushed to timers (L1412-1421 and L1482-1499):
- APB1 timers (L1242): TIM2-7, TIM12-14. APB2 timers (L1255): TIM1, TIM8, TIM15-17.
- The divisor is the "Table 57" timer ratio (L1480 comment: `The lookup value is taken from "Table 57. Ratio between clock timer and pclk"`).
- `LookupHclkDivisorForTimersKernelClock(d2Prescaler)` (L1482-1499), with the input being D2PPRE1 or D2PPRE2:
  - D2PPRE = 1 or 2 -> divisor 1 (the timer clock = HCLK).
  - D2PPRE = 4 -> `timersClocksPrescaler.Value ? 1ul : 2` (TIMPRE=1 -> 1, else 2).
  - D2PPRE = 8 -> TIMPRE ? 2 : 4. D2PPRE = 16 -> TIMPRE ? 4 : 8.
- TIMPRE is a read/write flag, bit 15 of CFGR: L231 `.WithFlag(15, out timersClocksPrescaler, name: "TIMPRE")`. It is applied. HRTIMSEL (L230) is a tag.
- Timer frequency = `sys_ck / (D1CPRE * HPRE * k)`.
- ArduPilot values (Part B): sys 400 MHz, D1CPRE /1, HPRE /2, D2PPRE /2 (k=1) -> timers 400/(1*2*1) = 200 MHz. This equals the 4400f measured result (`FEEDBACK` L99-L103: "pushed to 200 MHz").
- The timers' own `frequency` is an `ulong` property of `LimitTimer`, set with `Frequency =` (LimitTimer.cs L251-269). The setter throws for 0 and updates the clock entry: `clockSource.ExchangeClockEntryWith(OnLimitReached, oldEntry => oldEntry.With(frequency: effectiveFrequency));` (L266). So the counting timer changes frequency at once.
- STM32_Timer has NO override of Frequency [no grep hits for any such setter in `STM32_Timer.cs`]. The capture/compare channel timers are separate `LimitTimer` objects built with the constructor frequency: `STM32_Timer.cs` L104 `channel.Timer = new LimitTimer(machine.ClockSource, frequency, this, String.Format("cctimer{0}", j + 1), ...`. They are never retuned by the RCC. (Agrees with 4400f L107 "A propagated timer's channels stay at 250 MHz".)

Other pushes:
- APB1/APB2 peripherals (SPI2/3, USART2/3, UART4/5/7/8, I2C1-3 ...; USART1, USART6, SPI1/4/5): `sys_ck/(D1CPRE*HPRE*D2PPRE)` (L1425-1431; arrays at L1264-1296). This is the pclk, not the kernel clock; the file's own comment says `// This is a simplification of how these devices are clocked.`
- But only objects that implement `IHasFrequency` get it: L1462 `var peripheral = peri as IHasFrequency; if(peripheral == null) { continue; }`.
- `STM32F7_USART` is declared `public sealed class STM32F7_USART : UARTBase, IUARTWithBufferState, IDoubleWordPeripheral, IKnownSize` (STM32F7_USART.cs L19). It takes the clock in the constructor `uint frequency` (L21), stores it `private readonly uint frequency;` (L439) and uses it only in `BaudRate => BaudRateMultiplier * frequency / Math.Max(1, (uint)baudRateDivisor.Value)` (L50-51) and the receiver timeout (L129). So no RCC push reaches a USART [inferred: `UARTBase` was not fetched; if UARTBase were IHasFrequency the class would still have no setter]. 4400f says the same ("USARTs cannot follow", FEEDBACK L246). Agree.

SysTick (RCC.cs L1386-1410 `UpdateSystick`):
- `systick.Frequency = freq / d1CpreDivider;` with the comment `// We use the rcc_c_ck as the systick clock`. NOTE: it is sys_ck / D1CPRE only. HPRE is NOT applied. Real hardware SysTick runs from HCLK/8 or the processor clock (rcc_c_ck / hclk), so with ArduPilot's HPRE = /2 Renode gives SysTick 400 MHz where HCLK is 200 MHz [inferred; not checked against silicon].
- If `SysCkBaseFrequency` is null: `this.Log(LogLevel.Error, "Cannot update systick - invalid configuration of the sys_ck");` (L1402).
- `systick` is an `IHasDivisibleFrequency`, which the stock platform sets to the NVIC: `repl` L513-514 `systick: nvic`. `NVIC` is `IHasDivisibleFrequency` (NVIC.cs L30) with `Frequency` set -> `systick.Get(...).Frequency = value` (NVIC.cs L956-961). The NVIC constructor default is `systickFrequency = 50 * 0x800000` (L32). `stm32h743.repl` sets `nvic: systickFrequency: 480_000_000`.
- Constructor order: the RCC constructor calls `Reset()` (RCC.cs L40) which calls `PropagateClock()` (L81), so SysTick is set to HSI 64 MHz at construction (matches 4400f F4 and L105: "SysTick loads at 64 MHz, not 480 MHz"). Agree.
- If no systick was handed in: `this.Log(LogLevel.Warning, "Systick not passed in the RCC constructor. Changes to the system clock will be ignored");` (L31).

When it pushes (every `PropagateClock()`, L1501-1511, which does timers, APB1-4, AHB and SysTick):
- Callers: `Reset()` (L81); a change callback on CR (L204-206); CFGR (L237-239); D1CFGR (L247-249); D2CFGR (L258-260).
- NOT on: D3CFGR, PLLCKSELR, PLLCFGR, PLL1DIVR, PLL1FRACR, or the kernel mux registers. So the pushed clock is updated the first time one of the four registers actually changes value (change callback), after the dividers are stored.
- "Change callback" means the push happens only if the register value changes [inferred: `WithChangeCallback`].
- 4400f L13 lists the same four (its L206/L239/L249/L260 match my numbers). Agree.

Typical ChibiOS order and effect [inferred, from hal_lld.c + the above]:
1. `RCC->CFGR = 0` (SW=HSI): push at HSI.
2. `RCC->CR = HSION` (L270): change callback, push.
3. `RCC->CFGR = cfgr (TIMPRE | ...)`: push, still HSI.
4. HSEON|HSEBYP: change callback, push (still HSI).
5. PLLCKSELR, PLLCFGR, PLL1DIVR: no push. `CR |= PLL1ON|PLL2ON|PLL3ON`: push, still sys_ck = HSI because SW is not yet PLL.
6. D1CFGR, D2CFGR: push, still HSI sys_ck with the new dividers (so timers briefly get 64/(2*1)=32 MHz at D1HPRE=2, D2PPRE=2, TIMPRE: `4400f` L99 "at 3.77 us it is pushed to 32 MHz" matches).
7. `CFGR |= SW(PLL1)`: push -> sys_ck = PLL1P = 400 MHz, timers 200 MHz, SysTick 400 MHz. 4400f L100 "at 3.98 us it is pushed to 200 MHz, and SysTick to 400 MHz". Agree.

### A4. PLL2, PLL3, kernel clock muxes

- PLL2/PLL3 dividers are plain stored value fields, never read for a frequency:
  - L372-377 `PLL2DividersConfiguration` (`DIVN2` 8 bits, `DIVP2`, `DIVQ2`, `DIVR2`). L383-388 `PLL3DividersConfiguration`.
  - L379-382, L390-393: FRACN2/3 are tags.
  - DIVM2 and DIVM3 are plain value fields with no use (L286 `.WithValueField(12, 6, name: "DIVM2")`, L288 `DIVM3`).
  - PLL2FRACEN, PLL3FRACEN, PLL2/3VCOSEL (partly), PLL2/3RGE are tags (L294-299).
- PLL2ON/PLL3ON and RDY are state flags only (A1). The model "tracks their state for invalid configurations": the only use is the PLLSRC/HSIDIV guards (L151 `pll1On.Value || pll2On.Value || pll3On.Value`, L269).
- Kernel clock muxes are all tags and have no effect. Register and offsets (RCC.cs L1793-1796 enum):
  - `Domain1KernelClockConfiguration` D1CCIPR 0x4C (L394-402): FMCSEL, QSPISEL, SDMMCSEL, `CKPERSEL` (L401 `.WithTag("CKPERSEL", 28, 2)`) are tags.
  - `Domain2KernelClockConfigurationLow` D2CCIP1R 0x50 (L403-418): SAI1SEL, SAI23SEL, `SPI123SEL` (L408), `SPI45SEL` (L410), SPDIFSEL, DFSDM1SEL, FDCANSEL, SWPSEL are tags.
  - `Domain2KernelClockConfigurationHigh` D2CCIP2R 0x54 (L419-431): `USART234578SEL` (L420), `USART16SEL` (L421), RNGSEL, `I2C123SEL` (L425), USBSEL, CECSEL, LPTIM1SEL are tags.
  - `Domain3KernelClockConfiguration` D3CCIPR 0x58 (L432-448): LPUART1SEL, `I2C4SEL` (L435), LPTIM2SEL, LPTIM345SEL, ADCSEL, SAI4A/BSEL, SPI6SEL are tags.
- Tags read as 0 and drop writes. So D2CCIP2R reads 0 (agrees with 4400f L112 "reads the register back as 0" and F2). The logged line is `Unhandled write to offset 0x54` per 4400f [measured by 4400f; I did not re-run it].
- Consequence: PLL2 and PLL3 outputs, and every kernel mux, are invisible to the model. A judge that decodes USART or I2C kernel clocks can only use the ArduPilot-style write log line, or an external declaration (as 4400f Q2 says).

### A5. Warnings or refusals during a typical bring-up (ChibiOS hal_lld.c order)

Source order, `$IMG/modules/ChibiOS/os/hal/ports/STM32/STM32H7xx/hal_lld.c`:
- Exact lines (grep): `RCC->CFGR = 0` L265; `RCC->CR = RCC_CR_HSION` L270; HSE enable L293-300; PLL `CR |= onmask` L397, ready poll L398; `RCC->D1CFGR` L409; kernel muxes from L416; `FLASH->ACR` L455, LATENCY poll L457; `RCC->CFGR |= STM32_SW` L464, SWS poll L465; `PWR->CR2` L111, `PWR->CPUCR` L113.

Result against the model:

| step | model outcome |
|---|---|
| `RCC->CFGR = 0` then poll SWS == HSI | SW=0, SWS=0 at once. Pass. |
| `RCC->CR = HSION` (clears HSEON, PLLxON) | HSEON/PLLxON cleared. No guard fires (SW is HSI). |
| `RCC->HSICFGR = 0x40000000`, `RCC->CSICFGR` | CSICFGR is not written in the revision-XY build (`#if !defined(STM32_ENFORCE_H7_REV_XY)`). HSICFGR is offset 0x04 `InternalClockSourceCalibration` (L206-211, tags). [inferred] Fine. |
| `CR |= HSEON | HSEBYP` | HSEON accepted, HSEBYP tag. No refusal. |
| poll HSERDY | reads 1 at once. Pass. |
| `PLLCKSELR = DIVM3 | DIVM2 | DIVM1 | PLLSRC(HSE)` | PLLSRC written while no PLL is on, so OK; DIVM1 OK. No warning. |
| `PLL1FRACR`, `PLL1DIVR` | DIVN1 range OK (N-1 = 49 for 50 etc.); DIVP1 even; no warning. 4400f measured readback `0x01090231`. |
| `PLLCFGR = cfgmask` | all tags. |
| `CR |= PLL1ON|PLL2ON|PLL3ON`, poll PLLxRDY | RDY=1 at once. Pass. |
| `D1CFGR`, `D2CFGR`, `D3CFGR` | stored; pushes. D3CFGR no push. |
| `D1CCIPR`, `D2CCIP1R`, `D2CCIP2R`, `D3CCIPR` | tag writes. 4400f measured the "Unhandled write to offset 0x54" line for D2CCIP2R. I expect the same for the other three [inferred, same mechanism]. |
| `FLASH->ACR` + LATENCY poll | passes if ACR reads back what was written [inferred, A1]. |
| `CFGR |= SW(PLL1)`, poll SWS == 3 | SWS=3 at once; propagates 400 MHz. Pass. |
| PWR | Unhandled-access warnings for CR2 and CPUCR (A1). |

Overall: no firmware poll can hang on the stock model, whatever the HSE value is. The only way to hang is an HSEON/PLLxON never written. A wrong HSE (e.g. a board using 25 MHz crystal) would not hang; it would silently give a wrong sys_ck, e.g. 25 MHz firmware on the 8 MHz default would produce `8/2*64/2 = 128 MHz` [inferred arithmetic from PLL1 numbers in Part B].

### A6. CSS (HSECSSON)

- L187: `.WithFlag(19, FieldMode.Read | FieldMode.Set, name: "HSECSSON")`. A flag that can only be set (write-1-to-set), read-back. No failure detection, no NMI, no clock switch. `HSECSSF` (L468) and `HSECSSC` (L481) are tags. CSS is NOT modelled beyond holding the bit.
- Because the bit is Read|Set it cannot be cleared by firmware once set [inferred from the field mode].

### A7. After a peripheral reset of a timer

- `LimitTimer.InternalReset()` (LimitTimer.cs L322-335): `frequency = initialFrequency;` (L324) and `divider = initialDivider;`, then a new `ClockEntry(initialLimit, frequency / divider, ...)` is exchanged in. `initialFrequency` is the constructor's frequency (L38).
- `STM32_Timer.Reset()` (STM32_Timer.cs L555-568) calls `base.Reset()` (which is `LimitTimer.Reset()` -> `InternalReset()`, LimitTimer.cs L76-78), then `registers.Reset()` and each channel's `Reset()`.
- So after an RCC `xxxRST` write (RCC.cs L563 `TIM5RST` -> `ResetRegisteredPeripheral` L1377-1384, which calls `peri.Reset()`), the timer returns to the `.repl` `frequency` (250 MHz in the stock platform, `repl` L430-436 `timer5 ... frequency: 250000000`), NOT to the RCC's current figure.
- `ResetRegisteredPeripheral` does not call `PropagateClock()`. Agrees with 4400f L19, L99-L103 ("then reset to 250 MHz").
- A `machine Reset` order effect (4400f P14): RCC `Reset()` pushes HSI, each timer's `Reset()` restores the constructor value.
- Any later write that changes CR, CFGR, D1CFGR or D2CFGR would push again; with ChibiOS that does not happen after the system-timer reset [inferred from 4400f].
- Consequence for a .repl: to make ArduPilot's `TIM5` count at 200 MHz after the reset, `timer5: frequency: 200000000` must be declared. (4400f L98 says this works, "reads 200 MHz at load".)

### A8. Agreements and disagreements with 4400f FEEDBACK

Agree (verified against the source):
- §1.1 sys_ck sources, HSE default 8 MHz, FRACEN tag; four propagating registers and their line numbers; peripheral reset does not propagate; TIM5 pushed to 200 then reset to 250.
- F2 / D2CCIP2R reads 0. F4: SysTick 64 MHz from construction.
- USARTs cannot follow the RCC (they are not retunable).

Differences or additions (not contradictions):
- 4400f says SysTick follows the RCC (P12, F4). The exact formula is `sys_ck / D1CPRE` with no HPRE (L1398). With ArduPilot's HPRE /2 that is 400 MHz, not 200 MHz [inferred as a modelling inaccuracy, not checked on silicon].
- 4400f §1.1 says "the constructor runs Reset() before any peripheral registers, so at load nothing is pushed". True for peripherals, but SysTick is passed in the constructor and IS set (4400f's own F4 measures 64 MHz). The two statements are consistent only if "nothing" means "no peripheral".
- New: PWR is a Python stub (`repl` L728-745); RDY bits are immediate for HSE/PLL1/2/3/HSI/CSI/HSI48; SW is not validated against source readiness; PLL2/PLL3 are not computed at all.
- New: `Ref1ClockMinimumFrequency && ...Maximum` check is dead code (L122).

---------------------------------------------------------------------------------------------------
## PART B. ArduPilot Copter-4.7.2 `stm32h7_mcuconf.h`

### B0. Provenance

- `/home/forge/.cache/aet-4210/ardupilot/libraries/AP_HAL_ChibiOS/hwdef/common/stm32h7_mcuconf.h` sha256 = `67f48e5f34466b6da633a54b3321dac44f7c7d4d6cf6065fad84a7059201a426`. **MATCHES** the expected value. The `$IMG` copy has the same sha256.
- Checkout: detached HEAD `fb773567896014da007bca58358704d4a0d8516f`, commit subject `Plane: version to 4.7.2` (Sun Oct 11 10:05:22 2026). Tags at HEAD include `Copter-4.7.2`, `Plane-4.7.2`, `AP_Periph-4.7.2`, `ArduCopter-stable`. So it is Copter-4.7.2. The checkout is a sparse checkout ("6% of tracked files present"). The `$IMG` tree has the same HEAD (`fb773567...`).
- ChibiOS submodule pin: `d103983f4362e5506bcf7062ccacf345fc0c0859` (`git ls-tree HEAD modules/ChibiOS`; `$IMG/modules/ChibiOS` HEAD reads the same).

### B1. STM32_HSECLK branches (mcuconf L113-166 DIVM, L168-253 DIVN/P/Q/R)

Global settings for all H7 branches (L84-110 and L256-300): `STM32_VOS = STM32_VOS_SCALE1` (L110); `STM32_SW = STM32_SW_PLL1_P_CK` (L287); `STM32_D1CPRE_DIV1`, `STM32_D1HPRE_DIV2` (L290), `STM32_D1PPRE3_DIV2`, `STM32_D2PPRE1_DIV2` (L292), `STM32_D2PPRE2_DIV2`, `STM32_D3PPRE4_DIV2` (L288-294); `STM32_TIMPRE_ENABLE TRUE` (L312); `STM32_PLL1/2/3_ENABLED TRUE`; PLL3_P `FALSE` (L278); `STM32_PLL1_FRACN_VALUE 0`, `STM32_PLL2_FRACN_VALUE 0`, `STM32_PLL3_FRACN_VALUE 0` (L269, 275, 281 area). No fractional settings anywhere. `STM32_PLLSRC` defaults to `STM32_PLLSRC_HSE_CK` (L259-261). `STM32_HSI48_ENABLED TRUE`, `STM32_CSI_ENABLED FALSE`, `STM32_LSE_ENABLED FALSE`, `STM32_HSIDIV_DIV1` (L98-105).

Where PLL values are `M / N / P / Q / R` (N is the value, the register is N-1): VCO = ref * N, outputs = VCO / P,Q,R.

| HSECLK | mcuconf lines | PLL1 (M N P Q R) -> VCO, P, Q | PLL2 -> VCO; P, Q, R | PLL3 -> VCO; Q, R | sys_ck |
|---|---|---|---|---|---|
| 0 (HSI) | L113-122, L168-183 | 4 / 50 / 2 / 10 / 2: HSI 64/4=16 -> 800; P 400, Q 80 | 8 / 45 / 2 / 5 / 8: 64/8=8 -> 360; P 180, Q 72, R 45 | 4 / 15 / - / 5 / 8: 16 -> 240; Q 48, R 30 | 400 |
| 8 MHz | L124-130, L184-208 | 1 / 100 / 2 / 10 / 2: 8 -> 800; P 400, Q 80 | 1 / 75 / 3 / 6 / 3: 8 -> 600; P 200, Q 100, R 200 | 2 / 72 / - / 6 / 9: 4 -> 288; Q 48, R 32 | 400 |
| 16 MHz | L132-138, same PLL values | 2 / 100 / ...: 8 -> 800 | 2 / 75 ...: 8 -> 600 | 4 / 72 ...: 4 -> 288 | 400 |
| 24 MHz | L140-146, L209-235 | 3 / 100 / 2 / 10 / 2: 8 -> 800; P 400 | 2 / 50 / 3 / 6 / 3: 12 -> 600; P 200, Q 100, R 200 | 6 / 72 / - / 6 / 9: 4 -> 288; Q 48, R 32 | 400 |
| 25 MHz | L148-154, L236-253 | 2 / 64 / 2 / 10 / 2: 12.5 -> 800; P 400, Q 80 | 5 / 120 / 3 / 6 / 3: 5 -> 600; P 200, Q 100, R 200 | 5 / 48 / - / 5 / 8: 5 -> 240; Q 48, R 30 | 400 |
| 32 MHz | L156-162, common 8/16/32 | 4 / 100 ...: 8 -> 800 | 4 / 75 ...: 8 -> 600 | 8 / 72 ...: 4 -> 288 | 400 |
| other | L164-166 | `#error "Unsupported HSE clock"` | | | build fails |

Verbatim anchors:
- L113: `#if STM32_HSECLK == 0U`; L114: `// no crystal, this gives 400MHz system clock`.
- L124-125: `#elif STM32_HSECLK == 8000000U` / `// this gives 400MHz system clock`; L127 `#define STM32_HSI_ENABLED FALSE`; L128 `#define STM32_PLL1_DIVM_VALUE 1`.
- L148-154 (25 MHz): `STM32_PLL1_DIVM_VALUE 2`, `STM32_PLL2_DIVM_VALUE 5`, `STM32_PLL3_DIVM_VALUE 5`.
- L165-166: `#else` / `#error "Unsupported HSE clock"`.
- L184-196 8/16/32 MHz branch: `// common clock tree for multiples of 8MHz crystals`, `STM32_PLL1_DIVN_VALUE 100`, `STM32_PLL1_DIVQ_VALUE 10`.
- `HAL_CUSTOM_MCU_CLOCKRATE` special case: only for 8/16/32 MHz (L186-192) and 24 MHz (L210-219): when `== 480000000`, `STM32_PLL1_DIVN_VALUE 120` and `STM32_PLL1_DIVQ_VALUE 12` (960 MHz VCO -> 480 MHz sys_ck, HCLK 240, timers 240). The 24 MHz branch also allows 200 MHz (N=50, Q=5). The 25 MHz branch: `#ifdef HAL_CUSTOM_MCU_CLOCKRATE` -> `#error "Unable to configure custom clockrate"` (L237-239). Unset (default Copter build) -> 400 MHz.
- The `STM32_ENFORCE_H7_REV_XY` define (L22-24) is set when no custom clock rate or <= 400 MHz.
- No fractional PLL anywhere.

HSI contrast branch, quotes (L113-122): `#if STM32_HSECLK == 0U` / `// no crystal, this gives 400MHz system clock` / `#define STM32_HSE_ENABLED FALSE` / `#define STM32_HSI_ENABLED TRUE` / `#define STM32_PLL1_DIVM_VALUE 4` / `#define STM32_PLL2_DIVM_VALUE 8` / `#define STM32_PLL3_DIVM_VALUE 4` / `#define STM32_PLLSRC STM32_PLLSRC_HSI_CK` / `#define STM32_MCO1SEL STM32_MCO1SEL_HSI_CK` / `#define STM32_CKPERSEL STM32_CKPERSEL_HSI_CK`.
- This is the only branch with PLL2 P = 2, Q = 5, R = 8, N = 45 (VCO 360 MHz) and `CKPERSEL = HSI`.

Timer clock in every branch (default 400 MHz sys_ck): sys_ck 400 / D1CPRE 1 / D1HPRE 2 = HCLK 200 MHz. D2PPRE1 = D2PPRE2 = /2 -> pclk 100 MHz; with Table 57 the timer kernel clock is 2 x pclk = 200 MHz (TIMPRE=TRUE does not change it at /2). [inferred, from L290-294 + Table 57; this matches the Renode lookup A3]. In the custom 480 branch: sys 480, HCLK 240, timers 240.

### B2. Kernel selects per branch

The kernel-clock selects are NOT inside the HSECLK branches. They are set once for every HSE value (and for the HSI branch), with these exceptions:
- `STM32_CKPERSEL`: `#ifndef STM32_CKPERSEL` -> `STM32_CKPERSEL_HSE_CK` (L317-319); the HSI branch pre-sets `STM32_CKPERSEL_HSI_CK` (L122).
- `STM32_PLLSRC`: HSE default (L259-261); the HSI branch pre-sets HSI (L120).
- `STM32_MCO1SEL`: HSE_CK default (L300-302), HSI_CK in the HSI branch (L121).
- `STM32_QSPISEL`: `#ifndef` default `STM32_QSPISEL_PLL2_R_CK` (L321-323); `STM32_QSPISEL_HCLK` can override (L324-326).

Fixed for all (L330-352):
- L332 `STM32_SPI45SEL STM32_SPI45SEL_PLL2_Q_CK`
- L333 `STM32_SPI123SEL STM32_SPI123SEL_PLL1_Q_CK`
- L340 `STM32_USBSEL STM32_USBSEL_PLL3_Q_CK`
- L341 `STM32_I2C123SEL STM32_I2C123SEL_PLL3_R_CK`
- L342 `STM32_RNGSEL STM32_RNGSEL_HSI48_CK`
- L343 `STM32_USART16SEL STM32_USART16SEL_PLL2_Q_CK`
- L344 `STM32_USART234578SEL STM32_USART234578SEL_PLL2_Q_CK`
- L345 `STM32_SPI6SEL STM32_SPI6SEL_PLL2_Q_CK`
- L348 `STM32_ADCSEL STM32_ADCSEL_PLL3_R_CK`
- L351 `STM32_I2C4SEL STM32_I2C4SEL_PLL3_R_CK`
- L352 `STM32_LPUART1SEL STM32_LPUART1SEL_PCLK4`

Resulting frequencies (computed from B1) [inferred arithmetic]:

| clock | HSI | 8/16/32 MHz | 24 MHz | 25 MHz |
|---|---|---|---|---|
| SPI123 (PLL1_Q) | 80 | 80 | 80 | 80 |
| SPI45, SPI6 (PLL2_Q) | 72 | 100 | 100 | 100 |
| USART1/6 and USART2/3/4/5/7/8 (PLL2_Q) | 72 | 100 | 100 | 100 |
| I2C123, I2C4, ADC (PLL3_R) | 30 | 32 | 32 | 30 |
| USB (PLL3_Q) | 48 | 48 | 48 | 48 |
| CKPER | HSI 64 | HSE (8/16/32) | HSE 24 | HSE 25 |
| timers | 200 | 200 | 200 | 200 |

- So timers stay at 200 MHz in every branch of the default build. The USART kernel clock is 100 MHz with any HSE crystal; **72 MHz applies only to the HSI branch** (PLL2: 8 MHz ref x 45 = 360 / Q5). The WO's "72 MHz" is the HSI figure, not the HSE one.
- In the 480 MHz custom build, only PLL1 changes: SPI123 = 960/12 = 80 (unchanged), the timers 240.
- 4400f already measured ArduPilot's write `RCC_D2CCIP2R = 0xE01009` (`USART16SEL` and `USART234578SEL` = `PLL2_Q`, `I2C123SEL` = `PLL3_R`) and its `usart1: frequency: 72000000` declaration (FEEDBACK L98). [Cross-check needed: if that run used an 8 MHz HSE build, the USART kernel clock was actually 100 MHz and the 72 MHz declaration belongs to the HSI branch. I did not determine which hwdef/OSCILLATOR_HZ the 4400f run used.]

### B3. HSE bypass, OSCILLATOR_HZ mapping, HSE failure behaviour

OSCILLATOR_HZ -> STM32_HSECLK:
- `$IMG/libraries/AP_HAL_ChibiOS/hwdef/scripts/chibios_hwdef.py` L885 (`write_mcu_config`): `f.write('#define STM32_HSECLK %sU\n\n' % self.get_config('OSCILLATOR_HZ'))`. The sparse checkout contains this file too (same line).
- OSCILLATOR_HZ is a mandatory `get_config` key (no default), so `OSCILLATOR_HZ 0` gives the HSI branch.

STM32_HSE_BYPASS:
- It is set by the hwdef keyword `define STM32_HSE_BYPASS` (and also by a bare line starting `STM32_`): `chibios_hwdef.py` L1005-1011 `if d.startswith('STM32_'): f.write('#define %s\n' % d)` and `if d.startswith('define '): ... f.write('#define %s\n' % d[7:])`.
- `mcuconf.h` does NOT mention HSE_BYPASS (grep: no hit). It is consumed by ChibiOS: `$IMG/modules/ChibiOS/os/hal/ports/STM32/STM32H7xx/hal_lld.c` L293-297:
  ```
  #if STM32_HSE_ENABLED == TRUE
  #if defined(STM32_HSE_BYPASS)
    RCC->CR |= RCC_CR_HSEON | RCC_CR_HSEBYP;
  #else
    RCC->CR |= RCC_CR_HSEON;
  ```
  HSEON and HSEBYP are set in ONE read-modify-write of CR.
- Allowed range check (`hal_lld_type1.h` L102-117): `STM32_HSECLK_MAX 48000000`, `STM32_HSECLK_BYP_MAX 50000000`, `STM32_HSECLK_MIN 4000000`, `STM32_HSECLK_BYP_MIN 4000000`.
- `STM32_HSE_BYPASS` requires 4-50 MHz; without it 4-48 MHz.

HSERDY wait: UNBOUNDED.
- `hal_lld.c` L299-300:
  ```
  while ((RCC->CR & RCC_CR_HSERDY) == 0)
    ;                           /* Waits until HSE is stable.               */
  ```
  There is no timeout, no fallback to HSI, no error path. (Also `stm32_hse.inc` L98-100 has the same unbounded loop for the LLD variant used by other families: `while ((RCC->CR & RCC_CR_HSERDY) == 0U) { /* Waiting for HSE activation.*/ }`.) The init_pwr waits also carry `/* CHTODO timeout handling.*/` (L108, L128). So a dead HSE (no crystal, or a Nucleo without bypass when the ST-LINK MCO is not feeding it) hangs ChibiOS at boot, in `__early_init`/`stm32_clock_init` before any ArduPilot code. HAL checks `STM32_HSE_ENABLED` only.
- Same for PLL ready: `hal_lld.c` L398 `while ((RCC->CR & rdymask) != rdymask)`. SW: L465 `while ((RCC->CFGR & RCC_CFGR_SWS_Msk) != (STM32_SW << RCC_CFGR_SWS_Pos))`. FLASH latency: L457. All unbounded.
- CSS is not enabled by this code: `grep -n CSS hal_lld.c` returns no line.

### B4. Nucleo H7 hwdefs (from `$IMG`, same commit)

Boards present: `NucleoH743`, `NucleoH753ZI`, `NucleoH755` (all under `$IMG/libraries/AP_HAL_ChibiOS/hwdef/`). In the sparse checkout they are NOT present.

NucleoH743 `hwdef.dat` (L7-12):
```
MCU STM32H7xx STM32H743xx

# crystal frequency
OSCILLATOR_HZ 8000000

define STM32_HSE_BYPASS
```
- `NucleoH743/hwdef-bl.dat` L8, L10: same, `OSCILLATOR_HZ 8000000` and `define STM32_HSE_BYPASS`.

NucleoH753ZI `hwdef.dat` (L68-74):
```
MCU STM32H7xx STM32H753xx

# board ID for firmware load
APJ_BOARD_ID AP_HW_NUCLEO_H753ZI

# crystal frequency
OSCILLATOR_HZ 8000000
```
- No `STM32_HSE_BYPASS` anywhere in `NucleoH753ZI/hwdef.dat` or `hwdef-bl.dat` (grep `BYPASS` over the two files: no hit; its only bypass hits in the tree are `NucleoH743`, `3DRControlZeroG`, `VM-L431`). Also L77 `STM32_ST_USE_TIMER 12` and L78 `define CH_CFG_ST_RESOLUTION 16`: ChibiOS's system timer is TIM12 (a 16-bit APB1 timer, not TIM5), unlike the other H7 boards (`NucleoH755` uses `STM32_ST_USE_TIMER 5`, L12).
- `NucleoH753ZI/hwdef-bl.dat` L11: `OSCILLATOR_HZ 8000000`.
- Consequence [inferred]: on silicon, the H753ZI build without bypass wants an 8 MHz crystal on OSC_IN/OSC_OUT; the Nucleo-144's HSE is driven by ST-LINK MCO (needs bypass) unless solder bridges are changed. That is a hardware matter for the WO; the model is not affected: Renode's RDY is immediate either way (A1), and HSEBYP has no effect (A2). Whether the HSE flavour changes the silicon behaviour (RDY hang) is exactly what the twin cannot show.

NucleoH755 `hwdef.dat`: `OSCILLATOR_HZ 8000000` (L10), `define SMPS_PWR` (L15), no bypass. Not an H753; for contrast only.

### B5. Implications for a spike (short)

- Stock Renode defaults (HSE 8 MHz) match every ArduPilot Nucleo H7 board's `OSCILLATOR_HZ`; a clock-tree mismatch only arises for 16/24/25/32 MHz crystals, not for Nucleo.
- Timers: Renode gives 200 MHz for the 8 MHz HSE (identical to the firmware's own tree), but `timerN: frequency:` in the .repl is what the timer reads after any peripheral reset, so the declared value needs to be 200 MHz for ArduPilot.
- USART: the model cannot know the kernel clock; declare it. For an 8 MHz HSE build the correct figure is 100 MHz (PLL2_Q), not 72 MHz (B2).
- A firmware hang on HSERDY / PLL RDY / SWS cannot be reproduced on the model (A1); silicon bring-up failure paths are out of the twin's reach.
