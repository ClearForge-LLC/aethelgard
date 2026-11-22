# AET-WO-1218 — results, against `PREDICTIONS.md` (9156a5e8)

Kit: `measure.sh` (the baseline sweep by `spike/1211d/measure/sweep.sh`, then every probe of `probes.py`: validate, solve, emit twice, the two emits compared byte for byte, the board's references by `refs.py`). The null sweep is `spike/1211a/measure/run.sh` and `spike/1211e/compare.py`. Mutants: `mutants.py`. Outputs: `out/`.

## P1 — the baseline
- **P1.1 held.** 34 committed specifications. 2 carry pin devices (D1 and D2, both `board.drawn: false`), 4 carry test points, none carry both. Exits: `out/baseline-exits.tsv`.
- **P1.2 held.** The sweep run twice with main's binary was identical: 280 files.

## P2 — the scratch boards, today (`out/probes-base.tsv`)
- **P2.1 held.** D2 is drawn here, with its PWM limit declared. D2 plus `TP1` on `GND` is refused at validate, solve and emit with `TestPointReferenceTaken`: "test point `TP1` takes a reference that `TP1` already holds".
- **P2.2 held.** D1 plus `TP1`, and a test point on `DRV_NFAULT`, on `BOOT0`, or two test points, all give the same refusal.
- **P2.3 held, with a correction.** `U2`, `u02`, `R2` and `r2` are refused, but by `emitter_prefix` (U, R, C or SW and a number), never by the positional slice. So they do not exercise (1).
  - Added: collisions outside the emitter's numbering. D2's device is renamed `DRV1`, against a test point `DRV1` or `drv01`. D1's sense resistor is renamed `RSENSE`, against `RSENSE` or `rsense`.
  - Today `drv01` is refused naming **itself** ("`drv01` already holds"). The right kind, for the wrong reason: the test point collides with itself.

## P3 — after (1) (`out/probes-after.tsv`, `out/refs-after.tsv`)
- **P3.1 held.** `d1-tp-gnd`, `d2-tp-gnd`, `d2-tp-nfault`, `d2-tp-boot0` and `d2-tp-two` each validate, solve and emit.
- **P3.2 held.** Each is emitted twice, byte-identical.
- **P3.3 held.** Each board's references are its bare board's plus `TP1` (and `TP2`). On `BOOT0` the bench's pull-down `R1` is drawn after every other part, and no other reference moves.
- **P3.4 held.** Every collision is still `TestPointReferenceTaken` at step 15a. `drv01` now names its real holder, `DRV1`.
- **P3.5 held.** The null sweep before the bump: 34 specifications, 280 files identical.

## P4 — (2), (3), (4)
- **P4.1 held in substance, ordered differently.** A topology with `mcu` renamed (`STM32H743ZITx`, or `stm32h753zitx`) and `FLASH_CS` on `PC2` passes `SolvedTopology::validate` today. Its pads are then no row's.
  - After: `check pinout` refuses it with `PartMismatch`, exit 2.
  - The check is in `pincheck::judge`, so it runs after the CLI has read the ELF. A malformed ELF is therefore reported first. The prediction said "before the ELF is read".
- **P4.2 held; the band is unreachable through `validate`.** The lowest held capacitor is rated 10 V, and VDDA is at most 3.6 V.
  - The test therefore adds a test row through `power::check_against`. It is refused at 3135 mV (VDDA's minimum) and at 3464 mV, and admitted at 3465 mV (the maximum).
  - Before the fix, 3135 and 3464 were admitted.
- **P4.3 held.** On the H753, `PC2` and `PC3` are the pads. They carry `SPI2_MISO` and `SPI2_MOSI`, and no UART signal.
  - A bus refusal now ends "; `PC2` withheld as an analog pad (the MCU row's `io.analog_pads`)".
  - The serial path is unreachable on the real table, so it is tested on the IR's synthetic table, relabelled as the H753.
- **P4.4 held, and wider than the row named.** On D1, R2 is under RAIN at 1070, 1000 and 900 Ω. A reservoir part `NOPE`, or a resistor row, validated today; after, it is `PowerPartUnknown` at the reservoir's path.
  - On the power tree's sense, `NOPE` was already refused by the tree's own part lookup. A resistor row validated, and `emit` refused it (`PowerNotDrawable`). After, it is `PowerPartUnknown`.
  - The held capacitor stays admitted (`out/r5-probes.tsv`).

## §5 — the adversarial pass, targeted (probes `adv-*` in `probes.py`)
- **A reference read differently by two readers.**
  - The bench's step 15a now reads `names::references_but_test_points`.
  - `names::check` (step 20), `mcu_reference` and `switch_reference` read `names::references` as a set, so its order moves none of them.
  - eda's claim chain (`schematic.rs`) is a hand copy of `names::references`' order. It is unchanged and claims one set.
  - The emitted boards agree with the bench:
    - D2 with the reset switch numbers `SW1` with and without a test point on `NRST`.
    - A test point on the bridge output `MA_OUT1` or on `VMOTOR` emits beside `U2`.
    - Every board's references are the bare board's plus the test point's (`refs.py`).
  - The generator (`generate`, D1 with and without `TP1`) moves its KiCad files by the test point's footprint and symbol, and every other generated file by its sha line only: 0 other lines moved.
  - D2 does not generate, bare or not (`BridgeAcrossParts`, the generator's own refusal).
  - `check pinout` reads no reference.
- **A renamed MCU that still escapes a check.** Only two commands read a topology file: `check pinout` and `generate hwdef`.
  - `check pinout` refuses `PartMismatch`.
  - `generate hwdef` refuses `UnknownMcu` for `STM32H743ZITx` and for `stm32h753zitx`. With `FLASH_CS` on `PC2`, the topology's own `AnalogPadPin` fires on the H753.
  - The H753 is the power parts table's only MCU row, so no rename lands on another row's pads.
  - Every other topology the engine judges is solved in-process from a specification, where `solve` refuses a table/part mismatch (`PartMismatch`).

## Mutants (`mutants.py`, `out/mutants.json`)
11 of 11 killed: one per guard this work order adds or changes, each against the suites that hold it. The unmutated tree passes them all.

## Gates before the bump (3eece126)
`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass. `cargo test --workspace`: 160 binaries, 2784 passed, 0 failed.

The null sweep (`out/null-sweep-prebump.json`) ran main's binary against the branch's, each on the same specification bytes. Over 34 specifications, nothing moved: no exit, no `validate` or `solve` output, no emitted file, no generated file.
