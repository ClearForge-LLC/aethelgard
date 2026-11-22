# AET-WO-4300h — predictions, written before any reader ran

Written 2026-11-22 ~09:55Z at `main` `f46c3c42`. Inputs: the work order, `1216` FEEDBACK (F1, F5, D2 and the ruling), `1216`'s published results (`spike/1216/RESULTS.md`, run at `9cb87b4c`), `3400` FEEDBACK (V4/V5: motors pinned to PE9–PE14 to keep PA8, PC9, PA11/PA12, PD0/PD1 free), and a key-level inventory of the committed specifications (connector and test-point counts, hand pins). No 4300h reader, `solve` or netlist read had run at `f46c3c42`.

## The universe

1. **34 committed boards**, by `spike/1216/universe.sh` (schema 24, an `mcu` object, no `base`): the same 34 as `1216`.
2. **26 draw a board**, one more than `1216`'s 25: `1211f` (#316) made D1 (`d1-drv8876.json`) drawable. D2 and `invalid-zero-domain` still fail `validate`; 3 `solve/` fixtures still fail `solve`; `hse-external-8`, `imu-baro-1v8` and `sentinel-02-bench-hse8` still draw none.

## Connector- and test-point-facing MCU pins

3. **33 of 34 boards** have at least one: all but `invalid-zero-domain`, which declares no connector.
4. **Every one is direct**: the connector pad or test-point pad sits on the MCU pad's own net. **No connector or test point reaches an MCU pin through a series resistor, an inductor or a level translator** on any committed board. The one translator (U13 TXU0304, `sentinel-02-power`) has no connector on its B side.
5. **The TT pins among them** (DS12117 Rev 10 Table 8):
   - `sentinel-03`: **MOTOR1 on PE9 (TT_ha)**, at J5.1 and TP19. A **hand pin** (`motor_outputs[0].pin`).
   - `sentinel-01`, `sentinel-01-bench`, `sentinel-01-ext-bmp581`: **PA5 (TT_ha) and PA7 (TT_a)** at test points, the flash bus's SCK and MOSI on SPI1. **Solver-chosen** (no hand pin), because `PA5` and `PA7` sort first among SPI1's pins.
   - No other board has a TT pin on a connector or a test point.
6. **An undeclared off-board end, listed in its own class:** `sentinel-02-bench` and `sentinel-02-bench-hse8` put the IMU's SPI1 SCK on **PA5 (TT_ha) by hand**, and the IMU and barometer are breakouts wired to the Nucleo's headers (roadmap, `3304`). `connectors[]` declares none of it, so a reader of connectors and test points does not see it. Predicted: 2 boards, PA5 among their header pins. No option as the work order states it judges these.
7. **The non-GPIO pins:** NRST (RST) and BOOT0 (B) face a connector on every board whose SWD connector carries them (the `sentinel-01` family, `sentinel-03`), and their test points on the boards that have them.
8. **Pins holding an internal pull from reset** (`MCU_RESET_PULL_UPS`/`_DOWNS`: PA13, PA14, PA15, PB4): PA13 and PA14 face every SWD connector. PA15 and PB4 face none.
9. **The three Table 8 reads agree** on every listed pin: `power-parts.json` `io.pins`, `spike/1211e/out/table8-text.tsv` and `table8-visual.tsv`. Every TT pin and every proposed FT alternative is confirmed on the rendered page.

## The alternatives

10. **MOTOR1 (TIM1_CH1, dshot600):** on the LQFP144, **TIM1_CH1 is on PA8 and PE9 only.** PA8 (FT_fha) is the one same-channel FT alternative, and it is one of `3400`'s kept-free pins (MCO1's only pin).
    - With `motor_outputs[0].pin` set to `null`, `solve` puts MOTOR1 on **PA8**.
    - With it set to `PA8`: `solve` succeeds. MOTOR1's timer, channel and AF (AF1) do not move, only its pin. The other three motors do not move.
    - Moving all four to an all-FT timer: **TIM4 on PD12–PD15** is free and all-FT on `sentinel-03`. TIM8 and TIM3 on PC6–PC9 also are, but PC9 is MCO2 (kept free).
11. **PA5/PA7 on the `sentinel-01` family (SPI1 SCK/MOSI):** the FT alternatives on SPI1 are **PB3 and PG11 for SCK, and PB5 and PD7 for MOSI**. Hand-pinning SCK to PB3 and MOSI to PB5 solves on all three boards, and moves only those two pins.
12. **The solver's pin table** (parsed independently from the open pin data the solver reads) contains every (pin, signal, AF) that the committed boards' `solve` outputs assign.

## The limits

13. **With VDD up** (Table 13 p. 105, A1's reading at `mcu.supply`'s minimum, 3135 mV on 32 of the 34 boards):
    - TT takes **3435 mV**;
    - FT takes **5499 mV** (`Min(3135 + 3600, < 5.5 V)`);
    - a pin holding an internal pull from reset takes VDD + 0.3 V (note 3), 3435 mV, whatever its structure.

    A "3.3 V" peer at +5 % (3465 mV) exceeds TT by 30 mV. A 5 V peer at +5 % (5250 mV) is inside FT.
14. **With VDD = 0** (Table 10 p. 103): FT and TT both take **4.0 V**, and BOOT0 9.0 V. So moving a pin from TT to FT changes nothing unpowered.

## The options, measured on the committed boards

15. **(a) The solver prefers FT for a connector- or test-point-facing signal and refuses a hand TT pin there without a stated reason:**
    - **refuses 1 board** (`sentinel-03`, MOTOR1);
    - **moves 3 boards** (the `sentinel-01` family, 2 pins each);
    - leaves every other board byte-identical in `solve`.
16. **(b) A declared drive bound per connector pin** (`connectors[].pins[].offboard.drive_max_millivolts` with a source):
    - **b-required** (a connector pin on an MCU net with no bound is refused) refuses **33 of 34 boards**, because no specification declares a bound today;
    - **b-declared** (judged where declared, named where not) refuses **0** today.
    - On hand sidecars:
      - DS-009's nominal 3300 mV on every DS-009 pin refuses **0** with VDD up and **0** at VDD = 0;
      - a 5 V ESC peer (5250 mV) on Sentinel-03's J5 refuses **PE9 alone with VDD up**, and **all four J5 pins at VDD = 0** (5250 > 4000 mV).
    - (b) bounds no test point. A test point is not a `connectors[]` pin.
17. **(c) both:** the union.
    - b-required: 33.
    - b-declared: (a)'s 1 refusal and 3 moves.
    - With the J5 sidecar: (a) moves PE9 to FT, which clears the powered case. The VDD = 0 refusal of all four J5 pins stands.
18. **§7's immediate stop does not fire:** no committed board puts more than Table 13's limit on an MCU pin with VDD up by an on-board path that no option judges. A1 already judges every on-board pull and source with VDD up (`1211e`).
