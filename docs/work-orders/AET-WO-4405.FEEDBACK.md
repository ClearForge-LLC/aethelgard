# AET-WO-4405 FEEDBACK — serial protocol, options and baud read from the firmware; a third GPS port named

**Builder:** Claude (standard tier), worktree `~/aethelgard-4405`, branch `wo/aet-4405-serial-from-elf` from `main` `5eecc929`.
**Cadence:** build, with one measured stop (§1.1, did not fire). The work order was built from the docs-branch copy (`~/.cache/aet-4405-wo.md`), identical to the one `main` `bb1bcb8c` merged.

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

A fresh subagent ran it, on Opus because it is judgement work. It worked in its own clone at `0409ad4f`, made three builds of its own in the pinned image (`capped-run --mem 8G`, `--network none`), and fuzzed the reader. Its evidence is in `~/.cache/aet-4405/adv/`. I re-judged its three builds with this branch's binary myself: each exits 0 with no finding, as it reported [measured]. **It found no input that passes on this branch and fails on `main`: this diff opens no hole.**

| id | probe | severity | verdict, evidence |
|---|---|---|---|
| A1 | A defaults file overriding `SERIALn_*` after `var_info` | medium-high, **pre-existing** | **Passes wrongly on both binaries, by two routes** [measured]. (1) fc-base with a `defaults.parm` beside its `hwdef.dat` (`SERIAL1_PROTOCOL -1`, `SERIAL3_PROTOCOL 2`, `SERIAL3_OPTIONS 3`): `var_info` byte-identical, ROMFS gains `defaults.parm`, `load_defaults_file` linked. (2) fc-base with only two `hwdef.dat` lines, `define AP_PARAM_MAX_EMBEDDED_PARAM 1024` and `env DEFAULT_PARAMETERS <file>`: waf's `apj_tool` patches `AP_Param::param_defaults_data` in the linked ELF (`PARMDEF`, 67 bytes, `SERIAL3_PROTOCOL -1`). `get_default_value` returns an override before `def_value` (`AP_Param.cpp:2612-2622`). **Decision D4.** The README residual now names both routes. |
| A2 | The retired `hwdef.h` refusals: agree for the wrong reason | none | **Hold.** Each retired form is folded into `var_info` by the compiler, and the crossed-header proof (§3) shows the ELF is judged. No library or Copter code writes `SERIALn_*` by name; the one caller is an example that is not linked [measured grep]. `HAL_OTG1_CONFIG`'s forced MAVLink 2 on SERIAL0 matches `4404` D4. |
| A3 | Is the second GPS port bound at compiled defaults? | medium, **pre-existing** | **No.** `1_TYPE` and `2_TYPE` both decode to default 0 in fc-base's `AP_GPS::var_info` [measured]; only `params[0]` is set to `HAL_GPS1_TYPE_DEFAULT` (1) (`AP_GPS.cpp:328`) [read]. So fc-base's SERIAL3 GPS is unbound until a user sets `GPS2_TYPE`. **Decision D2.** The README and the code's doc comment said "binds the first two"; both now say what happens at defaults. |
| A4 | `GPS_MAX_RECEIVERS` redefined | medium-low, **pre-existing** | **The constant 2 misses a one-receiver build** [measured]: fc-base with `define GPS_MAX_RECEIVERS 1` builds, `AP_GPS::var_info` loses its `2_` subgroup, and both binaries exit 0. Above 2 binds no more (only `1_` and `2_` have rows, `AP_GPS.cpp:276-286`). A GPS or RCIN port whose consumer is compiled out (`HAL_GPS1_TYPE_DEFAULT 0`, `AP_GPS_ENABLED 0`, `AP_RCPROTOCOL_ENABLED 0`) also passes [read]. The generator emits none of these. **Decision D5.** |
| A5 | A row whose name disagrees with its offset | low (hostile ELF) | **Passes.** Swapping the `offset` of `1_PROTOCOL` and `2_PROTOCOL` in a copy of fc-base's ELF exits 0 [measured]; ArduPilot applies a default by offset (`AP_Param.cpp:1490-1497`). On all 11 boards checked, every port row's offset is `24n + {0 BAUD, 4 OPTIONS, 8 PROTOCOL}` [measured]. A residual, named in the README; a measured-constant guard is possible (D5). |
| A6 | A decoy `var_info` symbol | low | **Not caught**, as for `gpio_default_config` without `hwdef.h`. The README said the `hwdef.h` cross-check catches a decoy; that is true only for the GPIO image. Corrected in the README and `elf.rs`. |
| A7 | Other row kinds: nested group, flags, `DEFAULT_POINTER`, wrong type, a duplicate, an unknown name, rows after `AP_GROUPEND`, a malformed `AP_GROUPEND` | none | **Each refused by name and tested.** The adversary's own single-guard mutants of the `AP_GROUPEND` checks (`e[4..14]`, `idx`, the empty name, must-be-last) and of `n > 0` on `_OPTIONS` are all killed [measured]. |
| A8 | Fuzz | none | **3,000 mutated real ELFs** (fc-base and eight; 1–4 byte changes in `var_info`, the headers or anywhere; 10 % truncated): no panic; every failure is a named refusal; the 15 differing `Ok` reads were faithful reads of a mutated default [measured]. |
| A9 | A guard whose removal no test sees | low | **Fixed.** Dropping `held`'s lower bound survived: nothing tested a default below its type's range. `1_PROTOCOL -129` and `1_BAUD -3e9` are now refused in tier 1. The mutant (M11b) is killed. |
| A10 | The kept SERIAL0 `hwdef.h` read | none | **Right.** The HAL opens SERIAL0 at `SERIAL0_BAUD`, which is `DEFAULT_SERIAL0_BAUD` or else 115200 (`HAL_ChibiOS_Class.cpp:42-45`, `:259`), and so does `init_console` (`:419`). `SERIAL0_DEFAULT_BAUD` matches. |
| A11 | The GPS baud | low, **pre-existing** | `WrongSerialBaud` judges the rate the port opens at (`AP_SerialManager.cpp:590`); AP_GPS then probes its own list of rates (`AP_GPS.cpp:79`, `:700-723`) [read]. Unchanged from `main`. |

Not probed, inherent to judging compiled defaults: values saved in parameter storage that survive a reflash, and Lua scripts in ROMFS.

The adversary also notes that `4404` §5's "defaults.parm … hold" cannot be traced to evidence in `4404`'s kit; it most likely meant that no generated board ships one. A1 shows the judge is blind to a defaults file.

## 5. Decisions for the architect

- **D1. SERIAL0's boot rate stays read from `hwdef.h`.** It is the one serial fact `var_info` cannot hold (§2). Retiring it would reopen `4400b2` A7: define 115999 opens the console at 115999, while its row says 115. **Recommended:** keep it, as built. If ruled retired, it is a one-line removal and the A7 test goes with it.
- **D2. At compiled defaults only the first GPS port is bound.** `GPS2_TYPE` defaults to 0 (`AP_GPS_Params.cpp:31`); only `GPS1_TYPE` is set to `HAL_GPS1_TYPE_DEFAULT` (1) (`AP_GPS.cpp:328`). `AP_GPS::init` gives a port only to a receiver whose type `needs_uart` (`AP_GPS.cpp:345-349`, `:306-318`), so the second GPS port waits for the user to set `GPS2_TYPE` [read]. The ruled rule names the third, and that is what is built. **Recommended:** a §7.1 row deciding whether a second GPS port is a finding, or must be acknowledged as needing `GPS2_TYPE`, while no `defaults.parm` is emitted.
- **D3. A baud now judged where it was "not stated".** A port n ≥ 1 with no `DEFAULT_SERIALn_BAUD` was always a finding. It is now judged at its row's default (`57` → 57600 on 1, 2, 5–9; `230` → 230400 on 3 and 4). No CI board has such a non-RC port. **Recommended:** accept; the ELF states it.
- **D4. Parameter defaults files (A1).** The judge reads compiled defaults; a defaults file overrides them at boot, and a `hwdef.dat` alone can embed one. Both binaries pass such a board. The generator's own header says motor settings "wait for the defaults.parm work order (D10)" (`aethelgard-hwdef/src/lib.rs`), so this becomes reachable once that work order lands. **Recommended:** before any defaults-file work order, a cheap ELF guard in `pincheck` (inside this crate's fence, but not this work order's scope, so not built). It would refuse by name when `AP_Param::load_defaults_file` (`_ZN8AP_Param18load_defaults_fileEPKcb`, linked only in the ROMFS build; `reload_defaults_file` is in every build) or `load_embedded_param_defaults` is linked [measured on sentinel-01 and the adversary's build], `param_defaults_data`'s length is not 0, or ROMFS lists `defaults.parm`. That work order would read the overrides instead, once boards emit them.
- **D5. The GPS cap and the row offsets are constants of the tag (A4, A5).** **Recommended:** a §7.1 row to read the cap from the ELF: count the `n_` subgroup rows in `AP_GPS::var_info` (the same `GroupInfo` layout), and refuse a port whose consumer is compiled out. Optionally, hold each port row's offset to the measured `24n + {0, 4, 8}`. Neither is reachable from the generator today.

## 6. Reduction pass, mutants and gates

- **Reduction pass (diff): −45 lines net from one table · 1 duplicate removed · 2 mechanisms removed · 2 protected kept · 1 finding below.**
  - **The `SerialProtocol` name table** went from 51 rows to 6. Its parsing job is gone (§2), so it only labels a finding now: None, MAVLink, MAVLink 2, GPS, GPS2 and RCIN, the names a finding can carry, each tied to the judge's own constants. Any other value is shown as its number.
  - **Duplicate:** `map_baudrate` has one copy in the crate (`elf.rs`); `identity.rs`'s copy is gone.
  - **Mechanisms:** a `var_info` name is read where its `PT_LOAD` flashes it, not through the section table as well. That dropped the section lookup and its offset comparison, which no test could reach (mutant M17 survived before the change). Names identify rows and do not change what ArduPilot applies (A5).
  - **Protected, kept:** `map_baudrate` (the generator holds its own copy, labelled `belt-and-suspenders:` at both sites) and the role → protocol match (`RUL-20`).
  - **Finding, outside the fence:** `container/ardupilot-pinout.sh:14-16` still says the identity judge reads "bauds" from `hwdef.h`. It now reads them from the ELF, apart from SERIAL0's boot rate. The comment needs a one-word fix by whoever next touches `container/`.
- **Mutants** (`spike/4405/mutants.py`; one per guard this work order adds, each against `pincheck/tests/identity.rs`; the file is restored from a copy, and a mutant that does not compile is never counted as killed): **19 of 19 killed, each by a named failing test** (`spike/4405/out/mutants.txt`), on the head that merged `main` `943f7112`.
  - The work order's three: **M1**, the baud float taken as a baud, is killed on SERIAL0 (`the_serial_order_and_bauds_are_judged`, `each_baud_is_the_parameter_mapped`, …). **M2**, reading past `AP_GROUPEND`, is killed (`every_malformed_var_info_is_refused_by_name`). **M3**, the GPS cap dropped, is killed: the five-GPS-port board then passes, and `a_third_gps_port_is_named` fails.
  - The rest: GPS2 counted; undeclared ports counted; flags; type; a duplicate; an unknown row; a fractional default; a default above and below its type (M11, M11b, the adversary's A9); a partial port; `AP_GROUPEND`'s fields; the name length; the name's end flashed; a missing declared port; SERIAL0's boot rate; RC twice.
  - Dropped as not meaningful: one equivalent mutant (`continue` for `break` at a terminator that must be last), and one that broke every read.
- **CI planner (`1910`):** no tier-2 test; no test in a lock-serialised binary; the renamed tests are tier 1 in `pincheck`, under 0.5 s, with no row in `ci/test-weights.tsv` (P1, P2). `TIER2_MIN` is unchanged.

## 7. Gates, CI and status

- **Gates** at `cd1fafbe` (`main` `943f7112`, `4240v`, merged; capped scope):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace`: **2,396 passed, 0 failed**. An earlier run at `0409ad4f` gave 2,360 passed, 0 failed.
- **The final binary on the real boards** [measured]. It reads `4240v`'s topology schema, so every topology was re-solved first: the CI three and Sentinel-01 from their committed specs, and `4404`'s three at specification 19 with Serial-Four's `core_supply`. Every regenerated `hwdef.dat` has `4404`'s lines exactly.
  - `container/ardupilot-pinout.sh`, unchanged, on the seven CI boards: exit 0, no `::error` (`spike/4405/out/pinout-final.log`).
  - The thirteen `4404` boards: each verdict is identical to the earlier branch run (`out/judge-final.txt`).
  - The crossed headers: as §3 (`out/cross-final.txt`).
- **CI on the first push** (`9fc09d59`, PR #277): every job but one tools shard had passed when checked, the ardupilot-image `pinout` job among them (run 44711552849).
- **Outside the fence:**
  - the two crate READMEs (`aethelgard-pincheck`, and the `check pinout` paragraph and exit-3 list in `aethelgard-cli`'s). The work order's rules make their old "read from `hwdef.h`" sentence false, so they were brought true;
  - `spike/4405/` holds Python decode and prediction scripts, as `spike/1303` does. No ELF or `hwdef.h` byte is committed.
  - Nothing else: no generator, IR, specification, `container/`, workflow or `ci/` change.
- **CI green on `4e92c5b4`** (the final code, `main` `d70a99e5` merged): `ci` run 44714820375 (gates, tools shards, bench-package), `ardupilot-image` run 44714820705 (the four hand boards, the generated boards, **`pinout`**, `foreign`, `pinned`), and `engine-image` run 44714820532. `TIER2_MIN` is unchanged.
- **Status: READY.** This commit adds only this line and the CI record above, and it is the last push. One PR (#277), left unmerged; the architect merges. The title and body are in `~/.cache/aet-4405/pr-title-body.md`.
