# aethelgard-svd-hal

Vendor pin data for the STM32H753ZI (LQFP144), parsed into `aethelgard_ir::PinTable` for the pin solver (`AET-WO-1200`). `pin_table("STM32H753ZITx")` is the entry point. It knows one part. The register-block side of this crate (`AET-WO-1400`) is not built yet.

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
