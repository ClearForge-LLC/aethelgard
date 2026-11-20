# AET-WO-4405 FEEDBACK — serial protocol, options and baud read from the firmware; a third GPS port named

**Builder:** Claude (standard tier), worktree `~/aethelgard-4405`, branch `wo/aet-4405-serial-from-elf` from `main` `5eecc929`.
**Cadence:** build, with one measured stop (§1.1, did not fire). The work order is read from the docs branch (`~/.cache/aet-4405-wo.md`) and is not committed here.

Labels:
- **[read]**: at source, this repository or ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`;
- **[measured]**: run on Furnace, ArduPilot builds in that image under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`;
- **[inferred]**: neither.

## 1. §1.1: the decode table, with its predictions

Everything is in `spike/4405/`. The predictions (`README.md` §1, `predict.py`) were committed at `f3fdcdce`, before any ELF was decoded. The results were committed at `6c128938`.

- **Boards:** the thirteen `4404` built on 2026-11-19 in the same image (§2.3/§4 boards, plus its adversary's `adv-numforms` and `adv-realaf`), and the seven CI boards (`step-g`, `seed-pin`, `seed-af`, `seed-pkg`; the generated `sentinel-01`, `sentinel-02`, `serial-dshot-9600`), rebuilt from `5eecc929`'s files [measured].
- **The layout is one on all 20** [measured]:
  - `STT_OBJECT` in `.text` (allocated, not writable);
  - `16 × (rows + 1)` bytes;
  - `AP_GROUPEND` `{→"", 0, 0, 0, 0xFF, 0}` as the last entry, with no earlier type 0;
  - every name in read-only `.text`;
  - every row's flags 0;
  - types `_PROTOCOL` 1, `_BAUD` 3, `_OPTIONS` 3 (`UARTState::options` is `AP_Int32`), `_PASS*` 1.
  - Only the terminator's name pointer differs between boards, and only as an address.
  - **The §7 stop does not fire.**
- **Every row equals its prediction on all 20 boards** (a mechanical compare, `spike/4405/README.md` §2.2). The decode table is there, one row per board.
- **No disagreement with today's judge.** `main`'s binary gives `4404` §4's verdicts on the thirteen, finding for finding. `container/ardupilot-pinout.sh` exits 0 on the seven (§2.3).
  - The thirteen were judged on their topologies, re-solved from `4404`'s specs with only `schema_version` raised to 18; every regenerated `hwdef.dat` has `4404`'s lines exactly.
- **The baud trap, measured:** `0_BAUD` is `57` on fc-base (define 57600), `115` on the 115200 boards, and `9` on `serial-dshot-9600` (define 9600). Each maps back to the define through `map_baudrate`.

## 2. What was built

- **The reader,** `elf::serial_params(bytes) -> Result<Vec<SerialParam>, Refusal>` (`pincheck/src/elf.rs`).
  - It finds `_ZN16AP_SerialManager8var_infoE` through the same `object()` as `gpio_default_config`: one `STT_OBJECT` in read-only allocated `PROGBITS`, located by its section and by its `PT_LOAD`, both agreeing.
  - It reads whole 16-byte entries up to the first of type `AP_PARAM_NONE`, where ArduPilot stops (`AP_Param.cpp:1498-1500`). That entry must be exactly `AP_GROUPEND` and the symbol's last.
  - Each name is read where it is flashed: one `PT_LOAD`'s file image holds it in place, and it ends within 16 bytes.
  - Each row must be `n_PROTOCOL` (type 1), `n_BAUD` or `n_OPTIONS` (type 3), or a pass-through row. It must have flags 0, so its union is the float default (`AP_PARAM_FLAG_DEFAULT_POINTER` and the group pointers are flagged, `AP_Param.h:137-170`). Its default must be one its type holds exactly.
  - **Refused by name:** `SymbolMissing`, `SymbolSize` (a partial entry), `VarInfoUnterminated`, `MalformedVarInfo` (anything after `AP_GROUPEND`, an `AP_GROUPEND` that is not exactly one, flags, a wrong type including a nested group, an inexact default, a name that is not UTF-8), `SymbolNotLoaded` (a name not flashed), `VarInfoNameTooLong`, `UnknownVarInfoRow`, `VarInfoRowTwice`, `VarInfoPartialPort`.
  - **`map_baudrate`** is ArduPilot's (`AP_SerialManager.cpp:737-770`) with its `int32` semantics (`<= 0` is 57). It moved here from `identity.rs`, which held the only other copy in this crate. The scaled rows are named at `SerialParam::baud`: `0_BAUD` always (`:172`), and each `n_BAUD` left at its kbaud `#ifndef` default (`:47-162`).
- **The judge** (`identity.rs`): protocol, options and baud come from `serial_params`.
  - **SERIAL0's protocol is read** (`4404` D4 holds): with no console, any role but MAVLink standing there is named.
  - **A declared port with no rows is named** `MissingSerialParameters`, whatever stopped it: `HAL_HAVE_SERIALn_PARAMS`, or n past 9.
  - **The GPS cap:** `GpsPortUnbound` names each compiled GPS (or GPS2) port after the second, citing `AP_GPS_config.h:16` and `AP_GPS.cpp:345-349`.
  - `RcInputTwice` and the GPS cap count every compiled port, declared or not, as ArduPilot's `find_protocol_instance` does (`:600-612`).
- **Retired, each with why it is no longer needed:**

| retired | what it guarded | why it can go |
|---|---|---|
| reading `DEFAULT_SERIALn_PROTOCOL` (n ≥ 1) and the judge's `DEFAULT_PROTOCOLS` table (`:47-152`) | the define, else the `#ifndef` default for its n | `n_PROTOCOL`'s default is the compiler's result, with the `#ifndef` already resolved |
| `MalformedHeader` for a protocol that is neither a C integer nor a `SerialProtocol` name | a form the judge could not read as GCC does | the judge no longer reads the form; GCC's reading is the row |
| `MalformedHeader` for a protocol name also defined as a macro | a macro standing in for the enumerator | the same: the row holds what the macro made |
| `MalformedHeader` for `HAL_HAVE_SERIALn_PARAMS` other than `1` | a port whose parameters may not compile | `var_info` lists exactly the compiled rows; a declared port without them is `MissingSerialParameters` |
| reading `DEFAULT_SERIALn_OPTIONS` (and its C-integer refusal), with SERIAL7 and 8 read as a literal 0 (`:357`, `:376`) | the options as compiled | `n_OPTIONS` holds them, literal included (`eight-opt7`: define 8, row 0) |
| reading `DEFAULT_SERIALn_BAUD` for n ≥ 1 (and its refusal), and "not stated" when absent | the baud as compiled | `n_BAUD` holds it, the default included (`57`, `230`) |
| computing SERIAL0's parameter baud as define / 1000 | the GCS reopen rate | `0_BAUD` holds it |
| `MissingSerialParameters` only past SERIAL9 | an 11th port | the general absence rule covers it |

  The header reader's generic refusals (a conditional, `#undef`, `#pragma`, a second definition, malformed structure) stay: the other facts, and SERIAL0's boot rate, are still read through it.
- **Kept from `hwdef.h`: SERIAL0's boot rate** (decision D1). `init_console` opens SERIAL0 at the `DEFAULT_SERIAL0_BAUD` literal (`AP_SerialManager.cpp:419`), which `var_info` cannot hold: `0_BAUD` is that value / 1000 in C integer division (`:172`), so 115999 and 115200 share one row. Tier 1 pins it: define 115999 with row 115 is `WrongSerialBaud` "115999 baud, then 115200 baud". Mutant M19 is killed.
- **Tests** (`pincheck/tests/identity.rs`; the synthetic ELF in `tests/common/mod.rs` gains a `var_info`, a raw tail and an unflashed RAM section as knobs; `cli/tests/pinout_tier1.rs` gives its judged ELF SERIAL0's rows):
  - Each `4404` §1.1 (c) mutation is still refused by name, now from `var_info` while `hwdef.h` holds fc-base's correct defines. The same mutations in `hwdef.h`, with `var_info` correct, pass.
  - Each `hwdef.h` trap the old judge refused is shown harmless (`the_header_s_serial_defines_no_longer_reach_the_judge`). Beside each, fc-base's `var_info` passes and a mutated one fails.
  - SERIAL7: the define is ignored, the row is judged, both ways. Option bits, RC twice, the console, the bauds, the GPS cap (two pass; the eight-port board names SERIAL4, 5, 6; GPS2 counts; an undeclared port counts).
  - The reader: the spike's fc-base table read back exactly, and each refusal above.
- **No CLI source changed.** No generator, IR, container, workflow, `ci/` or specification change.

## 3. §2 acceptance

- **The decode table, with predictions:** §1, and `spike/4405/README.md` §2.
- **The judge reads the ELF** [measured]. These are real builds whose `hwdef.h` differ from fc-base's only in the serial defines (and the board name), judged crossed on fc-base's topology (`spike/4405/out/cross.txt`):

| ELF | `hwdef.h` | `main` | this branch |
|---|---|---|---|
| fc-base | adv-opt-swap (`1_OPTIONS 8`) | exit 1, `WrongSerialOptions SERIAL1 0x8` | **exit 0** |
| adv-opt-swap | fc-base | exit 0 | **exit 1, `WrongSerialOptions SERIAL1 0x8`** |
| fc-base | adv-proto-swap | exit 1, two `WrongSerialProtocol` | **exit 0** |
| adv-proto-swap | fc-base | exit 0 | **exit 1, `WrongSerialProtocol` SERIAL1 and SERIAL2** |

  `eight-opt7` (define `DEFAULT_SERIAL7_OPTIONS 8`, row `7_OPTIONS 0`) gives no options finding; its only findings are the GPS cap's.
- **The GPS cap refuses the eight-port board's third GPS port by name** [measured]: `eight`, `eight-opt6` and `eight-opt7` each give `GpsPortUnbound` SERIAL4, SERIAL5 and SERIAL6 ("at most 2 GPS ports, SERIAL1 and SERIAL3"). `main` passed `eight`.
- **Every committed and CI-built board passes as on `main`** [measured]: `container/ardupilot-pinout.sh`, unchanged, with this branch's binary on the seven rebuilt CI boards: exit 0, no `::error`. The thirteen `4404` boards give `main`'s verdicts plus the GPS cap on the three eight-port boards (`spike/4405/out/judge-branch.txt`).
- **Gates and CI:** §7.

## 4. Adversarial pass

(pending)

## 5. Decisions for the architect

- **D1. SERIAL0's boot rate stays read from `hwdef.h`.** It is the one serial fact `var_info` cannot hold (§2). Retiring it would reopen `4400b2` A7: define 115999 opens the console at 115999, while its row says 115. **Recommended:** keep it, as built. If ruled retired, it is a one-line removal and the A7 test goes with it.
- **D2. At compiled defaults only the first GPS port is bound.** `GPS2_TYPE` defaults to 0 (`AP_GPS_Params.cpp:31`); only `GPS1_TYPE` is set to `HAL_GPS1_TYPE_DEFAULT` (1) (`AP_GPS.cpp:328`). `AP_GPS::init` gives a port only to a receiver whose type `needs_uart` (`AP_GPS.cpp:345-349`, `:306-318`), so the second GPS port waits for the user to set `GPS2_TYPE` [read]. The ruled rule names the third, and that is what is built. **Recommended:** a §7.1 row deciding whether a second GPS port is a finding, or must be acknowledged as needing `GPS2_TYPE`, while no `defaults.parm` is emitted.
- **D3. A baud now judged where it was "not stated".** A port n ≥ 1 with no `DEFAULT_SERIALn_BAUD` was always a finding. It is now judged at its row's default (`57` → 57600 on 1, 2, 5–9; `230` → 230400 on 3 and 4). No CI board has such a non-RC port. **Recommended:** accept; the ELF states it.

## 6. Reduction pass, mutants and gates

(pending)

## 7. Status

(pending)
