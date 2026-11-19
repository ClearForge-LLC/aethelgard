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

### §1.4 The bring-up (`aethelgard-svd-hal`)

- **`Block::Pwr`** at `0x5802_4800`. Every offset and bit position is read from ST's header (`PWR_TypeDef`, `PWR_CR3_SCUEN/LDOEN/BYPASS`, `PWR_CSR1_ACTVOSRDY`, `PWR_D3CR_VOS/VOSRDY`), and `Board` gains `pwr: Pwr`.
- **`Pwr::scale2(self, r, polls) -> Result<Scale2, PwrFault>`** consumes the singleton, so it runs once. Under `ldo` it:
  1. writes `CR3` to `(read & !0x7) | 0x2` and reads it back;
  2. compares under `0x3` (`LDOEN`, `BYPASS`), and on a mismatch returns `SupplyNotDeclared` with no further write;
  3. polls `CSR1` bit 13 at most `polls` times (`ActvosrdyTimeout`);
  4. writes `D3CR` whole as `0x0000_8000`;
  5. polls `D3CR` bit 13 at most `polls` times (`VosrdyTimeout`).
- **The generator's words are its own:** `PwrFacts` reads the header and the mode, with no default arm. The generator never calls `aethelgard_ir::vos`; only the oracle, the spy harness and the tests do.
- **`PWR_POLLS` = 1,000,000.** DS12117 Rev 10 states no settling maximum for either bit: Tables 16, 30, 115 and 127 were read from the page images. The only figure in either document is a "20 µs" dimension drawn on RM0433 Figure 31 (p. 281), for VOS1→VOS0 only. So the bound is an engine choice, set generously, as `ECHO_POLLS` is.
- **The token.** `Rcc::reset_clock(&self, r, &Scale2)`, so no `ResetClock`, and so no I2C, SPI, USART, timer or kernel-select call, can precede it. `RESET_CLOCK_FIELDS` is unchanged, and `ResetClock`'s documentation says the core is at VOS2. Four ui fail cases show it: `pwr_reset_clock_without_scale2`, `pwr_reset_clock_without_token`, `pwr_forged_scale2` and `pwr_scale2_twice`.
- **`main`:** the sequence is the first hardware access, before the reset-clock read and the `FLASH_OPTSR_CUR` read. On a fault, the fault's word goes to word 0, then the firmware parks. A plan with no clock still runs it and binds `_scale2`; no committed board is such a plan (measured on all 20 that generate), so a test-only GPIO-only topology covers it.
- **Refusals:**
  - `generate` (and `register_fields`) refuses `bypass` as `CoreSupplyNotGenerated` in `Plan::new`;
  - `check_silicon` runs `vos::judge_generated` first and refuses a verdict that does not hold (`VosRuleFails`, by clock). That is stricter than `validate`: a clock with no cited maximum, or a kernel the rule cannot read, is refused there. No committed board hits it.
- **The residue,** written in the firmware module, the ledger and the README:
  - the reset handler's `.bss`/`.data`, the stack and `Board::take`'s `TAKEN` flag are RAM writes made before `ACTVOSRDY` (RM0433 §6.4.1, p. 262: in Run*, "write accesses to the RAMs are not permitted");
  - so are the boot words of `SUPPLY_NOT_DECLARED` and `ACTVOSRDY_TIMEOUT` themselves. **On silicon those two words may not land.** Bench row B-2 therefore reads `PWR_CR3` and `PWR_CSR1` directly, not word 0, for those faults.
- **No solved word moved:** across all 20 boards that generate, no register word other than the new `PWR` ones moved (measured by the svd-hal slice). For `solve`, see §2.

### §1.5 Kani

- **The prelude, not a scheduled action.** Every schedule harness, whole and per part, runs `scale2(&mut img, 1)` with `assume(ok)` before its loop. A `PWR` action owned by one part would overlap the other parts' footprints, since every part needs the token. So `PWR` is in no footprint and no rely; each part's write guarantee skips `Block::Pwr`; and each part's final check fixes `PWR` exactly. This narrows "every order" to "`PWR` first", which is the order `main` takes.
- **The oracle clause:** "PWR is not before plus exactly the declared supply and VOS Scale 2":
  - `after.CR3` equals `before.CR3` except the three supply bits, which are `vos::cr3_supply_bits`;
  - `after.D3CR & !VOSRDY == vos::D3CR_VOS_SCALE2`, with `VOS == 10`;
  - the image holds `D3CR.VOSRDY` read-only and refuses any `PWR` write but `CR3` and `D3CR`.
  - **Deviation:** the WO's "before except VOS" cannot hold for `D3CR`, because the WO also requires a whole-word write. A whole word cannot equal "before except VOS" from an arbitrary starting image, so the clause is the rule's whole word.
- **`pwr_scale2_is_once_in_order_and_bounded`,** on a spy with nondeterministic reads and a bound of two polls. It proves:
  - the writes are exactly `[CR3, D3CR]` with the rule's words;
  - no write follows a fault;
  - each fault has its own name;
  - `Scale2` comes back only when `CR3` read back as declared and both bits read 1;
  - no other block is touched;
  - all four `kani::cover!` outcomes are reached.

  It takes about 1 s (3,819 symex steps).
- **Harness counts** (measured): every board goes from N to N + 1. The committed `boards/sentinel-01` goes 7 → 8 and `sentinel-01-bench` 8 → 9, as §0.2 predicted. `examples/sentinel-01.json` as generated goes 8 → 9, motors-pwm 9 → 10 and Sentinel-03 18 → 19.
- **The cost** on Sentinel-01's `every_valid_sequence`, one sample each; memory is a 1 s poll of summed RSS, not the cgroup peak; §2 has the scope peaks:

  | | Wall | Symex steps | Summed RSS |
  |---|---|---|---|
  | before (`e62eb70c`) | 945 s | 1,461,330 | 11.3 GiB |
  | after | 1,192 s / 1,181 s | 1,532,173 | 13.3 GiB |

  That is about +26 % in time, with about 2.7 GiB left under the 16 GiB cap.
- **Mutants:** K1–K7 were applied to the generated `board.rs` text; M6–M11 to the generator.

  | Mutant | Killed by |
  |---|---|
  | K1, drop the `D3CR` write | the oracle clause in `every_valid_sequence` |
  | K2, write `CR3` twice | **only `pwr_scale2_is_once_in_order_and_bounded`.** A second identical write leaves the same final state, so the oracle clause cannot kill it, though the WO names "the Kani clause" for both mutants (finding F2) |
  | K3–K7: no supply check, no `ACTVOSRDY` wait, no `VOSRDY` wait, a fault's name swapped, `D3CR` before the read-back | the PWR harness |
  | M6, token bypass | `board_ui` |
  | M7, `bypass` not refused | `a_bypass_supply_is_refused_as_not_generated` |
  | M8, no VOS refusal in `check_silicon` | `check_silicon_refuses_a_tree_the_vos_rule_does_not_hold` |
  | M9, a boot word's value changed | `the_firmware_follows_the_judges_pwr_boot_words` |
  | M10, fault-to-word mapping swapped; M11, a hardware access before the sequence | `main_runs_the_supply_and_scale2_first_and_a_fault_writes_only_its_word` |

### §1.6 The twin and the CLI

- **Measured** in the pinned tools image (Renode `1.17.0+20260907gitf1dd1b4af.aet1605`), with a test firmware doing the bring-up's accesses plus a second `CR3` write:
  - **The emitted platform's update entry replaces the stock `pwr` script.** Measured twice: on a hand-written override, and on the platform `generate` emits for `sentinel-01-minimal`.
    - Stock: `CR3` reads `0x4000000`, the write is dropped, and `D3CR` reads `0x2000`.
    - Override: `CR3` `0x46` → write `0x42` → reads `0x42`; a second write is ignored.
    - Override: `D3CR` `0x6000` → write `0x8000` → reads `0xA000`; `CSR1` then reads `0xA000`.
  - **`LogPeripheralAccess sysbus.pwr true` logs the script's accesses** in the shape the parser already reads, e.g. `pwr: [cpu: 0x8000014] WriteUInt32 to 0xC (unknown), value 0x42`.
  - **`machine Reset` re-runs the script's initialization**, so `CR3` reads `0x46` again. The twin's warm reset is a power-on reset for `CR3`: the lock across a warm reset is not modelled at all. This is a measured narrowing of the WO's residue ("only as far as D-3 (a) stores the first write").
- **The override** (`platform::pwr_override`) is written from RM0433. It uses the header's offsets and `serial::PWR_CR3_POR` = `0x46`, and is emitted last on a generated platform only, never when `foreign_clock` is declared. It stores `CR3`'s first write and ignores the rest; stores `D3CR.VOS` with `VOSRDY` 1; reads `CSR1` as `ACTVOS` = the stored `VOS` with `ACTVOSRDY` 1; and answers any other offset as stock does.
  - **Deviation:** `USB33RDY` reads 0, not the stock 1. Rev V's power-on word has it 0; no generated firmware touches USB; and ArduPilot, the reason the stock stub sets it, keeps the stock stub.
- **`PwrRule`** (`twin` `serial.rs` `pwr_rule`, `judge_pwr`) takes `vos::cr3_supply_bits(topology.core_supply)` and `vos::D3CR_VOS_SCALE2`, never `svd-hal`'s words; the offsets are the header's, as the RCC rule's are.
  - One finding, `pwr_access`, joins every departure: a second write, `D3CR` before `CR3`, a wrong supply bit, `CR3`'s other bits not as read, a `D3CR` other than `0x8000`, a `PWR` write after an RCC enable-register write, any other `PWR` write, a byte write, and (beyond the WO) "never written in the boot the reads judge".
  - Only a generated boot is given it. Only RCC enable-register writes count as "after RCC", per the WO's wording; a select write before `PWR` is left to Kani and the reset-clock read.
- **The CLI:**
  - `PWR` and RCC are logged on every generated boot; `PWR` reads are filtered except `CR3`'s, so the polls are not kept.
  - `boot_word_missing` keeps its kind and names the word: `boot_supply_not_declared`, `boot_actvosrdy_timeout`, `boot_vosrdy_timeout`.
  - `Image::unobservable` gains `PWR.CSR1.ACTVOSRDY` and `PWR.D3CR.VOSRDY`, and every generated verdict's `boot.residuals` carries `serial::PWR_RESIDUAL`.
  - `CoreSupplyNotGenerated` (exit 2) is raised at `generate` (and so `verify` and `prove`), `emit` and `ready()` (`simulate`, `campaign`, and `--foreign` too).
- **A `bypass` board's acknowledgements are stale.** It has no generated firmware, so no ledger, and `validate` refuses an acknowledgement as `UnwitnessedAcknowledgementStale`. A `bypass` board without acknowledgements validates. Tested.
- **23 mutants, 23 killed**, all by tier-1 tests (`~/.cache/aet-4240v/a2/py/mutants.log`); `TIER2_MIN` does not move.

### §1.7 The ledger

Nine rows (`svd-hal` `register_fields`, `cli` `ledger.rs`); the reference ledger goes from 289 to 298 rows.

| Row | Access | Class |
|---|---|---|
| `PWR.CR3.SCUEN` = 0, `.LDOEN` = 1, `.BYPASS` = 0 | written (read-modify-write) | `independent-rule`; ignored by the stock model |
| `PWR.D3CR.VOS` = 2 | written (whole word) | `independent-rule`; ignored by the stock model |
| `PWR.D3CR.VOSRDY` = 0 | written (whole word; the bit is read-only, but every field of a whole-word write has a row) | `independent-rule` |
| `PWR.CSR1.ACTVOSRDY` = 1, `PWR.D3CR.VOSRDY` = 1 | polled (`@reset`, the `D2CKRDY` precedent) | `silicon-only` |
| `FLASH.ACR.LATENCY` = 7, `.WRHIGHFREQ` = 3 | depended on at reset | `silicon-only` |

- Each `silicon-only` register is acknowledged by name in every specification that acknowledges: `examples/sentinel-01.json`, `sentinel-01-minimal.json`, and the `four-layer`, `sentinel-01-bench` and `sentinel-03` fixtures.
- The bench rows gain `PWR.CSR1` (`ACTVOSRDY`) and `FLASH.ACR` (`LATENCY`, `WRHIGHFREQ`). `PWR.D3CR.VOSRDY@reset` is not a bench row, because its written row is judged; B-2 reads `D3CR` whole.

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
