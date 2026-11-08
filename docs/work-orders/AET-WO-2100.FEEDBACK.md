# AET-WO-2100 — FEEDBACK

**Branch:** `wo/aet-2100-twin-boot` · **Base:** `7fab0e1` (the merge of PR #55), as the kickoff set it · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace (x86_64, kernel 7.0.0-38, 20 CPUs). One Sonnet subagent measured S2's `cortex-m-rt` option, and the lead reproduced its sizes, licences, `unsafe` counts and boot before citing them. · **Cadence:** spike, stopped and ruled on PR #56 (R1 to R6, in `~/.cache/aet-2100-ruling.md` and on the PR), then PR A (#57, the image's thumb target, merged; the `IMAGE` bump is #59, `89a3514`), then the build, below. `origin/main` at `89a3514` is merged in with a merge commit, `bc16a9b`.

**The kits** (kickoff item 2):
- **Renode 1.17.0:** `~/.cache/aet-0300a-renode/x/renode` (`--version`: `1.17.0+20260907gitf1dd1b4af`, .NET 8.0.12). `~/.cache/aet-0300a-renode/bin/renode` is a symlink to it. The kickoff names "AET-WO-1600's kit", but `~/.cache/aet-1600-adv/tools/renode` is a symlink to that work order's `probe.py` shim, not Renode. The real Renode is 0300a's.
- **Thumb toolchain:** `RUSTUP_HOME=~/.cache/aet-1400-spike/rustup`, with `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and `thumbv7em-none-eabihf` added. It links with the bundled `rust-lld` (the ELF's `.comment` names LLD 23.1.1). `rust-objcopy` needs `LD_LIBRARY_PATH=$RUSTUP_HOME/toolchains/1.99.1-x86_64-unknown-linux-gnu/lib`.
- **Probes and outputs:** the probes as they stand at commit `459005f`; `spike/2100/out/s3/` as re-taken on the committed ELF in the commit after `bd3b99a` (the PR head); the rest at `459005f`, unchanged since. Scratch measurements, including the shell loops that drove the boots and the subagent's raw files, are outside the repository in `~/.cache/aet-2100-spike/` (Furnace).

**§7, held:**
- nothing under `container/` or `.github/`;
- no `unsafe` in a workspace crate (every spike crate declares its own `[workspace]`);
- no boot verdict on Renode's exit code;
- no test skipped, ignored or weakened;
- no `sudo`, system package or secret.

One licence term needs a ruling: `unicode-ident`'s `Unicode-3.0`, under S2 option 2 (F8).

Entry schema: `finding · where · severity · type · recommendation · decision-needed`.

## What the spike added (the build deletes all of it)

- `spike/2100/emit/`: the S1 probe emitter, with its own `[workspace]`. It reads a `SolvedTopology`, Renode's `platforms/cpus` include chain and the hash-checked ST header through `aethelgard-svd-hal`'s public `cmsis` API. It writes a `.repl` to stdout and one `PROVENANCE` line per fact to stderr.
- `spike/2100/fw-bare/`: the S2 firmware, option 1. It has a hand-written vector table and `link.x`, and no dependency but `boards/sentinel-01`. The `uart` feature adds S3's UART candidate, around the board crate.
- `spike/2100/fw-rt/`: the S2 firmware, option 2. Same `main` body, with `cortex-m-rt` 0.7.7 (`#[entry]`, its `link.x`, and a `memory.x`).
- `spike/2100/run/*.resc`: the Renode scripts:
  - `s3-image` reads every candidate before and after 10 virtual ms;
  - `s3-image-cold` does the same from silicon reset values of the RCC enables;
  - `s3-gate` tests RCC gating;
  - `s3-uart` captures USART3 to a file;
  - `s4-boot` is one boot for a fixed virtual time.
- `spike/2100/out/`: the inputs and raw outputs cited below.
- `spike/2100/.gitignore`: the spike crates' `target/`.

## Spike

The spike's files are cited as they stood at `0dacab8`, PR #56's spike head. The build deleted `spike/` (`d799405`).

Unless a row says otherwise, every boot below runs `fw-bare` release, sha256 `bfd902aede164c10…` (Furnace build, the committed source), on `spike/2100/out/sentinel-01.repl`. The UART candidate is `--features uart`, sha256 `fd44ae463359ffd5…`. The S3 outputs in `out/s3/` were re-taken on these two ELFs after the last source edit. Renode is driven as `renode --disable-gui --console --plain -e "<commands>" </dev/null` with `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 DOTNET_EnableDiagnostics=0`.

### S1 The platform description

**Emission.** From the repository root:
- the solved topology: `aethelgard solve --file examples/sentinel-01-minimal.json > spike/2100/out/solved.json`, specification sha256 `acd60081…`;
- the platform: `spike/2100/emit/target/debug/aet-2100-s1-emit spike/2100/out/solved.json ~/.cache/aet-0300a-renode/x crates/aethelgard-svd-hal/data/st-cmsis-device-h7/stm32h753xx.h`.

It ran twice, and `cmp` found the two outputs byte-identical (sha256 `5869fabe92a6895b0daafaa6a21675d3b4935bb62f554e7e4ede937cbedd6f26`). The whole file (`spike/2100/out/sentinel-01.repl`):

```
using "platforms/cpus/stm32h753.repl"

spi1: SPI.STM32H7_SPI @ {
        sysbus 0x40013000;
        rcc RegistrationPeripheralAlias.SPI1
    }
    IRQ -> nvic@35
```

The emitter adds only what the topology solved and Renode lacks. It never redeclares a stock instance: §2.9 counts a duplicate registration among the broken platforms.

**Loaded through today's `simulate`.** `PATH=~/.cache/aet-0300a-renode/bin:$PATH aethelgard simulate --platform spike/2100/out/sentinel-01.repl` gave exit 0, `"verdict":"pass"`, `cpus: ["cpu"]`, 92 peripherals, no findings, under Landlock ABI 8 (`spike/2100/out/s1-simulate.json`), in 1.8 s wall. A platform of only `using "platforms/cpus/stm32h753.repl"` (`stock-h753.repl`) lists 91 (`s1-simulate-stock.json`). The set difference of the two listings is exactly `{(spi1, STM32H7_SPI)}`, with nothing removed. Every solved peripheral is listed:
- `gpioPortA` (`STM32_GPIOPort`);
- `gpioPortB` (`STM32_GPIOPort`);
- `i2c1` (`STM32F7_I2C`);
- `spi1` (`STM32H7_SPI`).

**Provenance** (`spike/2100/out/provenance.txt`). Each Renode file is hash-recorded (`stm32h753.repl` `13c5571e…`, `stm32h743.repl` `86b2c2c5…`, `stm32h7.repl` `b85cb4af…`). For a stock instance, the probe checks that Renode's `sysbus` address equals the header's, and refuses otherwise.

| Fact | Value | From |
|---|---|---|
| `gpioPortA`, `STM32_GPIOPort` | `0x58020000` | name and model: `stm32h7.repl:178`; address: `GPIOA_BASE` in `stm32h753xx.h`, equal to Renode's |
| `gpioPortB`, `STM32_GPIOPort` | `0x58020400` | `stm32h7.repl:169`; `GPIOB_BASE`, equal |
| `i2c1`, `STM32F7_I2C` | `0x40005400`; event `exti@22` → `nvic@31`, error `nvic@32` | `stm32h7.repl:355` and the EXTI map at `:19`; `I2C1_BASE`; `I2C1_EV_IRQn = 31` and `I2C1_ER_IRQn = 32` (header lines 92–93) agree |
| `spi1`, `STM32H7_SPI`, **added** | `0x40013000`, `IRQ -> nvic@35` | name: Renode's lowercase ST name, as for `spi4`; model: `stm32h7.repl:193`'s `spi4`; address: `SPI1_BASE` = `D2_APB2PERIPH_BASE + 0x3000`; interrupt: `SPI1_IRQn = 35` (line 96) |
| `rcc RegistrationPeripheralAlias.SPI1` | ST's instance name | measured: Renode accepts `SPI1` and refuses `SPI9` with `Error E21: Could not find any suitable constructor for this registration point` |
| the `IRQ` connector's name | `IRQ` | measured: `sysbus.spi4`'s monitor help lists the properties `GPIO DMARecieve` and `GPIO IRQ` |

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F1. **`simulate --platform @platforms/cpus/stm32h753.repl` fails today.** `stm32h753.repl` includes `"./stm32h743.repl"`, which is file-relative, and `simulate` judges a copy in its scratch directory: `Error E36: Using './stm32h743.repl' resolved as '/tmp/aethelgard-…/tool/stm32h743.repl' does not exist` (exit 1). The root-relative form `using "platforms/cpus/…"` that `stm32h743.repl` itself uses resolves | `simulate`'s `@` input; the emitter | medium | measurement | The emitter writes only root-relative includes (the probe does). The `@` form stays usable for files with no file-relative include, such as `nucleo_h753zi.repl`, which the README cites | no; a note for the README at close-out |
| F2. Renode's own `spi4` leaves its `IRQ` unconnected (`SPI4_IRQn = 84` in the header), so stock SPI raises no interrupt. The emitter wires `IRQ` for the instances it adds | S1 | low | measurement | Wire `IRQ` for added instances, as the probe does. Leave `spi4` alone: the topology does not use it | no |
| F3. `aethelgard-svd-hal::cmsis` reads `#define`s only, so `<P>_IRQn` (an `enum` member) is not in its facts. The probe hash-checks the header and reads the one enum line itself | S1 | low | build input | The build adds one refusing shape to the `cmsis` parse, `NAME_IRQn = <decimal>,`, inside `IRQn_Type` | no |
| F4. DMA request lines (`spi4`'s `DMARecieve -> dmamux1@83`) do not trace to the pinned header: ST keeps DMAMUX request ids in the HAL headers, which are not vendored. Booting needs none | S1 | low | provenance | Emit no DMA line until a work order needs DMA and vendors its source | no |
| F5. `simulate`'s listing carries name and model, not interrupt wiring, so a wrong `nvic@N` would still list. The `peripherals` listing cannot catch it | S1, §5.3 | low | test design | The build's emission test checks each added line's numbers against the header, independently of the emitter, as `provenance.txt` does | no |

### S2 The firmware

Both options run the same `main`:
1. `Board::take()`;
2. enable GPIOA, GPIOB, I2C1 and SPI1;
3. put `PA0` into output, and `PA5`, `PA6` and `PA7` into SPI1 AF5, and `PB6` and `PB7` into I2C1 AF4;
4. `I2c1::init::<Fast400k>` and `Spi1::init`;
5. `FLASH_CS` low, one SPI byte, `FLASH_CS` high;
6. write `BOOT_MAGIC = 0x2100_B007` to `AETHELGARD_BOOT`, the first word of RAM at `0x20000000` in a `NOLOAD` section;
7. spin.

It uses only the board crate's API for hardware, and both options boot to the same values (S3). Commands: `RUSTUP_HOME=~/.cache/aet-1400-spike/rustup cargo build --offline --release` in each crate (`.cargo/config.toml` sets the target); then `size -A`; `rust-objcopy -O binary`; `cargo tree -e normal,build --target thumbv7em-none-eabihf -f '{p} {l}'`; and `grep -nwH unsafe <files> | grep -vcE '^[^:]*:[0-9]+:\s*//'`.

| | Option 1: `fw-bare` (hand-written) | Option 2: `fw-rt` (`cortex-m-rt` 0.7.7) |
|---|---|---|
| Crates (normal and build, thumb) | 2: the firmware and the board crate | 8: adds `cortex-m-rt`, `cortex-m-rt-macros` (proc-macro), `proc-macro2` 1.0.107, `quote` 1.0.47, `syn` 3.0.9, `unicode-ident` 1.0.27 |
| Licences | all `MIT OR Apache-2.0` | all `MIT OR Apache-2.0`, except `unicode-ident`: `(MIT OR Apache-2.0) AND Unicode-3.0`. The workspace's `Cargo.lock` already holds `proc-macro2`, `quote`, `syn` 3.0.9 and `unicode-ident` at these versions; `cortex-m-rt*` is new |
| Offline | **yes, always**: nothing to fetch | **only with the crates present.** With an empty `CARGO_HOME`, `--offline` fails: `no matching package named 'cortex-m-rt' found`. Vendored (`cargo vendor`: 282 files, 3,310,247 B), it builds with an empty `CARGO_HOME`. In the tools image, offline fails the same way, and with the network it builds in 2.2 s |
| `unsafe` (non-comment lines) | 12 in `src/main.rs`: the attributes on the vector table and the reserved word, the `extern` block, `Reset`, the `.data`/`.bss` loops, `fault`, `Mmio::new`, the magic write | 4 in `src/main.rs`; 38 in `cortex-m-rt/src`; 20 in `cortex-m-rt-macros/src`. All are outside the workspace |
| Assembly | none | one `global_asm!` block, `Reset`: `bl __pre_init`, zero `.bss`, copy `.data`, **enable the FPU** (CPACR `0xE000ED88`, for `eabihf`), `bl main` |
| ELF file | 132,936 B | 135,212 B |
| Flash image (`objcopy -O binary`) | **1,316 B**: vector table 64 + `.text` 1,232 + `.rodata` 20 | **2,308 B**: vector table 1,024 + `.text` 1,264 + `.rodata` 20 |
| RAM | 8 B: boot word 4 + `.bss` 4 (`TAKEN`) | 8 B, the same |
| Cold release build on Furnace (`cargo clean` first) | 0.13 s | 1.66 s |
| Boot (S3 script) | `0x2100B007` at `0x20000000`, 1,000,000 instructions, MODER `0xA801`, ODR `0x1` | the same; the park loop is at `0x8000878`, not `0x80004ae` |

- **How Renode starts the core.** Before `start`, `cpu PC` and `cpu SP` read 0. On start it logs `Guessing VectorTableOffset value to be 0x8000000` and `Setting initial values: PC = 0x8000041, SP = 0x20020000`. That is the table's reset vector and the linker script's initial SP, with no alias at address 0 needed.
- **How fast the fixed point comes.** It is reached after more than 1,000 and at most 2,000 instructions: the word is 0 at 10 µs virtual and set at 20 µs (100 MIPS).

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F6. Option 1 does `.data`/`.bss` initialisation in Rust before `main`. `cortex-m-rt` does it in `global_asm!`, as its docs advise for `__pre_init`. Option 1 reads no static before the loops end. Whether a Rust reset handler is sound under the language's rules was not measured | S2 | low | design | If option 1 is ruled, generate the reset handler as a few lines of `global_asm!` (zero `.bss`, copy `.data`, branch to `main`). That stays dependency-free, and the `unsafe` stays outside the workspace | with the S2 ruling |
| F7. Option 1 does not enable the FPU, and `cortex-m-rt` does. On `thumbv7em-none-eabihf`, the first floating-point instruction faults without CPACR set. The probe firmware has none, but P2's sensor loops will | S2 | medium | correctness | If option 1, the generated reset handler sets CPACR CP10/CP11 as `cortex-m-rt` does | with the S2 ruling |
| F8. **Option 2 brings in `Unicode-3.0`** (`unicode-ident`, through the macros crate), which is not `MIT OR Apache-2.0`. It is already in the workspace's build graph at the same version, through the existing proc-macro dependencies, so `RUL-11` admits it in practice without naming it | S2 | medium | licence (`RUL-11`) | Prefer option 1, which has no such term. If option 2 is ruled, name `Unicode-3.0` beside the dependency inventory in `RUL-11` | **yes**, with S2 |
| F9. **Recommendation: option 1.** The firmware crate is dependency-free like the board crate (`RUL-04` as amended), always builds offline, and is 1.3 KB of flash against 2.3 KB, with 64 B of vectors against 1,024. Its runtime is about 70 lines the generator can own, with F6 and F7 folded in. Option 2 buys a maintained runtime at the cost of a vendored or fetched 282-file tree in every offline build, plus F8 | S2 | — | ruling input | Option 1, generated beside the board crate | **yes** |

### S3 The observable point

Every candidate is read before `start` and after `emulation RunFor "0.01"`, by `run/s3-image.resc` (`out/s3/s3-image.out`). `PA0` is watched through a test-only `Miscellaneous.LED` wired to `gpioPortA@0`, added with `LoadPlatformDescriptionFromString` and never emitted.

| Candidate | Before → after | Deterministic | Expected value from (`INV-07`) | Satisfied by reset values? | Cost |
|---|---|---|---|---|---|
| **Reserved RAM word** (`sysbus ReadDoubleWord 0x20000000`) | `0x00000000` → `0x2100B007` | yes: 49 parsed boots identical (S4) | a CLI constant and a linker-script convention, neither from the ELF nor from the specification | no: RAM reads 0, and Renode writes the `NOLOAD` segment's 4 bytes as zeros at load (`Loading block of 4 bytes length at 0x20000000`) | none: plain memory, no board API |
| UART line (USART3 to `CreateFileBackend`, `--features uart`) | nothing → `AET-2100 BOOT\n`, 14 B (`out/s3/uart.log`) | yes: `cmp` of two runs | the firmware's own text | no | **raw USART3 access around the board crate**: the solved topology has no UART, so it breaks §1.2 ("uses only the board crate's API") and bypasses `INV-02` |
| GPIO on a solved pin (`PA0`, `FLASH_CS`) | LED `False` → `True`, ODR `0x0` → `0x1` | yes | the specification (`gpio.FLASH_CS = PA0`) | partly: `into_output` already drives it high, so it shows that `PA0` was configured, not that the fixed point was reached. It is one bit, and its other value is the reset value | none |
| Register image after N ms | GPIOA MODER `0` → `0x0000A801`, AFRL `0` → `0x55500000`; GPIOB MODER `0` → `0x0000A000`, AFRL `0` → `0x44000000`; I2C1 CR1 `0` → `1`; SPI1 CR1 `0` → `1` | yes (S4) | the specification: each solved pin's mode and AF number from `SolvedTopology` | **partly, see F10**: the RCC enables and SPI1 `CFG2` read the configured values before the firmware runs, and `OTYPER` and `TIMINGR` are never stored | none |

**Witnesses for a boot that did not happen** (`out/s3/load-junk.out`, `load-x86.out`, `load-unmapped.out`; `logLevel 3` is set only after `RunFor`, so every load and run warning is in the file; §5.1):
- A random 4 KiB file: `sysbus LoadELF` prints `There was an error executing command 'sysbus LoadELF …' / Error while loading ELF: Could not load ELF from path`.
- **An x86-64 ELF (`/bin/true`), and the ARM firmware relinked to `0x0A000000` (unmapped):** both load with only `[WARNING]` lines and no error string.
  - The x86 file: `WriteDoubleWord to non existing peripheral`.
  - The relinked firmware: `Tried to access bytes at non-existing peripheral`, then `Tried to set VTOR address at 0xA000000 which is not mapped … Aborted`.
  - Both runs exit 0. After `RunFor "0.01"` the word is `0x00000000`, `cpu PC` is `0xeffffffe`, and `cpu ExecutedInstructions` is **0**.
- An unquoted `echo OBS` inside `-e` is itself an error (`No such emulation element: OBS`), and Renode runs no later command, still exiting 0.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F10. **Renode's RCC comes out of reset with every solved clock already enabled.** `RCC_AHB4ENR` reads `0x032807FF`, `APB1LENR` `0xC0FE03FF` and `APB2ENR` `0x00073033` before the firmware runs (GPIOA and GPIOB, I2C1 and SPI1 bits set). Renode does gate: with `AHB4ENR` cleared, a write to GPIOA is dropped with `[WARNING] sysbus: Tried to write a locked peripheral: machine-0.gpioPortA`, and the read returns 0 (`run/s3-gate.resc`, `out/s3/s3-gate.out`). From the stock reset, a firmware that skipped an enable would still work, and the image could not tell. **Measured fix** (`run/s3-image-cold.resc`, `out/s3/s3-image-cold.out`): the boot script writes 0 to the three enable registers before `start`. The firmware then leaves exactly `0x3`, `0x00200000` and `0x00001000`, the solved enables and nothing else, and no locked-peripheral warning fires during the run | S3 | medium | fidelity | The boot always starts from those three registers at 0, and a `locked peripheral` warning during the run is a `fail` finding. The expected enables come from the topology | **yes** |
| F11. **Two solved fields are not observable in Renode 1.17.0.** `OTYPER`: `gpioPortB: Unhandled write to offset 0x4 … Tags: OT6`/`OT7`, and it reads back 0. `I2C1_TIMINGR`'s timing bits: `Unhandled bits … SCLL (0x73), SCLH (0x21), SCLDEL (0x6)`, and it reads back 0. SPI1 `CFG2` reads `0x00400000` at reset, before the firmware sets `MASTER` | S3 | low | fidelity | The register image omits them, and the verdict names them in an `unobservable` list. Kani (P1 clause 5) is what proves those writes | no |
| F12. **Two half-failed loads give no error string**, so the error-string witness alone would not separate them from a boot. What does separate them: the boot word stays 0 and the CPU executes 0 instructions | S3, §5.1 | high | verdict design | Refuse before Renode runs: an ELF that is not ELF32 little-endian `EM_ARM` `ET_EXEC`, or a `PT_LOAD` with file bytes outside the platform's flash and RAM (`FLASH_BANK1_BASE`, `D1_DTCMRAM_BASE` and sizes from the header). Inside the run, require `ExecutedInstructions` = MIPS × virtual time (S4) beside the boot word | **yes**, with S6 |
| F13. **Recommendation:** judge by three witnesses, none of them Renode's exit code. (a) The boot word equals the CLI's constant: the fixed point was reached. (b) The solved register image equals the one computed from `SolvedTopology`, from F10's cold RCC: the board was configured as solved. (c) The instruction count is MIPS × time: the core never halted. In addition, there must be no `There was an error executing command` and no `locked peripheral` warning. Reject the UART (breaks §1.2) and a lone GPIO bit (one of its values is the reset value). The boot word's address and magic are a contract the CLI publishes and the generated linker script follows, not a value read from the ELF | S3 | — | ruling input | As stated | **yes** |

### S4 Headless and deterministic

`run/s4-boot.resc` sets `logLevel 3`, loads, boots for `$time = "0.01"` (10 ms virtual), then reads the boot word, `cpu PC`, `cpu ExecutedInstructions`, seven solved registers, the two `system_flash_ro` words at `0x1FF1E800`/`0x1FF1E804`, `emulation GetTimeSourceInfo`, and `cpu PerformanceInMips`. Raw lines are in `out/s4/`.

| Set | Host condition | Wall per boot | Host time for 10 ms virtual | Observed (word, PC, instructions, image) |
|---|---|---|---|---|
| plain, 5 (`plain.txt`) | idle | 1.83, 1.86, 1.94, 1.95, 2.04 s | 22.1–22.4 ms (`plain-host-time.txt`, a second run of five) | all `0x2100B007 0x80004ae 0xF4240 0xA801 0x1 0x55500000 0xA000 0x44000000 0x1 0x1` |
| loaded, 13 (`loaded.txt`) | `taskset -c 3`, beside four busy loops pinned to core 3 | 11.5–12.4 s | — | 12 identical. One printed no value, and its raw output was not saved. The stress set below reproduces one cause, a `\r` before the marker, but cannot show that it was this run's |
| stress, 24 (`stress.txt`) | six at a time on cores 3–4, beside four busy loops; in a `MemoryMax=32G` scope | — | 0.32–2.47 s | 23 parsed and identical. In `stress-3.raw` the console wrote `\rOBSERVED`, so a line-start match missed it; the values after it are identical (F14) |
| seeded, 3 (`seed.txt`) | idle, `emulation SetSeed 2100` twice, then `7` | ~1.9 s | — | identical |

**Determinism.**
- **Every parsed observation is identical:** 49 of 50 boots. That counts the 37 saved raw outputs re-read with `\r` stripped, plus the unsaved first batches.
- **This holds across a 100-fold spread of host speed** (22 ms to 2.47 s of host time for the same 10 ms virtual). The instruction count is exactly `0xF4240` = 1,000,000 every time, a function of virtual time alone.

**Renode's time framework, measured:**
- `cpu PerformanceInMips` = `0x64` (100), and 10 ms gives 1,000,000 instructions.
- `GetTimeSourceInfo`: `Quantum: 00:00:00.000100000` (100 µs), `Advance immediately: False`, and elapsed virtual time printed to 9 decimal places.
- **Resolution 1 ns:** `RunFor "0.000000001"` gives `Elapsed Virtual Time: 00:00:00.000000001`, and `RunFor "0.0000000015"` also gives `.000000001`. `RunFor "0.00000001"` executes exactly 1 instruction.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F14. **Renode's console output is not line-clean.** (a) A carriage return can precede a monitor line (`stress-3.raw`: `^MOBSERVED`), timing-dependent: 1 in 24 under stress. (b) `[WARNING]` log lines interleave with monitor values *inside a token*: `0x00[05:58:25.9357] [WARNING] … 000000` in `out/s3/s3-image-cold.out`. (c) `logFile @path` adds a file sink but does not take logs off the console: 16 log lines stayed on stdout. Today's `renode::judge` trims spaces, `│` and leading whitespace, so a `\r` before a `│` continuation line would drop that entry. A lost CPU fails closed (`ToolError`); a lost peripheral would fail the build's listing test. This was reasoned from the code, not observed in a listing | S4; `renode.rs` | medium | reader | Strip `\r` before parsing. Read each value between unique sentinels, run the reads under `logLevel 3`, and take warnings from a `logFile`. A value that does not parse is exit 5, never a `fail` | no |
| F15. **Every boot differs in two words unless the seed is fixed.** `stm32h7.repl`'s `system_flash_ro` `init:` writes two words from `RandomGenerator.NextUlong()`, and the seed is random per run (`Pseudorandom Number Generator was created with seed: …`; five plain boots gave five different pairs). With `emulation SetSeed 2100` before `mach create`, both runs read `0x4E78AE41 0x73B9B03A`, and seed 7 reads otherwise. The firmware does not read them, so the observation never moved | S4, §5.2 | medium | determinism (`INV-04`) | `simulate --firmware` always sets a fixed seed and reports it in the verdict | with S6 |
| F16. Under `systemd-run --user --scope`, Renode printed its usage instead of running whenever the `-e` text began with a `$`-variable assignment (`-e '$x=1; quit'`); `-e quit` ran. The cause was not found. The S4 load used `taskset` instead | S4 | info | technique | None for the CLI, which execs Renode directly. A note for anyone wrapping Renode | no |

### S5 Where it runs in CI

All three routes need a ruling. The first changes `container/`; the other two change a workflow.

| Route | What changes | Measured |
|---|---|---|
| **A. Image** | `container/Dockerfile:49`'s `rustup-init` gains `--target thumbv7em-none-eabihf`. The existing `chmod -R a+rX /opt/rustup` covers it | In a throwaway container of the pinned image (`podman run --rm … $(cat container/IMAGE)`, with the network): `rustup target add --toolchain 1.99.1 thumbv7em-none-eabihf` took 1.41 s. `/opt/rustup` grew from 1,267,937,481 to 1,340,110,419 B (+72,172,938), and the target directory is 72,172,351 B, the same as `AET-WO-1400` S2. That is +1.2 % on the 5.98 GB image. Then `fw-bare` built `--offline --locked` in 0.27 s. A tier-2 test can build and boot in `tools` under `--network none` (`out/s5/`) |
| B. Build in `gates`, artifact to `tools` | `gates` adds `rustup target add`, builds, and uploads the ELF; `tools` downloads it and boots it | `gates`' toolchain is rustup's 1.99.1 from `rust-toolchain.toml`, which the Furnace kit stands in for (same rustup distribution). It needs `actions/upload-artifact` and `download-artifact`, two more third-party actions to pin. CI was not run, since the workflow needs a ruling |
| C. Target in `tools`' networked step | `rustup target add` inside the `--rm` container, which does not persist into the `--network none` steps. It needs a mounted `RUSTUP_HOME` or a second image layer at run time | not run. It is the image change made at run time, without the digest's reproducibility |

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F17. **The same source gives a different ELF file in each environment, and the same loaded bytes.** `fw-bare` on Furnace (`/home/forge/aethelgard-2100`) has sha256 `bfd902ae…`, and in the image (`/repo`) `a4d93ec9…`. Their flash images (`rust-objcopy -O binary`) are identical, `53ba1f84…`, and so are the stripped ELFs, `73cd28ea…`. `nm` shows the only difference: the v0-mangled crate hashes (`Cs2gqfZF4l6CG_` against `Cskgz…`), which come from Cargo's `-C metadata`, a hash that includes the package path. `fw-rt` behaves the same way (image `2c987a51…`, stripped `90c82e00…`, identical in both). `gates` checks out at `${{ github.workspace }}`, while `tools` mounts `/repo`, so routes A and B would hash different ELF files | S5 | medium | `INV-01`, `INV-03` | Identify firmware by `image_sha256`: the sha256 of its `PT_LOAD` file bytes in address order, which is what Renode loads. Report it beside the file's sha256 | **yes**, with S5 |
| F18. **Recommendation: route A.** `INV-01`: the loaded image is a function of the source in every route (F17), so neither route breaks it, provided identity is the image hash. `INV-03`: in A, one job builds the ELF and boots it, from one checkout, so "the same hashes" is checked inside one log. In B the booted artifact crosses jobs, and the hash must be carried and re-checked across them. B also adds two actions and a trust edge from a networked job to the `--network none` one. A costs 72 MB and about 1.4 s of image build | S5 | — | ruling input | Route A, at the ruling, as one `container/` change with its own publish (`RUL-15`) | **yes** |

### S6 The CLI surface

**Measured constraints.**
- `mcp.rs` names one tool per leaf, by its path joined with `_` (`leaves`, `the_tools_come_from_the_tree`).
- `leaves` walks into any command that has a subcommand instead of listing it (`mcp.rs:68`). So `simulate boot` would turn `simulate` into a group and remove today's `simulate` tool.
- MCP refuses positional and boolean arguments (`a_positional_argument_panics`, `a_boolean_flag_panics`), so every new input is a valued flag.

**Proposal.** Flags on `simulate`, not a new leaf:

```
aethelgard simulate --platform <repl> [--firmware <elf> --spec <spec.json> --for <virtual time>]
```

- `--firmware` requires `--spec` and `--for` (clap `requires`, exit 64 without them), and `--spec`/`--for` are refused without `--firmware`. Without `--firmware`, `simulate` is unchanged.
- `--spec` is solved in-process, as `emit` does, and every expected register value comes from that solve (`INV-07`, as `check drc --spec` takes its rules). The boot word's address and magic are CLI constants in the README contract. Nothing expected is read from the ELF.
- `--for` is a fixed virtual time: an integer with `ns`, `us` or `ms`, at most a bound the ruling sets (1 s is 10⁸ instructions). A fixed `RunFor` is what S4 measured as deterministic. A stop-on-condition `--until` would need a watchpoint and a Python hook inside Renode (`RUL-12` allows a generated one), which was not measured, so it is not proposed now.

**The run.**
1. Copy the ELF beside the platform (16 MiB cap, `RUL-13`).
2. Check the ELF header and `PT_LOAD` ranges (F12).
3. In Renode: `emulation SetSeed <constant>`; load the platform; `LoadELF`; write 0 to the three RCC enable registers (F10); `RunFor`; read between sentinels (F14).
4. Judge.

**Verdict additions** (the existing shape is kept; `machine` as today):

```json
"boot": {
  "firmware": {"path": "…", "sha256": "…", "image_sha256": "…"},
  "specification_sha256": "…",
  "seed": 2100, "virtual_time_ns": 10000000, "mips": 100,
  "instructions": 1000000, "pc": "0x080004ae",
  "boot_word": {"address": "0x20000000", "expected": "0x2100b007", "observed": "0x2100b007"},
  "registers": [{"block": "GPIOA", "register": "MODER", "mask": "0x0000fc03",
                 "expected": "0x0000a801", "observed": "0x0000a801"}, …],
  "unobservable": ["GPIOB.OTYPER", "I2C1.TIMINGR"]
}
```

| Exit | When | `kind` / error |
|---|---|---|
| `0` pass | all three witnesses (F13), no load error, no locked-peripheral warning | — |
| `1` fail | the boot word is not the magic; a solved register differs; fewer instructions than MIPS × time; `There was an error executing command` at `LoadELF`; a locked-peripheral access | `boot_word_missing`, `register_mismatch`, `cpu_halted`, `firmware_load_error`, `locked_peripheral_access` |
| `3` refused | the ELF is unreadable, irregular or over the cap; not ELF32 LE `EM_ARM` `ET_EXEC`; a `PT_LOAD` outside flash and RAM; a malformed `--spec` | `UnreadableInput`, `InputTooLarge`, `MalformedFirmware`, `FirmwareOutsideMemory`, and the existing specification errors |
| `4` | Renode missing or unusable; sandbox unavailable | as today |
| `5` | the witnesses disagree (the boot word is set but the CPU halted, or a load error beside a set word); an observation that does not parse (F14); a nonzero exit | `ToolError` |
| `64` | `--firmware` without `--spec` or `--for`; a malformed `--for` | usage |

This becomes the MCP tool `simulate` with three more optional string properties, and no new tool.

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F19. The surface: flags on `simulate` (keeps the MCP name), a fixed `--for`, `--spec` as the source of expected values, the verdict and refusals above. The alternative is a new top-level leaf, `boot`, which also keeps `simulate` intact but splits one tool's contract in two | S6 | — | ruling input | Flags on `simulate` | **yes** |
| F20. Where the `.repl` emitter lives. `architecture.md` §3 gives `aethelgard-twin` "simulator platform descriptions", with the specification its only input. The emitter also needs the ST register facts from `aethelgard-svd-hal` (addresses, `IRQn`), which is a new workspace edge, `twin → svd-hal`. The alternative is to emit it from `svd-hal`, beside the board crate and its regeneration test (P1 clause 4's pattern), committed at `boards/sentinel-01/` | S1, §1.1 | medium | ruling input | Emit from `svd-hal` beside the board crate, and commit `sentinel-01.repl` with the firmware crate under the same regeneration guard. `aethelgard-twin` stays the home of the bridge (`2101`) | **yes** |

## Gates (spike)

Furnace, on `459005f`:
- `cargo fmt --all -- --check` ok;
- `cargo clippy --workspace --all-targets --locked -- -D warnings` ok, and with `--features aethelgard-cli/with-tools` too;
- `cargo test --workspace --locked`: 486 passed, 0 failed.

The three spike crates pass `cargo fmt --check` and `cargo clippy --release -- -D warnings` in their own directories, `fw-bare` with and without `--features uart`. No workspace crate, `container/` or `.github/` file changed (`git diff --stat 7fab0e1 -- crates container .github` is empty). CI on the PR runs the unchanged workflows; it cannot witness the thumb build until S5 is ruled. The adversarial pass (§5) and the reduction pass (§5a) close the build half and are not run on a spike.

## Asked at the spike

1. **S2 (F9, F6, F7, F8):** option 1, a generated dependency-free runtime with the reset handler in `global_asm!` and the FPU enabled? Or option 2, `cortex-m-rt`, with `Unicode-3.0` named in `RUL-11`?
2. **S3 (F13, F10, F12):** the three witnesses, the cold RCC reset, and the pre-Renode ELF checks. Is the boot word's address and magic a published CLI contract?
3. **S4 (F15, F14):** a fixed seed constant, and the hardened reader.
4. **S5 (F18, F17):** route A, the image change: one flag, +72 MB, its own publish. Or route B or C? And firmware identity by `image_sha256`.
5. **S6 (F19):** flags on `simulate` with `--for`, and the bound on virtual time. Or a `boot` leaf?
6. **F20:** the emitter in `svd-hal`, beside the board crate under one regeneration guard? Or in `aethelgard-twin`, with a new `twin → svd-hal` edge?

## Build (after the ruling on PR #56)

**Commits:**
- `dc48cca`: the twin;
- `d799405`: the firmware and `simulate --firmware`, with `spike/` deleted;
- `8d4e87f`: the adversarial fixes and the reduction;
- `3d92724`: the mutants;
- `bc16a9b`: the merge of `origin/main`;
- `d595d44`: B1 and B2, after the ruling on the build.

**§7, held:**
- no change under `container/` or `.github/` on this branch (`git diff origin/main HEAD -- container .github` is empty; PR A carried the one ruled `container/` change);
- no `unsafe` in a workspace crate (the generator writes it as text into `boards/`);
- no verdict on Renode's exit code;
- no test skipped, ignored or weakened;
- no licence outside `RUL-11` (option 1, nothing fetched).

The protected surfaces diff to empty against `origin/main`: the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**` and `aethelgard-ir`. The board crate's API is unchanged; its tree gained `firmware/` and `sentinel-01.repl`, both generated.

### What was built

- **R6, `aethelgard-twin`:**
  - **`platform::emit`** lowers a `SolvedTopology` to a `.repl`. It includes `stm32h753.repl` root-relatively, never redeclares a stock instance, and adds only an SPI instance Renode lacks, with `<P>_BASE` and `<P>_IRQn` from the header.
  - **The stock table** (16 instances) and the three Renode files' hashes are pinned. A tier-2 test checks both against the installed Renode.
  - **`boot`** is the judge's half: the memory from the header, the boot word and its magic, the fixed seed, the 100 ms bound, and `expected()`. That function computes the register image from the topology alone: each solved pin's `MODER` and `AFRL`/`AFRH` field, and each cold RCC register in full.
  - **`cmsis`** gains two refusing shapes: `<NAME>_IRQn = <decimal>,` inside `IRQn_Type`, and a region's size from its base's comment. The header states DTCM's 128 KB only there.
- **R1, `aethelgard-svd-hal` (one subagent, reviewed, reproduced):** `generate` also writes `firmware/` beside the board crate:
  - a `global_asm!` reset handler that zeroes `.bss`, copies `.data`, enables the FPU (CPACR CP10/CP11, `dsb`, `isb`), then calls `main`;
  - the exception table;
  - `link.x`, from `FLASH_BANK1_BASE` to `FLASH_BANK2_BASE` and from `D1_DTCMRAM_BASE` with 128 KB, with the boot word first in RAM;
  - a `.cargo/config.toml` that links with `-Tlink.x` and builds into the top-level `target/`, measured to need no build script;
  - a `main` that uses only the board crate's API and ends at the boot word.

  The emitted `sentinel-01.repl` is committed beside it. The regeneration guard covers every generated byte, with hand-edit controls for `firmware/src/main.rs` and the `.repl`, and checks the twin's magic and boot word against the generator's.
- **R2 to R5, `simulate --platform --firmware --spec --for`:**
  - **Before Renode is looked for**, it refuses five things: a platform that is not byte for byte the one emitted for the spec, or an `@` path (`PlatformNotEmitted`); an ELF that is not ELF32 LE `EM_ARM` `ET_EXEC` with a sound header table (`MalformedFirmware`); a `PT_LOAD` outside flash and RAM (`FirmwareOutsideMemory`); overlapping `PT_LOAD` memory; and file bytes over the boot word (`FirmwareLoadsBootWord`).
  - **The boot** runs `emulation SetSeed 2100`, loads the platform and the ELF, zeroes the cold RCC registers and the boot word, sets `VectorTableOffset` to `FLASH_BANK1_BASE`, then `RunFor` the time.
  - **The reads** come under `logLevel 3` between whole-line markers, as bare values only, with a stray `\r` tolerated. Then `peripherals`.
  - **The judge** uses the three witnesses, plus a locked-peripheral warning (a fail), the listing naming every solved instance (else exit 5), the pinned Renode version (else exit 4), and the PC inside an executable segment.
  - **`--for`** is an integer `ns`, `us` or `ms`, at most 100 ms and a multiple of 10 ns, so the instruction count is exact.
  - **The verdict** carries `boot`, with `image_sha256`. The contract is in `crates/aethelgard-cli/README.md`, "Booting firmware".

### Shape-check changes (roadmap §6: graduation is stated)

In `crates/aethelgard-cli/tests/workspace_shape.rs`:
- `aethelgard-twin` leaves `libraries()` and gets `twin_has_only_its_known_files_keys_and_dependencies`: files `Cargo.toml`, `src/*.rs` and `tests/*.rs`; dependencies `aethelgard-ir`, `aethelgard-svd-hal` and `thiserror` (the ruled edge); dev-dependency `serde_json`.
- `svd_hal_has_only…` admits the dev-dependency `aethelgard-twin`, for the regeneration guard.
- `boards_are_excluded_and_dependency_free` also checks `firmware/`:
  - manifest keys `package`, `dependencies` and `profile` only;
  - no `build` or `links` key and no `build.rs`;
  - exactly one dependency, the board, at `path = ".."`.

### §1.4, and where each clause runs

| Clause | Test | Where |
|---|---|---|
| The platform loads with every solved peripheral listed | `twin_tier2::the_emitted_platform_loads_and_lists_every_solved_peripheral`, plus the per-boot listing check | `tools` |
| The firmware boots to the observable | `twin_tier2::the_reference_firmware_boots_and_two_boots_agree`: built offline in the image, exit 0, word `0x2100b007`, 1,000,000 of 1,000,000 instructions, every register as expected | `tools` |
| Two emissions are byte-identical | `aethelgard-twin` `two_emissions_are_byte_identical`; `svd-hal` `two_generations_are_byte_identical` (which now covers the `.repl` and `firmware/`) and the regeneration guard | `gates` and `tools` |
| Two boots give the same observation | the same tier-2 test compares the two `boot` objects and listings | `tools` |

Also in tier 2:
- Renode's files match the pinned hashes and the stock table;
- a 10 µs boot fails with `boot_word_missing`;
- the A3 and A7 mutants fail (below).

Tier 1 (`boot_tier1.rs`, 16 tests, `gates`) drives every branch of the judge through a stub that also refuses any command line without the boot's fixed parts.

**Measured on Furnace in podman, `tools`' own steps** (`cargo test --workspace --all-targets --locked --features aethelgard-cli/with-tools --no-run`, then fmt, clippy and test under `--network none`, `CARGO_TARGET_DIR=/repo/target`):
- **Before the bump:** on the pinned image with the target added (a local stand-in for #57), 554 passed and 0 failed at `d799405`. That includes `board_tier2`'s Kani proofs, 517 s: `prove` still judges the board directory with `firmware/` inside it.
- **After the merge:** on the published `2df68259…` (`container/IMAGE` at `89a3514`), at `bc16a9b`: fmt ok; clippy `-D warnings` ok; 562 passed and 0 failed. `ANSWER 2100` lines: the firmware builds offline in 0.05 s; the platform passes; boots 1 and 2 pass and are identical (2.64 s and 2.58 s); the 10 µs boot fails; A3 and A7 fail.
- **ELF identity in the build (F17):** the reference ELF's file hash differs between Furnace (`66a8a43a…`) and the image (`c9754946…`), while the image hash agrees; under the final address-bound definition both give `8d0e710e…`.

### Adversarial pass (§5), by a fresh subagent (Opus) on the frozen commit `d799405`

About 43 firmware, ELF and input variants went through the judged call, plus direct Renode probes and a 54-case topology probe of the twin. Evidence is in `~/.cache/aet-2100-adv/` (Furnace). The lead reproduced each critical and high finding on its own build before fixing it. After the fixes it predicted each outcome and reran the mutants (`~/.cache/aet-2100-adv/rerun.txt`):

| id | target | attack | before (d799405) | after |
|---|---|---|---|---|
| A1 | 1 | `.aethelgard_boot` made loadable with the magic, `udf` instead of the write | **pass** (critical) | exit 3 `FirmwareLoadsBootWord` |
| A2 | 1 | a 4-byte `PT_LOAD` holding the magic appended to a build that never writes it | **pass** (critical) | exit 3 `FirmwareLoadsBootWord` |
| A1c | 1 | the word placed first in `.data`, so the reset handler copies it in | **pass** (critical) | exit 3 `FirmwareLoadsBootWord` |
| A3 | 1 | flash linked at `0x08010000`: no vector table where the part boots; Renode guessed one | **pass** (critical) | fail: `boot_word_missing`, `cpu_halted`, `pc_outside_firmware`, `register_mismatch` |
| A4 | 2 | a zero-fill (`memsz`) over `.text`; two ELFs differing in header order shared one hash and got opposite verdicts | pass and fail, one hash (high) | both exit 3 `MalformedFirmware` (overlap) |
| A5 | 2 | `.rodata` moved by `0x1000`, same bytes: same hash, different verdict | fail with the reference's hash (high) | fail, its own hash |
| A7 | 1 | a jump into the FMC bank (zero-filled mapped memory) after the word | **pass** (medium) | fail `pc_outside_firmware` |
| A6 | 1 | a jump into a hole after the word: "This machine has aborted", and `RunFor` never returns | CLI hung; Renode at 10.47 GB RSS at 184 s (high) | **fixed by B1:** exit 1 `cpu_aborted` in 14.3 s on Furnace and 14.9 s in the image, killed at the 1 GiB cap |
| A8 | 1 | `udf` or `bkpt` after the word (the fault handler parks) | pass | pass: the fixed point was reached (residual, F22) |
| A9 | 1 | a log flood: 241,141 warnings in 10 ms | exit 5, output over 16 MiB (medium) | unchanged: F23 |
| A10 | 1 | `wfi` after the word | exit 5 (ruled) | unchanged |
| A11 | 1 | word first with no configuration; GPIOA or SPI1 before its clock; GPIOC touched; an extra clock; a clock re-disabled | fail each time | fail each time |
| A13 | 1 | a clock outside the cold registers (DMA1, SYSCFG), DMA1 written without its clock | pass (low) | unchanged: F24 |
| B1 | 2 | 6 in parallel plus 2 sequential, for the reference and for a firmware folding RNG, `system_flash_ro`, RTC, `CYCCNT` and SysTick into the registers | 8 of 8 identical, each | (no change needed) |
| C1, C2 | 3 | SPI1/2/3/5/6, SPI4, I2C2–4, GPIOJ/K, two SPIs, AFRH pins; 24 malformed names | held: right base, IRQ, clock bit and gating; every malformed name refused | held |
| C3 | 3 | a name the header lacks (SPI7) gave exit 70 | low | exit 2 |
| C4 | 3 | the CLI never pinned Renode at run time, nor checked the listing | low | fixed: exit 4 for another version; exit 5 for a listing without a solved instance |
| A17, A19 | reader | symlinks, FIFO, CRLF, `/dev/stdin`, TOCTOU; UART and semihosting injection of markers or values | held | held |
| A18 | reader | `TMPDIR` with a space or `;` reaches the monitor's `@path` | exit 5 (low) | unchanged: F25 |

The adversary's topology probe (`crates/aethelgard-twin/tests/adv_2100.rs`, in its frozen copy) was not adopted. Its refusal matrix overlaps `aethelgard-twin`'s `what_cannot_be_lowered_is_refused` and svd-hal's new `pin_names_are_read_strictly`.

### Reduction pass (§5a) and mutants

**Diff-sized.** The four questions over the diff:
- **One fact stated twice:** two parsers of pin names. svd-hal's passed the digits to `parse::<u32>`, which accepted `PA+5` as PA5; the twin's was strict. Now there is one: svd-hal's `board::pin_name`, strict, with a test, which both wrap.
- **Two mechanisms that were one:** two tier-2 boot helpers, and a reads-to-transcript conversion repeated in every tier-1 case.
- **Redundant, shown by mutant M11:** the 100 MIPS check. The expected instruction count is the twin's constant, so any other speed already fails the instruction witness.
- **Protected and labelled `belt-and-suspenders:`:** the generator's and the judge's separate copies of the magic, the `MODER` values and the memory map (`INV-07`: the judge never reuses the generator's code), and the twin's only-an-SPI-is-added refusal (M15).

**Mutants, one per guard this work order added, against the suite it lives in** (`~/.cache/aet-2100-spike/build/mutants.py`, its output beside it): 19 run, 18 killed. Two survivors were handled:
- **M11**, removed as redundant (above);
- **M15**, which survives because every I2C and GPIO instance of the H753 header is stock; it is kept, labelled.

The killed ones:
- boot-word coverage; overlap by `memsz`; the hash binding addresses;
- the boot word zeroed; `VectorTableOffset`; the PC; the listing; the Renode pin;
- the locked warning; more instructions than the run holds; the halt; the register compare;
- the cold RCC judged in full; `pin_name`'s sign; whole instructions; the emitted-platform check; whole-line markers.

### Gates (build)

On `bc16a9b` on Furnace:
- `cargo fmt --all -- --check` ok;
- `cargo clippy --workspace --all-targets --locked -- -D warnings` ok, and with `--features aethelgard-cli/with-tools`;
- `cargo test --workspace --locked`: 522 passed, 0 failed, across 42 test targets.

Tier 2 is above.

**CI, the first push after the ruling** (`b2df9a5`, acceptance 3), green:
- run [42459301466](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42459301466): `gates` 2 m 40 s and `tools` 15 m 10 s, on the bumped image;
- run [42459301461](https://github.com/ClearForge-LLC/aethelgard/actions/runs/42459301461): `image` 4 m 17 s.

The architect can re-read the `tools` job's `ANSWER 2100` lines from the log:
- the firmware builds in 0.20 s;
- the platform passes;
- boots 1 and 2 pass, identically;
- the 10 µs boot, A3 and A7 fail;
- 562 passed, 0 failed.

The reference's `image_sha256` there is `8d0e710e34a17bd7ac34e3c0e2c7cbdcfa1753ed9a18c08681301d0040ae2c4c`, equal to Furnace's and podman's. Firmware identity holds across three checkout paths (F17).

Reduction pass (diff): −42 lines net (the pin parser −18, the speed check −7, the tier-2 boot helper −17; the tier-1 transcript helper about even) · 3 duplicates removed · 1 redundant check removed · 4 protected kept, labelled · 1 finding below (F30).

### Findings (build)

| finding | where | severity | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F21. **A6: an aborted machine hung the call, and Renode's memory grew without bound.** **Ruled B1 and fixed.** Renode logged `This machine has aborted`, `RunFor` never returned, and Renode grew at about 55 MB/s, to 10.47 GB RSS at 184 s. Now the reaper kills the tree at a 30 s wall bound or a 1 GiB resident cap, and the call is a named `fail`. Measurements below ("B1 and B2") | `simulate --firmware`; `RUL-13` as amended | high | host safety | Done | no (ruled B1) |
| F22. **The residual of A1/A8:** the boot word witnesses that the solved configuration happened and that the core kept executing inside the firmware's own code, not that `main` returned. A firmware that writes it early and then faults into its own handler passes. **Ruled B2: accepted, named**, and now stated in one sentence in the CLI README's "Booting firmware" | the boot contract | medium | residual | Done | no (ruled B2) |
| F23. A9: a firmware flooding the log, 241,141 warnings in 10 ms, makes the output exceed `RUL-13`'s 16 MiB, so the call is exit 5, never a verdict. That is the safe direction, but a locked-peripheral flood was due a `fail`. Renode collapses identical lines, so only varying ones flood | the reader | medium | verdict quality | A later work order: send the log to a file Renode writes, and read only the marker block from stdout | no |
| F24. A13: only the RCC registers that gate a solved port or peripheral are cold-written and judged. A clock in another register (DMA1 in `AHB1ENR`) and writes to that peripheral pass, though on silicon they would be dropped while gated | the cold start | low | scope | Cold-write and judge every `RCC_*ENR` the header defines, when a work order needs it | no |
| F25. A18: the scratch path (from `TMPDIR`) reaches the monitor's `@path` unquoted. A space or `;` in it is exit 5. Not caller input; the same in plain `simulate` since `AET-WO-0300` | `renode.rs` | low | robustness | Refuse a scratch path outside a safe character set, or quote it | no |
| F26. **`image_sha256` as built:** for every `PT_LOAD` with memory, in physical-address order, its address, file size and memory size (each a `u32` LE), then its bytes. R4's first wording gave one identity to two images (A4, A5). It is path-independent (F17, three checkout paths). **Ruled B3: ratified** | R4; the CLI README | medium | ruling amendment | Done | no (ruled B3) |
| F27. **The added checks**, each from the adversarial pass: `FirmwareLoadsBootWord`; overlapping `PT_LOAD` memory; `pc_outside_firmware`; for a boot, the pinned Renode version (exit 4) and the listing naming every solved instance (exit 5); the boot word zeroed and `VectorTableOffset` set before the run. None can fail a firmware that keeps the boot contract. **Ruled B4: ratified** | R2, R5 | medium | scope | Done | no (ruled B4) |
| F28. `twin_stop` maps a header name the topology needs and the header lacks to exit 2 (C3). A header that fails its own checks stays exit 70, a corrupted build | `main.rs` | info | correction | None | no |
| F29. `cargo fmt --all` does not reach `boards/sentinel-01/firmware/` (it is no dependency of a member), so its formatting is the generator's alone, pinned by the guard | `firmware/` | info | note | None | no |
| F30. The subagent's `cargo fmt --all` overlapped the lead's edits during the build. Nothing was lost (the lead re-read every file it adopted), but two agents ran a workspace-wide formatter in one tree | process | info | process | A builder subagent runs `cargo fmt -p <its crate>`, never `--all` | no |

### Asked at the build (ruled on PR #56, comment 6718943342)

The ruling is saved verbatim at `~/.cache/aet-2100-spike/build/ruling-6718943342.json`.
- **B1 (F21):** fixed before merge (below).
- **B2 (F22):** the residual is accepted and stated in the README.
- **B3 (F26):** ratified.
- **B4 (F27):** ratified.
- **F23, F24 and F25:** carried to roadmap §7 by the architect at close-out.
- **F30:** becomes a roadmap §6 lesson.

### B1 and B2 (after the ruling on the build)

**The measurement.** The reference boot at `--for 100ms`, five runs each, came through the real call, `simulate --platform … --firmware … --spec … --for 100ms`. A poller (`~/.cache/aet-2100-spike/build/b1/poll.py`) sampled every 10 ms the summed `VmRSS` and the largest `VmHWM` of the call's whole process tree.

| Where | Wall time | Peak tree RSS | Renode `VmHWM` |
|---|---|---|---|
| Furnace, release | 2.10–2.16 s | 392,620–394,392 KiB | 385,672–387,376 KiB |
| tools image `2df68259…` | 2.54–2.59 s | 399,832–404,620 KiB | 387,976–392,540 KiB |

The raw lines are in `b1/furnace-100ms.txt` and `b1/image-100ms.txt`.

**The constants** (`renode::BOOT_BOUNDS`):
- **Wall bound: 30 s.** The ruling's floor is 10 × 2.59 s, rounded up: 26 s. 30 s leaves room for CI, whose boots under parallel tests took up to 6.84 s (run 42459301466).
- **Memory cap: 1 GiB (1,048,576 KiB).** The floor is 2 × 404,620 KiB = 809,240 KiB, about 791 MiB.
- **No §7 stop:** 2 × the peak is 0.77 GiB, under 4 GiB.

**The mechanism, measured, not assumed.** `RLIMIT_AS` is unusable. `prlimit --as=N renode --version` fails at 1, 4, 16 and 256 GiB, with `Failed to create CoreCLR, HRESULT: 0x80070008` and then `GC heap initialization failed with error 0x8007000E`. So the reaper does it:
- it is the tool tree's unconfined parent and subreaper, and already polls every 20 ms;
- with `--wall-ms` and `--rss-kib`, which only `Invocation::run_bounded` passes, it checks the wall clock each tick and the tree's summed `VmRSS` every fifth tick (100 ms, a `/proc` scan);
- past either, it kills the tool's group and every descendant, then writes `ended bound=wall` or `ended bound=memory`;
- `simulate --firmware` turns that into a `fail`: `wall_bound_exceeded`, `memory_cap_exceeded`, or `cpu_aborted` when the output holds `This machine has aborted`, with `tool.exit_code` `-1` and nothing read.

**Scope.** `run.rs`'s `run` is unchanged (it calls the shared start with no bounds), so plain `simulate`, `prove` and `check` pass the reaper the same arguments as before. `git diff --stat` shows no existing test file changed: 1600's `channels_tier1`, `sockets_tier1`, `descriptors_tier1` and `lifecycle_tier1` pass as they were. The only test files touched are this work order's own `boot_tier1.rs` and `twin_tier2.rs`, each with new tests added.

**The tests:**
- **Tier 1** (`boot_tier1.rs`, now 19 tests) drives all three reasons through the stub:
  - a stub holding about 1.3 GB in a shell variable is `memory_cap_exceeded` in under 30 s;
  - the same after printing `This machine has aborted` is `cpu_aborted`;
  - a stub that sleeps is `wall_bound_exceeded` at 30–35 s.
- **Tier 2** (`twin_tier2.rs`): the A6 mutant (a jump to `0x0A000001` after the word) returns as `cpu_aborted` inside the bound plus 5 s. Measured at 14.92 s in the image (`ANSWER 2100: A6 hole`), and 14.31 s on Furnace with a peak tree RSS of 1,062,204 KiB: the cap plus one 100 ms poll's growth.

**Gates, at `d595d44`:**
- **Furnace:** fmt ok; clippy `-D warnings` ok, with and without `with-tools`; `cargo test --workspace --locked`: 525 passed, 0 failed, across 42 targets.
- **The `tools` sequence in podman, on the published image:** fmt ok; clippy ok; 566 passed, 0 failed (`~/.cache/aet-2100-spike/build/tools-local3.log`).

**B2.** The CLI README's "Booting firmware" states the residual as ruled. The boot word witnesses that the solved configuration happened and that the core kept executing inside the firmware's own code; it does not witness that `main` returned.
