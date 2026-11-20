# AET-WO-4405 §1.1: `AP_SerialManager::var_info` decoded on every board

Labels: **[read]** at source, ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`; **[measured]** run on Furnace; **[inferred]** neither.

## 1. Predictions, committed before any decode

Written before `varinfo.py` was run on any ELF. The ten `4404` §2.3/§4 boards and the adversary's two (`adv-numforms`, `adv-realaf`) were built by `4404` on 2026-11-19 in this same image, and `4404` read their PROTOCOL and OPTIONS rows (§2.3, §4). Their BAUD rows, the row list, the layout and the passthrough rows have not been read. The seven CI boards (`step-g`, `seed-pin`, `seed-af`, `seed-pkg`, and the generated `sentinel-01`, `sentinel-02`, `serial-dshot-9600`) are rebuilt from this branch's base (`main` `5eecc929`), because their files changed after `4404` (spec 18, `4202`).

### 1.1 The layout (P1–P6), the same on every board [read]

| # | prediction | basis |
|---|---|---|
| P1 | `_ZN16AP_SerialManager8var_infoE` is one `STT_OBJECT`, in allocated, non-writable `PROGBITS` (`4404` saw it as `T`, `.text`), its file offset the same by section and by `PT_LOAD` | `const` array; `4404` §2.4 |
| P2 | Its size is `16 × (rows + 1)`: every entry 16 bytes, the last `AP_GROUPEND` | `AP_Param.h:206-218`, `:170` |
| P3 | `AP_GROUPEND` is `{name → "", offset 0, union 0, flags 0, idx 0xFF, type 0}`, and no earlier entry has type 0. ArduPilot stops at the first `type == AP_PARAM_NONE` | `AP_Param.h:170`; `AP_Param.cpp:1498-1500` |
| P4 | Every name pointer resolves into allocated, non-writable flash, to a NUL-terminated string | `const char *` literals |
| P5 | Every row's flags are 0 (`AP_GROUPINFO`), so `def_value` is a float, never a `DEFAULT_POINTER` offset | `AP_Param.h:137-140`, `AP_Param.cpp:2619-2622` |
| P6 | Types: `n_PROTOCOL` 1 (`AP_Int8`), `n_BAUD` 3 (`AP_Int32`), `n_OPTIONS` 3 (`AP_Int32`, not 16: `UARTState::options` is `AP_Int32`, `AP_SerialManager.h:157-159`); `_PASS1`, `_PASS2`, `_PASSTIMO` 1 | `AP_SerialManager.h:157-159`, `:223-225` |

Row order follows the source: `0_BAUD`, `0_PROTOCOL`, then `n_PROTOCOL`, `n_BAUD` for n = 1 to 6, then `n_OPTIONS` for 1 to 6, `_PASS1`, `_PASS2`, `_PASSTIMO`, then `n_PROTOCOL`, `n_BAUD`, `n_OPTIONS` for 7, 8, 9; each n only where `HAL_HAVE_SERIALn_PARAMS` is 1 (`AP_SerialManager.cpp:164-397`). There is no `0_OPTIONS`.

### 1.2 The values (P7), by `predict.py`, from each board's `hwdef.h` (or, for the seven CI boards, its `hwdef.dat`, whose defines ArduPilot copies verbatim)

`predict.py` is the rule today's `hwdef.h` judge applies (`identity.rs`, `serial_ports` and `serial_settings`), written again in Python. **So P7 also predicts no disagreement with today's judge on any board.** The one place the two readings were already known to differ in the define, `eight-opt7` (`DEFAULT_SERIAL7_OPTIONS 8`), both today's judge and `predict.py` read as 0 (`:357`). `->` is `map_baudrate` (`:737-770`).

```
adv-opt-inv 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=3 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
adv-opt-swap 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=8 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
adv-proto-del1 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=2 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
adv-proto-del3 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
adv-proto-swap 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=23 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=5 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
eight-opt6 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0 4_PROTOCOL=5 4_BAUD=115200->115200 4_OPTIONS=0 5_PROTOCOL=5 5_BAUD=115200->115200 5_OPTIONS=0 6_PROTOCOL=5 6_BAUD=115200->115200 6_OPTIONS=8 7_PROTOCOL=2 7_BAUD=115200->115200 7_OPTIONS=0
eight-opt7 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0 4_PROTOCOL=5 4_BAUD=115200->115200 4_OPTIONS=0 5_PROTOCOL=5 5_BAUD=115200->115200 5_OPTIONS=0 6_PROTOCOL=5 6_BAUD=115200->115200 6_OPTIONS=0 7_PROTOCOL=2 7_BAUD=115200->115200 7_OPTIONS=0
fc-base 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
fc-mav3 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=2 3_BAUD=115200->115200 3_OPTIONS=0
lab-lower 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
eight 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0 4_PROTOCOL=5 4_BAUD=115200->115200 4_OPTIONS=0 5_PROTOCOL=5 5_BAUD=115200->115200 5_OPTIONS=0 6_PROTOCOL=5 6_BAUD=115200->115200 6_OPTIONS=0 7_PROTOCOL=2 7_BAUD=115200->115200 7_OPTIONS=0
adv-numforms 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
adv-realaf 0_PROTOCOL=2 0_BAUD=57->57600 1_PROTOCOL=5 1_BAUD=115200->115200 1_OPTIONS=0 2_PROTOCOL=23 2_BAUD=57->57600 2_OPTIONS=0 3_PROTOCOL=5 3_BAUD=115200->115200 3_OPTIONS=0
seed-af 0_PROTOCOL=2 0_BAUD=115->115200 1_PROTOCOL=2 1_BAUD=57->57600 1_OPTIONS=0
seed-pin 0_PROTOCOL=2 0_BAUD=115->115200 1_PROTOCOL=2 1_BAUD=57->57600 1_OPTIONS=0
seed-pkg 0_PROTOCOL=2 0_BAUD=115->115200 1_PROTOCOL=2 1_BAUD=57->57600 1_OPTIONS=0
sentinel-01 0_PROTOCOL=2 0_BAUD=115->115200
sentinel-02 0_PROTOCOL=2 0_BAUD=115->115200
serial-dshot-9600 0_PROTOCOL=2 0_BAUD=9->9600
step-g 0_PROTOCOL=2 0_BAUD=115->115200 1_PROTOCOL=2 1_BAUD=57->57600 1_OPTIONS=0
```

Beside these, on every board: `_PASS1=0`, `_PASS2=-1`, `_PASSTIMO=15` (`:324`, `:331`, `:339`).

### 1.3 What the judge must therefore handle [inferred]

- **P8. SERIAL0's boot open is not in `var_info`.** `init_console` opens SERIAL0 at `DEFAULT_SERIAL0_BAUD` itself (`AP_SerialManager.cpp:413-421`, the call at `:419`), and the GCS reopens it at `map_baudrate(0_BAUD)`. `0_BAUD` is the define divided by 1000 in C integer arithmetic (`:172`), so `var_info` cannot tell 115200 from 115999. Today's judge reads both from `hwdef.h` (`4400b2` adversarial A7).
- **P9. The eight-port board has five GPS ports** (SERIAL1, 3, 4, 5, 6). ArduPilot binds the first two (`AP_GPS_config.h:16`, `AP_GPS.cpp:345-349`, `find_protocol_instance` at `AP_SerialManager.cpp:600-612`). A GPS2 (6) port counts as GPS there (`protocol_match`, `:786-790`).
- **P10. A default in `var_info` is overridden by a defaults file** (`AP_Param.cpp:2612-2622`: `param_overrides` first). A board directory holding `defaults.parm` puts it in ROMFS (`chibios_hwdef.py:2726-2750`, `:3083`). No CI board has one: the build installs only `hwdef.dat` (`container/ardupilot-build.sh`).

## 2. Results [measured]

Decoded with `varinfo.py`, a self-contained ELF32 reader (no `nm`, no `readelf`), on 2026-11-20. The seven CI boards were built from `main` `5eecc929`'s files with `container/ardupilot-build.sh boards …` in the pinned image under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`, 20–24 s each; the three generated files were first regenerated by `main`'s binary and `cmp`-equal to the committed fixtures. The thirteen `4404` ELFs are the ones `4404` built in the same image on 2026-11-19. Raw output: `out/decode-4404.txt`, `out/decode-ci.txt`; predictions as run: `out/predict.txt`.

### 2.1 The layout: the same on all 20 boards. **The §7 stop does not fire.**

| # | prediction | measured, on every one of the 20 ELFs |
|---|---|---|
| P1 | one `STT_OBJECT`, allocated, not writable | `STT_OBJECT` (1), in `.text`, `sh_flags` 6 (`ALLOC`, `EXECINSTR`), not writable |
| P2 | size `16 × (rows + 1)` | 96 B (5 rows), 144 B (8), 240 B (14), 432 B (26); `AP_GROUPEND` the last 16 bytes in every one |
| P3 | `AP_GROUPEND` `{→"", 0, 0, 0, 0xFF, 0}`, no earlier type 0 | `<ptr> 00000000 00000000 0000 ff 00`, the pointer to an empty string in `.text`; no earlier type 0 |
| P4 | names in read-only flash, NUL-terminated | every name in `.text`, read-only |
| P5 | every row's flags 0 | 0 on every row |
| P6 | types `_PROTOCOL` 1, `_BAUD` 3, `_OPTIONS` 3, `_PASS*` 1 | exactly so |

Only the terminator's name pointer differs between boards, as an address (`0x08121c30`, `0x08122284`, …): the layout is one, the addresses are the link's. Row order is the source's (§1.1) on every board.

### 2.2 The values: all 20 boards equal P7, row for row

A mechanical compare of `out/predict.txt` against the decoded rows (`_PASS*` aside) gives **20 of 20 equal**. `_PASS1=0`, `_PASS2=-1`, `_PASSTIMO=15` on all 20. The decode table, one line per board (`n_ROW=value`, `->` the baud after `map_baudrate`):

| board | SERIAL0 | SERIAL1 | SERIAL2 | SERIAL3 | SERIAL4–7 |
|---|---|---|---|---|---|
| fc-base, lab-lower, adv-numforms, adv-realaf | MAV2, 57→57600 | GPS, 115200, opt 0 | RCIN, 57→57600, opt 0 | GPS, 115200, opt 0 | — |
| adv-proto-swap | MAV2, 57→57600 | **RCIN**, 115200, 0 | **GPS**, 57→57600, 0 | GPS, 115200, 0 | — |
| adv-opt-swap | MAV2, 57→57600 | GPS, 115200, **opt 8** | RCIN, 57→57600, 0 | GPS, 115200, 0 | — |
| adv-opt-inv | MAV2, 57→57600 | GPS, 115200, **opt 3** | RCIN, 57→57600, 0 | GPS, 115200, 0 | — |
| adv-proto-del1 | MAV2, 57→57600 | **MAV2** (`:48`), 115200, 0 | RCIN, 57→57600, 0 | GPS, 115200, 0 | — |
| fc-mav3 | MAV2, 57→57600 | GPS, 115200, 0 | RCIN, 57→57600, 0 | MAV2, 115200, 0 | — |
| adv-proto-del3 | MAV2, 57→57600 | GPS, 115200, 0 | RCIN, 57→57600, 0 | **GPS** (`:74`), 115200, 0 | — |
| eight | MAV2, 57→57600 | GPS, 115200, 0 | RCIN, 57→57600, 0 | GPS, 115200, 0 | S4–S6 GPS, S7 MAV2; 115200; opt 0 |
| eight-opt6 | as eight | | | | **6_OPTIONS 8** |
| eight-opt7 | as eight | | | | **7_OPTIONS 0** (the define says 8, `:357`) |
| step-g, seed-pin, seed-af, seed-pkg | MAV2, 115→115200 | MAV2 (default), 57→57600, 0 (USART3, not in the topology) | — | — | — |
| sentinel-01, sentinel-02 | MAV2, 115→115200 | — | — | — | — |
| serial-dshot-9600 | MAV2, **9→9600** | — | — | — | — |

### 2.3 Against today's judge: no disagreement

`main`'s binary (`5eecc929`), run on each board (`judge.sh`, `out/judge-main.txt`):
- **The seven CI boards:** `container/ardupilot-pinout.sh`, unchanged, exits 0 with no `::error`.
- **The thirteen `4404` boards,** each on its topology re-solved under specification 18 from `4404`'s own spec files (only `schema_version` changed; every regenerated `hwdef.dat` has `4404`'s lines exactly): the verdicts are `4404` §4's, finding for finding. `fc-base`, `fc-mav3`, `lab-lower`, `eight`, `eight-opt7` and `adv-numforms` pass; each mutation gives exactly its `4404` finding; `adv-realaf` fails on its forward `NotOutput PA11` only.

Every value today's judge read from `hwdef.h` is the value `var_info` holds, on all 20 boards. So no board's verdict should move on the protocol and options, and no §7 stop fires. Two readings move by design, in the build:
- **A port n ≥ 1 with no `DEFAULT_SERIALn_BAUD`** is a finding today ("not stated"). The ELF states it (`57` → 57600 on SERIAL1, 2 and 5–9; `230` → 230400 on SERIAL3 and 4), so it is judged by that value. No CI board has such a port in its topology. fc-base's SERIAL2 is RC input, whose baud is not judged.
- **The GPS cap fires on `eight`,** which passes today: SERIAL4, 5 and 6 are its third to fifth GPS ports (P9).
