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
