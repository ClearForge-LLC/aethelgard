# AET-WO-2203 — FEEDBACK

**Branch:** `wo/aet-2203-solved-devices` · **Base:** `ad3321c` · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace · **Cadence:** build-only. No §7 condition fired.

## What was built

- `SolvedTopology.i2c_devices: BTreeMap<reference, SolvedI2cDevice { index, part, bus, peripheral, address }>`, required in the JSON (a topology without it is refused). `solve` fills it from the specification, on the peripheral that carries the bus's `sda` net, as `i2c_targets` did.
- `SolvedTopology::i2c_devices_in_order()` yields `(reference, &device)` sorted by `index`. Every consumer iterates it and never the map, whose order is the references' (`U10` sorts before `U3`).
- `parts::i2c_targets`, `parts::I2cTarget` and `parts::TargetError` are deleted. `DeviceNotPlaced` is now a `SolveError` variant (same kind string, `id` the device).
- Consumers: `platform::{emit, emit_faulted, instances, present}`, `boot::{i2c, expected}` take the topology alone. `board::generate_with` takes a predicate (`answers: &dyn Fn(&str) -> bool`) in place of the device slice, because `aethelgard-svd-hal` cannot depend on the twin's `model`. `model::answerable` is deleted. The CLI's `Boot` and `Ready` no longer carry devices.

## Goldens: old and new

| Golden | Old | New |
|---|---|---|
| `solve_cli.rs` `REFERENCE_DIGEST` (the success line for `examples/sentinel-01-minimal.json`) | `1b98f4c626d7e6cbae798d42514364edcfb8a89aa2cba6a9a19ce878a1cb4049` | `d684cd6ddd6788ba4617118ec624a199de0040ca0f25131b6f0f810ccd10084f` |
| `solve_cli.rs` `STRESS_DIGEST` | `d5f2741684970dc6e43738d2522d88d1fa741702f4e174f25a1db43a286d9300` | `dd192c61982451df5c39fc522d10b598568748e5558b2f6a7aca587eaad9a4e6` |

The only change is the added `"i2c_devices"` object. The twin's `REFERENCE` topology constant and the CLI's analog-deck fixture gained the same object, written to match `solve`'s output.

**Every emitted byte is unchanged.** The regeneration guard (`aethelgard-svd-hal/tests/board.rs`) passes with no `AETHELGARD_BLESS`, and `git status` shows nothing under `boards/` or `examples/`.

## The blind spot, and the test for it

The committed board's devices are `U2`, `U3`, already in reference order, so a change that sorted devices by reference would pass the regeneration guard. `aethelgard-svd-hal/tests/order_2203.rs` solves a specification that lists `U10`, `U3`, `U2` (two BME280, one TMP117) and pins the sha256 of the firmware `main.rs`, the platform, the faulted platform and the boot image. The four digests were taken from the code before the change (`i2c_targets` in the caller) and are identical after it.

## Adversarial pass (targeted)

One mutant each, against the suites the guard lives in:

| Mutant | Caught by |
|---|---|
| `i2c_devices_in_order` does not sort (reference order) | `devices_come_out_in_index_order_not_reference_order`, `order_2203` |
| `solve` writes `index: 0` | `order_2203` |
| `validate` drops the peripheral check | `solved_refuses_a_device_on_an_unknown_bus_or_the_wrong_peripheral` |
| `validate` drops the index check | `solved_refuses_indices_that_are_not_exactly_zero_to_n` |
| a `&Specification` reader added to `aethelgard-twin/src` | `nothing_outside_solve_reads_devices_from_a_specification` |

Each was reverted and the file compared with the saved original.

## Items for the architect

| # | Item | Status |
|---|---|---|
| F1 | **`validate`'s wording.** The WO says "a bus that is not in `assignments`". `assignments` is keyed by net, not by bus id. I read it as a key of `i2c_ceilings` (the bus-id map, as `validate_analog_lines` does) and added the matching check that the device's `peripheral` is the one its bus's analog lines were solved to. The address check reuses `I2cAddressOutOfRange` (0x08..=0x77, which is 7-bit and what a specification may declare). Indices must be exactly `0..n`, each once (`I2cDeviceIndex`), or the order is ambiguous. | built; confirm or narrow |
| F2 | **§1.3 as literally worded is not met; I scoped the grep rather than stop.** `aethelgard-eda` still reads the specification's devices (`schematic.rs`, two places). It is outside the §2 fence, and the schematic is design intent rather than a firmware, platform or judge input, which is what `RUL-16`'s grounds cite. Stopping on a change outside the fence seemed worse than a pinned exception. The test covers `svd-hal`, `twin`, `chaos` and the CLI, and pins the eda readers at exactly two, so a third fails. Whether the schematic should read the topology is a ruling. | open: rule on eda |
| F3 | **`DeviceNotPlaced` is unreachable.** `Specification::validate` has already refused an unknown bus, and `solve` places every bus, so no input reaches the variant. The WO says to keep the refusal, so I did; no test can reach it. The CLI no longer maps it by hand. | kept per WO; a candidate for the next reduction pass |
| F4 | **`TwinError::DeviceOffBus` is now nearly unreachable.** `validate` requires a device's peripheral to be its bus's solved one, which must be an `I2C<n>` line. The check stays for an `I2C<n>` the stock file lacks. The test that provoked it by hand-editing a device's peripheral now meets `InvalidTopology` first, so that assertion is gone. Likewise a device key that is no identifier (`U 2`) is an `InvalidTopology` before it is an `InvalidDeviceName`; the test says so. | noted |
| F5 | `crates/aethelgard-cli/README.md` named `parts::i2c_targets`; the sentence now names `SolvedTopology::i2c_devices`. A one-line doc fix outside the listed fence. | noted |
| F6 | A stored `SolvedTopology` JSON from before this WO has no `i2c_devices` and is refused (`INV-06`). None is committed. | noted |

## Verification

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the same clippy with `--features aethelgard-cli/with-tools`: clean.
- `cargo test --workspace --no-fail-fast` on Furnace: every binary passes (tier 1).
- Tier 2 is proven by CI's tools job; I changed `twin_tier2.rs` and `boot_tier1.rs` only to take devices from the topology, and checked that they compile and lint with `with-tools`.

## Reduction pass

Removed: `i2c_targets`, `I2cTarget`, `TargetError`, `model::answerable`, the CLI's `devices` plumbing (`Boot.devices`, `Ready.devices`, the `present` call), and the twin tests' `reference_devices` and `target` helpers. Nothing was added that the WO did not name, apart from F1's checks and the index.
