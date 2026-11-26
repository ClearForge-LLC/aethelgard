# `check pinout` on every DShot board's ELF (AET-WO-4412, ruling 2026-11-26T17:35Z)

The ELFs were built locally in the pinned ArduPilot image by `container/ardupilot-build.sh boards`, from `generate hwdef`'s `hwdef.dat` and `defaults.parm`. Hashes are in `sha256.txt`. Each was judged with `--hwdef-h`, by this branch's binary (`new`) and by main's (`main`, 4406's guard as merged). The results are in `verdicts.jsonl`.

- **Which boards:** six committed boards declare DShot, and only these two generate. `hse-crystal-16` and `sentinel-02-shaped` are refused `NoConsole`, `hse-external-8` `OscillatorNotCited`, and `motors-dshot600` `NoSerialPort`, all before any hwdef is written.
- **Results:** `serial-dshot-9600` and `sentinel-03` are each admitted and judged by `new` (ok, decodes agree, identity judged, 0 findings). `main` refuses both with `ParamDefaultsOverride`.
- **The foreign Sentinel-03** (ELF 1cfc5e02, the armread12 ELF) is admitted and judged by `new` in the same way.
