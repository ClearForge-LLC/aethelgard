# AET-WO-4406 FEEDBACK — `check pinout` refuses firmware that can override its compiled defaults at boot, and reads the GPS cap from the ELF

**Builder:** Claude (standard tier), worktree `~/aethelgard-4406`, branch `wo/aet-4406-defaults-guard` from `main` `472d2d84` (`main` `14bb1e5b` merged in).
**Cadence:** build with one measured stop (§1.1). **Neither stop fired.** The work order was built from the docs-branch copy (`~/.cache/aet-4406-wo.md`).

Labels:
- **[read]**: at source, this repository or ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`;
- **[measured]**: run on Furnace. ArduPilot builds ran in that image under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`;
- **[inferred]**: neither.

## 1. §1.1: the signature table, with its predictions

Predictions committed first (`2e254213`), then results (`f9fb4708`, `spike/4406/README.md`; decoder `spike/4406/measure.py`, a self-contained ELF32 reader). The boards: the 20 `4405` decoded, plus `sentinel-02-noimu` (CI builds it; built here from `main` `472d2d84` as `ardupilot-image.yml` does). The adversaries: `4405`'s `advromfs` and `advapj` (A1) and `advgps1` (A4). ELF hashes: `spike/4406/out/SHA256SUMS`.

| board | `load_defaults_file` | `load_embedded_param_defaults` | `param_defaults_data` length | ROMFS lists `defaults.parm` | predicted | measured |
|---|---|---|---|---|---|---|
| each of the 21 committed and CI-built boards¹ | absent | absent | symbol absent | no (`hwdef.dat` only) | none | **none** |
| `advgps1` | absent | absent | symbol absent | no | none | none |
| `advromfs` (A1 route 1) | **defined** (264 B) | absent | symbol absent | **yes** (40/57 B) | 2 | **2** |
| `advapj` (A1 route 2) | absent | **defined** (16 B) | **67** (`max_length` 1024) | no | 2 | **2** |

¹ `adv-numforms`, `adv-opt-inv`, `adv-opt-swap`, `adv-proto-del1`, `adv-proto-del3`, `adv-proto-swap`, `adv-realaf`, `eight`, `eight-opt6`, `eight-opt7`, `fc-base`, `fc-mav3`, `lab-lower`, `seed-af`, `seed-pin`, `seed-pkg`, `step-g`, `sentinel-01`, `sentinel-02`, `serial-dshot-9600`, `sentinel-02-noimu`.

- **The GPS cap** [measured]. `AP_GPS::var_info` is one read-only `STT_OBJECT` on every ELF: whole 16-byte rows, then `AP_GROUPEND`. Its receiver rows are `1_` (idx 32) and `2_` (idx 33), each flags 1 (`AP_PARAM_FLAG_NESTED_OFFSET`) and type 6 (`AP_PARAM_GROUP`).
  - That is **a cap of 2 on all 21 boards** and on both A1 builds, and **1 on `advgps1`**.
  - Every other row has flags 0 and type 1 or 2. Between boards, only the terminator's name pointer differs.
  - **The second §7 stop does not fire.**
- **The serial row offsets** [measured]. Every `SERIALn` row on all 24 ELFs is at `24n` (`BAUD`), `24n + 4` (`OPTIONS`) or `24n + 8` (`PROTOCOL`), so §1.4 is built.
- **Every prediction (Q1–Q7) holds.**

## 2. What was built

- **`elf::defaults_signatures`** lists every signature found, one line each:
  - a symbol `AP_Param::load_defaults_file` or `AP_Param::load_embedded_param_defaults`;
  - `AP_Param::param_defaults_data`'s length not 0;
  - `AP_ROMFS::files` listing `defaults.parm`;
  - **Lua, ruled on the PR (D-c), two signatures named separately:**
    - (e) ROMFS listing any `scripts/*.lua`, ArduPilot's own test (`boards.py:669-672`, `lua_scripts.cpp:548-550`);
    - (f) `AP_Scripting::var_info`'s `ENABLE` (`SCR_ENABLE`) compiling to a default other than 0, which runs a script from the SD card as well. The `ENABLE` row must be one `AP_Int8` flagged `AP_PARAM_FLAG_ENABLE` alone, or the table is refused (`MalformedScriptingVarInfo`). With AP_Scripting compiled out (no table, as on `advnoromfs`) there is no signature.

  `reload_defaults_file` is not a signature. Each read is refused by name when malformed:
  - `param_defaults_data` without `"PARMDEF"` and its magic, with `max_length` not its room, or with a length past `max_length` (`MalformedParamDefaults`); shorter than its header (`SymbolSize`);
  - a ROMFS that is not whole 20-byte entries or is zero-size (`SymbolSize`), or a name with no NUL within 128 bytes (`RomfsNameTooLong`).

  An absent symbol is not read: a board without ROMFS has no `AP_ROMFS::files` (measured, `advnoromfs`).
- **`elf::read` refuses an ELF with any signature** (`ParamDefaultsOverride`, every signature in the message). `check pinout` reads every ELF through it first, with or without `--hwdef-h` (D-b). No CLI change.
- **`elf::gps_receivers`** reads `AP_GPS::var_info` with the shared row reader. It counts the receiver rows, which must be `1_`, `2_`, … in order with `AP_PARAM_FLAG_NESTED_OFFSET`; anything else is `MalformedGpsVarInfo`. It returns `None` when the symbol is absent.
  - `GpsPortUnbound` counts against that number, and its message says so.
  - With no table, each port the topology declares a GPS is `GpsNotCompiled`, and the count is not made.
  - At this tag that case is unreachable: `AP_GPS_ENABLED 0` does not build Copter (`spike/4406/out/advnogps-build-errors.txt`).
- **`serial_params` holds each port row's offset** to `24n + {0, 4, 8}` (`VarInfoRowOffset`); the pass-through rows are not judged. The reader now shares `group_rows` (the `AP_GROUPEND` walk) and `flash_cstr` (a flashed string with a bound) with the GPS and ROMFS reads.
- **Tests (tier 1):**
  - `tests/defaults.rs`: the board shape every measured board has passes (`SCR_ENABLE` 0, `hwdef.dat` in ROMFS); `advlua` is refused naming both Lua signatures, and each alone is refused; a `scripts/` file that is not `.lua` is not a signature; a malformed scripting table is refused by name; both A1 builds are refused with exactly their two signatures, `advapj`'s 1044 bytes cut from its ELF (`tests/fixtures/advapj.param_defaults_data.bin`); each signature alone is refused; each malformed read is refused by name.
  - In `tests/identity.rs`: the cap from 0, 1, 2 and 3 receivers; `GpsNotCompiled`, and a stray compiled GPS row with no table and no declared GPS port, which gives no finding; malformed GPS tables; `4405` A5's offset swap and three other offsets.
  - The builder's `var_row` puts port rows at their measured offsets, and the identity helpers give every test ELF `fc-base`'s measured `AP_GPS::var_info`.
- **README** (`crates/aethelgard-pincheck/README.md`): the GPS rule, the offset rule and the defaults paragraph, Lua included, now say what is built (D7).

## 3. §2 acceptance

- **The signature table is in §1,** one row per board, with the predictions beside it.
- **The guard refuses the A1 build by name, and every committed and CI-built board passes** [measured, `spike/4406/out/judge.txt`: the `main` binary (`14bb1e5b`) and the branch's on all 24 ELFs and `advlua`, the 4404 boards' topologies re-solved at spec 20].
  - `advromfs` and `advapj` exit 3 with `ParamDefaultsOverride`, each naming its two signatures and no other, with `--hwdef-h` and without.
  - `advlua` exits 3 with `ParamDefaultsOverride`, naming both Lua signatures (the D-c ruling's proof). `main` passes it.
  - The 21 boards' verdicts do not move. `eight`, `eight-opt6` and `eight-opt7` keep every finding (kind, pin, decoded); only the cap's expected text changed.
  - `container/ardupilot-pinout.sh` exits 0 with no `::error` on the CI set with either binary.
  - `4405`'s `swap.elf` (A5), which passed `main`, is refused (`VarInfoRowOffset`).
- **The cap is read:** `the_gps_cap_is_read_from_ap_gps_var_info` moves fc-base's verdict at one receiver, and the eight-port board's at three. On real ELFs, `advgps1` gives `GpsPortUnbound SERIAL3`, where `main` passes it.
- **Gates and CI:** §6. **`TIER2_MIN` is unchanged:** this work order adds no tier-2 test.

## 4. Adversarial pass (targeted)

| # | probe | verdict |
|---|---|---|
| A1 | **Lua from ROMFS at boot** | **A path with no signature, ruled and built (D-c, D1)** [measured]. `advlua` (`fc-base` + `define SCRIPTING_ENABLE_DEFAULT 1` + `ROMFS scripts/adv.lua …`, a script setting `SERIAL3_PROTOCOL` by `param:set_default` and `param:set`) builds; ROMFS lists `scripts/adv.lua`; `AP_Scripting::var_info`'s `ENABLE` defaults to 1. Before the ruling both binaries passed it (`spike/4406/out/judge-advlua.txt`); the branch now refuses it, naming both (`judge.txt`). AP_Scripting is linked on every board, with a Lua binding to `set_default_by_name` [measured]; scripts load from `@ROMFS/scripts` when any `scripts/*.lua` is in ROMFS (`boards.py:669-672`, `lua_scripts.cpp:548-550`) and `SCR_ENABLE` is set (`AP_Scripting.cpp:55-72`, `:220`) [read]. **D1.** |
| A2 | ChibiOS persistent parameters | **A path, unreachable on every board.** `Util::apply_persistent_params` sets defaults by name at boot from the bootloader sector (`Util.cpp:581-640`, `HAL_ChibiOS_Class.cpp:285-286`) [read]. It compiles only when `FLASH_RESERVE_START_KB` is not 0 (`chibios_hwdef.py:1030-1032`), and is absent from all 25 ELFs [measured]. Proposed signature: the symbol `_ZNK7ChibiOS4Util23apply_persistent_paramsEv` [inferred mangling]. **D4.** |
| A3 | IOMCU at boot | **A path, unreachable.** `AP_Vehicle.cpp:369` sets the IOMCU port's protocol and baud by `set_and_default` at boot [read]. `AP_IOMCU` is absent from all 25 ELFs, and ArduPilot requires `io_firmware.bin` in ROMFS (`chibios_hwdef.py:1729`) [measured, read]. Proposed signature: ROMFS lists `io_firmware.bin`, or `AP_SerialManager::set_protocol_and_baud` is linked. **D5.** |
| A4 | Defaults set by compiled code | **Hold for the judged rows.** Copter's `set_default` calls (`system.cpp:496-512`, `Parameters.cpp:1364-1365`, `radio.cpp:46`) touch PIDs, the notch and heli tables. `AP_SerialManager::init`'s `set_and_default` bauds (`:470-570`) are for FrSky, AlexMos, Gimbal, ESC telemetry, EFI and MSP, none a role the judge accepts [read]. `GPS1_TYPE`'s default is code (`AP_GPS.cpp:328`, `HAL_GPS1_TYPE_DEFAULT`), which bears on `4405` D2 only. |
| A5 | A `param_defaults_data` the reader misparses | **Hold.** The layout is checked against the real bytes (`max_length` 0x0400 at 16, length 0x43 at 18, `advapj`). The reader passes only at length 0, the value ArduPilot itself tests before loading (`AP_Param.cpp:1619`). A wrong magic, `max_length` or overrun is refused, and every guard's mutant is killed. `apj_tool` finds the struct by scanning for its magic (`apj_tool.py:108-131`), the reader by symbol: a decoy is the trusted-symbol-table residual (`4405` A6). **A gap:** `load_embedded_param_defaults` is a 16-byte out-of-line function; were it inlined, a later `apj_tool` patch of the `.apj` would carry defaults the ELF does not show, while `param_defaults_data` itself still exists. **D6.** |
| A6 | A GPS-protocol port bound by something other than AP_GPS | **Hold.** The one other consumer is DroneCAN's serial tunnel (`AP_DroneCAN_serial.cpp:96-97`) [read], linked on no board [measured]. |
| A7 | Compiler clones (`.isra`, `.constprop`) of a signature function | **None** on any ELF [measured]. |
| A8 | `PARAM_DEFAULTS` (216 B, linked on every board) | **Not defaults:** `AP_OSD_ParamScreen`'s list of parameters to show [measured, neighbouring symbols]. |
| A9 | A mission | **No path:** neither `AP_Mission` nor `mode_auto` sets a parameter [read]. |
| A10 | Values saved in parameter storage, GCS sets, MAVLink FTP | **Residual, unchanged:** they are not in the image. |

## 5. Decisions for the architect

- **D1 (D-c), RULED 2026-11-20 14:42Z and built: Lua (A1).** Two signatures, named separately: (e) ROMFS lists any `scripts/*.lua`; (f) `SCR_ENABLE`'s compiled default is not 0. Proved: `advlua` refused naming both, a mutant each (W21, W22, plus W23–W25 on the reads), every committed and CI-built board passes.
- **D2 (D-a), ruled accepted. `ParamDefaultsOverride` is a refusal**, the work order's "refuses by name": error JSON, the unreadable exit (3), no verdict. One site wraps the list, so turning it into a finding is one edit.
- **D3 (D-b), ruled accepted. The guard runs in `elf::read`,** so `check pinout --elf` alone refuses too. In `identity::judge` it would have run only with `--hwdef-h`.
- **D4. Persistent parameters (A2), ruled a FEEDBACK row:** proposed signature the symbol `_ZNK7ChibiOS4Util23apply_persistent_paramsEv`, before any work order that reserves bootloader flash.
- **D5. IOMCU (A3), ruled a FEEDBACK row:** proposed signature ROMFS listing `io_firmware.bin`, or `AP_SerialManager::set_protocol_and_baud` linked, before any IOMCU board.
- **D6. Embedded defaults by capability (A5):** refusing on `param_defaults_data` being present at all, not only on its length, would cover an inlined loader. No measured build needs it today.
- **D7. Fence edge:** the work order's fence does not list the crate's own `README.md`. Its GPS and defaults paragraphs became false with this build, so they were corrected (`4405` edited the same paragraphs). Revert them if you rule otherwise.
- **D8. A residual of the literal reading (§1.3):** with no `AP_GPS::var_info` and no declared GPS port, a compiled GPS-protocol row is not named. The work order says so; it is unreachable at this tag.

## 6. Reduction pass, mutants and gates

- **Reduction pass (diff): −14 lines net · 1 duplicate removed · 1 dead path removed · 3 single-use constants inlined · 3 protected kept · 2 proposals rejected.** A fresh subagent asked the four questions of the diff.
  - **Duplicate:** `defaults_signatures`' doc listed each signature with its cite, the same text as the lines it pushes. It now points at them.
  - **Dead path:** a function signature counted only a defined symbol; a linked image holds no undefined one, and no test reached the filter. Any symbol of the name now counts, which is stricter.
  - **Inlined:** `BAUD_AT`, `OPTIONS_AT` and `PROTOCOL_AT`, one use each. `UART_STATE_LEN` keeps the doc that gives all three.
  - **Shared already, in the build:** one `AP_GROUPEND` walk (`group_rows`) for both tables, one flashed-string reader (`flash_cstr`) for names and ROMFS paths, and one test-builder `group_info` for both tables.
  - **Protected, kept:** the three `belt-and-suspenders:` sites (`elf.rs` `map_baudrate`, two in `identity.rs`), none in this diff.
  - **Rejected:** shortening `VarInfoRowOffset`'s message and the README's offset sentence (both user-facing); "port(s)" in place of the singular form (message churn, no line saved).
  - After the cuts the suite stays green and W1, W5 and W10 are still killed.
  - **The ruled Lua increment (D-c):** nothing to remove. It reuses the ROMFS walk and `group_rows`, and adds one constant and one flag.
- **Mutants** (`spike/4406/mutants.py`; one per guard this work order adds, each run against every pincheck test binary; the file is restored from a copy, and a mutant that does not compile is never counted): **25 of 25 killed, each by a named test** (`spike/4406/out/mutants.txt`).
  - The work order's: dropping each signature in turn (W1–W4); `reload_defaults_file` taken as a signature, which refuses the measured-board shape (W5); the cap held at 2 (W6, killed by the one- and three-receiver tables).
  - The rest: the guard not called by `read`; no table read as cap 0; `GpsNotCompiled` dropped; the offset; a receiver's flags; its name; group rows not told from scalars; an absent table read; the magic; `max_length`; the length bound; the ROMFS name bound; a zero-size ROMFS; an absent ROMFS read.
  - Ruled D-c: dropping the ROMFS-script signature (W21); dropping `SCR_ENABLE` (W22); any `scripts/` file taken as Lua (W23); `ENABLE`'s type and flags not held (W24); a second `ENABLE` row ignored (W25).
  - W1's first literal did not compile and was rerun.
- **Gates** at `3d45cd37` (all code, `main` `824c3ff3` merged): `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0; `cargo test --workspace` **2445 passed, 0 failed** (`~/.cache/aet-4406/gates-3d45cd37.log`). An earlier run at `94507715`, before the ruling: 2443/0.

## 7. Status

- Built to the work order and the 14:42Z rulings (D-a, D-b accepted; D-c built; persistent parameters and IOMCU as FEEDBACK rows D4, D5).
- No stop fired. One PR, left unmerged; the architect merges.
- CI on the PR, the ardupilot-image `pinout` job included, decides READY.
