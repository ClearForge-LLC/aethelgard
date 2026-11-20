# AET-WO-4406 §1.1: the defaults signatures, the GPS cap and the serial row offsets, measured on every board

Labels: **[read]** at source, ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`; **[measured]** run on Furnace; **[inferred]** neither.

The ELFs: the 20 that `4405` §2 decoded (thirteen built by `4404` on 2026-11-19, seven CI boards built by `4405` from `main` `5eecc929`), and `4405`'s adversary builds `advromfs` and `advapj` (its A1, the two routes) and `advgps1` (its A4, `GPS_MAX_RECEIVERS 1`), all built in the pinned image. Since `5eecc929` the three generated CI fixtures changed in one comment line only (the specification hash), so their code is unchanged. CI also builds `sentinel-02-noimu` (`sentinel-02` with its `IMU` line dropped), which `4405` did not decode; it is measured here as a 21st board, built from `main` `472d2d84`.

## 1. Predictions, committed before any decode

Written before `measure.py` was run on any ELF.

### 1.1 What ArduPilot links for each route [read]

- **The ROMFS route.** ChibiOS asks for `@ROMFS/defaults.parm` on every board (`AP_HAL_ChibiOS/Util.h:43-45`). `chibios_hwdef.py` puts a board directory's `defaults.parm` (or `--default-parameters`) into ROMFS (`:2727-2750`, `:3083`) and then writes `AP_PARAM_DEFAULTS_FILE_PARSING_ENABLED 1` (`:2509-2512`), which compiles `AP_Param::load_defaults_file` (`AP_Param.cpp:2315`, `:2406`); with no file it writes `0`.
- **The embedded route.** `AP_PARAM_MAX_EMBEDDED_PARAM` is 0 unless `FORCE_APJ_DEFAULT_PARAMETERS` or a `define` sets it (`AP_Param.h:53-63`). Above 0 it compiles `param_defaults_data` (`AP_Param.cpp:128-139`: `"PARMDEF"`, 8 magic bytes, `uint16_t max_length`, `uint16_t length`, then the text, `AP_Param.h:682-688`) and `load_embedded_param_defaults` (`:2602-2605`), called when `length` is not 0 (`:1618-1622`). waf's `apj_tool` writes the text and the length into the linked ELF (`chibios.py:141-155`, `:465-469`; `apj_tool.py:108-151`).
- **ROMFS on every board.** `hwdef.dat` itself goes into ROMFS unless `NO_ROMFS_HWDEF` is set (`chibios_hwdef.py:2452-2455`). `AP_ROMFS::files` is an array of `embedded_file` (`AP_ROMFS.h:30-36`: `filename` pointer, `compressed_size`, `decompressed_size`, `crc`, `contents` pointer; 20 bytes on ARM32).
- **`AP_GPS::var_info`** is one `AP_Param::GroupInfo` array (`AP_GPS.cpp:135-290`). The receivers are `AP_SUBGROUPINFO(params[0], "1_", 32, …)` and, only when `GPS_MAX_RECEIVERS > 1`, `(params[1], "2_", 33, …)` (`:280-286`): flags `AP_PARAM_FLAG_NESTED_OFFSET` (1), type `AP_PARAM_GROUP` (6) (`AP_Param.h:159`, `:192`).

### 1.2 Predictions

| # | prediction | boards |
|---|---|---|
| Q1 | `_ZN8AP_Param18load_defaults_fileEPKcb` is not defined | all 21 boards, `advapj`, `advgps1` |
| Q1a | it is defined (`STT_FUNC`, in `.text`) | `advromfs` |
| Q2 | `_ZN8AP_Param28load_embedded_param_defaultsEb` is not defined | all 21 boards, `advromfs`, `advgps1` |
| Q2a | it is defined | `advapj` |
| Q3 | `_ZN8AP_Param19param_defaults_dataE` is not in `.symtab` (so its length is not read) | all 21 boards, `advromfs`, `advgps1` |
| Q3a | it is one `STT_OBJECT` of `20 + 1024` bytes, read-only, `"PARMDEF\0"` and the magic, `max_length` 1024, `length` 67 (`4405` A1), the text holding `SERIAL3_PROTOCOL -1` | `advapj` |
| Q4 | `_ZN8AP_ROMFS5filesE` is one `STT_OBJECT` of one 20-byte entry, read-only, whose name is `hwdef.dat` | all 21 boards, `advapj`, `advgps1` |
| Q4a | two entries, `defaults.parm` then `hwdef.dat` (the `ap_romfs_embedded.h` order) | `advromfs` |
| Q5 | `_ZN6AP_GPS8var_infoE` is one `STT_OBJECT` of whole 16-byte rows ending in `AP_GROUPEND`; its subgroup rows named `<digit>_` are exactly `1_` (idx 32) and `2_` (idx 33), each flags 1, type 6: **a cap of 2** | all 21 boards, `advromfs`, `advapj` |
| Q5a | only `1_`: **a cap of 1** | `advgps1` |
| Q6 | every other row of `AP_GPS::var_info` has flags 0 and a scalar type (1 to 5) | all 24 |
| Q7 | every `SERIALn` row's `offset` is `24n` for `n_BAUD`, `24n + 4` for `n_OPTIONS`, `24n + 8` for `n_PROTOCOL` | all 24 |

So no committed or CI-built board shows a signature (Q1–Q4), the cap is 2 on every one (Q5), and the A1 builds each show at least one signature: `advromfs` two (Q1a, Q4a), `advapj` two (Q2a, Q3a). If Q1–Q4 fail on a board, §7 stop 1 fires; if Q5 or Q6 fails, §7 stop 2.

Q7 is from `4405` A5 (11 boards measured) and `UARTState` (`AP_SerialManager.h:143-162`: `baud`, `options`, `protocol` in that order) [read]; that a `UARTState` is 24 bytes and `state[]` sits at the object's offset 0 is [inferred] from A5's numbers, not read.
