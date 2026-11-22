# AET-WO-1217 — predictions, written before the measurement runs

Written before `measure` ran on any file, at main `22a1b3c0` (specification 25, solved 16).

1. **Baseline.** Every committed specification `run.sh` finds (`git ls-files '*.json'` outside `spike/`, carrying `schema_version`, `mcu`, `motor_outputs` and `project_name`) gives the same `validate`, `solve` and `emit` exit codes as the last null sweep recorded (`1216b`: 34 specifications); none that validates today is refused by step 24.
2. **F4, PC2_C and PC3_C.** No committed specification places any signal on `PC2` or `PC3` (the pin table's names for the `PC2_C` and `PC3_C` pads):
   - no bus line: the solver reaches `SPI2_MISO`/`SPI2_MOSI` on `PC2`/`PC3` only when no earlier pin in its fixed order carries them, and `PB14`/`PB15` (and `PI2`/`PI3`) come first in key order;
   - no chip select, pin-device GPIO or motor output: the greedy passes take the first free pin in key order (`PA0` onward), and `PC2`/`PC3` carry no timer channel;
   - no ADC input: a battery sense and a pin device's ADC input take the free ADC pin of lowest package position, and ADC pins below position 28 (`PF3` to `PF10`, `PC0`, `PC1`) come first.
   So the solver's change moves no committed board, and there is no stop for (2).
3. **F9, step 24's order.** No committed specification is refused at step 24 today. On a probe (D1 with VREF tied to `GND` and its reservoir removed), today's kind is the ADC rule's `AdcSourceImpedance`; with `trip()` judged first it is `TripReferenceGrounded`.
4. **F10, an unknown `mcu.part`.** No committed specification carries an `mcu.part` other than `STM32H753ZITx`. On a probe (D1 with `mcu.part` changed to a part with no power-parts row), today's refusal is `PassivePartUnknown` at `pin_devices[0].mcu`, from `analog()`.
5. **F2, the GND tie.** On a probe (D1 with `IPROPI` tied to `GND` and its `analog[]` entry removed), today's refusal is `SignalKindMismatch` at `pin_devices[0].straps.IPROPI`; with the strap kept and the `analog[]` entry also kept, `DevicePinBoundTwice` is not reached because the strap is refused first.
6. **F8, the reservoir.** No held capacitor row is rated between any legal VDDA's minimum and maximum (every held capacitor is rated at 6.3 V or more; `mcu.supply` is at most 3.6 V), so no specification can tell the minimum from the maximum through the embedded table.
