# AET-WO-4406 — `check pinout` refuses firmware that can override its compiled defaults at boot, and reads the GPS cap from the ELF

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4406-defaults-guard`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4 (`pincheck`). Written 2026-11-20. **Cadence:** a build with one measured stop.

**Grounds** (`4405` D4, D5, #277; roadmap §7.1):
- **The judge reads compiled defaults; a defaults file replaces them at boot.** Since `4405`, every serial fact comes from `AP_SerialManager::var_info` in the ELF. A parameter defaults file overrides those defaults at boot. A `hwdef.dat` alone can embed one. `4405`'s adversarial pass (A1) built such a board, and both `main` and `4405` passed it.
- **The generator emits no defaults file today.** Its header says motor settings "wait for the defaults.parm work order (D10)" (`aethelgard-hwdef/src/lib.rs`). This hole opens the day that work order lands, so the guard comes first.
- **`4405` D4 measured the signatures,** on sentinel-01 and the adversary's build:
  - `AP_Param::load_defaults_file` (`_ZN8AP_Param18load_defaults_fileEPKcb`) is linked only in a ROMFS build. `reload_defaults_file` is linked in every build, so it is not a signature.
  - `load_embedded_param_defaults` is linked when defaults are embedded.
  - `param_defaults_data`'s length field is not 0 when defaults are embedded.
  - ROMFS can list `defaults.parm`.
- **The GPS cap is a constant of the tag** (`4405` D5, A4, A5). `GpsPortUnbound` counts against `GPS_MAX_RECEIVERS` 2 (`AP_GPS_config.h:16`) as a number in `pincheck`. `AP_GPS::var_info` holds one `n_` subgroup row per compiled receiver, in the same `AP_Param::GroupInfo` layout `4405`'s reader already decodes.

> **What this is:**
> - `check pinout` refuses, by name, an ELF that can apply parameter defaults at boot (`ParamDefaultsOverride`), naming which signature it found;
> - the GPS cap is read from `AP_GPS::var_info`, not assumed;
> - each serial row's offset is held to the layout `4405` measured, if §1.1 shows it holds on every board.
>
> It is NOT:
> - reading a defaults file's values (the `defaults.parm` work order reads them, once boards emit them);
> - the second-GPS-port decision (`4405` D2 stays its own row);
> - the console baud refusal at `validate` (`4405` D1 stays its own row);
> - a change to the generator, ArduPilot, the image or the workflows.

## 1. Build

1. **Measure first,** in `spike/4406/`, with predictions committed before the runs.
   - On every board `4405` §1.1 decoded (its 20) and on `4405`'s A1 adversary build, record:
     - each of the four signatures above: present or absent, and where it was found;
     - the `n_` subgroup rows of `AP_GPS::var_info`, and the cap they give;
     - each SERIALn row's offset against `24n + {0, 4, 8}`.
   - **Prediction:** every committed and CI-built board shows no signature, and its cap is 2. The A1 build shows at least one signature.
   - **Stop and report** if a committed or CI-built board shows a signature. That board passes today and would be refused. The architect rules whether the signature or the board is wrong.
   - **Stop and report** if `AP_GPS::var_info` is not one `GroupInfo` array with `n_` subgroups on every board. Name what differs, with the bytes.
2. **The guard,** in `pincheck/src/elf.rs`.
   - It reads the symbol table, `param_defaults_data`'s length field and the ROMFS directory. Each read is refused by name if it is malformed.
   - `ParamDefaultsOverride` names the signature found. Every signature found is listed, not only the first.
   - A missing ROMFS is not a finding: a board without ROMFS cannot list a file. A ROMFS that cannot be read is refused.
3. **The GPS cap from the ELF.** `GpsPortUnbound` counts against the cap read from `AP_GPS::var_info`.
   - An ELF where AP_GPS is compiled out (no `var_info`) and a GPS-protocol port is declared is refused by name: no consumer exists.
   - A missing `AP_GPS::var_info` with no GPS port declared is not a finding.
4. **Row offsets.** Do this only if §1.1 shows `24n + {0, 4, 8}` on every board. Then `serial_params` refuses a row at any other offset by name. If not, record the measured layout in FEEDBACK and build nothing for it.
5. **Tests.**
   - Every committed and CI-built board passes as on `main`.
   - The A1 adversary build is refused by `ParamDefaultsOverride`. Commit a fixture cut from it, or build it in the pinned image as `4405` did.
   - **Mutants:**
     - drop each signature in turn: a test fails for each;
     - take `reload_defaults_file` as a signature: every committed board is refused, and a test fails;
     - hold the cap at 2 as a constant again: a test fails, on a fixture whose `var_info` gives another count.

## 2. Acceptance

- **The signature table** is in FEEDBACK, one row per board, with the predictions beside it.
- **The guard refuses the A1 build** by name, and every committed and CI-built board passes.
- **The cap is read:** a test shows a `var_info` with a cap other than 2 changing the verdict.
- **The CLAUDE.md gates pass, and CI is green,** the ardupilot-image `pinout` job included. `TIER2_MIN` rises by your own tier-2 count, if any.

## 3. Invariants and fences

- **Invariants:** `INV-07` (the judge reads the artifact, not the generator), `INV-06`.
- **Fence:**
  - `crates/aethelgard-pincheck/src` (`elf.rs`, `identity.rs`, and the types they share);
  - `crates/aethelgard-cli/src/pinout.rs`, only if a new finding must reach the CLI by a new path;
  - tests and their fixtures;
  - `spike/4406/`;
  - FEEDBACK.
  - **Not:** the generator (`aethelgard-hwdef`), `container/`, workflows, the IR, specifications, or `ci/` (except a weight row the P1/P2 guard asks for).
- **Running beside you:**
  - `4240a` (#281): IR, svd-hal and the generator;
  - `4241` (#279): the firmware and the twin;
  - `1210` (#280): IR `validate`;
  - `1318` (#283): eda;
  - `3306`: evolve.

  None touches `pincheck`. Merge `main` before READY.
- **ArduPilot builds** run in the pinned image only, under `capped-run`, one container of yours at a time, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-4406/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A build that applies defaults at boot with none of the four signatures.** Examples: a defaults file reached by another path, or a parameter set from a script or a mission. Cite ArduPilot's source for each path you check. A path found is a finding, with a proposed signature.
- **A `param_defaults_data` the reader misparses,** so the guard passes a board with embedded defaults.
- **A GPS-protocol port bound by something other than AP_GPS,** if any. Cite the source.

## 7. Flag-and-stop conditions

- **A committed or CI-built board shows a signature.** Stop with the bytes.
- **`AP_GPS::var_info`'s layout differs between boards.** Stop with the bytes.
