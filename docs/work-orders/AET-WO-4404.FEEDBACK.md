# AET-WO-4404 FEEDBACK — serial protocol and options judged, the AF-prefix refusal, the fixture headers

**Builder:** Claude (strong tier), worktree `~/aethelgard-4404`, branch `wo/aet-4404-hwdef-serial` from `main` `cd43b207`.
**Cadence:** build, with one measured stop (§1.1). The work order is read from the docs branch (`~/.cache/aet-4404-wo.md`) and is not committed here.

Labels:
- **[read]**: at source, this repository or ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`;
- **[measured]**: run on Furnace in that image under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`;
- **[inferred]**: neither.

## 1. Predictions (before any judged build)

Written before any build result was read, and never edited afterwards. **One exception, stated plainly:** the configure-only label probes (§1.1 (c), 14 prefixes and the lower-case label) ran a few minutes before this table was committed, and their results had been read. The rows for the label rule below are therefore labelled **post hoc**. They match Grounds A9 (`4400b`), which predicted them first.

| # | question | prediction | basis |
|---|---|---|---|
| P1 | fc-base, the compiled protocol of SERIAL0..3 | SERIAL0 MAVLink2 (the literal at `:180`); SERIAL1 GPS (5), SERIAL2 RCIN (23), SERIAL3 GPS (5), each from its define | [read] |
| P2 | fc-base, the compiled options | SERIAL1 0, SERIAL2 0 (its define is not emitted, so the `#ifndef` default is 0; RC input then sets RXINV itself), SERIAL3 0; SERIAL0 has no options parameter | [read] |
| P3 | fc-base builds; `main`'s `check pinout` exits | builds; exit 0 | [inferred], from `4400b2` |
| P4 | each mutation builds unmodified ArduPilot, and `hwdef.h` carries its define verbatim (or lacks the deleted one) | yes for all five | [read], `chibios_hwdef.py:1004-1010` |
| P5 | `main`'s `check pinout` on each of `adv-proto-swap`, `adv-opt-swap` (S1 OPTIONS 8), `adv-opt-inv` (S1 OPTIONS 3), `adv-proto-del1` (GPS port at SERIAL1 with no define: runs MAVLink2), `adv-proto-del3` (MAVLink port at SERIAL3 with no define: runs GPS) | exit 0 on every one (the gap) | [inferred]: `main` reads no `_PROTOCOL` or `_OPTIONS` |
| P6 | this branch's `check pinout` on the same | fc-base 0; each mutation exit 1 with `identity` findings: `adv-proto-swap` two `WrongSerialProtocol` (SERIAL1, SERIAL2); `adv-opt-swap` and `adv-opt-inv` one `WrongSerialOptions` (SERIAL1); `adv-proto-del1` and `adv-proto-del3` one `WrongSerialProtocol` each (SERIAL1, SERIAL3) | [inferred] |
| P7 | any committed `hwdef.dat` golden or topology moves | none: no committed board has a SERIAL7 or 8, and none has a SERIAL1–9 at all (§1.1 (b)) | [measured] (b) |
| P8 | **post hoc:** the 14 AF prefixes as a chip-select label | each refused at configure (`Unknown pin function`) | [measured] before this table |
| P9 | **post hoc:** a lower-case `spi_cs` | configures, so it is not refused by the generator | [measured] configure before this table; the full build is predicted to pass |
