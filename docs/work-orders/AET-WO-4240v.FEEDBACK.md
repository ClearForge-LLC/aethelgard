# AET-WO-4240v — FEEDBACK (build)

Branch `wo/aet-4240v-vos2`, from `main` at `3f989435` (the kickoff named `ee835bde`; `main` moved by `4600`'s record only, docs and `spike/4600`). The work order is `docs/work-orders/AET-WO-4240v.md` on the docs branch (copy at `~/.cache/aet-4240v-wo.md`).

Every answer is labelled **measured**, **read at source** or **inferred**.

## §0 Predictions (committed before the first run, §1.0)

Written after the IR's types compiled and before any run of changed code. The baselines in §2 come from unmodified `main` `3f989435`, in a separate worktree.

### §0.1 What moves

| What | Predicted | Class |
|---|---|---|
| Every generated ELF (each committed specification `generate` accepts, `boards/sentinel-01{,-bench}`, `3304`'s `sentinel-02-bench.elf`, the bench package's ELF) | moves: `main` gains the `PWR` sequence | inferred |
| `boards/sentinel-01{,-bench}`: `board.rs`, `proofs.rs`, `firmware/src/main.rs`, the `.repl` | all four move (`Block::Pwr`, the `Pwr` singleton, `Scale2`, the harness, the `pwr` override) | inferred |
| `svd-hal` `tests/order_2203.rs` digests | every digest whose file holds the bring-up or the `Block` enum moves | inferred |
| `svd-hal` `tests/ui/{motors-pwm,serial-four}/board.rs` | move (`Block::Pwr`, `Pwr`, `reset_clock` takes `Scale2`) | inferred |
| `tests/board.rs` `settle` and bound lines | **do not move** (no clock moves) | inferred |
| Every emitted `.repl` and every pinned platform hash (`twin/src/platform.rs`) | moves (the `pwr` override) | inferred |
| The 26 committed specifications at schema 17 (4 `examples/`, 21 under `cli/tests/fixtures/`, 1 under `ir/tests/fixtures/`) | each gains `mcu.core_supply` and schema 18; `sentinel-02-own.json` also gains `foreign_clock.vos` | read (the inventory, by script) |
| The 5 solved fixtures (`hwdef` ×3, `pincheck`, `svd-hal/tests/ui/motors-pwm`) and the 6 embedded solved JSONs (`twin/src/tests.rs`, `twin/tests/spi_platform.rs`, `cli/src/renode.rs` ×2, `cli/src/analog.rs`, `pincheck/tests/identity.rs`) | re-derived by `solve`: version 12, `core_supply`, `clock.vos`, and a new `specification_sha256` | inferred |
| Every solved word (`PSC`, `ARR`, `CCR`, `BRR`, `MBR`, `TIMINGR`, the kernel selects) | **none moves** | inferred (the RCC tree does not change) |
| Bench package (`fab/sentinel-01-bench/package/`, 17 files) | only the ELF, `MANIFEST.json` and `FAB-NOTES.md` move | inferred |
| Any generated-firmware verdict | **none flips** | inferred |
| ArduPilot's foreign witnesses | byte-identical, if the override is not emitted for a foreign platform, or if ArduPilot's reads of `CR3` do not change its path | inferred, to be measured |

### §0.2 Kani harnesses per board

| Board | Before (read, `boards/*/src/proofs.rs`) | After |
|---|---|---|
| `sentinel-01` | 7: `every_valid_sequence`, `i2c1_transfer_is_bounded_and_ordered`, `spi1_transfer_is_bounded_and_ordered`, `spi1_transfer_refuses_too_many_frames`, `reset_clock_is_ok_exactly_at_reset`, `usart1_start_writes_the_solved_values`, `flash_option_status_is_one_read` | 8: the same, plus `pwr_scale2_is_once_in_order_and_bounded` |
| `sentinel-01-bench` | 8: as above, plus `i2c2_transfer_is_bounded_and_ordered` | 9 |

### §0.3 New ledger rows and their classes

| Row | Access | Class |
|---|---|---|
| `PWR.CR3.SCUEN`, `.LDOEN`, `.BYPASS` | read-modify-write | `independent-rule` (the §1.3 rule's word; the twin's rule and Kani hold the firmware to it); ignored by the stock model |
| `PWR.D3CR.VOS` | written | `independent-rule`; ignored by the stock model |
| `PWR.CSR1.ACTVOSRDY`, `PWR.D3CR.VOSRDY` | polled | `silicon-only` |
| `FLASH.ACR.LATENCY`, `.WRHIGHFREQ` | depended on at reset | `silicon-only` |

Four new `silicon-only` rows, each acknowledged by name in `examples/sentinel-01.json`.

### §0.4 Bench row B-2's words (predicted; the source class beside each)

On the NUCLEO-H753ZI, revision V (`DBGMCU_IDC` `0x20036450`, measured in `3302`):

| Read | Predicted | Class |
|---|---|---|
| Under reset, before the firmware: `PWR_D3CR` | `0x0000_6000` (`VOS` `01` Scale 3, `VOSRDY` 1: reset `0x4000`, "Following reset VOSRDY will be read 1", RM0433 Rev 9 p. 310) | rule |
| After a power cycle and a boot: `PWR_CR3` | `0x0000_0042`: rev V's `0x46` with `SCUEN` 0, `LDOEN` 1, `BYPASS` 0 written, bit 6 kept | generator (the write); inferred (that `SCUEN` reads as written) |
| `PWR_CSR1` under `0x0000_E000` | `0x0000_A000` (`ACTVOS` `10`, `ACTVOSRDY` 1) | rule |
| `PWR_D3CR` | `0x0000_A000` (`VOS` `10`, `VOSRDY` 1) | rule |
| word 0 | `0x2100_B007` | generator |

### §0.5 What fails before the specifications move

- After the bump and before the specifications move, **every test that reads a committed specification** fails, refused as `UnsupportedSchemaVersion { version: 17, expected: 18 }` (inferred). Those are the `examples/` walkers (`cli/tests/common/{corpus,spec,verify,foreign,th}.rs` and the tier-1 files that use them, `svd-hal/tests/{serial_ports,spi_devices,order_2203}.rs`, `pincheck/tests/common`, `twin/src/tests.rs`) and the IR's own unit tests that read `examples/`.
- **Measured** on the IR crate alone, before any specification moved: `parts::tests::aet_1207_a_device_whose_modes_exclude_the_driven_one_is_refused` and `orientation::tests::an_imu_without_cited_axes_or_frame_is_refused_by_name` fail, 45 pass. (These ran before this table was committed. They are unit tests of unmodified-logic code, not a run of the change.)

### §0.6 The Nucleo's supply

- **Predicted:** `ldo`. If the MB1364 schematic shows VCAP decoupled only, with no external VCORE driving it, this is read at source. If it does not, or if the file cannot be parsed reliably, this is a §7 stop for that board alone.
- UM2407 Rev 6 does not state the H753ZI's core supply. A search of its text for LDO, SMPS, VCAP, bypass, VCORE and regulator finds only the 3V3 and 1V8 regulators (read at source).

## §1 What was built

### §1.1 The declaration (`aethelgard-ir`)

- **`mcu.core_supply` = `{mode, source}`** (`lib.rs` `CoreSupply`, `vos::CoreSupplyMode`), required, in specification schema **18**. `mode` is `ldo` or `bypass`, the two legal rows of RM0433 Rev 9 Table 34 (p. 261, read at source from the page image): "LDO supply" `LDOEN` 1 `BYPASS` 0, and "LDO Bypass" `LDOEN` 0 `BYPASS` 1. The other two pairs are "Illegal combination, the default configuration is kept (write data will be ignored)". By §6.8.4 (p. 308), "the written-once mechanism will lock the register ... The system shall be power cycled before writing a new value", so no other pair is offered.
  - The H753 has no SMPS: the header's `PWR_CR3` has no SMPS field (L14492–14518), and RM0433 Rev 9 has no "SMPS" anywhere in its text (read at source, by search).
  - `source` takes `foreign_clock`'s shape: 1–200 bytes of printable ASCII, no space at either end, refused as `InvalidSource` at `mcu.core_supply.source` (validate step 2c). The field's documentation states F-CITE strict.
  - Missing is ``missing field `core_supply` ``; `null` is "invalid type: null, expected a CoreSupply object"; any other mode, mixed case included, is "unknown variant `LDO`, expected `ldo` or `bypass`". All are tested (`ir/tests/vos.rs`).
- **`bypass`** validates and solves (tested); `generate`, `emit` and `simulate` refuse it (§1.4, §1.6).
- **The solved schema is 12.** `SolvedTopology.core_supply` (the mode) and `SolvedClock.vos` (`scale0`–`scale3`) are new.
  - `SolvedClock::reset()` is now the reset RCC tree at Scale 2, the only tree generated.
  - **No stand-in under any version.** The reader's `None if !current => SolvedClock::reset()` path is gone, and `core_supply` has none either. An older file without them is refused at parsing by the key it lacks; one carrying them reaches `validate` and is refused by its version. Both cases are tested (`an_older_solved_file_is_refused_not_defaulted`).
  - **Deviation:** `motor.rs`'s `a_version_4_topology_without_the_new_fields_is_refused_as_a_version` asserted the old stand-in. It is now `..._refused_never_given_a_clock` and expects the parse refusal.
- **`mcu.foreign_clock.vos`** = `{value, source}` (`motor::SourcedVos`), required in a declared foreign clock. `sentinel-02-own.json` declares `scale1` from Copter-4.7.2's `stm32h7_mcuconf.h`, **L109** (not L110), `#define STM32_VOS STM32_VOS_SCALE1` (read at source: `~/.cache/aet-4210/ardupilot` at `fb773567`, file sha256 `67f48e5f…a426`). `SolvedClock::from_rcc` takes the declared scale; it is never decoded.

### §1.2 Every committed specification declares, with its source

All 26 files at schema 17 moved to 18 by one script (`~/.cache/aet-4240v/kit/specs.py`), which inserts `core_supply` after `bor_level` and keeps each file's formatting.

| Path | Mode | Source (as written) | How read |
|---|---|---|---|
| `examples/sentinel-01.json`, `sentinel-01-minimal.json`, `invalid-zero-domain.json`; all 21 under `crates/aethelgard-cli/tests/fixtures/` (including `solve/*`), except as below; `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` | `ldo` | "DS12117 Rev 10 sec. 3.5.1 p. 27, Figure 65 p. 206, Table 113 p. 211: VCAP decoupled only, the MCU support group of power-parts.json (2 x 2.2 uF VCAP to GND); no external VCORE" | read at source from the page images (`reads/pg/ds-27.png`, `ds-206.png`, `ds-211.png`), and the parts rows `power-parts.json` L1073–1077 and L1187–1198. **Those rows cite `DS12117` with no revision**, so the revision is the source string's, from the held Rev 10 (`607b9ead…c032`) |
| `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` | `ldo` | the same | the same. ArduPilot's own `LDOEN \| USB33DEN` write (`stm32h7_mcuconf.h` L92) is the firmware's choice, not the board's, and is not the source |
| `examples/sentinel-02-bench.json` (NUCLEO-H753ZI, MB1364) | `ldo` | "MB1364 rev C-01 and E-01 schematic, MCU_144_POWER: VCAP1/VCAP2 (pins 71/106) only to C48, C50 2.2 uF to GND, fitted per MB1364-H753ZI-C01/E01 BOM; no external VCORE" | read at source from ST's board design package. **UM2407 Rev 6 states no core supply** (searched for LDO, SMPS, VCAP, bypass, VCORE and regulator), which is the WO's §7 stop. The kickoff names MB1364 as the source, so I read it. The Altium sources were parsed as OLE plus a netlist for both revisions: the `VCAP` net is exactly U14.71, U14.106, C48.2 and C50.2, and nothing else on any sheet. C48 and C50 are "Fitted" in both H753ZI BOMs (`CC0603KRX5R6BB225`). No 1.0–1.4 V regulator exists in the design. Posted on the bus (§4) with the option of holding it as a stop |

The solved fixtures were re-derived by `solve` (`hwdef` ×3: only the spec sha256 line of each `hwdef.dat` moves). `pincheck`'s `common/topology.json` is the solve of `examples/sentinel-01-minimal.json` as of `10806528` (sha256 `03a82a04…`), not today's file. Re-deriving it from today's file fails 9 of `pincheck`'s tests that rest on its older content. So it was migrated by schema only, as #238 (`ee835bde`) migrated it from 10 to 11.

### §1.3 The rule (`aethelgard-ir` `vos.rs`, a module beside `silicon.rs`)

- **The home is a new module, `vos.rs`.** It shares nothing with `solve.rs`, `motor.rs`, `serial.rs` or `aethelgard-svd-hal`:
  - every bus clock and the timer kernel (Table 57 with `TIMPRE`) are computed from the `SolvedClock` fields in its own code;
  - its APB2 timer list is its own;
  - the kernel selects at reset and the HSI's fastest come from `silicon` (`spi_kernel_hz`, `usart_kernel_hz`, `fast_hz`), the other independent rule.
- **The limits,** every cell re-read from the page images (`reads/pg/ds-210.png`, `rm-355.png`, `rm-356.png`) and tested cell by cell (`each_family_holds_exactly_to_its_maximum_at_every_scale`). They agree with the WO's table, and LPUART1 is 125/100/75/50 at VOS0–3 (Table 60, p. 356). Footnote 5 ("TJMax = 105 °C") is on VOS0's `fHCLK` and `fPCLK`. Footnote 6 ("Maximum APB clock frequency when at least one peripheral is enabled") is on VOS3's `fPCLK` only.
- **Revisions.** Rev Y's operating conditions (DS12117 Rev 10 Table 13, p. 105) have no frequency rows. Every verdict that judges a bus clock carries `bus_limits_rev_v`.
- **At its fastest** (#240 S14): 64 MHz judges as 64,958,976 Hz. The S14 mutant is killed: 74 MHz at VOS2 is refused, because its fastest is 75,108,816 Hz.
- **`FLASH_ACR` at reset** [D-7]: `LATENCY` 7 and `WRHIGHFREQ` `11` against Table 17 (p. 160, read from the image) at VOS2 with the AXI clock's fastest, 64.96 MHz: the ]55; 110] MHz row needs 1 WS and `01`. It holds: p. 160 calls the default 7 WS "a safety margin". `WRHIGHFREQ` `11` appears in no row; a longer programming delay than the row's is judged within, which is inferred (this firmware never programs the flash). The refusal is named `FlashWaitStatesBelowTable`.
- **Refusals:**
  - `VosLimitExceeded { clock, fastest_hz, limit_hz, vos, cite }`, by clock name;
  - `SolvedTopology::validate` runs the rule first, then requires the reset tree at Scale 2 (`UnsupportedClock` otherwise);
  - so the reset tree at VOS3 is refused naming `fPCLK1`. The rule's verdict names all ten clocks over 50 MHz: `fPCLK1`–`4`, SPI4, SPI6, I2C1, I2C4, USART1 and UART4 (`the_reset_tree_at_vos3_is_refused_naming_each_clock_over_50_mhz`);
  - the reset tree at Scale 0 or 1 passes the rule and is `UnsupportedClock` (tested).
- **A kernel the rule cannot read** is named in the verdict's `unread`. `validate` leaves it to the controller checks, which refuse it by name (`SpiControllerMismatch`); refusing it first as `UnsupportedClock` pre-empted that name, which a test caught. `Verdict::holds()` counts it.
- **The foreign judge** [D-4] (`vos::judge_foreign`, reported by `cli` `renode/foreign.rs` `ClockReport.vos`):
  - the rule runs at the run's end on `from_rcc`'s decode at the declared scale, plus the declared USART kernels;
  - every clock is judged at the HSI's fastest only when the tree is the HSI's (`SWS` HSI, or PLL1 with `PLLSRC` `00`; RM0433 §8.7.11, p. 399, where one source feeds every PLL);
  - otherwise nothing is judged, and each clock is named with why;
  - `fPCLK4` is never judged (`D3PPRE` is not read);
  - a clock over its maximum adds `vos_limit_at_boundary: <clock> …` to `conditions`, never a finding;
  - ArduPilot's measured registers at VOS1 name `fCPU`, `fHCLK`, `fPCLK1`–`3` and both timer kernels (tested), and its 72 MHz USARTs hold.

### §1.9 Bench row B-2, extended (for the architect to place; Nucleo, t9o777, Scotty's)

`P=~/tools/probe-rs/bin/probe-rs`, chip `STM32H753ZITx`, syntax as `3304.bench.md` L90 uses it (`$P read --chip STM32H753ZITx b32 <address> 1`). Each predicted value has its class: **rule** (RM0433 text), **generator** (the firmware's write), **inferred**, or **not predicted**.

**The control, under reset, before any firmware runs** (after a power cycle, `--connect-under-reset`):

| Read | Predicted | Class |
|---|---|---|
| `PWR_D3CR` `0x58024818` | `0x0000_6000`: `VOS` `01` (Scale 3) and `VOSRDY` 1 | rule: reset `0x0000 4000` and "Following reset VOSRDY will be read 1 by software" (p. 310). The RM's own Table 49 shows `VOSRDY` 0 at reset, and the two are not reconciled, so a `0x4000` here is the RM's ambiguity, not a failed row |
| `PWR_CR3` `0x5802480C` | `0x0000_0046` (rev V) | rule (p. 307) |
| `PWR_CSR1` `0x58024804` | `ACTVOSRDY` 0 or 1: **not predicted** | Figure 21 (p. 263) draws it low until the supply is programmed. Whether silicon reads 0 here is what this row measures (F-VOS) |

**After a power cycle, then a boot of the new `sentinel-02-bench.elf`** (sha256 in §2), halted after the boot word:

| Read | Predicted | Class |
|---|---|---|
| `PWR_CR3` `0x5802480C` | `0x0000_0042`: `0x46` with `SCUEN` 0, `LDOEN` 1, `BYPASS` 0 written and bit 6 kept | generator (the write). Inferred that `SCUEN` reads as written: p. 308 does not say, and Figure 21 draws `SCUEN` falling at "Supply configuration programmed" |
| `PWR_CSR1` under `0x0000_E000` | `0x0000_A000`: `ACTVOSRDY` 1, `ACTVOS` `10` | rule for `ACTVOSRDY`; inferred for `ACTVOS`, for which RM0433 gives no encoding |
| `PWR_D3CR` `0x58024818` | `0x0000_A000`: `VOS` `10`, `VOSRDY` 1 | rule (p. 310) |
| word 0 `0x20000000` (DTCM, `link.x`) | `0x2100_B007` | generator |

**After a warm reset** (`$P reset`, no power cycle). `CR3` keeps whatever was written since power-on (p. 307), and the outcome follows from `LDOEN`/`BYPASS` as read back:

| What ran since power-on | `CR3` before our write | Outcome |
|---|---|---|
| nothing, or `3304`'s ELF `375fd31a` (it writes no `PWR`) | `0x0000_0046` | our write takes; `0x42`; boot `0x2100_B007` (generator) |
| the new ELF itself | `0x0000_0042`, locked | our write is ignored; `LDOEN` 1, `BYPASS` 0 read back; boot `0x2100_B007` (generator) |
| ArduPilot (`PWR->CR3` = `LDOEN \| USB33DEN`, `stm32h7_mcuconf.h` L92) | `0x0100_0002`, `USB33RDY` as the USB supply gives | boot `0x2100_B007`: `LDOEN` 1, `BYPASS` 0 (inferred: that the ChibiOS write is whole and lands; ChibiOS's `hal_lld.c` was not read at a pinned source) |
| a firmware that selected bypass | `BYPASS` 1 | `SUPPLY_NOT_DECLARED`, word 0 = `0x2100_B0F1`, no `D3CR` write. **Not predicted for the bench:** no such firmware is known to be on it |

Which firmware is on the board when the row runs is Scotty's to record. The row can fail: a `D3CR` of `0x6000` after the boot, or any word 0 other than `0x2100_B007`.
