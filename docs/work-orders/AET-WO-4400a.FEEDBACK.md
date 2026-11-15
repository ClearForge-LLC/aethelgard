# AET-WO-4400a FEEDBACK — ArduPilot in its own pinned image, built in CI, and a pinout check read from the linked ELF

**Builder:** Claude (strong tier), worktree `~/aethelgard-4400a`, branch `wo/aet-4400a-ardupilot-image` from `main` `cb44872`.
**Cadence:** build, with one measured stop (§1.4). The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #132); it is not committed here.

Labels:
- **[read]**: read at source, in this repository at `cb44872`, in ArduPilot at `Copter-4.7.2` (`fb773567`), or in Renode 1.17.0.
- **[measured]**: run on Furnace or on GitHub's runner; the cap, `nice` and `-j` are given with each run.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any image build, ArduPilot build, decode or Renode run)

All **[inferred]** from the `4400` spike's measurements (`AET-WO-4400.FEEDBACK.md` §2, §4, §5) unless marked.

**The image.**
- Base `debian:trixie-slim` by index digest (the tools image is trixie too), Python 3.13 from the archive. ArduPilot's waf runs under 3.13 as it did under the spike's 3.14.
- Every one of the nine pip packages has a wheel that 3.13 on x86_64 can install, except `empy==3.3.4`, which is an sdist only. Building it needs `setuptools`, which `--require-hashes` does not cover through pip's build isolation. So `setuptools` is pinned with its hash and installed first, and the rest go in with `--no-build-isolation`.
- Image size: 2.3–2.8 GB uncompressed (1.4 GB clone, 0.69 GB toolchain, about 0.15 GB venv, about 0.1 GB of Debian).
- Cold build on GitHub's runner: 3–6 min, most of it the toolchain download from `firmware.ardupilot.org` and the clone. Warm, from the `gha` cache: 2–4 min, dominated by importing about 2.5 GB of layers.
- The smoke (`MatekH743`, `-j$(nproc)`, 4 vCPU): 331 CPU-s ÷ 4 plus overhead, so **90–150 s**, under `--network none`, and `git status --porcelain` prints nothing.

**The `ardupilot` job.**
- Each of the four boards takes 280 CPU-s on Furnace, so **75–120 s** on the runner from clean.
- **One tree against four clean trees.** All four boards are installed as `Sentinel01`, so building them one after another in one tree reuses `build/Sentinel01`. `hwdef.h` reaches nearly every translation unit, so the reuse saves under 20 %. I expect one container per board and one container for all four to differ by under 10 %, and I will keep whichever is simpler if they do.
- The whole job (image from cache, four boards, building the `aethelgard` binary on the runner, the checks) takes **10–15 min**, under the 20-minute line.
- **D4:** with `GITHUB_ACTIONS=true` in the container, `configure` exits non-zero with `Numeric board ID found (7777)` (`hwdef.py:129-139`, [read] in the spike). With it unset, all four boards configure and build.

**The decode of step G** (all 11 ports; the spike hand-decoded GPIOA only):
- GPIOA: `MODER` `0x0000A801`, `AFRL` `0x55500000`, `AFRH` `0`.
- GPIOB: `MODER` `0x0000A000`, `AFRL` `0x44000000`.
- GPIOD: `MODER` `0x000A0000`, `AFRH` `0x00000077`.
- Every other port: `MODER` `0`, both AFRs `0`. Every unnamed pin is input, including PA13, PA14, PA15, PB3 and PB4, whose RM0433 reset state is alternate (debug).
- `hwdef.h` agrees with the ELF on every port.

**The reverse and package checks need a definition of "configured".**
- I expect the literal "every pin whose `MODER` differs from its reset value" to flag almost every pin. ArduPilot writes input (`00`) to every unnamed pin, while RM0433 resets most pins to analog (`11`). It would also flag ports I, J and K, which mostly do not exist on LQFP144.
- **Predicted definition:** a pin is configured when it is driven, that is output or alternate, and is not at its RM0433 reset `MODER` and AF. Input and analog drive nothing.
- Under it, step G configures exactly the 6 topology pins plus PD8 and PD9.

**The seeds**, as the spike measured them, now from the Rust check:
- `seed-pin`: forward, `I2C_SCL` at PB6 is input, not alternate AF4; reverse, PB8 alternate AF4 named by nothing.
- `seed-af`: forward, PB6 and PB7 are on AF6, solved AF4.
- `seed-pkg`: package, PI0 is an output and is not a pin of `STM32H753ZITx` (LQFP144); reverse also names PI0.

**The ELF reader.** Hand-written, about 200 lines, with no new dependency. The CLI already holds a strict ELF32 header reader (`crates/aethelgard-cli/src/elf.rs`), and `object` would add a dependency tree for about 60 lines of parsing.

**D8, one Renode run of step G's ELF.**
- **The cap may stop it before it starts.** `4210` measured (its F10, bus note 02:40 UTC) that podman 5.7.0 with `--memory 8g --memory-swap 8g` leaves `memory.swap.max` at 8589934592, not 0. The kickoff's rule then stops the run before Renode starts. I expect the same reading.
- **If it runs:** nothing on USART3 within a bounded virtual time. The last PC sits in a ChibiOS or ArduPilot busy-wait on a peripheral Renode's H753 platform models only partly. My guess is the storage or flash driver, or the IWDG. The spike's reading found no blocker before the banner, so this is a weak prediction.

**CI time across the PR.**
- This PR also triggers `tools-image` (`container/**`) and `engine-image` (`crates/**`, `Cargo.*`): 10–14 min and about 4 min, as on recent runs.
- `tools-shard (1)` already takes 17 m 19 s. The new crate's tests are tier 1 and add seconds to `gates`, not to any shard.

## 2. What was built

### 2.1 The image (`container/ardupilot.Dockerfile`)

- **The base** is `debian:trixie-slim@sha256:a72b4433…b6b2`, the index digest Docker Hub served for the tag on 2026-11-15 [measured, registry `HEAD`].
- **apt:** eight packages at exact versions, taken from `trixie` and `trixie-security` through Debian's madison service the same day [measured]: `ca-certificates`, `curl`, `python3`, `python3-venv`, `make`, `git`, `rsync`, `bzip2`.
- **The toolchain:** `curl` plus `sha256sum -c` on `21134caa…cd618`, unpacked to `/opt`.
- **The clone:** `git clone --depth 1 -b Copter-4.7.2 --recurse-submodules --shallow-submodules`. Then `test "$(git rev-parse HEAD)" = fb773567…` and `test -z "$(git status --porcelain)"`.
- **pip:** the nine packages of the spike's freeze, from a requirements file with one sha256 per file pip may take.
  - Each hash is PyPI's for the CPython 3.13 x86_64 wheel, or the sdist where there is no wheel [measured, PyPI JSON].
  - **`empy==3.3.4` is an sdist only** [measured], as predicted. `setuptools==84.2.1`, also by hash, goes in first and alone, so `empy` builds with `--no-build-isolation`. That is the one package beyond the frozen nine, and it is there only to build `empy`.
  - Both installs are `--no-deps`, so the file is the whole set.
- **`ARDUPILOT-LICENSE.md`** names `GPL-3.0-or-later` (ArduPilot's `COPYING.txt` is GPLv3, and its headers say "or any later version" [read]). It also states that the image carries ArduPilot's complete source with `.git`.

**`check-pins.sh`** gains two shapes, each as tight as the existing `curl` shape:
- **The clone:** exactly `git clone --depth 1 -b REF --recurse-submodules --shallow-submodules https://URL DIR`, and the next line must be the `rev-parse HEAD` test against 40 hex.
- **pip:** exactly `pip install --no-cache-dir --require-hashes --no-deps [--no-build-isolation] -r PATH`. `PATH` must be the target of `COPY NAME PATH`, and `NAME`, beside the Dockerfile, may hold only `name==version --hash=sha256:<64 hex>…` lines.
- Any other `pip` or `git clone` is still refused by the deny-list.
- [measured] All three Dockerfiles pass. Ten mutants are each refused by name: short commit, no HEAD line, an unhashed pip line, a `>=` range, an `--index-url` line, no `--require-hashes`, a `COPY` from another file, an extra clone flag, and an unpinned apt package. A changed toolchain hash passes the static check by design and fails `sha256sum -c` at build time.

### 2.2 The crate `aethelgard-pincheck` and `check pinout`

- **The ELF reader** is hand-written, about 300 lines, with no new dependency. `object` was not taken: the reader needs about 60 lines of field reads, and the CLI already carries a hand-written ELF32 reader.
  - It refuses by name, among others: a second `.symtab`, a symbol that is not `STT_OBJECT`, a symbol in a section that is not allocated `PROGBITS`.
  - **It locates the bytes twice,** through the symbol's section and through the one `PT_LOAD` whose file image holds the address, and refuses (`SectionTableDisagrees`) when the two offsets differ.
- **The `hwdef.h` reader** requires each of the 33 macros to be defined exactly once and named nowhere else, with exactly one term per pin.
- **The package data is not in `aethelgard-ir`.** The work order says the crate takes "the package data" from `ir`. `ir` holds only the `PinTable` type; the XML and its parser are `aethelgard-svd-hal`'s, which D7's separation forbids. So the check takes a `&PinTable`, and the CLI, which already depends on `svd-hal`, passes `pin_table("STM32H753ZITx")`. Nothing is copied.
- **"Configured", not "differs from reset"** (§1.3 asks for the latter). With the literal rule, ArduPilot's input on every unnamed pin against RM0433's analog reset would flag about 170 pins on every board, and every pin of ports I, J and K would fail the package check.
  - **The rule built:** a pin is configured when it is output or alternate and not at its RM0433 Rev 9 §11.4.1 reset state, which is analog except PA13–PA15 and PB3–PB4 at alternate AF0 (§11.4.9–10) [read, `rm0433-rev9.pdf`].
  - **What it lets through:** a pin left as an input with a pull, which drives nothing. That is a blind spot, listed in the crate's README.
