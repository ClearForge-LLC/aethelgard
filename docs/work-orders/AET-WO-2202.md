# AET-WO-2202 — The generated firmware completes one I2C transaction with the sensor model

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2202-i2c-transaction`.
**Author:** Claude (architect) · **Builder:** a cheap builder, with a targeted adversarial pass on the judge (`docs/roadmap.md` §6).
**Phase:** P2. It closes the gate's first clause: "The P1 firmware image boots in the container's Renode and logs at least one complete I2C transaction with the sensor model, with no host interaction." It runs in parallel with `AET-WO-2200`, behind a fence.

**Grounds:**
- **`AET-WO-2100`.** The reference firmware configures every solved pin and peripheral and writes the boot word. It talks to no device.
- **`AET-WO-2101` S3, measured.** A firmware read of the BME280's ID register on I2C1 works in Renode 1.17.0:
  - the device is at address 0x76, the register is 0xD0, and the value is 0x60;
  - Renode's stock `I2C.BME280` model answers;
  - the controller is `STM32F7_I2C` at `0x40005400`, using the I2Cv2 write-then-read sequence.

  An absent device times out at TXIS. The probe's source is `spike/2101/renode/fw/` on PR #66's branch. Read it for the register sequence; do not copy its `unsafe` style.
- **`architecture.md` §2.16:** Renode 1.17.0 has a `BME280` model. It has none for the TMP117 or TMP102.
- **`INV-07`:** what the judge expects is never taken from the generator's code.

> **What this is:**
> - a part-identity table;
> - the generated firmware reading each modelled I2C device's identity register once;
> - the judge checking each result against the table.
>
> It is NOT:
> - the analog bridge or any fault (`AET-WO-2200`);
> - a driver API for users;
> - a change to the board crate's typestate API.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **Part identity facts, in `aethelgard-ir`.** Add a `parts` module that maps a part name to `Option<Identity { register: u8, width: u8, value: u32, source: &str }>`. The source is a datasheet citation.
   - **BME280:** register 0xD0, width 1, value 0x60 (Bosch BST-BME280-DS002, §5.4.1).
   - **TMP117:** register 0x0F, width 2, value 0x0117 (TI SNOSD82D, §7.6.1.8).

   A part with no entry has no identity read.
2. **What the twin can model, in `aethelgard-twin`.** A table of the parts with a stock Renode 1.17.0 model. Today that is `BME280` only, as `I2C.BME280`.
   - The `.repl` emitter declares one model instance for every solved I2C device whose part is in that table, at its solved bus and address.
   - A device whose part has no model is listed in the platform's provenance comment as **unmodelled**. The judge reports it as a finding: `device_unmodelled`, severity info. It is never silently passed.
3. **The firmware, in `aethelgard-svd-hal`'s generator.** For each solved I2C device whose part has both an identity entry and a twin model, `main` performs one identity read, then writes the boot word:
   - the write is the register address;
   - the read is `width` bytes;
   - every wait is bounded, and the bound is a named constant;
   - only the board crate's register API is used.

   The result goes in a per-device record at a fixed RAM layout after the boot word: status (OK, TXIS timeout, RXNE timeout or STOP timeout) and the bytes read. The layout is the twin's contract, held in `aethelgard-twin::boot`, with a belt-and-suspenders copy in the generator (§2.15). The guard tests that the two agree.
4. **The judge.** The judge is `aethelgard-twin::boot::expected` together with the CLI's boot judging. For each modelled device it reads the record.
   - **Pass:** status OK and the value equal to the identity table's.
   - **Otherwise a named `fail`:** `i2c_identity_mismatch` or `i2c_timeout`, with the device reference, address, expected value and observed value or status.
   - The verdict's `boot` object gains `i2c: [{reference, address, register, expected, observed, status}]`.
5. **The regeneration guard** covers the new firmware and `.repl` bytes, as it does today.

## 2. Invariants and fences

- `INV-01`, `INV-06`, `INV-07`. No new `unsafe` in any workspace crate; the generator writes the firmware's as text.
- **Fence:**
  - `aethelgard-ir`: the new `parts` module only;
  - `aethelgard-twin`: `boot.rs`, `platform.rs` and the model table;
  - `aethelgard-svd-hal`: `board/firmware.rs`;
  - the CLI's boot-judging code in `renode.rs` and `verdict.rs`;
  - the committed `boards/sentinel-01/` regeneration;
  - tests;
  - the CLI README's "Booting firmware" section;
  - this work order's FEEDBACK.
- **`AET-WO-2200` runs in parallel.** It touches `aethelgard-ir`'s `SolvedTopology`, a new `aethelgard-twin` analog module, the CLI's `simulate` analog pass and `bridge/`. Do not touch those. Expect to merge `main` if it lands first.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **The tier-2 test:** `simulate --firmware --spec` on the reference specification reads U2 (BME280, 0x76) as 0x60 and passes, and two boots agree.
- **The verdict lists U3** (TMP117) as `device_unmodelled`.
- **A tier-2 negative:**
  - a platform with the BME280 declaration removed, built by hand in the test, fails `i2c_timeout` at TXIS;
  - a firmware whose expected value is changed by one bit fails `i2c_identity_mismatch`.
- §1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — targeted, on the judge

1. Pass a device read without the transaction happening: preload the record, or skip the read.
2. Make the judge accept a value taken from the firmware instead of the table.
3. Hide an unmodelled device.

## 7. Flag-and-stop conditions

- A change outside the fence.
- Renode's `BME280` does not answer 0x60 in the generated firmware's sequence.
- A test that can only go green by being weakened.
