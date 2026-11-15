# AET-WO-4400a — ArduPilot in its own pinned image, built in CI, and a pinout check read from the linked ELF

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400a-ardupilot-image`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, the first build piece of `AET-WO-4400` (spike ruled on PR #130, comment 6887864517). **Cadence: build, with one measured stop (§1.4).** It changes no solver, board-crate or generator code, so it runs beside `4300c`, `1307`, `1303` and `4210`.

**Grounds, all measured in the `4400` spike (`AET-WO-4400.FEEDBACK.md` §2, §4, §5) on Furnace:**
- **Unmodified ArduPilot builds on Furnace:** `Copter-4.7.2` at `fb773567`, toolchain `gcc-arm-none-eabi-10-2020-q4-major` (sha256 `21134caa478bbf5352e239fbc6e2da3038f8d2207e089efc96c3b55f1edcd618`), with the frozen pip set in FEEDBACK §2.
  - `./waf copter` takes 60 s and 331 CPU-s at `-j6`, with a 1.23 GB peak.
  - The build phase makes 0 network calls.
- **`spike/4400/hwdef/step-g.hwdef.dat` builds** a 21-line Sentinel-01 board.
- **The three seeds** (`seed-pin`, `seed-af`, `seed-pkg`) are legal `hwdef.dat` that ArduPilot builds without complaint.
- **`gpio_default_config`, 308 bytes in the linked ELF,** is what `gpio_init` writes at reset: 11 ports × `MODER OTYPER OSPEEDR PUPDR ODR AFRL AFRH`. `hwdef.h`'s `VAL_GPIOx_*` initialisers agree with it.

**Ruled (PR #130):**
- **D4:** `APJ_BOARD_ID`, by unsetting `GITHUB_ACTIONS` for the ArduPilot build step, measured here.
- **D5:** a second image.
- **D7:** forward, reverse and package checks; ELF primary, `hwdef.h` cross-check; the check and the generator in crates that do not depend on each other.
- **D8:** one Renode run of step G.

> **What this is:** ArduPilot built from pinned sources in an image of its own, by CI, on our fixtures, and a Rust check that reads what ArduPilot will write to the GPIO registers at reset and compares it with the `SolvedTopology`, so that `4400b`'s generator is judged by something it does not compute (`INV-07`).
>
> It is NOT:
> - the `hwdef.dat` generator (`4400b`, after `4300a` and `4300b`);
> - `OSPEEDR`/`PUPDR`/`OTYPER` judgement (`4400b`, against `4300c`'s solved speeds);
> - ArduPilot in the twin beyond D8's one recorded run.

## 1. Build

1. **The image `aethelgard-ardupilot`,** from `container/ardupilot.Dockerfile`, under `RUL-15`'s rules exactly as `engine-image.yml` applies them:
   - **The base:** an Ubuntu or Debian base pinned by index digest, with apt packages pinned to exact versions. It needs only what S1 found waf uses (`python3` with `venv`, `make`, `git`, `rsync`) plus `bzip2` to unpack.
   - **The toolchain:** fetched from `firmware.ardupilot.org` and checked against the sha256 above in the Dockerfile.
   - **The clone:** `git clone --depth 1 -b Copter-4.7.2 --recurse-submodules --shallow-submodules`, then a check that `HEAD` is `fb773567896014da007bca58358704d4a0d8516f`; refuse otherwise.
   - **The pip set:** exact versions, installed with `--require-hashes` (write the hashes).
   - **`container/check-pins.sh`** (or a sibling) refuses an unpinned line in the new Dockerfile.
   - **`.github/workflows/ardupilot-image.yml`:** pull requests build and smoke it under `--network none`; a push to `main` publishes from the one job holding `packages: write`.
   - **The smoke:** `./waf configure --board MatekH743 && ./waf copter` inside the image under `--network none`, plus `git status --porcelain` empty.
   - **The pin:** `container/ARDUPILOT-IMAGE` holds the digest after the first publish. Until then consumers build from the Dockerfile, as `RUL-15` says.
   - **The licence:** a `container/ARDUPILOT-LICENSE.md` names GPL-3.0 and states that the image carries ArduPilot's complete source at the pinned commit. Nothing GPL enters a crate, the tools image or the engine image.
2. **The CI job** `ardupilot` (in `ardupilot-image.yml`, or `ci.yml` if it fits better; say which and why) builds four boards in the image under `--network none`:
   - **The boards:** step G and the three seeds, copied into an untracked board directory exactly as `spike/4400/s3.sh` does. Move the four `hwdef.dat` files to a fixture directory under the new crate; they are ours, not ArduPilot's.
   - **The artifacts:** each board's ELF and `hwdef.h` go to the check.
   - **D4:** this step runs with `GITHUB_ACTIONS` unset and a comment saying why. If ArduPilot still refuses the board ID, **STOP** with the message (§1.4).
   - **The time:** record each board's wall time on the runner. Measure whether `-j$(nproc)` and building the four in one tree beats four clean trees. The job stays under 20 minutes.
3. **The check: crate `aethelgard-pincheck`** (`MIT OR Apache-2.0`). It reads a `SolvedTopology`, a linked ELF, optionally `hwdef.h`, and the solver's vendored `STM32H753ZITx.xml`.
   - **The ELF:** a small strict ELF32 little-endian reader. Find `gpio_default_config` in `.symtab` by name and size (refuse if absent, duplicated or not 308 bytes), map its address to its section's file offset, and decode the 11 × 7 words.
     - Use no toolchain at check time. A vetted dependency (`object`, read-only features) is acceptable if it beats hand-written code; say which and why.
     - **Every malformed input is refused by name** (`INV-06`): truncated headers, a symbol outside its section, a wrong machine or class.
   - **Forward:** every topology assignment's pin must be in alternate mode at the solved AF, and every `gpio` entry an output.
   - **Reverse:** every pin whose `MODER` differs from its reset value must be named by the topology, or by a **declared allow-list passed as an input** (USART3 PD8/PD9 for step G). The check holds no list of its own.
   - **Package:** every pin the image configures, in any port and mode other than reset, must exist in the package XML. `seed-pkg` (PI0) must fail.
   - **Cross-check:** when `hwdef.h` is given, its `VAL_GPIOx_MODER/AFRL/AFRH` must equal the ELF's decode for every port. A disagreement is a refusal, never a pass.
   - **Separation (D7):** the crate depends on `aethelgard-ir` for the topology type and the package data, and on nothing that generates firmware or `hwdef.dat`. A test runs `cargo tree` (or reads `cargo metadata`) and fails if `aethelgard-pincheck` reaches `aethelgard-svd-hal` or any future `hwdef` generator crate.
   - **The CLI:** `aethelgard check pinout --topology <t> --elf <e> [--hwdef-h <h>] [--allow <file>]`, one JSON line with each finding named by pin, net and decode, under `RUL-10`'s exit codes.
4. **Tests:**
   - **Tier 1:** the decoder against the spike's hand decode (GPIOA `MODER` = `0x0000A801`), on a small checked-in ELF excerpt or a synthetic ELF the test builds. A checked-in ArduPilot ELF is a GPL binary, so do not commit one; if a test needs a real one, it runs in the `ardupilot` CI job against the build's output.
   - **Every refusal:** truncated ELF, missing symbol, wrong size, and two decodes disagreeing.
   - **In CI:** step G passes, and each seed fails with its spike finding, `seed-pkg` now named by the package check.
5. **D8, one measured run:** load step G's ELF in the tools image's Renode 1.17.0 on the H753 platform, with USART3 as an analyzer, for a bounded virtual time. Record what prints, or the last PC and the peripheral it waits on. One run, recorded in FEEDBACK. Do not chase it.

**§1.4 STOP conditions:**
- D4 fails on GitHub's runner.
- The ELF decode and `hwdef.h` disagree on any committed board.
- The `ardupilot` job cannot be held under 20 minutes.

## 2. Acceptance

- **The image builds and smokes under `--network none` on a pull request.** Every pin is checked by the pin script.
- **`ardupilot` is green:** step G passes the check, and the three seeds fail it, each by name, with the expected finding.
- **The check's tier-1 tests pass** on every machine without ArduPilot.
- **D8's run is recorded.**
- **Nothing else changes:** no existing crate, fixture, generated file or verdict.
- **The CLAUDE.md gates pass,** and every CI shard stays under 20 minutes.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-10`, `RUL-11`, `RUL-12`, `RUL-15`, `RUL-16`.
- **Fence:**
  - `crates/aethelgard-pincheck` (new);
  - the CLI's `check pinout` wiring only;
  - `container/ardupilot*`, `container/check-pins.sh`;
  - `.github/workflows/ardupilot-image.yml` (and `ci.yml` only if the job lives there);
  - `Cargo.toml`'s workspace members;
  - this work order's FEEDBACK.
- **Never vendor, commit or link ArduPilot code or binaries.** The image and CI artifacts hold them.
- **On Furnace:**
  - reuse `~/.cache/aet-4400/`'s clone and toolchain read-only, or build the image;
  - one container of yours at a time, at 8 GiB or less, `nice 10`;
  - other builders: `1307`, `4300c`, `1303`, `4210`; never touch their worktrees.
- One PR, left unmerged. Write `~/.cache/aet-4400a/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 5. Adversarial pass, full

1. **Make a wrong board pass:**
   - a second `gpio_default_config` symbol;
   - a pin set to alternate mode at run time only;
   - an AF the topology names on a pin in input mode;
   - the allow-list used to hide a topology pin;
   - an ELF whose section table lies.
2. **Make a correct board fail:** `DEFAULTGPIO` lines, analog-mode pins (reset is analog on H7: say which reset value the reverse check uses, and cite RM0433), PH0/PH1 under HSE.
3. **Break the image:** a moved tag, a toolchain mirror serving other bytes, a pip package without its hash.
