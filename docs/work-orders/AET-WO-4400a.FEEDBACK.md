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
- [measured] All three Dockerfiles pass. Nine mutants are each refused by name: short commit, no HEAD line, an unhashed pip line, a `>=` range, an `--index-url` line, no `--require-hashes`, a `COPY` from another file, an extra clone flag, and an unpinned apt package. A tenth, a changed toolchain hash, passes the static check by design and fails `sha256sum -c` at build time.

### 2.2 The crate `aethelgard-pincheck` and `check pinout`

- **The ELF reader** is hand-written, about 300 lines, with no new dependency. `object` was not taken: the reader needs about 60 lines of field reads, and the CLI already carries a hand-written ELF32 reader.
  - It refuses by name, among others: a second `.symtab`, a symbol that is not `STT_OBJECT`, a symbol in a section that is not allocated `PROGBITS`.
  - **It locates the bytes twice,** through the symbol's section and through the one `PT_LOAD` whose file image holds the address, and refuses (`SectionTableDisagrees`) when the two offsets differ.
- **The `hwdef.h` reader** requires each of the 33 macros to be defined exactly once and named nowhere else, with exactly one term per pin.
- **The package data is not in `aethelgard-ir`.** The work order says the crate takes "the package data" from `ir`. `ir` holds only the `PinTable` type; the XML and its parser are `aethelgard-svd-hal`'s, which D7's separation forbids. So the check takes a `&PinTable`, and the CLI, which already depends on `svd-hal`, passes `pin_table("STM32H753ZITx")`. Nothing is copied.
- **"Configured", not "differs from reset"** (§1.3 asks for the latter). With the literal rule, ArduPilot's input on every unnamed pin against RM0433's analog reset would flag about 170 pins on every board, and every pin of ports I, J and K would fail the package check.
  - **The rule built:** a pin is configured when it is output or alternate and not at its RM0433 Rev 9 §11.4.1 reset state, which is analog except PA13–PA15 and PB3–PB4 at alternate AF0 (§11.4.9–10) [read, `rm0433-rev9.pdf`].
  - **What it lets through:** a pin left as an input with a pull, which drives nothing. That is a blind spot, listed in the crate's README.
- **The CLI** maps refusals to `RUL-10`'s codes: a malformed ELF, `hwdef.h` or allow-list exits 3, a topology of the wrong shape exits 2, and `DecodesDisagree` exits 5 (two witnesses of one image disagree). The contract is `crates/aethelgard-cli/README.md`, `## check pinout`.
- **Separation (D7):** `tests/separation.rs` reads every member's declared dependencies (`cargo metadata --no-deps`).
  - The check reaches only `aethelgard-ir`, dev dependencies included.
  - Only the CLI links the check. `svd-hal`, `twin` and `evolve` reach it only through their test-only dev dependency on the CLI.
  - The first version read Cargo's full resolve with `--offline`. CI has not downloaded other platforms' crates, so it failed in `gates`, `tools-image` and two shards (`failed to download android_system_properties`). `--no-deps` needs no download [measured, empty `CARGO_HOME`].

### 2.3 CI: where the job lives and what it costs [measured, GitHub's runner, 4 vCPU]

- **The jobs live in `ardupilot-image.yml`, not `ci.yml`,** because they need the image, which only this workflow builds.
- **The layout:**
  - `image`: build, smoke, D4 control;
  - `ardupilot (<board>)`, a matrix of four, one board per job, each building the image from the same `gha` cache;
  - `cli`: the `aethelgard` binary;
  - `pinout`: the four checks, against a topology from `aethelgard solve` on `examples/sentinel-01-minimal.json`;
  - `publish`, from `main` only.
- **Why four jobs.** Run 2 (one job) measured image 3 m 31 s, smoke 3 m 22 s, then 182–190 s per clean board. That is about 20.5 min before anything else, and the job hit its timeout.

| run | job | wall |
|---|---|---|
| 1 (`ded6baf`) | image, cold | 2 m 39 s, then the smoke failed on `gcc: not found` (below) |
| 2 (`ee65b5b`) | one job | timed out at 25 min during the reuse measurement |
| 3 (`e61b061`) | `image` | 5:06 (image from cache 1 m 11 s, 2,633,999,212 B; smoke 216 s) |
| 3 | `ardupilot (step-g / seed-pin / seed-af / seed-pkg)` | 3:30 / 4:23 / 4:38 / 3:36 (boards 142–183 s) |
| 3 | `ardupilot-reuse` (measurement) | 10:37: 145, 142, 142, 141 s |
| 3 | `cli` / `pinout` | 0:48 / 0:07 |

- **One tree against four clean trees.** Reusing the build directory saves almost nothing. Inside the reuse job, the first board (clean) took 145 s and the three that reused its directory 141–142 s. The four clean matrix jobs took 142–183 s. `hwdef.h` reaches nearly every translation unit. Four clean trees in parallel jobs win on wall time, and the measurement job is removed. `-j$(nproc)` is 4 here; waf's default is the same.
- **The image needs a host C compiler.** waf compiles `libraries/AP_Scripting/generator/src/main.c` for the host. Furnace has `gcc`, so the spike's S1 never saw it: a correction to its "no host compiler needed". `gcc` and `libc6-dev` are pinned (run 1).
- **D4 holds on GitHub's runner.**
  - The runner prints `GITHUB_ACTIONS on the runner: true`.
  - With it passed into the container, configure fails with `ValueError: Numeric board ID found (7777)`.
  - With it unset, all four boards configure and build.
- **`pinout`:** step G exits 0. Each seed exits 1 with exactly the findings expected, `seed-pkg` named `package NotInPackage PI0`.
- **Reproducible across machines.** `seed-pkg`'s ELF built on the runner has the same sha256 (`061f7a36…`) as the spike's on Furnace, under another path and another Python.
- **The other workflows this PR triggers:** `tools-image` 8:07 and `engine-image` 3:36. The tools shards run 11:38–16:48. `tools-shard (4)` is the slowest at 16:48, and I did not rebalance it.

### 2.4 Step G's full decode [measured, the CI ELF, sha256 `75d483f2…`]

- **Ports A, B and D,** exactly as predicted:
  - A: `MODER 0x0000a801`, `AFRL 0x55500000`;
  - B: `MODER 0x0000a000`, `AFRL 0x44000000`;
  - D: `MODER 0x000a0000`, `AFRH 0x00000077`.
- **Every other port:** `MODER` and both AF registers 0.
- **Not predicted, and for `4400b`:**
  - every pin's `OSPEEDR` is `10` (high speed), `0xaaaaaaaa` on every port;
  - PB6 and PB7 are open-drain (`OTYPER 0xc0`);
  - PA0, PD8 and PD9 have pull-ups (`PUPDR`);
  - `ODR` is `0xffff` on every port.

### 2.5 D8, the one Renode run [measured]

- **The setup:**
  - Renode 1.17.0 in the pinned tools image, its own `platforms/cpus/stm32h753.repl`;
  - step G's CI ELF loaded with `cpu VectorTableOffset 0x08000000`, and USART3 to a file backend;
  - `emulation RunFor "2"`;
  - the architect's replaced rule 3: scope `memory.max` 8589934592, `memory.swap.max` 0, `pids.max` 8192; 4 s wall.
  - The script was rehearsed first on a 10-byte hand-made ELF, which ran the same commands.
- **What prints:** one byte on USART3, `\n`. The banner is `"\n\nInit …"`, so it stopped inside the first line.
- **Where it stops:** PC `0x080ffdb0` at 2 s and again after a further 0.2 ms, which is ChibiOS's `__idle_thread` (`addr2line`). No thread is runnable.
  - USART3 and DMA1's streams were configured, and Renode logged their interrupt-enable bits as unhandled.
  - So the firmware waits for an interrupt the model does not raise, most likely the USART3 TX DMA's transfer-complete [inferred].
  - Not chased, as ruled.
- **The earlier attempt (rule 3 as it was):** podman's `--memory 8g --memory-swap 8g` read back `memory.swap.max` 8589934592. My runner held the command, so nothing ran, as `4210`'s F10 also measured.

## 3. Results beside predictions

| prediction | result |
|---|---|
| empy sdist only, setuptools pinned first | ✔ |
| image 2.3–2.8 GB | ✔ 2.44–2.63 GB |
| cold image build 3–6 min | ✔ 2 m 39 s – 3 m 31 s (just under) |
| smoke 90–150 s | ✘ 202 s |
| a board 75–120 s | ✘ 142–190 s: a runner core is about 2.5× slower than Furnace's |
| one tree saves under 20 % | ✔ about 2–3 % (145 s, then 141–142 s) |
| the job 10–15 min | ✘ about 20.5 min as one job, so it is four (§2.3) |
| D4 refusal message and the unset build | ✔ both |
| step G's full decode | ✔ every `MODER`/`AFR` word |
| "configured" needed, and step G configures 6 + PD8/PD9 | ✔ |
| the three seeds' findings | ✔ exact |
| ELF reader hand-written, no dependency | ✔ |
| D8 stopped by the cap | ✔ under the old rule 3. Under the replaced rule it ran |
| D8 prints nothing, stalls on a busy-wait | ✘ in part: one byte, then idle waiting on an interrupt, not a busy-wait |
| no host compiler (spike S1) | ✘ `gcc` needed |

## 4. Adversarial pass (a fresh subagent, Opus: judgement, not volume)

It was given the code, the work order's §5 and the real data, not my conclusions. Its harnesses are in `~/.cache/aet-4400a/adv/`.

I spot-checked every BROKEN finding at source before acting:
- **A2:** `readelf` of the real ELF.
- **A3:** `check-pins.sh` and `sh -c` on the mutant.
- **A5:** ChibiOS `crt0_v7m.S:269` calls `__early_init` before the data initialisation at `:316`, and ArduPilot's `board.c` `__early_init` calls `stm32_gpio_init()`, whose comment says DATA may not be relied on.
- **A1:** `chibios_hwdef.py:2877` sets `default_gpio`, and `:2894-2897` applies it when `MCU` is read. I counted 11 boards with `DEFAULTGPIO` before `MCU` [read].

I re-ran its 41 crafted cases on the fixed binary.

- **A2, BROKEN, fixed.** The reader used `p_vaddr`, but the flash image follows `p_paddr`: a second `PT_LOAD` could flash other bytes over the symbol and pass. The holding segment must now run where it is flashed, and no other segment may flash over the symbol (`SymbolNotInPlace`, `SegmentsOverlap`). The real symbol is in `.text`, with vaddr equal to paddr [measured, `readelf`].
- **A5, BROKEN (latent), fixed.** A symbol in a writable section was accepted, but ChibiOS's `__early_init` runs before `.data` is copied. A writable section is now `SymbolOutsideSection`.
- **A3, BROKEN, fixed in `check-pins.sh`.** `;` after the HEAD test or after `sha256sum -c` let a moved tag or other bytes through, because Docker's `sh -c` has no `-e` [measured]. `;` and a lone `&` are now refused.
- **A4, BROKEN, fixed.**
  - `git -c … clone` passed the deny-list. It now matches `git … clone`.
  - Three holes older than this work order (they pass at `cb44872`), closed in the second round:
    - `COPY --from=busybox` and `RUN --mount=…,from=alpine:latest` pulled from a registry. A `from=` must now name a stage declared above or a digest-pinned image.
    - `ADD git@…` fetched. `ADD` is now refused outright, since no Dockerfile here uses it.
  - [measured] Each of the four mutants is refused. A declared stage (`FROM … AS base` then `COPY --from=base`) still passes, and all three Dockerfiles pass.
- **ET_CORE, fixed:** `e_type` other than `ET_EXEC` is `WrongType`.
- **A1, BROKEN by the ruled rule: a decision for `4400b`.** A `DEFAULTGPIO OUTPUT` line before `MCU` (11 upstream boards do this) drives every pin of ports A–K. The package check then names the 62 pins the LQFP144 does not bond out, and no input can clear them. §1.3 mandates that outcome. Either the generator never emits such a line, or a declared input exempts pins the package does not bond out. The READMEs now say so.
- **A6, BLIND SPOT, documented.** A symbol table that points at a decoy copy of good bytes passes on the ELF alone. Only the cross-check catches it (`11-symlie-realh` exits 5), and CI always passes `hwdef.h`.
- **A7, BLIND SPOT, declared in `check-pins.sh`'s header.** `git submodule update --remote`, `git fetch`/`pull` and `pipx` pass the static check. The venv's pip comes from Debian's `python3-pip-whl`, an apt dependency whose version is not pinned [inferred].
- **A8, a static bypass that the runtime check still stops.** A second `COPY` over the requirements path, or `PIP_EXTRA_INDEX_URL`, is still caught by `--require-hashes`.
- **A11, documented in the crate README:**
  - the firmware's MCU is not read, so an H743 or H757 image decodes like an H753;
  - ArduPilot's run-time `ALT(n)` sets are invisible;
  - an allow-listed pin can carry a second copy of a topology signal;
  - `hwdef.h`'s `OTYPER` is not compared (4400b's).
- **Withstood:**
  - two `gpio_default_config` symbols, a lying section offset, `NOBITS`, two holding segments, and offset overflows;
  - an AF on a pin in input mode;
  - the allow-list hiding a topology pin;
  - every malformed allow-list and topology tried;
  - CRLF in `hwdef.h`;
  - the three Dockerfiles still passing, and a moved tag or other bytes failing the `&&` chain.
- **Correct boards that pass:**
  - SWD on PA13/PA14 at AF0, which equals reset;
  - PB3, PB4 and PA15 at AF0;
  - an analog PC4;
  - PH0/PH1 with `OSCILLATOR_HZ 8000000`, which ArduPilot leaves as input.
  - The reset baseline matches RM0433 Rev 9 §11.4.1 [read].

## 5. Gates

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` pass on Furnace, under a 16 GiB scope at nice 10.
- `check-pins.sh` passes all three Dockerfiles.
- **Mutation, one mutant per guard this work order adds:**
  - 19 in `aethelgard-pincheck`, each killed by the crate's suite;
  - two separation mutants (`pincheck` depending on `svd-hal`; `twin` depending on `pincheck`), both killed;
  - four mutants of the CLI's exit-code mapping (`DecodesDisagree` to 1, a malformed input to 2, a wrong-shape topology to 3, findings to 0), each killed by `tests/pinout_tier1.rs`, which also pins exactly one `ok` key on the line;
  - 17 `check-pins.sh` mutants refused, one (the toolchain hash) passing by design, and one declared-stage control passing.
- **`ci.yml`'s package step** was reproduced on Furnace as written, without its `rm -rf`, because `gates` failed before it on `e61b061`. `aethelgard-pincheck` packages at 26,182 B with its README and licences, and the CLI installs offline from the nine packages and validates the example.
- **Reduction pass (diff):**
  - −446 net. 2 duplicates removed: the spike's topology copy, and `lib.rs` restating the README's rules. 2 dead paths removed: the `ardupilot-reuse` measurement job and the build script's `-i` flag. The rest of `spike/4400/` is deleted, as builds delete `spike/` (roadmap §6) and the spike's FEEDBACK §10 names it. 0 protected kept. 1 finding below.
  - **The finding:** the image build step is written three times in `ardupilot-image.yml` (`image`, `ardupilot`, `publish`), as in the two other image workflows. A composite action would make it one. That is for the gate pass.

## 6. For the architect

- **D-1, the package rule under `DEFAULTGPIO` (adversarial A1).** As ruled, a board whose `DEFAULTGPIO OUTPUT` comes before `MCU` can never pass, because it drives the 62 pins the LQFP144 does not bond out. **Recommended:** `4400b`'s generator never emits `DEFAULTGPIO`, and the check stays as ruled.
- **D-2, the GPL toolchain binaries in a published image.** The image carries Arm's GCC binaries (GPL) as well as ArduPilot's source. `ARDUPILOT-LICENSE.md` names Arm's release as the corresponding source. The tools image does the same with KiCad's official image. Whether a pointer to Arm's source meets GPLv3 §6 for a public `ghcr.io` package is Scotty's call [inferred].
- **D-3, the `ardupilot-image.yml` triggers** include `crates/**`, `examples/**` and `Cargo.*`, so any solver change re-judges step G. That costs about 5 min of runner wall time (parallel jobs) on most PRs. Narrow it to the check's and the solver's crates if that is too much.
- **D-4, the first publish.** `container/ARDUPILOT-IMAGE` does not exist until `main` publishes. A follow-up PR sets it from the publish summary, as `RUL-15` says.
- **`tablediff.py` leaves with `spike/4400/`.** D7's independence measurement (597 shared pairs, 0 AF disagreements) has no Rust successor. The spike's FEEDBACK §5 is its record. A test could be added if the table comparison should stay live.
- **Q-1, the refused removal (§8).** Rule 3 says to stop on a tool call denied by the permission classifier. Claude Code's built-in removal check refused one command. I read that check as distinct from the classifier and carried on without the removal, which was never done another way. The reading is yours to make: if it counts, this work order should have stopped there.

## 7. Edits outside the literal fence, each forced by adding a member

- `crates/aethelgard-cli/tests/workspace_shape.rs`: nine members, five `ir` dependants. `pincheck` carries the same `RUL-16` `clippy.toml`. `crates/aethelgard-ir/tests/rul16_fence.rs` pins only three crates' files, and I did not add `pincheck` to it (outside the fence).
- `crates/aethelgard-cli/src/mcp.rs` and `tests/mcp_tier1.rs`: `check_pinout` in the two pinned tool lists, and the count 16 → 17.
- `crates/aethelgard-cli/tests/package_licences.rs` (published list), `crates/aethelgard-cli/tests/strict_fence.rs` (`pinout.rs`'s role, `Other`), and the CLI README's contract section.
- `.github/workflows/ci.yml`: `aethelgard-pincheck` in the publishable crate list, before the CLI. `docs/releasing.md`'s publish order is not updated (protected).
- `spike/4400/`: the four fixtures moved, as the work order says, and the rest deleted (§5).

## 8. Process notes

- **One tool call was refused by Claude Code's built-in removal check** (§6, Q-1).
  - The command was a test of `check-pins.sh` mutants. It began `rm -rf $S/p`, where `S` was this session's scratchpad (`/tmp/claude-1001/…/scratchpad`).
  - Nothing in that command ran, including the edit to `check-pins.sh` it carried.
  - I did not re-attempt the removal. The work went on in a new directory, `…/scratchpad/pins2`. The old directory, `…/scratchpad/p`, is left in place.
- **Rule 3 was replaced mid-build.** D8 ran under the replacement. The earlier containers were:
  - `renode --version` under the old rule, which the gate held: nothing ran;
  - `renode --version` under the new rule;
  - one Renode rehearsal of the D8 script on a 10-byte hand-made ELF.

## 9. After the first READY commit

- **AET-WO-4300c (#129) merged** after the first READY commit, `3a5a817`, which was CI green on every job. PR #135 then conflicted.
- **Two rules apply.** Kickoff item 4 says to merge `main` whenever another PR lands; item 7 says to push nothing after the READY commit unless asked. I merged, so that the head under review is mergeable, and put a new READY commit on top, so the last push still carries the READY subject.
- **The merge** keeps both `check` subcommands: `pinout`, then 4300c's `speeds`. That puts 18 tools in the list. Solved schema 4 (4300c) changes the test topology's `schema_version`, `specification_sha256`, `spi_devices` and `spi_lines`, but not its pins, so `tests/common/topology.json` is regenerated with `aethelgard solve`.
- **The same commit** closes adversarial A4's three older `check-pins.sh` holes (§4).
- **The architect accepted PR #135 at `03d4aa1`** (2026-11-15) and asked for one more merge of `main` before it merges. Rulings: D-1 accepted as recommended (4400b's generator never emits DEFAULTGPIO; the check stays as ruled); D-2 goes to the operator and does not block this PR; D-3, the triggers stay; D-4, the follow-up PR sets `ARDUPILOT-IMAGE` after `main` publishes; Q-1, Claude Code's built-in removal check is not the auto-mode classifier, so not retrying it was right and no stop was owed.
- **The merges** bring in 1307 (#126), 4300d (#137), 4300f (#138), 4220 (#140), docs #139 and #141, and then 1308 (#142), which landed during the gates. No conflicts. Solved schema is still 4. `aethelgard solve --file examples/sentinel-01-minimal.json` changes only `specification_sha256`, from `9eb61c49…` to `1fe63a4f…`, because the example specification is now v5. The pins, devices and lines are unchanged [measured]. `tests/common/topology.json` is regenerated from that output. TIER2_MIN stays at main's 101: this work order adds no tier-2 test.
- **Gates after the merges** [measured, Furnace, capped scope]: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean; `cargo test --workspace` exits 0, with 1381 tests passing across 89 test binaries.
