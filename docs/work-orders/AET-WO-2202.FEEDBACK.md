# AET-WO-2202 — FEEDBACK

**Branch:** `wo/aet-2202-i2c-transaction` · **Base:** `ec218d9` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only, with one stop at §7 (F1) and the architect's ruling, below.

## F1 The I2C read path (ruled; the fence widened)

**What stopped the build.** §1.3 says the generated firmware performs the read "using only the board crate's register API". That API had no read path:
- `Registers` is sealed, and every method takes a `Token` only the board crate can make;
- `I2c1<Enabled<S>>` had one method, `write_byte`, which writes `TXDR`.

So the firmware could not write `CR2` or read `ISR`/`RXDR`/`ICR` without a board-crate change (outside the §2 fence, and the WO's "NOT" list) or raw MMIO (a break of `INV-02`; `AET-WO-2100.FEEDBACK.md` rejected it for the UART).

**Ruling (architect, 2026-11-09, `~/.cache/aet-2202-ruling.md`): option (a), additive only.**
1. The fence widens to `crates/aethelgard-svd-hal/src/board.rs` (the generator), its generated Kani harness and trybuild tests, and the `boards/sentinel-01` regeneration.
2. One generated, typestate-gated, bounded transfer on `I2c1<Enabled<S>>`: write N then read M, or only write; each wait bounded by a named constant; a typed result; never panics; every existing signature byte-identical.
3. The generated harness gains a proof over it on the harness-owned image.
4. trybuild gains one compile-fail case on a non-`Enabled` I2C.
5. No raw MMIO in the firmware; no `unsafe` outside the board crate's `mmio` (`RUL-04`).

**What was built** (all in `board.rs`'s generator; the diff of `boards/sentinel-01/src/board.rs` is additions only, 0 deletions):
- `pub const I2C_WAIT_SPINS: u32 = 20_000` (the spike's bound), `pub enum I2cError { Length, Txis, Tc, Rxne, Stop }` and a private bounded `i2c_wait`;
- `I2c1<Enabled<S>>::transfer(r, address, write, read) -> Result<usize, I2cError>`, and `transfer_within(…, spins)` (`#[doc(hidden)]`), which `transfer` calls with `I2C_WAIT_SPINS` and which the proof calls with a bound of 2. Every register offset and bit position is read from ST's header (`CR2`, `ISR`, `ICR`, `RXDR`, `SADD`, `RD_WRN`, `NBYTES`, `START`, `AUTOEND`, `TXIS`, `TC`, `RXNE`, `STOPF`, `STOPCF`, `NACKCF`);
- `i2c1_transfer_is_bounded_and_ordered` (Kani, with an `I2c1Spy` that implements `Registers`): only I2C1 registers are touched; the START precedes every poll; `TXDR` is written only in the write phase and `RXDR` read only in the read phase; the read phase begins only after the write phase's `TC` wait; `ICR` is cleared last and to exactly `STOPCF | NACKCF`; no wait reads `ISR` more than its bound; `Ok` means every byte moved. Seven covers (wrote-then-read, write-only, read-only, and each of the four timeouts) keep it non-vacuous;
- `tests/ui/fail/transfer_before_enabled.rs` (E0599 on `I2c1<Clocked>`), and `valid_driver.rs` now calls `transfer`.

**Variances from the ruling's wording, for the architect:**
- The ruling's timeouts are `Txis`, `Rxne`, `Stop`. The measured sequence (`AET-WO-2101` S3) waits `TC` between the write and the repeated-start read, so I kept the wait and gave it a fourth variant, `Tc`. Dropping it to fit the enum would have changed the measured sequence. `Length` is a fifth, for a direction over 255 bytes (`NBYTES` is eight bits): the other choice was a panic or a silent truncation.
- On a timeout or a completed transfer the code clears `ICR` with `STOPCF | NACKCF`. Renode's `STM32F7_I2C` logs `Unhandled write … NACKCF` at WARNING for it. The judge treats only `[ERROR]` as a finding, so this is noise, not a failure; clearing `STOPCF` alone would drop the NACK clear on silicon.
- The write-only path (`AUTOEND` on the write phase, then `STOPF`) is proven but was not measured in Renode: the reference firmware only uses write-then-read. It has no test at tier 2.

## F2 The devices are not in `SolvedTopology` (signature changes, not a fence change)

The work order's judge, emitter and generator take `&SolvedTopology`, which holds no devices: a device's reference, part and address are in the specification, and its peripheral needs `bus.sda.net → assignments[net].peripheral`. `SolvedTopology` is `AET-WO-2200`'s struct, so I did not add to it. Instead:
- `aethelgard_ir::parts::i2c_targets(&Specification, &SolvedTopology) -> Vec<I2cTarget>` (in the allowed `parts` module) is the one derivation, in the specification's order, used by the generator's caller, the platform emitter and the judge. A device on an unplaced bus is a refusal (`DeviceNotPlaced`, exit 2).
- `platform::emit`, `platform::instances` and `boot::expected` take `&[I2cTarget]`. `board::generate` is unchanged; `board::generate_with(…, devices)` is new and `generate` calls it with `&[]`.
- `twin::model::answerable(devices)` is the one filter ("has a Renode model") the firmware's generator, the platform and the judge share. `aethelgard-svd-hal` cannot depend on the twin, so the caller passes the filtered list to `generate_with`; `tests/board.rs` proves the firmware reads exactly the devices the judge checks.
- `main.rs` changes are inside the `Command::Simulate` arm, `Boot`, `ready()` and `boot_report()`.

## F3 Acceptance: what was built and where it runs

| Clause | Evidence |
|---|---|
| U2 (BME280, 0x76) reads 0x60 and passes; two boots agree | `twin_tier2::the_reference_firmware_boots_and_two_boots_agree` (asserts `boot.i2c`, equal `boot` objects) |
| U3 (TMP117) listed as `device_unmodelled` | same test; `boot_tier1::a_passing_identity_read…` |
| Negative: absent device → `i2c_timeout` at TXIS | `a_read_no_model_answers_fails_as_a_timeout_at_txis` and `a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record` |
| Negative: value one bit off → `i2c_identity_mismatch` | `a_value_one_bit_off_fails_as_an_identity_mismatch` (record 0x160 for 0x60) |
| Renode's `BME280` answers 0x60 in the generated sequence (§7) | measured: it does, first run |

**Deviation (negative A).** §3 asks for "a platform with the BME280 declaration removed, built by hand in the test". `simulate --firmware` refuses any platform other than the emitted one (`PlatformNotEmitted`, `AET-WO-2100` R6), so that platform cannot go through the CLI, and I added no seam to the byte-compare. The negative is covered two ways: (1) through the CLI, the firmware reads 0x77, where the emitted platform declares nothing, which is the same absent-device behaviour; (2) with Renode run directly in the test, on the emitted platform with `u2`'s line removed by hand, the record reads `0x22020001` (TXIS) and with the line present it reads OK and `0x60`.

**Another finding in the same place.** The judge's severity-`info` finding has to leave the verdict `pass`. `Verdict::new` computed `ok = findings.is_empty()`; it now passes when every finding has severity `info`. Only the boot path makes an `info` finding (KiCad's severities are `error` and `warning`). `boot_tier1`'s and `twin_tier2`'s `kinds()` helpers skip `info` findings.

## F4 Adversarial pass (targeted, on the judge), by the builder

The record is the firmware's account of the transfer, so the judge's job is to leave the firmware no way to supply a pass it did not earn.

1. **Pass a read without the transaction.**
   - *Preload the record.* The judge writes `0xFFFFFFFF` to both words of every record after `LoadELF`, as it does the boot word (`BootPlan::commands`), so bytes loaded from the image are overwritten. A record the firmware never writes reads as the fill and fails `i2c_record_invalid`. Tier 1: `a_record_the_firmware_never_wrote_or_wrote_oddly_fails` (fill, status 0, an unknown status, swapped words). Tier 2, on Renode: `a_firmware_that_skips_the_read_or_its_record_cannot_pass`.
   - *Skip the read.* The buffer stays zero: status OK with value 0 is an `i2c_identity_mismatch`.
   - *Written but not performed.* The first build left this open: a firmware that writes `STATUS_OK` and `0x60` without calling `transfer` passed. I measured the witness (`I2C1.CR2 = 0x020104EC`, `I2C1.TXDR = 0xD0` after the reference boot, both stored by Renode) and proposed it as backlog; the architect's review made it blocking (W1) and it is built, below.
2. **Make the judge accept a value taken from the firmware.** Nothing the firmware writes names the expected value: `I2cCheck.expected` is `aethelgard_ir::parts::identity(part).value`. Tier 1: `the_expected_value_is_the_tables_whatever_the_record_says…` (0x61 observed, 0x60 expected). Width 2: `a_two_byte_identity_is_judged_whole_and_most_significant_byte_first`.
3. **Hide an unmodelled device.** A part is modelled only on an exact match (`bme280`, `BME280 `, `BME 280`, empty and a zero-width-space name are all `device_unmodelled`; unit test). `device_unmodelled` is reported on every boot that reaches the judge, including a failed one (`a_record_is_not_judged_when_the_boot_word_is_missing`), and in the platform's header comment. A modelled part with no identity would be declared and never read or reported; none exists today, and `every_modelled_part_has_an_identity…` makes adding one a failing test. A bounded kill (`wall_bound_exceeded`, `cpu_aborted`) reads nothing, so it reports no unmodelled device: those verdicts are already `fail`.

**Mutants (one each, against the proof they live in).** Five mutants of the generated board crate were run through `aethelgard prove --harness i2c1_transfer_is_bounded_and_ordered` in the tools image; all five fail: the bound (`n <= spins`) `proof_failed`; the `TC` wait removed `proof_failed` ("began without waiting for TC", added after a first run showed this mutant caught only by a cover); a read of `Block::Rcc` `proof_failed`; `ICR` not cleared `proof_failed`; a timeout returned as `Ok` `proof_failed`.

## W1 The controller-state witness (architect's review of PR #69, blocking; built)

After the run the judge reads `<bus>.CR2` and `<bus>.TXDR` for the last identity read on each bus. The expectation comes from the solved address, the part table and the header, never from the firmware (`INV-07`):
- `SADD` = address << 1 (the 10-bit field), `RD_WRN` = 1, `NBYTES` = the identity's width, `AUTOEND` = 1, and `TXDR` = the identity register. `START` (not stored once set) and every other bit are outside the masks.
- It is judged only when that record's status is OK, i.e. when the firmware claims the read completed. A record that says a wait expired claims nothing, and `i2c_timeout` alone names the failure. The same holds when the boot word is missing.
- A difference is `i2c_transaction_unwitnessed`, naming the device, its address and the bus, and each differing field with its expected and observed value. `boot.i2c_witness` in the verdict lists both registers (address, mask, expected, observed).
- The two reads are appended after the records, so the register list and its indices did not change: the "renumbering" the review accepted as a cost was avoided.
- `CR2.SADD` (10 bits) and `CR2.NBYTES` (8) widths are stated in `boot.rs` from RM0433 §48.7.2, labelled `belt-and-suspenders:`, because the header reader does not evaluate the `_Msk` shifts and `aethelgard-svd-hal`'s `cmsis.rs` is outside the fence; the positions come from the header.

**Tests.** Tier 1: unit tests for the expectation (it equals the measured `0x020104EC`/`0xD0`; each of the five fields differs alone; a controller no transaction touched differs in all five; only the last read on a bus is witnessed) and `boot_tier1` cases per field (`SADD`, `RD_WRN`, `NBYTES`, `AUTOEND`, `TXDR`) with the message naming the field, a forged record over an untouched controller, the not-judged cases and the pass with `START` and stray bits set. Tier 2, on Renode: `a_forged_record_without_a_transaction_fails_on_the_controller_state` (the firmware writes `STATUS_OK` and `0x60` and never calls `transfer`; the only finding is `i2c_transaction_unwitnessed`, all five fields named), and the skipped-read test now also fails it.

**Mutants (one each, tier-1 suites).** The witness judged whatever the record claims → `the_controller_is_not_judged_for_a_record_that_claims_nothing` fails; `AUTOEND` dropped from the expectation and the comparison → three tests fail; the witness taken for the first read on a bus instead of the last → `only_the_last_read_on_a_bus_is_witnessed` fails. (The third mutant first survived because `rustfmt` had split the line my `sed` matched, so it was not applied; it was reapplied and killed.)

**Residual, recorded as the review asks.** Only the last transaction on each bus is witnessed by controller state. Today each bus has one modelled device. A bus with two needs a per-transaction witness (a later work order).

## W2 Carried: `parts::i2c_targets` is interim

Per the review, `i2c_targets(&Specification, &SolvedTopology)` is interim (F2). Once this PR and `AET-WO-2200` merge, the device list moves into `SolvedTopology` as `i2c_devices` (`RUL-16`: every derived fact lives there, so the generators never see the specification) and `i2c_targets` is deleted. The architect is writing that as a small work order. `platform::emit`, `platform::instances`, `boot::expected` and `board::generate_with` take the list today and would take the topology alone then.

## F5 Other findings

- **`i2c_record_invalid`** is a kind the WO does not name. A record that is neither a status the firmware writes nor the judge's fill has no honest name among `i2c_identity_mismatch` and `i2c_timeout`.
- **`board_tier2::a_write_past_the_block_fails`** seeds a mutant at `r.write(TOKEN, Block::I2c1, 0x28, u32::from(byte));`, a site the new `transfer` now repeats, so the test's own uniqueness assertion failed. I named the site by `write_byte`'s signature. It is the same mutant, so the test is not weakened; it is the only existing test changed in its meaning.
- **The record layout and statuses** live twice, labelled `belt-and-suspenders:` (`aethelgard-twin::boot` and `board/firmware.rs`); `tests/board.rs::the_firmware_follows_the_judges_record_layout` compares them and the reads.
- **A podman run without `--memory-swap`.** The first full tools run used `--memory=28g` alone; the kickoff said `--memory` and `~/.claude/CLAUDE.md` now adds `--memory-swap`. The final run uses both, and its `memory.max` was read back (below).

## Reduction pass (diff)

−6 lines net at the first close (1 duplicate removed: `model::listed` repeated `platform::short`); for W1: nothing to remove, the witness adds one labelled constant pair and no copy of a fact (its field positions are the header's, its expectation the table's) · 3 protected kept (the record layout and statuses, and the two field widths, labelled) · 0 findings beyond the above.

## Gates

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (also with `--features aethelgard-cli/with-tools`) and `cargo test --workspace`: clean on the host.
- The tools job's steps in the pinned image (`container/IMAGE`), rootless podman, `--memory=28g --memory-swap=28g` (read back: the container scope's `memory.max` and `memory.swap.max` were both 30064771072), `cargo test --workspace --locked --offline --no-fail-fast --features aethelgard-cli/with-tools`: no failure, including `twin_tier2` (10 tests plus the 3 added), `board_tier2` (Kani, with `a_write_past_the_block_fails` as in F5) and the `prove` of `i2c1_transfer_is_bounded_and_ordered`.
- CI: (run id and result added below once green).
