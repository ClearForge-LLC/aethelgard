# AET-WO-4405 — `check pinout` reads each serial port's protocol, options and baud from the firmware, and names a GPS port ArduPilot never binds

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4405-serial-from-elf`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4 (`pincheck`). Written 2026-11-20. **Cadence:** a build with one measured stop.

**Grounds** (`4404` D1, D3 and §2.4, #244; roadmap §7.1):
- **Today's judge reads `hwdef.h`, the generator's own build directory.** `check pinout` judges each serial port's protocol, options and baud from the defines there (`pincheck/src/identity.rs`, about L660–L930). That binds them to the firmware only by coming from the same build directory. To keep that reading honest, the judge must refuse `hwdef.h` forms the compiler would read differently (`4404` §5, cases 1–10).
- **The firmware itself holds all three facts.** `4404` §2.4 measured them:
  - `AP_SerialManager::var_info` (`_ZN16AP_SerialManager8var_infoE`) is an array of 16-byte `AP_Param::GroupInfo` entries (`AP_Param.h:206-218`), ended by `AP_GROUPEND`;
  - it lists exactly the compiled SERIALn rows (`HAL_HAVE_SERIALn_PARAMS`), each `#ifndef` and the SERIAL7/8 literals already folded in;
  - **the baud trap:** `0_BAUD`'s default is `57`, because its `AP_GROUPINFO` divides by 1000 (`:172`). A reader must apply ArduPilot's `map_baudrate` rule, never take the float as a baud.
- **`pincheck` already reads the ELF.** `pincheck/src/elf.rs` reads the GPIO image and the serial table (`serial_table`, about L460).
- **A GPS port can compile and never be bound.** ArduPilot binds only the first two GPS-protocol ports (`AP_GPS_config.h:16`, `GPS_MAX_RECEIVERS 2`; `AP_GPS.cpp:345-349`). A third passes today. `4404`'s adversarial pass measured it on the eight-port board (A-D2).

> **What this is:**
> - `check pinout` judges each compiled port's protocol, options and baud from `var_info` in the ELF it is given, by the same finding kinds (`WrongSerialProtocol`, `WrongSerialOptions`, `WrongSerialBaud`);
> - a port the specification declares that `var_info` does not compile is refused by name;
> - a third GPS-protocol port is refused by name, as `RcInputTwice` refuses a second RCIN, citing ArduPilot's lines.
>
> It is NOT:
> - a change to the generator, ArduPilot, the image or the workflows;
> - the SPI, PWM, I2C or barometer facts (`4400b2` D1 (b)'s row stays).

## 1. Build

1. **Measure first,** in `spike/4405/`, with predictions committed before the runs.
   - Decode `var_info` on every board `4404` §2.3 built, and on the three generated boards CI's `pinout` job builds:
     - each SERIALn row's protocol, options and baud default, after `map_baudrate`;
     - against what today's `hwdef.h` judge reads.
   - Any disagreement is a finding, not a fix: today's judge passed that board.
   - **Stop and report** if `var_info`'s layout differs between the boards (the struct, the terminator, the name pointers). Name what differs.
2. **The reader,** in `pincheck/src/elf.rs`: `serial_params(bytes) -> Result<Vec<SerialParam>, Refusal>`.
   - Find the symbol. Read whole 16-byte entries up to `AP_GROUPEND`. Resolve each `name` pointer through the sections.
   - Refuse by name anything malformed: a missing symbol, a partial entry, no terminator, a name outside the image, or a duplicate row.
   - `map_baudrate` as ArduPilot defines it, cited. Each scaled row is named in the code.
3. **The judge.** Protocol, options and baud come from `serial_params`.
   - The `hwdef.h` reads of those three facts are retired.
   - Any `hwdef.h` refusal that exists only to keep that reading honest goes with them. List each one removed, and why it is no longer needed.
   - SERIAL0's rule (`4404` D4) holds. Its protocol is read, no longer assumed.
   - A declared port missing from `var_info` is refused by name.
4. **The GPS cap.** A third GPS-protocol port is refused by name, citing `AP_GPS_config.h:16` and `AP_GPS.cpp:345-349`.
5. **Tests.**
   - Every committed and CI-built board passes as on `main`.
   - Each of `4404`'s §1.1 (c) mutations is still refused by name. Each `hwdef.h` trap that no longer reaches the judge is shown to be harmless: the ELF says what the compiler did.
   - **Mutants:**
     - take the baud float as a baud: a test fails on SERIAL0;
     - read past `AP_GROUPEND`: a test fails;
     - drop the GPS cap: the five-GPS-port board passes, and a test fails.

## 2. Acceptance

- **The decode table** is in FEEDBACK, with predictions beside it and any disagreement with today's judge named.
- **The judge reads the ELF:** shown by a board whose `hwdef.h` and `var_info` disagree. `4404`'s eight-opt7 is one: the define is ignored by ArduPilot, and `var_info` says 0. The ELF's value is the one judged.
- **The GPS cap** refuses the eight-port board's third GPS port by name.
- **The CLAUDE.md gates pass, and CI is green,** the ardupilot-image `pinout` job included. `TIER2_MIN` rises by your own tier-2 count, if any.

## 3. Invariants and fences

- **Invariants:** `INV-07` (the judge reads the artifact, not the generator), `INV-06`.
- **Fence:**
  - `crates/aethelgard-pincheck/src` (`elf.rs`, `identity.rs`, and the types they share);
  - `crates/aethelgard-cli/src/pinout.rs`, only if the ELF must reach the serial judge by a new path;
  - tests and their fixtures;
  - `spike/4405/`;
  - FEEDBACK.
  - **Not:** the generator (`aethelgard-hwdef`), `container/`, workflows, the IR, specifications, or `ci/` (except a weight row the P1/P2 guard asks for).
- **Running beside you:**
  - `4240v` (#259): svd-hal and spec 19;
  - `2306` (#269): `cli` verdict lines;
  - `2305` (#271): chaos, `renode.rs` and `verify.rs`;
  - `1316` (#273): eda `schematic.rs` and the CLI's `validate`.

  None touches `pincheck`. Merge `main` before READY.
- **ArduPilot builds** run in the pinned image only, under `capped-run`, one container of yours at a time, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-4405/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A board where `var_info` and `hwdef.h` agree for the wrong reason,** so retiring a `hwdef.h` refusal opens a hole.
- **A build whose `var_info` the reader misparses** but which still passes. For example, an entry with a different `type`, or a group nested inside it.
- **The GPS cap** under a protocol that ArduPilot also counts as GPS, if any. Cite the source.

## 7. Flag-and-stop conditions

- **`var_info`'s layout differs between boards** in the pinned ArduPilot. Stop with the bytes.
- **A committed or CI-built board fails** on the ELF reading where it passed on `hwdef.h`. Stop with the finding: it may be a real fault today's judge missed. The architect rules.
