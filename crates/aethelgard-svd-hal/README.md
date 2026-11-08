# aethelgard-svd-hal

Vendor data for the STM32H753ZI (LQFP144), and the board-crate generator.

- `pin_table("STM32H753ZITx")`: the pin and alternate-function table, parsed into `aethelgard_ir::PinTable` for the pin solver (`AET-WO-1200`).
- `cmsis::register_facts()`: base addresses, register offsets and bit positions, from ST's CMSIS device header (`AET-WO-1400`).
- `board::generate(crate_name, &SolvedTopology, &PinTable)`: the source of a `#![no_std]`, dependency-free board crate, as a map from relative path to text (`AET-WO-1400`, `RUL-04` as amended on PR #42).

It knows one part.

## Source and licence

Two of ST's XML files from [`STM32_open_pin_data`](https://github.com/STMicroelectronics/STM32_open_pin_data), commit `7d1f1514ed5583ec5007ad91236b4e1d377295b1`, are vendored byte for byte under `data/st-open-pin-data/`, with ST's `LICENSE` (BSD-3-Clause, © STMicroelectronics) and a `SOURCE.md` giving each file's original path and sha256. ST's notice travels with the data (`RUL-11`); the engine's own licence is unchanged. They are embedded with `include_bytes!`; nothing is fetched at build or test time.

## The parse

Before anything is parsed, each file's sha256 is checked against the constants in `src/lib.rs`. Parsing uses `roxmltree` with its defaults (a DTD is refused).

1. **Which pins.** `<Pin>` elements of the MCU file with `Type="I/O"`: 114.
2. **Canonical names.** The leading run matching `P[A-K][0-9]{1,2}` (`PC14-OSC32_IN (OSC32_IN)` is `PC14`, `PC2_C` is `PC2`).
3. **Which alternate functions.** For each pin, the GPIO file's `<GPIO_Pin>` whose `Name` equals the MCU file's full pin name; each `<PinSignal>` that has a `GPIO_AF` value and is also a `<Signal>` of that pin in the MCU file. The GPIO file covers every package, so this intersection keeps other packages' functions out.
4. **AF numbers.** From `GPIO_AF<n>_<name>`, `n` in 0..=15.
5. **Left out, not refused.** Power, reset, boot and mono-IO pins; signals with no `GPIO_AF` value (analog and additional functions).

## Refusals (`PinTableError`, `kind()`)

- `UnknownPart`: any part but `STM32H753ZITx`.
- `SourceHashMismatch`: a vendored file's sha256 differs from the pin; names the file.
- `MalformedSource`, for each of: not UTF-8 or not XML, or a DTD; not the `<Mcu>` of `STM32H753ZITx`, or no `Package`; an I/O pin with no canonical name (a third digit counts: `PA123` is refused, not read as `PA12`); two I/O pins with one canonical name; a missing, non-numeric or repeated `Position`; a pin missing from the GPIO file, or a `<GPIO_Pin>` listed twice; an AF value of any other form (naming the pin and signal); a signal with two different AF numbers on one pin; a pin key or signal name that fails `RUL-14`.

## Adding a part

A new pinned commit and new hashes, by work order: add the files and update `SOURCE.md` and the constants in the same change. Never hand-edit a vendored file; the hash check would refuse it.

## Register facts (`cmsis`)

ST's `Include/stm32h753xx.h` from [`cmsis-device-h7`](https://github.com/STMicroelectronics/cmsis-device-h7), commit `8f922cdc7cc6de2344e75ddd657889f4ff761790` (tag `v1.10.7`), is vendored byte for byte under `data/st-cmsis-device-h7/` with ST's `LICENSE.md` (Apache-2.0, © STMicroelectronics) and a `SOURCE.md` giving each file's path and sha256. Its hash is checked before it is read. The parse is not a preprocessor, and refuses (`RegisterError`: `SourceHashMismatch`, `MalformedSource`, `Undefined`) anything outside three shapes:

1. `#define NAME VALUE`, where `VALUE` is a literal, a name, or a sum of them. A name defined twice, a chain deeper than 8, a shift or a cast is refused when it is asked for.
2. `typedef struct { … } <KIND>_TypeDef;`, whose every field is a `uint32_t` or an array of them. Offsets come from field order, and every `Address offset:` comment must agree. A field of another type is refused.
3. A peripheral's clock gate, `RCC_<register>_<peripheral>EN_Pos`, which must name exactly one register.

Field widths (MODER, OTYPER, OSPEEDR, PUPDR, AFR) are the difference between pin 1's and pin 0's positions. A test pins the offsets the `AET-WO-1400` spike used: RCC `AHB4ENR` 0x0E0 and `APB1LENR` 0x0E8, GPIO `MODER` 0x00 and `AFR` 0x20 and 0x24, I2C `CR1` 0x00, `TIMINGR` 0x10 and `TXDR` 0x28.

## The board crate (`board`)

`generate` reads a `SolvedTopology`, never a `Specification` (`RUL-16`), and the part's `PinTable`, against which every solved alternate function is checked. It writes five files:

- `Cargo.toml` has no dependencies and no `[workspace]` key, because `aethelgard prove` judges the crate with no network.
- `Cargo.lock` is what cargo writes for such a package.
- `src/lib.rs` is a shell holding `#![no_std]` and the two modules.
- `src/board.rs` holds the typestates. Each solved pin has one method, into its solved function. Each port's pins exist only once its clock is enabled. Each peripheral's `init` takes only its own pins, configured, from `Clocked`. A sealed `AtMostCeiling` trait admits the I2C speeds at or under the ceiling. `Registers` is the one door to the hardware, and `mmio` is its only implementation at real addresses and the crate's only `unsafe`.
- `src/proofs.rs` is the Kani harness, behind `cfg(kani)`. Its oracle is computed by a separate emitter from the plan, never read back from `board.rs`.

The two modules carry `#[rustfmt::skip]`, because `cargo fmt --all` reaches the crate through this crate's dev-dependency. The generator, not the formatter, owns their bytes.

Pin roles' output type, speed and pull are the generator's policy (`Role::policy`), not register facts. `I2C_TIMINGR` values and SPI frame settings are placeholders until a clock tree is solved.

**Refusals** (`BoardError`, `kind()`): `InvalidCrateName`, `InvalidTopology` (`SolvedTopology::validate` refused it), `ReservedPin` (the solver's `RESERVED_PINS`: SWD and crystal pins), `InvalidPin`, `UnknownPin`, `AssignmentMismatch` (a solved AF the pin table does not give that pin), `UnsupportedPeripheral` (anything but `I2Cn` and `SPIn`), `UnsupportedSignal`, `IncompletePeripheral`, `CeilingMismatch`, `CeilingTooLow` (unreachable behind `validate`), and the register errors.

**The I2C ceiling.** `SolvedTopology.i2c_ceilings` (`AET-WO-1202`) gives each I2C bus's maximum speed. The topology does not say which peripheral serves which bus, so `generate` needs exactly one ceiling per I2C peripheral (`CeilingMismatch` otherwise). It gives every I2C peripheral the lowest of them: exact for one bus, and never above any bus's ceiling for more.

**The reference board** is `boards/sentinel-01/`, generated from `examples/sentinel-01-minimal.json` and committed. The root `Cargo.toml` excludes `boards/`. Its tests are:
- `tests/board.rs` (gate clause 4): every byte of the tree equals the generator's output, the harness included, with a negative control. `AETHELGARD_BLESS=1 cargo test -p aethelgard-svd-hal --test board` rewrites the tree. The directory must exist for cargo to load the workspace.
- `tests/board_ui.rs` (clause 3): trybuild, with snapshots regenerated by `TRYBUILD=overwrite` whenever the board is.
- `crates/aethelgard-cli/tests/board_tier2.rs` (clause 5, tier 2): `aethelgard prove`, plus three seeded bugs that must fail.
